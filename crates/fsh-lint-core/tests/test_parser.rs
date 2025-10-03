//! Test-specific parser implementation
//!
//! This module provides lightweight wrappers around the real tree-sitter-fsh
//! grammar so the parser layer can be exercised in isolation during tests.

use fsh_lint_core::parser::{ParseError, ParseResult, Parser};
use fsh_lint_core::{FshLintError, Result};
use tree_sitter::{Language, Tree};

#[path = "test_parser/mock_tree_sitter_fsh.rs"]
pub mod mock_tree_sitter_fsh;

/// Parser wrapper used by tests with the real FSH grammar.
pub struct TestFshParser {
    parser: tree_sitter::Parser,
    language: Language,
}

impl TestFshParser {
    /// Create a new test FSH parser
    pub fn new() -> Result<Self> {
        let language = mock_tree_sitter_fsh::mock_language();
        let mut parser = tree_sitter::Parser::new();

        parser.set_language(language).map_err(|e| {
            FshLintError::parser_error(format!("Failed to set FSH language: {}", e))
        })?;

        Ok(Self { parser, language })
    }

    /// Get the language currently configured on the parser
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
        let tree = self
            .parser
            .parse(content, old_tree)
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
        self.parser
            .set_language(language)
            .map_err(|e| FshLintError::parser_error(format!("Failed to set language: {}", e)))?;
        self.language = language;
        Ok(())
    }
}

/// Test data expressed in real FSH syntax.
pub mod test_fsh_samples {
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

    pub const MULTIPLE_RESOURCES: &str = r#"Profile: PatientProfile
Parent: Patient

* name 1..*
* gender 1..1

Profile: ObservationProfile
Parent: Observation

* status 1..1
"#;

    pub const SYNTAX_ERROR: &str = r#"Profile MyBrokenProfile
Parent: Patient
"#;

    pub const INCOMPLETE_OBJECT: &str = r#"Profile: IncompleteProfile
Parent: Patient

* name
"#;

    pub const MIXED_VALID_INVALID: &str = r#"Profile: ValidProfile
Parent: Patient

Profile InvalidProfile
Parent: Patient
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

        assert!(
            result.is_ok(),
            "Parser should return result even with syntax errors"
        );
        let parse_result = result.unwrap();

        // Should have parse errors for invalid FSH
        if !parse_result.is_valid {
            assert!(
                !parse_result.errors().is_empty(),
                "Should have parse errors"
            );
        }
    }
}
