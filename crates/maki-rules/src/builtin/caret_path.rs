//! Caret path validation rules
//!
//! Validates that caret paths in FSH rules have correct syntax.
//! Caret rules are used to set metadata on elements or definitions.
//!
//! Examples of valid caret paths:
//! - `* ^version = "1.0.0"` - Profile-level metadata
//! - `* ^extension[FMM].valueInteger = 4` - Extension with index
//! - `* subject ^definition = "..."` - Element-level caret rule
//!
//! This rule detects malformed paths like:
//! - Double dots: `^foo..bar`
//! - Empty brackets: `^foo[]`
//! - Unbalanced brackets: `^foo[bar` or `^foo]bar`

use maki_core::cst::ast::{AstNode, CaretValueRule, Document};
use maki_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for invalid caret path detection
pub const INVALID_CARET_PATH: &str = "correctness/invalid-caret-path";

/// Validates caret paths for common syntax errors
pub fn check_invalid_caret_paths(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check all caret rules in profiles
    for profile in document.profiles() {
        for rule in profile.rules() {
            if let maki_core::cst::ast::Rule::CaretValue(caret_rule) = rule {
                if let Some(diag) = validate_caret_path(&caret_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check all caret rules in extensions
    for ext in document.extensions() {
        for rule in ext.rules() {
            if let maki_core::cst::ast::Rule::CaretValue(caret_rule) = rule {
                if let Some(diag) = validate_caret_path(&caret_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check all caret rules in value sets
    for vs in document.value_sets() {
        for rule in vs.rules() {
            if let maki_core::cst::ast::Rule::CaretValue(caret_rule) = rule {
                if let Some(diag) = validate_caret_path(&caret_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check all caret rules in code systems
    for cs in document.code_systems() {
        for rule in cs.rules() {
            if let maki_core::cst::ast::Rule::CaretValue(caret_rule) = rule {
                if let Some(diag) = validate_caret_path(&caret_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    diagnostics
}

/// Validate a single caret path and return a diagnostic if invalid
fn validate_caret_path(rule: &CaretValueRule, model: &SemanticModel) -> Option<Diagnostic> {
    let path = rule.caret_path()?;
    let path_text = path.syntax().text().to_string();

    // Check for double dots (e.g., ^foo..bar)
    if path_text.contains("..") {
        let location = model.source_map.node_to_diagnostic_location(
            rule.syntax(),
            &model.source,
            &model.source_file,
        );
        return Some(Diagnostic::new(
            INVALID_CARET_PATH,
            Severity::Error,
            format!(
                "Invalid caret path '{}': contains consecutive dots (..)",
                path_text
            ),
            location,
        ));
    }

    // Check for empty brackets (e.g., ^foo[])
    if path_text.contains("[]") {
        let location = model.source_map.node_to_diagnostic_location(
            rule.syntax(),
            &model.source,
            &model.source_file,
        );
        return Some(Diagnostic::new(
            INVALID_CARET_PATH,
            Severity::Error,
            format!(
                "Invalid caret path '{}': contains empty brackets []",
                path_text
            ),
            location,
        ));
    }

    // Check for unbalanced brackets
    let open_count = path_text.matches('[').count();
    let close_count = path_text.matches(']').count();
    if open_count != close_count {
        let location = model.source_map.node_to_diagnostic_location(
            rule.syntax(),
            &model.source,
            &model.source_file,
        );
        return Some(Diagnostic::new(
            INVALID_CARET_PATH,
            Severity::Error,
            format!("Invalid caret path '{}': unbalanced brackets", path_text),
            location,
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_caret_paths() {
        // These should not produce diagnostics - just a placeholder
        // Real tests would parse FSH and check the rule output
        assert_eq!(INVALID_CARET_PATH, "correctness/invalid-caret-path");
    }
}
