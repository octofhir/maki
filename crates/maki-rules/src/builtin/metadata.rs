//! Metadata documentation validation rules
//!
//! Validates that FHIR resources have proper documentation metadata.
//! These are warning-level rules that encourage good documentation practices.

use maki_core::cst::ast::{AstNode, CodeSystem, Document, Extension, Profile, ValueSet};
use maki_core::{Diagnostic, SemanticModel, Severity};

/// Rule ID for missing metadata validation
pub const MISSING_METADATA: &str = "documentation/missing-metadata";

/// Check for missing metadata documentation in FSH document
pub fn check_missing_metadata(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        diagnostics.extend(check_profile_metadata(&profile, model));
    }

    // Check extensions
    for extension in document.extensions() {
        diagnostics.extend(check_extension_metadata(&extension, model));
    }

    // Check value sets
    for value_set in document.value_sets() {
        diagnostics.extend(check_value_set_metadata(&value_set, model));
    }

    // Check code systems
    for code_system in document.code_systems() {
        diagnostics.extend(check_code_system_metadata(&code_system, model));
    }

    diagnostics
}

/// Check metadata for Profile
fn check_profile_metadata(profile: &Profile, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the profile
    if profile.description().is_none() {
        if let Some(name) = profile.name() {
            diagnostics.push(create_missing_metadata_diagnostic(
                "Profile",
                &name,
                "Description",
                profile.syntax(),
                model,
                "Profiles should have a Description field for documentation",
            ));
        }
    }

    diagnostics
}

/// Check metadata for Extension
fn check_extension_metadata(extension: &Extension, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the extension
    if extension.description().is_none() {
        if let Some(name) = extension.name() {
            diagnostics.push(create_missing_metadata_diagnostic(
                "Extension",
                &name,
                "Description",
                extension.syntax(),
                model,
                "Extensions should have a Description field for documentation",
            ));
        }
    }

    diagnostics
}

/// Check metadata for ValueSet
fn check_value_set_metadata(value_set: &ValueSet, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding what values are included
    if value_set.description().is_none() {
        if let Some(name) = value_set.name() {
            diagnostics.push(create_missing_metadata_diagnostic(
                "ValueSet",
                &name,
                "Description",
                value_set.syntax(),
                model,
                "ValueSets should have a Description field explaining the purpose and contents",
            ));
        }
    }

    diagnostics
}

/// Check metadata for CodeSystem
fn check_code_system_metadata(code_system: &CodeSystem, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Description is important for understanding the code system
    if code_system.description().is_none() {
        if let Some(name) = code_system.name() {
            diagnostics.push(create_missing_metadata_diagnostic(
                "CodeSystem",
                &name,
                "Description",
                code_system.syntax(),
                model,
                "CodeSystems should have a Description field explaining the codes",
            ));
        }
    }

    diagnostics
}

/// Create a diagnostic for missing metadata field
fn create_missing_metadata_diagnostic(
    resource_type: &str,
    resource_name: &str,
    field_name: &str,
    node: &maki_core::cst::FshSyntaxNode,
    model: &SemanticModel,
    message: &str,
) -> Diagnostic {
    // Use SourceMap for precise location!
    let location =
        model
            .source_map
            .node_to_diagnostic_location(node, &model.source, &model.source_file);

    // Find the first line of the resource declaration to insert the field after it
    let first_line_text = model
        .source
        .lines()
        .nth(location.line.saturating_sub(1))
        .unwrap_or("");

    // Create an insertion location at the end of the first line
    // Column is 1-based, so column after last char is len + 1
    let insert_location = maki_core::diagnostics::Location {
        file: location.file.clone(),
        line: location.line,
        column: first_line_text.chars().count() + 1, // 1-based, position after last character
        end_line: Some(location.line),
        end_column: Some(first_line_text.chars().count() + 1), // Same as start for insertion
        offset: location.offset + first_line_text.len(),
        length: 0,
        span: Some((
            location.offset + first_line_text.len(),
            location.offset + first_line_text.len(),
        )),
    };

    Diagnostic::new(
        MISSING_METADATA,
        Severity::Warning, // Warning, not error - documentation is encouraged but not required
        format!(
            "{resource_type} '{resource_name}' is missing recommended field: {field_name}. {message}"
        ),
        location.clone(),
    )
    .with_suggestion(maki_core::CodeSuggestion::safe(
        format!("Add {field_name} field"),
        format!("\n{field_name}: \"\""),
        insert_location,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::SemanticModel;
    use maki_core::cst::{ast::AstNode, parse_fsh};
    use std::path::PathBuf;

    fn create_test_model_from_source(source: &str) -> SemanticModel {
        let (cst, _) = parse_fsh(source);
        let source_map = maki_core::SourceMap::new(source);
        SemanticModel {
            cst,
            resources: Vec::new(),
            symbols: Default::default(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
        }
    }

    #[test]
    fn test_profile_with_description() {
        let source = r#"Profile: TestProfile
Id: test-profile
Title: "Test Profile"
Description: "A test profile"
"#;
        let model = create_test_model_from_source(source);
        let document = Document::cast(model.cst.clone()).expect("Should parse as document");
        let profile = document.profiles().next().expect("Should have profile");

        let diagnostics = check_profile_metadata(&profile, &model);
        assert_eq!(
            diagnostics.len(),
            0,
            "Profile with description should have no warnings"
        );
    }

    #[test]
    fn test_profile_missing_description() {
        let source = r#"Profile: TestProfile
Id: test-profile
Title: "Test Profile"
"#;
        let model = create_test_model_from_source(source);
        let document = Document::cast(model.cst.clone()).expect("Should parse as document");
        let profile = document.profiles().next().expect("Should have profile");

        let diagnostics = check_profile_metadata(&profile, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn test_extension_missing_description() {
        let source = r#"Extension: TestExtension
Id: test-ext
Title: "Test Extension"
"#;
        let model = create_test_model_from_source(source);
        let document = Document::cast(model.cst.clone()).expect("Should parse as document");
        let extension = document.extensions().next().expect("Should have extension");

        let diagnostics = check_extension_metadata(&extension, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_value_set_missing_description() {
        let source = r#"ValueSet: TestVS
Id: test-vs
Title: "Test VS"
"#;
        let model = create_test_model_from_source(source);
        let document = Document::cast(model.cst.clone()).expect("Should parse as document");
        let value_set = document.value_sets().next().expect("Should have value set");

        let diagnostics = check_value_set_metadata(&value_set, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_code_system_missing_description() {
        let source = r#"CodeSystem: TestCS
Id: test-cs
Title: "Test CS"
"#;
        let model = create_test_model_from_source(source);
        let document = Document::cast(model.cst.clone()).expect("Should parse as document");
        let code_system = document
            .code_systems()
            .next()
            .expect("Should have code system");

        let diagnostics = check_code_system_metadata(&code_system, &model);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Description"));
    }

    #[test]
    fn test_check_metadata_in_document() {
        let source = r#"Profile: TestProfile
Id: test
Title: "Test"

ValueSet: TestVS
Id: test-vs
Title: "Test VS"
"#;
        let model = create_test_model_from_source(source);

        let diagnostics = check_missing_metadata(&model);
        assert_eq!(diagnostics.len(), 2, "Should find 2 missing descriptions");
        assert!(diagnostics.iter().all(|d| d.severity == Severity::Warning));
    }
}
