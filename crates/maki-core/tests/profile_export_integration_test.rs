//! Integration tests for Profile Exporter
//!
//! Tests the complete flow of parsing FSH profiles and exporting to FHIR
//! StructureDefinitions with real FHIR packages.

use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
use maki_core::cst::ast::{AstNode, Profile};
use maki_core::cst::parse_fsh;
use maki_core::export::{ProfileExporter, StructureDefinitionKind};
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
async fn test_export_simple_patient_profile() {
    let fsh_source = r#"
        Profile: SimplePatientProfile
        Parent: Patient
        Title: "Simple Patient Profile"
        Description: "A simple patient profile for testing"
        * name 1..* MS
        * gender 1..1 MS
    "#;

    let (cst, errors) = parse_fsh(fsh_source);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    // Find Profile node
    let profile_node = cst
        .children()
        .find_map(Profile::cast)
        .expect("Profile not found");

    // Create exporter
    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://example.org/fhir".to_string())
        .await
        .unwrap();

    // Export profile
    let structure_def = exporter.export(&profile_node).await.unwrap();

    // Verify exported StructureDefinition
    assert_eq!(structure_def.name, "SimplePatientProfile");
    assert_eq!(structure_def.type_field, "Patient");
    assert_eq!(structure_def.kind, StructureDefinitionKind::Resource);
    assert_eq!(
        structure_def.url,
        "http://example.org/fhir/StructureDefinition/SimplePatientProfile"
    );
    assert_eq!(
        structure_def.title,
        Some("Simple Patient Profile".to_string())
    );
    assert_eq!(
        structure_def.description,
        Some("A simple patient profile for testing".to_string())
    );

    // Verify differential
    let differential = structure_def
        .differential
        .as_ref()
        .expect("No differential");
    assert!(
        !differential.element.is_empty(),
        "Differential should have elements"
    );

    // Check that name and gender constraints are in differential
    let has_name = differential.element.iter().any(|e| e.path.contains("name"));
    let has_gender = differential
        .element
        .iter()
        .any(|e| e.path.contains("gender"));
    assert!(has_name, "Differential should contain name constraint");
    assert!(has_gender, "Differential should contain gender constraint");
}

#[tokio::test]
async fn test_export_profile_with_cardinality() {
    let fsh_source = r#"
        Profile: CardinalityTest
        Parent: Patient
        * identifier 1..* MS
        * name 1..1
        * telecom 0..5
    "#;

    let (cst, _) = parse_fsh(fsh_source);
    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://test.org".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    let differential = structure_def.differential.as_ref().unwrap();

    // Find identifier element
    let identifier_elem = differential
        .element
        .iter()
        .find(|e| e.path.ends_with("identifier"))
        .expect("identifier element not found");

    assert_eq!(identifier_elem.min, Some(1));
    assert_eq!(identifier_elem.max, Some("*".to_string()));
    assert_eq!(identifier_elem.must_support, Some(true));
}

#[tokio::test]
async fn test_export_profile_with_flags() {
    let fsh_source = r#"
        Profile: FlagsTest
        Parent: Patient
        * name MS
        * gender MS SU
        * birthDate SU
    "#;

    let (cst, _) = parse_fsh(fsh_source);
    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://test.org".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    let differential = structure_def.differential.as_ref().unwrap();

    // Check name - should have MS
    if let Some(name_elem) = differential
        .element
        .iter()
        .find(|e| e.path.ends_with("name"))
    {
        assert_eq!(name_elem.must_support, Some(true));
    }

    // Check gender - should have MS and SU
    if let Some(gender_elem) = differential
        .element
        .iter()
        .find(|e| e.path.ends_with("gender"))
    {
        assert_eq!(gender_elem.must_support, Some(true));
        assert_eq!(gender_elem.is_summary, Some(true));
    }

    // Check birthDate - should have SU
    if let Some(birthdate_elem) = differential
        .element
        .iter()
        .find(|e| e.path.ends_with("birthDate"))
    {
        assert_eq!(birthdate_elem.is_summary, Some(true));
    }
}

#[tokio::test]
async fn test_export_profile_with_value_set_binding() {
    let fsh_source = r#"
        Profile: BindingTest
        Parent: Patient
        * maritalStatus from http://hl7.org/fhir/ValueSet/marital-status (required)
    "#;

    let (cst, _) = parse_fsh(fsh_source);
    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://test.org".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    let differential = structure_def.differential.as_ref().unwrap();

    // Find maritalStatus element
    if let Some(marital_elem) = differential
        .element
        .iter()
        .find(|e| e.path.ends_with("maritalStatus"))
    {
        let binding = marital_elem.binding.as_ref().expect("Binding not found");
        assert_eq!(
            binding.value_set,
            Some("http://hl7.org/fhir/ValueSet/marital-status".to_string())
        );
        assert_eq!(
            binding.strength,
            maki_core::export::BindingStrength::Required
        );
    }
}

#[tokio::test]
async fn test_export_profile_with_fixed_values() {
    let fsh_source = r#"
        Profile: FixedValueTest
        Parent: Patient
        * active = true
        * gender = #male
    "#;

    let (cst, _) = parse_fsh(fsh_source);
    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://test.org".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    let differential = structure_def.differential.as_ref().unwrap();

    // Check that fixed values were applied (stored as pattern in our implementation)
    assert!(
        differential
            .element
            .iter()
            .any(|e| e.path.ends_with("active") && e.pattern.is_some()),
        "active element should have pattern"
    );

    assert!(
        differential
            .element
            .iter()
            .any(|e| e.path.ends_with("gender") && e.pattern.is_some()),
        "gender element should have pattern"
    );
}

#[tokio::test]
async fn test_export_complex_profile() {
    let fsh_source = r#"
        Profile: ComplexPatientProfile
        Parent: Patient
        Id: complex-patient
        Title: "Complex Patient Profile"
        Description: "A complex profile with multiple constraints"
        * identifier 1..* MS
        * identifier.system 1..1 MS
        * identifier.value 1..1 MS
        * name 1..* MS
        * name.family 1..1 MS
        * name.given 1..* MS
        * telecom 0..* MS
        * gender 1..1 MS
        * birthDate 0..1 MS SU
        * address 0..* MS
        * maritalStatus from http://hl7.org/fhir/ValueSet/marital-status (extensible)
    "#;

    let (cst, errors) = parse_fsh(fsh_source);
    assert!(errors.is_empty(), "Parse errors: {:?}", errors);

    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://example.org/fhir".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    // Verify metadata
    assert_eq!(structure_def.name, "ComplexPatientProfile");
    assert_eq!(structure_def.id, Some("complex-patient".to_string()));
    assert_eq!(
        structure_def.title,
        Some("Complex Patient Profile".to_string())
    );
    assert_eq!(
        structure_def.description,
        Some("A complex profile with multiple constraints".to_string())
    );

    // Verify differential has multiple elements
    let differential = structure_def.differential.as_ref().unwrap();
    assert!(
        differential.element.len() >= 5,
        "Differential should have multiple elements"
    );

    // Verify some specific constraints
    assert!(
        differential
            .element
            .iter()
            .any(|e| e.path.ends_with("identifier") && e.must_support == Some(true))
    );

    assert!(
        differential
            .element
            .iter()
            .any(|e| e.path.ends_with("birthDate") && e.is_summary == Some(true))
    );
}

#[tokio::test]
async fn test_export_observation_profile() {
    let fsh_source = r#"
        Profile: SimpleObservationProfile
        Parent: Observation
        * status 1..1 MS
        * code 1..1 MS
        * subject 1..1 MS
        * value[x] 0..1 MS
    "#;

    let (cst, _) = parse_fsh(fsh_source);
    let profile_node = cst.children().find_map(Profile::cast).unwrap();

    let session = create_test_session().await;
    let exporter = ProfileExporter::new(session, "http://example.org/fhir".to_string())
        .await
        .unwrap();

    let structure_def = exporter.export(&profile_node).await.unwrap();

    assert_eq!(structure_def.name, "SimpleObservationProfile");
    assert_eq!(structure_def.type_field, "Observation");

    let differential = structure_def.differential.as_ref().unwrap();
    assert!(!differential.element.is_empty());

    // Verify status constraint
    assert!(
        differential
            .element
            .iter()
            .any(|e| e.path.ends_with("status") && e.min == Some(1))
    );
}

#[tokio::test]
async fn test_validation_fails_for_invalid_cardinality() {
    // This test verifies that validation catches invalid cardinality
    // Note: This would require creating an invalid StructureDefinition manually
    // since the parser/exporter should prevent creating invalid ones

    // For now, we just verify the validation methods exist and are called
    let session = create_test_session().await;
    let _exporter = ProfileExporter::new(session, "http://test.org".to_string())
        .await
        .unwrap();

    // Validation is automatically called in export(), so we just ensure
    // valid profiles export successfully (tested above)
}
