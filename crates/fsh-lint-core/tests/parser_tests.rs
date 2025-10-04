use std::sync::Arc;

use fsh_lint_core::parser::{CachedFshParser, FshParser, Parser, ParserConfig};

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
    let document = result.document.expect("document present");
    assert_eq!(document.value_sets.len(), 1);

    let vs = &document.value_sets[0];
    assert_eq!(vs.name.value, "VitalSignsVS");
    assert_eq!(vs.id.as_ref().unwrap().value, "vital-signs-vs");
    assert_eq!(vs.components.len(), 1);
    assert_eq!(vs.rules.len(), 1);
}

#[test]
fn whitespace_is_invalid_but_parses() {
    let mut parser = FshParser::new();
    let result = parser.parse(EMPTY_CONTENT).expect("parse succeeds");

    assert!(!result.is_valid());
    assert!(result.document.is_none());
    assert!(!result.errors.is_empty());
}

#[test]
fn reports_parser_errors() {
    let mut parser = FshParser::new();
    let input = "ValueSet VitalSignsVS"; // missing colon
    let result = parser.parse(input).expect("parse succeeds");

    assert!(!result.is_valid());
    assert!(result.errors.iter().any(|err| err.kind.is_parser()));
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
    fn is_parser(self) -> bool;
}

impl ParseErrorKindExt for fsh_lint_core::parser::ParseErrorKind {
    fn is_parser(self) -> bool {
        matches!(self, fsh_lint_core::parser::ParseErrorKind::Parser)
    }
}
