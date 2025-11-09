//! Rewrite validation - ensure rewrites are safe and don't break code
//!
//! This module provides:
//! - Syntax validation (rewritten code parses correctly)
//! - Semantic preservation checking
//! - Conflict detection with other fixes
//! - Preview mode for dry-runs

use maki_core::Result;

/// Result of validating a rewrite
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Rewrite is safe
    Safe,
    /// Rewrite has issues
    Unsafe { issues: Vec<String> },
}

impl ValidationResult {
    /// Check if validation passed
    pub fn is_safe(&self) -> bool {
        matches!(self, ValidationResult::Safe)
    }

    /// Get list of issues
    pub fn issues(&self) -> Vec<&str> {
        match self {
            ValidationResult::Safe => vec![],
            ValidationResult::Unsafe { issues } => issues.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Validator for rewrite safety
pub struct RewriteValidator;

impl RewriteValidator {
    /// Validate that rewrite is safe and doesn't conflict
    pub fn validate(original: &str, rewritten: &str) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        // 1. Basic syntax check - rewritten code should not have obvious syntax errors
        if !Self::is_syntactically_valid(rewritten) {
            issues.push("Rewrite introduces syntax errors".to_string());
        }

        // 2. Check that content is not completely empty (unless that's intentional)
        if rewritten.is_empty() && !original.is_empty() {
            issues.push("Rewrite produces empty content".to_string());
        }

        // 3. Structural preservation: line count shouldn't change drastically
        let original_lines = original.lines().count();
        let rewritten_lines = rewritten.lines().count();
        let line_diff = (original_lines as i32 - rewritten_lines as i32).abs();

        // Allow up to 20% change in line count for legitimate rewrites
        if line_diff > original_lines.max(1) as i32 / 5 && line_diff > 3 {
            // This is just a warning, not a failure
            tracing::warn!(
                "Line count changed significantly: {} -> {}",
                original_lines,
                rewritten_lines
            );
        }

        if issues.is_empty() {
            Ok(ValidationResult::Safe)
        } else {
            Ok(ValidationResult::Unsafe { issues })
        }
    }

    /// Preview rewrite without applying
    pub fn preview(original: &str, replacement: &str, range: (usize, usize)) -> String {
        let (start, end) = range;
        let mut result = original.to_string();

        if start <= result.len() && end <= result.len() {
            result.replace_range(start..end, replacement);
        }

        result
    }

    /// Check if code is syntactically valid by basic heuristics
    fn is_syntactically_valid(code: &str) -> bool {
        // Check for balanced braces and brackets
        let mut brace_count = 0;
        let mut bracket_count = 0;
        let mut paren_count = 0;
        let mut in_string = false;
        let mut in_comment = false;
        let mut escape_next = false;

        for ch in code.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '/' if !in_string => {
                    // Very basic comment handling
                    in_comment = true;
                }
                '\n' if in_comment => in_comment = false,
                '{' if !in_string && !in_comment => brace_count += 1,
                '}' if !in_string && !in_comment => brace_count -= 1,
                '[' if !in_string && !in_comment => bracket_count += 1,
                ']' if !in_string && !in_comment => bracket_count -= 1,
                '(' if !in_string && !in_comment => paren_count += 1,
                ')' if !in_string && !in_comment => paren_count -= 1,
                _ => {}
            }

            // If counts go negative, we have unmatched closing brackets
            if brace_count < 0 || bracket_count < 0 || paren_count < 0 {
                return false;
            }
        }

        // All counts should be zero at the end
        brace_count == 0 && bracket_count == 0 && paren_count == 0
    }

    /// Check if two rewrites conflict (overlap)
    pub fn check_conflict(range1: (usize, usize), range2: (usize, usize)) -> bool {
        let (start1, end1) = range1;
        let (start2, end2) = range2;

        // Ranges overlap if one starts before the other ends
        !(end1 <= start2 || end2 <= start1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_safe_rewrite() {
        let original = "Profile: badName";
        let rewritten = "Profile: BadName";

        let result = RewriteValidator::validate(original, rewritten).unwrap();
        assert!(result.is_safe());
    }

    #[test]
    fn test_validation_unbalanced_braces() {
        let original = "Profile: GoodProfile";
        let rewritten = "Profile: GoodProfile {";

        let result = RewriteValidator::validate(original, rewritten).unwrap();
        assert!(!result.is_safe());
    }

    #[test]
    fn test_validation_empty_rewrite() {
        let original = "Profile: GoodProfile";
        let rewritten = "";

        let result = RewriteValidator::validate(original, rewritten).unwrap();
        assert!(!result.is_safe());
    }

    #[test]
    fn test_validation_empty_both() {
        let original = "";
        let rewritten = "";

        let result = RewriteValidator::validate(original, rewritten).unwrap();
        assert!(result.is_safe());
    }

    #[test]
    fn test_preview_replacement() {
        let original = "Hello World";
        let replacement = "Rust";
        let range = (6, 11); // "World"

        let result = RewriteValidator::preview(original, replacement, range);
        assert_eq!(result, "Hello Rust");
    }

    #[test]
    fn test_preview_insertion() {
        let original = "Hello World";
        let replacement = " beautiful";
        let range = (5, 5); // Insert at position 5

        let result = RewriteValidator::preview(original, replacement, range);
        assert_eq!(result, "Hello beautiful World");
    }

    #[test]
    fn test_check_conflict_overlapping() {
        let range1 = (0, 10);
        let range2 = (5, 15);

        assert!(RewriteValidator::check_conflict(range1, range2));
    }

    #[test]
    fn test_check_conflict_non_overlapping() {
        let range1 = (0, 5);
        let range2 = (10, 15);

        assert!(!RewriteValidator::check_conflict(range1, range2));
    }

    #[test]
    fn test_check_conflict_adjacent() {
        let range1 = (0, 5);
        let range2 = (5, 10);

        assert!(!RewriteValidator::check_conflict(range1, range2));
    }

    #[test]
    fn test_syntax_valid_balanced() {
        assert!(RewriteValidator::is_syntactically_valid(
            "Profile: MyProfile { Title: \"Test\" }"
        ));
    }

    #[test]
    fn test_syntax_invalid_unbalanced_brace() {
        assert!(!RewriteValidator::is_syntactically_valid(
            "Profile: MyProfile {"
        ));
    }

    #[test]
    fn test_syntax_invalid_unbalanced_paren() {
        assert!(!RewriteValidator::is_syntactically_valid("fn test("));
    }

    #[test]
    fn test_syntax_valid_with_string() {
        assert!(RewriteValidator::is_syntactically_valid(
            r#"Title: "This has { and } in it""#
        ));
    }
}
