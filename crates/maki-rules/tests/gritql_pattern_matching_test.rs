//! Test GritQL pattern matching execution

use maki_rules::gritql::GritQLCompiler;

#[test]
fn test_profile_naming_pattern_lowercase_match() {
    use grit_util::{Ast, AstNode};
    use maki_rules::gritql::FshGritTree;

    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Match Profile nodes where the text contains "Profile:" followed by whitespace and lowercase letter
    // This uses implicit context matching on the node's text content
    let pattern = compiler
        .compile_pattern(
            r#"Profile where { contains r"Profile:\s+[a-z]" }"#,
            "test-profile-naming",
        )
        .expect("Failed to compile pattern");

    // Test FSH code with lowercase profile name (should match)
    let test_code = r#"
Profile: myPatientProfile
Parent: Patient
Description: "A test profile with lowercase name"
"#;

    // DEBUG: Print CST structure
    let tree = FshGritTree::parse(test_code);
    let root = tree.root_node();
    println!("DEBUG: Root node kind: {:?}", root.kind());
    println!("DEBUG: Root children:");
    for (i, child) in root.children().enumerate() {
        let text = child.text().unwrap_or_default();
        println!("  [{}] kind={:?}, text={:?}", i, child.kind(), text);
    }

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Found {} matches for lowercase profile:", matches.len());
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line, m.range.start_column, m.matched_text
        );
    }

    // Should find at least one match (the profile with lowercase name)
    assert!(
        !matches.is_empty(),
        "Should match profile with lowercase name"
    );
}

#[test]
fn test_profile_naming_pattern_uppercase_nomatch() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Match Profile nodes where the text contains "Profile:" followed by whitespace and lowercase letter
    // This should NOT match profiles that start with uppercase
    let pattern = compiler
        .compile_pattern(
            r#"Profile where { contains r"Profile:\s+[a-z]" }"#,
            "test-profile-naming",
        )
        .expect("Failed to compile pattern");

    // Test FSH code with uppercase profile name (should NOT match)
    let test_code = r#"
Profile: MyPatientProfile
Parent: Patient
Description: "A test profile with proper naming"
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!(
        "Found {} matches for uppercase profile (expected 0):",
        matches.len()
    );
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line, m.range.start_column, m.matched_text
        );
    }

    // Should NOT find any matches (profile starts with uppercase)
    assert_eq!(
        matches.len(),
        0,
        "Should not match profile with uppercase name"
    );
}

#[test]
fn test_extension_url_missing_pattern() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Pattern from extension-url-required.grit
    let pattern = compiler
        .compile_pattern(
            r#"Extension where { not contains "^url" }"#,
            "test-extension-url",
        )
        .expect("Failed to compile pattern");

    // Test FSH code with extension missing URL (should match)
    let test_code = r#"
Extension: PatientNickname
* value[x] only string
Description: "A patient's nickname"
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Found {} matches for extension without URL:", matches.len());
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line,
            m.range.start_column,
            m.matched_text.trim()
        );
    }

    // Should find the extension without URL
    assert!(!matches.is_empty(), "Should match extension without ^url");
}

#[test]
fn test_extension_url_present_nomatch() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    let pattern = compiler
        .compile_pattern(
            r#"Extension where { not contains "^url" }"#,
            "test-extension-url",
        )
        .expect("Failed to compile pattern");

    // Test FSH code with extension having URL (should NOT match)
    let test_code = r#"
Extension: PatientNickname
* ^url = "http://example.org/fhir/StructureDefinition/patient-nickname"
* value[x] only string
Description: "A patient's nickname"
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!(
        "Found {} matches for extension with URL (expected 0):",
        matches.len()
    );
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line, m.range.start_column, m.matched_text
        );
    }

    // Should NOT find any matches (extension has ^url)
    assert_eq!(matches.len(), 0, "Should not match extension with ^url");
}

#[test]
fn test_multiple_profiles_mixed() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Match Profile nodes where the text contains "Profile:" followed by whitespace and lowercase letter
    let pattern = compiler
        .compile_pattern(
            r#"Profile where { contains r"Profile:\s+[a-z]" }"#,
            "test-multi-profile",
        )
        .expect("Failed to compile pattern");

    // Test with multiple profiles, some matching, some not
    let test_code = r#"
Profile: myLowercaseProfile
Parent: Patient

Profile: AnotherLowercaseProfile
Parent: Patient

Profile: yetAnotherLowercase
Parent: Observation

Profile: ProperProfile
Parent: Condition
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Found {} matches for mixed profiles:", matches.len());
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line,
            m.range.start_column,
            m.matched_text.lines().next().unwrap_or("")
        );
    }

    // Should find 2 matches (myLowercaseProfile and yetAnotherLowercase)
    // Note: AnotherLowercaseProfile starts with 'A' (uppercase)
    assert!(
        matches.len() >= 2,
        "Should match at least 2 profiles with lowercase names"
    );
}

#[test]
fn test_variable_binding_syntax() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Test the new Profile: $name variable binding syntax
    let pattern = compiler
        .compile_pattern(
            r#"Profile: $name where { contains r"Profile:\s+[a-z]" }"#,
            "test-var-binding",
        )
        .expect("Failed to compile pattern");

    // Test FSH code with lowercase profile name (should match)
    let test_code = r#"
Profile: myPatientProfile
Parent: Patient
Description: "A test profile with lowercase name"
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!(
        "Found {} matches with variable binding syntax:",
        matches.len()
    );
    for m in &matches {
        println!(
            "  Match at {}:{} - '{}'",
            m.range.start_line, m.range.start_column, m.matched_text
        );
    }

    // Should find at least one match (the profile with lowercase name)
    assert!(
        !matches.is_empty(),
        "Should match profile with lowercase name using variable binding syntax"
    );
}
