//! Required field validation rules
//!
//! These are blocking rules that must pass before other rules can run.
//! They ensure that critical fields are present in FSH definitions.

use fsh_lint_core::ast::{CodeSystem, Profile, ValueSet};
use fsh_lint_core::{Diagnostic, Location, SemanticModel, Severity};
use std::path::PathBuf;

/// Rule ID for required field validation
pub const REQUIRED_FIELD_PRESENT: &str = "correctness/required-field-present";

/// Check if FSH document has all required fields
pub fn check_required_fields(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check profiles
    for profile in &model.document.profiles {
        diagnostics.extend(check_profile_fields(profile, model));
    }

    // Check value sets
    for value_set in &model.document.value_sets {
        diagnostics.extend(check_value_set_fields(value_set, model));
    }

    // Check code systems
    for code_system in &model.document.code_systems {
        diagnostics.extend(check_code_system_fields(code_system, model));
    }

    diagnostics
}

/// Check required fields for Profile
fn check_profile_fields(profile: &Profile, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Profile name is always present (required by parser)
    // Check for Id
    if profile.id.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "Profile",
            &profile.name.value,
            "Id",
            &profile.span,
            model,
            "Profiles must have an Id field",
        ));
    }

    // Check for Title
    if profile.title.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "Profile",
            &profile.name.value,
            "Title",
            &profile.span,
            model,
            "Profiles must have a Title field",
        ));
    }

    diagnostics
}

/// Check required fields for ValueSet
fn check_value_set_fields(value_set: &ValueSet, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for Id
    if value_set.id.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "ValueSet",
            &value_set.name.value,
            "Id",
            &value_set.span,
            model,
            "ValueSets must have an Id field",
        ));
    }

    // Check for Title
    if value_set.title.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "ValueSet",
            &value_set.name.value,
            "Title",
            &value_set.span,
            model,
            "ValueSets must have a Title field",
        ));
    }

    diagnostics
}

/// Check required fields for CodeSystem
fn check_code_system_fields(code_system: &CodeSystem, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for Id
    if code_system.id.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "CodeSystem",
            &code_system.name.value,
            "Id",
            &code_system.span,
            model,
            "CodeSystems must have an Id field",
        ));
    }

    // Check for Title
    if code_system.title.is_none() {
        diagnostics.push(create_missing_field_diagnostic(
            "CodeSystem",
            &code_system.name.value,
            "Title",
            &code_system.span,
            model,
            "CodeSystems must have a Title field",
        ));
    }

    diagnostics
}

/// Create a diagnostic for a missing required field
fn create_missing_field_diagnostic(
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
        REQUIRED_FIELD_PRESENT,
        Severity::Error,
        &format!(
            "{} '{}' is missing required field: {}. {}",
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

    #[test]
    fn test_profile_with_all_required_fields() {
        let source = "Profile: TestProfile\nParent: Patient\nId: test-profile\nTitle: \"Test Profile\"\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: Some(Spanned::new("Patient".to_string(), 20..27)),
            id: Some(Spanned::new("test-profile".to_string(), 30..42)),
            title: Some(Spanned::new("Test Profile".to_string(), 50..62)),
            description: None,
            rules: Vec::new(),
            span: 0..100,
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

        let diagnostics = check_profile_fields(&profile, &model);
        assert_eq!(diagnostics.len(), 0, "Profile with all required fields should have no diagnostics");
    }

    #[test]
    fn test_profile_missing_id() {
        let source = "Profile: TestProfile\nParent: Patient\nTitle: \"Test Profile\"\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: Some(Spanned::new("Patient".to_string(), 20..27)),
            id: None,
            title: Some(Spanned::new("Test Profile".to_string(), 50..62)),
            description: None,
            rules: Vec::new(),
            span: 0..100,
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

        let diagnostics = check_profile_fields(&profile, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Id"));
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn test_profile_missing_title() {
        let source = "Profile: TestProfile\nParent: Patient\nId: test-profile\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: Some(Spanned::new("Patient".to_string(), 20..27)),
            id: Some(Spanned::new("test-profile".to_string(), 30..42)),
            title: None,
            description: None,
            rules: Vec::new(),
            span: 0..100,
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

        let diagnostics = check_profile_fields(&profile, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Title"));
    }

    #[test]
    fn test_profile_missing_both() {
        let source = "Profile: TestProfile\nParent: Patient\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: Some(Spanned::new("Patient".to_string(), 20..27)),
            id: None,
            title: None,
            description: None,
            rules: Vec::new(),
            span: 0..100,
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

        let diagnostics = check_profile_fields(&profile, &model);
        assert_eq!(diagnostics.len(), 2, "Should report both missing Id and Title");
    }

    #[test]
    fn test_value_set_missing_fields() {
        let source = "ValueSet: TestVS\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let value_set = ValueSet {
            name: Spanned::new("TestVS".to_string(), 0..6),
            parent: None,
            id: None,
            title: None,
            description: None,
            components: Vec::new(),
            rules: Vec::new(),
            span: 0..50,
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

        let diagnostics = check_value_set_fields(&value_set, &model);
        assert_eq!(diagnostics.len(), 2, "Should report missing Id and Title");
    }

    #[test]
    fn test_code_system_missing_fields() {
        let source = "CodeSystem: TestCS\n";
        let source_map = fsh_lint_core::SourceMap::new(source);

        let code_system = CodeSystem {
            name: Spanned::new("TestCS".to_string(), 0..6),
            id: None,
            title: None,
            description: None,
            concepts: Vec::new(),
            rules: Vec::new(),
            span: 0..50,
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

        let diagnostics = check_code_system_fields(&code_system, &model);
        assert_eq!(diagnostics.len(), 2, "Should report missing Id and Title");
    }
}
