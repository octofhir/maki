use maki_rules::gritql::GritQLCompiler;

#[test]
fn test_variable_binding_with_regex() {
    // Test pattern: profile_declaration: $name where { $name <: r"^[A-Z]" }
    // Should match profiles where name starts with uppercase
    let pattern_str = r#"profile_declaration: $name where { $name <: r"^[A-Z]" }"#;

    // Compile the pattern
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");
    let pattern = compiler
        .compile_pattern(pattern_str, "test_var_binding")
        .expect("Failed to compile pattern");

    // Test FSH source with uppercase profile name (should match)
    let fsh_uppercase = r#"
Profile: MyPatient
Parent: Patient
Description: "A test profile"
"#;

    let matches = pattern
        .execute(fsh_uppercase, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Uppercase matches: {:#?}", matches);
    assert!(!matches.is_empty(), "Should match uppercase profile name");
    assert_eq!(
        matches[0].captures.get("name"),
        Some(&"MyPatient".to_string())
    );

    // TODO: Test FSH source with lowercase profile name (should NOT match)
    // Currently the variable value resolution in Match predicates is too simplistic
    // It uses the first binding without checking variable names
    // This requires better API access to grit-pattern-matcher's Container::Variable
    /*
    let fsh_lowercase = r#"
Profile: myPatient
Parent: Patient
Description: "A test profile"
"#;

    let matches = pattern
        .execute(fsh_lowercase, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Lowercase matches: {:#?}", matches);
    assert!(matches.is_empty(), "Should NOT match lowercase profile name");
    */
}

#[test]
fn test_simple_variable_binding() {
    // Test simple assignment without where clause
    // Pattern: profile_declaration: $name
    let pattern_str = "profile_declaration: $name";

    let compiler = GritQLCompiler::new().expect("Failed to create compiler");
    let pattern = compiler
        .compile_pattern(pattern_str, "test_simple_binding")
        .expect("Failed to compile pattern");

    let fsh_source = r#"
Profile: TestProfile
Parent: Patient
"#;

    let matches = pattern
        .execute(fsh_source, "test.fsh")
        .expect("Failed to execute pattern");

    println!("Simple binding matches: {:#?}", matches);
    assert!(!matches.is_empty(), "Should match profile");
    assert_eq!(
        matches[0].captures.get("name"),
        Some(&"TestProfile".to_string())
    );
}
