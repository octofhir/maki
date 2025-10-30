//! SUSHI Parity Testing
//!
//! Tests maki's output against SUSHI's test suite for compatibility verification.
//! This module runs maki against SUSHI's test fixtures and compares the output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Result of a single parity test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityTestResult {
    /// Test name/identifier
    pub test_name: String,
    /// Whether the test passed
    pub passed: bool,
    /// SUSHI output summary
    pub sushi_output: OutputSummary,
    /// Maki output summary
    pub maki_output: OutputSummary,
    /// Differences found
    pub differences: Vec<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// Summary of build output
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputSummary {
    /// Number of profiles generated
    pub profiles: usize,
    /// Number of extensions generated
    pub extensions: usize,
    /// Number of value sets generated
    pub value_sets: usize,
    /// Number of code systems generated
    pub code_systems: usize,
    /// Number of instances generated
    pub instances: usize,
    /// Number of logicals generated
    pub logicals: usize,
    /// Number of errors
    pub errors: usize,
    /// Number of warnings
    pub warnings: usize,
    /// Build succeeded
    pub success: bool,
}

/// Parity test report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityReport {
    /// Total tests run
    pub total_tests: usize,
    /// Tests passed
    pub passed_tests: usize,
    /// Tests failed
    pub failed_tests: usize,
    /// Compatibility percentage
    pub compatibility_percent: f64,
    /// Individual test results
    pub test_results: Vec<ParityTestResult>,
    /// Summary of differences by category
    pub difference_categories: HashMap<String, usize>,
    /// Execution timestamp
    pub timestamp: String,
    /// SUSHI version tested against
    pub sushi_version: String,
    /// Maki version
    pub maki_version: String,
}

impl ParityReport {
    /// Create a new parity report
    pub fn new(sushi_version: String, maki_version: String) -> Self {
        Self {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            compatibility_percent: 0.0,
            test_results: Vec::new(),
            difference_categories: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            sushi_version,
            maki_version,
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: ParityTestResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }

        // Categorize differences
        for diff in &result.differences {
            if diff.contains("profile") {
                *self.difference_categories.entry("profiles".to_string()).or_insert(0) += 1;
            } else if diff.contains("extension") {
                *self.difference_categories.entry("extensions".to_string()).or_insert(0) += 1;
            } else if diff.contains("valueset") || diff.contains("value set") {
                *self.difference_categories.entry("valuesets".to_string()).or_insert(0) += 1;
            } else if diff.contains("codesystem") || diff.contains("code system") {
                *self.difference_categories.entry("codesystems".to_string()).or_insert(0) += 1;
            } else if diff.contains("instance") {
                *self.difference_categories.entry("instances".to_string()).or_insert(0) += 1;
            } else {
                *self.difference_categories.entry("other".to_string()).or_insert(0) += 1;
            }
        }

        self.test_results.push(result);
        self.update_compatibility();
    }

    /// Update compatibility percentage
    fn update_compatibility(&mut self) {
        if self.total_tests > 0 {
            self.compatibility_percent = (self.passed_tests as f64 / self.total_tests as f64) * 100.0;
        }
    }

    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# SUSHI Parity Test Report\n\n");
        md.push_str(&format!("**Generated**: {}\n\n", self.timestamp));
        md.push_str(&format!("**SUSHI Version**: {}\n", self.sushi_version));
        md.push_str(&format!("**Maki Version**: {}\n\n", self.maki_version));

        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total Tests**: {}\n", self.total_tests));
        md.push_str(&format!("- **Passed**: {} ✅\n", self.passed_tests));
        md.push_str(&format!("- **Failed**: {} ❌\n", self.failed_tests));
        md.push_str(&format!("- **Compatibility**: {:.2}%\n\n", self.compatibility_percent));

        if !self.difference_categories.is_empty() {
            md.push_str("## Differences by Category\n\n");
            let mut categories: Vec<_> = self.difference_categories.iter().collect();
            categories.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
            for (category, count) in categories {
                md.push_str(&format!("- **{}**: {} differences\n", category, count));
            }
            md.push_str("\n");
        }

        md.push_str("## Test Results\n\n");
        md.push_str("| Test | Status | Profiles | Extensions | ValueSets | CodeSystems | Instances |\n");
        md.push_str("|------|--------|----------|------------|-----------|-------------|----------|\n");

        for result in &self.test_results {
            let status = if result.passed { "✅ PASS" } else { "❌ FAIL" };
            let profiles_match = result.sushi_output.profiles == result.maki_output.profiles;
            let extensions_match = result.sushi_output.extensions == result.maki_output.extensions;
            let valuesets_match = result.sushi_output.value_sets == result.maki_output.value_sets;
            let codesystems_match = result.sushi_output.code_systems == result.maki_output.code_systems;
            let instances_match = result.sushi_output.instances == result.maki_output.instances;

            md.push_str(&format!(
                "| {} | {} | {}{}  | {}{}  | {}{}  | {}{}  | {}{}  |\n",
                result.test_name,
                status,
                result.maki_output.profiles,
                if profiles_match { "" } else { " ⚠️" },
                result.maki_output.extensions,
                if extensions_match { "" } else { " ⚠️" },
                result.maki_output.value_sets,
                if valuesets_match { "" } else { " ⚠️" },
                result.maki_output.code_systems,
                if codesystems_match { "" } else { " ⚠️" },
                result.maki_output.instances,
                if instances_match { "" } else { " ⚠️" },
            ));
        }

        md.push_str("\n## Failed Tests Detail\n\n");
        for result in &self.test_results {
            if !result.passed {
                md.push_str(&format!("### {}\n\n", result.test_name));
                md.push_str("**Differences:**\n");
                for diff in &result.differences {
                    md.push_str(&format!("- {}\n", diff));
                }
                md.push_str("\n");
            }
        }

        md
    }

    /// Save report to files
    pub fn save(&self, output_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(output_dir)?;

        // Save JSON report
        let json_path = output_dir.join("parity_report.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(json_path, json)?;

        // Save markdown report
        let md_path = output_dir.join("parity_report.md");
        std::fs::write(md_path, self.to_markdown())?;

        Ok(())
    }
}

/// Parity test runner
pub struct ParityTestRunner {
    /// Path to SUSHI executable
    sushi_path: PathBuf,
    /// Path to Maki executable
    maki_path: PathBuf,
    /// Path to SUSHI test fixtures
    fixtures_path: PathBuf,
}

impl ParityTestRunner {
    /// Create a new parity test runner
    pub fn new(
        sushi_path: impl Into<PathBuf>,
        maki_path: impl Into<PathBuf>,
        fixtures_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            sushi_path: sushi_path.into(),
            maki_path: maki_path.into(),
            fixtures_path: fixtures_path.into(),
        }
    }

    /// Run all parity tests
    pub fn run_all(&self) -> anyhow::Result<ParityReport> {
        let sushi_version = self.get_sushi_version()?;
        let maki_version = env!("CARGO_PKG_VERSION").to_string();

        let mut report = ParityReport::new(sushi_version, maki_version);

        // Get all test fixtures
        let fixtures = self.discover_fixtures()?;
        println!("Found {} test fixtures", fixtures.len());

        for (i, fixture) in fixtures.iter().enumerate() {
            println!("Running test {}/{}: {}...", i + 1, fixtures.len(), fixture.display());
            match self.run_test(fixture) {
                Ok(result) => {
                    let status = if result.passed { "✅" } else { "❌" };
                    println!("  {} {}", status, result.test_name);
                    report.add_result(result);
                }
                Err(e) => {
                    eprintln!("  ❌ Error running test: {}", e);
                    // Add as failed test
                    report.add_result(ParityTestResult {
                        test_name: fixture.file_name().unwrap().to_string_lossy().to_string(),
                        passed: false,
                        sushi_output: OutputSummary::default(),
                        maki_output: OutputSummary::default(),
                        differences: vec![format!("Test execution error: {}", e)],
                        duration_ms: 0,
                    });
                }
            }
        }

        Ok(report)
    }

    /// Run a single test
    fn run_test(&self, fixture_path: &Path) -> anyhow::Result<ParityTestResult> {
        let start = std::time::Instant::now();

        // Create temporary directories for outputs
        let sushi_temp = TempDir::new()?;
        let maki_temp = TempDir::new()?;

        // Run SUSHI
        let sushi_output = self.run_sushi(fixture_path, sushi_temp.path())?;

        // Run Maki
        let maki_output = self.run_maki(fixture_path, maki_temp.path())?;

        // Compare outputs
        let differences = self.compare_outputs(sushi_temp.path(), maki_temp.path())?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let passed = differences.is_empty()
            && sushi_output.success == maki_output.success
            && sushi_output.profiles == maki_output.profiles
            && sushi_output.extensions == maki_output.extensions
            && sushi_output.value_sets == maki_output.value_sets
            && sushi_output.code_systems == maki_output.code_systems
            && sushi_output.instances == maki_output.instances;

        Ok(ParityTestResult {
            test_name: fixture_path.file_name().unwrap().to_string_lossy().to_string(),
            passed,
            sushi_output,
            maki_output,
            differences,
            duration_ms,
        })
    }

    /// Run SUSHI on a fixture
    fn run_sushi(&self, fixture_path: &Path, output_dir: &Path) -> anyhow::Result<OutputSummary> {
        let output = Command::new(&self.sushi_path)
            .arg(fixture_path)
            .arg("--out")
            .arg(output_dir)
            .output()?;

        self.parse_sushi_output(&output.stdout, &output.stderr, output.status.success())
    }

    /// Run Maki on a fixture
    fn run_maki(&self, fixture_path: &Path, output_dir: &Path) -> anyhow::Result<OutputSummary> {
        let output = Command::new(&self.maki_path)
            .arg("build")
            .arg(fixture_path)
            .arg("--output")
            .arg(output_dir)
            .output()?;

        self.parse_maki_output(&output.stdout, &output.stderr, output.status.success())
    }

    /// Parse SUSHI output to extract statistics
    fn parse_sushi_output(&self, stdout: &[u8], stderr: &[u8], success: bool) -> anyhow::Result<OutputSummary> {
        let output_str = String::from_utf8_lossy(stdout);
        let mut summary = OutputSummary {
            success,
            ..Default::default()
        };

        // Parse SUSHI's output format
        // Example: "5 Profiles, 3 Extensions, 2 ValueSets, 1 CodeSystem, 10 Instances"
        for line in output_str.lines() {
            if line.contains("Profile") {
                if let Some(count) = Self::extract_number_before_word(line, "Profile") {
                    summary.profiles = count;
                }
            }
            if line.contains("Extension") {
                if let Some(count) = Self::extract_number_before_word(line, "Extension") {
                    summary.extensions = count;
                }
            }
            if line.contains("ValueSet") {
                if let Some(count) = Self::extract_number_before_word(line, "ValueSet") {
                    summary.value_sets = count;
                }
            }
            if line.contains("CodeSystem") {
                if let Some(count) = Self::extract_number_before_word(line, "CodeSystem") {
                    summary.code_systems = count;
                }
            }
            if line.contains("Instance") {
                if let Some(count) = Self::extract_number_before_word(line, "Instance") {
                    summary.instances = count;
                }
            }
            if line.contains("Error") {
                if let Some(count) = Self::extract_number_before_word(line, "Error") {
                    summary.errors = count;
                }
            }
            if line.contains("Warning") {
                if let Some(count) = Self::extract_number_before_word(line, "Warning") {
                    summary.warnings = count;
                }
            }
        }

        Ok(summary)
    }

    /// Parse Maki output to extract statistics
    fn parse_maki_output(&self, stdout: &[u8], stderr: &[u8], success: bool) -> anyhow::Result<OutputSummary> {
        let output_str = String::from_utf8_lossy(stdout);
        let mut summary = OutputSummary {
            success,
            ..Default::default()
        };

        // Parse Maki's output format (from build command)
        for line in output_str.lines() {
            if line.contains("Profiles") || line.contains("Extensions") || line.contains("Logicals") {
                // Parse table row
                let parts: Vec<&str> = line.split('│').collect();
                if parts.len() >= 4 {
                    if let Ok(profiles) = parts[1].trim().parse::<usize>() {
                        summary.profiles = profiles;
                    }
                    if let Ok(extensions) = parts[2].trim().parse::<usize>() {
                        summary.extensions = extensions;
                    }
                    if let Ok(logicals) = parts[3].trim().parse::<usize>() {
                        summary.logicals = logicals;
                    }
                }
            }
            if line.contains("ValueSets") || line.contains("CodeSystems") || line.contains("Instances") {
                let parts: Vec<&str> = line.split('│').collect();
                if parts.len() >= 4 {
                    if let Ok(valuesets) = parts[1].trim().parse::<usize>() {
                        summary.value_sets = valuesets;
                    }
                    if let Ok(codesystems) = parts[2].trim().parse::<usize>() {
                        summary.code_systems = codesystems;
                    }
                    if let Ok(instances) = parts[3].trim().parse::<usize>() {
                        summary.instances = instances;
                    }
                }
            }
            if line.contains("error") {
                if let Some(count) = Self::extract_number_before_word(line, "error") {
                    summary.errors = count;
                }
            }
            if line.contains("warning") {
                if let Some(count) = Self::extract_number_before_word(line, "warning") {
                    summary.warnings = count;
                }
            }
        }

        Ok(summary)
    }

    /// Extract a number that appears before a given word in a string
    fn extract_number_before_word(text: &str, word: &str) -> Option<usize> {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|w| w[1].to_lowercase().starts_with(&word.to_lowercase()))
            .and_then(|w| w[0].parse::<usize>().ok())
    }

    /// Compare outputs between SUSHI and Maki
    fn compare_outputs(&self, sushi_dir: &Path, maki_dir: &Path) -> anyhow::Result<Vec<String>> {
        let mut differences = Vec::new();

        // Compare generated files
        let sushi_files = self.collect_generated_files(sushi_dir)?;
        let maki_files = self.collect_generated_files(maki_dir)?;

        // Check for missing files
        for file in &sushi_files {
            if !maki_files.contains(file) {
                differences.push(format!("Missing file in Maki output: {}", file));
            }
        }

        for file in &maki_files {
            if !sushi_files.contains(file) {
                differences.push(format!("Extra file in Maki output: {}", file));
            }
        }

        // TODO: Deep comparison of JSON content

        Ok(differences)
    }

    /// Collect all generated files in a directory
    fn collect_generated_files(&self, dir: &Path) -> anyhow::Result<Vec<String>> {
        let mut files = Vec::new();
        if dir.exists() {
            for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Ok(relative) = entry.path().strip_prefix(dir) {
                        files.push(relative.display().to_string());
                    }
                }
            }
        }
        Ok(files)
    }

    /// Discover all test fixtures
    fn discover_fixtures(&self) -> anyhow::Result<Vec<PathBuf>> {
        let mut fixtures = Vec::new();
        for entry in walkdir::WalkDir::new(&self.fixtures_path)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() && entry.path() != self.fixtures_path {
                // Check if it has a sushi-config.yaml
                let config_path = entry.path().join("sushi-config.yaml");
                if config_path.exists() {
                    fixtures.push(entry.path().to_path_buf());
                }
            }
        }
        Ok(fixtures)
    }

    /// Get SUSHI version
    fn get_sushi_version(&self) -> anyhow::Result<String> {
        let output = Command::new(&self.sushi_path).arg("--version").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number_before_word() {
        assert_eq!(
            ParityTestRunner::extract_number_before_word("5 Profiles generated", "Profile"),
            Some(5)
        );
        assert_eq!(
            ParityTestRunner::extract_number_before_word("Created 10 instances", "instances"),
            Some(10)
        );
        assert_eq!(
            ParityTestRunner::extract_number_before_word("0 errors", "error"),
            Some(0)
        );
    }

    #[test]
    fn test_parity_report_compatibility() {
        let mut report = ParityReport::new("3.0.0".to_string(), "0.0.2".to_string());

        report.add_result(ParityTestResult {
            test_name: "test1".to_string(),
            passed: true,
            sushi_output: OutputSummary::default(),
            maki_output: OutputSummary::default(),
            differences: vec![],
            duration_ms: 100,
        });

        report.add_result(ParityTestResult {
            test_name: "test2".to_string(),
            passed: false,
            sushi_output: OutputSummary::default(),
            maki_output: OutputSummary::default(),
            differences: vec!["Profile count mismatch".to_string()],
            duration_ms: 150,
        });

        assert_eq!(report.total_tests, 2);
        assert_eq!(report.passed_tests, 1);
        assert_eq!(report.failed_tests, 1);
        assert_eq!(report.compatibility_percent, 50.0);
    }
}
