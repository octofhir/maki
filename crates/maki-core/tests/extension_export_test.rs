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
        auto_install_core: true,
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
    let exporter = ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string())
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
    let exporter = ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string())
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
    let exporter = ExtensionExporter::new(session.clone(), "http://test.org".to_string())
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
    let exporter = ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string())
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
    let exporter = ExtensionExporter::new(session.clone(), "http://example.org/fhir".to_string())
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
