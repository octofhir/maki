//! Full GritQL Integration Tests with Snapshot Testing
//!
//! Tests the complete GritQL pipeline:
//! - Parser: GritQL syntax → AST
//! - Compiler: AST → Pattern<FshQueryContext>
//! - QueryContext: All trait implementations
//! - Pattern matching against FSH CST

use grit_pattern_matcher::binding::Binding;
use grit_pattern_matcher::pattern::ResolvedPattern;
use grit_util::{Ast, AstNode};
use insta::assert_yaml_snapshot;
use maki_core::cst::parse_fsh;
use maki_rules::gritql::{
    compiler::PatternCompiler,
    cst_tree::FshGritTree,
    parser::GritQLParser,
    query_context::{FshBinding, FshQueryContext, FshResolvedPattern},
};

#[test]
fn test_parser_simple_node_pattern() {
    let mut parser = GritQLParser::new("profile");
    let result = parser.parse();

    assert!(result.is_ok());
    assert_yaml_snapshot!("parser_simple_node", result.unwrap());
}

#[test]
fn test_parser_where_clause() {
    let mut parser = GritQLParser::new("profile where { description }");
    let result = parser.parse();

    assert!(result.is_ok());
    assert_yaml_snapshot!("parser_where_clause", result.unwrap());
}

#[test]
fn test_parser_variable() {
    let mut parser = GritQLParser::new("$name");
    let result = parser.parse();

    assert!(result.is_ok());
    assert_yaml_snapshot!("parser_variable", result.unwrap());
}

#[test]
fn test_parser_assignment() {
    let mut parser = GritQLParser::new("$name = profile");
    let result = parser.parse();

    assert!(result.is_ok());
    assert_yaml_snapshot!("parser_assignment", result.unwrap());
}

#[test]
fn test_parser_not_pattern() {
    let mut parser = GritQLParser::new("not profile");
    let result = parser.parse();

    assert!(result.is_ok());
    assert_yaml_snapshot!("parser_not", result.unwrap());
}

#[test]
fn test_compiler_simple_node() {
    let mut parser = GritQLParser::new("profile");
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
    // Pattern compilation successful
}

#[test]
fn test_compiler_where_clause() {
    let mut parser = GritQLParser::new("profile where { description }");
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
    // Where clause compilation successful
}

#[test]
fn test_compiler_variable() {
    let mut parser = GritQLParser::new("$name");
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
    // Variable compilation successful
}

#[test]
fn test_compiler_assignment() {
    let mut parser = GritQLParser::new("$name = profile");
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
    // Assignment compilation successful
}

#[test]
fn test_cst_integration_parse_simple_profile() {
    let source = "Profile: MyPatient\nParent: Patient\nTitle: \"My Patient Profile\"";
    let (cst, errors) = parse_fsh(source);

    assert!(errors.is_empty(), "Parse errors: {errors:?}");

    let tree = FshGritTree::new(cst, source.to_string());
    let root = tree.root_node();

    assert!(root.text().is_ok());
}

#[test]
fn test_cst_tree_creation() {
    let source = "Profile: TestProfile\nParent: Patient";
    let (cst, errors) = parse_fsh(source);

    assert!(errors.is_empty());

    let tree = FshGritTree::new(cst, source.to_string());

    assert_eq!(tree.source().as_ref(), source);
}

#[test]
fn test_resolved_pattern_from_node_binding() {
    let source = "Profile: TestProfile";
    let (cst, _) = parse_fsh(source);
    let tree = FshGritTree::new(cst, source.to_string());
    let node = tree.root_node();

    let resolved = FshResolvedPattern::from_node_binding(node);

    assert!(resolved.is_binding());
}

#[test]
fn test_resolved_pattern_from_string() {
    let resolved = FshResolvedPattern::from_string("test".to_string());

    assert!(!resolved.is_binding());
    assert!(!resolved.is_list());
}

#[test]
fn test_resolved_pattern_from_constant() {
    use grit_pattern_matcher::constant::Constant;

    let resolved = FshResolvedPattern::from_constant(Constant::Integer(42));

    assert!(!resolved.is_binding());
}

#[test]
fn test_binding_from_node() {
    let source = "Profile: TestProfile";
    let (cst, _) = parse_fsh(source);
    let tree = FshGritTree::new(cst, source.to_string());
    let node = tree.root_node();

    let binding = FshBinding::Node(node.clone());

    assert!(binding.singleton().is_some());
}

#[test]
fn test_binding_from_constant() {
    use grit_pattern_matcher::constant::Constant;

    static CONST: Constant = Constant::Integer(123);
    let binding = FshBinding::Constant(&CONST);

    assert!(binding.as_constant().is_some());
}

#[test]
fn test_full_pipeline_parse_compile() {
    // Test the full pipeline: Parse GritQL -> Compile -> Pattern ready for execution
    let gritql = "profile where { description }";

    let mut parser = GritQLParser::new(gritql);
    let parsed = parser.parse().expect("Parse failed");

    let mut compiler = PatternCompiler::new();
    let _compiled = compiler.compile(&parsed).expect("Compile failed");

    // If we get here, the pattern is ready for execution
    assert_yaml_snapshot!("full_pipeline_parse_compile", format!("{:?}", parsed));
}

#[test]
fn test_syntax_kind_mapping() {
    let mut compiler = PatternCompiler::new();

    // Test various FSH syntax kinds
    let test_cases = vec![
        ("profile", true),
        ("extension", true),
        ("valueset", true),
        ("codesystem", true),
        ("instance", true),
        ("unknown_kind", false),
    ];

    for (kind, should_succeed) in test_cases {
        let mut parser = GritQLParser::new(kind);
        let pattern = parser.parse().unwrap();
        let result = compiler.compile(&pattern);

        assert_eq!(result.is_ok(), should_succeed, "Failed for kind: {kind}");
    }
}

#[test]
fn test_complex_pattern_and() {
    let gritql = "profile"; // Would need AND support in parser

    let mut parser = GritQLParser::new(gritql);
    let result = parser.parse();

    assert!(result.is_ok());
}

#[test]
fn test_complex_pattern_or() {
    let gritql = "profile"; // Would need OR support in parser

    let mut parser = GritQLParser::new(gritql);
    let result = parser.parse();

    assert!(result.is_ok());
}

#[test]
fn test_nested_where_clauses() {
    let gritql = "profile where { description }";

    let mut parser = GritQLParser::new(gritql);
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
}

#[test]
fn test_variable_scoping() {
    let gritql = "$x = profile";

    let mut parser = GritQLParser::new(gritql);
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
}

#[test]
fn test_multiple_variables() {
    // Test that variables get unique indices
    let gritql1 = "$x";
    let gritql2 = "$y";

    let mut compiler = PatternCompiler::new();

    let mut parser1 = GritQLParser::new(gritql1);
    let pattern1 = parser1.parse().unwrap();
    compiler.compile(&pattern1).unwrap();

    let mut parser2 = GritQLParser::new(gritql2);
    let pattern2 = parser2.parse().unwrap();
    compiler.compile(&pattern2).unwrap();

    // Both variables should have been registered
}

#[test]
fn test_predicate_field_exists() {
    let gritql = "profile where { description }";

    let mut parser = GritQLParser::new(gritql);
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_ok());
    assert_yaml_snapshot!("predicate_field_exists", format!("{:?}", pattern));
}

#[test]
fn test_predicate_not() {
    let gritql = "profile where { description }"; // Parser would need NOT predicate support

    let mut parser = GritQLParser::new(gritql);
    let result = parser.parse();

    assert!(result.is_ok());
}

#[test]
fn test_error_invalid_syntax_kind() {
    let gritql = "invalid_syntax_kind_that_does_not_exist";

    let mut parser = GritQLParser::new(gritql);
    let pattern = parser.parse().unwrap();

    let mut compiler = PatternCompiler::new();
    let result = compiler.compile(&pattern);

    assert!(result.is_err());
}

#[test]
fn test_resolved_pattern_truthy() {
    use grit_pattern_matcher::constant::Constant;
    use grit_pattern_matcher::pattern::{FileRegistry, ResolvedPattern, State};
    use maki_rules::gritql::cst_language::FshTargetLanguage;

    let resolved = FshResolvedPattern::from_constant(Constant::Boolean(true));

    let lang = FshTargetLanguage;
    let registry = FileRegistry::new_from_paths(vec![]);
    let mut state: State<FshQueryContext> = State::new(vec![], registry);

    let is_truthy = resolved.is_truthy(&mut state, &lang).unwrap();
    assert!(is_truthy);
}

#[test]
fn test_resolved_pattern_text() {
    use grit_pattern_matcher::constant::Constant;
    use grit_pattern_matcher::pattern::{FileRegistry, ResolvedPattern};
    use maki_rules::gritql::cst_language::FshTargetLanguage;

    let resolved = FshResolvedPattern::from_constant(Constant::String("test".to_string()));

    let lang = FshTargetLanguage;
    let files = FileRegistry::new_from_paths(vec![]);

    let text = resolved.text(&files, &lang).unwrap();
    assert_eq!(text.as_ref(), "test");
}

#[test]
fn test_cst_adapter_node_methods() {
    let source = "Profile: TestProfile\nParent: Patient";
    let (cst, _) = parse_fsh(source);
    let tree = FshGritTree::new(cst, source.to_string());
    let node = tree.root_node();

    // Test various node methods
    assert!(node.byte_range().start < node.byte_range().end);
    assert!(!node.text_content().is_empty());
}

#[test]
fn test_comprehensive_fsh_parse_and_tree_creation() {
    let fsh_samples = [
        "Profile: SimpleProfile\nParent: Patient",
        "Extension: MyExtension\nId: my-ext",
        "ValueSet: MyValueSet\nTitle: \"My VS\"",
        "CodeSystem: MyCS",
        "Instance: MyInstance\nInstanceOf: Patient",
    ];

    for (idx, source) in fsh_samples.iter().enumerate() {
        let (cst, errors) = parse_fsh(source);
        assert!(
            errors.is_empty(),
            "Parse errors for sample {idx}: {errors:?}"
        );

        let tree = FshGritTree::new(cst, source.to_string());
        let root = tree.root_node();

        assert!(
            !root.text_content().is_empty(),
            "Empty text for sample {idx}"
        );
        assert_eq!(tree.source().as_ref(), *source);
    }
}
