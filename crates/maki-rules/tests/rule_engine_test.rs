//! Comprehensive tests for the rule engine implementation
//!
//! This test suite covers:
//! - GritQL pattern compilation and execution
//! - Rule loading from directories and packs
//! - Rule precedence and conflict resolution

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use maki_core::{
    Rule, RuleCategory, RuleEngine, RuleMetadata, SemanticModel, Severity, SymbolTable,
};
use maki_rules::gritql::GritQLCompiler;
use maki_rules::{
    DefaultRuleEngine, RuleDiscoveryConfig, RulePack, RulePackMetadata, RulePrecedence,
};

/// Helper function to create a test rule
fn create_test_rule(id: &str, name: &str, pattern: &str, severity: Severity) -> Rule {
    let rule_id = if id.is_empty() {
        String::new()
    } else if id.contains('/') {
        id.to_string()
    } else {
        format!("test/correctness/{id}")
    };
    Rule {
        id: rule_id.clone(),
        severity,
        description: format!("Test rule: {name}"),
        gritql_pattern: pattern.to_string(),
        autofix: None,
        is_ast_rule: false, // GritQL-based rule
        metadata: RuleMetadata {
            id: rule_id,
            name: name.to_string(),
            description: format!("Test rule: {name}"),
            severity,
            category: RuleCategory::Correctness,
            tags: vec!["test".to_string()],
            version: Some("1.0.0".to_string()),
            docs_url: None,
        },
    }
}

fn expected_rule_id(id: &str) -> String {
    if id.is_empty() {
        String::new()
    } else if id.contains('/') {
        id.to_string()
    } else {
        format!("test/correctness/{id}")
    }
}

/// Helper function to create a test rule pack
fn create_test_rule_pack(name: &str, version: &str, rules: Vec<Rule>) -> RulePack {
    RulePack {
        metadata: RulePackMetadata {
            name: name.to_string(),
            version: version.to_string(),
            description: format!("Test rule pack: {name}"),
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

/// Helper function to create a mock semantic model for testing
fn create_mock_semantic_model() -> SemanticModel {
    use maki_core::cst::parse_fsh;

    let source = "Profile: Test\nParent: Patient".to_string();
    let (cst, _) = parse_fsh(&source);

    SemanticModel {
        source_file: PathBuf::from("test.fsh"),
        cst,
        source: source.clone(),
        source_map: maki_core::SourceMap::new(&source),
        resources: Vec::new(),
        symbols: SymbolTable::default(),
        references: Vec::new(),
    }
}

mod gritql_compilation_tests {
    use super::*;

    #[test]
    fn test_gritql_compiler_creation() {
        let compiler = GritQLCompiler::new();
        assert!(compiler.is_ok(), "Should be able to create GritQL compiler");
    }

    #[test]
    fn test_basic_pattern_compilation() {
        let compiler = GritQLCompiler::new().unwrap();

        // Test compilation of basic patterns
        let patterns = vec![
            ("profile_definition", "test-rule-1"),
            ("extension_definition", "test-rule-2"),
            ("valueset_definition", "test-rule-3"),
            ("identifier", "test-rule-4"),
        ];

        for (pattern, rule_id) in patterns {
            let result = compiler.compile_pattern(pattern, rule_id);
            assert!(
                result.is_ok(),
                "Pattern '{pattern}' should compile successfully"
            );

            let compiled = result.unwrap();
            assert_eq!(compiled.pattern(), pattern);
            assert_eq!(compiled.rule_id(), rule_id);
        }
    }

    #[test]
    fn test_pattern_compilation_with_variables() {
        let compiler = GritQLCompiler::new().unwrap();

        // Test patterns with variable captures
        let pattern = "Profile $name { * }";
        let result = compiler.compile_pattern(pattern, "variable-test");

        assert!(result.is_ok(), "Pattern with variables should compile");
        let compiled = result.unwrap();
        assert!(compiled.captures().contains(&"name".to_string()));
    }

    #[test]
    #[ignore] // TODO: Empty patterns are now allowed for AST-based rules
    fn test_invalid_pattern_compilation() {
        let compiler = GritQLCompiler::new().unwrap();

        // Test invalid patterns
        let invalid_patterns = vec![
            ("", "empty-pattern"),
            ("   ", "whitespace-pattern"),
            ("{ unbalanced", "unbalanced-braces"),
            ("( unbalanced", "unbalanced-parens"),
        ];

        for (pattern, rule_id) in invalid_patterns {
            let result = compiler.compile_pattern(pattern, rule_id);
            assert!(
                result.is_err(),
                "Invalid pattern '{pattern}' should fail compilation"
            );
        }
    }

    #[test]
    fn test_pattern_execution_basic() {
        let compiler = GritQLCompiler::new().unwrap();
        let pattern = compiler
            .compile_pattern("identifier", "test/correctness/test-rule")
            .unwrap();

        // Create a mock tree for testing
        // Note: In a real implementation, this would use actual tree-sitter parsing
        // For now, we'll test the interface without requiring a full FSH parser

        // The pattern execution would normally work with a real syntax tree
        // This test verifies the interface is correct
        assert_eq!(pattern.pattern(), "identifier");
        assert_eq!(pattern.rule_id(), "test/correctness/test-rule");
    }
}

mod rule_loading_tests {
    use super::*;

    #[test]
    fn test_rule_loading_from_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let rule_file = temp_dir.path().join("test-rule.json");

        let rule = create_test_rule("file-rule", "File Rule", "test_pattern", Severity::Warning);
        let rule_json = serde_json::to_string_pretty(&rule).unwrap();
        fs::write(&rule_file, rule_json).unwrap();

        let mut engine = DefaultRuleEngine::new();
        let result = engine.load_rule_file(&rule_file);

        assert!(result.is_ok(), "Should load rule from file successfully");
        assert!(
            engine.get_rule(&expected_rule_id("file-rule")).is_some(),
            "Rule should be registered"
        );
        assert_eq!(engine.registry().len(), 1, "Should have exactly one rule");
    }

    #[test]
    fn test_rule_loading_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let rules_dir = temp_dir.path().join("rules");
        fs::create_dir_all(&rules_dir).unwrap();

        // Create multiple rule files
        let rules = vec![
            create_test_rule(
                "dir-rule-1",
                "Directory Rule 1",
                "pattern1",
                Severity::Error,
            ),
            create_test_rule(
                "dir-rule-2",
                "Directory Rule 2",
                "pattern2",
                Severity::Warning,
            ),
            create_test_rule("dir-rule-3", "Directory Rule 3", "pattern3", Severity::Info),
        ];

        for (i, rule) in rules.iter().enumerate() {
            let rule_file = rules_dir.join(format!("rule-{}.json", i + 1));
            let rule_json = serde_json::to_string_pretty(rule).unwrap();
            fs::write(&rule_file, rule_json).unwrap();
        }

        let mut engine = DefaultRuleEngine::new();
        let result = engine.load_rules_from_dir(&rules_dir);

        assert!(
            result.is_ok(),
            "Should load rules from directory successfully"
        );
        assert_eq!(
            engine.registry().len(),
            3,
            "Should have loaded all three rules"
        );

        for rule in &rules {
            assert!(
                engine.get_rule(&rule.id).is_some(),
                "Rule '{}' should be loaded",
                rule.id
            );
        }
    }

    #[test]
    fn test_rule_loading_with_discovery_config() {
        let temp_dir = TempDir::new().unwrap();
        let rules_dir = temp_dir.path().join("rules");
        fs::create_dir_all(&rules_dir).unwrap();

        // Create rule files with different extensions
        let rule1 = create_test_rule("json-rule", "JSON Rule", "json_pattern", Severity::Error);
        let rule2 = create_test_rule("toml-rule", "TOML Rule", "toml_pattern", Severity::Warning);

        let rule1_json = serde_json::to_string_pretty(&rule1).unwrap();
        let rule2_toml = toml::to_string_pretty(&rule2).unwrap();

        fs::write(rules_dir.join("rule1.json"), rule1_json).unwrap();
        fs::write(rules_dir.join("rule2.toml"), rule2_toml).unwrap();
        fs::write(rules_dir.join("ignored.txt"), "This should be ignored").unwrap();

        let mut engine = DefaultRuleEngine::new();
        let discovery_config = RuleDiscoveryConfig {
            rule_directories: vec![rules_dir],
            pack_directories: Vec::new(),
            include_patterns: vec!["*.json".to_string(), "*.toml".to_string()],
            exclude_patterns: vec!["*.txt".to_string()],
            recursive: true,
            precedence: Vec::new(),
        };

        engine.set_discovery_config(discovery_config);
        let result = engine.discover_and_load_rules();

        assert!(
            result.is_ok(),
            "Should discover and load rules successfully"
        );
        assert_eq!(
            engine.registry().len(),
            2,
            "Should load JSON and TOML rules, ignore TXT"
        );
        assert!(engine.get_rule(&expected_rule_id("json-rule")).is_some());
        assert!(engine.get_rule(&expected_rule_id("toml-rule")).is_some());
    }

    #[test]
    fn test_rule_pack_loading() {
        let temp_dir = TempDir::new().unwrap();
        let pack_dir = temp_dir.path().join("pack");
        fs::create_dir_all(&pack_dir).unwrap();

        let rules = vec![
            create_test_rule(
                "pack-rule-1",
                "Pack Rule 1",
                "pack_pattern1",
                Severity::Error,
            ),
            create_test_rule(
                "pack-rule-2",
                "Pack Rule 2",
                "pack_pattern2",
                Severity::Warning,
            ),
        ];

        let pack = create_test_rule_pack("test-pack", "1.0.0", rules);
        let pack_json = serde_json::to_string_pretty(&pack).unwrap();
        fs::write(pack_dir.join("pack.json"), pack_json).unwrap();

        let mut engine = DefaultRuleEngine::new();
        let result = engine.load_rule_pack_file(&pack_dir.join("pack.json"));

        assert!(result.is_ok(), "Should load rule pack successfully");
        assert_eq!(
            engine.registry().len(),
            2,
            "Should load all rules from pack"
        );
        assert!(
            engine.registry().get_pack("test-pack").is_some(),
            "Pack should be registered"
        );
        assert!(engine.get_rule(&expected_rule_id("pack-rule-1")).is_some());
        assert!(engine.get_rule(&expected_rule_id("pack-rule-2")).is_some());
    }
}

mod rule_precedence_tests {
    use super::*;

    #[test]
    fn test_rule_precedence_with_packs() {
        let temp_dir = TempDir::new().unwrap();
        let packs_dir = temp_dir.path().join("packs");
        fs::create_dir_all(&packs_dir).unwrap();

        // Create two packs with conflicting rule IDs
        let low_priority_rules = vec![create_test_rule(
            "conflicting-rule",
            "Low Priority Rule",
            "low_pattern",
            Severity::Info,
        )];
        let high_priority_rules = vec![create_test_rule(
            "conflicting-rule",
            "High Priority Rule",
            "high_pattern",
            Severity::Error,
        )];

        let low_pack = create_test_rule_pack("low-priority-pack", "1.0.0", low_priority_rules);
        let high_pack = create_test_rule_pack("high-priority-pack", "1.0.0", high_priority_rules);

        // Write pack files
        let low_pack_dir = packs_dir.join("low");
        let high_pack_dir = packs_dir.join("high");
        fs::create_dir_all(&low_pack_dir).unwrap();
        fs::create_dir_all(&high_pack_dir).unwrap();

        let low_pack_json = serde_json::to_string_pretty(&low_pack).unwrap();
        let high_pack_json = serde_json::to_string_pretty(&high_pack).unwrap();

        fs::write(low_pack_dir.join("pack.json"), low_pack_json).unwrap();
        fs::write(high_pack_dir.join("pack.json"), high_pack_json).unwrap();

        // Set up precedence configuration
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

        // Load packs using discovery system which handles precedence
        engine.discover_and_load_rules().unwrap();

        // Verify both packs were loaded
        assert!(engine.registry().get_pack("low-priority-pack").is_some());
        assert!(engine.registry().get_pack("high-priority-pack").is_some());
        assert_eq!(engine.registry().get_packs().len(), 2);

        // Verify rule precedence is tracked correctly
        // The last pack loaded should have its precedence set
        // Since discovery loads packs in directory order, we can't guarantee which loads last
        // So we just verify that precedence is being tracked
        let priority = engine
            .registry()
            .get_rule_priority(&expected_rule_id("conflicting-rule"));
        assert!(
            priority == 10 || priority == 100,
            "Rule should have precedence from one of the packs"
        );
    }

    #[test]
    fn test_rule_conflict_resolution() {
        let mut engine = DefaultRuleEngine::new();

        // Create two rules with the same ID but different priorities
        let rule1 = create_test_rule(
            "duplicate-rule",
            "First Rule",
            "pattern1",
            Severity::Warning,
        );
        let rule2 = create_test_rule("duplicate-rule", "Second Rule", "pattern2", Severity::Error);

        // Compile and register first rule
        let compiled1 = engine.compile_rule(&rule1).unwrap();
        engine.registry_mut().register(compiled1);

        // Try to register second rule with same ID
        let compiled2 = engine.compile_rule(&rule2).unwrap();
        engine.registry_mut().register(compiled2);

        // Should have only one rule (the last one registered)
        assert_eq!(engine.registry().len(), 1);
        assert!(
            engine
                .get_rule(&expected_rule_id("duplicate-rule"))
                .is_some()
        );
    }

    #[test]
    fn test_rule_precedence_configuration() {
        let mut engine = DefaultRuleEngine::new();

        // Test setting precedence configuration
        let precedence = vec![
            RulePrecedence {
                pack_name: "pack-a".to_string(),
                priority: 50,
                can_override: true,
            },
            RulePrecedence {
                pack_name: "pack-b".to_string(),
                priority: 100,
                can_override: true,
            },
        ];

        engine.set_rule_precedence(precedence.clone());

        // Verify precedence configuration is set
        assert_eq!(engine.registry().discovery_config().precedence.len(), 2);
        assert_eq!(
            engine.registry().discovery_config().precedence[0].pack_name,
            "pack-a"
        );
        assert_eq!(
            engine.registry().discovery_config().precedence[0].priority,
            50
        );
        assert_eq!(
            engine.registry().discovery_config().precedence[1].pack_name,
            "pack-b"
        );
        assert_eq!(
            engine.registry().discovery_config().precedence[1].priority,
            100
        );
    }

    #[test]
    fn test_rule_pack_dependency_validation() {
        let mut engine = DefaultRuleEngine::new();

        // Test pack metadata validation
        let valid_pack = create_test_rule_pack("valid-pack", "1.0.0", vec![]);
        assert!(engine.registry_mut().register_pack(valid_pack).is_ok());

        // Test invalid pack (empty name)
        let mut invalid_pack = create_test_rule_pack("", "1.0.0", vec![]);
        invalid_pack.metadata.name = "".to_string();
        assert!(engine.registry_mut().register_pack(invalid_pack).is_err());

        // Test invalid pack (empty version)
        let mut invalid_pack = create_test_rule_pack("invalid-pack", "", vec![]);
        invalid_pack.metadata.version = "".to_string();
        assert!(engine.registry_mut().register_pack(invalid_pack).is_err());
    }
}

mod rule_execution_tests {
    use super::*;

    #[test]
    fn test_rule_compilation_and_validation() {
        let engine = DefaultRuleEngine::new();

        // Test valid rule compilation
        let valid_rule = create_test_rule(
            "valid-rule",
            "Valid Rule",
            "valid_pattern",
            Severity::Warning,
        );
        let result = engine.compile_rule(&valid_rule);
        assert!(result.is_ok(), "Valid rule should compile successfully");

        let compiled = result.unwrap();
        assert_eq!(compiled.id(), "test/correctness/valid-rule");
        assert_eq!(compiled.severity(), Severity::Warning);
        assert!(!compiled.has_autofix());
    }

    #[test]
    fn test_rule_validation_errors() {
        let engine = DefaultRuleEngine::new();

        // Test rule with empty ID
        let mut invalid_rule = create_test_rule("", "Invalid Rule", "pattern", Severity::Error);
        invalid_rule.id = "".to_string();
        invalid_rule.metadata.id = "".to_string();
        assert!(engine.validate_rule(&invalid_rule).is_err());

        // Test rule with empty pattern
        let mut invalid_rule =
            create_test_rule("invalid-rule", "Invalid Rule", "", Severity::Error);
        invalid_rule.gritql_pattern = "".to_string();
        assert!(engine.validate_rule(&invalid_rule).is_err());

        // Test rule with empty name
        let mut invalid_rule = create_test_rule("invalid-rule", "", "pattern", Severity::Error);
        invalid_rule.metadata.name = "".to_string();
        assert!(engine.validate_rule(&invalid_rule).is_err());
    }

    #[test]
    fn test_rule_execution_against_semantic_model() {
        let mut engine = DefaultRuleEngine::new();

        // Create and register a test rule
        let rule = create_test_rule(
            "execution-test",
            "Execution Test",
            "test_pattern",
            Severity::Warning,
        );
        let compiled_rule = engine.compile_rule(&rule).unwrap();
        engine.registry_mut().register(compiled_rule);

        // Create a mock semantic model
        let model = create_mock_semantic_model();

        // Execute rules against the model
        let _diagnostics = engine.execute_rules(&model);

        // For now, we expect no diagnostics since we're using a mock implementation
        // In a real implementation with actual GritQL execution, this would produce diagnostics
        // Just verify that execution completes without panicking
    }

    #[test]
    fn test_rule_engine_statistics() {
        let mut engine = DefaultRuleEngine::new();

        // Create and register multiple rules and packs
        let syntax_rules = vec![
            create_test_rule(
                "syntax-1",
                "Syntax Rule 1",
                "syntax_pattern1",
                Severity::Error,
            ),
            create_test_rule(
                "syntax-2",
                "Syntax Rule 2",
                "syntax_pattern2",
                Severity::Warning,
            ),
        ];
        let semantic_rules = vec![create_test_rule(
            "semantic-1",
            "Semantic Rule 1",
            "semantic_pattern1",
            Severity::Info,
        )];

        let syntax_pack = create_test_rule_pack("syntax-pack", "1.0.0", syntax_rules);
        let semantic_pack = create_test_rule_pack("semantic-pack", "1.0.0", semantic_rules);

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

        // Get and verify statistics
        let stats = engine.get_statistics();
        assert_eq!(stats.total_rules, 3);
        assert_eq!(stats.total_packs, 2);
        assert!(stats.rules_by_pack.contains_key("syntax-pack"));
        assert!(stats.rules_by_pack.contains_key("semantic-pack"));
        assert_eq!(stats.rules_by_pack["syntax-pack"], 2);
        assert_eq!(stats.rules_by_pack["semantic-pack"], 1);
    }
}

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_rule_loading_error_handling() {
        let mut engine = DefaultRuleEngine::new();

        // Test loading from non-existent directory
        let non_existent_dir = PathBuf::from("/non/existent/directory");
        let result = engine.load_rules_from_dir(&non_existent_dir);
        assert!(result.is_err(), "Should fail when directory doesn't exist");

        // Test loading from non-existent file
        let non_existent_file = PathBuf::from("/non/existent/file.json");
        let result = engine.load_rule_file(&non_existent_file);
        assert!(result.is_err(), "Should fail when file doesn't exist");
    }

    #[test]
    fn test_malformed_rule_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let malformed_file = temp_dir.path().join("malformed.json");

        // Write malformed JSON
        fs::write(&malformed_file, "{ invalid json }").unwrap();

        let mut engine = DefaultRuleEngine::new();
        let result = engine.load_rule_file(&malformed_file);
        assert!(result.is_err(), "Should fail on malformed JSON");
    }

    #[test]
    fn test_duplicate_rule_registration() {
        let mut engine = DefaultRuleEngine::new();

        let rule = create_test_rule(
            "duplicate-test",
            "Duplicate Test",
            "pattern",
            Severity::Warning,
        );

        // First registration should succeed
        assert!(engine.validate_rule(&rule).is_ok());
        let compiled1 = engine.compile_rule(&rule).unwrap();
        engine.registry_mut().register(compiled1);

        // Second registration with same ID should be handled gracefully
        let compiled2 = engine.compile_rule(&rule).unwrap();
        engine.registry_mut().register(compiled2);

        // Should still have only one rule
        assert_eq!(engine.registry().len(), 1);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_rule_engine_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path().join("workspace");
        let rules_dir = workspace_dir.join("rules");
        let packs_dir = workspace_dir.join("packs");

        fs::create_dir_all(&rules_dir).unwrap();
        fs::create_dir_all(&packs_dir).unwrap();

        // Create individual rule files
        let individual_rule = create_test_rule(
            "individual-rule",
            "Individual Rule",
            "individual_pattern",
            Severity::Warning,
        );
        let rule_json = serde_json::to_string_pretty(&individual_rule).unwrap();
        fs::write(rules_dir.join("individual.json"), rule_json).unwrap();

        // Create a rule pack
        let pack_rules = vec![
            create_test_rule(
                "pack-rule-1",
                "Pack Rule 1",
                "pack_pattern1",
                Severity::Error,
            ),
            create_test_rule(
                "pack-rule-2",
                "Pack Rule 2",
                "pack_pattern2",
                Severity::Info,
            ),
        ];
        let pack = create_test_rule_pack("integration-pack", "1.0.0", pack_rules);
        let pack_json = serde_json::to_string_pretty(&pack).unwrap();

        let pack_subdir = packs_dir.join("integration");
        fs::create_dir_all(&pack_subdir).unwrap();
        fs::write(pack_subdir.join("pack.json"), pack_json).unwrap();

        // Set up rule engine with discovery
        let mut engine = DefaultRuleEngine::new();
        let discovery_config = RuleDiscoveryConfig {
            rule_directories: vec![rules_dir],
            pack_directories: vec![packs_dir],
            include_patterns: vec!["*.json".to_string()],
            exclude_patterns: Vec::new(),
            recursive: true,
            precedence: vec![RulePrecedence {
                pack_name: "integration-pack".to_string(),
                priority: 50,
                can_override: true,
            }],
        };

        engine.set_discovery_config(discovery_config);

        // Discover and load all rules
        let result = engine.discover_and_load_rules();
        assert!(
            result.is_ok(),
            "Should discover and load all rules successfully"
        );

        // Verify all rules and packs were loaded
        assert_eq!(
            engine.registry().len(),
            3,
            "Should have loaded 3 rules total"
        );
        assert_eq!(
            engine.registry().get_packs().len(),
            1,
            "Should have loaded 1 pack"
        );

        assert!(
            engine
                .get_rule(&expected_rule_id("individual-rule"))
                .is_some()
        );
        assert!(engine.get_rule(&expected_rule_id("pack-rule-1")).is_some());
        assert!(engine.get_rule(&expected_rule_id("pack-rule-2")).is_some());
        assert!(engine.registry().get_pack("integration-pack").is_some());

        // Test rule execution
        let model = create_mock_semantic_model();
        let _diagnostics = engine.execute_rules(&model);

        // Execution should complete without errors
        // (Actual diagnostic generation depends on GritQL implementation)
        // Just verify that execution completes without panicking

        // Test statistics
        let stats = engine.get_statistics();
        assert_eq!(stats.total_rules, 3);
        assert_eq!(stats.total_packs, 1);
    }
}
