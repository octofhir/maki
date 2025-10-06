//! Diagnostic rendering demo
//!
//! This example demonstrates the diagnostic rendering capabilities
//! of the fsh-lint diagnostic system.
//!
//! Run with: cargo run --package fsh-lint-core --example diagnostic_demo

use fsh_lint_core::{
    Applicability, CodeSuggestion, Diagnostic, DiagnosticRenderer, Location, OutputFormat, Severity,
};
use std::io::Write;
use tempfile::NamedTempFile;

fn main() {
    println!("=== FSH Lint Diagnostic Rendering Demo ===\n");

    // Create a temporary FSH file for demonstration
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let fsh_content = r#"Profile: myPatientProfile
Parent: Patient
Title: "my patient profile"
Description: "A custom patient profile"

* name 1..1
* gender 0..1
* birthDate 1..1
"#;
    temp_file
        .write_all(fsh_content.as_bytes())
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");

    // Demo 1: Error diagnostic with code frame
    println!("--- Example 1: Error with Code Frame ---\n");
    let error_diagnostic = Diagnostic {
        rule_id: "naming/profile-pascal-case".to_string(),
        severity: Severity::Error,
        message: "Profile name must use PascalCase".to_string(),
        location: Location {
            file: temp_file.path().to_path_buf(),
            line: 1,
            column: 10,
            end_line: Some(1),
            end_column: Some(27),
            offset: 9,
            length: 17,
            span: Some((9, 26)),
        },
        suggestions: vec![CodeSuggestion {
            message: "Rename to use PascalCase".to_string(),
            replacement: "MyPatientProfile".to_string(),
            location: Location {
                file: temp_file.path().to_path_buf(),
                line: 1,
                column: 10,
                end_line: Some(1),
                end_column: Some(27),
                offset: 9,
                length: 17,
                span: Some((9, 26)),
            },
            applicability: Applicability::Always,
            labels: vec![],
        }],
        code_snippet: None,
        code: Some("E001".to_string()),
        source: Some("naming-rules".to_string()),
        category: None,
    };

    let renderer = DiagnosticRenderer::new();
    println!("{}", renderer.render(&error_diagnostic));

    // Demo 2: Warning diagnostic
    println!("\n--- Example 2: Warning with Suggestion ---\n");
    let warning_diagnostic = Diagnostic {
        rule_id: "style/title-case".to_string(),
        severity: Severity::Warning,
        message: "Title should use Title Case".to_string(),
        location: Location {
            file: temp_file.path().to_path_buf(),
            line: 3,
            column: 8,
            end_line: Some(3),
            end_column: Some(28),
            offset: 48,
            length: 20,
            span: Some((48, 68)),
        },
        suggestions: vec![CodeSuggestion {
            message: "Use Title Case for profile title".to_string(),
            replacement: r#"Title: "My Patient Profile""#.to_string(),
            location: Location {
                file: temp_file.path().to_path_buf(),
                line: 3,
                column: 1,
                end_line: Some(3),
                end_column: Some(28),
                offset: 41,
                length: 27,
                span: Some((41, 68)),
            },
            applicability: Applicability::MaybeIncorrect,
            labels: vec![],
        }],
        code_snippet: None,
        code: None,
        source: None,
        category: None,
    };

    println!("{}", renderer.render(&warning_diagnostic));

    // Demo 3: Multiple diagnostics with unsafe fix note
    println!("\n--- Example 3: Multiple Diagnostics with Unsafe Fix Note ---\n");
    let info_diagnostic = Diagnostic {
        rule_id: "documentation/description".to_string(),
        severity: Severity::Info,
        message: "Consider adding more detailed description".to_string(),
        location: Location {
            file: temp_file.path().to_path_buf(),
            line: 4,
            column: 14,
            end_line: Some(4),
            end_column: Some(40),
            offset: 82,
            length: 26,
            span: Some((82, 108)),
        },
        suggestions: vec![],
        code_snippet: None,
        code: None,
        source: None,
        category: None,
    };

    let diagnostics = vec![
        error_diagnostic.clone(),
        warning_diagnostic.clone(),
        info_diagnostic,
    ];

    // Use render_diagnostics_with_summary to show the unsafe fix note
    println!("{}", renderer.render_diagnostics_with_summary(&diagnostics));

    // Demo 4: No colors output
    println!("\n--- Example 4: Plain Text (No Colors) ---\n");
    let no_color_renderer = DiagnosticRenderer::no_colors();
    println!("{}", no_color_renderer.render(&error_diagnostic));

    // Demo 5: JSON output
    println!("\n--- Example 5: JSON Output ---\n");
    let json_renderer = DiagnosticRenderer::with_format(OutputFormat::JsonPretty);
    println!("{}", json_renderer.render(&error_diagnostic));

    // Demo 6: Multiple diagnostics in JSON
    println!("\n--- Example 6: Multiple Diagnostics as JSON ---\n");
    let diagnostics = vec![error_diagnostic.clone(), warning_diagnostic.clone()];
    println!("{}", json_renderer.render_diagnostics(&diagnostics));

    println!("\n=== Demo Complete ===");
}
