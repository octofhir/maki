//! Test loading GritQL rules from directory

use fsh_lint_rules::gritql::{GritQLCompiler, GritQLRuleLoader};
use std::path::Path;

#[test]
fn test_load_gritql_rules_from_examples_directory() {
    // Load rules from examples/gritql directory
    let gritql_dir = Path::new("examples/gritql");

    // Skip test if directory doesn't exist (e.g., in CI)
    if !gritql_dir.exists() {
        eprintln!("Skipping test: examples/gritql directory not found");
        return;
    }

    let loader = GritQLRuleLoader::load_from_directories(&[gritql_dir])
        .expect("Failed to load GritQL rules from directory");

    // Should have loaded the example .grit files
    assert!(loader.len() > 0, "Should have loaded at least one rule");

    println!("Loaded {} GritQL rules:", loader.len());
    for rule in loader.all_rules() {
        println!("  - {} (from {})", rule.id(), rule.source_path().display());
    }

    // Verify specific rules were loaded
    let rule_ids: Vec<&str> = loader.all_rules().iter().map(|r| r.id()).collect();
    assert!(
        rule_ids.iter().any(|id| id.contains("profile-naming")),
        "Should have loaded profile-naming rule"
    );
    assert!(
        rule_ids.iter().any(|id| id.contains("extension-url")),
        "Should have loaded extension-url rule"
    );
}

#[test]
fn test_execute_loaded_gritql_rule() {
    // Load rules from examples/gritql directory
    let gritql_dir = Path::new("examples/gritql");

    // Skip test if directory doesn't exist
    if !gritql_dir.exists() {
        eprintln!("Skipping test: examples/gritql directory not found");
        return;
    }

    let loader = GritQLRuleLoader::load_from_directories(&[gritql_dir])
        .expect("Failed to load GritQL rules from directory");

    // Test FSH code with a profile that should match the naming rule
    let test_code = r#"
Profile: myPatientProfile
Parent: Patient
Description: "A test profile with lowercase name"
"#;

    // Execute each loaded rule against the test code
    for rule in loader.all_rules() {
        println!("Executing rule: {}", rule.id());
        let matches = rule
            .pattern()
            .execute(test_code, "test.fsh")
            .expect("Failed to execute pattern");

        println!("  Found {} matches", matches.len());
        for m in &matches {
            println!(
                "    Match at {}:{} - {}",
                m.range.start_line, m.range.start_column, m.matched_text
            );
        }
    }
}

#[test]
fn test_gritql_compiler_with_custom_pattern() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Compile a pattern that looks for profiles
    let pattern = compiler
        .compile_pattern(
            r#"Profile: $name where { $name <: r"^[a-z]" }"#,
            "test-custom-rule",
        )
        .expect("Failed to compile pattern");

    // Test against FSH code
    let test_code = r#"
Profile: myLowercaseProfile
Parent: Patient

Profile: MyUppercaseProfile
Parent: Patient
"#;

    let matches = pattern
        .execute(test_code, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Custom pattern found {} matches", matches.len());
    for m in &matches {
        println!(
            "  Match: {} at {}:{}",
            m.matched_text, m.range.start_line, m.range.start_column
        );
    }
}
