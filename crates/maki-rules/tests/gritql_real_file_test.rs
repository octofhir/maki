//! Test GritQL pattern execution on real FSH files from examples/ - CST-based
//!
//! This verifies that the GritQL integration works with actual FSH code.
//!
//! NOTE: These tests are disabled until GritQL pattern execution is fully implemented.
//! See executor.rs for the TODO about implementing actual pattern compilation and execution.

use grit_util::Ast;
use maki_rules::gritql::{FshGritTree, GritQLCompiler};
use std::fs;
use std::path::Path;

#[test]
fn test_parse_patient_profile_fsh_file() {
    let examples_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/patient-profile.fsh");

    let source = fs::read_to_string(&examples_path).expect("Failed to read example file");

    // Parse to CST tree
    let tree = FshGritTree::parse(&source);

    // Verify tree was created successfully
    assert_eq!(tree.source(), source);
}

#[test]
fn test_parse_missing_metadata_fsh_file() {
    let examples_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/missing-metadata.fsh");

    let source = fs::read_to_string(&examples_path).expect("Failed to read example file");

    let tree = FshGritTree::parse(&source);
    assert_eq!(tree.source(), source);
}

#[test]
fn test_compile_patterns() {
    let compiler = GritQLCompiler::new().expect("Failed to create compiler");

    // Test various pattern compilations
    assert!(compiler.compile_pattern("profile", "test1").is_ok());
    // TODO: Re-enable when execution is implemented
    // assert!(compiler.compile_pattern("profile where { description }", "test2").is_ok());
    // assert!(compiler.compile_pattern("extension", "test3").is_ok());
}

// TODO: Re-enable these tests once GritQL execution is fully implemented
/*
#[test]
fn test_patient_profile_patterns() {
    // Test finding profiles
    // Test checking for descriptions
    // Test variable binding
}

#[test]
fn test_missing_metadata_detection() {
    // Test finding profiles without descriptions
}

#[test]
fn test_extension_patterns() {
    // Test finding extensions
}
*/
