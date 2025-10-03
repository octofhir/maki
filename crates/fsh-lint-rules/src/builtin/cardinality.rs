//! Cardinality validation rules
//!
//! Validates cardinality constraints in FSH element rules.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule ID for invalid cardinality
pub const INVALID_CARDINALITY: &str = "builtin/correctness/invalid-cardinality";

/// Check for invalid cardinality expressions
pub fn check_cardinality(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through all element rule nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "element_rule" {
            if let Some(card_diag) = check_element_cardinality(child, source, file_path) {
                diagnostics.extend(card_diag);
            }
        }

        // Recursively check children
        diagnostics.extend(check_cardinality(child, source, file_path));
    }

    diagnostics
}

/// Check cardinality in a single element rule
fn check_element_cardinality(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Find cardinality node
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "cardinality" {
            let cardinality_text = child
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim();

            // Parse the cardinality expression
            if let Some(diag) = validate_cardinality_syntax(
                cardinality_text,
                child,
                source,
                file_path,
            ) {
                diagnostics.push(diag);
            }
        }
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Validate cardinality syntax and semantics
fn validate_cardinality_syntax(
    cardinality: &str,
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Diagnostic> {
    let location = create_location(node, file_path);

    // Check for invalid characters or patterns
    if cardinality.contains("..*..")  {
        return Some(create_diagnostic(
            &location,
            "Invalid cardinality syntax: multiple '..' separators",
            Some(format!("Remove extra '..' separator")),
        ));
    }

    // Check for non-numeric bounds (except * for unbounded)
    if !is_valid_cardinality_format(cardinality) {
        return Some(create_diagnostic(
            &location,
            &format!("Invalid cardinality format: '{}'", cardinality),
            Some("Use format: 'min..max' where min and max are numbers or '*'".to_string()),
        ));
    }

    // Parse bounds
    let (min, max) = parse_cardinality_bounds(cardinality)?;

    // Check for reversed bounds (e.g., 1..0)
    if let (Some(min_val), Some(max_val)) = (min, max) {
        if min_val > max_val {
            return Some(create_reversed_bounds_diagnostic(
                &location,
                min_val,
                max_val,
                cardinality,
            ));
        }
    }

    // Check for negative bounds
    if let Some(min_val) = min {
        if cardinality.contains(&format!("-{}", min_val)) {
            return Some(create_diagnostic(
                &location,
                "Cardinality bounds cannot be negative",
                Some(format!("Change to: {}..{}", min_val, max.map_or("*".to_string(), |m| m.to_string()))),
            ));
        }
    }

    None
}

/// Check if cardinality format is valid
fn is_valid_cardinality_format(cardinality: &str) -> bool {
    // Valid formats:
    // - Single number: "1"
    // - Range: "0..1", "1..*", "0..*"
    // - Unbounded: "*"

    if cardinality == "*" {
        return true;
    }

    if cardinality.contains("..") {
        let parts: Vec<&str> = cardinality.split("..").collect();
        if parts.len() != 2 {
            return false;
        }

        // Check min (must not be empty)
        if parts[0].is_empty() || !parts[0].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check max (must not be empty)
        if parts[1].is_empty() {
            return false;
        }

        if parts[1] != "*" && !parts[1].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        true
    } else {
        // Single number (must not be empty)
        !cardinality.is_empty() && cardinality.chars().all(|c| c.is_ascii_digit())
    }
}

/// Parse cardinality bounds into (min, max) tuple
/// Returns (Some(min), Some(max)) for numeric bounds, None for unbounded (*)
fn parse_cardinality_bounds(cardinality: &str) -> Option<(Option<usize>, Option<usize>)> {
    if cardinality == "*" {
        return Some((None, None));
    }

    if cardinality.contains("..") {
        let parts: Vec<&str> = cardinality.split("..").collect();
        if parts.len() != 2 {
            return None;
        }

        let min = parts[0].parse::<usize>().ok();
        let max = if parts[1] == "*" {
            None
        } else {
            parts[1].parse::<usize>().ok()
        };

        Some((min, max))
    } else {
        // Single number means exact cardinality (n..n)
        let val = cardinality.parse::<usize>().ok()?;
        Some((Some(val), Some(val)))
    }
}

/// Create a diagnostic for reversed bounds (e.g., 1..0)
fn create_reversed_bounds_diagnostic(
    location: &Location,
    min: usize,
    max: usize,
    _original: &str,
) -> Diagnostic {
    let swapped = format!("{}..{}", max, min);

    let mut diagnostic = Diagnostic::new(
        INVALID_CARDINALITY,
        Severity::Error,
        &format!(
            "upper bound ({}) cannot be less than lower bound ({})",
            max, min
        ),
        location.clone(),
    );

    // Add suggestion to swap the bounds
    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("⚠ swap to {}", swapped),
        replacement: swapped,
        location: location.clone(),
        is_safe: false, // Swapping might not always be the right fix
    });

    diagnostic
}

/// Create a generic cardinality diagnostic
fn create_diagnostic(
    location: &Location,
    message: &str,
    suggestion: Option<String>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        INVALID_CARDINALITY,
        Severity::Error,
        message,
        location.clone(),
    );

    if let Some(sugg_text) = suggestion {
        diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
            message: "Fix cardinality syntax".to_string(),
            replacement: sugg_text,
            location: location.clone(),
            is_safe: false,
        });
    }

    diagnostic
}

/// Create a Location from a tree-sitter Node
fn create_location(node: Node, file_path: &PathBuf) -> Location {
    Location {
        file: file_path.clone(),
        line: node.start_position().row + 1,
        column: node.start_position().column + 1,
        end_line: Some(node.end_position().row + 1),
        end_column: Some(node.end_position().column + 1),
        offset: node.start_byte(),
        length: node.end_byte() - node.start_byte(),
        span: Some((node.start_byte(), node.end_byte())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cardinality_formats() {
        assert!(is_valid_cardinality_format("0..1"));
        assert!(is_valid_cardinality_format("1..*"));
        assert!(is_valid_cardinality_format("0..*"));
        assert!(is_valid_cardinality_format("*"));
        assert!(is_valid_cardinality_format("1"));
        assert!(is_valid_cardinality_format("0"));
    }

    #[test]
    fn test_invalid_cardinality_formats() {
        assert!(!is_valid_cardinality_format("one..many"));
        assert!(!is_valid_cardinality_format("1..*..2"));
        assert!(!is_valid_cardinality_format("..1"));
        assert!(!is_valid_cardinality_format("1.."));
        assert!(!is_valid_cardinality_format("-1..5"));
    }

    #[test]
    fn test_parse_cardinality_bounds() {
        assert_eq!(
            parse_cardinality_bounds("0..1"),
            Some((Some(0), Some(1)))
        );
        assert_eq!(
            parse_cardinality_bounds("1..*"),
            Some((Some(1), None))
        );
        assert_eq!(
            parse_cardinality_bounds("*"),
            Some((None, None))
        );
        assert_eq!(
            parse_cardinality_bounds("5"),
            Some((Some(5), Some(5)))
        );
    }

    #[test]
    fn test_detect_reversed_bounds() {
        let (min, max) = parse_cardinality_bounds("1..0").unwrap();
        assert_eq!(min, Some(1));
        assert_eq!(max, Some(0));
        assert!(min > max);
    }

    #[test]
    fn test_valid_bounds_not_reversed() {
        let (min, max) = parse_cardinality_bounds("0..1").unwrap();
        assert_eq!(min, Some(0));
        assert_eq!(max, Some(1));
        assert!(min <= max);
    }
}
