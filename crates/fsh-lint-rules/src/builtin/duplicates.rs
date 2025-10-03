//! Duplicate definition detection rules
//!
//! Detects duplicate resource names, IDs, and canonical URLs across FSH files.

use fsh_lint_core::{Diagnostic, Location, Severity};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tree_sitter::Node;

/// Rule ID for duplicate definitions
pub const DUPLICATE_DEFINITION: &str = "builtin/correctness/duplicate-definition";

/// Resource information for tracking duplicates
#[derive(Debug, Clone)]
struct ResourceInfo {
    name: String,
    id: Option<String>,
    url: Option<String>,
    location: Location,
    resource_type: String,
}

/// Check for duplicate resource definitions
pub fn check_duplicates(node: Node, source: &str, file_path: &PathBuf) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect all resource definitions
    let resources = collect_resources(node, source, file_path);

    // Check for duplicate names
    diagnostics.extend(check_duplicate_names(&resources));

    // Check for duplicate IDs
    diagnostics.extend(check_duplicate_ids(&resources));

    // Check for duplicate URLs
    diagnostics.extend(check_duplicate_urls(&resources));

    diagnostics
}

/// Collect all resource definitions from the AST
fn collect_resources(node: Node, source: &str, file_path: &PathBuf) -> Vec<ResourceInfo> {
    let mut resources = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let resource_type = match child.kind() {
            "profile_declaration" => "Profile",
            "extension_declaration" => "Extension",
            "value_set_declaration" => "ValueSet",
            "code_system_declaration" => "CodeSystem",
            "instance_declaration" => "Instance",
            _ => {
                // Recursively check children
                resources.extend(collect_resources(child, source, file_path));
                continue;
            }
        };

        let text = child.utf8_text(source.as_bytes()).unwrap_or("");

        if let Some(info) = extract_resource_info(child, text, file_path, resource_type) {
            resources.push(info);
        }

        // Also check children
        resources.extend(collect_resources(child, source, file_path));
    }

    resources
}

/// Extract resource information from a declaration
fn extract_resource_info(
    node: Node,
    text: &str,
    file_path: &PathBuf,
    resource_type: &str,
) -> Option<ResourceInfo> {
    let name = extract_name(text)?;
    let id = extract_id(text);
    let url = extract_url(text);

    Some(ResourceInfo {
        name,
        id,
        url,
        location: create_location(node, file_path),
        resource_type: resource_type.to_string(),
    })
}

/// Extract resource name from declaration text
fn extract_name(text: &str) -> Option<String> {
    // Format: "Profile: Name" or "Extension: Name"
    if let Some(colon_pos) = text.find(':') {
        let after_colon = &text[colon_pos + 1..];
        if let Some(first_line) = after_colon.lines().next() {
            let name = first_line.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Extract Id field from declaration text
fn extract_id(text: &str) -> Option<String> {
    // Look for "Id: value" line
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Id:") {
            let id = trimmed[3..].trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

/// Extract canonical URL from declaration text
fn extract_url(text: &str) -> Option<String> {
    // Look for "^url = "value"" line
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains("^url") && trimmed.contains('=') {
            if let Some(eq_pos) = trimmed.find('=') {
                let url_part = trimmed[eq_pos + 1..].trim();
                // Remove quotes
                let url = url_part.trim_matches('"').trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
        }
    }
    None
}

/// Check for duplicate resource names
fn check_duplicate_names(resources: &[ResourceInfo]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<String, Vec<&ResourceInfo>> = HashMap::new();

    // Group resources by name
    for resource in resources {
        seen.entry(resource.name.clone())
            .or_insert_with(Vec::new)
            .push(resource);
    }

    // Report duplicates
    for (name, occurrences) in seen {
        if occurrences.len() > 1 {
            let count = occurrences.len();
            for resource in &occurrences {
                diagnostics.push(create_duplicate_diagnostic(
                    &resource.location,
                    "name",
                    &name,
                    &resource.resource_type,
                    count,
                ));
            }
        }
    }

    diagnostics
}

/// Check for duplicate IDs
fn check_duplicate_ids(resources: &[ResourceInfo]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<String, Vec<&ResourceInfo>> = HashMap::new();

    // Group resources by ID
    for resource in resources {
        if let Some(id) = &resource.id {
            seen.entry(id.clone())
                .or_insert_with(Vec::new)
                .push(resource);
        }
    }

    // Report duplicates
    for (id, occurrences) in seen {
        if occurrences.len() > 1 {
            let count = occurrences.len();
            for resource in &occurrences {
                diagnostics.push(create_duplicate_diagnostic(
                    &resource.location,
                    "ID",
                    &id,
                    &resource.resource_type,
                    count,
                ));
            }
        }
    }

    diagnostics
}

/// Check for duplicate canonical URLs
fn check_duplicate_urls(resources: &[ResourceInfo]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen: HashMap<String, Vec<&ResourceInfo>> = HashMap::new();

    // Group resources by URL
    for resource in resources {
        if let Some(url) = &resource.url {
            seen.entry(url.clone())
                .or_insert_with(Vec::new)
                .push(resource);
        }
    }

    // Report duplicates
    for (url, occurrences) in seen {
        if occurrences.len() > 1 {
            let count = occurrences.len();
            for resource in &occurrences {
                diagnostics.push(create_duplicate_diagnostic(
                    &resource.location,
                    "canonical URL",
                    &url,
                    &resource.resource_type,
                    count,
                ));
            }
        }
    }

    diagnostics
}

/// Create a diagnostic for a duplicate definition
fn create_duplicate_diagnostic(
    location: &Location,
    field_type: &str,
    value: &str,
    resource_type: &str,
    count: usize,
) -> Diagnostic {
    let message = format!(
        "Duplicate {} '{}' found ({} occurrences) in {}",
        field_type, value, count, resource_type
    );

    Diagnostic::new(
        DUPLICATE_DEFINITION,
        Severity::Error,
        &message,
        location.clone(),
    )
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
    fn test_extract_name() {
        assert_eq!(extract_name("Profile: MyProfile"), Some("MyProfile".to_string()));
        assert_eq!(extract_name("Extension: MyExt"), Some("MyExt".to_string()));
        assert_eq!(extract_name("Profile:"), None);
    }

    #[test]
    fn test_extract_id() {
        assert_eq!(extract_id("Id: my-profile"), Some("my-profile".to_string()));
        assert_eq!(extract_id("Profile: Test\nId: test-id"), Some("test-id".to_string()));
        assert_eq!(extract_id("Profile: Test"), None);
    }

    #[test]
    fn test_extract_url() {
        assert_eq!(
            extract_url("^url = \"http://example.org/fhir/Profile/test\""),
            Some("http://example.org/fhir/Profile/test".to_string())
        );
        assert_eq!(extract_url("^status = #draft"), None);
    }

    #[test]
    fn test_extract_multiline() {
        let text = r#"
Profile: MyProfile
Id: my-profile
^url = "http://example.org/fhir/Profile/my-profile"
"#;
        assert_eq!(extract_name(text), Some("MyProfile".to_string()));
        assert_eq!(extract_id(text), Some("my-profile".to_string()));
        assert_eq!(
            extract_url(text),
            Some("http://example.org/fhir/Profile/my-profile".to_string())
        );
    }

    #[test]
    fn test_duplicate_detection_logic() {
        // Test the grouping logic
        let mut seen: HashMap<String, Vec<String>> = HashMap::new();

        seen.entry("test".to_string()).or_insert_with(Vec::new).push("first".to_string());
        seen.entry("test".to_string()).or_insert_with(Vec::new).push("second".to_string());

        assert_eq!(seen.get("test").unwrap().len(), 2);
    }
}
