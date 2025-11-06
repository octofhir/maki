//! Comprehensive SUSHI Compliance Test Suite
//!
//! This module provides a complete test suite for validating Maki's
//! compatibility with SUSHI across all grammar features and use cases.

mod sushi_compatibility;

use std::env;
use std::path::PathBuf;
use std::time::Instant;
use sushi_compatibility::{SushiCompatibilityHarness, TestCase, format_semantic_results};
use tempfile::NamedTempFile;

/// Comprehensive test suite covering all FSH grammar features
#[test]
#[ignore] // Only run in CI or when explicitly requested
fn test_comprehensive_sushi_compliance() {
    let start_time = Instant::now();

    let mut harness =
        SushiCompatibilityHarness::with_threshold(90.0).expect("Failed to create test harness");

    // Load all available test cases
    load_all_test_cases(&mut harness);

    println!("Running comprehensive SUSHI compliance test suite...");
    println!("Total test cases: {}", harness.test_case_count());

    let results = harness.run_all_tests();
    let total_time = start_time.elapsed();

    // Generate comprehensive report
    let report = harness.generate_report(&results);
    println!("{}", report);

    // Generate semantic equivalence report
    let semantic_results: Vec<_> = results
        .iter()
        .flat_map(|r| &r.semantic_results)
        .cloned()
        .collect();

    if !semantic_results.is_empty() {
        let semantic_report = format_semantic_results(&semantic_results);
        println!("\n{}", semantic_report);
    }

    // Performance analysis
    analyze_performance(&results, total_time);

    // Generate CI artifacts
    generate_ci_artifacts(&harness, &results);

    // Assert overall compliance
    assert!(
        harness.meets_threshold(&results),
        "SUSHI compliance below threshold of {}%",
        harness.compatibility_threshold()
    );

    println!("✅ Comprehensive SUSHI compliance test completed successfully");
}

/// Test all new grammar features for identical behavior
#[test]
fn test_new_grammar_features_compliance() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    // Test cases for new grammar features
    let new_features = vec![
        (
            "parameterized_rulesets",
            r#"
RuleSet: TestRule(version, status)
* ^version = {version}
* ^status = {status}

Profile: TestProfile
Parent: Patient
Id: test-profile
* insert TestRule("1.0.0", #active)
"#,
        ),
        (
            "canonical_with_version",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* type only Canonical(StructureDefinition|1.0.0)
"#,
        ),
        (
            "complex_references",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* subject only Reference(Patient) or Reference(Group)
* performer only CodeableReference(Practitioner)
"#,
        ),
        (
            "advanced_valueset_filters",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://loinc.org where concept is-a #123 and property = value
* include codes from system http://snomed.info/sct where concept descendent-of #456
"#,
        ),
        (
            "code_caret_rules",
            r#"
CodeSystem: TestCS
Id: test-cs
* #code1 ^display = "Code 1"
* #code2 ^definition = "Code 2 definition"
"#,
        ),
        (
            "code_insert_rules",
            r#"
RuleSet: CodeRule
* ^display = "Test Display"

CodeSystem: TestCS
Id: test-cs
* #code1 insert CodeRule
"#,
        ),
        (
            "flag_combinations",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* name MS SU TU N D ?!
* identifier MS SU
"#,
        ),
        (
            "complex_slicing",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* identifier ^slicing.discriminator.type = #pattern
* identifier ^slicing.discriminator.path = "type"
* identifier ^slicing.rules = #open
* identifier contains ssn 0..1 MS and mrn 1..1 MS
* identifier[ssn].type = http://terminology.hl7.org/CodeSystem/v2-0203#SS
* identifier[mrn].type = http://terminology.hl7.org/CodeSystem/v2-0203#MR
"#,
        ),
    ];

    for (name, fsh_content) in new_features {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("New Grammar Features Compliance Report:\n{}", report);

        // All new features should have high compatibility
        assert!(
            harness.meets_threshold(&results),
            "New grammar features compliance below threshold"
        );

        // Check for any critical semantic issues
        let critical_issues: Vec<_> = results
            .iter()
            .flat_map(|r| &r.semantic_results)
            .flat_map(|sr| &sr.semantic_issues)
            .filter(|issue| matches!(issue.severity, sushi_compatibility::SemanticSeverity::High))
            .collect();

        if !critical_issues.is_empty() {
            println!("Critical issues in new grammar features:");
            for issue in critical_issues {
                println!("  - {}: {}", issue.path, issue.description);
            }
            panic!("Critical semantic issues in new grammar features");
        }
    }
}

/// Performance comparison tests
#[test]
#[ignore] // Only run for performance analysis
fn test_performance_comparison() {
    let mut harness = SushiCompatibilityHarness::new().expect("Failed to create test harness");

    // Load performance test cases (subset of examples)
    load_performance_test_cases(&mut harness);

    println!("Running performance comparison tests...");
    let start_time = Instant::now();
    let results = harness.run_all_tests();
    let total_time = start_time.elapsed();

    if !results.is_empty() {
        analyze_performance(&results, total_time);

        // Generate performance report
        let ci_report = harness.generate_ci_report(&results);

        // Save performance data for trending
        if let Ok(reports_dir) = env::var("MAKI_REPORTS_DIR") {
            let report_path = PathBuf::from(reports_dir).join("performance_comparison.json");
            if let Err(e) = std::fs::write(&report_path, &ci_report) {
                eprintln!("Failed to write performance report: {}", e);
            }
        }

        // Check if Maki is competitive with SUSHI
        let maki_times: Vec<_> = results.iter().map(|r| r.maki_time.as_millis()).collect();
        let sushi_times: Vec<_> = results
            .iter()
            .filter_map(|r| r.sushi_time.map(|t| t.as_millis()))
            .collect();

        if !maki_times.is_empty() && !sushi_times.is_empty() {
            let avg_maki = maki_times.iter().sum::<u128>() as f64 / maki_times.len() as f64;
            let avg_sushi = sushi_times.iter().sum::<u128>() as f64 / sushi_times.len() as f64;

            println!("Performance Summary:");
            println!("  Average Maki time: {:.1}ms", avg_maki);
            println!("  Average SUSHI time: {:.1}ms", avg_sushi);

            if avg_maki > 0.0 {
                let speedup = avg_sushi / avg_maki;
                println!("  Maki speedup: {:.2}x", speedup);

                // Maki should be at least competitive (not more than 2x slower)
                assert!(
                    speedup >= 0.5,
                    "Maki is significantly slower than SUSHI ({}x slower)",
                    1.0 / speedup
                );
            }
        }
    }
}

/// Continuous integration validation test
#[test]
fn test_ci_validation() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(85.0).expect("Failed to create test harness");

    // Load a representative subset for CI
    load_ci_test_cases(&mut harness);

    let results = harness.run_all_tests();

    if !results.is_empty() {
        // Generate CI report
        let ci_report = harness.generate_ci_report(&results);
        println!("CI Validation Report:\n{}", ci_report);

        // Save CI artifacts
        generate_ci_artifacts(&harness, &results);

        // CI should pass with lower threshold for faster feedback
        assert!(
            harness.meets_threshold(&results),
            "CI validation failed - compatibility below {}%",
            harness.compatibility_threshold()
        );
    }
}

/// Regression testing against known working cases
#[test]
fn test_regression_prevention() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    // Load known working test cases
    load_regression_test_cases(&mut harness);

    let results = harness.run_all_tests();

    if !results.is_empty() {
        let report = harness.generate_report(&results);
        println!("Regression Test Report:\n{}", report);

        // Regression tests should have very high compatibility
        assert!(
            harness.meets_threshold(&results),
            "Regression detected - compatibility dropped below {}%",
            harness.compatibility_threshold()
        );

        // Check for any new semantic issues
        let semantic_issues: Vec<_> = results
            .iter()
            .flat_map(|r| &r.semantic_results)
            .filter(|sr| !sr.is_equivalent)
            .collect();

        if !semantic_issues.is_empty() {
            println!("Semantic regressions detected:");
            for result in semantic_issues {
                println!(
                    "  File: {} (Score: {:.2})",
                    result.file, result.equivalence_score
                );
                for issue in &result.semantic_issues {
                    println!("    - {}: {}", issue.path, issue.description);
                }
            }
            panic!("Semantic regressions detected");
        }
    }
}

/// Load all available test cases
fn load_all_test_cases(harness: &mut SushiCompatibilityHarness) {
    // Load examples
    if let Err(e) = harness.load_examples_as_tests() {
        eprintln!("Warning: Failed to load examples: {}", e);
    }

    // Try to load SUSHI reference files
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    let reference_locations = vec![
        workspace_root.join("test-data/sushi-reference"),
        workspace_root.join("sushi-test-files"),
    ];

    for location in reference_locations {
        if location.exists()
            && let Ok(()) = harness.load_reference_files(&location)
        {
            println!("Loaded SUSHI reference files from: {:?}", location);
            break;
        }
    }

    // Add specific compliance test cases
    add_compliance_test_cases(harness);
}

/// Add specific test cases for compliance testing
fn add_compliance_test_cases(harness: &mut SushiCompatibilityHarness) {
    let compliance_cases = vec![
        (
            "minimal_profile",
            r#"
Profile: MinimalProfile
Parent: Patient
Id: minimal-profile
"#,
        ),
        (
            "complete_profile",
            r#"
Profile: CompleteProfile
Parent: Patient
Id: complete-profile
Title: "Complete Profile"
Description: "A complete profile with all metadata"
* ^status = #active
* ^version = "1.0.0"
* ^publisher = "Test Publisher"
* ^experimental = false
* name 1..1 MS
* identifier 1..* MS
"#,
        ),
        (
            "extension_definition",
            r#"
Extension: TestExtension
Id: test-extension
Title: "Test Extension"
Description: "Test extension definition"
Context: Patient
* value[x] only string
"#,
        ),
        (
            "valueset_definition",
            r#"
ValueSet: TestValueSet
Id: test-valueset
Title: "Test ValueSet"
Description: "Test valueset definition"
* ^status = #active
* include codes from system http://loinc.org
"#,
        ),
        (
            "codesystem_definition",
            r#"
CodeSystem: TestCodeSystem
Id: test-codesystem
Title: "Test CodeSystem"
Description: "Test codesystem definition"
* ^status = #active
* #code1 "Code 1"
* #code2 "Code 2"
"#,
        ),
    ];

    for (name, fsh_content) in compliance_cases {
        add_test_case(harness, name, fsh_content);
    }
}

/// Load performance-focused test cases
fn load_performance_test_cases(harness: &mut SushiCompatibilityHarness) {
    // Load a subset of examples for performance testing
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let examples_dir = workspace_root.join("examples");

    if examples_dir.exists() {
        let performance_files = vec![
            "comprehensive-test.fsh",
            "test-parameterized-rulesets.fsh",
            "valueset-examples.fsh",
            "test-canonical-references.fsh",
        ];

        for file_name in performance_files {
            let file_path = examples_dir.join(file_name);
            if file_path.exists() {
                let test_case = TestCase {
                    name: format!("perf_{}", file_name.replace(".fsh", "")),
                    fsh_files: vec![file_path],
                    config_file: None,
                    expected_outputs: vec![],
                };
                harness.add_test_case(test_case);
            }
        }
    }
}

/// Load CI-focused test cases (fast subset)
fn load_ci_test_cases(harness: &mut SushiCompatibilityHarness) {
    let ci_cases = vec![
        (
            "ci_basic_profile",
            r#"
Profile: CIBasicProfile
Parent: Patient
Id: ci-basic-profile
* name 1..1 MS
"#,
        ),
        (
            "ci_extension",
            r#"
Extension: CIExtension
Id: ci-extension
Context: Patient
* value[x] only string
"#,
        ),
        (
            "ci_valueset",
            r#"
ValueSet: CIValueSet
Id: ci-valueset
* include codes from system http://loinc.org
"#,
        ),
    ];

    for (name, fsh_content) in ci_cases {
        add_test_case(harness, name, fsh_content);
    }
}

/// Load regression test cases (known working patterns)
fn load_regression_test_cases(harness: &mut SushiCompatibilityHarness) {
    let regression_cases = vec![
        (
            "regression_profile",
            r#"
Profile: RegressionProfile
Parent: Patient
Id: regression-profile
Title: "Regression Test Profile"
* ^status = #active
* name 1..1 MS
* identifier 1..* MS
"#,
        ),
        (
            "regression_ruleset",
            r#"
RuleSet: RegressionRule(version)
* ^version = {version}

Profile: RegressionProfile
Parent: Patient
Id: regression-profile
* insert RegressionRule("1.0.0")
"#,
        ),
    ];

    for (name, fsh_content) in regression_cases {
        add_test_case(harness, name, fsh_content);
    }
}

/// Analyze performance results
fn analyze_performance(
    results: &[sushi_compatibility::ComparisonResult],
    total_time: std::time::Duration,
) {
    println!("\nPerformance Analysis:");
    println!("===================");
    println!("Total execution time: {:.2}s", total_time.as_secs_f64());

    if !results.is_empty() {
        let maki_times: Vec<_> = results.iter().map(|r| r.maki_time.as_millis()).collect();
        let sushi_times: Vec<_> = results
            .iter()
            .filter_map(|r| r.sushi_time.map(|t| t.as_millis()))
            .collect();

        if !maki_times.is_empty() {
            let total_maki = maki_times.iter().sum::<u128>();
            let avg_maki = total_maki as f64 / maki_times.len() as f64;
            let min_maki = *maki_times.iter().min().unwrap();
            let max_maki = *maki_times.iter().max().unwrap();

            println!("Maki Performance:");
            println!("  Total: {}ms", total_maki);
            println!("  Average: {:.1}ms", avg_maki);
            println!("  Min: {}ms", min_maki);
            println!("  Max: {}ms", max_maki);
        }

        if !sushi_times.is_empty() {
            let total_sushi = sushi_times.iter().sum::<u128>();
            let avg_sushi = total_sushi as f64 / sushi_times.len() as f64;
            let min_sushi = *sushi_times.iter().min().unwrap();
            let max_sushi = *sushi_times.iter().max().unwrap();

            println!("SUSHI Performance:");
            println!("  Total: {}ms", total_sushi);
            println!("  Average: {:.1}ms", avg_sushi);
            println!("  Min: {}ms", min_sushi);
            println!("  Max: {}ms", max_sushi);

            if !maki_times.is_empty() {
                let avg_maki = maki_times.iter().sum::<u128>() as f64 / maki_times.len() as f64;
                let speedup = avg_sushi / avg_maki;
                println!("Relative Performance:");
                println!("  Maki speedup: {:.2}x", speedup);
            }
        }
    }
}

/// Generate CI artifacts and reports
fn generate_ci_artifacts(
    harness: &SushiCompatibilityHarness,
    results: &[sushi_compatibility::ComparisonResult],
) {
    if let Ok(reports_dir) = env::var("MAKI_REPORTS_DIR") {
        let reports_path = PathBuf::from(reports_dir);

        // Ensure reports directory exists
        if let Err(e) = std::fs::create_dir_all(&reports_path) {
            eprintln!("Failed to create reports directory: {}", e);
            return;
        }

        // Generate JSON report for CI
        let ci_report = harness.generate_ci_report(results);
        let ci_report_path = reports_path.join("sushi_compliance.json");
        if let Err(e) = std::fs::write(&ci_report_path, &ci_report) {
            eprintln!("Failed to write CI report: {}", e);
        }

        // Generate human-readable report
        let human_report = harness.generate_report(results);
        let human_report_path = reports_path.join("sushi_compliance.txt");
        if let Err(e) = std::fs::write(&human_report_path, &human_report) {
            eprintln!("Failed to write human report: {}", e);
        }

        // Generate semantic equivalence report
        let semantic_results: Vec<_> = results
            .iter()
            .flat_map(|r| &r.semantic_results)
            .cloned()
            .collect();

        if !semantic_results.is_empty() {
            let semantic_report = format_semantic_results(&semantic_results);
            let semantic_report_path = reports_path.join("semantic_equivalence.txt");
            if let Err(e) = std::fs::write(&semantic_report_path, &semantic_report) {
                eprintln!("Failed to write semantic report: {}", e);
            }
        }

        println!("CI artifacts generated in: {:?}", reports_path);
    }
}

/// Helper function to add a test case from FSH content
fn add_test_case(harness: &mut SushiCompatibilityHarness, name: &str, fsh_content: &str) {
    let temp_file = NamedTempFile::with_suffix(".fsh").expect("Failed to create temp file");

    std::fs::write(temp_file.path(), fsh_content).expect("Failed to write test content");

    let test_case = TestCase {
        name: format!("compliance_{}", name),
        fsh_files: vec![temp_file.path().to_path_buf()],
        config_file: None,
        expected_outputs: vec![],
    };

    harness.add_test_case(test_case);
}
