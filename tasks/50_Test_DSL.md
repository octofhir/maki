# Task 50: Test DSL and Framework

**Phase**: 5 (Testing Framework - Weeks 19-20)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Tasks 21 (Instance Exporter), 29 (FHIR Validation)

## Overview

Implement a testing framework for FSH projects that validates instances against profiles. This includes a YAML-based test DSL, test discovery, instance validation, assertion library, and detailed error reporting.

## Goals

1. **Test file format** - YAML-based test definitions
2. **Test discovery** - Find and load all .test.yaml files
3. **Instance validation** - Validate instances against profiles
4. **Assertion library** - Rich assertions for FHIR elements
5. **Test reporting** - Pass/fail results with detailed errors

## Technical Specification

### Test File Format

```yaml
# test/patient-example.test.yaml
name: "Patient Example Tests"
tests:
  - name: "Valid patient instance"
    instance: PatientExample
    profile: MyPatientProfile
    expect: valid

  - name: "Missing required field"
    instance: InvalidPatientExample
    profile: MyPatientProfile
    expect: invalid
    errors:
      - path: name
        message: "required element missing"

  - name: "Custom assertions"
    instance: PatientExample
    profile: MyPatientProfile
    assertions:
      - path: name.given
        equals: "John"
      - path: gender
        exists: true
      - path: birthDate
        matches: "\\d{4}-\\d{2}-\\d{2}"
```

### Test Runner Implementation

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<Test>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Test {
    pub name: String,
    pub instance: String,
    pub profile: String,
    #[serde(default)]
    pub expect: TestExpectation,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub errors: Vec<ExpectedError>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TestExpectation {
    Valid,
    Invalid,
}

impl Default for TestExpectation {
    fn default() -> Self {
        Self::Valid
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Assertion {
    pub path: String,
    #[serde(flatten)]
    pub kind: AssertionKind,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AssertionKind {
    Exists(bool),
    Equals(serde_json::Value),
    Matches(String),
    Cardinality { min: u32, max: Option<u32> },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExpectedError {
    pub path: String,
    pub message: String,
}

pub struct TestRunner {
    workspace: Workspace,
    validator: Box<dyn Validator>,
}

impl TestRunner {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            validator: Box::new(FhirValidator::new()),
        }
    }

    pub fn discover_tests(&self, root: &Path) -> Result<Vec<TestSuite>> {
        let mut suites = Vec::new();

        for entry in WalkDir::new(root) {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("test.yaml") {
                let content = fs::read_to_string(path)?;
                let suite: TestSuite = serde_yaml::from_str(&content)?;
                suites.push(suite);
            }
        }

        Ok(suites)
    }

    pub fn run_test(&self, test: &Test) -> TestResult {
        // Get instance from workspace
        let instance = match self.workspace.symbol_table.get(&test.instance) {
            Some(inst) => inst,
            None => {
                return TestResult {
                    test_name: test.name.clone(),
                    passed: false,
                    errors: vec![format!("Instance '{}' not found", test.instance)],
                };
            }
        };

        // Export instance to FHIR JSON
        let fhir_json = match self.export_instance(instance) {
            Ok(json) => json,
            Err(e) => {
                return TestResult {
                    test_name: test.name.clone(),
                    passed: false,
                    errors: vec![format!("Failed to export instance: {}", e)],
                };
            }
        };

        // Validate against profile
        let validation_result = self.validator.validate(&fhir_json, &test.profile);

        // Check expectation
        let expectation_met = match test.expect {
            TestExpectation::Valid => validation_result.is_valid(),
            TestExpectation::Invalid => !validation_result.is_valid(),
        };

        // Check assertions
        let mut assertion_errors = Vec::new();
        for assertion in &test.assertions {
            if let Err(e) = self.check_assertion(&fhir_json, assertion) {
                assertion_errors.push(e);
            }
        }

        // Check expected errors
        if !test.errors.is_empty() {
            let actual_errors = validation_result.errors();
            for expected_error in &test.errors {
                if !self.has_matching_error(&actual_errors, expected_error) {
                    assertion_errors.push(format!(
                        "Expected error not found: {} - {}",
                        expected_error.path, expected_error.message
                    ));
                }
            }
        }

        let passed = expectation_met && assertion_errors.is_empty();

        TestResult {
            test_name: test.name.clone(),
            passed,
            errors: if passed {
                vec![]
            } else {
                let mut errors = Vec::new();
                if !expectation_met {
                    errors.push(format!(
                        "Expected {:?} but validation {}",
                        test.expect,
                        if validation_result.is_valid() { "passed" } else { "failed" }
                    ));
                }
                errors.extend(assertion_errors);
                errors
            },
        }
    }

    fn check_assertion(
        &self,
        fhir_json: &serde_json::Value,
        assertion: &Assertion,
    ) -> Result<()> {
        let value = self.get_path_value(fhir_json, &assertion.path)?;

        match &assertion.kind {
            AssertionKind::Exists(should_exist) => {
                let exists = value.is_some();
                if exists != *should_exist {
                    return Err(anyhow!(
                        "Path '{}': expected exists={}, got exists={}",
                        assertion.path, should_exist, exists
                    ));
                }
            }
            AssertionKind::Equals(expected) => {
                let actual = value.ok_or_else(|| anyhow!("Path '{}' not found", assertion.path))?;
                if actual != expected {
                    return Err(anyhow!(
                        "Path '{}': expected {:?}, got {:?}",
                        assertion.path, expected, actual
                    ));
                }
            }
            AssertionKind::Matches(pattern) => {
                let actual = value.ok_or_else(|| anyhow!("Path '{}' not found", assertion.path))?;
                let regex = regex::Regex::new(pattern)?;
                let text = actual.as_str().unwrap_or("");
                if !regex.is_match(text) {
                    return Err(anyhow!(
                        "Path '{}': value '{}' does not match pattern '{}'",
                        assertion.path, text, pattern
                    ));
                }
            }
            AssertionKind::Cardinality { min, max } => {
                // Check array cardinality
                let array = value
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow!("Path '{}' is not an array", assertion.path))?;

                let len = array.len() as u32;
                if len < *min {
                    return Err(anyhow!(
                        "Path '{}': cardinality {} is less than minimum {}",
                        assertion.path, len, min
                    ));
                }
                if let Some(max_val) = max {
                    if len > *max_val {
                        return Err(anyhow!(
                            "Path '{}': cardinality {} exceeds maximum {}",
                            assertion.path, len, max_val
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn get_path_value<'a>(
        &self,
        json: &'a serde_json::Value,
        path: &str,
    ) -> Result<Option<&'a serde_json::Value>> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            current = match current.get(part) {
                Some(v) => v,
                None => return Ok(None),
            };
        }

        Ok(Some(current))
    }
}

#[derive(Debug)]
pub struct TestResult {
    pub test_name: String,
    pub passed: bool,
    pub errors: Vec<String>,
}
```

## Implementation Location

**Primary Crate**: `crates/maki-test/` (new crate)

**Files**:
- `src/test_suite.rs` - Test suite data structures
- `src/runner.rs` - Test runner implementation
- `src/assertions.rs` - Assertion checking
- `src/validator.rs` - FHIR validation wrapper

## Acceptance Criteria

- [ ] Test files are discovered recursively
- [ ] YAML test format parses correctly
- [ ] Instances validate against profiles
- [ ] All assertion types work (exists, equals, matches, cardinality)
- [ ] Expected errors are checked
- [ ] Test results show pass/fail
- [ ] Detailed error messages provided
- [ ] Unit tests cover all assertion types
- [ ] Integration tests verify end-to-end

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: Medium
**Updated**: 2025-11-03
