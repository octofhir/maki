//! Cardinality validation rules
//!
//! Validates cardinality constraints in FSH profiles and extensions.

use maki_core::cst::ast::{AstNode, CardRule, Document};
use maki_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for cardinality validation
pub const VALID_CARDINALITY: &str = "blocking/valid-cardinality";
pub const INVALID_CARDINALITY: &str = "blocking/invalid-cardinality";

/// Check cardinality rules
pub fn check_cardinality(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        diagnostics.extend(check_resource_cardinality_rules(
            profile.rules().filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            }),
            model,
        ));
    }

    // Check extensions
    for extension in document.extensions() {
        diagnostics.extend(check_resource_cardinality_rules(
            extension.rules().filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            }),
            model,
        ));
    }

    diagnostics
}

/// Check cardinality rules in a resource
fn check_resource_cardinality_rules(
    rules: impl Iterator<Item = CardRule>,
    model: &SemanticModel,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in rules {
        if let Some(cardinality_node) = rule.cardinality() {
            // Parse cardinality: "min..max"
            let cardinality_str = cardinality_node.as_string();
            if let Some((min_str, max_str)) = cardinality_str.split_once("..") {
                let min_result = min_str.trim().parse::<u32>();
                let max_result = if max_str.trim() == "*" {
                    Ok(None)
                } else {
                    max_str.trim().parse::<u32>().map(Some)
                };

                match (min_result, max_result) {
                    (Ok(min), Ok(max_opt)) => {
                        // Check: upper bound cannot be less than lower bound
                        if let Some(max) = max_opt
                            && max < min
                        {
                            let location = model.source_map.node_to_diagnostic_location(
                                rule.syntax(),
                                &model.source,
                                &model.source_file,
                            );
                            diagnostics.push(
                                    Diagnostic::new(
                                        INVALID_CARDINALITY,
                                        Severity::Error,
                                        format!(
                                            "Invalid cardinality: upper bound ({max}) cannot be less than lower bound ({min})"
                                        ),
                                        location,
                                    )
                                    .with_code("invalid-cardinality".to_string()),
                                );
                        }
                    }
                    _ => {
                        // Invalid cardinality syntax (non-numeric or malformed)
                        let location = model.source_map.node_to_diagnostic_location(
                            rule.syntax(),
                            &model.source,
                            &model.source_file,
                        );
                        diagnostics.push(
                            Diagnostic::new(
                                INVALID_CARDINALITY,
                                Severity::Error,
                                format!(
                                    "Invalid cardinality syntax: '{cardinality_str}'. Expected format: 'min..max' (e.g., '0..1', '1..*')"
                                ),
                                location,
                            )
                            .with_code("invalid-cardinality-syntax".to_string()),
                        );
                    }
                }
            } else {
                // Malformed cardinality (no "..")
                let location = model.source_map.node_to_diagnostic_location(
                    rule.syntax(),
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        INVALID_CARDINALITY,
                        Severity::Error,
                        format!(
                            "Invalid cardinality format: '{cardinality_str}'. Expected 'min..max'"
                        ),
                        location,
                    )
                    .with_code("malformed-cardinality".to_string()),
                );
            }
        }
    }

    diagnostics
}
