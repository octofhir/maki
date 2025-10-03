//! Metadata documentation validation rules
//!
//! Validates that FHIR resources have proper documentation metadata.
//! These are warning-level rules that encourage good documentation practices.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule ID for missing metadata validation
pub const MISSING_METADATA: &str = "builtin/documentation/missing-metadata";

/// Required and recommended metadata fields by resource type
const METADATA_FIELDS: &[&str] = &[
    "Description",
    "Title",
    "Publisher",
    "Contact",
    "Copyright",
    "Purpose",
];

/// Check for missing metadata documentation
pub fn check_missing_metadata(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Walk through resource declarations
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();

        if matches!(
            kind,
            "profile_declaration" | "extension_declaration" | "value_set_declaration" | "code_system_declaration"
        ) {
            if let Some(metadata_diags) = check_resource_metadata(child, source, file_path, kind) {
                diagnostics.extend(metadata_diags);
            }
        }

        // Recursively check children
        diagnostics.extend(check_missing_metadata(child, source, file_path));
    }

    diagnostics
}

/// Check metadata for a single resource
fn check_resource_metadata(
    node: Node,
    source: &str,
    file_path: &PathBuf,
    resource_type: &str,
) -> Option<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();

    // Extract resource name for context
    let resource_name = extract_resource_name(node, source);

    // Get resource content
    let resource_text = node
        .utf8_text(source.as_bytes())
        .unwrap_or("");

    // Check for missing Description (most important)
    if !has_field(resource_text, "Description") && !has_caret_field(resource_text, "description") {
        diagnostics.push(create_missing_metadata_diagnostic(
            node,
            file_path,
            &resource_name,
            "Description",
            resource_type,
            Severity::Warning,
        ));
    }

    // Check for missing Title (already checked by required-fields for some, but good practice for all)
    if !has_field(resource_text, "Title") && !has_caret_field(resource_text, "title") {
        diagnostics.push(create_missing_metadata_diagnostic(
            node,
            file_path,
            &resource_name,
            "Title",
            resource_type,
            Severity::Info, // Less critical
        ));
    }

    // Check for missing Publisher (recommended for published resources)
    if !has_field(resource_text, "Publisher") && !has_caret_field(resource_text, "publisher") {
        diagnostics.push(create_missing_metadata_diagnostic(
            node,
            file_path,
            &resource_name,
            "Publisher",
            resource_type,
            Severity::Info,
        ));
    }

    // Check for missing Contact (recommended)
    if !has_field(resource_text, "Contact") && !has_caret_field(resource_text, "contact") {
        diagnostics.push(create_missing_metadata_diagnostic(
            node,
            file_path,
            &resource_name,
            "Contact",
            resource_type,
            Severity::Info,
        ));
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Extract resource name from declaration
fn extract_resource_name(node: Node, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or("");

    // Try to find the name after the resource type keyword
    // Format: "Profile: Name" or "Extension: Name"
    if let Some(colon_pos) = text.find(':') {
        let after_colon = &text[colon_pos + 1..];
        if let Some(first_line) = after_colon.lines().next() {
            return first_line.trim().to_string();
        }
    }

    "Unknown".to_string()
}

/// Check if a field is present using keyword syntax
/// Example: "Description: Some text"
fn has_field(text: &str, field_name: &str) -> bool {
    let pattern = format!("{}: ", field_name);
    text.contains(&pattern) || text.contains(&format!("{}:", field_name))
}

/// Check if a field is present using caret syntax
/// Example: "^description = \"Some text\""
fn has_caret_field(text: &str, field_name: &str) -> bool {
    let pattern = format!("^{}", field_name);
    text.contains(&pattern)
}

/// Create diagnostic for missing metadata field
fn create_missing_metadata_diagnostic(
    node: Node,
    file_path: &PathBuf,
    resource_name: &str,
    field_name: &str,
    resource_type: &str,
    severity: Severity,
) -> Diagnostic {
    let location = create_location(node, file_path);

    let message = format!(
        "{} '{}' is missing {} field",
        resource_type.replace("_declaration", "").replace('_', " "),
        resource_name,
        field_name
    );

    let mut diagnostic = Diagnostic::new(
        MISSING_METADATA,
        severity,
        &message,
        location.clone(),
    );

    // Suggest adding the field
    let suggestion_text = match field_name {
        "Description" => format!("Description: \"Describes the purpose and usage of {}\"", resource_name),
        "Title" => format!("Title: \"{}\"", format_title(resource_name)),
        "Publisher" => "Publisher: \"Your Organization\"".to_string(),
        "Contact" => "Contact: \"contact@example.org\"".to_string(),
        _ => format!("{}: \"TODO: Add {}\"", field_name, field_name.to_lowercase()),
    };

    diagnostic = diagnostic.with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Add {} field for better documentation", field_name),
        replacement: format!("\n{}", suggestion_text),
        location: location.clone(),
        is_safe: true, // Adding documentation is safe
    });

    diagnostic
}

/// Format resource name into a readable title
fn format_title(name: &str) -> String {
    // Convert PascalCase or kebab-case to Title Case
    name.chars()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && c.is_uppercase() {
                vec![' ', c]
            } else if c == '-' || c == '_' {
                vec![' ']
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .split_whitespace()
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
    fn test_has_field() {
        assert!(has_field("Description: Some text", "Description"));
        assert!(has_field("Title: My Title\nDescription: Text", "Description"));
        assert!(!has_field("Title: My Title", "Description"));
    }

    #[test]
    fn test_has_caret_field() {
        assert!(has_caret_field("^description = \"text\"", "description"));
        assert!(has_caret_field("^title = \"My Title\"", "title"));
        assert!(!has_caret_field("^title = \"text\"", "description"));
    }

    #[test]
    fn test_format_title() {
        assert_eq!(format_title("MyPatientProfile"), "My Patient Profile");
        assert_eq!(format_title("patient-profile"), "Patient Profile");
        assert_eq!(format_title("USCorePatient"), "U S Core Patient");
        assert_eq!(format_title("simple"), "Simple");
    }

    #[test]
    fn test_extract_resource_name() {
        // These would normally work with actual tree-sitter nodes
        // For now we test the format_title helper instead
        assert_eq!(format_title("PatientProfile"), "Patient Profile");
    }

    #[test]
    fn test_metadata_field_detection() {
        let text = r#"
Profile: MyProfile
Description: "A test profile"
Title: "My Profile"
"#;
        assert!(has_field(text, "Description"));
        assert!(has_field(text, "Title"));
        assert!(!has_field(text, "Publisher"));
    }

    #[test]
    fn test_caret_syntax_detection() {
        let text = r#"
Profile: MyProfile
^description = "A test profile"
^title = "My Profile"
"#;
        assert!(has_caret_field(text, "description"));
        assert!(has_caret_field(text, "title"));
        assert!(!has_caret_field(text, "publisher"));
    }

    #[test]
    fn test_mixed_syntax_detection() {
        let text = r#"
Profile: MyProfile
Description: "A test profile"
^title = "My Profile"
"#;
        assert!(has_field(text, "Description"));
        assert!(has_caret_field(text, "title"));
    }
}
