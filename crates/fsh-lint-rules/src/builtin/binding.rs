//! Binding strength validation rules
//!
//! Validates that bindings to value sets have proper strength specifications
//! and use valid strength values.

use fsh_lint_core::ast::{BindingStrength, SDRule, ValueSetRule};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for binding strength validation
pub const BINDING_STRENGTH_PRESENT: &str = "correctness/binding-strength-present";

/// Check for missing or invalid binding strengths in FSH document
pub fn check_binding_strength(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check profiles
    for profile in &model.document.profiles {
        for rule in &profile.rules {
            if let SDRule::ValueSet(vs_rule) = rule {
                if let Some(diag) = validate_binding_strength(vs_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check extensions
    for extension in &model.document.extensions {
        for rule in &extension.rules {
            if let SDRule::ValueSet(vs_rule) = rule {
                if let Some(diag) = validate_binding_strength(vs_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    diagnostics
}

/// Validate a single ValueSet binding rule
fn validate_binding_strength(vs_rule: &ValueSetRule, model: &SemanticModel) -> Option<Diagnostic> {
    // Check if strength is missing
    if vs_rule.strength.is_none() {
        // Use SourceMap for precise location!
        let location = model.source_map.span_to_diagnostic_location(
            &vs_rule.span,
            &model.source,
            &model.source_file,
        );

        return Some(
            Diagnostic::new(
                BINDING_STRENGTH_PRESENT,
                Severity::Error,
                &format!(
                    "Binding to '{}' is missing strength specification. Must be one of: required, extensible, preferred, example",
                    vs_rule.value_set.value
                ),
                location.clone(),
            )
            .with_suggestion(fsh_lint_core::Suggestion {
                message: "Add binding strength".to_string(),
                replacement: format!(
                    "{} from {} (required)",
                    vs_rule.path.value,
                    vs_rule.value_set.value
                ),
                location,
                is_safe: false,
            }),
        );
    }

    // Check if strength is invalid (Unknown variant)
    if let Some(ref strength_spanned) = vs_rule.strength {
        if let BindingStrength::Unknown(ref invalid_strength) = strength_spanned.value {
            // Use SourceMap for precise location!
            let location = model.source_map.span_to_diagnostic_location(
                &strength_spanned.span,
                &model.source,
                &model.source_file,
            );

            return Some(
                Diagnostic::new(
                    BINDING_STRENGTH_PRESENT,
                    Severity::Error,
                    &format!(
                        "Invalid binding strength '{}'. Must be one of: required, extensible, preferred, example",
                        invalid_strength
                    ),
                    location.clone(),
                )
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Use 'required' (strongest)".to_string(),
                    replacement: "required".to_string(),
                    location: location.clone(),
                    is_safe: false,
                })
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Use 'extensible'".to_string(),
                    replacement: "extensible".to_string(),
                    location: location.clone(),
                    is_safe: false,
                })
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Use 'preferred'".to_string(),
                    replacement: "preferred".to_string(),
                    location: location.clone(),
                    is_safe: false,
                })
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: "Use 'example' (weakest)".to_string(),
                    replacement: "example".to_string(),
                    location,
                    is_safe: false,
                }),
            );
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::ast::{FSHDocument, Profile, Spanned};
    use fsh_lint_core::SemanticModel;
    use std::path::PathBuf;

    fn create_test_model() -> SemanticModel {
        let source = "Profile: Test\n* status from StatusVS (required)".to_string();
        let source_map = fsh_lint_core::SourceMap::new(&source);
        SemanticModel {
            document: FSHDocument::new(0..source.len()),
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source,
        }
    }

    #[test]
    fn test_valid_binding_with_strength() {
        let model = create_test_model();
        let vs_rule = ValueSetRule {
            path: Spanned::new("status".to_string(), 0..6),
            value_set: Spanned::new("StatusVS".to_string(), 12..20),
            strength: Some(Spanned::new(BindingStrength::Required, 22..30)),
            span: 0..31,
        };

        let result = validate_binding_strength(&vs_rule, &model);
        assert!(result.is_none(), "Valid binding with strength should not produce diagnostic");
    }

    #[test]
    fn test_binding_missing_strength() {
        let model = create_test_model();
        let vs_rule = ValueSetRule {
            path: Spanned::new("status".to_string(), 0..6),
            value_set: Spanned::new("StatusVS".to_string(), 12..20),
            strength: None,
            span: 0..20,
        };

        let result = validate_binding_strength(&vs_rule, &model);
        assert!(result.is_some(), "Binding without strength should produce diagnostic");

        let diag = result.unwrap();
        assert_eq!(diag.rule_id, BINDING_STRENGTH_PRESENT);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("missing strength"));
    }

    #[test]
    fn test_binding_invalid_strength() {
        let model = create_test_model();
        let vs_rule = ValueSetRule {
            path: Spanned::new("status".to_string(), 0..6),
            value_set: Spanned::new("StatusVS".to_string(), 12..20),
            strength: Some(Spanned::new(BindingStrength::Unknown("invalid".to_string()), 22..29)),
            span: 0..30,
        };

        let result = validate_binding_strength(&vs_rule, &model);
        assert!(result.is_some(), "Binding with invalid strength should produce diagnostic");

        let diag = result.unwrap();
        assert_eq!(diag.rule_id, BINDING_STRENGTH_PRESENT);
        assert!(diag.message.contains("Invalid binding strength"));
        assert!(diag.suggestions.len() == 4, "Should suggest all 4 valid strengths");
    }

    #[test]
    fn test_all_valid_strengths() {
        let model = create_test_model();
        for strength in &[
            BindingStrength::Required,
            BindingStrength::Extensible,
            BindingStrength::Preferred,
            BindingStrength::Example,
        ] {
            let vs_rule = ValueSetRule {
                path: Spanned::new("status".to_string(), 0..6),
                value_set: Spanned::new("StatusVS".to_string(), 12..20),
                strength: Some(Spanned::new(strength.clone(), 22..30)),
                span: 0..31,
            };

            let result = validate_binding_strength(&vs_rule, &model);
            assert!(result.is_none(), "Valid strength {:?} should not produce diagnostic", strength);
        }
    }

    #[test]
    fn test_check_binding_in_profile() {
        let source = "Profile: TestProfile\n* status from StatusVS".to_string();
        let source_map = fsh_lint_core::SourceMap::new(&source);

        let mut doc = FSHDocument::new(0..source.len());

        doc.profiles.push(Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: None,
            title: None,
            description: None,
            rules: vec![SDRule::ValueSet(ValueSetRule {
                path: Spanned::new("status".to_string(), 20..26),
                value_set: Spanned::new("StatusVS".to_string(), 32..40),
                strength: None,
                span: 20..40,
            })],
            span: 0..50,
        });

        let model = SemanticModel {
            document: doc,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source,
        };

        let diagnostics = check_binding_strength(&model);
        assert_eq!(diagnostics.len(), 1, "Should find 1 binding strength error");
        assert_eq!(diagnostics[0].rule_id, BINDING_STRENGTH_PRESENT);
    }
}
