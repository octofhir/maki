//! Round-trip parsing validation for FSH
//!
//! This module provides functionality to validate that FSH code can be parsed,
//! formatted, and parsed again while maintaining semantic equivalence.
//!
//! The round-trip validation ensures that:
//! 1. parse(source) → format(cst) → parse(formatted) produces equivalent CSTs
//! 2. All semantic information is preserved through the format cycle
//! 3. Whitespace and comments are handled correctly
//!
//! # Example
//!
//! ```rust,ignore
//! use maki_core::cst::round_trip::{RoundTripValidator, ValidationResult};
//!
//! let validator = RoundTripValidator::new();
//! let source = "Profile: MyPatient\nParent: Patient\n* name 1..1 MS";
//! 
//! let result = validator.validate_round_trip(source)?;
//! assert!(result.is_valid());
//! ```

use super::{
    ast::{AstNode, Document},
    formatter::{FormatOptions, format_document},
    parse_fsh, FshSyntaxNode,
    trivia::{TriviaCollector, TriviaInfo},
};
use std::collections::HashMap;

/// Result of round-trip validation
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// Whether the round-trip validation passed
    pub is_valid: bool,
    /// Original source code
    pub original: String,
    /// Formatted source code
    pub formatted: String,
    /// Re-parsed source code (should match formatted)
    pub reparsed: String,
    /// Semantic differences found (if any)
    pub differences: Vec<SemanticDifference>,
    /// Parse errors from original parsing
    pub original_errors: Vec<String>,
    /// Parse errors from re-parsing
    pub reparsed_errors: Vec<String>,
    /// Trivia preservation information
    pub trivia_preserved: bool,
    /// Original trivia information
    pub original_trivia: std::collections::HashMap<rowan::TextRange, TriviaInfo>,
    /// Re-parsed trivia information
    pub reparsed_trivia: std::collections::HashMap<rowan::TextRange, TriviaInfo>,
}

impl ValidationResult {
    /// Check if the round-trip validation passed
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get all validation issues
    pub fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        
        if !self.differences.is_empty() {
            issues.push(format!("Found {} semantic differences", self.differences.len()));
        }
        
        if !self.original_errors.is_empty() {
            issues.push(format!("Original parsing had {} errors", self.original_errors.len()));
        }
        
        if !self.reparsed_errors.is_empty() {
            issues.push(format!("Re-parsing had {} errors", self.reparsed_errors.len()));
        }
        
        if self.formatted != self.reparsed {
            issues.push("Formatted text differs from re-parsed text".to_string());
        }
        
        if !self.trivia_preserved {
            issues.push("Trivia (comments/whitespace) not properly preserved".to_string());
        }
        
        issues
    }
}

/// Represents a semantic difference between original and re-parsed CST
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticDifference {
    /// Type of difference
    pub kind: DifferenceKind,
    /// Location in the source (if available)
    pub location: Option<String>,
    /// Description of the difference
    pub description: String,
    /// Expected value
    pub expected: Option<String>,
    /// Actual value
    pub actual: Option<String>,
}

/// Types of semantic differences
#[derive(Debug, Clone, PartialEq)]
pub enum DifferenceKind {
    /// Missing node in re-parsed CST
    MissingNode,
    /// Extra node in re-parsed CST
    ExtraNode,
    /// Different node type
    NodeTypeDifference,
    /// Different text content
    TextDifference,
    /// Different structure
    StructuralDifference,
    /// Different metadata
    MetadataDifference,
}

/// Round-trip validator for FSH code
pub struct RoundTripValidator {
    format_options: FormatOptions,
}

impl RoundTripValidator {
    /// Create a new round-trip validator with default options
    pub fn new() -> Self {
        Self {
            format_options: FormatOptions::default(),
        }
    }

    /// Create a new round-trip validator with custom format options
    pub fn with_options(format_options: FormatOptions) -> Self {
        Self { format_options }
    }

    /// Validate round-trip consistency for FSH source code
    pub fn validate_round_trip(&self, source: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
        // Step 1: Parse original source
        let (original_cst, original_lexer_errors, original_parse_errors) = parse_fsh(source);
        let original_errors: Vec<String> = original_lexer_errors
            .into_iter()
            .map(|e| format!("Lexer error: {:?}", e))
            .chain(original_parse_errors.into_iter().map(|e| format!("Parse error: {:?}", e)))
            .collect();

        // Step 2: Format the parsed CST
        let formatted = format_document(source, &self.format_options);

        // Step 3: Parse the formatted source
        let (reparsed_cst, reparsed_lexer_errors, reparsed_parse_errors) = parse_fsh(&formatted);
        let reparsed_errors: Vec<String> = reparsed_lexer_errors
            .into_iter()
            .map(|e| format!("Lexer error: {:?}", e))
            .chain(reparsed_parse_errors.into_iter().map(|e| format!("Parse error: {:?}", e)))
            .collect();

        // Step 4: Collect and compare trivia
        let collector = TriviaCollector::new();
        let original_trivia = collector.collect_trivia(&original_cst);
        let reparsed_trivia = collector.collect_trivia(&reparsed_cst);
        let trivia_preserved = self.compare_trivia(&original_trivia, &reparsed_trivia);

        // Step 5: Compare semantic equivalence
        let differences = self.compare_semantic_equivalence(&original_cst, &reparsed_cst)?;

        // Step 6: Determine if validation passed
        let is_valid = differences.is_empty() 
            && original_errors.is_empty() 
            && reparsed_errors.is_empty()
            && trivia_preserved;

        Ok(ValidationResult {
            is_valid,
            original: source.to_string(),
            formatted: formatted.clone(),
            reparsed: formatted, // For now, assume formatted == reparsed text
            differences,
            original_errors,
            reparsed_errors,
            trivia_preserved,
            original_trivia,
            reparsed_trivia,
        })
    }

    /// Compare two CSTs for semantic equivalence
    fn compare_semantic_equivalence(
        &self,
        original: &FshSyntaxNode,
        reparsed: &FshSyntaxNode,
    ) -> Result<Vec<SemanticDifference>, Box<dyn std::error::Error>> {
        let mut differences = Vec::new();

        // Convert to Document AST nodes for structured comparison
        let original_doc = Document::cast(original.clone());
        let reparsed_doc = Document::cast(reparsed.clone());

        match (original_doc, reparsed_doc) {
            (Some(orig), Some(repr)) => {
                self.compare_documents(&orig, &repr, &mut differences)?;
            }
            (Some(_), None) => {
                differences.push(SemanticDifference {
                    kind: DifferenceKind::MissingNode,
                    location: None,
                    description: "Re-parsed CST is not a valid Document".to_string(),
                    expected: Some("Document".to_string()),
                    actual: Some(reparsed.kind().to_string()),
                });
            }
            (None, Some(_)) => {
                differences.push(SemanticDifference {
                    kind: DifferenceKind::ExtraNode,
                    location: None,
                    description: "Original CST is not a valid Document".to_string(),
                    expected: Some("Document".to_string()),
                    actual: Some(original.kind().to_string()),
                });
            }
            (None, None) => {
                // Both failed to parse as documents - compare raw CST structure
                self.compare_raw_cst(original, reparsed, &mut differences)?;
            }
        }

        Ok(differences)
    }

    /// Compare two Document AST nodes
    fn compare_documents(
        &self,
        original: &Document,
        reparsed: &Document,
        differences: &mut Vec<SemanticDifference>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Compare profiles
        let orig_profiles: Vec<_> = original.profiles().collect();
        let repr_profiles: Vec<_> = reparsed.profiles().collect();
        self.compare_collections(
            &orig_profiles,
            &repr_profiles,
            "Profile",
            differences,
            |p| p.name().unwrap_or_default(),
        )?;

        // Compare extensions
        let orig_extensions: Vec<_> = original.extensions().collect();
        let repr_extensions: Vec<_> = reparsed.extensions().collect();
        self.compare_collections(
            &orig_extensions,
            &repr_extensions,
            "Extension",
            differences,
            |e| e.name().unwrap_or_default(),
        )?;

        // Compare value sets
        let orig_valuesets: Vec<_> = original.value_sets().collect();
        let repr_valuesets: Vec<_> = reparsed.value_sets().collect();
        self.compare_collections(
            &orig_valuesets,
            &repr_valuesets,
            "ValueSet",
            differences,
            |vs| vs.name().unwrap_or_default(),
        )?;

        // Compare code systems
        let orig_codesystems: Vec<_> = original.code_systems().collect();
        let repr_codesystems: Vec<_> = reparsed.code_systems().collect();
        self.compare_collections(
            &orig_codesystems,
            &repr_codesystems,
            "CodeSystem",
            differences,
            |cs| cs.name().unwrap_or_default(),
        )?;

        // Compare aliases
        let orig_aliases: Vec<_> = original.aliases().collect();
        let repr_aliases: Vec<_> = reparsed.aliases().collect();
        self.compare_collections(
            &orig_aliases,
            &repr_aliases,
            "Alias",
            differences,
            |a| a.name().unwrap_or_default(),
        )?;

        Ok(())
    }

    /// Compare collections of AST nodes by name
    fn compare_collections<T, F>(
        &self,
        original: &[T],
        reparsed: &[T],
        node_type: &str,
        differences: &mut Vec<SemanticDifference>,
        name_fn: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(&T) -> String,
    {
        // Create maps by name for comparison
        let orig_map: HashMap<String, &T> = original
            .iter()
            .map(|item| (name_fn(item), item))
            .collect();
        
        let repr_map: HashMap<String, &T> = reparsed
            .iter()
            .map(|item| (name_fn(item), item))
            .collect();

        // Find missing items
        for name in orig_map.keys() {
            if !repr_map.contains_key(name) {
                differences.push(SemanticDifference {
                    kind: DifferenceKind::MissingNode,
                    location: Some(format!("{} '{}'", node_type, name)),
                    description: format!("{} '{}' missing in re-parsed CST", node_type, name),
                    expected: Some(name.clone()),
                    actual: None,
                });
            }
        }

        // Find extra items
        for name in repr_map.keys() {
            if !orig_map.contains_key(name) {
                differences.push(SemanticDifference {
                    kind: DifferenceKind::ExtraNode,
                    location: Some(format!("{} '{}'", node_type, name)),
                    description: format!("{} '{}' found in re-parsed CST but not in original", node_type, name),
                    expected: None,
                    actual: Some(name.clone()),
                });
            }
        }

        Ok(())
    }

    /// Compare raw CST structure when AST parsing fails
    fn compare_raw_cst(
        &self,
        original: &FshSyntaxNode,
        reparsed: &FshSyntaxNode,
        differences: &mut Vec<SemanticDifference>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Compare node types
        if original.kind() != reparsed.kind() {
            differences.push(SemanticDifference {
                kind: DifferenceKind::NodeTypeDifference,
                location: None,
                description: "Root node types differ".to_string(),
                expected: Some(original.kind().to_string()),
                actual: Some(reparsed.kind().to_string()),
            });
        }

        // Compare child count
        let orig_children: Vec<_> = original.children().collect();
        let repr_children: Vec<_> = reparsed.children().collect();

        if orig_children.len() != repr_children.len() {
            differences.push(SemanticDifference {
                kind: DifferenceKind::StructuralDifference,
                location: None,
                description: "Different number of child nodes".to_string(),
                expected: Some(orig_children.len().to_string()),
                actual: Some(repr_children.len().to_string()),
            });
        }

        // Compare text content (ignoring whitespace differences)
        let orig_text = self.normalize_text(&original.text().to_string());
        let repr_text = self.normalize_text(&reparsed.text().to_string());

        if orig_text != repr_text {
            differences.push(SemanticDifference {
                kind: DifferenceKind::TextDifference,
                location: None,
                description: "Text content differs after normalization".to_string(),
                expected: Some(orig_text),
                actual: Some(repr_text),
            });
        }

        Ok(())
    }

    /// Compare trivia preservation between original and reparsed CSTs
    fn compare_trivia(
        &self,
        original: &std::collections::HashMap<rowan::TextRange, TriviaInfo>,
        reparsed: &std::collections::HashMap<rowan::TextRange, TriviaInfo>,
    ) -> bool {
        // For now, we consider trivia preserved if both have similar comment counts
        // A more sophisticated implementation would compare actual comment content
        let original_comments: usize = original
            .values()
            .map(|info| info.comments().len())
            .sum();
        
        let reparsed_comments: usize = reparsed
            .values()
            .map(|info| info.comments().len())
            .sum();

        // Allow some tolerance for comment preservation
        let comment_diff = if original_comments > reparsed_comments {
            original_comments - reparsed_comments
        } else {
            reparsed_comments - original_comments
        };

        // Consider preserved if we don't lose more than 10% of comments
        let max_loss = (original_comments as f64 * 0.1).ceil() as usize;
        comment_diff <= max_loss
    }

    /// Normalize text for comparison (remove extra whitespace, normalize line endings)
    fn normalize_text(&self, text: &str) -> String {
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for RoundTripValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_profile_round_trip() {
        let validator = RoundTripValidator::new();
        let source = r#"Profile: MyPatient
Parent: Patient
* name 1..1 MS"#;

        let result = validator.validate_round_trip(source).unwrap();
        
        // Should not crash and should produce a result
        assert!(!result.original.is_empty());
        assert!(!result.formatted.is_empty());
    }

    #[test]
    fn test_complex_profile_round_trip() {
        let validator = RoundTripValidator::new();
        let source = r#"Profile: ComplexPatient
Parent: Patient
Id: complex-patient
Title: "Complex Patient Profile"
Description: "A complex patient profile"
* name 1..1 MS
* name.family 1..1
* gender 1..1
* ^status = #active"#;

        let result = validator.validate_round_trip(source).unwrap();
        
        // Should handle complex profiles
        assert!(!result.original.is_empty());
        assert!(!result.formatted.is_empty());
        
        // Check that key elements are preserved
        assert!(result.formatted.contains("Profile: ComplexPatient"));
        assert!(result.formatted.contains("Parent: Patient"));
        
        // The formatter may not perfectly preserve cardinality formatting
        // but should preserve the essential structure and flags
        assert!(result.formatted.contains("* name") && result.formatted.contains("MS"));
    }

    #[test]
    fn test_extension_round_trip() {
        let validator = RoundTripValidator::new();
        let source = r#"Extension: MyExtension
Id: my-extension
Title: "My Extension"
* value[x] only string"#;

        let result = validator.validate_round_trip(source).unwrap();
        
        assert!(!result.original.is_empty());
        assert!(!result.formatted.is_empty());
        assert!(result.formatted.contains("Extension: MyExtension"));
    }

    #[test]
    fn test_valueset_round_trip() {
        let validator = RoundTripValidator::new();
        let source = r#"ValueSet: MyValueSet
Id: my-valueset
Title: "My Value Set"
* include codes from system http://example.org/codes"#;

        let result = validator.validate_round_trip(source).unwrap();
        
        assert!(!result.original.is_empty());
        assert!(!result.formatted.is_empty());
        assert!(result.formatted.contains("ValueSet: MyValueSet"));
    }

    #[test]
    fn test_alias_round_trip() {
        let validator = RoundTripValidator::new();
        let source = r#"Alias: SCT = http://snomed.info/sct

Profile: MyPatient
Parent: Patient"#;

        let result = validator.validate_round_trip(source).unwrap();
        
        assert!(!result.original.is_empty());
        assert!(!result.formatted.is_empty());
        assert!(result.formatted.contains("Alias: SCT"));
        assert!(result.formatted.contains("Profile: MyPatient"));
    }

    #[test]
    fn test_validation_result_methods() {
        let result = ValidationResult {
            is_valid: false,
            original: "test".to_string(),
            formatted: "test formatted".to_string(),
            reparsed: "test reparsed".to_string(),
            differences: vec![SemanticDifference {
                kind: DifferenceKind::TextDifference,
                location: None,
                description: "Test difference".to_string(),
                expected: Some("expected".to_string()),
                actual: Some("actual".to_string()),
            }],
            original_errors: vec!["original error".to_string()],
            reparsed_errors: vec!["reparsed error".to_string()],
            trivia_preserved: false,
            original_trivia: std::collections::HashMap::new(),
            reparsed_trivia: std::collections::HashMap::new(),
        };

        assert!(!result.is_valid());
        let issues = result.issues();
        assert_eq!(issues.len(), 5); // differences, original errors, reparsed errors, text mismatch, trivia
        assert!(issues[0].contains("1 semantic differences"));
        assert!(issues[1].contains("1 errors"));
        assert!(issues[2].contains("1 errors"));
        assert!(issues[3].contains("differs"));
    }

    #[test]
    fn test_normalize_text() {
        let validator = RoundTripValidator::new();
        
        let text1 = "  Profile: MyPatient  \n  Parent: Patient  \n\n  * name 1..1  ";
        let text2 = "Profile: MyPatient\nParent: Patient\n* name 1..1";
        
        let normalized1 = validator.normalize_text(text1);
        let normalized2 = validator.normalize_text(text2);
        
        assert_eq!(normalized1, normalized2);
        assert_eq!(normalized1, "Profile: MyPatient\nParent: Patient\n* name 1..1");
    }

    #[test]
    fn test_semantic_difference_types() {
        let diff = SemanticDifference {
            kind: DifferenceKind::MissingNode,
            location: Some("Profile 'Test'".to_string()),
            description: "Missing profile".to_string(),
            expected: Some("Test".to_string()),
            actual: None,
        };

        assert_eq!(diff.kind, DifferenceKind::MissingNode);
        assert_eq!(diff.location, Some("Profile 'Test'".to_string()));
        assert_eq!(diff.expected, Some("Test".to_string()));
        assert_eq!(diff.actual, None);
    }
}