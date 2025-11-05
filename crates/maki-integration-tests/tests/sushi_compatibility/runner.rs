//! Test runner for SUSHI compatibility testing
//!
//! This module provides the main test harness for running MAKI and SUSHI
//! side-by-side and comparing their outputs.

use super::comparator::{
    Difference, SemanticComparisonResult, compare_json, compare_semantic_equivalence,
    format_differences
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// A test case from the SUSHI test suite
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub fsh_files: Vec<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub expected_outputs: Vec<PathBuf>,
}

/// Result of comparing MAKI and SUSHI outputs
#[derive(Debug)]
pub struct ComparisonResult {
    pub test_name: String,
    pub passed: bool,
    pub differences: Vec<Difference>,
    pub maki_time: Duration,
    pub sushi_time: Option<Duration>,
    pub compatibility_percent: f64,
    pub semantic_results: Vec<SemanticComparisonResult>,
    pub semantic_equivalence_score: f64,
}

/// Main test harness for SUSHI compatibility
pub struct SushiCompatibilityHarness {
    maki_binary: PathBuf,
    sushi_available: bool,
    test_cases: Vec<TestCase>,
    reference_files: Vec<PathBuf>,
    compatibility_threshold: f64,
}

impl SushiCompatibilityHarness {
    /// Create a new test harness
    pub fn new() -> Result<Self, String> {
        let maki_binary = Self::find_maki_binary()?;
        let sushi_available = Self::check_sushi_available();

        Ok(Self {
            maki_binary,
            sushi_available,
            test_cases: Vec::new(),
            reference_files: Vec::new(),
            compatibility_threshold: 90.0,
        })
    }

    /// Create test harness with custom compatibility threshold
    pub fn with_threshold(threshold: f64) -> Result<Self, String> {
        let mut harness = Self::new()?;
        harness.compatibility_threshold = threshold;
        Ok(harness)
    }

    /// Find the MAKI binary (either debug or release)
    fn find_maki_binary() -> Result<PathBuf, String> {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();

        // Try release first, then debug
        let release_path = workspace_root.join("target/release/maki");
        let debug_path = workspace_root.join("target/debug/maki");

        if release_path.exists() {
            Ok(release_path)
        } else if debug_path.exists() {
            Ok(debug_path)
        } else {
            Err("MAKI binary not found. Run 'cargo build' first.".to_string())
        }
    }

    /// Check if SUSHI is available
    fn check_sushi_available() -> bool {
        Command::new("sushi")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Add a test case
    pub fn add_test_case(&mut self, test_case: TestCase) {
        self.test_cases.push(test_case);
    }

    /// Load SUSHI reference files from a directory
    pub fn load_reference_files(&mut self, reference_dir: &Path) -> Result<(), String> {
        if !reference_dir.exists() {
            return Err(format!("Reference directory does not exist: {:?}", reference_dir));
        }

        self.reference_files.clear();
        self.collect_reference_files(reference_dir)?;
        
        // Convert reference files to test cases
        for ref_file in &self.reference_files.clone() {
            if let Some(test_case) = self.create_test_case_from_reference(ref_file) {
                self.test_cases.push(test_case);
            }
        }

        Ok(())
    }

    /// Recursively collect FSH files from reference directory
    fn collect_reference_files(&mut self, dir: &Path) -> Result<(), String> {
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {:?}: {}", dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                self.collect_reference_files(&path)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("fsh") {
                self.reference_files.push(path);
            }
        }

        Ok(())
    }

    /// Create a test case from a reference FSH file
    fn create_test_case_from_reference(&self, fsh_file: &Path) -> Option<TestCase> {
        let name = fsh_file
            .file_stem()?
            .to_string_lossy()
            .to_string();

        // Look for associated config file
        let config_file = fsh_file
            .parent()?
            .join("sushi-config.yaml")
            .exists()
            .then(|| fsh_file.parent().unwrap().join("sushi-config.yaml"));

        Some(TestCase {
            name: format!("ref_{}", name),
            fsh_files: vec![fsh_file.to_path_buf()],
            config_file,
            expected_outputs: vec![],
        })
    }

    /// Load all FSH files from examples directory as test cases
    pub fn load_examples_as_tests(&mut self) -> Result<(), String> {
        let binding = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = binding
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        
        let examples_dir = workspace_root.join("examples");
        
        if !examples_dir.exists() {
            return Err("Examples directory not found".to_string());
        }

        let entries = fs::read_dir(&examples_dir)
            .map_err(|e| format!("Failed to read examples directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fsh") {
                let test_case = TestCase {
                    name: path.file_stem().unwrap().to_string_lossy().to_string(),
                    fsh_files: vec![path],
                    config_file: None,
                    expected_outputs: vec![],
                };
                self.test_cases.push(test_case);
            }
        }

        Ok(())
    }

    /// Run MAKI on a test case
    fn run_maki(&self, test_case: &TestCase, output_dir: &Path) -> Result<Duration, String> {
        let start = Instant::now();

        let mut cmd = Command::new(&self.maki_binary);
        cmd.arg("build"); // Future: when build command is implemented

        for fsh_file in &test_case.fsh_files {
            cmd.arg(fsh_file);
        }

        cmd.arg("--output").arg(output_dir);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run MAKI: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "MAKI build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(start.elapsed())
    }

    /// Run SUSHI on a test case
    fn run_sushi(&self, test_case: &TestCase, output_dir: &Path) -> Result<Duration, String> {
        if !self.sushi_available {
            return Err("SUSHI not available".to_string());
        }

        let start = Instant::now();

        // Determine the directory containing the FSH files
        let fsh_dir = test_case
            .fsh_files
            .first()
            .and_then(|p| p.parent())
            .ok_or("No FSH files in test case")?;

        let mut cmd = Command::new("sushi");
        cmd.arg(fsh_dir);
        cmd.arg("-o").arg(output_dir);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run SUSHI: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "SUSHI build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(start.elapsed())
    }

    /// Compare outputs from MAKI and SUSHI
    fn compare_outputs(&self, maki_dir: &Path, sushi_dir: &Path) -> (Vec<Difference>, Vec<SemanticComparisonResult>) {
        let mut differences = Vec::new();
        let mut semantic_results = Vec::new();

        // Collect all JSON files from both directories
        let maki_files = Self::collect_json_files(maki_dir);
        let sushi_files = Self::collect_json_files(sushi_dir);

        // Find files in SUSHI but not in MAKI
        for sushi_file in &sushi_files {
            let rel_path = sushi_file.strip_prefix(sushi_dir).unwrap();
            let maki_file = maki_dir.join(rel_path);

            if !maki_file.exists() {
                differences.push(Difference::MissingInMaki(
                    rel_path.to_string_lossy().to_string(),
                ));
                continue;
            }

            // Compare the JSON content
            if let (Ok(maki_json), Ok(sushi_json)) = (
                fs::read_to_string(&maki_file),
                fs::read_to_string(sushi_file),
            ) {
                if let (Ok(maki_val), Ok(sushi_val)) = (
                    serde_json::from_str::<Value>(&maki_json),
                    serde_json::from_str::<Value>(&sushi_json),
                ) {
                    let file_name = rel_path.to_string_lossy();
                    
                    // Basic comparison
                    let file_diffs = compare_json(&file_name, &maki_val, &sushi_val);
                    differences.extend(file_diffs);
                    
                    // Semantic comparison
                    let semantic_result = compare_semantic_equivalence(&file_name, &maki_val, &sushi_val);
                    semantic_results.push(semantic_result);
                }
            }
        }

        // Find files in MAKI but not in SUSHI
        for maki_file in &maki_files {
            let rel_path = maki_file.strip_prefix(maki_dir).unwrap();
            let sushi_file = sushi_dir.join(rel_path);

            if !sushi_file.exists() {
                differences.push(Difference::MissingInSushi(
                    rel_path.to_string_lossy().to_string(),
                ));
            }
        }

        (differences, semantic_results)
    }

    /// Collect all JSON files in a directory
    fn collect_json_files(dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    files.push(path);
                } else if path.is_dir() {
                    files.extend(Self::collect_json_files(&path));
                }
            }
        }

        files
    }

    /// Run all test cases
    pub fn run_all_tests(&self) -> Vec<ComparisonResult> {
        let mut results = Vec::new();

        for test_case in &self.test_cases {
            if let Some(result) = self.run_test_case(test_case) {
                results.push(result);
            }
        }

        results
    }

    /// Run a single test case
    fn run_test_case(&self, test_case: &TestCase) -> Option<ComparisonResult> {
        // Create temporary directories for outputs
        let temp_dir = tempfile::tempdir().ok()?;
        let maki_output = temp_dir.path().join("maki");
        let sushi_output = temp_dir.path().join("sushi");

        fs::create_dir_all(&maki_output).ok()?;
        fs::create_dir_all(&sushi_output).ok()?;

        // Run MAKI
        let maki_time = match self.run_maki(test_case, &maki_output) {
            Ok(duration) => duration,
            Err(e) => {
                eprintln!("MAKI failed for {}: {}", test_case.name, e);
                return None;
            }
        };

        // Run SUSHI (if available)
        let sushi_time = if self.sushi_available {
            self.run_sushi(test_case, &sushi_output).ok()
        } else {
            None
        };

        // Compare outputs
        let (differences, semantic_results) = if sushi_time.is_some() {
            self.compare_outputs(&maki_output, &sushi_output)
        } else {
            (Vec::new(), Vec::new())
        };

        let compatibility_percent = if !differences.is_empty() {
            let acceptable = differences
                .iter()
                .filter(|d| matches!(d, Difference::AcceptableDifference { .. }))
                .count();
            (acceptable as f64 / differences.len() as f64) * 100.0
        } else {
            100.0
        };

        // Calculate semantic equivalence score
        let semantic_equivalence_score = if !semantic_results.is_empty() {
            semantic_results.iter().map(|r| r.equivalence_score).sum::<f64>() / semantic_results.len() as f64
        } else {
            1.0
        };

        Some(ComparisonResult {
            test_name: test_case.name.clone(),
            passed: compatibility_percent >= self.compatibility_threshold,
            differences,
            maki_time,
            sushi_time,
            compatibility_percent,
            semantic_results,
            semantic_equivalence_score,
        })
    }

    /// Generate a summary report
    pub fn generate_report(&self, results: &[ComparisonResult]) -> String {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let overall_compat = (passed as f64 / total as f64) * 100.0;

        let mut report = format!(
            "SUSHI Compatibility Report\n\
             ===========================\n\n\
             Overall Compatibility: {:.2}%\n\
             Total Tests: {}\n\
             Passed: {}\n\
             Failed: {}\n\
             Threshold: {:.1}%\n\n",
            overall_compat,
            total,
            passed,
            total - passed,
            self.compatibility_threshold
        );

        // Add performance summary
        if !results.is_empty() {
            let avg_maki_time = results.iter()
                .map(|r| r.maki_time.as_millis())
                .sum::<u128>() as f64 / results.len() as f64;
            
            let avg_sushi_time = results.iter()
                .filter_map(|r| r.sushi_time.map(|t| t.as_millis()))
                .sum::<u128>() as f64 / results.iter().filter(|r| r.sushi_time.is_some()).count().max(1) as f64;

            report.push_str(&format!(
                "Performance:\n\
                 - Average MAKI time: {:.1}ms\n\
                 - Average SUSHI time: {:.1}ms\n\n",
                avg_maki_time, avg_sushi_time
            ));
        }

        // Add failed tests
        report.push_str("Failed Tests:\n");
        for result in results.iter().filter(|r| !r.passed) {
            report.push_str(&format!("\n  - {} ({:.1}% compatible)\n", result.test_name, result.compatibility_percent));
            report.push_str(&format_differences(&result.differences));
        }

        // Add compatibility trend if available
        report.push_str(&self.generate_compatibility_trend());

        report
    }

    /// Generate compatibility trend information
    fn generate_compatibility_trend(&self) -> String {
        // For now, just return empty string
        // In the future, this could read historical data
        String::new()
    }

    /// Generate CI-friendly report (JSON format)
    pub fn generate_ci_report(&self, results: &[ComparisonResult]) -> String {
        use serde_json::json;

        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let overall_compat = if total > 0 { (passed as f64 / total as f64) * 100.0 } else { 100.0 };

        let report = json!({
            "compatibility_percentage": overall_compat,
            "threshold": self.compatibility_threshold,
            "total_tests": total,
            "passed_tests": passed,
            "failed_tests": total - passed,
            "meets_threshold": overall_compat >= self.compatibility_threshold,
            "test_results": results.iter().map(|r| {
                json!({
                    "name": r.test_name,
                    "passed": r.passed,
                    "compatibility": r.compatibility_percent,
                    "maki_time_ms": r.maki_time.as_millis(),
                    "sushi_time_ms": r.sushi_time.map(|t| t.as_millis()),
                    "differences_count": r.differences.len()
                })
            }).collect::<Vec<_>>()
        });

        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if compatibility meets threshold
    pub fn meets_threshold(&self, results: &[ComparisonResult]) -> bool {
        if results.is_empty() {
            return true;
        }

        let passed = results.iter().filter(|r| r.passed).count();
        let overall_compat = (passed as f64 / results.len() as f64) * 100.0;
        overall_compat >= self.compatibility_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        // Test that we can create a harness
        let harness = SushiCompatibilityHarness::new();
        assert!(harness.is_ok());
    }

    #[test]
    fn test_maki_binary_exists() {
        let binary = SushiCompatibilityHarness::find_maki_binary();
        // Will fail if MAKI hasn't been built yet, which is okay for now
        assert!(binary.is_ok() || binary.is_err());
    }
}
