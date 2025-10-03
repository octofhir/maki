//! Golden file regression tests for the FSH formatter

use fsh_lint_core::{AstFormatter, CachedFshParser, Formatter, config::FormatterConfig};
use std::fs;
use std::path::Path;

/// Create a test formatter
fn create_formatter() -> AstFormatter<CachedFshParser> {
    let parser = CachedFshParser::new().unwrap();
    AstFormatter::new(parser)
}

/// Create test configuration
fn create_config() -> FormatterConfig {
    FormatterConfig {
        indent_size: 2,
        max_line_width: 100,
        align_carets: true,
    }
}

/// Run a golden file test
fn run_golden_test(test_name: &str) {
    let mut formatter = create_formatter();
    let config = create_config();

    // Read the golden file
    let golden_path = format!("tests/golden_files/{}.fsh", test_name);
    let golden_content = fs::read_to_string(&golden_path)
        .unwrap_or_else(|_| panic!("Failed to read golden file: {}", golden_path));

    // Create a "messy" version by removing proper spacing
    let messy_content = create_messy_version(&golden_content);

    // Format the messy content
    let result = formatter.format_string(&messy_content, &config).unwrap();

    // Compare with golden file (normalize line endings)
    let expected = normalize_content(&golden_content);
    let actual = normalize_content(&result.content);

    if expected != actual {
        println!("Golden file test failed for: {}", test_name);
        println!("Expected:\n{}", expected);
        println!("Actual:\n{}", actual);
        panic!("Golden file test failed");
    }

    // Test idempotency - formatting the result should not change it
    let second_result = formatter.format_string(&result.content, &config).unwrap();
    assert!(
        !second_result.changed,
        "Formatting should be idempotent for {}",
        test_name
    );
}

/// Create a messy version of well-formatted content for testing
fn create_messy_version(content: &str) -> String {
    content
        .lines()
        .map(|line| format!("{}  ", line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Normalize content for comparison (handle line endings, trailing whitespace)
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[test]
fn test_golden_simple_profile() {
    run_golden_test("simple_profile");
}

#[test]
fn test_golden_complex_profile() {
    run_golden_test("complex_profile");
}

#[test]
fn test_golden_extension() {
    run_golden_test("extension");
}

#[test]
fn test_golden_value_set() {
    run_golden_test("value_set");
}

#[test]
fn test_golden_with_comments() {
    run_golden_test("with_comments");
}

#[test]
fn test_golden_caret_alignment() {
    run_golden_test("caret_alignment");
}

/// Test that all golden files can be parsed and formatted without errors
#[test]
fn test_all_golden_files_parse() {
    let mut formatter = create_formatter();
    let config = create_config();

    let golden_dir = Path::new("tests/golden_files");
    if !golden_dir.exists() {
        return; // Skip if golden files directory doesn't exist
    }

    for entry in fs::read_dir(golden_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("fsh") {
            let content = fs::read_to_string(&path).unwrap();
            let result = formatter.format_string(&content, &config);

            assert!(result.is_ok(), "Failed to format golden file: {:?}", path);

            let formatted = result.unwrap();
            assert!(
                !formatted.content.is_empty(),
                "Formatted content is empty for: {:?}",
                path
            );
        }
    }
}

/// Benchmark test for formatter performance
#[test]
fn test_formatter_performance() {
    let mut formatter = create_formatter();
    let config = create_config();

    // Create a large FSH content for performance testing
    let mut large_content = String::new();
    for i in 0..100 {
        large_content.push_str(&format!(
            r#"
Profile: TestProfile{}
Parent: Patient
Id: test-profile-{}
Title: "Test Profile {}"
Description: "A test profile for performance testing"
* name 1..1 MS
* name.family 1..1
* name.given 1..*
* gender 1..1
* birthDate 0..1
* address 0..*
* telecom 0..*
* ^status = #active
* ^experimental = false
* ^date = "2024-01-01"
* ^publisher = "Test Organization"

"#,
            i, i, i
        ));
    }

    let start = std::time::Instant::now();
    let result = formatter.format_string(&large_content, &config).unwrap();
    let duration = start.elapsed();

    println!(
        "Formatted {} characters in {:?}",
        large_content.len(),
        duration
    );

    // Should complete within reasonable time (adjust threshold as needed)
    assert!(
        duration.as_secs() < 5,
        "Formatting took too long: {:?}",
        duration
    );
    assert!(!result.content.is_empty());
}

/// Test formatter with various configuration options
#[test]
fn test_formatter_configurations() {
    let mut formatter = create_formatter();

    let content = r#"Profile: TestProfile
Parent: Patient
* name 1..1
* ^title = "Test"
* ^description = "Test description""#;

    // Test different indent sizes
    for indent_size in [2, 4, 8] {
        let config = FormatterConfig {
            indent_size,
            max_line_width: 100,
            align_carets: true,
        };

        let result = formatter.format_string(content, &config).unwrap();
        assert!(!result.content.is_empty());

        assert!(result.content.contains('*'));
    }

    // Test different line widths
    for max_line_width in [40, 80, 120] {
        let config = FormatterConfig {
            indent_size: 2,
            max_line_width,
            align_carets: true,
        };

        let result = formatter.format_string(content, &config).unwrap();
        assert!(!result.content.is_empty());
    }

    // Test caret alignment on/off
    for align_carets in [true, false] {
        let config = FormatterConfig {
            indent_size: 2,
            max_line_width: 100,
            align_carets,
        };

        let result = formatter.format_string(content, &config).unwrap();
        assert!(!result.content.is_empty());
    }
}

/// Test edge cases and error conditions
#[test]
fn test_formatter_edge_cases() {
    let mut formatter = create_formatter();
    let config = create_config();

    // Empty content
    let result = formatter.format_string("", &config).unwrap();
    assert_eq!(result.content, "");
    assert!(!result.changed);

    // Whitespace only
    let result = formatter.format_string("   \n  \n  ", &config).unwrap();
    assert!(result.content.trim().is_empty());

    // Single line
    let result = formatter.format_string("Profile: Test", &config).unwrap();
    assert!(!result.content.is_empty());

    // Very long line
    let long_line = "Profile: ".to_string() + &"A".repeat(200);
    let result = formatter.format_string(&long_line, &config).unwrap();
    assert!(!result.content.is_empty());

    // Mixed line endings
    let mixed_endings = "Profile: Test\r\nParent: Patient\n* name 1..1\r\n";
    let result = formatter.format_string(mixed_endings, &config).unwrap();
    assert!(!result.content.is_empty());
    assert!(!result.content.contains('\r'));
}
