//! Type aliases and utilities for FSH CST nodes
//!
//! This module provides convenient type aliases for working with FSH syntax trees.
//! These types are built on top of Rowan's generic tree types, parameterized with
//! our FshLanguage.

use super::{FshLanguage, FshSyntaxKind};

/// A node in the FSH concrete syntax tree
///
/// This is the main type for working with CST nodes. It provides:
/// - Access to children and parent nodes
/// - Text content reconstruction
/// - Token iteration
/// - Syntax kind querying
///
/// # Example
///
/// ```rust,ignore
/// use fsh_lint_core::cst::{FshSyntaxNode, FshSyntaxKind};
///
/// fn process_profile(node: &FshSyntaxNode) {
///     assert_eq!(node.kind(), FshSyntaxKind::Profile);
///
///     // Iterate over children
///     for child in node.children() {
///         println!("Child: {:?}", child.kind());
///     }
///
///     // Get text content (lossless!)
///     let source_text = node.text().to_string();
/// }
/// ```
// Newtype wrapper to impl Send/Sync
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FshSyntaxNode(rowan::SyntaxNode<FshLanguage>);

impl FshSyntaxNode {
    pub fn new(node: rowan::SyntaxNode<FshLanguage>) -> Self {
        Self(node)
    }

    pub fn new_root(green: rowan::GreenNode) -> Self {
        Self(rowan::SyntaxNode::new_root(green))
    }

    // Wrapper methods that return FshSyntaxNode instead of rowan::SyntaxNode
    pub fn parent(&self) -> Option<FshSyntaxNode> {
        self.0.parent().map(FshSyntaxNode::from)
    }

    pub fn children(&self) -> impl Iterator<Item = FshSyntaxNode> + '_ {
        self.0.children().map(FshSyntaxNode::from)
    }

    pub fn first_child(&self) -> Option<FshSyntaxNode> {
        self.0.first_child().map(FshSyntaxNode::from)
    }

    pub fn last_child(&self) -> Option<FshSyntaxNode> {
        self.0.last_child().map(FshSyntaxNode::from)
    }

    pub fn next_sibling(&self) -> Option<FshSyntaxNode> {
        self.0.next_sibling().map(FshSyntaxNode::from)
    }

    pub fn prev_sibling(&self) -> Option<FshSyntaxNode> {
        self.0.prev_sibling().map(FshSyntaxNode::from)
    }

    pub fn descendants(&self) -> impl Iterator<Item = FshSyntaxNode> + '_ {
        self.0.descendants().map(FshSyntaxNode::from)
    }

    pub fn ancestors(&self) -> impl Iterator<Item = FshSyntaxNode> + '_ {
        self.0.ancestors().map(FshSyntaxNode::from)
    }

    // Delegate other methods to inner node
    pub fn kind(&self) -> FshSyntaxKind {
        self.0.kind()
    }

    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }

    pub fn text(&self) -> rowan::SyntaxText {
        self.0.text()
    }

    pub fn children_with_tokens(&self) -> rowan::SyntaxElementChildren<FshLanguage> {
        self.0.children_with_tokens()
    }

    pub fn first_child_or_token(&self) -> Option<FshSyntaxElement> {
        self.0.first_child_or_token()
    }

    pub fn last_child_or_token(&self) -> Option<FshSyntaxElement> {
        self.0.last_child_or_token()
    }

    pub fn descendants_with_tokens(&self) -> impl Iterator<Item = FshSyntaxElement> + '_ {
        self.0.descendants_with_tokens()
    }

    pub fn siblings_with_tokens(
        &self,
        direction: Direction,
    ) -> impl Iterator<Item = FshSyntaxElement> + '_ {
        self.0.siblings_with_tokens(direction)
    }

    pub fn preorder(&self) -> impl Iterator<Item = WalkEvent<FshSyntaxNode>> + '_ {
        self.0
            .preorder()
            .map(|event| event.map(FshSyntaxNode::from))
    }

    pub fn preorder_with_tokens(&self) -> impl Iterator<Item = WalkEvent<FshSyntaxElement>> + '_ {
        self.0.preorder_with_tokens()
    }
}

impl std::ops::Deref for FshSyntaxNode {
    type Target = rowan::SyntaxNode<FshLanguage>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<rowan::SyntaxNode<FshLanguage>> for FshSyntaxNode {
    fn from(node: rowan::SyntaxNode<FshLanguage>) -> Self {
        Self(node)
    }
}

impl AsRef<rowan::SyntaxNode<FshLanguage>> for FshSyntaxNode {
    fn as_ref(&self) -> &rowan::SyntaxNode<FshLanguage> {
        &self.0
    }
}

// SAFETY: Rowan nodes are immutable after creation and use Arc internally
unsafe impl Send for FshSyntaxNode {}
unsafe impl Sync for FshSyntaxNode {}

/// A token in the FSH concrete syntax tree
///
/// Tokens are the leaf nodes of the tree and contain actual source text.
/// Unlike nodes, tokens cannot have children.
///
/// # Example
///
/// ```rust,ignore
/// use fsh_lint_core::cst::{FshSyntaxToken, FshSyntaxKind};
///
/// fn process_identifier(token: &FshSyntaxToken) {
///     assert_eq!(token.kind(), FshSyntaxKind::Ident);
///     println!("Identifier: {}", token.text());
/// }
/// ```
pub type FshSyntaxToken = rowan::SyntaxToken<FshLanguage>;

/// Either a node or a token in the CST
///
/// This type is used when iterating over all elements of a node,
/// including both child nodes and tokens.
///
/// # Example
///
/// ```rust,ignore
/// use fsh_lint_core::cst::{FshSyntaxElement, FshSyntaxKind};
/// use rowan::NodeOrToken;
///
/// fn process_elements(node: &FshSyntaxNode) {
///     for element in node.children_with_tokens() {
///         match element {
///             NodeOrToken::Node(n) => println!("Node: {:?}", n.kind()),
///             NodeOrToken::Token(t) => println!("Token: {:?} = {}", t.kind(), t.text()),
///         }
///     }
/// }
/// ```
pub type FshSyntaxElement = rowan::SyntaxElement<FshLanguage>;

/// Iterator over child nodes
pub type FshSyntaxNodeChildren = rowan::SyntaxNodeChildren<FshLanguage>;

/// Iterator over child nodes and tokens
pub type FshSyntaxElementChildren = rowan::SyntaxElementChildren<FshLanguage>;

// Note: SyntaxList is not available in Rowan 0.15
// We use iterators over children instead

// Re-export common rowan types for convenience
pub use rowan::{Direction, NodeOrToken, TextRange, TextSize, WalkEvent, ast::support};

/// Extension trait for FshSyntaxNode with FSH-specific helpers
pub trait FshSyntaxNodeExt {
    /// Check if this node matches the given kind
    fn is_kind(&self, kind: FshSyntaxKind) -> bool;

    /// Find the first child node of a specific kind
    fn child_of_kind(&self, kind: FshSyntaxKind) -> Option<FshSyntaxNode>;

    /// Find all child nodes of a specific kind
    fn children_of_kind(&self, kind: FshSyntaxKind) -> Vec<FshSyntaxNode>;

    /// Find the first child token of a specific kind
    fn token_of_kind(&self, kind: FshSyntaxKind) -> Option<FshSyntaxToken>;

    /// Get the text content without trivia
    fn trimmed_text(&self) -> String;
}

impl FshSyntaxNodeExt for FshSyntaxNode {
    fn is_kind(&self, kind: FshSyntaxKind) -> bool {
        self.kind() == kind
    }

    fn child_of_kind(&self, kind: FshSyntaxKind) -> Option<FshSyntaxNode> {
        self.children().find(|child| child.kind() == kind)
    }

    fn children_of_kind(&self, kind: FshSyntaxKind) -> Vec<FshSyntaxNode> {
        self.children()
            .filter(|child| child.kind() == kind)
            .collect()
    }

    fn token_of_kind(&self, kind: FshSyntaxKind) -> Option<FshSyntaxToken> {
        self.children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == kind)
    }

    fn trimmed_text(&self) -> String {
        self.text().to_string().trim().to_string()
    }
}

/// Extension trait for FshSyntaxToken with FSH-specific helpers
pub trait FshSyntaxTokenExt {
    /// Check if this token matches the given kind
    fn is_kind(&self, kind: FshSyntaxKind) -> bool;

    /// Check if this is a trivia token (whitespace, comment)
    fn is_trivia(&self) -> bool;

    /// Get the trimmed text (without surrounding whitespace)
    fn trimmed_text(&self) -> &str;
}

impl FshSyntaxTokenExt for FshSyntaxToken {
    fn is_kind(&self, kind: FshSyntaxKind) -> bool {
        self.kind() == kind
    }

    fn is_trivia(&self) -> bool {
        self.kind().is_trivia()
    }

    fn trimmed_text(&self) -> &str {
        self.text().trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowan::GreenNodeBuilder;

    /// Helper to create a simple CST for testing
    fn build_test_tree() -> FshSyntaxNode {
        let mut builder = GreenNodeBuilder::new();

        builder.start_node(FshSyntaxKind::Profile.into());
        builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
        builder.token(FshSyntaxKind::Colon.into(), ":");
        builder.token(FshSyntaxKind::Whitespace.into(), " ");
        builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
        builder.finish_node();

        FshSyntaxNode::new_root(builder.finish())
    }

    #[test]
    fn test_node_kind() {
        let tree = build_test_tree();
        assert_eq!(tree.kind(), FshSyntaxKind::Profile);
        assert!(tree.is_kind(FshSyntaxKind::Profile));
        assert!(!tree.is_kind(FshSyntaxKind::Extension));
    }

    #[test]
    fn test_token_extraction() {
        let tree = build_test_tree();

        let profile_kw = tree.token_of_kind(FshSyntaxKind::ProfileKw);
        assert!(profile_kw.is_some());
        assert_eq!(profile_kw.unwrap().text(), "Profile");

        let ident = tree.token_of_kind(FshSyntaxKind::Ident);
        assert!(ident.is_some());
        assert_eq!(ident.unwrap().text(), "MyPatient");
    }

    #[test]
    fn test_text_reconstruction() {
        let tree = build_test_tree();
        // Lossless property: we can reconstruct the source
        assert_eq!(tree.text().to_string(), "Profile: MyPatient");
    }

    #[test]
    fn test_trimmed_text() {
        let tree = build_test_tree();
        // trimmed_text should remove leading/trailing whitespace
        let trimmed = tree.trimmed_text();
        assert_eq!(trimmed, "Profile: MyPatient");
    }

    #[test]
    fn test_children_iteration() {
        let tree = build_test_tree();
        let mut token_count = 0;

        for element in tree.children_with_tokens() {
            if element.as_token().is_some() {
                token_count += 1;
            }
        }

        // Should have: PROFILE_KW, COLON, WHITESPACE, IDENT
        assert_eq!(token_count, 4);
    }

    #[test]
    fn test_trivia_detection() {
        let tree = build_test_tree();

        let whitespace = tree.token_of_kind(FshSyntaxKind::Whitespace);
        assert!(whitespace.is_some());
        assert!(whitespace.unwrap().is_trivia());

        let profile_kw = tree.token_of_kind(FshSyntaxKind::ProfileKw);
        assert!(profile_kw.is_some());
        assert!(!profile_kw.unwrap().is_trivia());
    }
}
