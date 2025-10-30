//! Integration tests for RuleSet expansion
//!
//! Tests the complete RuleSet expansion functionality including:
//! - Parameter substitution
//! - Bracket-aware substitution (SUSHI bug fix)
//! - Error handling
//! - Edge cases

use maki_core::semantic::ruleset::{RuleSet, RuleSetExpander, RuleSetInsert};
use std::path::PathBuf;

#[test]
fn test_simple_substitution() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "MetadataRules".to_string(),
        parameters: vec!["status".to_string()],
        rules: vec![
            "* status = {status}".to_string(),
            "* version = \"1.0.0\"".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "MetadataRules".to_string(),
        arguments: vec!["draft".to_string()],
        source_range: 50..70,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0], "* status = draft");
    assert_eq!(expanded[1], "* version = \"1.0.0\"");
}

#[test]
fn test_multiple_parameters() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "CardinalityRules".to_string(),
        parameters: vec!["path".to_string(), "min".to_string(), "max".to_string()],
        rules: vec![
            "* {path} {min}..{max}".to_string(),
            "* {path} MS".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "CardinalityRules".to_string(),
        arguments: vec!["name".to_string(), "1".to_string(), "1".to_string()],
        source_range: 50..80,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0], "* name 1..1");
    assert_eq!(expanded[1], "* name MS");
}

#[test]
fn test_bracket_aware_substitution() {
    let mut expander = RuleSetExpander::new();

    // Critical test: parameters inside [] should NOT be substituted
    let ruleset = RuleSet {
        name: "AddressRules".to_string(),
        parameters: vec!["use".to_string()],
        rules: vec![
            "* address[{use}].use = #{use}".to_string(),
            "* address[{use}].city MS".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "AddressRules".to_string(),
        arguments: vec!["home".to_string()],
        source_range: 50..70,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded.len(), 2);
    // {use} inside brackets should NOT be substituted
    assert_eq!(expanded[0], "* address[{use}].use = #home");
    assert_eq!(expanded[1], "* address[{use}].city MS");
}

#[test]
fn test_nested_brackets() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "NestedRules".to_string(),
        parameters: vec!["param".to_string()],
        rules: vec!["* extension[outer[{param}]].value[x] = {param}".to_string()],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "NestedRules".to_string(),
        arguments: vec!["test".to_string()],
        source_range: 50..70,
    };

    let expanded = expander.expand(&insert).unwrap();

    // {param} inside nested brackets should NOT be substituted
    // Only {param} outside brackets should be substituted
    assert_eq!(expanded[0], "* extension[outer[{param}]].value[x] = test");
}

#[test]
fn test_no_parameters() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "SimpleRules".to_string(),
        parameters: vec![],
        rules: vec![
            "* status = #draft".to_string(),
            "* experimental = true".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "SimpleRules".to_string(),
        arguments: vec![],
        source_range: 50..70,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded.len(), 2);
    assert_eq!(expanded[0], "* status = #draft");
    assert_eq!(expanded[1], "* experimental = true");
}

#[test]
fn test_parameter_count_mismatch_too_few() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "TwoParamRules".to_string(),
        parameters: vec!["param1".to_string(), "param2".to_string()],
        rules: vec!["* status = {param1}".to_string()],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "TwoParamRules".to_string(),
        arguments: vec!["draft".to_string()], // Only 1 arg, expected 2
        source_range: 50..70,
    };

    let result = expander.expand(&insert);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("expects 2 parameters"));
}

#[test]
fn test_parameter_count_mismatch_too_many() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "OneParamRules".to_string(),
        parameters: vec!["param1".to_string()],
        rules: vec!["* status = {param1}".to_string()],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "OneParamRules".to_string(),
        arguments: vec!["draft".to_string(), "extra".to_string()], // 2 args, expected 1
        source_range: 50..70,
    };

    let result = expander.expand(&insert);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("expects 1 parameters"));
}

#[test]
fn test_ruleset_not_found() {
    let expander = RuleSetExpander::new();

    let insert = RuleSetInsert {
        ruleset_name: "NonExistentRuleSet".to_string(),
        arguments: vec![],
        source_range: 0..20,
    };

    let result = expander.expand(&insert);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found: NonExistentRuleSet"));
}

#[test]
fn test_empty_ruleset() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "EmptyRuleSet".to_string(),
        parameters: vec![],
        rules: vec![],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..20,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "EmptyRuleSet".to_string(),
        arguments: vec![],
        source_range: 20..40,
    };

    let expanded = expander.expand(&insert).unwrap();
    assert_eq!(expanded.len(), 0);
}

#[test]
fn test_complex_substitution() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "ComplexRules".to_string(),
        parameters: vec!["url".to_string(), "system".to_string()],
        rules: vec![
            "* valueUri = \"{url}\"".to_string(),
            "* coding.system = \"{system}\"".to_string(),
            "* element[{url}].value = {url}".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..80,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "ComplexRules".to_string(),
        arguments: vec![
            "http://example.org/fhir".to_string(),
            "http://loinc.org".to_string(),
        ],
        source_range: 80..120,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded.len(), 3);
    assert_eq!(expanded[0], "* valueUri = \"http://example.org/fhir\"");
    assert_eq!(expanded[1], "* coding.system = \"http://loinc.org\"");
    // {url} inside brackets should NOT be substituted
    assert_eq!(
        expanded[2],
        "* element[{url}].value = http://example.org/fhir"
    );
}

#[test]
fn test_multiple_rulesets() {
    let mut expander = RuleSetExpander::new();

    let ruleset1 = RuleSet {
        name: "MetadataRules".to_string(),
        parameters: vec![],
        rules: vec!["* status = #draft".to_string()],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..30,
    };

    let ruleset2 = RuleSet {
        name: "CardinalityRules".to_string(),
        parameters: vec![],
        rules: vec!["* name 1..1 MS".to_string()],
        source_file: PathBuf::from("test.fsh"),
        source_range: 30..60,
    };

    expander.register_ruleset(ruleset1);
    expander.register_ruleset(ruleset2);

    let insert1 = RuleSetInsert {
        ruleset_name: "MetadataRules".to_string(),
        arguments: vec![],
        source_range: 60..80,
    };

    let insert2 = RuleSetInsert {
        ruleset_name: "CardinalityRules".to_string(),
        arguments: vec![],
        source_range: 80..100,
    };

    let expanded1 = expander.expand(&insert1).unwrap();
    let expanded2 = expander.expand(&insert2).unwrap();

    assert_eq!(expanded1[0], "* status = #draft");
    assert_eq!(expanded2[0], "* name 1..1 MS");
}

#[test]
fn test_whitespace_preservation() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "WhitespaceRules".to_string(),
        parameters: vec!["value".to_string()],
        rules: vec![
            "  * status = {value}  ".to_string(),
            "\t* version = \"1.0\"\t".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "WhitespaceRules".to_string(),
        arguments: vec!["draft".to_string()],
        source_range: 50..70,
    };

    let expanded = expander.expand(&insert).unwrap();

    // Whitespace should be preserved
    assert_eq!(expanded[0], "  * status = draft  ");
    assert_eq!(expanded[1], "\t* version = \"1.0\"\t");
}

#[test]
fn test_special_characters_in_parameters() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "SpecialChars".to_string(),
        parameters: vec!["url".to_string()],
        rules: vec![
            "* valueUri = \"{url}\"".to_string(),
            "* system = \"{url}/CodeSystem\"".to_string(),
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..50,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "SpecialChars".to_string(),
        arguments: vec!["http://example.org/fhir".to_string()],
        source_range: 50..80,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded[0], "* valueUri = \"http://example.org/fhir\"");
    assert_eq!(
        expanded[1],
        "* system = \"http://example.org/fhir/CodeSystem\""
    );
}

#[test]
fn test_mixed_bracket_types() {
    let mut expander = RuleSetExpander::new();

    let ruleset = RuleSet {
        name: "MixedBrackets".to_string(),
        parameters: vec!["type".to_string(), "value".to_string()],
        rules: vec![
            "* value[x] = {value}".to_string(),              // [x] is choice type marker
            "* extension[{type}] = {value}".to_string(),     // [{type}] should not substitute
            "* name[0].given = {value}".to_string(),         // [0] is index
            "* telecom[+].value = {value}".to_string(),      // [+] is soft indexing
        ],
        source_file: PathBuf::from("test.fsh"),
        source_range: 0..100,
    };

    expander.register_ruleset(ruleset);

    let insert = RuleSetInsert {
        ruleset_name: "MixedBrackets".to_string(),
        arguments: vec!["myType".to_string(), "testValue".to_string()],
        source_range: 100..130,
    };

    let expanded = expander.expand(&insert).unwrap();

    assert_eq!(expanded[0], "* value[x] = testValue"); // {value} outside [] substituted
    assert_eq!(expanded[1], "* extension[{type}] = testValue"); // {type} in [] NOT substituted
    assert_eq!(expanded[2], "* name[0].given = testValue"); // {value} outside [] substituted
    assert_eq!(expanded[3], "* telecom[+].value = testValue"); // {value} outside [] substituted
}
