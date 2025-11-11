//! Integration tests for the complete foundation pipeline
//!
//! Tests the integration of:
//! - FHIR Models (Task 01)
//! - ResourceLake (Task 02)
//! - File Loading (Task 03)
//! - Canonical Integration (Task 04)

use maki_decompiler::*;
use maki_core::canonical::FhirRelease;
use maki_core::Fishable;
use tempfile::TempDir;
use std::fs;
use serial_test::serial;

/// Helper to create a test StructureDefinition JSON
fn create_test_profile_json(url: &str, name: &str, base: &str) -> String {
    format!(
        r#"{{
            "resourceType": "StructureDefinition",
            "url": "{}",
            "name": "{}",
            "status": "active",
            "baseDefinition": "{}",
            "derivation": "constraint"
        }}"#,
        url, name, base
    )
}

/// Test complete foundation pipeline: load files → lake → fish
#[tokio::test]
#[serial]
async fn test_complete_foundation_pipeline() {
    // 1. Setup: Create test FHIR files
    let temp_dir = TempDir::new().unwrap();
    let profile_file = temp_dir.path().join("my-patient.json");

    let profile_json = create_test_profile_json(
        "http://example.org/StructureDefinition/MyPatient",
        "MyPatient",
        "http://hl7.org/fhir/StructureDefinition/Patient",
    );

    fs::write(&profile_file, profile_json).unwrap();

    // 2. Initialize canonical manager with FHIR R4
    let dependencies = vec![];

    let session = setup_canonical_environment(FhirRelease::R4, dependencies)
        .await
        .unwrap();

    // 3. Create ResourceLake
    let mut lake = ResourceLake::new(session);

    // 4. Load files
    let mut loader = FileLoader::new();
    let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

    assert_eq!(stats.loaded, 1);
    assert_eq!(stats.errors, 0);

    // 5. Test local resource lookup
    let local_profile = lake
        .get_structure_definition("http://example.org/StructureDefinition/MyPatient");

    assert!(local_profile.is_some());
    assert_eq!(local_profile.unwrap().name, "MyPatient");

    // 6. Test external resource lookup (falls back to canonical manager)
    let patient_base = lake
        .fish_by_url("http://hl7.org/fhir/StructureDefinition/Patient")
        .await
        .unwrap();

    assert!(patient_base.is_some());

    // 7. Test stats
    let lake_stats = lake.stats();
    assert_eq!(lake_stats.structure_definitions, 1);
    assert_eq!(lake_stats.value_sets, 0);
    assert_eq!(lake_stats.code_systems, 0);
}

/// Test with multiple resources in directory
#[tokio::test]
#[serial]
async fn test_multiple_resources() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple profiles
    for i in 1..=3 {
        let file_path = temp_dir.path().join(format!("profile{}.json", i));
        let json = create_test_profile_json(
            &format!("http://example.org/StructureDefinition/Profile{}", i),
            &format!("Profile{}", i),
            "http://hl7.org/fhir/StructureDefinition/Patient",
        );
        fs::write(&file_path, json).unwrap();
    }

    // Create a ValueSet
    let vs_file = temp_dir.path().join("valueset.json");
    fs::write(
        &vs_file,
        r#"{
            "resourceType": "ValueSet",
            "url": "http://example.org/ValueSet/TestVS",
            "name": "TestVS",
            "status": "active"
        }"#,
    )
    .unwrap();

    // Setup and load
    let session = setup_canonical_environment(FhirRelease::R4, vec![])
        .await
        .unwrap();
    let mut lake = ResourceLake::new(session);
    let mut loader = FileLoader::new();
    let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

    assert_eq!(stats.loaded, 4);
    assert_eq!(stats.errors, 0);

    // Verify resources
    let stats = lake.stats();
    assert_eq!(stats.structure_definitions, 3);
    assert_eq!(stats.value_sets, 1);
}

/// Test error handling for invalid files
#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Create invalid JSON file
    let bad_file = temp_dir.path().join("bad.json");
    fs::write(&bad_file, "{ invalid json }").unwrap();

    // Create valid file
    let good_file = temp_dir.path().join("good.json");
    let json = create_test_profile_json(
        "http://example.org/StructureDefinition/GoodProfile",
        "GoodProfile",
        "http://hl7.org/fhir/StructureDefinition/Patient",
    );
    fs::write(&good_file, json).unwrap();

    // Setup and load
    let session = setup_canonical_environment(FhirRelease::R4, vec![])
        .await
        .unwrap();
    let mut lake = ResourceLake::new(session);
    let mut loader = FileLoader::new();
    let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

    // Should load good file, record error for bad file
    assert_eq!(stats.loaded, 1);
    assert_eq!(stats.errors, 1);
    assert_eq!(stats.error_details.len(), 1);
}

/// Test convenience function create_lake_with_session
#[tokio::test]
#[serial]
async fn test_create_lake_with_session() {
    let dependencies = vec![];

    let lake = create_lake_with_session(FhirRelease::R4, dependencies)
        .await
        .unwrap();

    // Should be able to fish for FHIR core resources
    let patient = lake
        .fish_by_url("http://hl7.org/fhir/StructureDefinition/Patient")
        .await
        .unwrap();

    assert!(patient.is_some());
}

/// Test with R5 release
#[tokio::test]
#[serial]
async fn test_r5_release() {
    let lake = create_lake_with_session(FhirRelease::R5, vec![])
        .await
        .unwrap();

    // Should be able to fish for R5 resources
    let patient = lake
        .fish_by_url("http://hl7.org/fhir/StructureDefinition/Patient")
        .await
        .unwrap();

    assert!(patient.is_some());
}

/// Test parsing helper functions
#[test]
fn test_parse_helpers() {
    // Test parse_fhir_release
    assert_eq!(parse_fhir_release("R4").unwrap(), FhirRelease::R4);
    assert_eq!(parse_fhir_release("4.0.1").unwrap(), FhirRelease::R4);
    assert!(parse_fhir_release("invalid").is_err());

    // Test parse_package_spec
    let pkg = parse_package_spec("hl7.fhir.us.core@5.0.1").unwrap();
    assert_eq!(pkg.name, "hl7.fhir.us.core");
    assert_eq!(pkg.version, "5.0.1");
    assert!(parse_package_spec("invalid").is_err());

    // Test parse_cli_dependencies
    let (release, deps) = parse_cli_dependencies(
        "R4",
        &["hl7.fhir.us.core@5.0.1".to_string()],
    )
    .unwrap();
    assert_eq!(release, FhirRelease::R4);
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "hl7.fhir.us.core");
}
