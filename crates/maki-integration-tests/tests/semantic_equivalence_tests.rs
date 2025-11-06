//! Semantic Equivalence Validation Tests
//!
//! This module tests that Maki and SUSHI produce semantically equivalent
//! CST structures, focusing on the meaning and functionality rather than
//! exact textual matches.

mod sushi_compatibility;

use serde_json::json;
use sushi_compatibility::{
    SushiCompatibilityHarness, TestCase, compare_semantic_equivalence, format_semantic_results,
};
use tempfile::NamedTempFile;

/// Test semantic equivalence of basic FHIR resources
#[test]
fn test_basic_resource_semantic_equivalence() {
    // Test identical resources
    let identical_patient = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [{"family": "Doe", "given": ["John"]}],
        "gender": "male"
    });

    let result =
        compare_semantic_equivalence("patient.json", &identical_patient, &identical_patient);
    assert!(
        result.is_equivalent,
        "Identical resources should be semantically equivalent"
    );
    assert_eq!(
        result.equivalence_score, 1.0,
        "Identical resources should have perfect score"
    );

    // Test resources with acceptable differences
    let maki_patient = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [{"family": "Doe", "given": ["John"]}],
        "gender": "male",
        "date": "2025-01-01",
        "publisher": "Maki"
    });

    let sushi_patient = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [{"family": "Doe", "given": ["John"]}],
        "gender": "male",
        "date": "2025-01-02",
        "publisher": "SUSHI"
    });

    let result = compare_semantic_equivalence("patient.json", &maki_patient, &sushi_patient);
    assert!(
        result.is_equivalent,
        "Resources with only metadata differences should be equivalent"
    );
    assert_eq!(
        result.equivalence_score, 1.0,
        "Metadata differences shouldn't affect semantic score"
    );
}

/// Test semantic equivalence with critical differences
#[test]
fn test_critical_semantic_differences() {
    let maki_patient = json!({
        "resourceType": "Patient",
        "id": "example1",
        "name": [{"family": "Doe", "given": ["John"]}]
    });

    let sushi_patient = json!({
        "resourceType": "Patient",
        "id": "example2", // Different ID - semantically significant
        "name": [{"family": "Doe", "given": ["John"]}]
    });

    let result = compare_semantic_equivalence("patient.json", &maki_patient, &sushi_patient);
    assert!(
        !result.is_equivalent,
        "Different IDs should not be semantically equivalent"
    );
    assert!(
        result.equivalence_score < 1.0,
        "Different IDs should reduce semantic score"
    );

    // Check that the issue was detected
    assert!(
        !result.semantic_issues.is_empty(),
        "Should detect semantic issues"
    );
    assert!(
        result
            .semantic_issues
            .iter()
            .any(|issue| issue.path.contains("id")),
        "Should detect ID difference"
    );
}

/// Test semantic equivalence of StructureDefinitions
#[test]
fn test_structure_definition_semantic_equivalence() {
    let base_structure = json!({
        "resourceType": "StructureDefinition",
        "id": "test-profile",
        "url": "http://example.com/StructureDefinition/test-profile",
        "name": "TestProfile",
        "status": "draft",
        "kind": "resource",
        "abstract": false,
        "type": "Patient",
        "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient"
    });

    // Test with different status (semantically significant)
    let mut different_status = base_structure.clone();
    different_status["status"] = json!("active");

    let result = compare_semantic_equivalence("profile.json", &base_structure, &different_status);
    assert!(
        !result.is_equivalent,
        "Different status should affect semantic equivalence"
    );

    // Test with different metadata (not semantically significant)
    let mut different_metadata = base_structure.clone();
    different_metadata["date"] = json!("2025-01-01");
    different_metadata["publisher"] = json!("Test Publisher");

    let result = compare_semantic_equivalence("profile.json", &base_structure, &different_metadata);
    assert!(
        result.is_equivalent,
        "Metadata differences should not affect semantic equivalence"
    );
}

/// Test semantic equivalence with complex nested structures
/// Test semantic equivalence validation with real FSH compilation
#[test]
#[ignore] // Only run when SUSHI is available
fn test_real_fsh_semantic_equivalence() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    // Create a test FSH file with semantic content
    let fsh_content = r#"
Profile: SemanticTestProfile
Parent: Patient
Id: semantic-test-profile
Title: "Semantic Test Profile"
Description: "Profile for testing semantic equivalence"
* ^status = #draft
* name 1..1 MS
* identifier 1..* MS
* gender 1..1 MS
"#;

    let temp_file = NamedTempFile::with_suffix(".fsh").expect("Failed to create temp file");

    std::fs::write(temp_file.path(), fsh_content).expect("Failed to write test content");

    let test_case = TestCase {
        name: "semantic_equivalence_test".to_string(),
        fsh_files: vec![temp_file.path().to_path_buf()],
        config_file: None,
        expected_outputs: vec![],
    };

    harness.add_test_case(test_case);

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let result = &results[0];

        // Generate semantic equivalence report
        let semantic_report = format_semantic_results(&result.semantic_results);
        println!("Semantic Equivalence Report:\n{}", semantic_report);

        // Check semantic equivalence score
        println!(
            "Semantic Equivalence Score: {:.2}",
            result.semantic_equivalence_score
        );

        // Assert high semantic equivalence
        assert!(
            result.semantic_equivalence_score >= 0.95,
            "Semantic equivalence score should be >= 95%, got {:.2}",
            result.semantic_equivalence_score
        );

        // Check for critical semantic issues
        let critical_issues: Vec<_> = result
            .semantic_results
            .iter()
            .flat_map(|r| &r.semantic_issues)
            .filter(|issue| matches!(issue.severity, sushi_compatibility::SemanticSeverity::High))
            .collect();

        if !critical_issues.is_empty() {
            println!("Critical semantic issues found:");
            for issue in critical_issues {
                println!("  - {}: {}", issue.path, issue.description);
            }
            panic!("Critical semantic issues detected");
        }
    }
}

/// Test regression detection for semantic equivalence
#[test]
fn test_semantic_regression_detection() {
    // Simulate a regression where a previously working construct now fails
    let working_version = json!({
        "resourceType": "StructureDefinition",
        "id": "test-profile",
        "url": "http://example.com/StructureDefinition/test-profile",
        "type": "Patient",
        "differential": {
            "element": [
                {
                    "id": "Patient.name",
                    "path": "Patient.name",
                    "min": 1,
                    "max": "1"
                }
            ]
        }
    });

    let regressed_version = json!({
        "resourceType": "StructureDefinition",
        "id": "test-profile",
        "url": "http://example.com/StructureDefinition/test-profile",
        "type": "Observation", // Changed type - major regression
        "differential": {
            "element": [
                {
                    "id": "Patient.name", // Path doesn't match type anymore
                    "path": "Patient.name",
                    "min": 1,
                    "max": "1"
                }
            ]
        }
    });

    let result =
        compare_semantic_equivalence("regression.json", &working_version, &regressed_version);

    assert!(!result.is_equivalent, "Regression should be detected");
    assert!(
        result.equivalence_score < 0.5,
        "Major regression should significantly reduce score"
    );

    // Should detect type mismatch
    assert!(
        result.semantic_issues.iter().any(|issue| matches!(
            issue.issue_type,
            sushi_compatibility::SemanticIssueType::TypeMismatch
        ) || issue.path.contains("type")),
        "Should detect type regression"
    );
}

/// Test semantic equivalence with ValueSets
/// Helper function to create test cases for semantic validation
#[allow(dead_code)]
fn create_semantic_test_case(name: &str, fsh_content: &str) -> TestCase {
    let temp_file = NamedTempFile::with_suffix(".fsh").expect("Failed to create temp file");

    std::fs::write(temp_file.path(), fsh_content).expect("Failed to write test content");

    TestCase {
        name: format!("semantic_{}", name),
        fsh_files: vec![temp_file.path().to_path_buf()],
        config_file: None,
        expected_outputs: vec![],
    }
}
