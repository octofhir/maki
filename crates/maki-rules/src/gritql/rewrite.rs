//! GritQL code rewriting and autofix generation
//!
//! This module provides:
//! - Effect types for code transformations (Replace, Insert, Delete, RewriteField)
//! - Variable interpolation for dynamic replacements
//! - Safety classification for autofixes
//! - Integration with MAKI's CodeSuggestion system

use maki_core::{Applicability, CodeSuggestion, Location, MakiError, Result};
use regex::Regex;
use std::collections::HashMap;

/// Effect represents a code transformation from GritQL pattern
#[derive(Debug, Clone)]
pub enum Effect {
    /// Replace node with new text
    Replace {
        start_offset: usize,
        end_offset: usize,
        replacement: String,
    },

    /// Insert text at position
    Insert { position: usize, text: String },

    /// Delete node
    Delete {
        start_offset: usize,
        end_offset: usize,
    },

    /// Rewrite field value
    RewriteField {
        field_name: String,
        start_offset: usize,
        end_offset: usize,
        new_value: String,
    },
}

impl Effect {
    /// Apply effect to source code, returning a CodeSuggestion
    pub fn apply(
        &self,
        source: &str,
        variables: &HashMap<String, String>,
        file_path: &str,
    ) -> Result<CodeSuggestion> {
        match self {
            Effect::Replace {
                start_offset,
                end_offset,
                replacement,
            } => {
                let interpolated = Self::interpolate_variables(replacement, variables)?;

                let location =
                    Self::create_location(file_path, source, *start_offset, *end_offset)?;

                Ok(CodeSuggestion {
                    message: format!("Replace with '{}'", interpolated),
                    replacement: interpolated,
                    location,
                    applicability: Applicability::Always, // Most rewrites are safe
                    labels: vec![],
                })
            }

            Effect::Insert { position, text } => {
                let interpolated = Self::interpolate_variables(text, variables)?;

                let location = Self::create_location(file_path, source, *position, *position)?;

                Ok(CodeSuggestion {
                    message: format!("Insert '{}'", interpolated),
                    replacement: interpolated,
                    location,
                    applicability: Applicability::MaybeIncorrect, // Insertions need review
                    labels: vec![],
                })
            }

            Effect::Delete {
                start_offset,
                end_offset,
            } => {
                let location =
                    Self::create_location(file_path, source, *start_offset, *end_offset)?;

                Ok(CodeSuggestion {
                    message: "Delete this node".to_string(),
                    replacement: String::new(),
                    location,
                    applicability: Applicability::MaybeIncorrect, // Deletions need review
                    labels: vec![],
                })
            }

            Effect::RewriteField {
                field_name,
                start_offset,
                end_offset,
                new_value,
            } => {
                let interpolated = Self::interpolate_variables(new_value, variables)?;

                let location =
                    Self::create_location(file_path, source, *start_offset, *end_offset)?;

                // Field rewrites for cosmetic fields are safe
                let applicability = if matches!(field_name.as_str(), "id" | "name" | "title") {
                    Applicability::Always
                } else {
                    Applicability::MaybeIncorrect
                };

                Ok(CodeSuggestion {
                    message: format!("Set {} to '{}'", field_name, interpolated),
                    replacement: interpolated,
                    location,
                    applicability,
                    labels: vec![],
                })
            }
        }
    }

    /// Interpolate variable references in replacement text
    /// Example: "Profile: $name" with {name: "GoodName"} => "Profile: GoodName"
    pub fn interpolate_variables(
        template: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = template.to_string();

        // Find all $variable references
        let regex = Regex::new(r"\$(\w+)").map_err(|e| {
            MakiError::rule_error("gritql-rewrite", format!("Invalid regex: {}", e))
        })?;
        for cap in regex.captures_iter(template) {
            let var_name = &cap[1];
            if let Some(value) = variables.get(var_name) {
                result = result.replace(&format!("${}", var_name), value);
            } else {
                return Err(MakiError::rule_error(
                    "gritql-rewrite",
                    format!("Undefined variable: ${}", var_name),
                ));
            }
        }

        Ok(result)
    }

    /// Create a Location from byte offsets
    fn create_location(
        file_path: &str,
        source: &str,
        start: usize,
        end: usize,
    ) -> Result<Location> {
        let (start_line, start_col) = offset_to_line_col(source, start);
        let (end_line, end_col) = offset_to_line_col(source, end);

        Ok(Location {
            file: file_path.into(),
            line: start_line,
            column: start_col,
            end_line: Some(end_line),
            end_column: Some(end_col),
            offset: start,
            length: end - start,
            span: Some((start, end)),
        })
    }

    /// Determine if this effect is a safe autofix
    pub fn is_safe(&self) -> bool {
        match self {
            // Simple replacements are safe
            Effect::Replace { .. } => true,

            // Insertions might change semantics - mark as unsafe
            Effect::Insert { .. } => false,

            // Deletions are generally unsafe
            Effect::Delete { .. } => false,

            // Field rewrites might be safe depending on field
            Effect::RewriteField { field_name, .. } => {
                // Safe to rewrite cosmetic fields
                matches!(field_name.as_str(), "id" | "name" | "title")
            }
        }
    }
}

/// Convert byte offset to line and column (1-indexed)
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.chars().enumerate() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_interpolation() {
        let template = "Profile: $name";
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "GoodName".to_string());

        let result = Effect::interpolate_variables(template, &variables).unwrap();
        assert_eq!(result, "Profile: GoodName");
    }

    #[test]
    fn test_variable_interpolation_multiple() {
        let template = "Profile: $name where parent is $parent";
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "GoodName".to_string());
        variables.insert("parent".to_string(), "Patient".to_string());

        let result = Effect::interpolate_variables(template, &variables).unwrap();
        assert_eq!(result, "Profile: GoodName where parent is Patient");
    }

    #[test]
    fn test_variable_interpolation_undefined() {
        let template = "Profile: $name and $undefined";
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "GoodName".to_string());

        let result = Effect::interpolate_variables(template, &variables);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_effect_safety() {
        let effect = Effect::Replace {
            start_offset: 0,
            end_offset: 10,
            replacement: "new".to_string(),
        };
        assert!(effect.is_safe());
    }

    #[test]
    fn test_insert_effect_safety() {
        let effect = Effect::Insert {
            position: 5,
            text: "inserted".to_string(),
        };
        assert!(!effect.is_safe());
    }

    #[test]
    fn test_delete_effect_safety() {
        let effect = Effect::Delete {
            start_offset: 0,
            end_offset: 10,
        };
        assert!(!effect.is_safe());
    }

    #[test]
    fn test_field_rewrite_safe_field() {
        let effect = Effect::RewriteField {
            field_name: "name".to_string(),
            start_offset: 5,
            end_offset: 10,
            new_value: "new_name".to_string(),
        };
        assert!(effect.is_safe());
    }

    #[test]
    fn test_field_rewrite_unsafe_field() {
        let effect = Effect::RewriteField {
            field_name: "parent".to_string(),
            start_offset: 5,
            end_offset: 10,
            new_value: "NewParent".to_string(),
        };
        assert!(!effect.is_safe());
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "Line 1\nLine 2\nLine 3";
        assert_eq!(offset_to_line_col(source, 0), (1, 1));
        assert_eq!(offset_to_line_col(source, 7), (2, 1));
        assert_eq!(line_col_in_bounds(source, 14), (3, 1));
    }

    fn line_col_in_bounds(source: &str, offset: usize) -> (usize, usize) {
        offset_to_line_col(source, offset)
    }

    #[test]
    fn test_apply_replace_effect() {
        let effect = Effect::Replace {
            start_offset: 0,
            end_offset: 7,
            replacement: "Changed".to_string(),
        };

        let variables = HashMap::new();
        let result = effect.apply("Original text", &variables, "test.fsh");
        assert!(result.is_ok());

        let suggestion = result.unwrap();
        assert_eq!(suggestion.replacement, "Changed");
        assert!(suggestion.applicability == Applicability::Always);
    }
}
