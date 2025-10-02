//! Test-specific parser implementation
//! 
//! This module provides parser implementations that can be used for testing
//! when the actual tree-sitter-fsh might not be available or working.

use fsh_lint_core::parser::{ParseError, ParseResult, Parser};
use fsh_lint_core::{FshLintError, Result};
use tree_sitter::{Language, Tree};

pub mod mock_tree_sitter_fsh;

/// Test parser that uses a mock FSH language for testing
pub struct TestFshParser {
    parser: tree_sitter::Parser,
    language: Language,
}

impl TestFshParser {
    /// Create a new test FSH parser
    pub fn new() -> Result<Self> {
        let language = mock_tree_sitter_fsh::mock_language();
        let mut parser = tree_sitter::Parser::new();
        
        parser.set_language(language)
            .map_err(|e| FshLintError::parser_error(format!("Failed to set mock FSH language: {}", e)))?;
        
        Ok(Self { parser, language })
    }
    
    /// Get the mock FSH language
    pub fn language(&self) -> Language {
        self.language
    }
    
    /// Extract parse errors from the syntax tree
    fn extract_errors(&self, tree: &Tree, source: &str) -> Vec<ParseError> {
        mock_tree_sitter_fsh::create_mock_parse_errors(tree, source)
    }
}

impl Parser for TestFshParser {
    fn parse(&mut self, content: &str, old_tree: Option<&Tree>) -> Result<ParseResult> {
        let tree = self.parser.parse(content, old_tree)
            .ok_or_else(|| FshLintError::parser_error("Failed to parse FSH content".to_string()))?;
        
        let errors = self.extract_errors(&tree, content);
        let is_valid = errors.is_empty();
        
        Ok(ParseResult {
            tree,
            errors,
            is_valid,
            source: content.to_string(),
        })
    }
    
    fn parse_incremental(&mut self, content: &str, old_tree: &Tree) -> Result<ParseResult> {
        self.parse(content, Some(old_tree))
    }
    
    fn set_language(&mut self, language: Language) -> Result<()> {
        self.parser.set_language(language)
            .map_err(|e| FshLintError::parser_error(format!("Failed to set language: {}", e)))?;
        self.language = language;
        Ok(())
    }
}

/// Test data that works with our mock parser (JSON-like syntax)
pub mod test_fsh_samples {
    pub const SIMPLE_PROFILE: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient",
  "name": "MyPatient",
  "parent": "Patient",
  "elements": [
    {
      "path": "name",
      "cardinality": "1..1",
      "mustSupport": true
    },
    {
      "path": "name.family", 
      "cardinality": "1..1"
    }
  ]
}
"#;

    pub const EXTENSION_DEFINITION: &str = r#"
{
  "resourceType": "Extension",
  "id": "my-extension",
  "name": "MyExtension",
  "context": [
    {
      "type": "element",
      "expression": "Patient"
    }
  ],
  "valueType": "string"
}
"#;

    pub const MULTIPLE_RESOURCES: &str = r#"
[
  {
    "resourceType": "Profile",
    "id": "patient-profile",
    "name": "PatientProfile",
    "parent": "Patient"
  },
  {
    "resourceType": "Extension",
    "id": "patient-extension",
    "name": "PatientExtension"
  }
]
"#;

    pub const SYNTAX_ERROR: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient"
  "name": "MyPatient"
}
"#;

    pub const INCOMPLETE_OBJECT: &str = r#"
{
  "resourceType": "Profile",
  "id": "my-patient"
"#;

    pub const MIXED_VALID_INVALID: &str = r#"
[
  {
    "resourceType": "Profile",
    "id": "valid-profile",
    "name": "ValidProfile"
  },
  {
    "resourceType": "Profile"
    "id": "invalid-profile"
  }
]
"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parser_creation() {
        let parser = TestFshParser::new();
        assert!(parser.is_ok(), "Should be able to create test FSH parser");
    }
    
    #[test]
    fn test_parse_valid_content() {
        let mut parser = TestFshParser::new().unwrap();
        let result = parser.parse(test_fsh_samples::SIMPLE_PROFILE, None);
        
        assert!(result.is_ok(), "Should be able to parse valid content");
        let parse_result = result.unwrap();
        assert_eq!(parse_result.source(), test_fsh_samples::SIMPLE_PROFILE);
        
        let root = parse_result.root_node();
        assert!(!root.is_error(), "Root node should not be an error");
    }
    
    #[test]
    fn test_parse_invalid_content() {
        let mut parser = TestFshParser::new().unwrap();
        let result = parser.parse(test_fsh_samples::SYNTAX_ERROR, None);
        
        assert!(result.is_ok(), "Parser should return result even with syntax errors");
        let parse_result = result.unwrap();
        
        // Should have parse errors for invalid JSON
        if !parse_result.is_valid {
            assert!(!parse_result.errors().is_empty(), "Should have parse errors");
        }
    }
}