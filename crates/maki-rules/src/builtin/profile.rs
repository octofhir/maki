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
pub const SLICE_NAME_COLLISION: &str = "correctness/slice-name-collision";
pub const MUST_SUPPORT_PROPAGATION: &str = "suspicious/must-support-propagation";

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

        if !has_context_rule && let Some(name) = extension.name() {
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

    diagnostics
}

/// Check for slice name collisions
/// Detects when slice names conflict with existing FHIR element names
pub fn check_slice_name_collision(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles for slice name collisions
    for profile in document.profiles() {
        diagnostics.extend(check_entity_slice_collisions(model, profile.rules()));
    }

    // Check extensions for slice name collisions
    for extension in document.extensions() {
        diagnostics.extend(check_entity_slice_collisions(model, extension.rules()));
    }

    diagnostics
}

/// Check slice name collisions in a specific entity
fn check_entity_slice_collisions(
    model: &SemanticModel,
    rules: impl Iterator<Item = maki_core::cst::ast::Rule>,
) -> Vec<Diagnostic> {
    use maki_core::cst::ast::Rule;
    use std::collections::{HashMap, HashSet};

    let mut diagnostics = Vec::new();

    // Common FHIR element names that are likely to cause collisions
    // This is a simplified check - full validation would require FHIR definitions
    let common_element_names: HashSet<&str> = [
        "code",
        "value",
        "status",
        "system",
        "display",
        "text",
        "extension",
        "id",
        "meta",
        "reference",
        "type",
        "url",
    ]
    .into_iter()
    .collect();

    // Track slice names by path
    let mut slices_by_path: HashMap<String, Vec<(String, maki_core::cst::FshSyntaxNode)>> =
        HashMap::new();

    for rule in rules {
        if let Rule::Contains(contains_rule) = rule
            && let Some(path) = contains_rule.path()
        {
            let base_path = path.syntax().text().to_string().trim().to_string();

            // Get slice names
            for slice_name in contains_rule.items() {
                slices_by_path
                    .entry(base_path.clone())
                    .or_default()
                    .push((slice_name, contains_rule.syntax().clone()));
            }
        }
    }

    // Check for collisions with common element names
    for (base_path, slices) in slices_by_path {
        for (slice_name, node) in slices {
            // Check against common FHIR element names
            if common_element_names.contains(slice_name.as_str()) {
                let location = model.source_map.node_to_diagnostic_location(
                    &node,
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        SLICE_NAME_COLLISION,
                        Severity::Warning,
                        format!(
                            "Slice name '{}' may collide with FHIR element name at path '{}'",
                            slice_name, base_path
                        ),
                        location,
                    )
                    .with_code("potential-slice-collision".to_string()),
                );
            }
        }
    }

    diagnostics
}

/// Check MustSupport propagation consistency
/// Warns when child elements of MS-flagged elements are not also marked MS
pub fn check_must_support_propagation(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles for MS propagation
    for profile in document.profiles() {
        diagnostics.extend(check_entity_ms_propagation(model, profile.rules()));
    }

    // Check extensions for MS propagation
    for extension in document.extensions() {
        diagnostics.extend(check_entity_ms_propagation(model, extension.rules()));
    }

    diagnostics
}

/// Check MS propagation in a specific entity
fn check_entity_ms_propagation(
    model: &SemanticModel,
    rules: impl Iterator<Item = maki_core::cst::ast::Rule>,
) -> Vec<Diagnostic> {
    use maki_core::cst::ast::{FlagValue, Rule};
    use std::collections::{HashMap, HashSet};

    let mut diagnostics = Vec::new();

    // Track paths with MS flags and their child paths
    let mut ms_paths: HashSet<String> = HashSet::new();
    let mut all_paths: HashMap<String, maki_core::cst::FshSyntaxNode> = HashMap::new();
    let mut path_flags: HashMap<String, Vec<FlagValue>> = HashMap::new();

    // Collect all paths and their flags
    for rule in rules {
        let (path_opt, flags_opt, node) = match &rule {
            Rule::Card(card) => (
                card.path().map(|p| p.syntax().text().to_string()),
                card.flags(),
                card.syntax().clone(),
            ),
            Rule::Flag(flag) => (
                flag.path().map(|p| p.syntax().text().to_string()),
                flag.flags(),
                flag.syntax().clone(),
            ),
            // OnlyRule and ValueSetRule don't have flags() methods, just path
            Rule::Only(only) => (
                only.path().map(|p| p.syntax().text().to_string()),
                Vec::new(),
                only.syntax().clone(),
            ),
            Rule::ValueSet(vs) => (
                vs.path().map(|p| p.syntax().text().to_string()),
                Vec::new(),
                vs.syntax().clone(),
            ),
            _ => (None, Vec::new(), rule.syntax().clone()),
        };

        if let Some(path) = path_opt {
            all_paths.entry(path.clone()).or_insert(node);

            // Check if this rule has MS flag
            if !flags_opt.is_empty() {
                path_flags
                    .entry(path.clone())
                    .or_default()
                    .extend(flags_opt.clone());

                if flags_opt.contains(&FlagValue::MustSupport) {
                    ms_paths.insert(path);
                }
            }
        }
    }

    // Check for child paths of MS elements that don't have MS
    for ms_path in &ms_paths {
        // Find child paths (paths that start with ms_path + ".")
        for (child_path, child_node) in &all_paths {
            if child_path.starts_with(&format!("{}.", ms_path)) {
                // This is a child of an MS element
                let child_flags = path_flags.get(child_path);

                // Check if child has MS flag
                let has_ms = child_flags
                    .map(|flags| flags.contains(&FlagValue::MustSupport))
                    .unwrap_or(false);

                if !has_ms {
                    let location = model.source_map.node_to_diagnostic_location(
                        child_node,
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            MUST_SUPPORT_PROPAGATION,
                            Severity::Warning,
                            format!(
                                "Child element '{}' of MustSupport element '{}' should typically also be marked MS",
                                child_path, ms_path
                            ),
                            location,
                        )
                        .with_code("missing-ms-propagation".to_string()),
                    );
                }
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
    fn test_slice_name_collision_with_common_elements() {
        let source = r#"
Profile: TestProfile
Parent: Patient
* identifier contains code 1..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_slice_name_collision(&model);

        assert!(
            !diagnostics.is_empty(),
            "Should detect 'code' as potential collision"
        );
        assert!(diagnostics.iter().any(|d| d.message.contains("code")));
    }

    #[test]
    fn test_no_slice_collision_with_safe_names() {
        let source = r#"
Profile: TestProfile
Parent: Patient
* identifier contains MRN 1..1 and SSN 0..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_slice_name_collision(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should not detect collision with safe names"
        );
    }

    #[test]
    fn test_must_support_propagation_warning() {
        let source = r#"
Profile: TestProfile
Parent: Patient
* name 1..* MS
* name.family 1..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_must_support_propagation(&model);

        assert!(
            !diagnostics.is_empty(),
            "Should warn about missing MS on child"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("name.family") && d.message.contains("MustSupport"))
        );
    }

    #[test]
    fn test_must_support_propagation_consistent() {
        let source = r#"
Profile: TestProfile
Parent: Patient
* name 1..* MS
* name.family 1..1 MS
* name.given 1..* MS
"#;
        let model = create_test_model(source);
        let diagnostics = check_must_support_propagation(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should not warn when MS is consistent"
        );
    }

    #[test]
    fn test_no_ms_propagation_when_no_ms() {
        let source = r#"
Profile: TestProfile
Parent: Patient
* name 1..*
* name.family 1..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_must_support_propagation(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Should not warn when parent has no MS"
        );
    }
}
