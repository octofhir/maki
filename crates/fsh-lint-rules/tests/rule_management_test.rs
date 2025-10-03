//! Integration tests for rule loading and management system

use fsh_lint_core::{Rule, RuleCategory, RuleEngine, RuleMetadata, Severity};
use fsh_lint_rules::{
    DefaultRuleEngine, RuleDiscoveryConfig, RulePack, RulePackMetadata, RulePrecedence,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test rule for testing
fn create_test_rule(id: &str, name: &str) -> Rule {
    let rule_id = format!("test/correctness/{}", id);
    Rule {
        id: rule_id.clone(),
        severity: Severity::Warning,
        description: format!("Test rule: {}", name),
        gritql_pattern: format!("test_pattern_{}", id),
        autofix: None,
        metadata: RuleMetadata {
            id: rule_id,
            name: name.to_string(),
            description: format!("Test rule: {}", name),
            severity: Severity::Warning,
            category: RuleCategory::Correctness,
            tags: vec!["test".to_string()],
            version: Some("1.0.0".to_string()),
            docs_url: None,
        },
    }
}

/// Create a test rule pack
fn create_test_rule_pack(name: &str, rules: Vec<Rule>) -> RulePack {
    RulePack {
        metadata: RulePackMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("Test rule pack: {}", name),
            author: Some("Test Author".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
            min_linter_version: None,
            tags: vec!["test".to_string()],
        },
        rules,
        dependencies: Vec::new(),
    }
}

#[test]
fn test_rule_discovery_from_directories() {
    let temp_dir = TempDir::new().unwrap();
    let rules_dir = temp_dir.path().join("rules");
    fs::create_dir_all(&rules_dir).unwrap();

    // Create some test rule files
    let rule1 = create_test_rule("syntax-rule-1", "Syntax Rule 1");
    let rule2 = create_test_rule("semantic-rule-1", "Semantic Rule 1");

    let rule1_json = serde_json::to_string_pretty(&rule1).unwrap();
    let rule2_json = serde_json::to_string_pretty(&rule2).unwrap();

    fs::write(rules_dir.join("syntax-rule-1.json"), rule1_json).unwrap();
    fs::write(rules_dir.join("semantic-rule-1.json"), rule2_json).unwrap();

    // Create a rule engine with discovery configuration
    let mut engine = DefaultRuleEngine::new();
    let discovery_config = RuleDiscoveryConfig {
        rule_directories: vec![rules_dir],
        pack_directories: Vec::new(),
        include_patterns: vec!["*.json".to_string()],
        exclude_patterns: Vec::new(),
        recursive: true,
        precedence: Vec::new(),
    };

    engine.set_discovery_config(discovery_config);

    // Discover and load rules
    engine.discover_and_load_rules().unwrap();

    // Verify rules were loaded
    assert!(
        engine
            .registry()
            .get("test/correctness/syntax-rule-1")
            .is_some()
    );
    assert!(
        engine
            .registry()
            .get("test/correctness/semantic-rule-1")
            .is_some()
    );
    assert_eq!(engine.registry().len(), 2);
}

#[test]
fn test_rule_pack_loading_and_precedence() {
    let temp_dir = TempDir::new().unwrap();
    let packs_dir = temp_dir.path().join("packs");
    fs::create_dir_all(&packs_dir).unwrap();

    // Create two rule packs with conflicting rule IDs
    let low_priority_rules = vec![create_test_rule("conflicting-rule", "Low Priority Rule")];
    let high_priority_rules = vec![create_test_rule("conflicting-rule", "High Priority Rule")];

    let low_priority_pack = create_test_rule_pack("low-priority-pack", low_priority_rules);
    let high_priority_pack = create_test_rule_pack("high-priority-pack", high_priority_rules);

    // Write pack files
    let low_pack_json = serde_json::to_string_pretty(&low_priority_pack).unwrap();
    let high_pack_json = serde_json::to_string_pretty(&high_priority_pack).unwrap();

    let low_pack_dir = packs_dir.join("low-priority");
    let high_pack_dir = packs_dir.join("high-priority");
    fs::create_dir_all(&low_pack_dir).unwrap();
    fs::create_dir_all(&high_pack_dir).unwrap();

    fs::write(low_pack_dir.join("pack.json"), low_pack_json).unwrap();
    fs::write(high_pack_dir.join("pack.json"), high_pack_json).unwrap();

    // Create engine with precedence configuration
    let mut engine = DefaultRuleEngine::new();
    let precedence = vec![
        RulePrecedence {
            pack_name: "low-priority-pack".to_string(),
            priority: 10,
            can_override: false,
        },
        RulePrecedence {
            pack_name: "high-priority-pack".to_string(),
            priority: 100,
            can_override: true,
        },
    ];

    let discovery_config = RuleDiscoveryConfig {
        rule_directories: Vec::new(),
        pack_directories: vec![packs_dir],
        include_patterns: vec!["*.json".to_string()],
        exclude_patterns: Vec::new(),
        recursive: true,
        precedence,
    };

    engine.set_discovery_config(discovery_config);

    // Load rule packs
    engine.discover_and_load_rules().unwrap();

    // Verify both packs were loaded
    assert!(engine.registry().get_pack("low-priority-pack").is_some());
    assert!(engine.registry().get_pack("high-priority-pack").is_some());
    assert_eq!(engine.registry().get_packs().len(), 2);

    // Verify rule precedence (high priority should win)
    assert_eq!(
        engine
            .registry()
            .get_rule_priority("test/correctness/conflicting-rule"),
        100
    );
}

#[test]
fn test_rule_engine_statistics() {
    let mut engine = DefaultRuleEngine::new();

    // Create and register some rule packs
    let syntax_rules = vec![
        create_test_rule("syntax-1", "Syntax Rule 1"),
        create_test_rule("syntax-2", "Syntax Rule 2"),
    ];
    let semantic_rules = vec![create_test_rule("semantic-1", "Semantic Rule 1")];

    let syntax_pack = create_test_rule_pack("syntax-pack", syntax_rules);
    let semantic_pack = create_test_rule_pack("semantic-pack", semantic_rules);

    // Register packs and compile rules
    engine
        .registry_mut()
        .register_pack(syntax_pack.clone())
        .unwrap();
    engine
        .registry_mut()
        .register_pack(semantic_pack.clone())
        .unwrap();

    for rule in &syntax_pack.rules {
        let compiled_rule = engine.compile_rule(rule).unwrap();
        engine.registry_mut().register(compiled_rule);
    }

    for rule in &semantic_pack.rules {
        let compiled_rule = engine.compile_rule(rule).unwrap();
        engine.registry_mut().register(compiled_rule);
    }

    // Get statistics
    let stats = engine.get_statistics();

    assert_eq!(stats.total_rules, 3);
    assert_eq!(stats.total_packs, 2);
    assert!(stats.rules_by_pack.contains_key("syntax-pack"));
    assert!(stats.rules_by_pack.contains_key("semantic-pack"));
    assert_eq!(stats.rules_by_pack["syntax-pack"], 2);
    assert_eq!(stats.rules_by_pack["semantic-pack"], 1);
}

#[test]
fn test_rule_directory_management() {
    let mut engine = DefaultRuleEngine::new();

    // Test adding rule directories
    engine.add_rule_directory(PathBuf::from("/rules1"));
    engine.add_rule_directory(PathBuf::from("/rules2"));

    assert_eq!(
        engine.registry().discovery_config().rule_directories.len(),
        2
    );
    assert!(
        engine
            .registry()
            .discovery_config()
            .rule_directories
            .contains(&PathBuf::from("/rules1"))
    );
    assert!(
        engine
            .registry()
            .discovery_config()
            .rule_directories
            .contains(&PathBuf::from("/rules2"))
    );

    // Test adding pack directories
    engine.add_pack_directory(PathBuf::from("/packs1"));
    engine.add_pack_directory(PathBuf::from("/packs2"));

    assert_eq!(
        engine.registry().discovery_config().pack_directories.len(),
        2
    );
    assert!(
        engine
            .registry()
            .discovery_config()
            .pack_directories
            .contains(&PathBuf::from("/packs1"))
    );
    assert!(
        engine
            .registry()
            .discovery_config()
            .pack_directories
            .contains(&PathBuf::from("/packs2"))
    );
}

#[test]
fn test_rule_pack_metadata_validation() {
    let mut engine = DefaultRuleEngine::new();

    // Test valid pack
    let valid_pack = create_test_rule_pack("valid-pack", vec![]);
    assert!(engine.registry_mut().register_pack(valid_pack).is_ok());

    // Test invalid pack (empty name)
    let mut invalid_pack = create_test_rule_pack("", vec![]);
    invalid_pack.metadata.name = "".to_string();
    assert!(engine.registry_mut().register_pack(invalid_pack).is_err());

    // Test invalid pack (empty version)
    let mut invalid_pack = create_test_rule_pack("invalid-pack", vec![]);
    invalid_pack.metadata.version = "".to_string();
    assert!(engine.registry_mut().register_pack(invalid_pack).is_err());
}
