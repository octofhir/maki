//! CST Adapter for GritQL
//!
//! This module provides adapters to expose our Rowan-based CST to GritQL's
//! tree-walking interface. The key insight is that Rowan already provides
//! exactly what GritQL needs: parent pointers, efficient traversal, and
//! precise text ranges.

use grit_util::error::GritResult;
use grit_util::{AstCursor, AstNode as GritAstNode, ByteRange, CodeRange};
use maki_core::cst::{FshSyntaxKind, FshSyntaxNode};
use std::borrow::Cow;

/// Wrapper around Rowan's FshSyntaxNode that implements GritQL's AstNode trait
///
/// This is a lightweight wrapper that just delegates to Rowan's APIs.
/// The Clone implementation is cheap because Rowan uses Arc internally.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FshGritNode {
    /// The underlying Rowan CST node
    node: FshSyntaxNode,
}

impl FshGritNode {
    /// Create a new GritQL node from a CST node
    pub fn new(node: FshSyntaxNode) -> Self {
        Self { node }
    }

    /// Get the underlying CST node
    pub fn syntax(&self) -> &FshSyntaxNode {
        &self.node
    }

    /// Get the syntax kind
    pub fn kind(&self) -> FshSyntaxKind {
        self.node.kind()
    }

    /// Check if this node represents a list
    pub fn is_list(&self) -> bool {
        false
    }

    /// Get named children (non-trivia children)
    pub fn named_children(&self) -> impl Iterator<Item = FshGritNode> + Clone {
        // Collect to Vec to get a cloneable iterator
        let children: Vec<_> = self
            .node
            .children()
            .filter(|n| !n.kind().is_trivia())
            .map(FshGritNode::new)
            .collect();
        children.into_iter()
    }

    /// Get the source text
    pub fn source(&self) -> &str {
        ""
    }

    /// Get the text content (convenience method)
    pub fn text_content(&self) -> String {
        self.node.text().to_string()
    }
}

impl GritAstNode for FshGritNode {
    fn ancestors(&self) -> impl Iterator<Item = Self> {
        // Rowan provides this directly!
        self.node
            .ancestors()
            .skip(1) // Skip self
            .map(FshGritNode::new)
    }

    fn children(&self) -> impl Iterator<Item = Self> {
        // Rowan provides this directly!
        self.node.children().map(FshGritNode::new)
    }

    fn parent(&self) -> Option<Self> {
        // Rowan provides this directly!
        self.node.parent().map(FshGritNode::new)
    }

    fn next_named_node(&self) -> Option<Self> {
        // In CST, we might want to skip trivia tokens
        // For now, just use next_sibling
        self.next_sibling()
    }

    fn previous_named_node(&self) -> Option<Self> {
        self.previous_sibling()
    }

    fn next_sibling(&self) -> Option<Self> {
        // Rowan provides this directly!
        self.node.next_sibling().map(FshGritNode::new)
    }

    fn previous_sibling(&self) -> Option<Self> {
        // Rowan provides this directly!
        self.node.prev_sibling().map(FshGritNode::new)
    }

    fn text(&self) -> GritResult<Cow<'_, str>> {
        // Rowan provides this directly - lossless text!
        Ok(Cow::Owned(self.node.text().to_string()))
    }

    fn byte_range(&self) -> ByteRange {
        let range = self.node.text_range();
        ByteRange::new(range.start().into(), range.end().into())
    }

    fn code_range(&self) -> CodeRange {
        let range = self.node.text_range();
        let source = self.node.text().to_string();

        CodeRange::new(range.start().into(), range.end().into(), &source)
    }

    fn walk(&self) -> impl AstCursor<Node = Self> {
        FshGritCursor::new(self.clone())
    }
}

/// Cursor for tree traversal
///
/// This wraps Rowan's traversal APIs to provide the cursor interface
/// that GritQL expects.
#[derive(Clone, Debug)]
pub struct FshGritCursor {
    /// Current node
    current: FshGritNode,
    /// Root node (cursor can't go above this)
    root: FshGritNode,
}

impl FshGritCursor {
    pub fn new(node: FshGritNode) -> Self {
        Self {
            current: node.clone(),
            root: node,
        }
    }
}

impl AstCursor for FshGritCursor {
    type Node = FshGritNode;

    fn node(&self) -> Self::Node {
        self.current.clone()
    }

    fn goto_first_child(&mut self) -> bool {
        if let Some(first_child) = self.current.node.first_child() {
            self.current = FshGritNode::new(first_child);
            true
        } else {
            false
        }
    }

    fn goto_parent(&mut self) -> bool {
        // Don't go above root
        if self.current.node == self.root.node {
            return false;
        }

        if let Some(parent) = self.current.node.parent() {
            self.current = FshGritNode::new(parent);
            true
        } else {
            false
        }
    }

    fn goto_next_sibling(&mut self) -> bool {
        if let Some(next) = self.current.node.next_sibling() {
            self.current = FshGritNode::new(next);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::cst::parse_fsh;

    #[test]
    fn test_node_wrapping() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let grit_node = FshGritNode::new(cst);

        assert_eq!(grit_node.kind(), FshSyntaxKind::Document);
    }

    #[test]
    fn test_parent_child_navigation() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);

        // Test children iteration
        let children: Vec<_> = root.children().collect();
        assert!(!children.is_empty(), "Root should have children");

        // Test parent navigation
        if let Some(first_child) = children.first() {
            let parent = first_child.parent();
            assert!(parent.is_some(), "Child should have parent");
        }
    }

    #[test]
    fn test_sibling_navigation() {
        let source = "Profile: Test1\nProfile: Test2";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);
        let children: Vec<_> = root.children().collect();

        if children.len() >= 2 {
            let first = &children[0];
            let next = first.next_sibling();
            assert!(next.is_some(), "First child should have next sibling");
        }
    }

    #[test]
    fn test_text_extraction() {
        let source = "Profile: MyPatient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);
        let text = root.text().unwrap();

        // Should match source (lossless!)
        assert_eq!(text.as_ref(), source);
    }

    #[test]
    fn test_byte_range() {
        let source = "Profile: MyPatient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);
        let range = root.byte_range();

        assert_eq!(range.start, 0);
        assert_eq!(range.end, source.len());
    }

    #[test]
    fn test_cursor_navigation() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);
        let mut cursor = FshGritCursor::new(root.clone());

        // Try to go to first child
        let has_child = cursor.goto_first_child();
        assert!(has_child, "Root should have children");

        // Try to go back to parent
        let returned = cursor.goto_parent();
        assert!(returned, "Should be able to return to parent");

        // Should be back at root
        assert_eq!(cursor.node().kind(), root.kind());
    }

    #[test]
    fn test_ancestors_iteration() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);

        if let Some(child) = root.children().next() {
            let ancestors: Vec<_> = child.ancestors().collect();
            assert!(!ancestors.is_empty(), "Child should have ancestors");
        }
    }

    #[test]
    fn test_lossless_property() {
        let source = "Profile: MyPatient  // Important profile\nParent: Patient";
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);

        let root = FshGritNode::new(cst);
        let reconstructed = root.text().unwrap();

        // This is the key property of CST!
        assert_eq!(reconstructed.as_ref(), source, "CST should be lossless");
    }
}
