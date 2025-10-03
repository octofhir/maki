//! Tree-sitter FSH utilities for parser tests
//!
//! These helpers wrap the real `tree-sitter-fsh` grammar so the test
//! suite exercises the same language implementation used in production.

use fsh_lint_core::parser::ParseError;
use tree_sitter::{Language, Node, Parser as TreeSitterParser, Tree, TreeCursor};

/// Return the FSH grammar supplied by `tree-sitter-fsh`.
pub fn mock_language() -> Language {
    tree_sitter_fsh::language()
}

/// Create a parser configured with the FSH grammar.
pub fn create_mock_parser() -> Result<TreeSitterParser, tree_sitter::LanguageError> {
    let mut parser = TreeSitterParser::new();
    parser.set_language(mock_language())?;
    Ok(parser)
}

/// Well-formed FSH snippets used by unit tests.
#[allow(dead_code)]
pub mod mock_valid_content {
    pub const SIMPLE_PROFILE: &str = r#"Profile: MyPatientProfile
Parent: Patient
Id: my-patient-profile
Title: "My Patient Profile"
Description: "Example patient profile"

* name 1..1
* name.given 1..1
"#;

    pub const EXTENSION_DEFINITION: &str = r#"Extension: MySimpleExtension
Id: my-simple-extension
Title: "My Simple Extension"
Description: "Example extension"
Context: Patient
* value[x] only string
"#;
}

/// Ill-formed FSH snippets that should surface diagnostics.
#[allow(dead_code)]
pub mod mock_invalid_content {
    pub const MISSING_COLON: &str = r#"Profile MyBrokenProfile
Parent: Patient
"#;

    pub const INCOMPLETE_RULE: &str = r#"Profile: IncompleteProfile
Parent: Patient

* name
"#;

    pub const INVALID_SYNTAX: &str = r#"Profile: InvalidSyntax
Parent: Patient
* = value
"#;
}

/// Determine whether a node represents a parse error.
pub fn is_error_node(node: &Node) -> bool {
    node.is_error() || node.is_missing() || node.kind() == "ERROR"
}

/// Collect every error node within a tree.
pub fn collect_error_nodes(tree: &Tree) -> Vec<Node> {
    let mut errors = Vec::new();
    let mut cursor = tree.walk();
    collect_errors_recursive(&mut cursor, &mut errors);
    errors
}

fn collect_errors_recursive<'a>(cursor: &mut TreeCursor<'a>, errors: &mut Vec<Node<'a>>) {
    let node = cursor.node();

    if is_error_node(&node) {
        errors.push(node);
    }

    if cursor.goto_first_child() {
        loop {
            collect_errors_recursive(cursor, errors);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Convert error nodes into `ParseError` instances used by the parser API.
pub fn create_mock_parse_errors(tree: &Tree, _source: &str) -> Vec<ParseError> {
    let error_nodes = collect_error_nodes(tree);
    let mut errors = Vec::new();

    for node in error_nodes {
        let start_point = node.start_position();
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        let message = if node.is_missing() {
            format!("Missing {}", node.kind())
        } else {
            format!("Syntax error: unexpected {}", node.kind())
        };

        errors.push(ParseError::new(
            message,
            start_point.row,
            start_point.column,
            start_byte,
            end_byte.saturating_sub(start_byte),
        ));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_creation() {
        let language = mock_language();
        assert!(language.version() > 0);
    }

    #[test]
    fn test_parser_creation() {
        let parser = create_mock_parser();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parse_valid_content() {
        let mut parser = create_mock_parser().unwrap();
        let tree = parser.parse(mock_valid_content::SIMPLE_PROFILE, None);

        assert!(tree.is_some());
        let tree = tree.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_invalid_content() {
        let mut parser = create_mock_parser().unwrap();
        let tree = parser.parse(mock_invalid_content::MISSING_COLON, None);

        assert!(tree.is_some());
        let tree = tree.unwrap();
        let errors = collect_error_nodes(&tree);
        assert!(!errors.is_empty(), "Invalid content should surface errors");
    }
}
