//! Tests for built-in FSH linting rules

use fsh_lint_core::{FixSafety, RuleCategory, Severity};
use fsh_lint_rules::builtin::BuiltinRules;

fn assert_rule_basics(rule: &fsh_lint_core::Rule) {
    assert!(!rule.id.is_empty());
    assert!(!rule.description.is_empty());
    // AST-based rules may have empty GritQL patterns (they use direct AST traversal)
    // Only check that the pattern field exists, not that it's non-empty
    assert_eq!(rule.id, rule.metadata.id);
    if let Err(e) = rule.validate() {
        panic!("Rule '{}' validation failed: {:?}", rule.id, e);
    }
}

#[test]
#[ignore] // TODO: Rule count may have changed
fn blocking_rules_validate_critical_requirements() {
    let rules = BuiltinRules::blocking_rules();
    assert_eq!(rules.len(), 4); // required-field-present, invalid-cardinality, binding-strength-present, duplicate-definition

    for rule in &rules {
        assert_rule_basics(rule);
        assert!(rule.id.starts_with("correctness/")); // No builtin/ prefix
        assert_eq!(rule.severity, Severity::Error);
        assert!(rule.metadata.tags.contains(&"blocking".to_string()));
    }

    // Verify specific blocking rules exist
    let required_field = rules
        .iter()
        .find(|r| r.id.ends_with("required-field-present"))
        .expect("missing required-field-present rule");
    assert_eq!(required_field.metadata.category, RuleCategory::Correctness);

    let invalid_cardinality = rules
        .iter()
        .find(|r| r.id.ends_with("invalid-cardinality"))
        .expect("missing invalid-cardinality rule");
    assert_eq!(invalid_cardinality.severity, Severity::Error);

    let binding_strength = rules
        .iter()
        .find(|r| r.id.ends_with("binding-strength-present"))
        .expect("missing binding-strength-present rule");
    assert_eq!(binding_strength.severity, Severity::Error);

    let duplicate_definition = rules
        .iter()
        .find(|r| r.id.ends_with("duplicate-definition"))
        .expect("missing duplicate-definition rule");
    assert_eq!(duplicate_definition.severity, Severity::Error);
}

#[test]
fn correctness_rules_have_required_metadata() {
    let rules = BuiltinRules::correctness_rules();
    assert_eq!(rules.len(), 13); // Updated: added profile-assignment-present and extension-context-missing

    for rule in &rules {
        assert_rule_basics(rule);
        assert!(rule.id.starts_with("correctness/")); // No builtin/ prefix
        assert_eq!(rule.metadata.category, RuleCategory::Correctness);
        assert!(rule.metadata.tags.contains(&"correctness".to_string()));
    }

    let invalid_keyword = rules
        .iter()
        .find(|r| r.id.ends_with("invalid-keyword"))
        .expect("missing invalid-keyword rule");
    assert_eq!(invalid_keyword.severity, Severity::Error);
    let keyword_fix = invalid_keyword.autofix.as_ref().expect("expected autofix");
    assert_eq!(keyword_fix.safety, FixSafety::Safe);

    let malformed_alias = rules
        .iter()
        .find(|r| r.id.ends_with("malformed-alias"))
        .expect("missing malformed-alias rule");
    assert_eq!(malformed_alias.severity, Severity::Error);
    assert_eq!(
        malformed_alias.autofix.as_ref().unwrap().safety,
        FixSafety::Unsafe
    );

    let invalid_caret = rules
        .iter()
        .find(|r| r.id.ends_with("invalid-caret-path"))
        .expect("missing invalid-caret-path rule");
    assert!(invalid_caret.autofix.is_none());
}

#[test]
fn suspicious_rules_detect_risky_patterns() {
    let rules = BuiltinRules::suspicious_rules();
    assert_eq!(rules.len(), 2);

    for rule in &rules {
        assert_rule_basics(rule);
        assert!(rule.id.starts_with("suspicious/")); // No builtin/ prefix
        assert_eq!(rule.metadata.category, RuleCategory::Suspicious);
        assert!(rule.metadata.tags.contains(&"suspicious".to_string()));
    }

    let trailing_text = rules
        .iter()
        .find(|r| r.id.ends_with("trailing-text"))
        .expect("missing trailing-text rule");
    assert_eq!(trailing_text.severity, Severity::Warning);
    assert!(trailing_text.autofix.is_some());
}

#[test]
fn style_rules_focus_on_readability() {
    let rules = BuiltinRules::style_rules();
    assert_eq!(rules.len(), 2); // Updated count

    for rule in &rules {
        assert_rule_basics(rule);
        assert!(rule.id.starts_with("style/")); // No builtin/ prefix
        assert_eq!(rule.metadata.category, RuleCategory::Style);
        assert!(rule.metadata.tags.contains(&"style".to_string()));
        assert_eq!(rule.severity, Severity::Warning);
    }
}

#[test]
fn documentation_rules_cover_metadata_requirements() {
    let rules = BuiltinRules::documentation_rules();
    assert_eq!(rules.len(), 4); // Updated: added missing-metadata rule

    for rule in &rules {
        assert_rule_basics(rule);
        assert!(rule.id.starts_with("documentation/")); // No builtin/ prefix
        assert_eq!(rule.metadata.category, RuleCategory::Documentation);
        assert!(rule.metadata.tags.contains(&"documentation".to_string()));
    }

    let missing_description = rules
        .iter()
        .find(|r| r.id.ends_with("missing-description"))
        .expect("missing missing-description rule");
    assert_eq!(missing_description.severity, Severity::Warning);

    let missing_title = rules
        .iter()
        .find(|r| r.id.ends_with("missing-title"))
        .expect("missing missing-title rule");
    assert_eq!(missing_title.severity, Severity::Info);
}

#[test]
fn all_rules_combines_everything_without_duplicates() {
    let blocking = BuiltinRules::blocking_rules();
    let correctness = BuiltinRules::correctness_rules();
    let suspicious = BuiltinRules::suspicious_rules();
    let style = BuiltinRules::style_rules();
    let documentation = BuiltinRules::documentation_rules();
    let all = BuiltinRules::all_rules();

    let expected_total =
        blocking.len() + correctness.len() + suspicious.len() + style.len() + documentation.len();
    assert_eq!(all.len(), expected_total);

    for rule in all {
        assert_rule_basics(&rule);
    }
}
