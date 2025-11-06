//! FSH GritQL Tree - CST-based implementation
//!
//! This module provides the `Ast` trait implementation that allows GritQL
//! to query our Rowan-based CST. Unlike the old implementation that converted
//! a flat AST to a tree, this one directly exposes the CST which already
//! has the tree structure we need.

use super::cst_adapter::FshGritNode;
use grit_util::Ast;
use maki_core::cst::{FshSyntaxNode, parse_fsh};
use std::borrow::Cow;

/// FSH GritQL Tree - implements grit_util::Ast over our Rowan CST
///
/// This is a lightweight wrapper that just holds the root node and source text.
/// The actual tree structure is provided by Rowan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FshGritTree {
    /// Root CST node wrapped for GritQL
    root: FshGritNode,
    /// Source text (owned for lifetime management)
    source: String,
}

impl FshGritTree {
    /// Parse FSH source and create a GritQL-queryable tree
    ///
    /// # Example
    ///
    /// ```ignore
    /// use maki_rules::gritql::FshGritTree;
    ///
    /// let source = "Profile: MyPatient\nParent: Patient";
    /// let tree = FshGritTree::parse(source);
    ///
    /// // Now you can query this tree with GritQL patterns
    /// ```
    pub fn parse(source: &str) -> Self {
        let (cst, _lexer_errors, _parse_errors) = parse_fsh(source);
        Self::from_cst(cst, source.to_string())
    }

    /// Create a GritQL tree from an existing CST node
    ///
    /// This is useful when you've already parsed the FSH and want to
    /// run GritQL queries on a specific subtree.
    pub fn from_cst(root: FshSyntaxNode, source: String) -> Self {
        let root = FshGritNode::new(root);
        Self { root, source }
    }

    /// Alias for from_cst (for backwards compatibility with tests)
    pub fn new(root: FshSyntaxNode, source: String) -> Self {
        Self::from_cst(root, source)
    }

    /// Get the root node
    pub fn root(&self) -> &FshGritNode {
        &self.root
    }

    /// Get the source text
    pub fn source_text(&self) -> &str {
        &self.source
    }
}

impl Ast for FshGritTree {
    type Node<'a> = FshGritNode;

    fn root_node(&self) -> Self::Node<'_> {
        // Cheap clone (Rowan uses Arc)
        self.root.clone()
    }

    fn source(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grit_util::{Ast, AstCursor, AstNode};

    #[test]
    fn test_parse_simple_profile() {
        let source = "Profile: MyPatient\nParent: Patient";
        let tree = FshGritTree::parse(source);

        let root = tree.root_node();
        assert!(root.children().count() > 0, "Root should have children");
    }

    #[test]
    fn test_source_preservation() {
        let source = "Profile: MyPatient  // Comment\nParent: Patient";
        let tree = FshGritTree::parse(source);

        // Lossless property: tree should contain exact source
        assert_eq!(tree.source(), source);
    }

    #[test]
    fn test_ast_trait_root_node() {
        let source = "Profile: Test";
        let tree = FshGritTree::parse(source);

        let root = tree.root_node();
        let text = root.text().unwrap();

        assert_eq!(text.as_ref(), source);
    }

    #[test]
    fn test_from_cst() {
        let source = "Profile: MyPatient";
        let (cst, _, _) = parse_fsh(source);

        let tree = FshGritTree::from_cst(cst, source.to_string());

        assert_eq!(tree.source(), source);
        assert!(tree.root().children().count() > 0);
    }

    #[test]
    fn test_multiple_definitions() {
        let source = r#"Profile: Patient1
Parent: Patient

Profile: Patient2
Parent: Patient

ValueSet: TestVS
"#;
        let tree = FshGritTree::parse(source);

        let root = tree.root_node();
        let children: Vec<_> = root.children().collect();

        assert!(
            children.len() >= 3,
            "Should have multiple top-level definitions"
        );
    }

    #[test]
    fn test_tree_navigation() {
        let source = "Profile: MyPatient\nParent: Patient";
        let tree = FshGritTree::parse(source);

        let root = tree.root_node();

        // Walk the tree
        let mut cursor = root.walk();

        // Should be able to navigate to children
        assert!(cursor.goto_first_child(), "Should have first child");

        // Should be able to navigate back
        assert!(cursor.goto_parent(), "Should return to parent");
    }

    #[test]
    fn test_empty_source() {
        let source = "";
        let tree = FshGritTree::parse(source);

        let root = tree.root_node();
        assert_eq!(
            root.children().count(),
            0,
            "Empty source should have no children"
        );
    }

    #[test]
    fn test_trivia_preservation() {
        let source = r#"
// This is a comment
Profile: MyPatient  // Inline comment
  Parent: Patient
"#;
        let tree = FshGritTree::parse(source);

        // The CST should preserve all trivia
        let root = tree.root_node();
        let reconstructed = root.text().unwrap();
        assert_eq!(
            reconstructed.as_ref(),
            source,
            "Should preserve comments and whitespace"
        );
    }
}
