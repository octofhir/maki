//! Required field validation rules
//!
//! These are blocking rules that must pass before other rules can run.
//! They ensure that critical fields are present in FSH definitions.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule IDs for required field validation
pub const REQUIRED_FIELD_PRESENT: &str = "builtin/correctness/required-field-present";

/// Check if a Profile has all required fields
pub fn check_profile_required_fields(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Required fields for Profile: Name, Id, Title
    let has_name = has_profile_name(node, source);
    let has_id = has_field(node, source, "Id");
    let has_title = has_field(node, source, "Title");

    if !has_name {
        diagnostics.push(create_missing_field_diagnostic(
            "Profile",
            "Name",
            node,
            file_path,
            "Profile declarations must start with 'Profile: <Name>'",
        ));
    }

    if !has_id {
        diagnostics.push(create_missing_field_diagnostic(
            "Profile",
            "Id",
            node,
            file_path,
            "Profiles must have an Id field",
        ));
    }

    if !has_title {
        diagnostics.push(create_missing_field_diagnostic(
            "Profile",
            "Title",
            node,
            file_path,
            "Profiles must have a Title field",
        ));
    }

    diagnostics
}

/// Check if a CodeSystem has all required fields
pub fn check_code_system_required_fields(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Required fields for CodeSystem: Name, Id, Title
    let has_name = has_code_system_name(node, source);
    let has_id = has_field(node, source, "Id");
    let has_title = has_field(node, source, "Title");

    if !has_name {
        diagnostics.push(create_missing_field_diagnostic(
            "CodeSystem",
            "Name",
            node,
            file_path,
            "CodeSystem declarations must start with 'CodeSystem: <Name>'",
        ));
    }

    if !has_id {
        diagnostics.push(create_missing_field_diagnostic(
            "CodeSystem",
            "Id",
            node,
            file_path,
            "CodeSystems must have an Id field",
        ));
    }

    if !has_title {
        diagnostics.push(create_missing_field_diagnostic(
            "CodeSystem",
            "Title",
            node,
            file_path,
            "CodeSystems must have a Title field",
        ));
    }

    diagnostics
}

/// Check if a ValueSet has all required fields
pub fn check_value_set_required_fields(
    node: Node,
    source: &str,
    file_path: &PathBuf,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Required fields for ValueSet: Name, Id, Title
    let has_name = has_value_set_name(node, source);
    let has_id = has_field(node, source, "Id");
    let has_title = has_field(node, source, "Title");

    if !has_name {
        diagnostics.push(create_missing_field_diagnostic(
            "ValueSet",
            "Name",
            node,
            file_path,
            "ValueSet declarations must start with 'ValueSet: <Name>'",
        ));
    }

    if !has_id {
        diagnostics.push(create_missing_field_diagnostic(
            "ValueSet",
            "Id",
            node,
            file_path,
            "ValueSets must have an Id field",
        ));
    }

    if !has_title {
        diagnostics.push(create_missing_field_diagnostic(
            "ValueSet",
            "Title",
            node,
            file_path,
            "ValueSets must have a Title field",
        ));
    }

    diagnostics
}

/// Check if a node has a Profile name declaration
fn has_profile_name(node: Node, source: &str) -> bool {
    // Look for "Profile: <Name>" pattern
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    text.trim_start().starts_with("Profile:") && text.contains(':')
}

/// Check if a node has a CodeSystem name declaration
fn has_code_system_name(node: Node, source: &str) -> bool {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    text.trim_start().starts_with("CodeSystem:") && text.contains(':')
}

/// Check if a node has a ValueSet name declaration
fn has_value_set_name(node: Node, source: &str) -> bool {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");
    text.trim_start().starts_with("ValueSet:") && text.contains(':')
}

/// Check if a node has a specific field
fn has_field(node: Node, source: &str, field_name: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let text = child.utf8_text(source.as_bytes()).unwrap_or("");
        if text.trim_start().starts_with(&format!("{field_name}:")) {
            return true;
        }
    }
    false
}

/// Create a diagnostic for a missing required field
fn create_missing_field_diagnostic(
    resource_type: &str,
    field_name: &str,
    node: Node,
    file_path: &PathBuf,
    message: &str,
) -> Diagnostic {
    let location = Location {
        file: file_path.clone(),
        line: node.start_position().row + 1,
        column: node.start_position().column + 1,
        end_line: Some(node.end_position().row + 1),
        end_column: Some(node.end_position().column + 1),
        offset: node.start_byte(),
        length: node.end_byte() - node.start_byte(),
        span: Some((node.start_byte(), node.end_byte())),
    };

    let mut diagnostic = Diagnostic::new(
        REQUIRED_FIELD_PRESENT,
        Severity::Error,
        message,
        location.clone(),
    );

    // Add suggestion to add the missing field
    let suggestion_text = match field_name {
        "Id" => format!("Id: {}", to_kebab_case(resource_type)),
        "Title" => format!("Title: \"{}\"", to_title_case(resource_type)),
        _ => format!("{field_name}: <value>"),
    };

    #[allow(deprecated)]
    diagnostic.suggestions.push(fsh_lint_core::Suggestion {
        message: format!("Add {field_name} field"),
        replacement: suggestion_text,
        location: Location {
            file: file_path.clone(),
            line: node.end_position().row + 2, // Add after the declaration
            column: 1,
            end_line: Some(node.end_position().row + 2),
            end_column: Some(1),
            offset: node.end_byte(),
            length: 0,
            span: Some((node.end_byte(), node.end_byte())),
        },
        is_safe: true, // Adding metadata fields is safe
    });

    diagnostic
}

/// Convert PascalCase to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Convert text to Title Case
fn to_title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("ProfileName"), "profile-name");
        assert_eq!(to_kebab_case("USCorePatient"), "u-s-core-patient");
        assert_eq!(to_kebab_case("MyProfile"), "my-profile");
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("profile name"), "Profile Name");
        assert_eq!(to_title_case("my profile"), "My Profile");
    }
}
