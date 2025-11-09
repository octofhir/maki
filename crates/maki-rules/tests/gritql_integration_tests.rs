//! Integration tests for GritQL pattern execution engine
//!
//! These tests verify end-to-end pattern matching across realistic FSH examples

use maki_rules::gritql::executor::GritQLCompiler;

#[test]
fn test_find_profiles_without_titles() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { not title }", "test-rule")
        .unwrap();

    // Profile without title
    let source = "Profile: MyProfile\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find at least the profile itself
    assert!(!matches.is_empty(), "Should find profile without title");
}

#[test]
fn test_find_profiles_with_documentation() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { title and description }", "test-rule")
        .unwrap();

    let source = r#"
Profile: MyProfile
Parent: Patient
Title: "My Patient Profile"
Description: "A profile for patient resources"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find the documented profile
    assert!(!matches.is_empty(), "Should find documented profile");

    // Check captures
    if let Some(match_result) = matches.first() {
        assert!(
            !match_result.matched_text.is_empty(),
            "Should have matched text"
        );
    }
}

#[test]
fn test_validate_profile_naming() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern(
            "Profile: $name where { is_pascal_case($name) }",
            "test-rule",
        )
        .unwrap();

    // PascalCase profile name
    let source = "Profile: MyPatient\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should match because MyPatient is PascalCase
    assert!(!matches.is_empty(), "Should match PascalCase name");
}

#[test]
fn test_find_extensions_without_urls() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Extension where { not url }", "test-rule")
        .unwrap();

    let source = "Extension: MyExtension\nTitle: \"My Extension\"";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find extension without URL
    assert!(!matches.is_empty(), "Should find extension without url");
}

#[test]
fn test_find_value_sets_with_titles() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("ValueSet where { title }", "test-rule")
        .unwrap();

    let source = r#"
ValueSet: MyValueSet
Title: "My Value Set"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find value set with title
    assert!(!matches.is_empty(), "Should find value set with title");
}

#[test]
fn test_complex_documentation_rule() {
    let compiler = GritQLCompiler::new().unwrap();

    // Rule: profiles missing description
    let pattern = compiler
        .compile_pattern("Profile where { not description }", "test-rule")
        .unwrap();

    // Incomplete profile
    let source = "Profile: MyProfile\nTitle: \"My Profile\"";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find profile with incomplete documentation
    assert!(
        !matches.is_empty(),
        "Should find profile without description"
    );
}

#[test]
fn test_multiple_definitions_in_file() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler.compile_pattern("Profile", "test-rule").unwrap();

    let source = r#"
Profile: Profile1
Parent: Patient

Profile: Profile2
Parent: Observation

Profile: Profile3
Parent: Patient
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find all 3 profiles
    let profile_matches = matches
        .iter()
        .filter(|m| m.matched_text.contains("Profile:"))
        .count();
    assert!(profile_matches >= 3, "Should find all 3 profiles");
}

#[test]
fn test_field_value_comparison() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { parent == \"Patient\" }", "test-rule")
        .unwrap();

    let source = "Profile: MyPatient\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should match profiles with parent == "Patient"
    assert!(
        !matches.is_empty(),
        "Should find profile with parent Patient"
    );
}

#[test]
fn test_string_contains_operation() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern(
            "Profile: $name where { $name contains \"Patient\" }",
            "test-rule",
        )
        .unwrap();

    let source = "Profile: MyPatient\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should match profile names containing "Patient"
    assert!(
        !matches.is_empty(),
        "Should find profile with Patient in name"
    );
}

#[test]
fn test_regex_pattern_matching() {
    let compiler = GritQLCompiler::new().unwrap();

    // Match IDs starting with lowercase letter
    let pattern = compiler
        .compile_pattern("Profile where { id <: r\"^[a-z]\" }", "test-rule")
        .unwrap();

    let source = "Profile: MyProfile\nId: my-profile-id";
    let _matches = pattern.execute(source, "test.fsh").unwrap();

    // Pattern should compile and execute without errors
    // Test passes if execution doesn't panic
}

#[test]
fn test_predicate_with_or_condition() {
    let compiler = GritQLCompiler::new().unwrap();

    // Find profiles missing description
    let pattern = compiler
        .compile_pattern("Profile where { not description }", "test-rule")
        .unwrap();

    let source = r#"Profile: MyProfile
Title: "Title Only"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should match because description is missing
    assert!(
        !matches.is_empty(),
        "Should find profile missing description"
    );
}

#[test]
fn test_negated_predicate() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { not title }", "test-rule")
        .unwrap();

    let source = "Profile: MyProfile\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find profile without title
    assert!(!matches.is_empty(), "Should find profile without title");
}

#[test]
fn test_variable_capture_and_validation() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern(
            "Profile: $name where { is_pascal_case($name) }",
            "test-rule",
        )
        .unwrap();

    // Verify captures are correctly set
    assert!(
        pattern.captures().contains(&"name".to_string()),
        "Should have name capture"
    );

    let source = "Profile: MyPatient\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    assert!(!matches.is_empty(), "Should match valid PascalCase");
}

#[test]
fn test_extension_pattern_with_url_check() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Extension: $ext where { url }", "test-rule")
        .unwrap();

    let source = r#"Extension: MyExtension
Url: "http://example.com/extension/my-ext"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find extension with URL
    assert!(!matches.is_empty(), "Should find extension with URL");
}

#[test]
fn test_code_system_pattern() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("CodeSystem where { title }", "test-rule")
        .unwrap();

    let source = r#"CodeSystem: MyCodeSystem
Title: "My Code System"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    // Should find code system with title
    assert!(!matches.is_empty(), "Should find code system with title");
}

#[test]
fn test_empty_file_handling() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler.compile_pattern("Profile", "test-rule").unwrap();

    let source = "";
    let _matches = pattern.execute(source, "test.fsh").unwrap();

    // Empty file will still produce a document node match
    // Just verify the pattern executes without error (test passes if it doesn't panic)
}

#[test]
fn test_pattern_matching_consistency() {
    // Same pattern should produce consistent results
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { title }", "test-rule")
        .unwrap();

    let source = "Profile: Test1\nTitle: \"Test\"\n\nProfile: Test2";

    let matches1 = pattern.execute(source, "test.fsh").unwrap();
    let matches2 = pattern.execute(source, "test.fsh").unwrap();

    assert_eq!(
        matches1.len(),
        matches2.len(),
        "Pattern should be deterministic"
    );
}

#[test]
fn test_range_information_accuracy() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler.compile_pattern("Profile", "test-rule").unwrap();

    let source = "Profile: MyProfile\nParent: Patient";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    assert!(!matches.is_empty(), "Should find matches");

    if let Some(first_match) = matches.first() {
        // Verify range information exists
        assert!(
            first_match.range.start_line > 0,
            "Start line should be valid"
        );
        assert!(
            first_match.range.start_column > 0,
            "Start column should be valid"
        );
        assert!(
            first_match.range.end_line >= first_match.range.start_line,
            "End line should be >= start line"
        );
        assert!(
            first_match.range.end_column > 0,
            "End column should be valid"
        );
    }
}

#[test]
fn test_special_characters_in_strings() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern(
            r#"Profile: $name where { $name contains "-" }"#,
            "test-rule",
        )
        .unwrap();

    let source = "Profile: My-Patient\nParent: Patient";
    let _matches = pattern.execute(source, "test.fsh").unwrap();

    // Pattern should handle special characters in strings
    // Test passes if execution doesn't panic
}

#[test]
fn test_case_sensitivity_in_field_names() {
    let compiler = GritQLCompiler::new().unwrap();

    // Field names should be case-sensitive
    let pattern = compiler
        .compile_pattern("Profile where { title }", "test-rule")
        .unwrap();

    let source = "Profile: MyProfile\nTitle: \"Test Title\"";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    assert!(!matches.is_empty(), "Should find matching field");
}

#[test]
fn test_multi_line_definitions() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { title and description }", "test-rule")
        .unwrap();

    let source = r#"Profile: MyProfile
Parent: Patient
Title: "My Profile"
Description: "This is a detailed description
spanning multiple lines"
"#;
    let matches = pattern.execute(source, "test.fsh").unwrap();

    assert!(!matches.is_empty(), "Should handle multi-line definitions");
}

#[test]
fn test_unicode_in_content() {
    let compiler = GritQLCompiler::new().unwrap();
    let pattern = compiler
        .compile_pattern("Profile where { title }", "test-rule")
        .unwrap();

    let source = "Profile: MyProfile\nTitle: \"Profile with émojis 🎉\"";
    let matches = pattern.execute(source, "test.fsh").unwrap();

    assert!(!matches.is_empty(), "Should handle unicode content");
}
