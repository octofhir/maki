//! Required field validation rules
//!
//! Validates that FHIR resources have required metadata fields.

use maki_core::cst::ast::{AstNode, Document};
use maki_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for required field validation
pub const REQUIRED_FIELD_MISSING: &str = "blocking/required-field-missing";
pub const REQUIRED_FIELD_PRESENT: &str = "blocking/required-field-present";

/// Check required fields
pub fn check_required_fields(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        // Profiles must have: Parent (already checked in profile.rs), Id, Title
        if profile.id().is_none()
            && let Some(name) = profile.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                profile.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("Profile '{name}' must specify an Id field"),
                    location,
                )
                .with_code("profile-missing-id".to_string()),
            );
        }

        if profile.title().is_none()
            && let Some(name) = profile.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                profile.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("Profile '{name}' must specify a Title field"),
                    location,
                )
                .with_code("profile-missing-title".to_string()),
            );
        }
    }

    // Check extensions
    for extension in document.extensions() {
        // Extensions must have: Id, Title
        if extension.id().is_none()
            && let Some(name) = extension.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                extension.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("Extension '{name}' must specify an Id field"),
                    location,
                )
                .with_code("extension-missing-id".to_string()),
            );
        }

        if extension.title().is_none()
            && let Some(name) = extension.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                extension.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("Extension '{name}' must specify a Title field"),
                    location,
                )
                .with_code("extension-missing-title".to_string()),
            );
        }
    }

    // Check value sets
    for value_set in document.value_sets() {
        // ValueSets must have: Id, Title
        if value_set.id().is_none()
            && let Some(name) = value_set.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                value_set.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("ValueSet '{name}' must specify an Id field"),
                    location,
                )
                .with_code("valueset-missing-id".to_string()),
            );
        }

        if value_set.title().is_none()
            && let Some(name) = value_set.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                value_set.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("ValueSet '{name}' must specify a Title field"),
                    location,
                )
                .with_code("valueset-missing-title".to_string()),
            );
        }
    }

    // Check code systems
    for code_system in document.code_systems() {
        // CodeSystems must have: Id, Title
        if code_system.id().is_none()
            && let Some(name) = code_system.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                code_system.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("CodeSystem '{name}' must specify an Id field"),
                    location,
                )
                .with_code("codesystem-missing-id".to_string()),
            );
        }

        if code_system.title().is_none()
            && let Some(name) = code_system.name()
        {
            let location = model.source_map.node_to_diagnostic_location(
                code_system.syntax(),
                &model.source,
                &model.source_file,
            );
            diagnostics.push(
                Diagnostic::new(
                    REQUIRED_FIELD_MISSING,
                    Severity::Error,
                    format!("CodeSystem '{name}' must specify a Title field"),
                    location,
                )
                .with_code("codesystem-missing-title".to_string()),
            );
        }
    }

    diagnostics
}
