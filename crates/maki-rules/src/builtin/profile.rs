//! Profile-specific validation rules
//!
//! Validates Profile and Extension definitions for proper structure and context.
//!
//! ## Parent Validation
//!
//! The `check_profile_assignments` function validates the Parent keyword in Profile definitions.
//! According to the FSH specification, the Parent keyword is required and can reference:
//!
//! 1. **FHIR base resources** - e.g., Patient, Observation, Condition (150+ R4/R5 resources)
//! 2. **Locally-defined profiles** - Profiles defined in the same project/file
//! 3. **External IG profiles** - Profiles from implementation guides (e.g., USCore, mCODE)
//! 4. **Canonical URLs** - Full URLs like `http://hl7.org/fhir/us/core/StructureDefinition/...`
//!
//! ### Validation Rules
//!
//! - **Valid (no diagnostic)**: Parent is a FHIR base resource, locally-defined profile, or valid canonical URL
//! - **Warning**: Parent appears to be from an external IG (USCore*, mcode-*, etc.) or is unknown
//! - **Error**: Parent is missing, or canonical URL format is invalid
//!
//! ### Examples
//!
//! ```fsh
//! // ✅ Valid - FHIR base resource
//! Profile: MyPatientProfile
//! Parent: Patient
//!
//! // ✅ Valid - Local profile
//! Profile: ExtendedPatientProfile
//! Parent: MyPatientProfile
//!
//! // ✅ Valid - Canonical URL
//! Profile: CustomProfile
//! Parent: http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient
//!
//! // ⚠️ Warning - External IG (cannot verify locally)
//! Profile: MyUSCoreExtension
//! Parent: USCorePatientProfile
//!
//! // ⚠️ Warning - Unknown (might be typo)
//! Profile: ProblemProfile
//! Parent: UnknownResourceType
//!
//! // ❌ Error - Missing Parent
//! Profile: BrokenProfile
//! Id: broken
//! ```

use crate::fhir_registry::{
    FhirVersion, is_canonical_url, is_fhir_resource, is_likely_external_profile,
    validate_canonical_url,
};
use maki_core::cst::ast::{AstNode, Document};
use maki_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for profile assignment validation
pub const PROFILE_PARENT_VALID: &str = "correctness/profile-parent-valid";
pub const PROFILE_ASSIGNMENT_PRESENT: &str = "correctness/profile-assignment-present";
pub const EXTENSION_CONTEXT_MISSING: &str = "correctness/extension-context-missing";

/// Check profile assignments
pub fn check_profile_assignments(model: &SemanticModel) -> Vec<Diagnostic> {
    check_profile_assignments_with_version(model, FhirVersion::R4)
}

/// Check profile assignments with specific FHIR version
///
/// This validates that Profile Parent keywords reference valid targets:
/// - FHIR base resources (Patient, Observation, etc.)
/// - Locally-defined profiles or extensions (in symbol table)
/// - Canonical URLs (http://... or https://...)
/// - Known external IG profiles (USCore*, mcode-*, etc.)
pub fn check_profile_assignments_with_version(
    model: &SemanticModel,
    fhir_version: FhirVersion,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check that all profiles have a valid parent
    for profile in document.profiles() {
        if let Some(parent_clause) = profile.parent() {
            if let Some(parent_name) = parent_clause.value() {
                // Validate the parent value
                let validation_result = validate_parent_value(&parent_name, model, fhir_version);

                match validation_result {
                    ParentValidationResult::Valid => {
                        // All good, no diagnostic needed
                    }
                    ParentValidationResult::Warning { message, help } => {
                        let location = model.source_map.node_to_diagnostic_location(
                            parent_clause.syntax(),
                            &model.source,
                            &model.source_file,
                        );

                        // Include help text in the message if provided
                        let full_message = if let Some(help_text) = help {
                            format!("{message}\n  Help: {help_text}")
                        } else {
                            message
                        };

                        diagnostics.push(
                            Diagnostic::new(
                                PROFILE_PARENT_VALID,
                                Severity::Warning,
                                full_message,
                                location,
                            )
                            .with_code("unknown-profile-parent".to_string()),
                        );
                    }
                    ParentValidationResult::Error { message, help } => {
                        let location = model.source_map.node_to_diagnostic_location(
                            parent_clause.syntax(),
                            &model.source,
                            &model.source_file,
                        );

                        // Include help text in the message if provided
                        let full_message = if let Some(help_text) = help {
                            format!("{message}\n  Help: {help_text}")
                        } else {
                            message
                        };

                        diagnostics.push(
                            Diagnostic::new(
                                PROFILE_PARENT_VALID,
                                Severity::Error,
                                full_message,
                                location,
                            )
                            .with_code("invalid-profile-parent".to_string()),
                        );
                    }
                }
            }
        } else {
            // Profile missing parent - this is an error
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

/// Result of parent validation
enum ParentValidationResult {
    Valid,
    Warning {
        message: String,
        help: Option<String>,
    },
    Error {
        message: String,
        help: Option<String>,
    },
}

/// Validate a parent value against multiple criteria
fn validate_parent_value(
    parent_name: &str,
    model: &SemanticModel,
    fhir_version: FhirVersion,
) -> ParentValidationResult {
    // 1. Check if it's a FHIR base resource
    if is_fhir_resource(parent_name, fhir_version) {
        return ParentValidationResult::Valid;
    }

    // 2. Check if it's a locally-defined profile/extension in the symbol table
    // Note: Symbol table keys are resource IDs, but Parent can reference by name
    // So we need to check both the symbol table AND the resources list
    if model.symbols.contains_symbol(parent_name) {
        return ParentValidationResult::Valid;
    }

    // Also check if any resource has this as its name (not just ID)
    if model
        .resources
        .iter()
        .any(|r| r.name.as_ref().is_some_and(|n| n == parent_name))
    {
        return ParentValidationResult::Valid;
    }

    // 3. Check if it's a canonical URL
    if is_canonical_url(parent_name) {
        // Validate URL structure
        match validate_canonical_url(parent_name) {
            Ok(_) => {
                // Valid URL - we can't verify it exists, but format is correct
                return ParentValidationResult::Valid;
            }
            Err(error_msg) => {
                return ParentValidationResult::Error {
                    message: format!("Invalid canonical URL format: {error_msg}"),
                    help: Some(
                        "Ensure the URL follows the pattern: http(s)://domain/StructureDefinition/profile-id"
                            .to_string(),
                    ),
                };
            }
        }
    }

    // 4. Check if it looks like a known external IG profile
    if is_likely_external_profile(parent_name) {
        return ParentValidationResult::Warning {
            message: format!(
                "Parent '{parent_name}' appears to be from an external implementation guide and cannot be verified locally"
            ),
            help: Some(
                "Consider using the canonical URL instead, or ensure this profile is defined in your dependencies"
                    .to_string(),
            ),
        };
    }

    // 5. Unknown parent - could be a typo or missing definition
    ParentValidationResult::Warning {
        message: format!(
            "Parent '{parent_name}' is not a known FHIR resource, locally-defined profile, or recognized external profile"
        ),
        help: Some(
            "Verify the spelling, ensure the parent profile is defined, or use a canonical URL"
                .to_string(),
        ),
    }
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
