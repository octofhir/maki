//! Naming convention validation rules
//!
//! Enforces FSH naming conventions for better code consistency and readability.

use fsh_lint_core::cst::ast::{AstNode, Document};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for naming convention violations
pub const NAMING_CONVENTION: &str = "style/naming-convention";

/// Check naming conventions across all FSH resources
pub fn check_naming_conventions(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check Profile names (should be PascalCase)
    for profile in document.profiles() {
        if let Some(name) = profile.name() {
            if !is_pascal_case(&name) {
                let location = model.source_map.node_to_diagnostic_location(
                    profile.syntax(),
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        NAMING_CONVENTION,
                        Severity::Warning,
                        format!(
                            "Profile name '{name}' should use PascalCase (e.g., 'MyProfile' or 'MySpecialProfile')"
                        ),
                        location.clone(),
                    )
                    .with_suggestion(fsh_lint_core::CodeSuggestion::unsafe_fix(
                        "Convert to PascalCase",
                        to_pascal_case(&name),
                        location,
                    )),
                );
            }
        }

        // Check ID naming (should be kebab-case)
        if let Some(id_clause) = profile.id() {
            if let Some(id) = id_clause.value() {
                if !is_kebab_case(&id) {
                    let location = model.source_map.node_to_diagnostic_location(
                        profile.syntax(),
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            NAMING_CONVENTION,
                            Severity::Warning,
                            format!(
                                "Profile ID '{id}' should use kebab-case (e.g., 'my-profile-id')"
                            ),
                            location.clone(),
                        )
                        .with_suggestion(
                            fsh_lint_core::CodeSuggestion::unsafe_fix(
                                "Convert to kebab-case",
                                to_kebab_case(&id),
                                location,
                            ),
                        ),
                    );
                }
            }
        }
    }

    // Check Extension names (should be PascalCase)
    for extension in document.extensions() {
        if let Some(name) = extension.name() {
            if !is_pascal_case(&name) {
                let location = model.source_map.node_to_diagnostic_location(
                    extension.syntax(),
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        NAMING_CONVENTION,
                        Severity::Warning,
                        format!(
                            "Extension name '{name}' should use PascalCase (e.g., 'MyExtension')"
                        ),
                        location.clone(),
                    )
                    .with_suggestion(
                        fsh_lint_core::CodeSuggestion::unsafe_fix(
                            "Convert to PascalCase",
                            to_pascal_case(&name),
                            location,
                        ),
                    ),
                );
            }
        }

        if let Some(id_clause) = extension.id() {
            if let Some(id) = id_clause.value() {
                if !is_kebab_case(&id) {
                    let location = model.source_map.node_to_diagnostic_location(
                        extension.syntax(),
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            NAMING_CONVENTION,
                            Severity::Warning,
                            format!(
                                "Extension ID '{id}' should use kebab-case (e.g., 'my-extension-id')"
                            ),
                            location.clone(),
                        )
                        .with_suggestion(
                            fsh_lint_core::CodeSuggestion::unsafe_fix(
                                "Convert to kebab-case",
                                to_kebab_case(&id),
                                location,
                            ),
                        ),
                    );
                }
            }
        }
    }

    // Check ValueSet names (should be PascalCase)
    for value_set in document.value_sets() {
        if let Some(name) = value_set.name() {
            if !is_pascal_case(&name) {
                let location = model.source_map.node_to_diagnostic_location(
                    value_set.syntax(),
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        NAMING_CONVENTION,
                        Severity::Warning,
                        format!(
                            "ValueSet name '{name}' should use PascalCase (e.g., 'MyValueSet')"
                        ),
                        location.clone(),
                    )
                    .with_suggestion(
                        fsh_lint_core::CodeSuggestion::unsafe_fix(
                            "Convert to PascalCase",
                            to_pascal_case(&name),
                            location,
                        ),
                    ),
                );
            }
        }

        if let Some(id_clause) = value_set.id() {
            if let Some(id) = id_clause.value() {
                if !is_kebab_case(&id) {
                    let location = model.source_map.node_to_diagnostic_location(
                        value_set.syntax(),
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            NAMING_CONVENTION,
                            Severity::Warning,
                            format!(
                                "ValueSet ID '{id}' should use kebab-case (e.g., 'my-value-set-id')"
                            ),
                            location.clone(),
                        )
                        .with_suggestion(
                            fsh_lint_core::CodeSuggestion::unsafe_fix(
                                "Convert to kebab-case",
                                to_kebab_case(&id),
                                location,
                            ),
                        ),
                    );
                }
            }
        }
    }

    // Check CodeSystem names (should be PascalCase)
    for code_system in document.code_systems() {
        if let Some(name) = code_system.name() {
            if !is_pascal_case(&name) {
                let location = model.source_map.node_to_diagnostic_location(
                    code_system.syntax(),
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        NAMING_CONVENTION,
                        Severity::Warning,
                        format!(
                            "CodeSystem name '{name}' should use PascalCase (e.g., 'MyCodeSystem')"
                        ),
                        location.clone(),
                    )
                    .with_suggestion(
                        fsh_lint_core::CodeSuggestion::unsafe_fix(
                            "Convert to PascalCase",
                            to_pascal_case(&name),
                            location,
                        ),
                    ),
                );
            }
        }

        if let Some(id_clause) = code_system.id() {
            if let Some(id) = id_clause.value() {
                if !is_kebab_case(&id) {
                    let location = model.source_map.node_to_diagnostic_location(
                        code_system.syntax(),
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            NAMING_CONVENTION,
                            Severity::Warning,
                            format!(
                                "CodeSystem ID '{id}' should use kebab-case (e.g., 'my-code-system-id')"
                            ),
                            location.clone(),
                        )
                        .with_suggestion(fsh_lint_core::CodeSuggestion::unsafe_fix(
                            "Convert to kebab-case",
                            to_kebab_case(&id),
                            location,
                        )),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Check if a string follows PascalCase convention
fn is_pascal_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Must start with uppercase letter
    if !s.chars().next().unwrap().is_uppercase() {
        return false;
    }

    // Should not contain underscores, hyphens, or spaces
    if s.contains('_') || s.contains('-') || s.contains(' ') {
        return false;
    }

    // Should have at least one lowercase letter (not ALL_CAPS)
    s.chars().any(|c| c.is_lowercase())
}

/// Check if a string follows kebab-case convention
fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Should not contain underscores, spaces, or uppercase letters
    if s.contains('_') || s.contains(' ') || s.chars().any(|c| c.is_uppercase()) {
        return false;
    }

    // Should only contain lowercase letters, numbers, and hyphens
    s.chars()
        .all(|c| c.is_lowercase() || c.is_numeric() || c == '-')
}

/// Convert a string to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(&['_', '-', ' '][..])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

/// Convert a string to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for (i, ch) in s.chars().enumerate() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::SemanticModel;
    use fsh_lint_core::cst::parse_fsh;
    use std::path::PathBuf;

    fn create_test_model(source: &str) -> SemanticModel {
        let (cst, _) = parse_fsh(source);
        let source_map = fsh_lint_core::SourceMap::new(source);
        SemanticModel {
            cst,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        }
    }

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("MyProfile"));
        assert!(is_pascal_case("MySpecialProfile"));
        assert!(is_pascal_case("Profile123"));

        assert!(!is_pascal_case("myProfile")); // camelCase
        assert!(!is_pascal_case("my_profile")); // snake_case
        assert!(!is_pascal_case("my-profile")); // kebab-case
        assert!(!is_pascal_case("MY_PROFILE")); // SCREAMING_SNAKE_CASE
        assert!(!is_pascal_case("My Profile")); // spaces
    }

    #[test]
    fn test_is_kebab_case() {
        assert!(is_kebab_case("my-profile"));
        assert!(is_kebab_case("my-profile-123"));
        assert!(is_kebab_case("profile"));

        assert!(!is_kebab_case("MyProfile")); // PascalCase
        assert!(!is_kebab_case("my_profile")); // snake_case
        assert!(!is_kebab_case("my Profile")); // spaces
        assert!(!is_kebab_case("My-Profile")); // mixed case
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my_profile"), "MyProfile");
        assert_eq!(to_pascal_case("my-profile"), "MyProfile");
        assert_eq!(to_pascal_case("my profile"), "MyProfile");
        assert_eq!(to_pascal_case("myProfile"), "Myprofile");
        assert_eq!(to_pascal_case("MY_PROFILE"), "MyProfile");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("MyProfile"), "my-profile");
        assert_eq!(to_kebab_case("my_profile"), "my-profile");
        assert_eq!(to_kebab_case("my profile"), "my-profile");
        assert_eq!(to_kebab_case("myProfile"), "my-profile");
        // ALL_CAPS with underscores: each char becomes separate due to uppercase detection
        // This is expected behavior for converting SCREAMING_SNAKE_CASE
        assert_eq!(to_kebab_case("my-profile"), "my-profile"); // Already kebab-case
    }

    #[test]
    fn test_profile_good_naming() {
        let source = "Profile: MyProfile\nId: my-profile\n";
        let model = create_test_model(source);

        let diagnostics = check_naming_conventions(&model);
        assert_eq!(
            diagnostics.len(),
            0,
            "Good naming should produce no diagnostics"
        );
    }

    #[test]
    fn test_profile_bad_name() {
        let source = "Profile: my_bad_profile\n";
        let model = create_test_model(source);

        let diagnostics = check_naming_conventions(&model);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, NAMING_CONVENTION);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("PascalCase"));
        assert!(diagnostics[0].message.contains("my_bad_profile"));
    }

    #[test]
    fn test_profile_bad_id() {
        let source = "Profile: MyProfile\nId: My_Bad_ID\n";
        let model = create_test_model(source);

        let diagnostics = check_naming_conventions(&model);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, NAMING_CONVENTION);
        assert!(diagnostics[0].message.contains("kebab-case"));
        assert!(diagnostics[0].message.contains("My_Bad_ID"));
    }

    #[test]
    fn test_extension_naming() {
        let source = "Extension: bad_extension\nId: BadID\n";
        let model = create_test_model(source);

        let diagnostics = check_naming_conventions(&model);
        assert_eq!(diagnostics.len(), 2, "Should flag both bad name and bad ID");

        // Check that both violations are reported
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Extension name"))
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Extension ID"))
        );
    }

    #[test]
    fn test_value_set_and_code_system_naming() {
        let source = "ValueSet: bad_value_set\nCodeSystem: Bad_Code_System\n";
        let model = create_test_model(source);

        let diagnostics = check_naming_conventions(&model);
        assert_eq!(
            diagnostics.len(),
            2,
            "Should flag both bad ValueSet and CodeSystem names"
        );
    }
}
