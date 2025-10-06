//! Profile-specific validation rules
//!
//! Validates Profile and Extension definitions for proper structure and context.

use fsh_lint_core::cst::ast::{AstNode, Document};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for profile assignment validation
pub const PROFILE_PARENT_VALID: &str = "correctness/profile-parent-valid";
pub const PROFILE_ASSIGNMENT_PRESENT: &str = "correctness/profile-assignment-present";
pub const EXTENSION_CONTEXT_MISSING: &str = "correctness/extension-context-missing";

/// Check profile assignments
pub fn check_profile_assignments(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Known FHIR base resources (simplified list - in production, this would be comprehensive)
    let known_resources = [
        "Patient",
        "Observation",
        "Condition",
        "Practitioner",
        "Organization",
        "Encounter",
        "Procedure",
        "MedicationRequest",
        "AllergyIntolerance",
        "DiagnosticReport",
        "CarePlan",
        "Goal",
        "ServiceRequest",
        "Immunization",
        "DocumentReference",
        "Bundle",
        "Extension",
    ];

    // Check that all profiles have a valid parent
    for profile in document.profiles() {
        if let Some(parent_clause) = profile.parent() {
            if let Some(parent_name) = parent_clause.value() {
                // Check if parent is a known FHIR resource or potentially a custom profile
                // For now, we'll just warn if it's not a known base resource
                if !known_resources.contains(&parent_name.as_str()) {
                    // Check if it might be a reference to another profile defined in the model
                    if !model.symbols.contains_symbol(&parent_name) {
                        let location = model.source_map.node_to_diagnostic_location(
                            parent_clause.syntax(),
                            &model.source,
                            &model.source_file,
                        );
                        diagnostics.push(
                            Diagnostic::new(
                                PROFILE_PARENT_VALID,
                                Severity::Warning,
                                format!(
                                    "Profile parent '{parent_name}' is not a known FHIR resource or defined profile"
                                ),
                                location,
                            )
                            .with_code("unknown-profile-parent".to_string()),
                        );
                    }
                }
            }
        } else {
            // Profile missing parent
            if let Some(name) = profile.name() {
                let location = model.source_map.node_to_diagnostic_location(
                    profile.syntax(),
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        PROFILE_ASSIGNMENT_PRESENT,
                        Severity::Error,
                        format!("Profile '{name}' must specify a Parent"),
                        location,
                    )
                    .with_code("missing-profile-parent".to_string()),
                );
            }
        }
    }

    diagnostics
}

/// Check extension context
pub fn check_extension_context(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check that extensions have context specified
    // Context is typically specified with ^context rules
    for extension in document.extensions() {
        // Check if extension has any rules that look like context definitions
        // In FSH, context is defined with: * ^context[+].type = #element
        // For simplicity, we'll check if there are any rules mentioning "context"
        let has_context_rule = extension.rules().any(|rule| {
            let syntax_text = rule.syntax().text().to_string();
            syntax_text.contains("^context") || syntax_text.contains("Context:")
        });

        if !has_context_rule {
            if let Some(name) = extension.name() {
                let location = model.source_map.node_to_diagnostic_location(
                    extension.syntax(),
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        EXTENSION_CONTEXT_MISSING,
                        Severity::Warning,
                        format!(
                            "Extension '{name}' should specify where it can be used with ^context rules"
                        ),
                        location,
                    )
                    .with_code("missing-extension-context".to_string()),
                );
            }
        }
    }

    diagnostics
}
