//! Duplicate definition detection rules
//!
//! Detects duplicate resource definitions (same name or ID) in FSH files.

use maki_core::cst::FshSyntaxNode;
use maki_core::cst::ast::{AstNode, Document};
use maki_core::{Diagnostic, SemanticModel, Severity};
use std::collections::HashMap;

/// Rule ID for duplicate definitions
pub const DUPLICATE_DEFINITION: &str = "blocking/duplicate-definition";

/// Check for duplicate resource definitions
pub fn check_duplicates(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Track resource IDs and names
    let mut profile_names: HashMap<String, FshSyntaxNode> = HashMap::new();
    let mut extension_names: HashMap<String, FshSyntaxNode> = HashMap::new();
    let mut value_set_names: HashMap<String, FshSyntaxNode> = HashMap::new();
    let mut code_system_names: HashMap<String, FshSyntaxNode> = HashMap::new();
    let mut ids: HashMap<String, (String, FshSyntaxNode)> = HashMap::new(); // id -> (resource_type, location)

    // Check profiles
    for profile in document.profiles() {
        if let Some(name) = profile.name() {
            if let Some(first_location) =
                profile_names.insert(name.clone(), profile.syntax().clone())
            {
                let location = model.source_map.node_to_diagnostic_location(
                    profile.syntax(),
                    &model.source,
                    &model.source_file,
                );
                let first_loc = model.source_map.node_to_diagnostic_location(
                    &first_location,
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        format!(
                            "Duplicate Profile name '{}' (first defined at {}:{})",
                            name, first_loc.line, first_loc.column
                        ),
                        location,
                    )
                    .with_code("duplicate-profile-name".to_string()),
                );
            }
        }

        // Check ID duplicates across resource types
        if let Some(id_clause) = profile.id() {
            if let Some(id) = id_clause.value() {
                if let Some((res_type, first_location)) = ids.insert(
                    id.clone(),
                    ("Profile".to_string(), profile.syntax().clone()),
                ) {
                    let location = model.source_map.node_to_diagnostic_location(
                        profile.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    let first_loc = model.source_map.node_to_diagnostic_location(
                        &first_location,
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            DUPLICATE_DEFINITION,
                            Severity::Error,
                            format!(
                                "Duplicate resource ID '{}' (first used in {} at {}:{})",
                                id, res_type, first_loc.line, first_loc.column
                            ),
                            location,
                        )
                        .with_code("duplicate-resource-id".to_string()),
                    );
                }
            }
        }
    }

    // Check extensions
    for extension in document.extensions() {
        if let Some(name) = extension.name() {
            if let Some(first_location) =
                extension_names.insert(name.clone(), extension.syntax().clone())
            {
                let location = model.source_map.node_to_diagnostic_location(
                    extension.syntax(),
                    &model.source,
                    &model.source_file,
                );
                let first_loc = model.source_map.node_to_diagnostic_location(
                    &first_location,
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        format!(
                            "Duplicate Extension name '{}' (first defined at {}:{})",
                            name, first_loc.line, first_loc.column
                        ),
                        location,
                    )
                    .with_code("duplicate-extension-name".to_string()),
                );
            }
        }

        if let Some(id_clause) = extension.id() {
            if let Some(id) = id_clause.value() {
                if let Some((res_type, first_location)) = ids.insert(
                    id.clone(),
                    ("Extension".to_string(), extension.syntax().clone()),
                ) {
                    let location = model.source_map.node_to_diagnostic_location(
                        extension.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    let first_loc = model.source_map.node_to_diagnostic_location(
                        &first_location,
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            DUPLICATE_DEFINITION,
                            Severity::Error,
                            format!(
                                "Duplicate resource ID '{}' (first used in {} at {}:{})",
                                id, res_type, first_loc.line, first_loc.column
                            ),
                            location,
                        )
                        .with_code("duplicate-resource-id".to_string()),
                    );
                }
            }
        }
    }

    // Check value sets
    for value_set in document.value_sets() {
        if let Some(name) = value_set.name() {
            if let Some(first_location) =
                value_set_names.insert(name.clone(), value_set.syntax().clone())
            {
                let location = model.source_map.node_to_diagnostic_location(
                    value_set.syntax(),
                    &model.source,
                    &model.source_file,
                );
                let first_loc = model.source_map.node_to_diagnostic_location(
                    &first_location,
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        format!(
                            "Duplicate ValueSet name '{}' (first defined at {}:{})",
                            name, first_loc.line, first_loc.column
                        ),
                        location,
                    )
                    .with_code("duplicate-valueset-name".to_string()),
                );
            }
        }

        if let Some(id_clause) = value_set.id() {
            if let Some(id) = id_clause.value() {
                if let Some((res_type, first_location)) = ids.insert(
                    id.clone(),
                    ("ValueSet".to_string(), value_set.syntax().clone()),
                ) {
                    let location = model.source_map.node_to_diagnostic_location(
                        value_set.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    let first_loc = model.source_map.node_to_diagnostic_location(
                        &first_location,
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            DUPLICATE_DEFINITION,
                            Severity::Error,
                            format!(
                                "Duplicate resource ID '{}' (first used in {} at {}:{})",
                                id, res_type, first_loc.line, first_loc.column
                            ),
                            location,
                        )
                        .with_code("duplicate-resource-id".to_string()),
                    );
                }
            }
        }
    }

    // Check code systems
    for code_system in document.code_systems() {
        if let Some(name) = code_system.name() {
            if let Some(first_location) =
                code_system_names.insert(name.clone(), code_system.syntax().clone())
            {
                let location = model.source_map.node_to_diagnostic_location(
                    code_system.syntax(),
                    &model.source,
                    &model.source_file,
                );
                let first_loc = model.source_map.node_to_diagnostic_location(
                    &first_location,
                    &model.source,
                    &model.source_file,
                );
                diagnostics.push(
                    Diagnostic::new(
                        DUPLICATE_DEFINITION,
                        Severity::Error,
                        format!(
                            "Duplicate CodeSystem name '{}' (first defined at {}:{})",
                            name, first_loc.line, first_loc.column
                        ),
                        location,
                    )
                    .with_code("duplicate-codesystem-name".to_string()),
                );
            }
        }

        if let Some(id_clause) = code_system.id() {
            if let Some(id) = id_clause.value() {
                if let Some((res_type, first_location)) = ids.insert(
                    id.clone(),
                    ("CodeSystem".to_string(), code_system.syntax().clone()),
                ) {
                    let location = model.source_map.node_to_diagnostic_location(
                        code_system.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    let first_loc = model.source_map.node_to_diagnostic_location(
                        &first_location,
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            DUPLICATE_DEFINITION,
                            Severity::Error,
                            format!(
                                "Duplicate resource ID '{}' (first used in {} at {}:{})",
                                id, res_type, first_loc.line, first_loc.column
                            ),
                            location,
                        )
                        .with_code("duplicate-resource-id".to_string()),
                    );
                }
            }
        }
    }

    diagnostics
}
