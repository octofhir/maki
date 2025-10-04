//! Metadata documentation validation rules
//!
//! Validates that FHIR resources have proper documentation metadata.
//! These are warning-level rules that encourage good documentation practices.

use fsh_lint_core::ast::{CodeSystem, Extension, Profile, ValueSet};
use fsh_lint_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for missing metadata validation
pub const MISSING_METADATA: &str = "documentation/missing-metadata";

/// Check for missing metadata documentation in FSH document
pub fn check_missing_metadata(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check profiles
    for profile in &model.document.profiles {
        diagnostics.extend(check_profile_metadata(profile, model));
    }

    // Check extensions
    for extension in &model.document.extensions {
        diagnostics.extend(check_extension_metadata(extension, model));
    }

    // Check value sets
    for value_set in &model.document.value_sets {
        diagnostics.extend(check_value_set_metadata(value_set, model));
    }

    // Check code systems
    for code_system in &model.document.code_systems {
        diagnostics.extend(check_code_system_metadata(code_system, model));
    }

    diagnostics
}

/// Check metadata for Profile
fn check_profile_metadata(profile: &Profile, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the profile
    if profile.description.is_none() {
        diagnostics.push(create_missing_metadata_diagnostic(
            "Profile",
            &profile.name.value,
            "Description",
            &profile.span,
            model,
            "Profiles should have a Description field for documentation",
        ));
    }

    diagnostics
}

/// Check metadata for Extension
fn check_extension_metadata(extension: &Extension, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the extension
    if extension.description.is_none() {
        diagnostics.push(create_missing_metadata_diagnostic(
            "Extension",
            &extension.name.value,
            "Description",
            &extension.span,
            model,
            "Extensions should have a Description field for documentation",
        ));
    }

    diagnostics
}

/// Check metadata for ValueSet
fn check_value_set_metadata(value_set: &ValueSet, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding what values are included
    if value_set.description.is_none() {
        diagnostics.push(create_missing_metadata_diagnostic(
            "ValueSet",
            &value_set.name.value,
            "Description",
            &value_set.span,
            model,
            "ValueSets should have a Description field explaining the purpose and contents",
        ));
    }

    diagnostics
}

/// Check metadata for CodeSystem
fn check_code_system_metadata(code_system: &CodeSystem, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the code system
    if code_system.description.is_none() {
        diagnostics.push(create_missing_metadata_diagnostic(
            "CodeSystem",
            &code_system.name.value,
            "Description",
            &code_system.span,
            model,
            "CodeSystems should have a Description field explaining the codes",
        ));
    }

    diagnostics
}

/// Create a diagnostic for missing metadata field
fn create_missing_metadata_diagnostic(
    resource_type: &str,
    resource_name: &str,
    field_name: &str,
    span: &std::ops::Range<usize>,
    model: &SemanticModel,
    message: &str,
) -> Diagnostic {
    // Use SourceMap for precise location!
    let location = model.source_map.span_to_diagnostic_location(
        span,
        &model.source,
        &model.source_file,
    );

    Diagnostic::new(
        MISSING_METADATA,
        Severity::Warning, // Warning, not error - documentation is encouraged but not required
        &format!(
            "{} '{}' is missing recommended field: {}. {}",
            resource_type, resource_name, field_name, message
        ),
        location.clone(),
    )
    .with_suggestion(fsh_lint_core::Suggestion {
        message: format!("Add {} field", field_name),
        replacement: format!("{}: \"\"", field_name),
        location,
        is_safe: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fsh_lint_core::ast::{FSHDocument, Spanned};
    use fsh_lint_core::SemanticModel;
    use std::path::PathBuf;

    fn create_test_model() -> SemanticModel {
        let source = "Profile: Test\nDescription: \"Test\"".to_string();
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
    fn test_profile_with_description() {
        let model = create_test_model();
        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: Some(Spanned::new("test-profile".to_string(), 20..32)),
            title: Some(Spanned::new("Test Profile".to_string(), 40..52)),
            description: Some(Spanned::new("A test profile".to_string(), 60..74)),
            rules: Vec::new(),
            span: 0..100,
        };

        let diagnostics = check_profile_metadata(&profile, &model);
        assert_eq!(diagnostics.len(), 0, "Profile with description should have no warnings");
    }

    #[test]
    fn test_profile_missing_description() {
        let model = create_test_model();
        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: Some(Spanned::new("test-profile".to_string(), 20..32)),
            title: Some(Spanned::new("Test Profile".to_string(), 40..52)),
            description: None,
            rules: Vec::new(),
            span: 0..100,
        };

        let diagnostics = check_profile_metadata(&profile, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_extension_missing_description() {
        let model = create_test_model();
        let extension = Extension {
            name: Spanned::new("TestExtension".to_string(), 0..13),
            parent: None,
            id: Some(Spanned::new("test-ext".to_string(), 20..28)),
            title: Some(Spanned::new("Test Extension".to_string(), 35..49)),
            description: None,
            contexts: Vec::new(),
            rules: Vec::new(),
            span: 0..100,
        };

        let diagnostics = check_extension_metadata(&extension, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_value_set_missing_description() {
        let model = create_test_model();
        let value_set = ValueSet {
            name: Spanned::new("TestVS".to_string(), 0..6),
            parent: None,
            id: Some(Spanned::new("test-vs".to_string(), 15..22)),
            title: Some(Spanned::new("Test VS".to_string(), 30..37)),
            description: None,
            components: Vec::new(),
            rules: Vec::new(),
            span: 0..50,
        };

        let diagnostics = check_value_set_metadata(&value_set, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_code_system_missing_description() {
        let model = create_test_model();
        let code_system = CodeSystem {
            name: Spanned::new("TestCS".to_string(), 0..6),
            id: Some(Spanned::new("test-cs".to_string(), 15..22)),
            title: Some(Spanned::new("Test CS".to_string(), 30..37)),
            description: None,
            concepts: Vec::new(),
            rules: Vec::new(),
            span: 0..50,
        };

        let diagnostics = check_code_system_metadata(&code_system, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_check_metadata_in_document() {
        let source = "Profile: Test\nExtension: Ext".to_string();
        let source_map = fsh_lint_core::SourceMap::new(&source);
        let mut doc = FSHDocument::new(0..source.len());

        // Add profile without description
        doc.profiles.push(Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: Some(Spanned::new("test".to_string(), 20..24)),
            title: Some(Spanned::new("Test".to_string(), 30..34)),
            description: None,
            rules: Vec::new(),
            span: 0..50,
        });

        // Add value set without description
        doc.value_sets.push(ValueSet {
            name: Spanned::new("TestVS".to_string(), 60..66),
            parent: None,
            id: Some(Spanned::new("test-vs".to_string(), 75..82)),
            title: Some(Spanned::new("Test VS".to_string(), 90..97)),
            description: None,
            components: Vec::new(),
            rules: Vec::new(),
            span: 60..100,
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

        let diagnostics = check_missing_metadata(&model);
        assert_eq!(diagnostics.len(), 2, "Should find 2 missing descriptions");
        assert!(diagnostics.iter().all(|d| d.severity == Severity::Warning));
    }
}
