//! Demonstration of rich diagnostic formatting
//! Run with: cargo test --test formatter_demo -- --nocapture

use maki_core::formatter::RichDiagnosticFormatter;
use maki_core::{
    Applicability, CodeSuggestion, Diagnostic, DiagnosticCategory, Location, Severity,
};
use std::path::PathBuf;

#[test]
fn demo_rich_diagnostic_output() {
    // Create a sample FSH source
    let source = r#"// Invalid cardinality example
Profile: ProblematicPatient
Parent: Patient
Id: problematic-patient
Title: "Problematic Patient Profile"

// ERROR: Upper bound cannot be less than lower bound
* identifier 1..0

// WARNING: Redundant cardinality
* gender 0..1"#;

    // Create a diagnostic for invalid cardinality
    let diagnostic = Diagnostic {
        rule_id: "invalid-cardinality".to_string(),
        severity: Severity::Error,
        message: "upper bound (0) cannot be less than lower bound (1)".to_string(),
        location: Location {
            file: PathBuf::from("examples/invalid-cardinality.fsh"),
            line: 8,
            column: 14,
            end_line: Some(8),
            end_column: Some(18),
            offset: 150,
            length: 4,
            span: Some((150, 154)),
        },
        suggestions: vec![CodeSuggestion {
            message: "swap to 0..1".to_string(),
            replacement: "0..1".to_string(),
            location: Location {
                file: PathBuf::from("examples/invalid-cardinality.fsh"),
                line: 8,
                column: 14,
                end_line: Some(8),
                end_column: Some(18),
                offset: 150,
                length: 4,
                span: Some((150, 154)),
            },
            applicability: Applicability::MaybeIncorrect, // Unsafe - semantic change
            labels: vec![],
        }],
        code_snippet: Some("* identifier 1..0".to_string()),
        code: Some("FSH001".to_string()),
        source: Some("builtin".to_string()),
        category: Some(DiagnosticCategory::Correctness),
    };

    // Format with rich formatter
    let formatter = RichDiagnosticFormatter::new();
    let output = formatter.format_diagnostic(&diagnostic, source);

    println!("\n{}", "=".repeat(80));
    println!("Rich Diagnostic Output Demo");
    println!("{}", "=".repeat(80));
    println!("{output}");
    println!("{}", "=".repeat(80));

    // Verify output contains expected elements
    assert!(output.contains("error[FSH001]"));
    assert!(output.contains("invalid-cardinality.fsh:8:14"));
    assert!(output.contains("* identifier 1..0"));
    assert!(output.contains("^^^^"));
    assert!(output.contains("suggestion"));
    assert!(output.contains("0..1"));
}

#[test]
fn demo_warning_diagnostic() {
    let source = r#"// Warning example: Missing recommended metadata
Profile: IncompletePatient
Parent: Patient

* identifier 1..* MS
* name 1..* MS"#;

    let diagnostic = Diagnostic {
        rule_id: "missing-metadata".to_string(),
        severity: Severity::Warning,
        message: "profile lacks required metadata: Id, Title, Description".to_string(),
        location: Location {
            file: PathBuf::from("examples/missing-metadata.fsh"),
            line: 2,
            column: 1,
            end_line: Some(2),
            end_column: Some(26),
            offset: 50,
            length: 25,
            span: Some((50, 75)),
        },
        suggestions: vec![
            CodeSuggestion {
                message: "add Id field".to_string(),
                replacement: "Id: incomplete-patient".to_string(),
                location: Location {
                    file: PathBuf::from("examples/missing-metadata.fsh"),
                    line: 3,
                    column: 1,
                    end_line: Some(3),
                    end_column: Some(1),
                    offset: 75,
                    length: 0,
                    span: Some((75, 75)),
                },
                applicability: Applicability::Always, // Safe to add
                labels: vec![],
            },
            CodeSuggestion {
                message: "add Title field".to_string(),
                replacement: "Title: \"Incomplete Patient Profile\"".to_string(),
                location: Location {
                    file: PathBuf::from("examples/missing-metadata.fsh"),
                    line: 3,
                    column: 1,
                    end_line: Some(3),
                    end_column: Some(1),
                    offset: 75,
                    length: 0,
                    span: Some((75, 75)),
                },
                applicability: Applicability::Always,
                labels: vec![],
            },
        ],
        code_snippet: Some("Profile: IncompletePatient".to_string()),
        code: Some("FSH023".to_string()),
        source: Some("builtin".to_string()),
        category: Some(DiagnosticCategory::Style),
    };

    let formatter = RichDiagnosticFormatter::new().no_colors(); // Disable colors for test
    let output = formatter.format_diagnostic(&diagnostic, source);

    println!("\n{}", "=".repeat(80));
    println!("Warning Diagnostic Demo");
    println!("{}", "=".repeat(80));
    println!("{output}");
    println!("{}", "=".repeat(80));

    assert!(output.contains("warning[FSH023]"));
    assert!(output.contains("missing-metadata"));
    assert!(output.contains("Profile: IncompletePatient"));
}

#[test]
fn demo_multiple_diagnostics() {
    let source = r#"Profile: TestProfile
Parent: Patient
* identifier 1..0
* name 2..*..5"#;

    let diagnostics = vec![
        Diagnostic {
            rule_id: "invalid-cardinality".to_string(),
            severity: Severity::Error,
            message: "upper bound cannot be less than lower bound".to_string(),
            location: Location {
                file: PathBuf::from("test.fsh"),
                line: 3,
                column: 14,
                end_line: Some(3),
                end_column: Some(18),
                offset: 50,
                length: 4,
                span: Some((50, 54)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: Some("FSH001".to_string()),
            source: Some("builtin".to_string()),
            category: Some(DiagnosticCategory::Correctness),
        },
        Diagnostic {
            rule_id: "invalid-cardinality-syntax".to_string(),
            severity: Severity::Error,
            message: "invalid cardinality syntax".to_string(),
            location: Location {
                file: PathBuf::from("test.fsh"),
                line: 4,
                column: 8,
                end_line: Some(4),
                end_column: Some(16),
                offset: 70,
                length: 8,
                span: Some((70, 78)),
            },
            suggestions: vec![],
            code_snippet: None,
            code: Some("FSH002".to_string()),
            source: Some("builtin".to_string()),
            category: Some(DiagnosticCategory::Correctness),
        },
    ];

    let formatter = RichDiagnosticFormatter::new().no_colors();
    let mut sources = std::collections::HashMap::new();
    sources.insert(PathBuf::from("test.fsh"), source.to_string());

    let output = formatter.format_diagnostics(&diagnostics, &sources);

    println!("\n{}", "=".repeat(80));
    println!("Multiple Diagnostics Demo");
    println!("{}", "=".repeat(80));
    println!("{output}");
    println!("{}", "=".repeat(80));

    assert!(output.contains("error[FSH001]"));
    assert!(output.contains("error[FSH002]"));
}
