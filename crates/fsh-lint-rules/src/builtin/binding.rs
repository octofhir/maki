//! Binding strength validation rules
//!
//! Validates that bindings to value sets have proper strength specifications
//! and use valid strength values.

use fsh_lint_core::cst::ast::{AstNode, Document, ValueSetRule};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for binding strength validation
pub const BINDING_STRENGTH_PRESENT: &str = "blocking/binding-strength-present";

/// Valid binding strengths in FHIR
const VALID_BINDING_STRENGTHS: &[&str] = &["required", "extensible", "preferred", "example"];

/// Check for missing or invalid binding strengths in FSH document
pub fn check_binding_strength(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        diagnostics.extend(check_value_set_bindings(
            profile.rules().filter_map(|r| match r {
                fsh_lint_core::cst::ast::Rule::ValueSet(vs) => Some(vs),
                _ => None,
            }),
            model,
        ));
    }

    // Check extensions
    for extension in document.extensions() {
        diagnostics.extend(check_value_set_bindings(
            extension.rules().filter_map(|r| match r {
                fsh_lint_core::cst::ast::Rule::ValueSet(vs) => Some(vs),
                _ => None,
            }),
            model,
        ));
    }

    diagnostics
}

/// Check value set binding rules for proper strength specification
fn check_value_set_bindings(
    rules: impl Iterator<Item = ValueSetRule>,
    model: &SemanticModel,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in rules {
        match rule.strength() {
            Some(strength) => {
                // Check if the strength is valid
                let strength_lower = strength.to_lowercase();
                if !VALID_BINDING_STRENGTHS.contains(&strength_lower.as_str()) {
                    let location = model.source_map.node_to_diagnostic_location(
                        rule.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            BINDING_STRENGTH_PRESENT,
                            Severity::Error,
                            format!(
                                "Invalid binding strength '{strength}'. Must be one of: required, extensible, preferred, example"
                            ),
                            location,
                        )
                        .with_code("invalid-binding-strength".to_string()),
                    );
                }
            }
            None => {
                // Missing binding strength
                if let Some(value_set_name) = rule.value_set() {
                    let location = model.source_map.node_to_diagnostic_location(
                        rule.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            BINDING_STRENGTH_PRESENT,
                            Severity::Error,
                            format!(
                                "Binding to ValueSet '{value_set_name}' must specify a strength (required, extensible, preferred, or example)"
                            ),
                            location,
                        )
                        .with_code("missing-binding-strength".to_string()),
                    );
                }
            }
        }
    }

    diagnostics
}
