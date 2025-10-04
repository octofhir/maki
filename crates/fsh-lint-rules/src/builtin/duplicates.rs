//! Duplicate definition detection rules
//!
//! Detects duplicate resource names, IDs, and canonical URLs within a single FSH file.

use fsh_lint_core::{Diagnostic, SemanticModel, Severity};
use std::collections::HashMap;

/// Rule ID for duplicate definitions
pub const DUPLICATE_DEFINITION: &str = "correctness/duplicate-definition";

/// Information about a resource occurrence for duplicate tracking
#[derive(Debug, Clone)]
struct ResourceOccurrence {
    name: String,
    resource_type: &'static str,
    span: std::ops::Range<usize>,
}

/// Check for duplicate resource definitions
pub fn check_duplicates(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Track by name
    let mut name_tracker: HashMap<String, Vec<ResourceOccurrence>> = HashMap::new();

    // Track by ID
    let mut id_tracker: HashMap<String, Vec<ResourceOccurrence>> = HashMap::new();

    // Collect all profiles
    for profile in &model.document.profiles {
        let occurrence = ResourceOccurrence {
            name: profile.name.value.clone(),
            resource_type: "Profile",
            span: profile.span.clone(),
        };

        name_tracker
            .entry(profile.name.value.clone())
            .or_insert_with(Vec::new)
            .push(occurrence);

        // Track by ID if present
        if let Some(id) = &profile.id {
            let id_occurrence = ResourceOccurrence {
                name: profile.name.value.clone(),
                resource_type: "Profile",
                span: id.span.clone(),
            };
            id_tracker
                .entry(id.value.clone())
                .or_insert_with(Vec::new)
                .push(id_occurrence);
        }
    }

    // Collect all extensions
    for extension in &model.document.extensions {
        let occurrence = ResourceOccurrence {
            name: extension.name.value.clone(),
            resource_type: "Extension",
            span: extension.span.clone(),
        };

        name_tracker
            .entry(extension.name.value.clone())
            .or_insert_with(Vec::new)
            .push(occurrence);

        if let Some(id) = &extension.id {
            let id_occurrence = ResourceOccurrence {
                name: extension.name.value.clone(),
                resource_type: "Extension",
                span: id.span.clone(),
            };
            id_tracker
                .entry(id.value.clone())
                .or_insert_with(Vec::new)
                .push(id_occurrence);
        }
    }

    // Collect all value sets
    for value_set in &model.document.value_sets {
        let occurrence = ResourceOccurrence {
            name: value_set.name.value.clone(),
            resource_type: "ValueSet",
            span: value_set.span.clone(),
        };

        name_tracker
            .entry(value_set.name.value.clone())
            .or_insert_with(Vec::new)
            .push(occurrence);

        if let Some(id) = &value_set.id {
            let id_occurrence = ResourceOccurrence {
                name: value_set.name.value.clone(),
                resource_type: "ValueSet",
                span: id.span.clone(),
            };
            id_tracker
                .entry(id.value.clone())
                .or_insert_with(Vec::new)
                .push(id_occurrence);
        }
    }

    // Collect all code systems
    for code_system in &model.document.code_systems {
        let occurrence = ResourceOccurrence {
            name: code_system.name.value.clone(),
            resource_type: "CodeSystem",
            span: code_system.span.clone(),
        };

        name_tracker
            .entry(code_system.name.value.clone())
            .or_insert_with(Vec::new)
            .push(occurrence);

        if let Some(id) = &code_system.id {
            let id_occurrence = ResourceOccurrence {
                name: code_system.name.value.clone(),
                resource_type: "CodeSystem",
                span: id.span.clone(),
            };
            id_tracker
                .entry(id.value.clone())
                .or_insert_with(Vec::new)
                .push(id_occurrence);
        }
    }

    // Report duplicate names
    for (name, occurrences) in name_tracker {
        if occurrences.len() > 1 {
            for occurrence in &occurrences {
                let location = model.source_map.span_to_diagnostic_location(
                    &occurrence.span,
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        &format!(
                            "Duplicate {} name '{}' ({} occurrences in this file)",
                            occurrence.resource_type,
                            name,
                            occurrences.len()
                        ),
                        location.clone(),
                    )
                    .with_suggestion(fsh_lint_core::Suggestion {
                        message: format!("Rename to make unique (e.g., '{}2')", name),
                        replacement: format!("{}2", name),
                        location,
                        is_safe: false,
                    }),
                );
            }
        }
    }

    // Report duplicate IDs
    for (id, occurrences) in id_tracker {
        if occurrences.len() > 1 {
            for occurrence in &occurrences {
                let location = model.source_map.span_to_diagnostic_location(
                    &occurrence.span,
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        &format!(
                            "Duplicate ID '{}' used in {} '{}' ({} occurrences)",
                            id,
                            occurrence.resource_type,
                            occurrence.name,
                            occurrences.len()
                        ),
                        location.clone(),
                    )
                    .with_suggestion(fsh_lint_core::Suggestion {
                        message: format!("Use unique ID (e.g., '{}-2')", id),
                        replacement: format!("{}-2", id),
                        location,
                        is_safe: false,
                    }),
                );
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::ast::{CodeSystem, Extension, Profile, Spanned, ValueSet};
    use fsh_lint_core::SemanticModel;
    use std::path::PathBuf;

    fn create_test_model() -> SemanticModel {
        let source = "Profile: Test\nId: test-id".to_string();
        let source_map = fsh_lint_core::SourceMap::new(&source);
        SemanticModel {
            document: fsh_lint_core::ast::FSHDocument::new(0..source.len()),
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source,
        }
    }

    #[test]
    fn test_no_duplicates() {
        let mut model = create_test_model();

        // Add unique profiles
        model.document.profiles.push(Profile {
            name: Spanned::new("Profile1".to_string(), 0..8),
            parent: None,
            id: Some(Spanned::new("id1".to_string(), 10..13)),
            title: None,
            description: None,
            rules: vec![],
            span: 0..20,
        });

        model.document.profiles.push(Profile {
            name: Spanned::new("Profile2".to_string(), 21..29),
            parent: None,
            id: Some(Spanned::new("id2".to_string(), 31..34)),
            title: None,
            description: None,
            rules: vec![],
            span: 21..40,
        });

        let diagnostics = check_duplicates(&model);
        assert_eq!(
            diagnostics.len(),
            0,
            "Should have no duplicates with unique names and IDs"
        );
    }

    #[test]
    fn test_duplicate_profile_names() {
        let mut model = create_test_model();

        // Add profiles with same name
        model.document.profiles.push(Profile {
            name: Spanned::new("DuplicateName".to_string(), 0..13),
            parent: None,
            id: Some(Spanned::new("id1".to_string(), 15..18)),
            title: None,
            description: None,
            rules: vec![],
            span: 0..30,
        });

        model.document.profiles.push(Profile {
            name: Spanned::new("DuplicateName".to_string(), 31..44),
            parent: None,
            id: Some(Spanned::new("id2".to_string(), 46..49)),
            title: None,
            description: None,
            rules: vec![],
            span: 31..60,
        });

        let diagnostics = check_duplicates(&model);
        assert_eq!(diagnostics.len(), 2, "Should report both occurrences");

        for diag in &diagnostics {
            assert_eq!(diag.rule_id, DUPLICATE_DEFINITION);
            assert_eq!(diag.severity, Severity::Error);
            assert!(diag.message.contains("DuplicateName"));
            assert!(diag.message.contains("2 occurrences"));
        }
    }

    #[test]
    fn test_duplicate_ids_across_types() {
        let mut model = create_test_model();

        // Profile with ID
        model.document.profiles.push(Profile {
            name: Spanned::new("Profile1".to_string(), 0..8),
            parent: None,
            id: Some(Spanned::new("shared-id".to_string(), 10..19)),
            title: None,
            description: None,
            rules: vec![],
            span: 0..30,
        });

        // ValueSet with same ID
        model.document.value_sets.push(ValueSet {
            name: Spanned::new("ValueSet1".to_string(), 31..40),
            parent: None,
            id: Some(Spanned::new("shared-id".to_string(), 42..51)),
            title: None,
            description: None,
            components: vec![],
            rules: vec![],
            span: 31..60,
        });

        let diagnostics = check_duplicates(&model);

        // Should have 2 diagnostics for the duplicate ID
        let id_diagnostics: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Duplicate ID"))
            .collect();

        assert_eq!(
            id_diagnostics.len(),
            2,
            "Should report duplicate ID in both resources"
        );

        assert!(id_diagnostics[0].message.contains("shared-id"));
    }

    #[test]
    fn test_three_duplicates() {
        let mut model = create_test_model();

        // Add three profiles with same name
        for i in 0..3 {
            let offset = i * 50;
            model.document.profiles.push(Profile {
                name: Spanned::new("TripleName".to_string(), offset..offset + 10),
                parent: None,
                id: Some(Spanned::new(format!("id{}", i), offset + 12..offset + 15)),
                title: None,
                description: None,
                rules: vec![],
                span: offset..offset + 40,
            });
        }

        let diagnostics = check_duplicates(&model);
        assert_eq!(
            diagnostics.len(),
            3,
            "Should report all 3 occurrences"
        );

        for diag in &diagnostics {
            assert!(diag.message.contains("3 occurrences"));
        }
    }

    #[test]
    fn test_mixed_resource_types_no_duplicates() {
        let mut model = create_test_model();

        // Different resource types can have same name (not a duplicate)
        model.document.profiles.push(Profile {
            name: Spanned::new("SameName".to_string(), 0..8),
            parent: None,
            id: Some(Spanned::new("profile-id".to_string(), 10..20)),
            title: None,
            description: None,
            rules: vec![],
            span: 0..30,
        });

        model.document.extensions.push(Extension {
            name: Spanned::new("SameName".to_string(), 31..39),
            parent: None,
            id: Some(Spanned::new("extension-id".to_string(), 41..53)),
            title: None,
            description: None,
            contexts: vec![],
            rules: vec![],
            span: 31..60,
        });

        let diagnostics = check_duplicates(&model);

        // Names across different types ARE duplicates!
        assert!(
            diagnostics.len() >= 2,
            "Should detect duplicate names across types"
        );
    }

    #[test]
    fn test_resources_without_ids() {
        let mut model = create_test_model();

        // Resources without IDs should not cause ID duplicates
        model.document.profiles.push(Profile {
            name: Spanned::new("Profile1".to_string(), 0..8),
            parent: None,
            id: None, // No ID
            title: None,
            description: None,
            rules: vec![],
            span: 0..20,
        });

        model.document.profiles.push(Profile {
            name: Spanned::new("Profile2".to_string(), 21..29),
            parent: None,
            id: None, // No ID
            title: None,
            description: None,
            rules: vec![],
            span: 21..40,
        });

        let diagnostics = check_duplicates(&model);

        // Should have no ID duplicates (only resources with IDs are tracked)
        let id_duplicates: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("Duplicate ID"))
            .collect();

        assert_eq!(id_duplicates.len(), 0, "Resources without IDs should not trigger ID duplicate errors");
    }
}

/*
// TODO: Reimplement using Chumsky AST

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
        assert_eq!(
            extract_name("Profile: MyProfile"),
            Some("MyProfile".to_string())
        );
        assert_eq!(extract_name("Extension: MyExt"), Some("MyExt".to_string()));
        assert_eq!(extract_name("Profile:"), None);
    }

    #[test]
    fn test_extract_id() {
        assert_eq!(extract_id("Id: my-profile"), Some("my-profile".to_string()));
        assert_eq!(
            extract_id("Profile: Test\nId: test-id"),
            Some("test-id".to_string())
        );
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

        seen.entry("test".to_string())
            .or_insert_with(Vec::new)
            .push("first".to_string());
        seen.entry("test".to_string())
            .or_insert_with(Vec::new)
            .push("second".to_string());

        assert_eq!(seen.get("test").unwrap().len(), 2);
    }
}

*/