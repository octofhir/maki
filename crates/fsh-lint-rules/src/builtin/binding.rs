//! Binding strength validation rules
//!
//! Validates that bindings to value sets have proper strength specifications
//! and use valid strength values.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule ID for binding strength validation
pub const BINDING_STRENGTH_PRESENT: &str = "builtin/correctness/binding-strength-present";

/// Valid FHIR binding strength values
const VALID_BINDING_STRENGTHS: &[&str] = &["required", "extensible", "preferred", "example"];

/// Check for missing or invalid binding strengths
pub fn check_binding_strength(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through all binding rule nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "binding_rule" {
            if let Some(binding_diag) = check_binding_rule(child, source, file_path) {
                diagnostics.extend(binding_diag);
            }
        }

        // Recursively check children
        diagnostics.extend(check_binding_strength(child, source, file_path));
    }

    diagnostics
}

/// Check a single binding rule for strength specification
fn check_binding_rule(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Get the full binding rule text
    let binding_text = node
        .utf8_text(source.as_bytes())
        .unwrap_or("")
        .trim();

    // Check if it has "from" keyword (indicates a binding)
    if !binding_text.contains(" from ") {
        return None;
    }

    // Check for binding strength in parentheses
    let has_strength = binding_text.contains('(') && binding_text.contains(')');

    if !has_strength {
        // Missing binding strength
        let location = create_location(node, file_path);
        diagnostics.push(create_missing_strength_diagnostic(
            &location,
            binding_text,
        ));
    } else {
        // Has strength - validate it
        if let Some(strength) = extract_binding_strength(binding_text) {
            if !is_valid_binding_strength(&strength) {
                let location = create_location(node, file_path);
                diagnostics.push(create_invalid_strength_diagnostic(
                    &location,
                    &strength,
                    binding_text,
                ));
            }
        }
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Extract binding strength from binding rule text
/// Example: "* code from ValueSet (required)" -> "required"
fn extract_binding_strength(binding_text: &str) -> Option<String> {
    // Find text within parentheses
    let start = binding_text.find('(')?;
    let end = binding_text.find(')')?;

    if end <= start {
        return None;
    }

    let strength = binding_text[start + 1..end].trim();
    Some(strength.to_string())
}

/// Check if a binding strength value is valid
fn is_valid_binding_strength(strength: &str) -> bool {
    VALID_BINDING_STRENGTHS.contains(&strength)
}

/// Create diagnostic for missing binding strength
fn create_missing_strength_diagnostic(
    location: &Location,
    binding_text: &str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        BINDING_STRENGTH_PRESENT,
        Severity::Error,
        "Binding is missing strength specification",
        location.clone(),
    );

    // Try to suggest a reasonable default based on context
    let suggested_strength = suggest_binding_strength(binding_text);
    let fixed_binding = format!("{} ({})", binding_text, suggested_strength);

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Add binding strength: ({})", suggested_strength),
        replacement: fixed_binding,
        location: location.clone(),
        is_safe: false, // Choosing strength requires semantic understanding
    });

    diagnostic
}

/// Create diagnostic for invalid binding strength
fn create_invalid_strength_diagnostic(
    location: &Location,
    invalid_strength: &str,
    binding_text: &str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        BINDING_STRENGTH_PRESENT,
        Severity::Error,
        &format!(
            "Invalid binding strength '{}'. Must be one of: {}",
            invalid_strength,
            VALID_BINDING_STRENGTHS.join(", ")
        ),
        location.clone(),
    );

    // Suggest the closest valid strength
    let suggested = suggest_closest_strength(invalid_strength);
    let fixed_binding = binding_text.replace(
        &format!("({})", invalid_strength),
        &format!("({})", suggested),
    );

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Use valid strength: ({})", suggested),
        replacement: fixed_binding,
        location: location.clone(),
        is_safe: false, // Changing strength changes semantics
    });

    diagnostic
}

/// Suggest an appropriate binding strength based on context
fn suggest_binding_strength(binding_text: &str) -> &'static str {
    // Heuristics for suggesting binding strength
    let lower = binding_text.to_lowercase();

    // Code/category often required or extensible
    if lower.contains("* code ") || lower.contains("*.code ") {
        "required"
    } else if lower.contains("* category") || lower.contains("*.category") {
        "extensible"
    } else if lower.contains("* interpretation") || lower.contains("*.interpretation") {
        "preferred"
    } else if lower.contains("* method") || lower.contains("*.method") {
        "example"
    } else if lower.contains("valuecoding") || lower.contains("valuecodeable") {
        "required"
    } else {
        // Default suggestion
        "extensible"
    }
}

/// Suggest the closest valid binding strength for a typo
fn suggest_closest_strength(invalid: &str) -> &'static str {
    let lower = invalid.to_lowercase();

    // Direct matches or common typos
    match lower.as_str() {
        "require" | "mandatory" | "must" | "strict" => "required",
        "extend" | "extensable" | "extendable" => "extensible",
        "prefer" | "preferred" | "recommended" => "preferred",
        "sample" | "examples" | "illustration" => "example",
        "very-strict" | "super-strict" => "required",
        _ => {
            // Fuzzy matching - find shortest edit distance
            let mut min_distance = usize::MAX;
            let mut closest = "extensible";

            for valid in VALID_BINDING_STRENGTHS {
                let distance = edit_distance(&lower, valid);
                if distance < min_distance {
                    min_distance = distance;
                    closest = valid;
                }
            }

            closest
        }
    }
}

/// Calculate Levenshtein distance between two strings
fn edit_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }

    matrix[len1][len2]
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
    fn test_valid_binding_strengths() {
        assert!(is_valid_binding_strength("required"));
        assert!(is_valid_binding_strength("extensible"));
        assert!(is_valid_binding_strength("preferred"));
        assert!(is_valid_binding_strength("example"));
    }

    #[test]
    fn test_invalid_binding_strengths() {
        assert!(!is_valid_binding_strength("mandatory"));
        assert!(!is_valid_binding_strength("strict"));
        assert!(!is_valid_binding_strength("very-strict"));
        assert!(!is_valid_binding_strength("optional"));
    }

    #[test]
    fn test_extract_binding_strength() {
        assert_eq!(
            extract_binding_strength("* code from ValueSet (required)"),
            Some("required".to_string())
        );
        assert_eq!(
            extract_binding_strength("* category from Codes (extensible)"),
            Some("extensible".to_string())
        );
        assert_eq!(
            extract_binding_strength("* code from ValueSet"),
            None
        );
    }

    #[test]
    fn test_suggest_binding_strength() {
        assert_eq!(suggest_binding_strength("* code from ValueSet"), "required");
        assert_eq!(suggest_binding_strength("* category from Codes"), "extensible");
        assert_eq!(suggest_binding_strength("* interpretation from Interp"), "preferred");
        assert_eq!(suggest_binding_strength("* method from Methods"), "example");
        assert_eq!(suggest_binding_strength("* other from Other"), "extensible");
    }

    #[test]
    fn test_suggest_closest_strength() {
        assert_eq!(suggest_closest_strength("require"), "required");
        assert_eq!(suggest_closest_strength("mandatory"), "required");
        assert_eq!(suggest_closest_strength("extensable"), "extensible");
        assert_eq!(suggest_closest_strength("prefer"), "preferred");
        assert_eq!(suggest_closest_strength("sample"), "example");
        assert_eq!(suggest_closest_strength("very-strict"), "required");
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("required", "required"), 0);
        assert_eq!(edit_distance("require", "required"), 1);
        assert_eq!(edit_distance("extensable", "extensible"), 1);
        assert_eq!(edit_distance("preferred", "prefered"), 1); // One 'r' removed
        assert_eq!(edit_distance("example", "sample"), 2); // e->s, x->a
    }

    #[test]
    fn test_fuzzy_matching() {
        // "extensable" should match "extensible" (1 char difference)
        assert_eq!(suggest_closest_strength("extensable"), "extensible");

        // "prefered" should match "preferred" (1 char difference - missing 'r')
        assert_eq!(suggest_closest_strength("prefered"), "preferred");

        // "requried" should match "required" (2 char difference - transposition)
        assert_eq!(suggest_closest_strength("requried"), "required");
    }
}
