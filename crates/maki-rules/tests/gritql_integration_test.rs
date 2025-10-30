//! End-to-end integration tests for GritQL pattern execution - CST-based
//!
//! These tests verify the complete GritQL pipeline using the new CST architecture:
//! 1. Pattern compilation
//! 2. FSH parsing to CST tree structure
//! 3. Pattern execution (TODO: implement actual execution)
//!
//! NOTE: Most complex tests are disabled until GritQL pattern execution is fully implemented.
//! See executor.rs for the TODO about implementing actual pattern compilation and execution.

use grit_util::{Ast, AstNode};
use maki_rules::gritql::{FshGritTree, GritQLCompiler};

#[test]
fn test_tree_creation() {
    let source = "Profile: MyPatientProfile\nParent: Patient\nDescription: \"Test profile\"";
    let tree = FshGritTree::parse(source);

    // Verify tree was created successfully
    assert_eq!(tree.source(), source);
    let root = tree.root_node();
    assert!(root.children().count() > 0);
}

#[test]
fn test_compiler_creation() {
    let compiler = GritQLCompiler::new();
    assert!(compiler.is_ok(), "Should create compiler successfully");
}

#[test]
fn test_pattern_compilation() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");
    let pattern = compiler.compile_pattern("profile", "test-simple");

    // Pattern compilation should work even though execution is not yet implemented
    assert!(pattern.is_ok(), "Should compile simple pattern");
}

// TODO: Re-enable these tests once GritQL execution is implemented
/*
#[test]
fn test_where_clause_field_exists() {
    // Test matching profiles with/without description field
}

#[test]
fn test_where_clause_negation() {
    // Test NOT clauses in where conditions
}

#[test]
fn test_variable_binding() {
    // Test capturing values in variables
}

#[test]
fn test_builtin_functions() {
    // Test is_profile, is_extension, etc.
}
*/
