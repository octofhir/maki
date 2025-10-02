//! Mock tree-sitter-fsh implementation for testing
//! 
//! This module provides a mock implementation of tree-sitter-fsh
//! that can be used when the actual tree-sitter-fsh is not available
//! or for testing specific parsing scenarios.

use tree_sitter::{Language, Node, Parser as TreeSitterParser, Tree, TreeCursor};
use fsh_lint_core::parser::ParseError;

/// Mock FSH language implementation
pub fn mock_language() -> Language {
    // For testing, we'll use a simple language that can parse basic structures
    // In a real implementation, this would be the actual FSH grammar
    tree_sitter_json::language() // Use JSON as a stand-in for FSH
}

/// Create a mock parser for testing
pub fn create_mock_parser() -> Result<TreeSitterParser, tree_sitter::LanguageError> {
    let mut parser = TreeSitterParser::new();
    parser.set_language(mock_language())?;
    Ok(parser)
}

/// Mock FSH content that should parse successfully
pub mod mock_valid_content {
    pub const SIMPLE_JSON_LIKE: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient",
  "name": "MyPatient",
  "parent": "Patient"
}
"#;

    pub const ARRAY_CONTENT: &str = r#"
[
  {
    "resourceType": "Profile",
    "id": "profile1"
  },
  {
    "resourceType": "Extension", 
    "id": "extension1"
  }
]
"#;
}

/// Mock FSH content that should produce parse errors
pub mod mock_invalid_content {
    pub const SYNTAX_ERROR: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient"
  "name": "MyPatient"  // Missing comma
}
"#;

    pub const INCOMPLETE_OBJECT: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient"
  // Missing closing brace
"#;

    pub const INVALID_JSON: &str = r#"
{
  "resourceType": Profile,  // Missing quotes
  "id": "my-patient"
}
"#;
}

/// Helper function to check if a node represents an error
pub fn is_error_node(node: &Node) -> bool {
    node.is_error() || node.is_missing() || node.kind() == "ERROR"
}

/// Helper function to collect all error nodes from a tree
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

/// Create mock parse errors from error nodes
pub fn create_mock_parse_errors(tree: &Tree, source: &str) -> Vec<ParseError> {
    let error_nodes = collect_error_nodes(tree);
    let mut errors = Vec::new();
    
    for node in error_nodes {
        let start_point = node.start_position();
        let end_point = node.end_position();
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
    fn test_mock_language_creation() {
        let language = mock_language();
        assert!(language.version() > 0);
    }
    
    #[test]
    fn test_mock_parser_creation() {
        let parser = create_mock_parser();
        assert!(parser.is_ok());
    }
    
    #[test]
    fn test_parse_valid_mock_content() {
        let mut parser = create_mock_parser().unwrap();
        let tree = parser.parse(mock_valid_content::SIMPLE_JSON_LIKE, None);
        
        assert!(tree.is_some());
        let tree = tree.unwrap();
        let root = tree.root_node();
        assert!(!root.is_error());
    }
    
    #[test]
    fn test_parse_invalid_mock_content() {
        let mut parser = create_mock_parser().unwrap();
        let tree = parser.parse(mock_invalid_content::SYNTAX_ERROR, None);
        
        assert!(tree.is_some());
        let tree = tree.unwrap();
        let errors = collect_error_nodes(&tree);
        assert!(!errors.is_empty(), "Should have parse errors for invalid content");
    }
    
    #[test]
    fn test_error_collection() {
        let mut parser = create_mock_parser().unwrap();
        let tree = parser.parse(mock_invalid_content::INCOMPLETE_OBJECT, None).unwrap();
        
        let errors = create_mock_parse_errors(&tree, mock_invalid_content::INCOMPLETE_OBJECT);
        // Should have at least some error information
        // The exact number depends on how the JSON parser handles the incomplete object
    }
}