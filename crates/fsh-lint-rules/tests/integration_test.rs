//! Integration tests for the rule system architecture

use fsh_lint_core::{Rule, RuleCategory, RuleEngine, RuleMetadata, Severity};
use fsh_lint_rules::DefaultRuleEngine;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_rule_system_integration() {
    // Create a test rule
    let rule = Rule {
        id: "test/correctness/integration-test-rule".to_string(),
        severity: Severity::Warning,
        description: "A test rule for integration testing".to_string(),
        gritql_pattern: "test_pattern_for_integration".to_string(),
        autofix: None,
        is_ast_rule: false,
        metadata: RuleMetadata {
            id: "test/correctness/integration-test-rule".to_string(),
            name: "Integration Test Rule".to_string(),
            description: "A test rule for integration testing".to_string(),
            severity: Severity::Warning,
            category: RuleCategory::Correctness,
            tags: vec!["test".to_string(), "integration".to_string()],
            version: Some("1.0.0".to_string()),
            docs_url: Some("https://example.com/docs/integration-test-rule".to_string()),
        },
    };

    // Test rule validation
    assert!(rule.validate().is_ok());

    // Create a rule engine
    let mut engine = DefaultRuleEngine::new();

    // Test rule compilation
    let compiled_rule = engine.compile_rule(&rule).unwrap();
    assert_eq!(compiled_rule.id(), "test/correctness/integration-test-rule");
    assert_eq!(compiled_rule.severity(), Severity::Warning);
    assert!(!compiled_rule.has_autofix());

    // Test rule validation through engine
    assert!(engine.validate_rule(&rule).is_ok());

    // Register the compiled rule
    engine.registry_mut().register(compiled_rule);

    // Verify the rule is registered
    assert!(
        engine
            .get_rule("test/correctness/integration-test-rule")
            .is_some()
    );
    assert_eq!(engine.registry().len(), 1);
}

#[test]
fn test_rule_loading_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let rule_file = temp_dir.path().join("test-rule.json");

    // Create a test rule
    let rule = Rule {
        id: "test/documentation/file-test-rule".to_string(),
        severity: Severity::Error,
        description: "A test rule loaded from file".to_string(),
        gritql_pattern: "file_test_pattern".to_string(),
        autofix: None,
        is_ast_rule: false,
        metadata: RuleMetadata {
            id: "test/documentation/file-test-rule".to_string(),
            name: "File Test Rule".to_string(),
            description: "A test rule loaded from file".to_string(),
            severity: Severity::Error,
            category: RuleCategory::Documentation,
            tags: vec!["test".to_string(), "file".to_string()],
            version: Some("1.0.0".to_string()),
            docs_url: None,
        },
    };

    // Write rule to file
    let rule_json = serde_json::to_string_pretty(&rule).unwrap();
    fs::write(&rule_file, rule_json).unwrap();

    // Load rule from file
    let mut engine = DefaultRuleEngine::new();
    engine.load_rule_file(&rule_file).unwrap();

    // Verify the rule was loaded
    let loaded_rule = engine
        .get_rule("test/documentation/file-test-rule")
        .unwrap();
    assert_eq!(loaded_rule.id(), "test/documentation/file-test-rule");
    assert_eq!(loaded_rule.severity(), Severity::Error);
}

#[test]
fn test_rule_loading_from_directory() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join("rules");
    fs::create_dir(&rules_dir).unwrap();

    // Create multiple rule files
    for i in 1..=3 {
        let rule = Rule {
            id: format!("test/style/dir-test-rule-{i}"),
            severity: Severity::Info,
            description: format!("Test rule {i} from directory"),
            gritql_pattern: format!("dir_test_pattern_{i}"),
            autofix: None,
            is_ast_rule: false,
            metadata: RuleMetadata {
                id: format!("test/style/dir-test-rule-{i}"),
                name: format!("Directory Test Rule {i}"),
                description: format!("Test rule {i} from directory"),
                severity: Severity::Info,
                category: RuleCategory::Style,
                tags: vec!["test".to_string(), "directory".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: None,
            },
        };

        let rule_file = rules_dir.join(format!("rule-{i}.json"));
        let rule_json = serde_json::to_string_pretty(&rule).unwrap();
        fs::write(&rule_file, rule_json).unwrap();
    }

    // Load rules from directory
    let mut engine = DefaultRuleEngine::new();
    engine.load_rules_from_dir(&rules_dir).unwrap();

    // Verify all rules were loaded
    assert_eq!(engine.registry().len(), 3);
    for i in 1..=3 {
        let rule_id = format!("test/style/dir-test-rule-{i}");
        assert!(engine.get_rule(&rule_id).is_some());
    }
}

#[test]
fn test_rule_validation_errors() {
    let engine = DefaultRuleEngine::new();

    // Test empty ID
    let mut invalid_rule = Rule {
        id: "".to_string(),
        severity: Severity::Warning,
        description: "Test rule".to_string(),
        gritql_pattern: "test_pattern".to_string(),
        autofix: None,
        is_ast_rule: false,
        metadata: RuleMetadata {
            id: "".to_string(),
            name: "Test Rule".to_string(),
            description: "Test rule".to_string(),
            severity: Severity::Warning,
            category: RuleCategory::Correctness,
            tags: vec![],
            version: None,
            docs_url: None,
        },
    };

    assert!(engine.validate_rule(&invalid_rule).is_err());

    // Test empty pattern
    invalid_rule.id = "test/correctness/test-rule".to_string();
    invalid_rule.metadata.id = "test/correctness/test-rule".to_string();
    invalid_rule.gritql_pattern = "".to_string();

    assert!(engine.validate_rule(&invalid_rule).is_err());

    // Test empty name
    invalid_rule.gritql_pattern = "test_pattern".to_string();
    invalid_rule.metadata.name = "".to_string();

    assert!(engine.validate_rule(&invalid_rule).is_err());
}
