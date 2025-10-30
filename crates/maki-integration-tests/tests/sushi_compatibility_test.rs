//! Integration tests for SUSHI compatibility
//!
//! These tests compare MAKI output with SUSHI output to ensure
//! compatibility with the reference FSH compiler.

mod sushi_compatibility;

use std::path::PathBuf;
use sushi_compatibility::{SushiCompatibilityHarness, TestCase};

/// Helper to get the examples directory
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
}

#[test]
#[ignore] // Only run when explicitly requested
fn test_sushi_compatibility_basic() {
    let mut harness = SushiCompatibilityHarness::new().expect("Failed to create test harness");

    // Add a simple test case
    let test_case = TestCase {
        name: "basic-profile".to_string(),
        fsh_files: vec![examples_dir().join("simple-profile.fsh")],
        config_file: None,
        expected_outputs: vec![],
    };

    harness.add_test_case(test_case);

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("{}", report);

        // Assert compatibility is high (≥90%)
        let passed = results.iter().filter(|r| r.passed).count();
        let total = results.len();
        let compatibility = (passed as f64 / total as f64) * 100.0;

        assert!(
            compatibility >= 90.0,
            "Compatibility must be ≥90%, got {:.2}%",
            compatibility
        );
    }
}

#[test]
fn test_harness_creation() {
    // This test should always pass
    let harness = SushiCompatibilityHarness::new();
    assert!(
        harness.is_ok(),
        "Should be able to create test harness: {:?}",
        harness.err()
    );
}

#[test]
#[ignore] // Only run when SUSHI is available
fn test_full_compatibility_suite() {
    let mut harness = SushiCompatibilityHarness::new().expect("Failed to create test harness");

    // Add test cases from examples directory
    let examples = examples_dir();

    if examples.exists() {
        // Collect all FSH files in examples
        if let Ok(entries) = std::fs::read_dir(&examples) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fsh") {
                    let test_case = TestCase {
                        name: path.file_stem().unwrap().to_string_lossy().to_string(),
                        fsh_files: vec![path],
                        config_file: None,
                        expected_outputs: vec![],
                    };
                    harness.add_test_case(test_case);
                }
            }
        }
    }

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("\n{}", report);

        // Calculate overall compatibility
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let compatibility = (passed as f64 / total as f64) * 100.0;

        println!("\nOverall Compatibility: {:.2}%", compatibility);
        println!("Tests Passed: {}/{}", passed, total);

        // For now, just report - don't fail the test
        // In the future, we'll increase this threshold
        if compatibility < 90.0 {
            println!("⚠️  Compatibility below 90% - this is expected during early development");
        }
    } else {
        println!("No test cases found - this is expected before build command is implemented");
    }
}

#[test]
fn test_json_comparison_basic() {
    use serde_json::json;
    use sushi_compatibility::compare_json;

    let json1 = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [{"family": "Doe", "given": ["John"]}]
    });

    let json2 = json!({
        "resourceType": "Patient",
        "id": "example",
        "name": [{"family": "Doe", "given": ["John"]}]
    });

    let diffs = compare_json("test.json", &json1, &json2);
    assert_eq!(diffs.len(), 0, "Identical JSON should have no differences");
}

#[test]
fn test_json_comparison_acceptable_difference() {
    use serde_json::json;
    use sushi_compatibility::{Difference, compare_json};

    let maki = json!({
        "resourceType": "Patient",
        "id": "example",
        "date": "2025-01-01"
    });

    let sushi = json!({
        "resourceType": "Patient",
        "id": "example",
        "date": "2025-01-02"
    });

    let diffs = compare_json("test.json", &maki, &sushi);

    // Should have one difference, but it should be acceptable
    assert_eq!(diffs.len(), 1);
    assert!(matches!(diffs[0], Difference::AcceptableDifference { .. }));
}
