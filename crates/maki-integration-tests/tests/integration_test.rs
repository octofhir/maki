use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test full workflow: init config, create FSH file, run lint
#[test]
fn test_full_workflow() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    // 1. Initialize config
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("config")
        .arg("init")
        .assert()
        .success();

    assert!(project_dir.join(".makirc.json").exists());

    // 2. Create test FSH file
    let fsh_content = r#"Profile: TestProfile
Parent: Patient
Id: test-profile
Title: "Test Profile"
Description: "Test profile for integration testing"
* name 1..1 MS
* birthDate 0..1
"#;

    fs::write(project_dir.join("test.fsh"), fsh_content).unwrap();

    // 3. Run lint
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("lint")
        .arg("test.fsh")
        .assert()
        .success();
}

// TODO: Re-add test_lint_with_errors - removed temporarily

/// Test listing rules
#[test]
fn test_rules_command() {
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.arg("rules")
        .assert()
        .success()
        .stdout(predicate::str::contains("documentation"));
}

/// Test detailed rules listing
#[test]
fn test_rules_detailed() {
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.arg("rules")
        .arg("--detailed")
        .assert()
        .success()
        .stdout(predicate::str::contains("Category"));
}

/// Test rules filtering by category
#[test]
fn test_rules_category_filter() {
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.arg("rules")
        .arg("--category")
        .arg("documentation")
        .assert()
        .success();
}

/// Test config init command
#[test]
fn test_config_init() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("config")
        .arg("init")
        .assert()
        .success();

    let config_path = project_dir.join(".makirc.json");
    assert!(config_path.exists());

    // Verify it's valid JSON
    let config_content = fs::read_to_string(config_path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&config_content).unwrap();
}

// TODO: Re-add test_config_init_jsonc - removed temporarily

// TODO: Re-add test_lint_with_fix - removed temporarily

/// Test linting multiple files
#[test]
fn test_lint_multiple_files() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    // Create multiple FSH files
    for i in 1..=3 {
        let content = format!(
            r#"Profile: TestProfile{i}
Parent: Patient
Id: test-profile-{i}
Title: "Test Profile {i}"
Description: "Test profile {i}"
"#
        );
        fs::write(project_dir.join(format!("test{i}.fsh")), content).unwrap();
    }

    // Lint all files
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("lint")
        .arg("test1.fsh")
        .arg("test2.fsh")
        .arg("test3.fsh")
        .assert()
        .success();
}

// TODO: Re-add test_json_output - removed temporarily

/// Test help command
#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

/// Test version command
#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("maki"));
}

/// Test linting directory
#[test]
fn test_lint_directory() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();
    let fsh_dir = project_dir.join("fsh");
    fs::create_dir(&fsh_dir).unwrap();

    // Create FSH files in directory
    for i in 1..=2 {
        let content = format!(
            r#"Profile: TestProfile{i}
Parent: Patient
Id: test-profile-{i}
Title: "Test Profile {i}"
Description: "Test"
"#
        );
        fs::write(fsh_dir.join(format!("test{i}.fsh")), content).unwrap();
    }

    // Lint directory
    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("lint")
        .arg("fsh/")
        .assert()
        .success();
}

// TODO: Re-add test_config_validate - removed temporarily

/// Test that invalid config is detected
#[test]
fn test_invalid_config() {
    let temp = TempDir::new().unwrap();
    let project_dir = temp.path();

    // Create invalid config (malformed JSON)
    let config = r#"{
  "rules": {
    "documentation/require-description": "error"
  # Missing closing brace
"#;
    fs::write(project_dir.join(".makirc.json"), config).unwrap();

    let mut cmd = Command::cargo_bin("maki").unwrap();
    cmd.current_dir(project_dir)
        .arg("lint")
        .arg(".")
        .assert()
        .failure();
}
