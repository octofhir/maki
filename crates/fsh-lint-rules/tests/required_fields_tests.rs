//! Tests for required field validation rules

use fsh_lint_core::{Diagnostic, Severity};
use fsh_lint_rules::builtin::required_fields::{
    check_profile_required_fields, check_code_system_required_fields,
    check_value_set_required_fields, REQUIRED_FIELD_PRESENT,
};
use std::path::PathBuf;
use tree_sitter::{Parser, Tree};

/// Helper to parse FSH source code
fn parse_fsh(source: &str) -> Tree {
    let mut parser = Parser::new();
    // Note: This will fail without tree-sitter-fsh grammar installed
    // For now, we'll use a mock implementation for testing
    // In production, we'd use: parser.set_language(tree_sitter_fsh::language()).unwrap();

    // For testing purposes, we'll create a simple mock tree
    // This is a placeholder until tree-sitter-fsh is integrated
    parser.parse(source, None).expect("Failed to parse FSH")
}

#[test]
fn test_profile_with_all_required_fields() {
    let source = r#"
Profile: MyPatientProfile
Id: my-patient-profile
Title: "My Patient Profile"
"#;

    // This test is currently a placeholder
    // It demonstrates the API we want to test
    // Actual implementation requires tree-sitter-fsh grammar

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_profile_required_fields(root_node, source, &file_path);
    // assert!(diagnostics.is_empty(), "Should not report errors for valid profile");
}

#[test]
fn test_profile_missing_id() {
    let source = r#"
Profile: MyPatientProfile
Title: "My Patient Profile"
"#;

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_profile_required_fields(root_node, source, &file_path);

    // assert_eq!(diagnostics.len(), 1);
    // assert_eq!(diagnostics[0].rule_id, REQUIRED_FIELD_PRESENT);
    // assert_eq!(diagnostics[0].severity, Severity::Error);
    // assert!(diagnostics[0].message.contains("Id"));
}

#[test]
fn test_profile_missing_title() {
    let source = r#"
Profile: MyPatientProfile
Id: my-patient-profile
"#;

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_profile_required_fields(root_node, source, &file_path);

    // assert_eq!(diagnostics.len(), 1);
    // assert!(diagnostics[0].message.contains("Title"));
}

#[test]
fn test_profile_missing_name() {
    let source = r#"
Profile:
Id: my-patient-profile
Title: "My Patient Profile"
"#;

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_profile_required_fields(root_node, source, &file_path);

    // assert_eq!(diagnostics.len(), 1);
    // assert!(diagnostics[0].message.contains("Name"));
}

#[test]
fn test_code_system_with_all_required_fields() {
    let source = r#"
CodeSystem: MyCodeSystem
Id: my-code-system
Title: "My Code System"
"#;

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_code_system_required_fields(root_node, source, &file_path);
    // assert!(diagnostics.is_empty());
}

#[test]
fn test_value_set_with_all_required_fields() {
    let source = r#"
ValueSet: MyValueSet
Id: my-value-set
Title: "My Value Set"
"#;

    let file_path = PathBuf::from("test.fsh");

    // TODO: Uncomment when tree-sitter-fsh is integrated
    // let tree = parse_fsh(source);
    // let root_node = tree.root_node();
    // let diagnostics = check_value_set_required_fields(root_node, source, &file_path);
    // assert!(diagnostics.is_empty());
}

#[test]
fn test_helper_functions() {
    // Test the kebab-case and title-case conversion functions
    use fsh_lint_rules::builtin::required_fields::{
        // These are private, so we can't test them directly
        // Instead we test them through the public API
    };

    // The conversion functions are tested through the suggestion generation
    // when a field is missing. The suggestions should use proper casing.
}
