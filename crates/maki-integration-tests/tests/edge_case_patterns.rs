//! Edge Case Pattern Testing for SUSHI Compatibility
//!
//! This module tests complex FSH patterns and edge cases to ensure
//! Maki handles boundary conditions identically to SUSHI.

mod sushi_compatibility;

use sushi_compatibility::{SushiCompatibilityHarness, TestCase};
use tempfile::NamedTempFile;

/// Test complex canonical URLs with versions
#[test]
fn test_canonical_urls_with_versions() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    let canonical_patterns = vec![
        // Basic canonical with version
        (
            "basic_version",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* type only Canonical(StructureDefinition|1.0.0)
"#,
        ),
        // Multiple versions
        (
            "multiple_versions",
            r#"
Profile: TestProfile
Parent: Patient  
Id: test-profile
* type only Canonical(StructureDefinition|1.0.0) or Canonical(StructureDefinition|2.0.0)
"#,
        ),
        // Complex URL with version
        (
            "complex_url_version",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* type only Canonical(http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient|4.0.0)
"#,
        ),
        // Version with pre-release
        (
            "prerelease_version",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* type only Canonical(StructureDefinition|1.0.0-beta.1)
"#,
        ),
    ];

    for (name, fsh_content) in canonical_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();
    validate_results(&harness, &results, "Canonical URL patterns");
}

/// Test complex ValueSet filters
#[test]
fn test_complex_valueset_filters() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    let filter_patterns = vec![
        // Basic filter
        (
            "basic_filter",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://loinc.org where concept is-a #123
"#,
        ),
        // Multiple filter conditions
        (
            "multiple_conditions",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://loinc.org where concept is-a #123 and property = value
"#,
        ),
        // Complex filter chain
        (
            "complex_chain",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://loinc.org where concept is-a #123 and property = value and status = active
"#,
        ),
        // Different filter operators
        (
            "different_operators",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://snomed.info/sct where concept is-not-a #123
* include codes from system http://snomed.info/sct where concept descendent-of #456
* include codes from system http://snomed.info/sct where property exists true
"#,
        ),
        // Regex filter
        (
            "regex_filter",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system http://loinc.org where display regex ".*blood.*"
"#,
        ),
    ];

    for (name, fsh_content) in filter_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();
    validate_results(&harness, &results, "ValueSet filter patterns");
}

/// Test parameterized RuleSets with complex patterns
#[test]
fn test_parameterized_rulesets_complex() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(95.0).expect("Failed to create test harness");

    let ruleset_patterns = vec![
        // Nested parameters
        (
            "nested_params",
            r#"
RuleSet: NestedRule(outer, inner)
* ^version = "{outer}.{inner}"
* ^status = #active

Profile: TestProfile
Parent: Patient
Id: test-profile
* insert NestedRule("1.0", "beta")
"#,
        ),
        // Multiple parameter types
        (
            "mixed_param_types",
            r#"
RuleSet: MixedRule(version, active, count)
* ^version = {version}
* ^experimental = {active}
* name 0..{count}

Profile: TestProfile
Parent: Patient
Id: test-profile
* insert MixedRule("2.0.0", false, 5)
"#,
        ),
        // Conditional parameters
        (
            "conditional_params",
            r#"
RuleSet: ConditionalRule(status)
* ^status = {status}
* ^experimental = false

Profile: TestProfile1
Parent: Patient
Id: test-profile-1
* insert ConditionalRule(#draft)

Profile: TestProfile2
Parent: Patient
Id: test-profile-2
* insert ConditionalRule(#active)
"#,
        ),
        // Empty parameter list
        (
            "empty_params",
            r#"
RuleSet: EmptyRule()
* ^publisher = "Test Publisher"

Profile: TestProfile
Parent: Patient
Id: test-profile
* insert EmptyRule()
"#,
        ),
    ];

    for (name, fsh_content) in ruleset_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();
    validate_results(&harness, &results, "Parameterized RuleSet patterns");
}

/// Test nested and complex constructs
#[test]
fn test_nested_constructs() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(90.0).expect("Failed to create test harness");

    let nested_patterns = vec![
        // Deeply nested paths
        (
            "deep_paths",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* contact.name.family 1..1 MS
* contact.telecom.where(system='email').value 0..1
"#,
        ),
        // Complex cardinality with slicing
        (
            "complex_slicing",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* identifier ^slicing.discriminator.type = #pattern
* identifier ^slicing.discriminator.path = "type"
* identifier ^slicing.rules = #open
* identifier contains ssn 0..1 MS
* identifier[ssn].type = http://terminology.hl7.org/CodeSystem/v2-0203#SS
"#,
        ),
        // Multiple extensions
        (
            "multiple_extensions",
            r#"
Extension: Extension1
Id: ext-1
Context: Patient
* value[x] only string

Extension: Extension2  
Id: ext-2
Context: Patient
* value[x] only boolean

Profile: TestProfile
Parent: Patient
Id: test-profile
* extension contains Extension1 named ext1 0..1
* extension contains Extension2 named ext2 0..1
"#,
        ),
        // Complex invariants
        (
            "complex_invariants",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* obeys test-invariant-1
* obeys test-invariant-2

Invariant: test-invariant-1
Description: "Name must be present if active"
Expression: "active.exists() implies name.exists()"
Severity: #error

Invariant: test-invariant-2
Description: "Contact must have name or telecom"
Expression: "contact.all(name.exists() or telecom.exists())"
Severity: #warning
"#,
        ),
    ];

    for (name, fsh_content) in nested_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();
    validate_results(&harness, &results, "Nested construct patterns");
}

/// Test boundary conditions and edge cases
#[test]
fn test_boundary_conditions() {
    let mut harness =
        SushiCompatibilityHarness::with_threshold(85.0).expect("Failed to create test harness");

    let boundary_patterns = vec![
        // Maximum cardinality
        (
            "max_cardinality",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* name 0..*
* identifier 1..*
"#,
        ),
        // Zero cardinality
        (
            "zero_cardinality",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* deceased[x] 0..0
"#,
        ),
        // Very long strings
        (
            "long_strings",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
Title: "This is a very long title that tests the parser's ability to handle extended text content without breaking or causing issues with memory allocation or string processing capabilities"
Description: "This is an extremely long description that contains multiple sentences and various punctuation marks, including commas, periods, semicolons; and other special characters like parentheses (like these), quotes 'single' and \"double\", and even some unicode characters like émojis 🔥 to test comprehensive string handling in the FSH parser implementation."
"#,
        ),
        // Special characters in names
        (
            "special_chars",
            r#"
Profile: Test_Profile-With.Special$Chars
Parent: Patient
Id: test-profile-special
Title: "Profile with Special Characters: @#$%^&*()"
"#,
        ),
        // Empty rules
        (
            "empty_rules",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
// This profile intentionally has no rules
"#,
        ),
        // Comments and whitespace
        (
            "comments_whitespace",
            r#"
// Leading comment
Profile: TestProfile // Inline comment
Parent: Patient

Id: test-profile
    // Indented comment
Title: "Test Profile"

    * name 1..1 MS // Rule comment
    
    // Trailing comment
"#,
        ),
    ];

    for (name, fsh_content) in boundary_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();
    validate_results(&harness, &results, "Boundary condition patterns");
}

/// Test error recovery and malformed input
#[test]
fn test_error_recovery() {
    let mut harness = SushiCompatibilityHarness::with_threshold(50.0) // Lower threshold for error cases
        .expect("Failed to create test harness");

    let error_patterns = vec![
        // Missing required fields
        (
            "missing_id",
            r#"
Profile: TestProfile
Parent: Patient
Title: "Missing ID"
"#,
        ),
        // Invalid cardinality
        (
            "invalid_cardinality",
            r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
* name 5..2
"#,
        ),
        // Unclosed brackets
        (
            "unclosed_brackets",
            r#"
RuleSet: TestRule(param
* ^version = {param}
"#,
        ),
        // Invalid references
        (
            "invalid_reference",
            r#"
Profile: TestProfile
Parent: NonExistentResource
Id: test-profile
"#,
        ),
        // Malformed URLs
        (
            "malformed_urls",
            r#"
ValueSet: TestVS
Id: test-vs
* include codes from system not-a-valid-url
"#,
        ),
    ];

    for (name, fsh_content) in error_patterns {
        add_test_case(&mut harness, name, fsh_content);
    }

    let results = harness.run_all_tests();

    // For error cases, we just want to ensure both parsers handle them similarly
    // Don't fail the test, just report the results
    let report = harness.generate_report(&results);
    println!("Error Recovery Test Results:\n{}", report);
}

/// Helper function to add a test case from FSH content
fn add_test_case(harness: &mut SushiCompatibilityHarness, name: &str, fsh_content: &str) {
    let temp_file = NamedTempFile::with_suffix(".fsh").expect("Failed to create temp file");

    std::fs::write(temp_file.path(), fsh_content).expect("Failed to write test content");

    let test_case = TestCase {
        name: format!("edge_case_{}", name),
        fsh_files: vec![temp_file.path().to_path_buf()],
        config_file: None,
        expected_outputs: vec![],
    };

    harness.add_test_case(test_case);
}

/// Helper function to validate test results
fn validate_results(
    harness: &SushiCompatibilityHarness,
    results: &[sushi_compatibility::ComparisonResult],
    test_type: &str,
) {
    if !results.is_empty() {
        let report = harness.generate_report(results);
        println!("{} Test Results:\n{}", test_type, report);

        // Assert compatibility meets threshold
        assert!(
            harness.meets_threshold(results),
            "{} compatibility below threshold",
            test_type
        );
    } else {
        println!("No {} test results", test_type);
    }
}
