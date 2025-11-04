//! Integration tests for Extension export
//!
//! Tests the ExtensionExporter with real FHIR definitions.

use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use maki_core::cst::{ast::*, parse_fsh};
use maki_core::export::{ExtensionExporter, StructureDefinitionKind};
use std::sync::Arc;

/// Helper to create a test session with FHIR R4 core
async fn create_test_session() -> Arc<maki_core::canonical::DefinitionSession> {
    let options = CanonicalOptions {
        auto_install_core: false, // Disable auto-install to avoid network issues
        quick_init: true,
        ..Default::default()
    };

    let facade = CanonicalFacade::new(options).await.unwrap();
    let session = facade.session([FhirRelease::R4]).await.unwrap();

    Arc::new(session)
}

#[tokio::test]
async fn test_export_simple_extension() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Parse FSH extension
    let fsh = r#"
Extension: PatientExtension
Id: patient-extension
Title: "Patient Extension"
Description: "An extension for patients"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Export extension
    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Export failed: {:?}", result.err());

    let structure_def = result.unwrap();

    // Validate structure
    assert_eq!(structure_def.resource_type, "StructureDefinition");
    assert_eq!(structure_def.name, "PatientExtension");
    assert_eq!(
        structure_def.url,
        "http://example.org/fhir/Extension/PatientExtension"
    );
    assert_eq!(structure_def.id, Some("patient-extension".to_string()));
    assert_eq!(structure_def.title, Some("Patient Extension".to_string()));
    assert_eq!(
        structure_def.description,
        Some("An extension for patients".to_string())
    );
    assert_eq!(structure_def.type_field, "Extension");
    assert_eq!(structure_def.kind, StructureDefinitionKind::ComplexType);
    assert_eq!(
        structure_def.base_definition,
        Some("http://hl7.org/fhir/StructureDefinition/Extension".to_string())
    );
    assert_eq!(structure_def.derivation, Some("constraint".to_string()));

    // Check context
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 1);
    assert_eq!(context[0].type_, "element");
    assert_eq!(context[0].expression, "Element");

    // Check differential
    assert!(structure_def.differential.is_some());
    let differential = structure_def.differential.as_ref().unwrap();
    assert!(
        !differential.element.is_empty(),
        "Differential should have elements"
    );

    // Check that Extension.url is fixed
    let url_element = differential
        .element
        .iter()
        .find(|e| e.path == "Extension.url");
    assert!(
        url_element.is_some(),
        "Extension.url should be in differential"
    );
    let url_element = url_element.unwrap();
    assert!(
        url_element.fixed.is_some(),
        "Extension.url should have fixed value"
    );
}

#[tokio::test]
async fn test_export_extension_with_cardinality() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Parse FSH extension with cardinality rules
    let fsh = r#"
Extension: USCoreRaceExtension
Id: us-core-race
Title: "US Core Race Extension"
Description: "Race extension"
* value[x] 0..0
* extension contains
    ombCategory 0..5 MS and
    text 1..1 MS
* extension[ombCategory].value[x] only Coding
* extension[text].value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Export extension
    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Export failed: {:?}", result.err());

    let structure_def = result.unwrap();

    // Validate basic structure
    assert_eq!(structure_def.name, "USCoreRaceExtension");
    assert_eq!(structure_def.id, Some("us-core-race".to_string()));
    assert_eq!(structure_def.type_field, "Extension");

    // This is a complex extension (has sub-extensions, no value[x])
    // Check differential for cardinality constraints
    if let Some(differential) = &structure_def.differential {
        // Look for value[x] cardinality
        let value_elem = differential
            .element
            .iter()
            .find(|e| e.path == "Extension.value[x]");

        if let Some(elem) = value_elem {
            // Should be 0..0 (prohibited)
            assert_eq!(elem.min, Some(0));
            assert_eq!(elem.max.as_deref(), Some("0"));
        }
    }
}

#[tokio::test]
async fn test_export_extension_metadata() {
    let session = create_test_session().await;
    let exporter = ExtensionExporter::new(session.clone(), "http://test.org".to_string(), None)
        .await
        .expect("Failed to create exporter");

    // Parse FSH extension with all metadata
    let fsh = r#"
Extension: TestExtension
Id: test-ext
Title: "Test Extension"
Description: "A test extension with metadata"
* value[x] only boolean
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Export extension
    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Export failed: {:?}", result.err());

    let structure_def = result.unwrap();

    // Validate all metadata
    assert_eq!(structure_def.name, "TestExtension");
    assert_eq!(structure_def.url, "http://test.org/Extension/TestExtension");
    assert_eq!(structure_def.id, Some("test-ext".to_string()));
    assert_eq!(structure_def.title, Some("Test Extension".to_string()));
    assert_eq!(
        structure_def.description,
        Some("A test extension with metadata".to_string())
    );
    // Status is inherited from base Extension (active)
    assert!(!structure_def.status.is_empty());
    assert_eq!(structure_def.kind, StructureDefinitionKind::ComplexType);
    assert!(!structure_def.is_abstract);
}

#[tokio::test]
async fn test_export_extension_without_id() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Parse FSH extension without Id
    let fsh = r#"
Extension: MinimalExtension
Title: "Minimal Extension"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    // Export extension
    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Export failed: {:?}", result.err());

    let structure_def = result.unwrap();

    // Should still export successfully
    assert_eq!(structure_def.name, "MinimalExtension");
    assert_eq!(
        structure_def.url,
        "http://example.org/fhir/Extension/MinimalExtension"
    );
    // Note: id is inherited from base Extension, not overridden when not specified in FSH
    // The important thing is that we don't crash and the profile-specific metadata is set correctly
}

#[tokio::test]
async fn test_extension_structure_validation() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    let fsh = r#"
Extension: ValidExtension
Id: valid-ext
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Valid extension should export successfully");

    let structure_def = result.unwrap();

    // Verify structure is valid
    assert!(!structure_def.url.is_empty());
    assert!(!structure_def.name.is_empty());
    assert_eq!(structure_def.type_field, "Extension");
    assert_eq!(structure_def.kind, StructureDefinitionKind::ComplexType);
}

#[tokio::test]
async fn test_extension_context_definition_generation() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with explicit context rules
    let fsh = r#"
Extension: PatientOnlyExtension
Id: patient-only-ext
^context = "Patient"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with context should export successfully"
    );

    let structure_def = result.unwrap();

    // Verify context is set correctly
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 1);
    assert_eq!(context[0].type_, "element");
    assert_eq!(context[0].expression, "Patient");
}

#[tokio::test]
async fn test_extension_context_multiple_types() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with multiple context types
    let fsh = r#"
Extension: MultiContextExtension
Id: multi-context-ext
^context = "Patient"
^context = "Observation"
* value[x] only boolean
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with multiple contexts should export successfully"
    );

    let structure_def = result.unwrap();

    // Verify multiple contexts are set
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 2);

    // Check first context
    assert_eq!(context[0].type_, "element");
    assert_eq!(context[0].expression, "Patient");

    // Check second context
    assert_eq!(context[1].type_, "element");
    assert_eq!(context[1].expression, "Observation");
}

#[tokio::test]
async fn test_extension_value_constraint_processing() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test simple extension with value[x] type constraint
    let fsh = r#"
Extension: StringExtension
Id: string-ext
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with value constraint should export successfully"
    );

    let structure_def = result.unwrap();

    // Check that differential contains value[x] constraint
    assert!(structure_def.differential.is_some());
    let differential = structure_def.differential.as_ref().unwrap();

    let value_element = differential
        .element
        .iter()
        .find(|e| e.path == "Extension.value[x]");

    if let Some(elem) = value_element {
        assert!(elem.type_.is_some());
        let types = elem.type_.as_ref().unwrap();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].code, "string");
    }
}

#[tokio::test]
async fn test_extension_cardinality_constraints() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with cardinality constraints
    let fsh = r#"
Extension: CardinalityExtension
Id: cardinality-ext
* . 1..1
* value[x] 1..1
* value[x] only integer
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with cardinality should export successfully"
    );

    let structure_def = result.unwrap();

    // Check root extension cardinality in snapshot
    if let Some(snapshot) = &structure_def.snapshot {
        let root_element = snapshot.element.iter().find(|e| e.path == "Extension");

        if let Some(elem) = root_element {
            assert_eq!(elem.min, Some(1));
            assert_eq!(elem.max.as_deref(), Some("1"));
        }
    }
}

#[tokio::test]
async fn test_nested_extension_support() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test complex extension with nested extensions (single line syntax)
    let fsh = r#"
Extension: ComplexExtension
Id: complex-ext
* extension contains subExt1 0..1 MS and subExt2 1..1
* extension[subExt1].value[x] only string
* extension[subExt2].value[x] only integer
* value[x] 0..0
"#;

    println!("Testing FSH:\n{}", fsh);

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Complex extension should export successfully"
    );

    let structure_def = result.unwrap();

    // Check that this is recognized as a complex extension
    if let Some(differential) = &structure_def.differential {
        // Should have value[x] prohibited (0..0)
        let value_element = differential
            .element
            .iter()
            .find(|e| e.path == "Extension.value[x]");

        if let Some(elem) = value_element {
            assert_eq!(elem.min, Some(0));
            assert_eq!(elem.max.as_deref(), Some("0"));
        }

        // Should have nested extension elements
        let nested_elements: Vec<_> = differential
            .element
            .iter()
            .filter(|e| e.path.starts_with("Extension.extension:"))
            .collect();

        assert!(
            !nested_elements.is_empty(),
            "Should have nested extension elements"
        );
    }
}

#[tokio::test]
async fn test_extension_fhir_version_from_session() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    let fsh = r#"
Extension: VersionTestExtension
Id: version-test-ext
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Extension should export successfully");

    let structure_def = result.unwrap();

    // Verify FHIR version is set from session (should be R4 = "4.0.1")
    assert!(structure_def.fhir_version.is_some());
    let fhir_version = structure_def.fhir_version.as_ref().unwrap();
    assert_eq!(fhir_version, "4.0.1");
}

#[tokio::test]
async fn test_extension_with_version_config() {
    let session = create_test_session().await;
    let version = Some("2.1.0".to_string());
    let exporter = ExtensionExporter::new(
        session.clone(),
        "http://example.org/fhir".to_string(),
        version.clone(),
    )
    .await
    .expect("Failed to create exporter");

    let fsh = r#"
Extension: VersionedExtension
Id: versioned-ext
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(result.is_ok(), "Extension should export successfully");

    let structure_def = result.unwrap();

    // Verify version is set from config
    assert_eq!(structure_def.version, version);
}

#[tokio::test]
async fn test_multiline_contains_rule_parsing() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test multiline contains rule with proper FSH syntax
    let fsh = r#"
Extension: MultilineExtension
Id: multiline-ext
* extension contains
    subExt1 0..1 MS and
    subExt2 1..1 and
    subExt3 0..* MS
* extension[subExt1].value[x] only string
* extension[subExt2].value[x] only integer
* extension[subExt3].value[x] only boolean
* value[x] 0..0
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Multiline extension should export successfully: {:?}",
        result.err()
    );

    let structure_def = result.unwrap();

    // Check that nested extensions were created
    if let Some(differential) = &structure_def.differential {
        let nested_elements: Vec<_> = differential
            .element
            .iter()
            .filter(|e| e.path.starts_with("Extension.extension:"))
            .collect();

        // Should have at least the nested extension elements
        assert!(
            !nested_elements.is_empty(),
            "Should have nested extension elements"
        );

        // Check for specific nested extensions
        let has_subext1 = nested_elements.iter().any(|e| e.path.contains("subExt1"));
        let has_subext2 = nested_elements.iter().any(|e| e.path.contains("subExt2"));
        let has_subext3 = nested_elements.iter().any(|e| e.path.contains("subExt3"));

        assert!(has_subext1, "Should have subExt1 nested extension");
        assert!(has_subext2, "Should have subExt2 nested extension");
        assert!(has_subext3, "Should have subExt3 nested extension");
    }
}

#[tokio::test]
async fn test_complex_context_rules() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test complex context rules with [+] and [=] syntax
    let fsh = r#"
Extension: ComplexContextExtension
Id: complex-context-ext
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element  
* ^context[=].expression = "Observation"
* ^context[+].type = #extension
* ^context[=].expression = "http://example.org/Extension/BaseExt"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Complex context extension should export successfully: {:?}",
        result.err()
    );

    let structure_def = result.unwrap();

    // Verify multiple contexts are set
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 3, "Should have 3 context entries");

    // Check context types and expressions
    let patient_context = context.iter().find(|c| c.expression == "Patient");
    assert!(patient_context.is_some(), "Should have Patient context");
    assert_eq!(patient_context.unwrap().type_, "element");

    let observation_context = context.iter().find(|c| c.expression == "Observation");
    assert!(
        observation_context.is_some(),
        "Should have Observation context"
    );
    assert_eq!(observation_context.unwrap().type_, "element");

    let extension_context = context
        .iter()
        .find(|c| c.expression == "http://example.org/Extension/BaseExt");
    assert!(extension_context.is_some(), "Should have extension context");
    assert_eq!(extension_context.unwrap().type_, "extension");
}

#[tokio::test]
async fn test_extension_multiline_context_with_plus_syntax() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with SUSHI [+] and [=] syntax for multiple contexts
    let fsh = r#"
Extension: MultiContextPlusExtension
Id: multi-context-plus-ext
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element
* ^context[=].expression = "Observation"
* ^context[+].type = #element
* ^context[=].expression = "Condition"
* value[x] only string
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with [+]/[=] context should export successfully"
    );

    let structure_def = result.unwrap();

    // Verify multiple contexts are set correctly
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 3);

    // Check all contexts
    let expressions: Vec<&str> = context.iter().map(|c| c.expression.as_str()).collect();
    assert!(expressions.contains(&"Patient"));
    assert!(expressions.contains(&"Observation"));
    assert!(expressions.contains(&"Condition"));

    // All should be element type
    for ctx in context {
        assert_eq!(ctx.type_, "element");
    }
}

#[tokio::test]
async fn test_extension_multiline_context_with_numeric_indices() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with numeric indices
    let fsh = r#"
Extension: MultiContextNumericExtension
Id: multi-context-numeric-ext
* ^context[0].type = #element
* ^context[0].expression = "Patient"
* ^context[1].type = #element
* ^context[1].expression = "Observation"
* ^context[2].type = #extension
* ^context[2].expression = "http://example.org/Extension/BaseExtension"
* value[x] only boolean
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with numeric context indices should export successfully"
    );

    let structure_def = result.unwrap();

    // Verify multiple contexts are set correctly
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 3);

    // Check specific contexts
    let patient_ctx = context.iter().find(|c| c.expression == "Patient");
    assert!(patient_ctx.is_some());
    assert_eq!(patient_ctx.unwrap().type_, "element");

    let obs_ctx = context.iter().find(|c| c.expression == "Observation");
    assert!(obs_ctx.is_some());
    assert_eq!(obs_ctx.unwrap().type_, "element");

    let ext_ctx = context
        .iter()
        .find(|c| c.expression == "http://example.org/Extension/BaseExtension");
    assert!(ext_ctx.is_some());
    assert_eq!(ext_ctx.unwrap().type_, "extension");
}

#[tokio::test]
async fn test_extension_mixed_context_syntax() {
    let session = create_test_session().await;
    let exporter =
        ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string(), None)
            .await
            .expect("Failed to create exporter");

    // Test extension with mixed syntax (some numeric, some [+]/[=])
    let fsh = r#"
Extension: MixedContextExtension
Id: mixed-context-ext
* ^context[0].type = #element
* ^context[0].expression = "Patient"
* ^context[+].type = #element
* ^context[=].expression = "Observation"
* value[x] only integer
"#;

    let (root, errors) = parse_fsh(fsh);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let doc = Document::cast(root).expect("Failed to cast document");
    let extension = doc.extensions().next().expect("No extension found");

    let result = exporter.export(&extension).await;
    assert!(
        result.is_ok(),
        "Extension with mixed context syntax should export successfully"
    );

    let structure_def = result.unwrap();

    // Verify contexts are set correctly
    assert!(structure_def.context.is_some());
    let context = structure_def.context.as_ref().unwrap();
    assert_eq!(context.len(), 2);

    // Check that both Patient and Observation are present
    let expressions: Vec<&str> = context.iter().map(|c| c.expression.as_str()).collect();
    assert!(expressions.contains(&"Patient"));
    assert!(expressions.contains(&"Observation"));
}
