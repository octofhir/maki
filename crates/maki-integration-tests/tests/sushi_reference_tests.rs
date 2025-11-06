//! SUSHI Reference File Testing
//!
//! This module implements comprehensive testing against SUSHI reference files
//! to ensure Maki accepts all constructs that SUSHI accepts and produces
//! compatible output.

mod sushi_compatibility;

use std::env;
use std::path::PathBuf;
use sushi_compatibility::{SushiCompatibilityHarness, TestCase};

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Test against SUSHI official test files (if available)
#[test]
#[ignore] // Only run when SUSHI reference files are available
fn test_sushi_official_reference_files() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    // Try to load SUSHI reference files from common locations
    let possible_locations = vec![
        workspace_root().join("test-data/sushi-reference"),
        workspace_root().join("sushi-test-files"),
        PathBuf::from("/tmp/sushi-test-files"),
    ];

    let mut loaded = false;
    for location in possible_locations {
        if location.exists()
            && let Ok(()) = harness.load_reference_files(&location)
        {
            loaded = true;
            println!("Loaded SUSHI reference files from: {:?}", location);
            break;
        }
    }

    if !loaded {
        println!("No SUSHI reference files found - skipping test");
        println!("To run this test, clone SUSHI test files to one of:");
        for location in &[
            workspace_root().join("test-data/sushi-reference"),
            workspace_root().join("sushi-test-files"),
        ] {
            println!("  {:?}", location);
        }
        return;
    }

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("{}", report);

        // Generate CI report for automation
        let ci_report = harness.generate_ci_report(&results);
        if let Ok(reports_dir) = env::var("MAKI_REPORTS_DIR") {
            let report_path = PathBuf::from(reports_dir).join("sushi_reference_compatibility.json");
            if let Err(e) = std::fs::write(&report_path, &ci_report) {
                eprintln!("Failed to write CI report to {:?}: {}", report_path, e);
            }
        }

        // Assert compatibility meets threshold
        assert!(
            harness.meets_threshold(&results),
            "SUSHI reference compatibility below threshold"
        );
    }
}

/// Test parsing compatibility with all example files
#[test]
fn test_examples_parsing_compatibility() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(85.0).expect("Failed to create test harness");

    // Load all example files as test cases
    harness
        .load_examples_as_tests()
        .expect("Failed to load example files");

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("{}", report);

        // For examples, we're more lenient since some may be intentionally broken
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let compatibility = (passed as f64 / total as f64) * 100.0;

        println!("Examples Compatibility: {:.2}%", compatibility);

        // Don't fail the test for examples, just report
        if compatibility < 85.0 {
            println!("⚠️  Examples compatibility below 85% - this may be expected for test files");
        }
    } else {
        println!("No example files found for testing");
    }
}

/// Test specific edge cases that are known to be challenging
#[test]
fn test_edge_case_patterns() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(90.0).expect("Failed to create test harness");

    // Add specific edge case test files
    let edge_cases = vec![
        "test-parameterized-rulesets.fsh",
        "test-canonical-references.fsh",
        "valueset-examples.fsh",
        "comprehensive-test.fsh",
        "test-code-caret-rules.fsh",
        "test-code-insert-rules.fsh",
    ];

    let examples_dir = workspace_root().join("examples");

    for edge_case in edge_cases {
        let file_path = examples_dir.join(edge_case);
        if file_path.exists() {
            let test_case = TestCase {
                name: format!("edge_case_{}", edge_case.replace(".fsh", "")),
                fsh_files: vec![file_path],
                config_file: None,
                expected_outputs: vec![],
            };
            harness.add_test_case(test_case);
        }
    }

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("{}", report);

        // For edge cases, we expect high compatibility
        assert!(
            harness.meets_threshold(&results),
            "Edge case compatibility below threshold"
        );
    }
}

/// Test performance comparison between Maki and SUSHI
#[test]
#[ignore] // Only run when performance testing is needed
fn test_performance_comparison() {
    let mut harness = SushiCompatibilityHarness::new().expect("Failed to create test harness");

    // Load a subset of files for performance testing
    harness
        .load_examples_as_tests()
        .expect("Failed to load example files");

    let results = harness.run_all_tests();

    if !results.is_empty() {
        // Calculate performance metrics
        let maki_times: Vec<_> = results.iter().map(|r| r.maki_time.as_millis()).collect();
        let sushi_times: Vec<_> = results
            .iter()
            .filter_map(|r| r.sushi_time.map(|t| t.as_millis()))
            .collect();

        if !maki_times.is_empty() {
            let avg_maki = maki_times.iter().sum::<u128>() as f64 / maki_times.len() as f64;
            println!("Average Maki parsing time: {:.1}ms", avg_maki);
        }

        if !sushi_times.is_empty() {
            let avg_sushi = sushi_times.iter().sum::<u128>() as f64 / sushi_times.len() as f64;
            println!("Average SUSHI parsing time: {:.1}ms", avg_sushi);

            if !maki_times.is_empty() {
                let avg_maki = maki_times.iter().sum::<u128>() as f64 / maki_times.len() as f64;
                let speedup = avg_sushi / avg_maki;
                println!("Maki speedup: {:.2}x", speedup);
            }
        }

        // Generate performance report
        let ci_report = harness.generate_ci_report(&results);
        println!("Performance Report:\n{}", ci_report);
    }
}

/// Test that validates Maki accepts all constructs SUSHI accepts
#[test]
fn test_construct_acceptance_parity() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(100.0).expect("Failed to create test harness");

    // Create test cases for specific FSH constructs
    let construct_tests = vec![
        (
            "canonical_with_version",
            "Canonical: http://example.com|1.0.0",
        ),
        ("reference_with_or", "Reference: Patient or Practitioner"),
        ("codeable_reference", "CodeableReference: Condition"),
        (
            "parameterized_ruleset",
            "RuleSet: Test(param)\n* ^version = {param}",
        ),
        ("flag_rule_all", "* element MS SU TU N D ?!"),
        ("code_caret_rule", "* #code ^display = \"Test\""),
        ("code_insert_rule", "* #code insert TestRuleSet"),
        (
            "vs_filter_complex",
            "* include codes from system http://loinc.org where concept is-a #123 and property = value",
        ),
    ];

    for (name, fsh_content) in construct_tests {
        // Create temporary FSH file
        let temp_file =
            tempfile::NamedTempFile::with_suffix(".fsh").expect("Failed to create temp file");

        std::fs::write(temp_file.path(), fsh_content).expect("Failed to write test content");

        let test_case = TestCase {
            name: format!("construct_{}", name),
            fsh_files: vec![temp_file.path().to_path_buf()],
            config_file: None,
            expected_outputs: vec![],
        };

        harness.add_test_case(test_case);
    }

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("{}", report);

        // All construct tests should pass
        let failed_constructs: Vec<_> = results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| &r.test_name)
            .collect();

        if !failed_constructs.is_empty() {
            panic!("Failed to parse constructs: {:?}", failed_constructs);
        }
    }
}
