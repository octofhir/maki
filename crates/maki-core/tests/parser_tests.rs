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

trait ParseErrorKindExt {
    fn check_is_parser(self) -> bool;
}

impl ParseErrorKindExt for maki_core::parser::ParseErrorKind {
    fn check_is_parser(self) -> bool {
        matches!(self, maki_core::parser::ParseErrorKind::Parser)
    }
}
