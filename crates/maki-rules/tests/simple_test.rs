//! Simple test to verify builtin module works

#[test]
fn test_builtin_module_exists() {
    // Just test that we can import the module
    use maki_rules::builtin::BuiltinRules;

    // Test that we can call methods on BuiltinRules
    let correctness_rules = BuiltinRules::correctness_rules();
    assert!(!correctness_rules.is_empty());
}
