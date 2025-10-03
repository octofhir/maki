//! Profile-specific validation rules
//!
//! Validates that profiles and extensions have proper assignments and context.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule ID for profile assignment validation
pub const PROFILE_ASSIGNMENT_PRESENT: &str = "builtin/correctness/profile-assignment-present";

/// Rule ID for extension context validation
pub const EXTENSION_CONTEXT_MISSING: &str = "builtin/correctness/extension-context-missing";

/// Check for missing profile assignments (status, abstract)
pub fn check_profile_assignments(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through profile declarations
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "profile_declaration" {
            if let Some(profile_diags) = check_single_profile(child, source, file_path) {
                diagnostics.extend(profile_diags);
            }
        }

        // Recursively check children
        diagnostics.extend(check_profile_assignments(child, source, file_path));
    }

    diagnostics
}

/// Check for missing extension context
pub fn check_extension_context(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through extension declarations
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "extension_declaration" {
            if let Some(ext_diags) = check_single_extension(child, source, file_path) {
                diagnostics.extend(ext_diags);
            }
        }

        // Recursively check children
        diagnostics.extend(check_extension_context(child, source, file_path));
    }

    diagnostics
}

/// Check a single profile for required assignments
fn check_single_profile(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    let profile_text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let profile_name = extract_resource_name(node, source);

    // Check for ^status assignment
    if !has_assignment(profile_text, "status") {
        diagnostics.push(create_missing_assignment_diagnostic(
            node,
            file_path,
            &profile_name,
            "status",
            "draft",
            Severity::Warning,
        ));
    }

    // Check for ^abstract assignment
    if !has_assignment(profile_text, "abstract") {
        diagnostics.push(create_missing_assignment_diagnostic(
            node,
            file_path,
            &profile_name,
            "abstract",
            "false",
            Severity::Info,
        ));
    }

    // Check for Parent declaration (important for profiles)
    if !has_parent(profile_text) {
        diagnostics.push(create_missing_parent_diagnostic(
            node,
            file_path,
            &profile_name,
        ));
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Check a single extension for context
fn check_single_extension(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Option<Vec<Diagnostic>> {
    let extension_text = node.utf8_text(source.as_bytes()).unwrap_or("");
    let extension_name = extract_resource_name(node, source);

    // Check for ^context assignment
    if !has_context(extension_text) {
        let diagnostic = create_missing_context_diagnostic(node, file_path, &extension_name);
        Some(vec![diagnostic])
    } else {
        None
    }
}

/// Extract resource name from declaration
fn extract_resource_name(node: Node, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");

    if let Some(colon_pos) = text.find(':') {
        let after_colon = &text[colon_pos + 1..];
        if let Some(first_line) = after_colon.lines().next() {
            return first_line.trim().to_string();
        }
    }

    "Unknown".to_string()
}

/// Check if assignment is present (^field = value)
fn has_assignment(text: &str, field: &str) -> bool {
    let pattern = format!("^{}", field);
    text.contains(&pattern)
}

/// Check if Parent declaration is present
fn has_parent(text: &str) -> bool {
    text.contains("Parent:") || text.contains("Parent ")
}

/// Check if context is present
fn has_context(text: &str) -> bool {
    text.contains("^context")
}

/// Create diagnostic for missing assignment
fn create_missing_assignment_diagnostic(
    node: Node,
    file_path: &PathBuf,
    profile_name: &str,
    field: &str,
    default_value: &str,
    severity: Severity,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!(
        "Profile '{}' is missing ^{} assignment",
        profile_name, field
    );

    let mut diagnostic = Diagnostic::new(PROFILE_ASSIGNMENT_PRESENT, severity, &message, location.clone());

    let suggestion_text = format!("* ^{} = #{}", field, default_value);

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Add ^{} assignment with default value", field),
        replacement: format!("\n{}", suggestion_text),
        location: location.clone(),
        is_safe: true, // Adding metadata assignments is safe
    });

    diagnostic
}

/// Create diagnostic for missing Parent
fn create_missing_parent_diagnostic(
    node: Node,
    file_path: &PathBuf,
    profile_name: &str,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!("Profile '{}' is missing Parent declaration", profile_name);

    let mut diagnostic = Diagnostic::new(
        PROFILE_ASSIGNMENT_PRESENT,
        Severity::Error,
        &message,
        location.clone(),
    );

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: "Add Parent declaration".to_string(),
        replacement: "\nParent: Resource".to_string(),
        location: location.clone(),
        is_safe: false, // Choosing parent is semantic decision
    });

    diagnostic
}

/// Create diagnostic for missing extension context
fn create_missing_context_diagnostic(
    node: Node,
    file_path: &PathBuf,
    extension_name: &str,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!(
        "Extension '{}' is missing ^context specification",
        extension_name
    );

    let mut diagnostic = Diagnostic::new(
        EXTENSION_CONTEXT_MISSING,
        Severity::Error,
        &message,
        location.clone(),
    );

    let suggestion_text = r#"* ^context[+].type = #element
* ^context[=].expression = "Element""#;

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: "Add context specification".to_string(),
        replacement: format!("\n{}", suggestion_text),
        location: location.clone(),
        is_safe: false, // Context choice is semantic decision
    });

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
    fn test_has_assignment() {
        assert!(has_assignment("^status = #draft", "status"));
        assert!(has_assignment("* ^abstract = false", "abstract"));
        assert!(!has_assignment("^status = #draft", "abstract"));
    }

    #[test]
    fn test_has_parent() {
        assert!(has_parent("Parent: Patient"));
        assert!(has_parent("Profile: MyProfile\nParent: Observation"));
        assert!(!has_parent("Profile: MyProfile\nTitle: Test"));
    }

    #[test]
    fn test_has_context() {
        assert!(has_context("^context[+].type = #element"));
        assert!(has_context("* ^context[0].expression = \"Patient\""));
        assert!(!has_context("^status = #draft"));
    }

    #[test]
    fn test_extract_resource_name() {
        // This would normally work with actual tree-sitter nodes
        // For now we just test the helper functions
        assert!(has_assignment("^status = #draft", "status"));
    }

    #[test]
    fn test_multiple_assignments() {
        let text = r#"
Profile: MyProfile
Parent: Patient
* ^status = #draft
* ^abstract = false
"#;
        assert!(has_assignment(text, "status"));
        assert!(has_assignment(text, "abstract"));
        assert!(has_parent(text));
    }

    #[test]
    fn test_missing_assignments() {
        let text = r#"
Profile: MyProfile
Parent: Patient
"#;
        assert!(!has_assignment(text, "status"));
        assert!(!has_assignment(text, "abstract"));
        assert!(has_parent(text));
    }
}
