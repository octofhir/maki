use fsh_lint_core::{
    DefaultDiagnosticCollector, DefaultOutputFormatter, Diagnostic, DiagnosticCategory,
    DiagnosticCollector, DiagnosticFormatter, DiagnosticOutputFormatter, Location, Severity,
    Suggestion,
};
use std::path::PathBuf;

#[test]
fn test_diagnostic_creation() {
    let location = Location::new(PathBuf::from("test.fsh"), 10, 5, 100, 15);

    let diagnostic = Diagnostic::new(
        "test-rule",
        Severity::Error,
        "Test error message",
        location.clone(),
    );

    assert_eq!(diagnostic.rule_id, "test-rule");
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.message, "Test error message");
    assert_eq!(diagnostic.location, location);
    assert!(diagnostic.suggestions.is_empty());
    assert!(diagnostic.code_snippet.is_none());
    assert!(diagnostic.category.is_none());
}

#[test]
fn test_diagnostic_with_suggestions() {
    let location = Location::new(PathBuf::from("test.fsh"), 10, 5, 100, 15);

    let suggestion = Suggestion::new(
        "Replace with correct syntax",
        "correct_syntax",
        location.clone(),
        true,
    );

    let diagnostic = Diagnostic::new("test-rule", Severity::Warning, "Test warning", location)
        .with_suggestion(suggestion)
        .with_category(DiagnosticCategory::Correctness)
        .with_code("W001")
        .with_source("parser");

    assert_eq!(diagnostic.suggestions.len(), 1);
    assert!(diagnostic.has_safe_fixes());
    assert_eq!(diagnostic.safe_fixes().len(), 1);
    assert_eq!(diagnostic.category, Some(DiagnosticCategory::Correctness));
    assert_eq!(diagnostic.code, Some("W001".to_string()));
    assert_eq!(diagnostic.source, Some("parser".to_string()));
}

#[test]
fn test_severity_ordering() {
    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Hint);
    assert!(Severity::Hint > Severity::Info);
}

#[test]
fn test_location_display() {
    let location = Location::new(PathBuf::from("src/test.fsh"), 42, 10, 500, 20);

    let display = format!("{}", location);
    assert_eq!(display, "src/test.fsh:42:10");
}

#[test]
fn test_location_with_end() {
    let location = Location::with_end(PathBuf::from("test.fsh"), 10, 5, 12, 8, 100, 25);

    assert_eq!(location.line, 10);
    assert_eq!(location.column, 5);
    assert_eq!(location.end_line, Some(12));
    assert_eq!(location.end_column, Some(8));
}

#[test]
fn test_diagnostic_collector_basic() {
    let mut collector = DefaultDiagnosticCollector::new();

    let diagnostic1 = create_test_diagnostic("rule1", Severity::Error, "Error 1", "file1.fsh", 10);
    let diagnostic2 =
        create_test_diagnostic("rule2", Severity::Warning, "Warning 1", "file2.fsh", 20);

    collector.collect(diagnostic1);
    collector.collect(diagnostic2);

    assert_eq!(collector.total_count(), 2);
    assert!(collector.has_errors());
    assert!(collector.has_warnings());
}

#[test]
fn test_diagnostic_collector_filtering() {
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error",
        "file.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Warning,
        "Warning",
        "file.fsh",
        2,
    ));
    collector.collect(create_test_diagnostic(
        "rule3",
        Severity::Info,
        "Info",
        "file.fsh",
        3,
    ));
    collector.collect(create_test_diagnostic(
        "rule4",
        Severity::Hint,
        "Hint",
        "file.fsh",
        4,
    ));

    let errors = collector.filter_by_severity(Severity::Error);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity, Severity::Error);

    let warnings_and_above = collector.filter_by_severity(Severity::Warning);
    assert_eq!(warnings_and_above.len(), 2);

    let hint_and_above = collector.filter_by_severity(Severity::Hint);
    assert_eq!(hint_and_above.len(), 3);

    let all_diagnostics = collector.filter_by_severity(Severity::Info);
    assert_eq!(all_diagnostics.len(), 4);
}

#[test]
fn test_diagnostic_collector_grouping() {
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error 1",
        "file1.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Warning,
        "Warning 1",
        "file1.fsh",
        2,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Error,
        "Error 2",
        "file2.fsh",
        1,
    ));

    // Test grouping by file
    let by_file = collector.group_by_file();
    assert_eq!(by_file.len(), 2);
    assert_eq!(by_file[&PathBuf::from("file1.fsh")].len(), 2);
    assert_eq!(by_file[&PathBuf::from("file2.fsh")].len(), 1);

    // Test grouping by rule
    let by_rule = collector.group_by_rule();
    assert_eq!(by_rule.len(), 2);
    assert_eq!(by_rule["rule1"].len(), 2);
    assert_eq!(by_rule["rule2"].len(), 1);

    // Test grouping by severity
    let by_severity = collector.group_by_severity();
    assert_eq!(by_severity[&Severity::Error].len(), 2);
    assert_eq!(by_severity[&Severity::Warning].len(), 1);
}

#[test]
fn test_diagnostic_collector_count_by_severity() {
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error 1",
        "file.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Error,
        "Error 2",
        "file.fsh",
        2,
    ));
    collector.collect(create_test_diagnostic(
        "rule3",
        Severity::Warning,
        "Warning 1",
        "file.fsh",
        3,
    ));

    let counts = collector.count_by_severity();
    assert_eq!(counts[&Severity::Error], 2);
    assert_eq!(counts[&Severity::Warning], 1);
    assert!(!counts.contains_key(&Severity::Info));
}

#[test]
fn test_diagnostic_collector_file_specific() {
    let mut collector = DefaultDiagnosticCollector::new();

    let file1 = PathBuf::from("file1.fsh");
    let file2 = PathBuf::from("file2.fsh");

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error 1",
        "file1.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Warning,
        "Warning 1",
        "file1.fsh",
        2,
    ));
    collector.collect(create_test_diagnostic(
        "rule3",
        Severity::Error,
        "Error 2",
        "file2.fsh",
        1,
    ));

    let file1_diagnostics = collector.diagnostics_for_file(&file1);
    assert_eq!(file1_diagnostics.len(), 2);

    let file2_diagnostics = collector.diagnostics_for_file(&file2);
    assert_eq!(file2_diagnostics.len(), 1);

    let nonexistent_file = PathBuf::from("nonexistent.fsh");
    let empty_diagnostics = collector.diagnostics_for_file(&nonexistent_file);
    assert_eq!(empty_diagnostics.len(), 0);
}

#[test]
fn test_diagnostic_collector_sorting() {
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Warning,
        "Warning",
        "file2.fsh",
        5,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Error,
        "Error",
        "file1.fsh",
        10,
    ));
    collector.collect(create_test_diagnostic(
        "rule3",
        Severity::Info,
        "Info",
        "file1.fsh",
        2,
    ));

    // Test sorting by location
    collector.sort_by_location();
    let diagnostics = collector.diagnostics();
    assert_eq!(diagnostics[0].location.file, PathBuf::from("file1.fsh"));
    assert_eq!(diagnostics[0].location.line, 2);
    assert_eq!(diagnostics[1].location.file, PathBuf::from("file1.fsh"));
    assert_eq!(diagnostics[1].location.line, 10);
    assert_eq!(diagnostics[2].location.file, PathBuf::from("file2.fsh"));

    // Test sorting by severity
    collector.sort_by_severity();
    let diagnostics = collector.diagnostics();
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert_eq!(diagnostics[1].severity, Severity::Warning);
    assert_eq!(diagnostics[2].severity, Severity::Info);
}

#[test]
fn test_diagnostic_collector_deduplication() {
    let mut collector = DefaultDiagnosticCollector::new();

    // Add duplicate diagnostics
    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error",
        "file.fsh",
        10,
    ));
    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error",
        "file.fsh",
        10,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Warning,
        "Warning",
        "file.fsh",
        20,
    ));

    assert_eq!(collector.total_count(), 3);

    collector.deduplicate();
    assert_eq!(collector.total_count(), 2);
}

#[test]
fn test_diagnostic_collector_clear() {
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error",
        "file.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Warning,
        "Warning",
        "file.fsh",
        2,
    ));

    assert_eq!(collector.total_count(), 2);

    collector.clear();
    assert_eq!(collector.total_count(), 0);
    assert!(!collector.has_errors());
    assert!(!collector.has_warnings());
}

#[test]
fn test_diagnostic_formatter_basic() {
    let formatter = DiagnosticFormatter::no_colors();
    let diagnostic = create_test_diagnostic(
        "test-rule",
        Severity::Error,
        "Test error message",
        "test.fsh",
        10,
    );

    let formatted = formatter.format_diagnostic(&diagnostic);

    assert!(formatted.contains("error"));
    assert!(formatted.contains("test.fsh:10:1"));
    assert!(formatted.contains("Test error message"));
}

#[test]
fn test_diagnostic_formatter_with_suggestions() {
    let formatter = DiagnosticFormatter::no_colors();

    let location = Location::new(PathBuf::from("test.fsh"), 10, 1, 100, 10);
    let suggestion = Suggestion::new("Use correct syntax", "fixed_code", location.clone(), true);

    let diagnostic = Diagnostic::new("test-rule", Severity::Warning, "Test warning", location)
        .with_suggestion(suggestion);

    let formatted = formatter.format_diagnostic(&diagnostic);

    assert!(formatted.contains("Suggestions:"));
    assert!(formatted.contains("Use correct syntax"));
    assert!(formatted.contains("fixed_code"));
}

#[test]
fn test_diagnostic_formatter_summary() {
    let formatter = DiagnosticFormatter::no_colors();
    let mut collector = DefaultDiagnosticCollector::new();

    collector.collect(create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error 1",
        "file.fsh",
        1,
    ));
    collector.collect(create_test_diagnostic(
        "rule2",
        Severity::Error,
        "Error 2",
        "file.fsh",
        2,
    ));
    collector.collect(create_test_diagnostic(
        "rule3",
        Severity::Warning,
        "Warning 1",
        "file.fsh",
        3,
    ));

    let summary = formatter.format_summary(&collector);

    assert!(summary.contains("3 issues"));
    assert!(summary.contains("2 errors"));
    assert!(summary.contains("1 warning"));
}

#[test]
fn test_diagnostic_formatter_no_issues() {
    let formatter = DiagnosticFormatter::no_colors();
    let collector = DefaultDiagnosticCollector::new();

    let summary = formatter.format_summary(&collector);
    assert!(summary.contains("No issues found"));
}

#[test]
fn test_output_formatter_json() {
    let formatter = DefaultOutputFormatter::new(DiagnosticFormatter::no_colors());
    let diagnostics = vec![create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error message",
        "test.fsh",
        10,
    )];

    let json_output = formatter.format_json(&diagnostics).unwrap();

    assert!(json_output.contains("\"rule_id\": \"rule1\""));
    assert!(json_output.contains("\"severity\": \"Error\""));
    assert!(json_output.contains("\"message\": \"Error message\""));
}

#[test]
fn test_output_formatter_sarif() {
    let formatter = DefaultOutputFormatter::new(DiagnosticFormatter::no_colors());
    let diagnostics = vec![create_test_diagnostic(
        "rule1",
        Severity::Error,
        "Error message",
        "test.fsh",
        10,
    )];

    let sarif_output = formatter.format_sarif(&diagnostics).unwrap();

    assert!(sarif_output.contains("\"version\": \"2.1.0\""));
    assert!(sarif_output.contains("\"name\": \"fsh-lint\""));
    assert!(sarif_output.contains("\"ruleId\": \"rule1\""));
    assert!(sarif_output.contains("\"level\": \"error\""));
}

#[test]
fn test_diagnostic_category_display() {
    assert_eq!(DiagnosticCategory::Correctness.to_string(), "correctness");
    assert_eq!(DiagnosticCategory::Suspicious.to_string(), "suspicious");
    assert_eq!(DiagnosticCategory::Complexity.to_string(), "complexity");
    assert_eq!(DiagnosticCategory::Performance.to_string(), "performance");
    assert_eq!(DiagnosticCategory::Style.to_string(), "style");
    assert_eq!(DiagnosticCategory::Nursery.to_string(), "nursery");
    assert_eq!(
        DiagnosticCategory::Accessibility.to_string(),
        "accessibility"
    );
    assert_eq!(
        DiagnosticCategory::Documentation.to_string(),
        "documentation"
    );
    assert_eq!(DiagnosticCategory::Security.to_string(), "security");
    assert_eq!(
        DiagnosticCategory::Compatibility.to_string(),
        "compatibility"
    );
    assert_eq!(
        DiagnosticCategory::Custom("custom".to_string()).to_string(),
        "custom"
    );
}

#[test]
fn test_severity_display() {
    assert_eq!(Severity::Error.to_string(), "error");
    assert_eq!(Severity::Warning.to_string(), "warning");
    assert_eq!(Severity::Info.to_string(), "info");
    assert_eq!(Severity::Hint.to_string(), "hint");
}

// Helper function to create test diagnostics
fn create_test_diagnostic(
    rule_id: &str,
    severity: Severity,
    message: &str,
    file: &str,
    line: usize,
) -> Diagnostic {
    let location = Location::new(PathBuf::from(file), line, 1, (line - 1) * 50, message.len());

    Diagnostic::new(rule_id, severity, message, location)
}

#[test]
fn test_diagnostic_collector_with_capacity() {
    let collector = DefaultDiagnosticCollector::with_capacity(100);
    assert_eq!(collector.total_count(), 0);
}

#[test]
fn test_diagnostic_collector_collect_all() {
    let mut collector = DefaultDiagnosticCollector::new();

    let diagnostics = vec![
        create_test_diagnostic("rule1", Severity::Error, "Error 1", "file.fsh", 1),
        create_test_diagnostic("rule2", Severity::Warning, "Warning 1", "file.fsh", 2),
        create_test_diagnostic("rule3", Severity::Info, "Info 1", "file.fsh", 3),
    ];

    collector.collect_all(diagnostics);
    assert_eq!(collector.total_count(), 3);
}

#[test]
fn test_location_with_span() {
    let location = Location::with_span(PathBuf::from("test.fsh"), 10, 5, 100, 15, (100, 115));

    assert_eq!(location.span, Some((100, 115)));
    assert_eq!(location.line, 10);
    assert_eq!(location.column, 5);
}

#[test]
fn test_suggestion_creation() {
    let location = Location::new(PathBuf::from("test.fsh"), 10, 5, 100, 10);
    let suggestion = Suggestion::new(
        "Fix the syntax error",
        "corrected_syntax",
        location.clone(),
        true,
    );

    assert_eq!(suggestion.message, "Fix the syntax error");
    assert_eq!(suggestion.replacement, "corrected_syntax");
    assert_eq!(suggestion.location, location);
    assert!(suggestion.is_safe);
}

#[test]
fn test_diagnostic_safe_fixes() {
    let location = Location::new(PathBuf::from("test.fsh"), 10, 5, 100, 10);

    let safe_suggestion = Suggestion::new("Safe fix", "safe_replacement", location.clone(), true);
    let unsafe_suggestion =
        Suggestion::new("Unsafe fix", "unsafe_replacement", location.clone(), false);

    let diagnostic = Diagnostic::new("test-rule", Severity::Warning, "Test warning", location)
        .with_suggestion(safe_suggestion)
        .with_suggestion(unsafe_suggestion);

    assert!(diagnostic.has_safe_fixes());
    assert_eq!(diagnostic.safe_fixes().len(), 1);
    assert_eq!(diagnostic.safe_fixes()[0].message, "Safe fix");
}
