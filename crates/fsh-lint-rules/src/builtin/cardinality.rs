//! Cardinality validation rules
//!
//! Validates cardinality constraints in FSH element rules.

use fsh_lint_core::ast::{CardRule, Cardinality, CardinalityMax, LRRule, SDRule};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for invalid cardinality
pub const INVALID_CARDINALITY: &str = "correctness/invalid-cardinality";

/// Check for invalid cardinality expressions in an FSH document
pub fn check_cardinality(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check profiles
    for profile in &model.document.profiles {
        for rule in &profile.rules {
            if let SDRule::Card(card_rule) = rule {
                if let Some(diag) = validate_card_rule(card_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check extensions
    for extension in &model.document.extensions {
        for rule in &extension.rules {
            if let SDRule::Card(card_rule) = rule {
                if let Some(diag) = validate_card_rule(card_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check logicals
    for logical in &model.document.logicals {
        for rule in &logical.rules {
            if let LRRule::SD(SDRule::Card(card_rule)) = rule {
                if let Some(diag) = validate_card_rule(card_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    // Check resources
    for resource in &model.document.resources {
        for rule in &resource.rules {
            if let LRRule::SD(SDRule::Card(card_rule)) = rule {
                if let Some(diag) = validate_card_rule(card_rule, model) {
                    diagnostics.push(diag);
                }
            }
        }
    }

    diagnostics
}

/// Validate a single cardinality rule
fn validate_card_rule(card_rule: &CardRule, model: &SemanticModel) -> Option<Diagnostic> {
    let cardinality = &card_rule.cardinality.value;
    let span = &card_rule.cardinality.span;

    // Check for reversed bounds (min > max)
    if let (Some(min), CardinalityMax::Number(max)) = (cardinality.min, &cardinality.max) {
        if min > *max {
            // Use SourceMap for precise location!
            let location = model.source_map.span_to_diagnostic_location(
                span,
                &model.source,
                &model.source_file,
            );
            let swapped = format!("{}..{}", max, min);

            return Some(
                Diagnostic::new(
                    INVALID_CARDINALITY,
                    Severity::Error,
                    &format!(
                        "Upper bound ({}) cannot be less than lower bound ({})",
                        max, min
                    ),
                    location.clone(),
                )
                .with_suggestion(fsh_lint_core::Suggestion {
                    message: format!("Swap bounds to {}", swapped),
                    replacement: swapped,
                    location: location.clone(),
                    is_safe: false,
                }),
            );
        }
    }

    // Note: Our parser already handles syntax validation, so we only need to check semantic issues
    // Invalid syntax like "-1..5", "1..abc", etc. are caught during parsing

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::ast::{FSHDocument, Profile, Spanned};
    use fsh_lint_core::SemanticModel;
    use std::path::PathBuf;

    #[test]
    fn test_valid_cardinality() {
        let source = "Profile: Test\n* name 0..1\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let card_rule = CardRule {
            path: Spanned::new("name".to_string(), 0..4),
            cardinality: Spanned::new(
                Cardinality {
                    min: Some(0),
                    max: CardinalityMax::Number(1),
                },
                5..9,
            ),
            flags: Vec::new(),
            span: 0..9,
        };

        let model = SemanticModel {
            document: FSHDocument::new(0..source.len()),
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let result = validate_card_rule(&card_rule, &model);
        assert!(result.is_none(), "Valid cardinality 0..1 should not produce diagnostic");
    }

    #[test]
    fn test_reversed_bounds() {
        let source = "Profile: Test\n* name 5..2\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let card_rule = CardRule {
            path: Spanned::new("name".to_string(), 0..4),
            cardinality: Spanned::new(
                Cardinality {
                    min: Some(5),
                    max: CardinalityMax::Number(2),
                },
                5..9,
            ),
            flags: Vec::new(),
            span: 0..9,
        };

        let model = SemanticModel {
            document: FSHDocument::new(0..source.len()),
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let result = validate_card_rule(&card_rule, &model);
        assert!(result.is_some(), "Reversed bounds 5..2 should produce diagnostic");

        let diag = result.unwrap();
        assert_eq!(diag.rule_id, INVALID_CARDINALITY);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("cannot be less than"));
    }

    #[test]
    fn test_unbounded_max() {
        let source = "Profile: Test\n* name 1..*\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let card_rule = CardRule {
            path: Spanned::new("name".to_string(), 0..4),
            cardinality: Spanned::new(
                Cardinality {
                    min: Some(1),
                    max: CardinalityMax::Star,
                },
                5..9,
            ),
            flags: Vec::new(),
            span: 0..9,
        };

        let model = SemanticModel {
            document: FSHDocument::new(0..source.len()),
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        };

        let result = validate_card_rule(&card_rule, &model);
        assert!(result.is_none(), "Valid cardinality 1..* should not produce diagnostic");
    }

    #[test]
    fn test_check_cardinality_in_profile() {
        let source = "Profile: TestProfile\n* name 5..2\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let mut doc = FSHDocument::new(0..source.len());

        doc.profiles.push(Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: None,
            title: None,
            description: None,
            rules: vec![SDRule::Card(CardRule {
                path: Spanned::new("name".to_string(), 20..24),
                cardinality: Spanned::new(
                    Cardinality {
                        min: Some(5),
                        max: CardinalityMax::Number(2),
                    },
                    25..29,
                ),
                flags: Vec::new(),
                span: 20..29,
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
            source: source.to_string(),
        };

        let diagnostics = check_cardinality(&model);
        assert_eq!(diagnostics.len(), 1, "Should find 1 cardinality error");
        assert_eq!(diagnostics[0].rule_id, INVALID_CARDINALITY);
    }
}
