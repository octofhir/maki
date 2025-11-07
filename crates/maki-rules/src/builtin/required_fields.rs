//! Required field validation rules
//!
//! Validates that FHIR resources have required metadata fields:
//! - Parent: Required for Profiles (validated in profile.rs)
//! - Id: Required for all entities (auto-generated from Name in kebab-case)
//! - Title: Required for all entities (auto-generated from Name with spaces)
//! - Description: Recommended (auto-generated with TODO placeholder)

use maki_core::cst::ast::{AstNode, Document};
use maki_core::{CodeSuggestion, Diagnostic, SemanticModel, Severity};

/// Rule IDs for required field validation
pub const REQUIRED_FIELD_MISSING: &str = "blocking/required-field-missing";
pub const REQUIRED_FIELD_PRESENT: &str = "blocking/required-field-present";
pub const REQUIRED_PARENT: &str = "blocking/required-parent";
pub const REQUIRED_ID: &str = "blocking/required-id";
pub const REQUIRED_TITLE: &str = "blocking/required-title";
pub const MISSING_DESCRIPTION: &str = "documentation/missing-description";

/// Convert PascalCase name to kebab-case ID
fn name_to_kebab_case_id(name: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for (i, ch) in name.chars().enumerate() {
        if ch == '_' || ch == ' ' {
            result.push('-');
            prev_was_lower = false;
        } else if ch.is_uppercase() {
            // Add hyphen before uppercase if previous was lowercase (camelCase/PascalCase)
            if i > 0 && prev_was_lower {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_was_lower = false;
        } else {
            result.push(ch);
            prev_was_lower = ch.is_lowercase();
        }
    }

    result
}

/// Convert PascalCase name to "Title Case" by adding spaces
fn name_to_title_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for c in name.chars() {
        if c.is_uppercase() && prev_was_lower {
            result.push(' ');
        }
        result.push(c);
        prev_was_lower = c.is_lowercase();
    }

    result
}

/// Check required fields
pub fn check_required_fields(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        if let Some(name) = profile.name() {
            let location = model.source_map.node_to_diagnostic_location(
                profile.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Parent
            if profile.parent().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_PARENT,
                        Severity::Error,
                        format!("Profile '{name}' must specify a Parent (e.g., 'Parent: Patient')"),
                        location.clone(),
                    )
                    .with_code("profile-missing-parent".to_string()),
                );
            }

            // Check required Id
            if profile.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("Profile '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("profile-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        location.clone(),
                    )),
                );
            }

            // Check required Title
            if profile.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("Profile '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("profile-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        location.clone(),
                    )),
                );
            }

            // Check optional Description
            if profile.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("Profile '{name}' should have a Description"),
                        location,
                    )
                    .with_code("profile-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        model.source_map.node_to_diagnostic_location(
                            profile.syntax(),
                            &model.source,
                            &model.source_file,
                        ),
                    )),
                );
            }
        }
    }

    // Check extensions
    for extension in document.extensions() {
        if let Some(name) = extension.name() {
            let location = model.source_map.node_to_diagnostic_location(
                extension.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if extension.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("Extension '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("extension-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        location.clone(),
                    )),
                );
            }

            // Check required Title
            if extension.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("Extension '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("extension-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        location.clone(),
                    )),
                );
            }

            // Check optional Description
            if extension.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("Extension '{name}' should have a Description"),
                        location,
                    )
                    .with_code("extension-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        model.source_map.node_to_diagnostic_location(
                            extension.syntax(),
                            &model.source,
                            &model.source_file,
                        ),
                    )),
                );
            }
        }
    }

    // Check value sets
    for value_set in document.value_sets() {
        if let Some(name) = value_set.name() {
            let location = model.source_map.node_to_diagnostic_location(
                value_set.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if value_set.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("ValueSet '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("valueset-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        location.clone(),
                    )),
                );
            }

            // Check required Title
            if value_set.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("ValueSet '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("valueset-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        location.clone(),
                    )),
                );
            }

            // Check optional Description
            if value_set.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("ValueSet '{name}' should have a Description"),
                        location,
                    )
                    .with_code("valueset-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        model.source_map.node_to_diagnostic_location(
                            value_set.syntax(),
                            &model.source,
                            &model.source_file,
                        ),
                    )),
                );
            }
        }
    }

    // Check code systems
    for code_system in document.code_systems() {
        if let Some(name) = code_system.name() {
            let location = model.source_map.node_to_diagnostic_location(
                code_system.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if code_system.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("CodeSystem '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("codesystem-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        location.clone(),
                    )),
                );
            }

            // Check required Title
            if code_system.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("CodeSystem '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("codesystem-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        location.clone(),
                    )),
                );
            }

            // Check optional Description
            if code_system.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("CodeSystem '{name}' should have a Description"),
                        location,
                    )
                    .with_code("codesystem-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        model.source_map.node_to_diagnostic_location(
                            code_system.syntax(),
                            &model.source,
                            &model.source_file,
                        ),
                    )),
                );
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::cst::parse_fsh;
    use std::path::PathBuf;

    fn create_test_model(source: &str) -> SemanticModel {
        let (cst, _, _) = parse_fsh(source);
        let source_map = maki_core::SourceMap::new(source);
        SemanticModel {
            cst,
            resources: Vec::new(),
            symbols: Default::default(),
            aliases: maki_core::semantic::AliasTable::new(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
            deferred_rules: maki_core::DeferredRuleQueue::new(),
        }
    }

    #[test]
    fn test_name_to_kebab_case_id() {
        assert_eq!(name_to_kebab_case_id("MyProfile"), "my-profile");
        assert_eq!(name_to_kebab_case_id("PatientProfile"), "patient-profile");
        assert_eq!(
            name_to_kebab_case_id("MyPatientProfileExtension"),
            "my-patient-profile-extension"
        );
        assert_eq!(name_to_kebab_case_id("simple"), "simple");
    }

    #[test]
    fn test_name_to_title_case() {
        assert_eq!(name_to_title_case("MyProfile"), "My Profile");
        assert_eq!(name_to_title_case("PatientProfile"), "Patient Profile");
        assert_eq!(
            name_to_title_case("MyPatientProfileExtension"),
            "My Patient Profile Extension"
        );
        assert_eq!(name_to_title_case("simple"), "simple");
    }

    #[test]
    fn test_profile_missing_parent() {
        let source = "Profile: MyProfile\nId: my-profile\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let parent_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_PARENT)
            .collect();
        assert_eq!(parent_diags.len(), 1);
        assert!(parent_diags[0].message.contains("Parent"));
    }

    #[test]
    fn test_profile_missing_id() {
        let source = "Profile: MyProfile\nParent: Patient\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let id_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_ID)
            .collect();
        assert_eq!(id_diags.len(), 1);
        assert!(id_diags[0].message.contains("Id"));
        assert!(id_diags[0].suggestions.iter().any(|s| s.message.contains("my-profile")));
    }

    #[test]
    fn test_profile_missing_title() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let title_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_TITLE)
            .collect();
        assert_eq!(title_diags.len(), 1);
        assert!(title_diags[0].message.contains("Title"));
        assert!(title_diags[0]
            .suggestions
            .iter()
            .any(|s| s.message.contains("My Profile")));
    }

    #[test]
    fn test_profile_missing_description() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let desc_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == MISSING_DESCRIPTION)
            .collect();
        assert_eq!(desc_diags.len(), 1);
        assert!(desc_diags[0].message.contains("Description"));
        assert!(desc_diags[0]
            .suggestions
            .iter()
            .any(|s| s.replacement.contains("TODO")));
    }

    #[test]
    fn test_profile_all_fields_present() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\nTitle: \"My Profile\"\nDescription: \"Test profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        // Should have no required field diagnostics
        let required_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.rule_id == REQUIRED_PARENT
                    || d.rule_id == REQUIRED_ID
                    || d.rule_id == REQUIRED_TITLE
            })
            .collect();
        assert_eq!(required_diags.len(), 0, "No required fields should be missing");
    }

    #[test]
    fn test_extension_missing_id_and_title() {
        let source = "Extension: MyExtension\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let id_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_ID && d.message.contains("Extension"))
            .collect();
        assert_eq!(id_diags.len(), 1);

        let title_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_TITLE && d.message.contains("Extension"))
            .collect();
        assert_eq!(title_diags.len(), 1);
    }

    #[test]
    fn test_valueset_missing_fields() {
        let source = "ValueSet: MyValueSet\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("ValueSet") && d.message.contains("must specify"))
            .collect();
        assert!(missing.len() >= 2, "Should detect missing Id and Title");
    }

    #[test]
    fn test_codesystem_missing_fields() {
        let source = "CodeSystem: MyCodeSystem\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("CodeSystem") && d.message.contains("must specify"))
            .collect();
        assert!(missing.len() >= 2, "Should detect missing Id and Title");
    }
}
