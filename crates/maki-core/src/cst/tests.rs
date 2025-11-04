//! Tests for CST construction and manipulation

use super::*;
use crate::cst::ast::{AstNode, Path};
use rowan::GreenNodeBuilder;

#[test]
fn path_segments_handle_keywords() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(FshSyntaxKind::Root.into());
    builder.start_node(FshSyntaxKind::Path.into());
    builder.token(FshSyntaxKind::True.into(), "true");
    builder.token(FshSyntaxKind::Dot.into(), ".");
    builder.token(FshSyntaxKind::AndKw.into(), "and");
    builder.finish_node();
    builder.finish_node();

    let green = builder.finish();
    let root = FshSyntaxNode::new_root(green);
    let path_node = root.first_child().expect("path node");
    let path = Path::cast(path_node).expect("cast to Path");

    assert_eq!(path.segments_as_strings(), vec!["true", "and"]);
}

#[test]
fn path_segments_preserve_brackets() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(FshSyntaxKind::Root.into());
    builder.start_node(FshSyntaxKind::Path.into());
    builder.token(FshSyntaxKind::Ident.into(), "extension");
    builder.token(FshSyntaxKind::LBracket.into(), "[");
    builder.token(FshSyntaxKind::Ident.into(), "slice");
    builder.token(FshSyntaxKind::RBracket.into(), "]");
    builder.finish_node();
    builder.finish_node();

    let green = builder.finish();
    let root = FshSyntaxNode::new_root(green);
    let path_node = root.first_child().expect("path node");
    let path = Path::cast(path_node).expect("cast to Path");

    assert_eq!(path.segments_as_strings(), vec!["extension[slice]"]);
}

/// Test basic CST construction and lossless property
#[test]
fn test_simple_profile_cst() {
    let mut builder = GreenNodeBuilder::new();

    // Build: Profile: MyPatient
    builder.start_node(FshSyntaxKind::Root.into());

    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.finish_node();

    builder.finish_node();

    let green = builder.finish();
    let root = FshSyntaxNode::new_root(green);

    // Verify structure
    assert_eq!(root.kind(), FshSyntaxKind::Root);

    // Find profile node
    let profile = root.first_child().expect("Should have profile child");
    assert_eq!(profile.kind(), FshSyntaxKind::Profile);

    // Verify lossless property
    assert_eq!(profile.text().to_string(), "Profile: MyPatient");
}

/// Test CST preserves comments
#[test]
fn test_cst_preserves_comments() {
    let mut builder = GreenNodeBuilder::new();

    // Build: Profile: MyPatient // comment
    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::CommentLine.into(), "// comment");
    builder.finish_node();

    let green = builder.finish();
    let profile = FshSyntaxNode::new_root(green);

    // Verify comment is preserved
    let text = profile.text().to_string();
    assert!(text.contains("// comment"));
    assert_eq!(text, "Profile: MyPatient // comment");
}

/// Test CST preserves multiple whitespace/newlines
#[test]
fn test_cst_preserves_whitespace() {
    let mut builder = GreenNodeBuilder::new();

    // Build: Profile:  MyPatient\n\n
    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), "  "); // Two spaces
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.token(FshSyntaxKind::Newline.into(), "\n");
    builder.token(FshSyntaxKind::Newline.into(), "\n");
    builder.finish_node();

    let green = builder.finish();
    let profile = FshSyntaxNode::new_root(green);

    // Verify exact whitespace is preserved
    assert_eq!(profile.text().to_string(), "Profile:  MyPatient\n\n");
}

/// Test traversing CST with children_with_tokens
#[test]
fn test_cst_traversal() {
    let mut builder = GreenNodeBuilder::new();

    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.finish_node();

    let green = builder.finish();
    let profile = FshSyntaxNode::new_root(green);

    let mut kinds = Vec::new();
    for element in profile.children_with_tokens() {
        if let Some(token) = element.as_token() {
            kinds.push(token.kind());
        }
    }

    assert_eq!(
        kinds,
        vec![
            FshSyntaxKind::ProfileKw,
            FshSyntaxKind::Colon,
            FshSyntaxKind::Whitespace,
            FshSyntaxKind::Ident,
        ]
    );
}

/// Test nested CST structure (Profile with rule)
#[test]
fn test_nested_cst() {
    let mut builder = GreenNodeBuilder::new();

    // Build: Profile: MyPatient\n* name 1..1 MS
    builder.start_node(FshSyntaxKind::Root.into());

    // Profile declaration
    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.token(FshSyntaxKind::Newline.into(), "\n");

    // Card rule
    builder.start_node(FshSyntaxKind::CardRule.into());
    builder.token(FshSyntaxKind::Asterisk.into(), "*");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "name");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");

    // Cardinality
    builder.start_node(FshSyntaxKind::Cardinality.into());
    builder.token(FshSyntaxKind::Integer.into(), "1");
    builder.token(FshSyntaxKind::Range.into(), "..");
    builder.token(FshSyntaxKind::Integer.into(), "1");
    builder.finish_node(); // CARDINALITY

    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::MsFlag.into(), "MS");
    builder.finish_node(); // CARD_RULE

    builder.finish_node(); // PROFILE
    builder.finish_node(); // ROOT

    let green = builder.finish();
    let root = FshSyntaxNode::new_root(green);

    // Verify structure
    assert_eq!(root.kind(), FshSyntaxKind::Root);

    let profile = root.first_child().expect("Should have profile");
    assert_eq!(profile.kind(), FshSyntaxKind::Profile);

    // Find card rule
    let card_rule = profile
        .children()
        .find(|n| n.kind() == FshSyntaxKind::CardRule)
        .expect("Should have card rule");

    // Find cardinality node
    let cardinality = card_rule
        .children()
        .find(|n| n.kind() == FshSyntaxKind::Cardinality)
        .expect("Should have cardinality");

    assert_eq!(cardinality.text().to_string(), "1..1");
}

/// Test FshSyntaxNodeExt helpers
#[test]
fn test_syntax_node_ext() {
    let mut builder = GreenNodeBuilder::new();

    builder.start_node(FshSyntaxKind::Profile.into());
    builder.token(FshSyntaxKind::ProfileKw.into(), "Profile");
    builder.token(FshSyntaxKind::Colon.into(), ":");
    builder.token(FshSyntaxKind::Whitespace.into(), " ");
    builder.token(FshSyntaxKind::Ident.into(), "MyPatient");
    builder.finish_node();

    let green = builder.finish();
    let profile = FshSyntaxNode::new_root(green);

    // Test is_kind
    assert!(profile.is_kind(FshSyntaxKind::Profile));

    // Test token_of_kind
    let profile_kw = profile.token_of_kind(FshSyntaxKind::ProfileKw);
    assert!(profile_kw.is_some());
    assert_eq!(profile_kw.unwrap().text(), "Profile");

    // Test trimmed_text
    assert_eq!(profile.trimmed_text(), "Profile: MyPatient");
}
