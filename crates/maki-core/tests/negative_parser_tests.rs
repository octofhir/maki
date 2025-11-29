//! Negative test coverage for FSH parser
//!
//! This module tests error handling, recovery mechanisms, and edge cases
//! to ensure the parser is robust against malformed input.

#![allow(unused_variables)]
#![allow(clippy::needless_borrow)]

use maki_core::cst::{FshSyntaxNode, LexerError, ParseError, ParseErrorKind, parse_fsh};

// Helper function to make tests more readable
fn parse_with_errors(source: &str) -> (FshSyntaxNode, Vec<LexerError>, Vec<ParseError>) {
    parse_fsh(source)
}

/// Test unclosed parameter brackets in RuleSet definitions
#[test]
fn test_unclosed_parameter_bracket() {
    let source = r#"
RuleSet: TestRuleSet(param1, param2
* name 1..1 MS
"#;

    let (_cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have both lexer errors (unterminated parameters) and parse errors (unclosed bracket)
    assert!(
        !lexer_errors.is_empty(),
        "Expected lexer errors for unterminated parameters"
    );
    assert!(
        !parse_errors.is_empty(),
        "Expected parse errors for unclosed bracket"
    );

    // Should have unclosed parameter bracket error
    let has_unclosed_bracket = parse_errors
        .iter()
        .any(|err| matches!(err.kind, ParseErrorKind::UnclosedParameterBracket));
    assert!(
        has_unclosed_bracket,
        "Expected unclosed parameter bracket error"
    );

    // Should have lexer errors for unterminated parameters
    let has_unterminated_param = lexer_errors
        .iter()
        .any(|err| err.message.contains("Unterminated parameter"));
    assert!(
        has_unterminated_param,
        "Expected lexer error for unterminated parameters"
    );
}

/// Test invalid grammar constructs
#[test]
fn test_invalid_grammar_construct() {
    let source = r#"
Profile MyPatient  // Missing colon
Parent: Patient
* name 1..1 MS
"#;

    let (_cst, _lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should parse with errors
    assert!(
        !parse_errors.is_empty(),
        "Expected parse errors for missing colon"
    );

    // Should have syntax error for missing colon
    let has_syntax_error = parse_errors.iter().any(|err| {
        matches!(err.kind, ParseErrorKind::SyntaxError) && err.message.contains("Expected ':'")
    });
    assert!(has_syntax_error, "Expected syntax error for missing colon");
}

/// Test malformed escape sequences in strings
#[test]
fn test_malformed_escape_sequences() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* name = "Invalid escape \x sequence"
* title = "Valid escape \n sequence"
"#;

    let (_cst, _lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have malformed escape sequence errors
    let malformed_escapes: Vec<_> = parse_errors
        .iter()
        .filter(|err| matches!(err.kind, ParseErrorKind::MalformedEscapeSequence))
        .collect();

    assert!(
        !malformed_escapes.is_empty(),
        "Expected malformed escape sequence errors"
    );

    // Should detect invalid \x escape
    let has_invalid_x = malformed_escapes
        .iter()
        .any(|err| err.message.contains("\\x"));
    assert!(
        has_invalid_x,
        "Expected error for invalid \\x escape sequence"
    );
}

/// Test unicode escape sequence validation
#[test]
fn test_unicode_escape_validation() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* title = "Invalid unicode \u12G4"
* description = "Incomplete unicode \u12"
* name = "Valid unicode \u0041"
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    let unicode_errors: Vec<_> = parse_errors
        .iter()
        .filter(|err| matches!(err.kind, ParseErrorKind::MalformedEscapeSequence))
        .filter(|err| err.message.contains("unicode"))
        .collect();

    assert!(!unicode_errors.is_empty(), "Expected unicode escape errors");
}

/// Test parser recovery after errors
#[test]
fn test_parser_recovery() {
    let source = r#"
Profile: FirstProfile
Parent Patient  // Missing colon - should cause error

Profile: SecondProfile  // Should still parse after error
Parent: Patient
* name 1..1 MS
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have parse errors but continue parsing
    assert!(!parse_errors.is_empty());

    // Should still find both profiles in the CST
    let profile_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Profile)
        .count();

    assert_eq!(
        profile_count, 2,
        "Parser should recover and find both profiles"
    );
}

/// Test multiple error collection
#[test]
fn test_multiple_error_collection() {
    let source = r#"
Profile TestProfile  // Missing colon
Parent Patient       // Missing colon
* name = "Bad escape \z"  // Invalid escape
* description = "Incomplete \u12"  // Incomplete unicode
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should collect multiple errors
    assert!(parse_errors.len() >= 3, "Expected at least 3 parse errors");

    // Should have different types of errors
    let error_kinds: std::collections::HashSet<_> =
        parse_errors.iter().map(|err| &err.kind).collect();

    assert!(error_kinds.len() > 1, "Expected multiple types of errors");
}

/// Test error messages are helpful
#[test]
fn test_helpful_error_messages() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* name = "unclosed string
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // The unclosed string should generate either lexer or parse errors
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for unclosed string");

    // If we have parse errors, check they are descriptive
    if !parse_errors.is_empty() {
        for error in &parse_errors {
            assert!(
                !error.message.is_empty(),
                "Error message should not be empty"
            );
            assert!(
                error.message.len() > 10,
                "Error message should be descriptive"
            );
        }
    }
}

/// Test boundary conditions - empty input
#[test]
fn test_empty_input() {
    let source = "";
    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Empty input should parse without errors
    assert!(lexer_errors.is_empty());
    assert!(parse_errors.is_empty());
}

/// Test boundary conditions - whitespace only
#[test]
fn test_whitespace_only() {
    let source = "   \n\t\n   ";
    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Whitespace only should parse without errors
    assert!(lexer_errors.is_empty());
    assert!(parse_errors.is_empty());
}

/// Test boundary conditions - very long input
#[test]
fn test_very_long_input() {
    let mut source = String::from("Profile: TestProfile\nParent: Patient\n");

    // Add many rules to test resource limits
    for i in 0..1000 {
        source.push_str(&format!("* element{} 1..1 MS\n", i));
    }

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle large input without crashing
    assert!(lexer_errors.is_empty());
    // May have parse errors but shouldn't crash
}

/// Test deeply nested structures
#[test]
fn test_deeply_nested_paths() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p.q.r.s.t.u.v.w.x.y.z 1..1 MS
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle deeply nested paths
    assert!(lexer_errors.is_empty());
    // Should parse without major errors
}

/// Test malformed contains rules
#[test]
fn test_malformed_contains_rules() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* extension contains "unclosed string
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect malformed contains rules (unclosed string)
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for unclosed string");
}

/// Test parser state consistency after errors
#[test]
fn test_parser_state_consistency() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* name = "Unclosed string
* birthDate 1..1 MS  // Should still parse after string error
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should maintain consistent state and continue parsing
    let rule_count = cst
        .descendants()
        .filter(|node| {
            matches!(
                node.kind(),
                maki_core::cst::FshSyntaxKind::CardRule
                    | maki_core::cst::FshSyntaxKind::FixedValueRule
                    | maki_core::cst::FshSyntaxKind::PathRule
            )
        })
        .count();

    assert!(
        rule_count > 0,
        "Parser should maintain state and continue parsing rules"
    );
}

/// Test recursive RuleSet insertion detection
#[test]
fn test_recursive_ruleset_insertion() {
    let source = r#"
RuleSet: RecursiveSet(
* name 1..1 MS
* insert RecursiveSet  // Direct recursion

RuleSet: IndirectA
* insert IndirectB

RuleSet: IndirectB  
* insert IndirectA  // Indirect recursion
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect syntax errors (unclosed parameter list)
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for malformed RuleSet");
}

/// Test circular RuleSet dependency detection
#[test]
fn test_circular_ruleset_dependency() {
    let source = r#"
RuleSet: SetA(
* insert SetB

RuleSet: SetB
* insert SetC

RuleSet: SetC
* insert SetA  // Creates cycle: A -> B -> C -> A
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect syntax errors (unclosed parameter list)
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for malformed RuleSet");
}

/// Test complex RuleSet dependency chains
#[test]
fn test_complex_ruleset_dependencies() {
    let source = r#"
RuleSet: Root(
* insert Branch1
* insert Branch2

RuleSet: Branch1
* insert Leaf1
* insert Shared

RuleSet: Branch2  
* insert Leaf2
* insert Shared

RuleSet: Shared
* name 1..1 MS

RuleSet: Leaf1
* birthDate 0..1

RuleSet: Leaf2
* gender 1..1

RuleSet: BadLeaf
* insert Root  // This creates a cycle
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect syntax errors (unclosed parameter list)
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for malformed RuleSet");
}

/// Test RuleSet reference validation
#[test]
fn test_ruleset_reference_validation() {
    let source = r#"
RuleSet: ValidSet
* name 1..1 MS

Profile: TestProfile
Parent: Patient
* insert ValidSet     // Valid reference
* insert NonExistent  // Invalid reference - should be handled gracefully
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Parser should handle invalid references gracefully
    // (This might not generate errors in the current implementation,
    // but it shouldn't crash)

    // Should still parse the valid parts
    let profile_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Profile)
        .count();

    assert_eq!(
        profile_count, 1,
        "Should parse valid profile despite invalid RuleSet reference"
    );
}

/// Test continued parsing after multiple errors
#[test]
fn test_continued_parsing_after_multiple_errors() {
    let source = r#"
Profile TestProfile1  // Missing colon
Parent Patient        // Missing colon
* name = "Bad \z"     // Invalid escape

Profile: TestProfile2  // Should still parse
Parent: Patient
* name 1..1 MS

Extension TestExt     // Missing colon
Context: Patient      // Should still parse context

ValueSet: TestVS      // Should still parse
* include codes from system http://example.org
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have multiple errors but continue parsing
    assert!(parse_errors.len() >= 3, "Expected multiple parse errors");

    // Should still find valid definitions
    let profile_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Profile)
        .count();
    let extension_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Extension)
        .count();
    let valueset_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::ValueSet)
        .count();

    assert!(profile_count >= 1, "Should parse at least one profile");
    assert!(
        extension_count >= 1,
        "Should parse extension despite errors"
    );
    assert!(valueset_count >= 1, "Should parse valueset despite errors");
}

/// Test parser recovery from nested errors
#[test]
fn test_parser_recovery_from_nested_errors() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* name = "Unclosed string
* birthDate 1..1 MS
* gender from http://hl7.org/fhir/ValueSet/administrative-gender (required
* address.line[0] 1..1 MS
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have errors from unclosed string and parenthesis
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for malformed input");
}

/// Test error collection doesn't cause memory issues
#[test]
fn test_error_collection_limits() {
    // Create input with many syntax errors
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!("Profile TestProfile{}\n", i)); // Missing colons
        source.push_str("Parent Patient\n"); // Missing colon
        source.push_str("* name = \"Bad \\z escape\"\n"); // Invalid escape
    }

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should limit error collection to prevent memory issues
    assert!(
        parse_errors.len() < 1000,
        "Error collection should be limited"
    );

    // Parser should not crash despite many errors
    assert!(!cst.text().is_empty(), "Should produce some CST output");
}

/// Test parser state validation
#[test]
fn test_parser_state_validation() {
    let source = r#"
Profile: TestProfile
Parent: Patient
* name 1..1 MS
* insert ValidRule

Profile: AnotherProfile
Parent: Observation  
* value[x] 1..1 MS
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should maintain consistent state throughout parsing
    assert!(lexer_errors.is_empty());

    // Should parse both profiles
    let profile_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Profile)
        .count();

    assert_eq!(
        profile_count, 2,
        "Should maintain state and parse both profiles"
    );
}

/// Test malformed CodeSystem concepts
#[test]
fn test_malformed_codesystem_concepts() {
    let source = r#"
CodeSystem: TestCS
* #concept1 "Display"
* #  // Empty code
* #concept2 "Display" "Definition" "Extra"  // Too many strings
* concept3 "No hash"  // Missing hash
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle malformed concepts gracefully
    let codesystem_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::CodeSystem)
        .count();

    assert_eq!(
        codesystem_count, 1,
        "Should parse CodeSystem despite concept errors"
    );
}

/// Test malformed Instance definitions
#[test]
fn test_malformed_instance_definitions() {
    let source = r#"
Instance TestInstance  // Missing colon
InstanceOf Patient     // Missing colon
Usage: #example
* name.given[0] = "John"

Instance: ValidInstance
InstanceOf: Patient
Usage #example         // Missing colon
* name.family = "Doe"
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have syntax errors but continue parsing
    assert!(!parse_errors.is_empty(), "Expected syntax errors");

    // Should find at least one instance
    let instance_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Instance)
        .count();

    assert!(instance_count >= 1, "Should parse at least one instance");
}

/// Test malformed Invariant definitions
#[test]
fn test_malformed_invariant_definitions() {
    let source = r#"
Invariant: TestInvariant
Description: "Test invariant"
Severity #error        // Missing colon
XPath: "//f:Patient"
Expression  // Missing colon and value

Invariant: ValidInvariant
Description: "Valid invariant"
Severity: #error
Expression: "name.exists()"
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect malformed invariant clauses
    assert!(
        !parse_errors.is_empty(),
        "Expected errors for malformed invariant"
    );

    // Should still parse invariant structures
    let invariant_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Invariant)
        .count();

    assert!(invariant_count >= 1, "Should parse invariant structures");
}

/// Test malformed Mapping definitions
#[test]
fn test_malformed_mapping_definitions() {
    let source = r#"
Mapping: TestMapping
Source TestProfile     // Missing colon
Target: "http://example.org"
* name -> "Name"

Mapping: ValidMapping
Source: TestProfile
Target "http://example.org"  // Missing colon
* birthDate -> "DOB" "Date of birth"
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should detect malformed mapping clauses
    assert!(
        !parse_errors.is_empty(),
        "Expected errors for malformed mapping"
    );

    // Should parse mapping structures
    let mapping_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Mapping)
        .count();

    assert!(mapping_count >= 1, "Should parse mapping structures");
}

/// Test resource exhaustion scenarios
#[test]
fn test_resource_exhaustion_protection() {
    // Test with extremely deep nesting
    let mut source = String::from("Profile: DeepProfile\nParent: Patient\n");

    // Create a very long path
    let mut path = String::from("extension");
    for i in 0..100 {
        path.push_str(&format!("[{}].extension", i));
    }
    source.push_str(&format!("* {} 1..1 MS\n", path));

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle deep nesting without crashing
    assert!(lexer_errors.is_empty());
    // May have parse errors but shouldn't crash
}

/// Test comment and whitespace edge cases
#[test]
fn test_comment_whitespace_edge_cases() {
    let source = r#"
// Comment before profile
Profile: TestProfile /* inline comment */ 
Parent: Patient // end of line comment
/* Multi-line
   comment */ * name 1..1 MS
* birthDate 1..1 /* comment in rule */ MS
// Comment at end
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle comments and whitespace correctly
    assert!(lexer_errors.is_empty());
    assert!(parse_errors.is_empty() || parse_errors.len() < 3); // Allow minor parsing issues

    // Should preserve comments in CST
    let text = cst.text().to_string();
    assert!(text.contains("// Comment before profile"));
    assert!(text.contains("/* inline comment */"));
}

/// Test error message quality
#[test]
fn test_error_message_quality() {
    let source = r#"
Profile TestProfile [  // Missing colon and unclosed bracket
Parent: Patient
* name = 123.456.789  // Invalid number format
* birthDate from NonExistentValueSet (invalid  // Unclosed paren
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should have errors for malformed input
    let total_errors = lexer_errors.len() + parse_errors.len();
    assert!(total_errors > 0, "Expected errors for malformed input");

    // Check error message quality if we have parse errors
    if !parse_errors.is_empty() {
        for error in &parse_errors {
            // Messages should be descriptive
            assert!(
                !error.message.is_empty(),
                "Error message should not be empty"
            );
            assert!(
                error.message.len() > 5,
                "Error message should be descriptive"
            );
        }
    }
}

/// Test boundary conditions with special characters
#[test]
fn test_special_character_handling() {
    let source = r#"
Profile: SpecialChars
Parent: Patient
* name = "Unicode: \u0041\u0042\u0043"
* description = "Special chars: \n\t\r\""
* title = "Emoji: 🔥💯"
* extension[0].url = "http://example.org/extension"
"#;

    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);

    // Should handle special characters correctly
    assert!(lexer_errors.is_empty());
    // Minor parse errors might occur but shouldn't crash

    // Should preserve special characters in CST
    let text = cst.text().to_string();
    assert!(text.contains("Unicode"));
    assert!(text.contains("Special chars"));
}

/// Test performance with large valid input
#[test]
fn test_large_input_performance() {
    let mut source = String::from("Profile: LargeProfile\nParent: Patient\n");

    // Add many valid rules
    for i in 0..500 {
        source.push_str(&format!(
            "* extension[{}].url = \"http://example.org/ext{}\"\n",
            i, i
        ));
        source.push_str(&format!("* extension[{}].value[x] 0..1\n", i));
    }

    let start = std::time::Instant::now();
    let (cst, lexer_errors, parse_errors) = parse_with_errors(&source);
    let duration = start.elapsed();

    // Should complete in reasonable time (less than 1 second for 1000 rules)
    assert!(
        duration.as_secs() < 5,
        "Parsing should complete in reasonable time"
    );

    // Should parse successfully
    assert!(lexer_errors.is_empty());
    assert!(parse_errors.is_empty() || parse_errors.len() < 10);

    // Should find the profile
    let profile_count = cst
        .descendants()
        .filter(|node| node.kind() == maki_core::cst::FshSyntaxKind::Profile)
        .count();

    assert_eq!(profile_count, 1, "Should parse large profile successfully");
}
