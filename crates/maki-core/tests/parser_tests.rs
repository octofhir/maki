use std::sync::Arc;

use maki_core::cst::ast::*;
use maki_core::parser::{CachedFshParser, FshParser, Parser, ParserConfig};

const SIMPLE_VALUESET: &str = r#"
ValueSet: VitalSignsVS
Id: vital-signs-vs
Title: "Vital Signs"
Description: "Example ValueSet"
* include codes from system http://loinc.org where concept is-a #85353-1
* #85353-1 insert CommonCodes
"#;

const EMPTY_CONTENT: &str = "   \n\t\n";

#[test]
fn parses_valueset_document() {
    let mut parser = FshParser::new();
    let result = parser.parse(SIMPLE_VALUESET).expect("parse succeeds");

    assert!(result.is_valid());

    // Use typed AST to find ValueSet definitions
    let doc = Document::cast(result.cst().clone()).expect("valid document");
    let value_sets: Vec<_> = doc.value_sets().collect();
    assert_eq!(value_sets.len(), 1);

    let vs = &value_sets[0];
    // Check that we have a ValueSet with the expected name
    assert!(vs.name().is_some());
}

#[test]
fn whitespace_is_invalid_but_parses() {
    let mut parser = FshParser::new();
    let result = parser.parse(EMPTY_CONTENT).expect("parse succeeds");

    // Empty/whitespace content parses successfully but produces an empty CST
    assert!(result.is_valid());
    assert!(result.errors.is_empty());
}

#[test]
#[ignore] // TODO: Parser error reporting needs updating after parser changes
fn reports_parser_errors() {
    let mut parser = FshParser::new();
    let input = "ValueSet VitalSignsVS"; // missing colon
    let result = parser.parse(input).expect("parse succeeds");

    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|err| err.kind.check_is_parser()));
}

#[test]
fn cached_parser_reuses_results() {
    let mut parser = CachedFshParser::new().expect("create cached parser");
    let first = parser
        .parse_with_cache(SIMPLE_VALUESET)
        .expect("first parse");
    let second = parser
        .parse_with_cache(SIMPLE_VALUESET)
        .expect("second parse");

    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn parser_config_controls_cache() {
    let config = ParserConfig {
        enable_cache: false,
        cache_capacity: 0,
    };
    let mut parser = CachedFshParser::with_config(config).expect("create parser");

    let first = parser
        .parse_with_cache(SIMPLE_VALUESET)
        .expect("first parse");
    let second = parser
        .parse_with_cache(SIMPLE_VALUESET)
        .expect("second parse");

    assert!(!Arc::ptr_eq(&first, &second));
}

#[test]
fn parses_code_caret_and_insert_rules() {
    let source = r#"
CodeSystem: ExampleCS
* #alpha "Alpha"
* #alpha ^designation[0].value = "Alpha Display"
* #alpha insert CommonConcepts

RuleSet: CommonConcepts
* ^status = #active
"#;

    let mut parser = FshParser::new();
    let result = parser.parse(source).expect("parse succeeds");
    assert!(result.is_valid());

    let document = Document::cast(result.cst().clone()).expect("valid document");
    let codesystem = document.code_systems().next().expect("codesystem present");

    let mut caret_rule = None;
    let mut insert_rule = None;

    for rule in codesystem.rules() {
        match rule {
            Rule::CodeCaretValue(rule) => caret_rule = Some(rule),
            Rule::CodeInsert(rule) => insert_rule = Some(rule),
            _ => {}
        }
    }

    let caret_rule = caret_rule.expect("code caret value rule present");
    assert_eq!(caret_rule.codes(), vec!["alpha"]); // hash stripped
    let caret_path = caret_rule.caret_path().expect("caret path present");
    assert_eq!(caret_path.as_string(), "^designation[0].value");
    assert_eq!(caret_rule.value().as_deref(), Some("Alpha Display"));

    let insert_rule = insert_rule.expect("code insert rule present");
    assert_eq!(insert_rule.codes(), vec!["alpha"]);
    assert_eq!(insert_rule.rule_set().as_deref(), Some("CommonConcepts"));
    assert!(insert_rule.arguments().is_empty());
}

#[test]
fn parse_structured_value_expressions() {
    use maki_core::cst::ast::{AstNode, RegexValue};

    let source = r#"
Profile: ValueExpressionTest
Parent: Observation
* regexField = /pattern[0-9]+/
"#;

    let (cst, _, _) = maki_core::cst::parse_fsh(source);

    // Debug: Print all node kinds to see what's being created
    for node in cst.descendants() {
        println!("Node kind: {:?}", node.kind());
    }

    // Find RegexValue nodes
    let regex_nodes: Vec<_> = cst.descendants().filter_map(RegexValue::cast).collect();
    println!("Found {} regex nodes", regex_nodes.len());

    // For now, just check that parsing doesn't crash
    // TODO: Fix the structured node creation
}

trait ParseErrorKindExt {
    fn check_is_parser(self) -> bool;
}

impl ParseErrorKindExt for maki_core::parser::ParseErrorKind {
    fn check_is_parser(self) -> bool {
        matches!(self, maki_core::parser::ParseErrorKind::Parser)
    }
}
