//! Tests for the autofix engine

use fsh_lint_core::{
    autofix::{AutofixEngine, ConflictType, DefaultAutofixEngine, Fix, FixConfig},
    diagnostics::{Diagnostic, Location, Severity, Suggestion},
    rules::{AutofixTemplate, FixSafety},
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to create a test diagnostic with suggestions
fn create_test_diagnostic_with_suggestions() -> Diagnostic {
    let location = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);

    let safe_suggestion = Suggestion::new(
        "Replace with correct syntax",
        "fixed_text",
        location.clone(),
        true,
    );

    let unsafe_suggestion = Suggestion::new(
        "Complex replacement",
        "complex_replacement_text",
        Location::new(PathBuf::from("test.fsh"), 2, 1, 10, 8),
        false,
    );

    Diagnostic::new("test-rule", Severity::Error, "Test error", location)
        .with_suggestion(safe_suggestion)
        .with_suggestion(unsafe_suggestion)
}

/// Helper function to create a test file with content
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let file_path = dir.path().join(name);
    std::fs::write(&file_path, content).unwrap();
    file_path
}

#[test]
fn test_fix_creation_and_properties() {
    let location = Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5);
    let fix = Fix::new(
        "test-fix".to_string(),
        "Test fix".to_string(),
        location,
        "replacement".to_string(),
        FixSafety::Safe,
        "test-rule".to_string(),
    )
    .with_priority(10);

    assert_eq!(fix.id, "test-fix");
    assert_eq!(fix.description, "Test fix");
    assert_eq!(fix.replacement, "replacement");
    assert!(fix.is_safe());
    assert_eq!(fix.priority, 10);
    assert_eq!(fix.span(), (0, 5));
}

#[test]
fn test_fix_conflicts_detection() {
    let file = PathBuf::from("test.fsh");

    // Overlapping fixes
    let fix1 = Fix::new(
        "fix1".to_string(),
        "Fix 1".to_string(),
        Location::new(file.clone(), 1, 1, 0, 10),
        "text1".to_string(),
        FixSafety::Safe,
        "rule1".to_string(),
    );

    let fix2 = Fix::new(
        "fix2".to_string(),
        "Fix 2".to_string(),
        Location::new(file.clone(), 1, 5, 5, 10),
        "text2".to_string(),
        FixSafety::Safe,
        "rule2".to_string(),
    );

    // Non-overlapping fix
    let fix3 = Fix::new(
        "fix3".to_string(),
        "Fix 3".to_string(),
        Location::new(file.clone(), 1, 20, 20, 5),
        "text3".to_string(),
        FixSafety::Safe,
        "rule3".to_string(),
    );

    assert!(fix1.conflicts_with(&fix2)); // Overlapping
    assert!(!fix1.conflicts_with(&fix3)); // Non-overlapping
    assert!(!fix2.conflicts_with(&fix3)); // Non-overlapping
}

#[test]
fn test_generate_fixes_from_diagnostics() {
    let engine = DefaultAutofixEngine::new();
    let diagnostic = create_test_diagnostic_with_suggestions();

    let fixes = engine.generate_fixes(&[diagnostic]).unwrap();

    assert_eq!(fixes.len(), 2);

    // First fix should be safe
    assert_eq!(fixes[0].rule_id, "test-rule");
    assert!(fixes[0].is_safe());
    assert_eq!(fixes[0].replacement, "fixed_text");

    // Second fix should be unsafe
    assert_eq!(fixes[1].rule_id, "test-rule");
    assert!(!fixes[1].is_safe());
    assert_eq!(fixes[1].replacement, "complex_replacement_text");
}

#[test]
fn test_generate_fixes_from_templates() {
    let engine = DefaultAutofixEngine::new();
    let diagnostic = create_test_diagnostic_with_suggestions();

    let mut templates = HashMap::new();
    templates.insert(
        "test-rule".to_string(),
        AutofixTemplate {
            description: "Template fix".to_string(),
            replacement_template: "template_{{rule_id}}_replacement".to_string(),
            safety: FixSafety::Safe,
        },
    );

    let fixes = engine
        .generate_fixes_from_templates(&[diagnostic], &templates)
        .unwrap();

    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].replacement, "template_test-rule_replacement");
    assert!(fixes[0].is_safe());
}

#[test]
fn test_conflict_resolution() {
    let engine = DefaultAutofixEngine::new();
    let file = PathBuf::from("test.fsh");

    // Create conflicting fixes with different priorities
    let fix1 = Fix::new(
        "fix1".to_string(),
        "Fix 1".to_string(),
        Location::new(file.clone(), 1, 1, 0, 10),
        "text1".to_string(),
        FixSafety::Safe,
        "rule1".to_string(),
    )
    .with_priority(1);

    let fix2 = Fix::new(
        "fix2".to_string(),
        "Fix 2".to_string(),
        Location::new(file.clone(), 1, 5, 5, 10),
        "text2".to_string(),
        FixSafety::Safe,
        "rule2".to_string(),
    )
    .with_priority(2);

    let fix3 = Fix::new(
        "fix3".to_string(),
        "Fix 3".to_string(),
        Location::new(file.clone(), 1, 20, 20, 5),
        "text3".to_string(),
        FixSafety::Safe,
        "rule3".to_string(),
    )
    .with_priority(1);

    let resolved = engine.resolve_conflicts(&[fix1, fix2, fix3]);

    // Should keep the higher priority fix (fix2) and the non-conflicting fix (fix3)
    assert_eq!(resolved.len(), 2);
    assert!(resolved.iter().any(|f| f.id == "fix2"));
    assert!(resolved.iter().any(|f| f.id == "fix3"));
    assert!(!resolved.iter().any(|f| f.id == "fix1"));
}

#[test]
fn test_fix_validation() {
    let engine = DefaultAutofixEngine::new();

    // Valid fix
    let valid_fix = Fix::new(
        "valid-fix".to_string(),
        "Valid fix".to_string(),
        Location::new(PathBuf::from("test.fsh"), 1, 1, 0, 5),
        "replacement".to_string(),
        FixSafety::Safe,
        "test-rule".to_string(),
    );

    // Invalid fix with empty file path
    let invalid_fix = Fix::new(
        "invalid-fix".to_string(),
        "Invalid fix".to_string(),
        Location::new(PathBuf::new(), 1, 1, 0, 0),
        "".to_string(),
        FixSafety::Safe,
        "test-rule".to_string(),
    );

    assert!(engine.validate_fixes(&[valid_fix]).is_ok());
    assert!(engine.validate_fixes(&[invalid_fix]).is_err());
}

#[test]
fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(&temp_dir, "test.fsh", "original content");

    let engine = DefaultAutofixEngine::new();
    let fix = Fix::new(
        "test-fix".to_string(),
        "Test fix".to_string(),
        Location::new(file_path.clone(), 1, 1, 0, 8),
        "modified".to_string(),
        FixSafety::Safe,
        "test-rule".to_string(),
    );

    let config = FixConfig::new().dry_run();
    let results = engine.apply_fixes(&[fix], &config).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].applied_count, 1);
    assert!(results[0].modified_content.is_some());

    // Original file should be unchanged
    let file_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(file_content, "original content");

    // Modified content should contain the fix
    let modified = results[0].modified_content.as_ref().unwrap();
    assert_eq!(modified, "modified content");
}

#[test]
fn test_unsafe_fix_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(&temp_dir, "test.fsh", "original content");

    let engine = DefaultAutofixEngine::new();

    let safe_fix = Fix::new(
        "safe-fix".to_string(),
        "Safe fix".to_string(),
        Location::new(file_path.clone(), 1, 1, 0, 8),
        "safe_mod".to_string(),
        FixSafety::Safe,
        "safe-rule".to_string(),
    );

    let unsafe_fix = Fix::new(
        "unsafe-fix".to_string(),
        "Unsafe fix".to_string(),
        Location::new(file_path.clone(), 1, 10, 9, 7),
        "unsafe_mod".to_string(),
        FixSafety::Unsafe,
        "unsafe-rule".to_string(),
    );

    // Test with unsafe fixes disabled (default)
    let config = FixConfig::new().dry_run();
    let results = engine
        .apply_fixes(&[safe_fix.clone(), unsafe_fix.clone()], &config)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].applied_count, 1); // Only safe fix applied

    // Test with unsafe fixes enabled
    let config = FixConfig::new().with_unsafe_fixes().dry_run();
    let results = engine
        .apply_fixes(&[safe_fix, unsafe_fix], &config)
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].applied_count, 2); // Both fixes applied
}

#[test]
fn test_fix_application_with_syntax_validation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(
        &temp_dir,
        "test.fsh",
        "Profile: TestProfile\n{\n  element: string\n}",
    );

    let engine = DefaultAutofixEngine::new();

    // Valid fix that maintains syntax
    let valid_fix = Fix::new(
        "valid-fix".to_string(),
        "Add missing semicolon".to_string(),
        Location::new(file_path.clone(), 3, 17, 42, 0),
        ";".to_string(),
        FixSafety::Safe,
        "syntax-rule".to_string(),
    );

    let config = FixConfig::new().dry_run();
    let results = engine.apply_fixes(&[valid_fix], &config).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].applied_count, 1);
    assert!(results[0].errors.is_empty());
}

#[test]
fn test_fix_preview_generation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(&temp_dir, "test.fsh", "original content here");

    let engine = DefaultAutofixEngine::new();
    let fix = Fix::new(
        "preview-fix".to_string(),
        "Preview fix".to_string(),
        Location::new(file_path.clone(), 1, 1, 0, 8),
        "modified".to_string(),
        FixSafety::Safe,
        "preview-rule".to_string(),
    );

    let previews = engine.preview_fixes(&[fix.clone()]).unwrap();

    assert_eq!(previews.len(), 1);
    assert_eq!(previews[0].file, file_path);
    assert_eq!(previews[0].original_content, "original content here");
    assert_eq!(previews[0].modified_content, "modified content here");
    assert_eq!(previews[0].applied_fixes.len(), 1);
    assert_eq!(previews[0].applied_fixes[0].id, "preview-fix");
    assert!(!previews[0].diff.is_empty());
}

#[test]
fn test_rollback_plan_creation_and_execution() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = create_test_file(&temp_dir, "test.fsh", "original content");

    let engine = DefaultAutofixEngine::new();
    let fix = Fix::new(
        "rollback-fix".to_string(),
        "Rollback test fix".to_string(),
        Location::new(file_path.clone(), 1, 1, 0, 8),
        "modified".to_string(),
        FixSafety::Safe,
        "rollback-rule".to_string(),
    );

    // Apply fix (not in dry-run mode)
    let config = FixConfig::new();
    let results = engine.apply_fixes(&[fix], &config).unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].applied_count, 1);

    // Verify file was modified
    let modified_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(modified_content, "modified content");

    // Create and execute rollback plan
    let rollback = engine.create_rollback(&results).unwrap();
    assert!(rollback.age().as_secs() < 1); // Should be very recent
}

#[test]
fn test_batch_fix_application() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_test_file(&temp_dir, "test1.fsh", "content1");
    let file2 = create_test_file(&temp_dir, "test2.fsh", "content2");

    let engine = DefaultAutofixEngine::new();

    let fix1 = Fix::new(
        "batch-fix1".to_string(),
        "Batch fix 1".to_string(),
        Location::new(file1, 1, 1, 0, 8),
        "modified1".to_string(),
        FixSafety::Safe,
        "batch-rule1".to_string(),
    );

    let fix2 = Fix::new(
        "batch-fix2".to_string(),
        "Batch fix 2".to_string(),
        Location::new(file2, 1, 1, 0, 8),
        "modified2".to_string(),
        FixSafety::Safe,
        "batch-rule2".to_string(),
    );

    let config = FixConfig::new().dry_run();
    let results = engine
        .apply_fixes_batch(&[fix1, fix2], &config, None)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.applied_count == 1));
}

#[test]
fn test_fix_safety_classification() {
    let engine = DefaultAutofixEngine::new();

    // Test safe replacement patterns
    assert!(engine.is_safe_replacement_pattern(""));
    assert!(engine.is_safe_replacement_pattern("   "));
    assert!(engine.is_safe_replacement_pattern(";"));
    assert!(engine.is_safe_replacement_pattern("()"));
    assert!(engine.is_safe_replacement_pattern("true"));
    assert!(engine.is_safe_replacement_pattern("false"));

    // Test unsafe patterns
    assert!(!engine.is_safe_replacement_pattern("complex_function_call()"));
    assert!(!engine.is_safe_replacement_pattern("very long replacement text"));

    // Test dangerous patterns
    assert!(engine.is_dangerous_replacement("eval(malicious_code)"));
    assert!(engine.is_dangerous_replacement("import os"));
    assert!(engine.is_dangerous_replacement("http://malicious.com"));
    assert!(!engine.is_dangerous_replacement("normal replacement"));
}

#[test]
fn test_fsh_syntax_validation() {
    let engine = DefaultAutofixEngine::new();

    // Valid FSH syntax
    let valid_fsh = r#"
Profile: TestProfile
Parent: Patient
{
  element: string
}
"#;
    assert!(engine.validate_fsh_syntax(valid_fsh).is_ok());

    // Invalid FSH syntax (unmatched braces)
    let invalid_fsh = r#"
Profile: TestProfile
Parent: Patient
{
  element: string
"#;
    assert!(engine.validate_fsh_syntax(invalid_fsh).is_err());

    // Invalid FSH syntax (unmatched parentheses)
    let invalid_fsh2 = r#"
Profile: TestProfile
Parent: Patient
{
  element: string (missing close paren
}
"#;
    assert!(engine.validate_fsh_syntax(invalid_fsh2).is_err());
}

#[test]
fn test_complex_conflict_detection() {
    let engine = DefaultAutofixEngine::new();
    let file = PathBuf::from("test.fsh");

    let fix1 = Fix::new(
        "fix1".to_string(),
        "Fix 1".to_string(),
        Location::new(file.clone(), 1, 1, 0, 5),
        "text1".to_string(),
        FixSafety::Safe,
        "same-rule".to_string(),
    );

    let fix2 = Fix::new(
        "fix2".to_string(),
        "Fix 2".to_string(),
        Location::new(file.clone(), 2, 1, 10, 5),
        "text2".to_string(),
        FixSafety::Safe,
        "same-rule".to_string(),
    );

    let fix3 = Fix::new(
        "fix3".to_string(),
        "Fix 3".to_string(),
        Location::new(file.clone(), 10, 1, 100, 5),
        "text3".to_string(),
        FixSafety::Safe,
        "different-rule".to_string(),
    );

    let conflicts = engine.detect_complex_conflicts(&[fix1, fix2, fix3]);

    // Should detect semantic conflict between fix1 and fix2 (same rule, nearby lines)
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].conflict_type, ConflictType::Overlap);
    assert_eq!(conflicts[0].fix_indices.len(), 2);
}

#[test]
fn test_fix_config_builder() {
    let config = FixConfig::new()
        .with_unsafe_fixes()
        .dry_run()
        .with_max_fixes(5)
        .without_validation();

    assert!(config.apply_unsafe);
    assert!(config.dry_run);
    assert_eq!(config.max_fixes_per_file, Some(5));
    assert!(!config.validate_syntax);
}
