//! Integration tests for the FSH Lint CLI
//!
//! These tests verify the CLI behavior end-to-end

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Helper function to create a test CLI command
#[allow(deprecated)]
fn cli() -> Command {
    Command::cargo_bin("maki").unwrap()
}

/// Helper function to create a temporary directory with test files
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    // Create a minimal valid FSH file that should pass all linting rules
    // Just an empty file with a comment to avoid parse errors
    let fsh_content = r#"// This is a test FSH file
"#;

    fs::write(temp_dir.path().join("test.fsh"), fsh_content).unwrap();

    // Create a configuration file that disables problematic rules
    // Note: Some GritQL rules have syntax errors and fail to compile,
    // so we disable them for integration tests that just test CLI behavior
    let config_content = r#"
{
  "include": ["**/*.fsh"],
  "exclude": ["node_modules/**"],
  "rules": {
    "correctness/invalid-caret-path": "off",
    "correctness/invalid-constraint": "off",
    "correctness/malformed-alias": "off",
    "correctness/missing-profile-id": "off",
    "documentation/profile-without-examples": "off",
    "suspicious/trailing-text": "off",
    "style/profile-naming-convention": "off"
  }
}
"#;

    fs::write(temp_dir.path().join(".makirc.json"), config_content).unwrap();

    temp_dir
}

#[test]
fn test_help_command() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "MAKI is a fast, extensible toolkit for FHIR Shorthand (FSH) projects.",
        ))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_version_command() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(VERSION));
}

#[test]
fn test_version_detailed() {
    cli()
        .args(["version", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("maki {VERSION}")))
        .stdout(predicate::str::contains("Build information:"))
        .stdout(predicate::str::contains("Target:"))
        .stdout(predicate::str::contains("OS:"));
}

#[test]
fn test_lint_help() {
    cli()
        .args(["lint", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Lint FSH files"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--write"));
}

#[test]
fn test_lint_with_json_format() {
    let temp_dir = create_test_project();
    cli()
        .args([
            "lint",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"files_checked\""))
        .stdout(predicate::str::contains("\"issues\""))
        .stdout(predicate::str::contains("\"summary\""));
}

#[test]
fn test_lint_with_sarif_format() {
    let temp_dir = create_test_project();
    cli()
        .args([
            "lint",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "sarif",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"version\": \"2.1.0\""))
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"runs\""));
}

#[test]
fn test_lint_with_compact_format() {
    let temp_dir = create_test_project();
    cli()
        .args([
            "lint",
            temp_dir.path().to_str().unwrap(),
            "--format",
            "compact",
        ])
        .assert()
        .success();
    // Compact format may not produce specific output when no issues found
}

#[test]
fn test_lint_with_progress() {
    let temp_dir = create_test_project();
    cli()
        .args(["lint", temp_dir.path().to_str().unwrap(), "--progress"])
        .assert()
        .success();
    // Progress output may vary
}

#[test]
fn test_lint_with_config_file() {
    let temp_dir = create_test_project();

    cli()
        .args([
            "lint",
            temp_dir.path().to_str().unwrap(),
            "--config",
            temp_dir.path().join(".makirc.json").to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn test_lint_nonexistent_path() {
    cli().args(["lint", "/nonexistent/path"]).assert().failure(); // Should fail with nonexistent path
}

#[test]
fn test_rules_list() {
    cli()
        .args(["rules", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available Rules:"))
        .stdout(predicate::str::contains("correctness/invalid-keyword"));
}

#[test]
fn test_rules_list_detailed() {
    cli()
        .args(["rules", "--detailed", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Category:"))
        .stdout(predicate::str::contains("Status:"));
}

#[test]
fn test_rules_explain() {
    cli()
        .args(["rules", "explain", "correctness/invalid-keyword"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rule: correctness/invalid-keyword",
        ))
        .stdout(predicate::str::contains("Description:"));
}

#[test]
fn test_rules_explain_nonexistent() {
    cli()
        .args(["rules", "explain", "NONEXISTENT"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn test_rules_search() {
    cli()
        .args(["rules", "search", "keyword"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rules matching"));
}

#[test]
fn test_rules_search_no_matches() {
    cli()
        .args(["rules", "search", "nonexistentquery"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No rules found"));
}

#[test]
fn test_config_init() {
    let temp_dir = TempDir::new().unwrap();

    cli()
        .args(["config", "init"])
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created configuration file"));

    // Verify the file was created
    assert!(temp_dir.path().join(".makirc.json").exists());
}

#[test]
fn test_config_init_with_examples() {
    let temp_dir = TempDir::new().unwrap();

    cli()
        .args(["config", "init", "--with-examples"])
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("includes example rules"));

    // Verify the file contains examples
    let content = fs::read_to_string(temp_dir.path().join(".makirc.json")).unwrap();
    assert!(content.contains("invalid-keyword"));
}

#[test]
fn test_config_init_toml_format() {
    let temp_dir = TempDir::new().unwrap();

    cli()
        .args(["config", "init", "--format", "toml"])
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Verify the TOML file was created
    assert!(temp_dir.path().join(".makirc.toml").exists());
}

#[test]
fn test_config_init_force_overwrite() {
    let temp_dir = TempDir::new().unwrap();

    // Create initial config
    cli()
        .args(["config", "init"])
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Try to overwrite without force (should fail)
    cli()
        .args(["config", "init"])
        .current_dir(temp_dir.path())
        .assert()
        .failure();

    // Overwrite with force (should succeed)
    cli()
        .args(["config", "init", "--force"])
        .current_dir(temp_dir.path())
        .assert()
        .success();
}

#[test]
fn test_config_validate_valid() {
    let temp_dir = create_test_project();

    cli()
        .args(["config", "validate", ".makirc.json"])
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration is valid"));
}

#[test]
fn test_config_validate_specific_file() {
    let temp_dir = create_test_project();

    cli()
        .args([
            "config",
            "validate",
            temp_dir.path().join(".makirc.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration is valid"));
}

#[test]
fn test_config_validate_nonexistent() {
    cli()
        .args(["config", "validate", "/nonexistent/config.json"])
        .assert()
        .failure();
}

#[test]
fn test_config_show() {
    let temp_dir = create_test_project();

    cli()
        .args(["config", "show"])
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration:"))
        .stdout(predicate::str::contains("include"));
}

#[test]
fn test_config_show_resolved() {
    let temp_dir = create_test_project();

    cli()
        .args(["config", "show", "--resolved"])
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Resolved Configuration:"));
}

#[test]
fn test_shell_completion_bash() {
    cli()
        .args(["--generate-completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_maki()"))
        .stdout(predicate::str::contains("complete -F"));
}

#[test]
fn test_shell_completion_zsh() {
    cli()
        .args(["--generate-completion", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_maki"));
}

#[test]
fn test_shell_completion_fish() {
    cli()
        .args(["--generate-completion", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"));
}

#[test]
fn test_verbose_output() {
    let temp_dir = create_test_project();
    cli()
        .args(["lint", temp_dir.path().to_str().unwrap(), "-v"])
        .assert()
        .success();
}

#[test]
fn test_fmt_help() {
    cli()
        .args(["fmt", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Format FSH files"))
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--diff"));
}

#[test]
fn test_fmt_command() {
    cli().args(["fmt", "."]).assert().success();
    // Formatting functionality is placeholder in this implementation
}

#[test]
fn test_invalid_command() {
    cli()
        .arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn test_invalid_option() {
    cli()
        .args(["lint", "--invalid-option"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}

#[test]
fn test_conflicting_options() {
    cli()
        .args(["lint", ".", "--write", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_exit_codes() {
    // Invalid argument case
    cli()
        .args(["lint", "--invalid"])
        .assert()
        .code(predicate::ne(0));
}

#[test]
fn test_large_number_of_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple FSH files
    for i in 0..10 {
        let content = format!(
            r#"Profile: TestProfile{i}
Parent: Patient
Id: test-profile-{i}
Title: "Test Profile {i}"
Description: "Test profile {i} for testing"
"#
        );
        fs::write(temp_dir.path().join(format!("test{i}.fsh")), content).unwrap();
    }

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_mixed_file_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create FSH file
    fs::write(
        temp_dir.path().join("test.fsh"),
        "Profile: Test\nParent: Patient\nId: test\nTitle: \"Test\"\nDescription: \"Test profile\"",
    )
    .unwrap();

    // Create non-FSH file
    fs::write(temp_dir.path().join("readme.txt"), "This is a readme").unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_nested_directories() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested directory structure
    let nested_dir = temp_dir.path().join("src").join("profiles");
    fs::create_dir_all(&nested_dir).unwrap();

    fs::write(
        nested_dir.join("patient.fsh"),
        "Profile: PatientProfile\nParent: Patient\nId: patient-profile\nTitle: \"Patient Profile\"\nDescription: \"Patient profile for testing\"",
    )
    .unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn test_symlinks() {
    use std::os::unix::fs as unix_fs;

    let temp_dir = TempDir::new().unwrap();

    // Create a file and a symlink to it (if supported by the OS)
    let original_file = temp_dir.path().join("original.fsh");
    fs::write(&original_file, "Profile: Original\nParent: Patient\nId: original\nTitle: \"Original\"\nDescription: \"Original profile\"").unwrap();

    // Try to create symlink (may fail on some systems)
    if unix_fs::symlink(&original_file, temp_dir.path().join("link.fsh")).is_ok() {
        cli()
            .args(["lint", temp_dir.path().to_str().unwrap()])
            .assert()
            .success();
    }
}

#[cfg(unix)]
#[test]
fn test_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("readonly.fsh");

    fs::write(&file_path, "Profile: ReadOnly\nParent: Patient\nId: readonly\nTitle: \"Read Only\"\nDescription: \"Read-only profile\"").unwrap();

    // Make file read-only
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o444);
    fs::set_permissions(&file_path, perms).unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_unicode_content() {
    let temp_dir = TempDir::new().unwrap();

    let unicode_content = r#"Profile: UnicodeProfile
Parent: Patient
Id: unicode-profile
Title: "Tëst Prøfîlé with Ünicødé"
Description: "A profile with unicode characters: 中文, العربية, русский"
"#;

    fs::write(temp_dir.path().join("unicode.fsh"), unicode_content).unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_very_long_lines() {
    let temp_dir = TempDir::new().unwrap();

    let long_line = "A".repeat(10000);
    let content = format!(
        r#"Profile: LongLineProfile
Parent: Patient
Id: long-line-profile
Title: "Long Line Profile"
Description: "{long_line}"
"#
    );

    fs::write(temp_dir.path().join("longline.fsh"), content).unwrap();

    cli()
        .args(["lint", temp_dir.path().to_str().unwrap()])
        .assert()
        .success();
}
