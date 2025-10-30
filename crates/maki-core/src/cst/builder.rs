//! CST builder for constructing green trees from tokens
//!
//! This module provides a high-level API for building CST nodes from lexer tokens.

use rowan::GreenNodeBuilder;

use super::{CstToken, FshSyntaxKind, FshSyntaxNode};

/// Builder for constructing CST trees
///
/// This is a thin wrapper around Rowan's `GreenNodeBuilder` that provides
/// a more convenient API for building FSH syntax trees from tokens.
///
/// # Example
///
/// ```rust,ignore
/// use maki_core::cst::{CstBuilder, FshSyntaxKind};
///
/// let mut builder = CstBuilder::new();
///
/// // Build: Profile: MyPatient
/// builder.start_node(FshSyntaxKind::Profile);
/// builder.token(FshSyntaxKind::ProfileKw, "Profile");
/// builder.token(FshSyntaxKind::Colon, ":");
/// builder.token(FshSyntaxKind::Whitespace, " ");
/// builder.token(FshSyntaxKind::Ident, "MyPatient");
/// builder.finish_node();
///
/// let cst = builder.finish();
/// assert_eq!(cst.text().to_string(), "Profile: MyPatient");
/// ```
pub struct CstBuilder {
    builder: GreenNodeBuilder<'static>,
}

impl CstBuilder {
    /// Create a new CST builder
    pub fn new() -> Self {
        Self {
            builder: GreenNodeBuilder::new(),
        }
    }

    /// Start a new node with the given kind
    ///
    /// Must be matched with a call to `finish_node()`.
    pub fn start_node(&mut self, kind: FshSyntaxKind) {
        self.builder.start_node(kind.into());
    }

    /// Add a token with the given kind and text
    pub fn token(&mut self, kind: FshSyntaxKind, text: &str) {
        self.builder.token(kind.into(), text);
    }

    /// Add a token from a CstToken
    pub fn add_token(&mut self, token: &CstToken) {
        self.builder.token(token.kind.into(), &token.text);
    }

    /// Finish the current node
    ///
    /// Must match a previous call to `start_node()`.
    pub fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Finish building and return the root syntax node
    pub fn finish(self) -> FshSyntaxNode {
        let green = self.builder.finish();
        FshSyntaxNode::new_root(green)
    }

    /// Build a simple token node (convenience method)
    ///
    /// Creates a node containing a single token.
    pub fn token_node(&mut self, node_kind: FshSyntaxKind, token_kind: FshSyntaxKind, text: &str) {
        self.start_node(node_kind);
        self.token(token_kind, text);
        self.finish_node();
    }
}

impl Default for CstBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a CST from a list of tokens
///
/// This is a higher-level function that automatically wraps tokens in a ROOT node.
///
/// # Example
///
/// ```rust,ignore
/// use maki_core::cst::{lex_with_trivia, build_cst_from_tokens};
///
/// let source = "Profile: MyPatient";
/// let (tokens, _) = lex_with_trivia(source);
/// let cst = build_cst_from_tokens(&tokens);
///
/// assert_eq!(cst.text().to_string(), source);
/// ```
pub fn build_cst_from_tokens(tokens: &[CstToken]) -> FshSyntaxNode {
    let mut builder = CstBuilder::new();

    builder.start_node(FshSyntaxKind::Root);

    for token in tokens {
        // Skip EOF token
        if token.kind == FshSyntaxKind::Eof {
            continue;
        }
        builder.add_token(token);
    }

    builder.finish_node();
    builder.finish()
}

/// Parse FSH source to CST (simple flat version)
///
/// This is a basic parser that creates a flat CST with all tokens directly under ROOT.
/// A more sophisticated parser will create proper hierarchical structure.
///
/// # Example
///
/// ```rust,ignore
/// use maki_core::cst::parse_fsh_simple;
///
/// let source = "Profile: MyPatient // comment\nParent: Patient";
/// let (cst, errors) = parse_fsh_simple(source);
///
/// // Verify lossless property
/// assert_eq!(cst.text().to_string(), source);
/// assert!(errors.is_empty());
/// ```
pub fn parse_fsh_simple(source: &str) -> (FshSyntaxNode, Vec<super::lexer::LexerError>) {
    let (tokens, errors) = super::lex_with_trivia(source);
    let cst = build_cst_from_tokens(&tokens);
    (cst, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut builder = CstBuilder::new();

        builder.start_node(FshSyntaxKind::Profile);
        builder.token(FshSyntaxKind::ProfileKw, "Profile");
        builder.token(FshSyntaxKind::Colon, ":");
        builder.token(FshSyntaxKind::Whitespace, " ");
        builder.token(FshSyntaxKind::Ident, "MyPatient");
        builder.finish_node();

        let node = builder.finish();
        assert_eq!(node.kind(), FshSyntaxKind::Profile);
        assert_eq!(node.text().to_string(), "Profile: MyPatient");
    }

    #[test]
    fn test_builder_nested() {
        let mut builder = CstBuilder::new();

        // Root node
        builder.start_node(FshSyntaxKind::Root);

        // Profile node
        builder.start_node(FshSyntaxKind::Profile);
        builder.token(FshSyntaxKind::ProfileKw, "Profile");
        builder.token(FshSyntaxKind::Colon, ":");
        builder.token(FshSyntaxKind::Whitespace, " ");
        builder.token(FshSyntaxKind::Ident, "MyPatient");
        builder.finish_node();

        builder.finish_node(); // ROOT

        let root = builder.finish();
        assert_eq!(root.kind(), FshSyntaxKind::Root);

        // Find profile child
        let profile = root.first_child().expect("Should have profile child");
        assert_eq!(profile.kind(), FshSyntaxKind::Profile);
        assert_eq!(profile.text().to_string(), "Profile: MyPatient");
    }

    #[test]
    fn test_build_from_tokens() {
        let tokens = vec![
            CstToken::new(FshSyntaxKind::ProfileKw, "Profile", Default::default()),
            CstToken::new(FshSyntaxKind::Colon, ":", Default::default()),
            CstToken::new(FshSyntaxKind::Whitespace, " ", Default::default()),
            CstToken::new(FshSyntaxKind::Ident, "MyPatient", Default::default()),
            CstToken::new(FshSyntaxKind::Eof, "", Default::default()),
        ];

        let cst = build_cst_from_tokens(&tokens);

        assert_eq!(cst.kind(), FshSyntaxKind::Root);
        assert_eq!(cst.text().to_string(), "Profile: MyPatient");
    }

    #[test]
    fn test_parse_simple_lossless() {
        let source = "Profile: MyPatient";
        let (cst, errors) = parse_fsh_simple(source);

        // Should have no errors
        assert!(errors.is_empty());

        // Verify lossless property
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_parse_simple_with_comment() {
        let source = "Profile: MyPatient // comment";
        let (cst, errors) = parse_fsh_simple(source);

        assert!(errors.is_empty());

        // Comment should be preserved
        assert_eq!(cst.text().to_string(), source);
        assert!(cst.text().to_string().contains("// comment"));
    }

    #[test]
    fn test_parse_simple_multiline() {
        let source = "Profile: MyPatient\nParent: Patient";
        let (cst, errors) = parse_fsh_simple(source);

        assert!(errors.is_empty());

        // Newline should be preserved
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_parse_simple_with_whitespace() {
        let source = "Profile:  MyPatient"; // Two spaces
        let (cst, errors) = parse_fsh_simple(source);

        assert!(errors.is_empty());

        // Extra space should be preserved
        assert_eq!(cst.text().to_string(), source);
    }

    #[test]
    fn test_token_node_convenience() {
        let mut builder = CstBuilder::new();

        builder.start_node(FshSyntaxKind::Root);
        builder.token_node(FshSyntaxKind::Profile, FshSyntaxKind::ProfileKw, "Profile");
        builder.finish_node();

        let root = builder.finish();
        let profile = root.first_child().unwrap();

        assert_eq!(profile.kind(), FshSyntaxKind::Profile);
        assert_eq!(profile.text().to_string(), "Profile");
    }

    #[test]
    fn test_complex_roundtrip() {
        let source = r#"Profile:  MyPatient // Important
Parent: Patient

* name 1..1 MS"#;

        let (cst, errors) = parse_fsh_simple(source);

        // No lexer errors
        assert!(errors.is_empty());

        // Perfect roundtrip
        assert_eq!(cst.text().to_string(), source);
    }
}
