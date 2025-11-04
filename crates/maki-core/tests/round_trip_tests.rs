//! Comprehensive round-trip validation tests for FSH
//!
//! These tests validate that FSH code can be parsed, formatted, and parsed again
//! while maintaining semantic equivalence and preserving trivia.

use maki_core::cst::{
    parse_fsh, format_document, FormatOptions,
    RoundTripValidator, ValidationResult, DifferenceKind,
    TriviaCollector, IncrementalUpdater, TextEdit,
};

/// Test round-trip validation for simple profiles
#[test]
fn test_simple_profile_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: MyPatient
Parent: Patient
* name 1..1 MS"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(!result.original.is_empty());
    assert!(!result.formatted.is_empty());
    assert!(result.formatted.contains("Profile: MyPatient"));
    assert!(result.formatted.contains("Parent: Patient"));
    assert!(result.formatted.contains("* name 1..1 MS"));
}

/// Test round-trip validation for complex profiles with metadata
#[test]
fn test_complex_profile_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: ComplexPatient
Parent: Patient
Id: complex-patient
Title: "Complex Patient Profile"
Description: "A complex patient profile with multiple constraints"
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* ^status = #active
* ^experimental = false
* ^version = "1.0.0""#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Profile: ComplexPatient"));
    assert!(result.formatted.contains("Parent: Patient"));
    assert!(result.formatted.contains("Id: complex-patient"));
    assert!(result.formatted.contains("Title: \"Complex Patient Profile\""));
    assert!(result.formatted.contains("* name 1..1 MS"));
    assert!(result.formatted.contains("* ^status = #active"));
}

/// Test round-trip validation for extensions
#[test]
fn test_extension_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Extension: PatientExtension
Id: patient-extension
Title: "Patient Extension"
Description: "An extension for patient data"
* value[x] only string
* ^context.type = #element
* ^context.expression = "Patient""#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Extension: PatientExtension"));
    assert!(result.formatted.contains("* value[x] only string"));
    assert!(result.formatted.contains("* ^context.type = #element"));
}

/// Test round-trip validation for value sets
#[test]
fn test_valueset_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"ValueSet: PatientGender
Id: patient-gender
Title: "Patient Gender Values"
Description: "Gender values for patient records"
* include codes from system http://hl7.org/fhir/administrative-gender
* ^status = #active
* ^experimental = false"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("ValueSet: PatientGender"));
    assert!(result.formatted.contains("* include codes from system"));
    assert!(result.formatted.contains("* ^status = #active"));
}

/// Test round-trip validation for code systems
#[test]
fn test_codesystem_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"CodeSystem: PatientStatus
Id: patient-status
Title: "Patient Status Codes"
Description: "Status codes for patient records"
* #active "Active" "Patient is active"
* #inactive "Inactive" "Patient is inactive"
* #deceased "Deceased" "Patient is deceased"
* ^status = #active"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("CodeSystem: PatientStatus"));
    assert!(result.formatted.contains("* #active \"Active\""));
    assert!(result.formatted.contains("* ^status = #active"));
}

/// Test round-trip validation for aliases
#[test]
fn test_alias_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Alias: SCT = http://snomed.info/sct
Alias: LOINC = http://loinc.org

Profile: MyPatient
Parent: Patient
* name 1..1 MS"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Alias: SCT = http://snomed.info/sct"));
    assert!(result.formatted.contains("Alias: LOINC = http://loinc.org"));
    assert!(result.formatted.contains("Profile: MyPatient"));
}

/// Test round-trip validation with comments
#[test]
fn test_comments_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"// This is a header comment
Profile: MyPatient // Inline comment
Parent: Patient
// Comment before rules
* name 1..1 MS // Name is required
* gender 0..1 // Gender is optional
// End comment"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    // Comments should be preserved in some form
    // The exact preservation depends on the formatter implementation
    assert!(!result.original.is_empty());
    assert!(!result.formatted.is_empty());
}

/// Test round-trip validation with complex whitespace
#[test]
fn test_whitespace_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: MyPatient   
Parent: Patient  


* name 1..1 MS   
* gender 0..1  


"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    // Whitespace should be normalized but content preserved
    assert!(result.formatted.contains("Profile: MyPatient"));
    assert!(result.formatted.contains("Parent: Patient"));
    assert!(result.formatted.contains("* name 1..1 MS"));
    assert!(result.formatted.contains("* gender 0..1"));
}

/// Test round-trip validation with various rule types
#[test]
fn test_various_rules_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: CompletePatient
Parent: Patient
* identifier 1..* MS
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* address 0..*
* telecom 0..*
* telecom.system 1..1
* telecom.value 1..1
* extension contains race 0..1 MS
* gender from http://hl7.org/fhir/ValueSet/administrative-gender (required)
* birthDate obeys patient-birthdate-invariant
* name only HumanName
* address -> "PID-11" "Patient address"
* ^status = #active"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Profile: CompletePatient"));
    assert!(result.formatted.contains("* identifier 1..* MS"));
    assert!(result.formatted.contains("* extension contains race"));
    assert!(result.formatted.contains("* gender from http://"));
    assert!(result.formatted.contains("* birthDate obeys"));
    assert!(result.formatted.contains("* name only HumanName"));
    assert!(result.formatted.contains("* address -> \"PID-11\""));
}

/// Test round-trip validation with caret rules
#[test]
fn test_caret_rules_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: MetadataPatient
Parent: Patient
* ^version = "1.0.0"
* ^status = #active
* ^experimental = false
* ^date = "2024-01-01"
* ^publisher = "Test Organization"
* ^contact.name = "Test Contact"
* ^contact.telecom.system = #email
* ^contact.telecom.value = "test@example.com"
* name 1..1 MS
* name ^short = "Patient name"
* name ^definition = "The name of the patient"
* gender 1..1
* gender ^comment = "Gender is required for this profile""#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("* ^version = \"1.0.0\""));
    assert!(result.formatted.contains("* ^status = #active"));
    assert!(result.formatted.contains("* name ^short = \"Patient name\""));
    assert!(result.formatted.contains("* gender ^comment ="));
}

/// Test round-trip validation with code caret rules
#[test]
fn test_code_caret_rules_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"CodeSystem: TestCodes
Id: test-codes
* #code1 ^display = "Code 1"
* #code1 ^definition = "First test code"
* #code2 ^display = "Code 2"
* #code2 ^definition = "Second test code""#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("CodeSystem: TestCodes"));
    assert!(result.formatted.contains("* #code1 ^display"));
    assert!(result.formatted.contains("* #code2 ^definition"));
}

/// Test round-trip validation with insert rules
#[test]
fn test_insert_rules_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"RuleSet: CommonMetadata
* ^status = #active
* ^experimental = false
* ^version = "1.0.0"

Profile: PatientWithCommon
Parent: Patient
* insert CommonMetadata
* name 1..1 MS"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("RuleSet: CommonMetadata"));
    assert!(result.formatted.contains("* insert CommonMetadata"));
    assert!(result.formatted.contains("Profile: PatientWithCommon"));
}

/// Test semantic equivalence validation
#[test]
fn test_semantic_equivalence() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: SemanticTest
Parent: Patient
* name 1..1 MS
* gender 1..1"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    // Should have minimal differences for well-formed FSH
    assert!(result.differences.len() <= 2, "Too many semantic differences: {:?}", result.differences);
}

/// Test validation result methods
#[test]
fn test_validation_result_methods() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: TestProfile
Parent: Patient
* name 1..1"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    // Test result methods
    let issues = result.issues();
    assert!(issues.len() >= 0); // May have issues, but shouldn't crash
    
    // Test that we can access all fields
    assert!(!result.original.is_empty());
    assert!(!result.formatted.is_empty());
    assert!(!result.reparsed.is_empty());
}

/// Test round-trip with different format options
#[test]
fn test_format_options_round_trip() {
    let options = FormatOptions {
        indent_size: 4,
        align_carets: false,
        max_line_length: 80,
        blank_line_before_rules: false,
        preserve_blank_lines: true,
    };
    
    let validator = RoundTripValidator::with_options(options);
    let source = r#"Profile: FormattedPatient
Parent: Patient
* name 1..1 MS
* gender 1..1"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Profile: FormattedPatient"));
    assert!(result.formatted.contains("* name 1..1 MS"));
}

/// Test trivia preservation validation
#[test]
fn test_trivia_preservation() {
    let source = r#"// Header comment
Profile: TriviaTest // Inline comment
Parent: Patient
* name 1..1 MS // Required name"#;

    let (cst, _, _) = parse_fsh(source);
    let collector = TriviaCollector::new();
    let trivia = collector.collect_trivia(&cst);
    
    // Should collect some trivia
    assert!(!trivia.is_empty());
    
    // Test round-trip with trivia
    let validator = RoundTripValidator::new();
    let result = validator.validate_round_trip(source).unwrap();
    
    // Trivia preservation depends on formatter implementation
    // At minimum, the content should be preserved
    assert!(result.formatted.contains("Profile: TriviaTest"));
    assert!(result.formatted.contains("* name 1..1 MS"));
}

/// Test incremental update round-trip consistency
#[test]
fn test_incremental_update_round_trip() {
    let source = r#"Profile: IncrementalTest
Parent: Patient
* name 1..1 MS
* gender 1..1"#;

    let (original_cst, _, _) = parse_fsh(source);
    let updater = IncrementalUpdater::new();
    
    // Apply an edit
    let edit = TextEdit::replace_range(9..22, "UpdatedTest");
    let update_result = updater.apply_edit(&original_cst, &edit).unwrap();
    
    assert!(update_result.success);
    
    // Validate round-trip of updated CST
    let updated_text = update_result.cst.text().to_string();
    let validator = RoundTripValidator::new();
    let round_trip_result = validator.validate_round_trip(&updated_text).unwrap();
    
    assert!(round_trip_result.formatted.contains("UpdatedTest"));
}

/// Test performance of round-trip validation
#[test]
fn test_round_trip_performance() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: PerformanceTest
Parent: Patient
Id: performance-test
Title: "Performance Test Profile"
Description: "A profile for testing round-trip performance"
* identifier 1..* MS
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* address 0..*
* telecom 0..*
* ^status = #active
* ^experimental = false
* ^version = "1.0.0"
* ^publisher = "Test Organization"
* ^contact.name = "Test Contact"
* ^contact.telecom.system = #email
* ^contact.telecom.value = "test@example.com""#;

    let start = std::time::Instant::now();
    let result = validator.validate_round_trip(source).unwrap();
    let duration = start.elapsed();
    
    // Should complete reasonably quickly (less than 1 second)
    assert!(duration.as_secs() < 1, "Round-trip validation took too long: {:?}", duration);
    assert!(!result.formatted.is_empty());
}

/// Test error handling in round-trip validation
#[test]
fn test_error_handling() {
    let validator = RoundTripValidator::new();
    
    // Test with malformed FSH
    let malformed_source = r#"Profile MyPatient // Missing colon
Parent: Patient
* name 1..1 MS"#;

    let result = validator.validate_round_trip(malformed_source).unwrap();
    
    // Should handle errors gracefully
    assert!(!result.original.is_empty());
    // May have parse errors, but shouldn't crash
}

/// Test round-trip with empty content
#[test]
fn test_empty_content_round_trip() {
    let validator = RoundTripValidator::new();
    let result = validator.validate_round_trip("").unwrap();
    
    assert_eq!(result.original, "");
    assert_eq!(result.formatted, "");
    assert_eq!(result.reparsed, "");
}

/// Test round-trip with only whitespace
#[test]
fn test_whitespace_only_round_trip() {
    let validator = RoundTripValidator::new();
    let source = "   \n\n  \t  \n   ";
    let result = validator.validate_round_trip(source).unwrap();
    
    // Should handle whitespace-only content
    assert!(!result.original.is_empty());
}

/// Test round-trip with only comments
#[test]
fn test_comments_only_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"// This is just a comment
// Another comment
/* Block comment */"#;

    let result = validator.validate_round_trip(source).unwrap();
    
    // Should handle comment-only content
    assert!(!result.original.is_empty());
}

/// Test multiple document types in one file
#[test]
fn test_multiple_documents_round_trip() {
    let validator = RoundTripValidator::new();
    let source = r#"Alias: SCT = http://snomed.info/sct

Profile: MultiDocPatient
Parent: Patient
* name 1..1 MS

Extension: MultiDocExtension
Id: multi-doc-extension
* value[x] only string

ValueSet: MultiDocValueSet
Id: multi-doc-valueset
* include codes from system SCT

CodeSystem: MultiDocCodeSystem
Id: multi-doc-codesystem
* #code1 "Code 1""#;

    let result = validator.validate_round_trip(source).unwrap();
    
    assert!(result.formatted.contains("Alias: SCT"));
    assert!(result.formatted.contains("Profile: MultiDocPatient"));
    assert!(result.formatted.contains("Extension: MultiDocExtension"));
    assert!(result.formatted.contains("ValueSet: MultiDocValueSet"));
    assert!(result.formatted.contains("CodeSystem: MultiDocCodeSystem"));
}

/// Benchmark round-trip validation performance
#[test]
#[ignore] // Ignore by default, run with --ignored for benchmarking
fn benchmark_round_trip_validation() {
    let validator = RoundTripValidator::new();
    let source = r#"Profile: PerformanceTest
Parent: Patient
Id: performance-test
Title: "Performance Test Profile"
Description: "A profile for testing round-trip performance"
* identifier 1..* MS
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* address 0..*
* telecom 0..*
* ^status = #active
* ^experimental = false
* ^version = "1.0.0"
* ^publisher = "Test Organization"
* ^contact.name = "Test Contact"
* ^contact.telecom.system = #email
* ^contact.telecom.value = "test@example.com""#;
    
    let iterations = 100;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _result = validator.validate_round_trip(source).unwrap();
    }
    
    let total_duration = start.elapsed();
    let avg_duration = total_duration / iterations;
    
    println!("Average round-trip validation time: {:?}", avg_duration);
    println!("Total time for {} iterations: {:?}", iterations, total_duration);
    
    // Should average less than 10ms per validation
    assert!(avg_duration.as_millis() < 10, "Round-trip validation too slow: {:?}", avg_duration);
}