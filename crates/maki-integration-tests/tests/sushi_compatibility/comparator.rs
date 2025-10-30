//! JSON comparison logic for SUSHI compatibility testing
//!
//! This module provides utilities for comparing FHIR JSON outputs
//! from MAKI and SUSHI, identifying differences, and determining
//! if differences are acceptable.

use serde_json::Value;
use std::collections::HashMap;

/// Difference types between MAKI and SUSHI outputs
#[derive(Debug, Clone, PartialEq)]
pub enum Difference {
    /// File exists in SUSHI output but not in MAKI
    MissingInMaki(String),

    /// File exists in MAKI output but not in SUSHI
    MissingInSushi(String),

    /// Content differs between MAKI and SUSHI
    ContentDifference {
        file: String,
        path: String,
        maki_value: String,
        sushi_value: String,
    },

    /// Acceptable difference (e.g., timestamps, generator info)
    AcceptableDifference {
        file: String,
        path: String,
        reason: String,
    },
}

/// Compare two JSON values, returning list of differences
pub fn compare_json(file: &str, maki: &Value, sushi: &Value) -> Vec<Difference> {
    let mut differences = Vec::new();
    compare_json_recursive(file, "", maki, sushi, &mut differences);
    differences
}

fn compare_json_recursive(
    file: &str,
    path: &str,
    maki: &Value,
    sushi: &Value,
    differences: &mut Vec<Difference>,
) {
    match (maki, sushi) {
        (Value::Object(m), Value::Object(s)) => {
            // Check keys present in both
            for (key, maki_val) in m {
                let new_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                if let Some(sushi_val) = s.get(key) {
                    // Check if this is an acceptable difference
                    if is_acceptable_field(key) {
                        differences.push(Difference::AcceptableDifference {
                            file: file.to_string(),
                            path: new_path.clone(),
                            reason: get_acceptable_reason(key),
                        });
                    } else {
                        compare_json_recursive(file, &new_path, maki_val, sushi_val, differences);
                    }
                } else {
                    // Key in MAKI but not in SUSHI
                    differences.push(Difference::ContentDifference {
                        file: file.to_string(),
                        path: new_path,
                        maki_value: maki_val.to_string(),
                        sushi_value: "null".to_string(),
                    });
                }
            }

            // Check keys only in SUSHI
            for key in s.keys() {
                if !m.contains_key(key) {
                    let new_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    differences.push(Difference::ContentDifference {
                        file: file.to_string(),
                        path: new_path,
                        maki_value: "null".to_string(),
                        sushi_value: s[key].to_string(),
                    });
                }
            }
        }
        (Value::Array(m), Value::Array(s)) => {
            // For arrays, compare normalized versions (order may differ)
            if m.len() != s.len() {
                differences.push(Difference::ContentDifference {
                    file: file.to_string(),
                    path: format!("{}.length", path),
                    maki_value: m.len().to_string(),
                    sushi_value: s.len().to_string(),
                });
            }
            // TODO: Implement deep array comparison
        }
        _ => {
            // Primitive comparison
            if maki != sushi {
                differences.push(Difference::ContentDifference {
                    file: file.to_string(),
                    path: path.to_string(),
                    maki_value: maki.to_string(),
                    sushi_value: sushi.to_string(),
                });
            }
        }
    }
}

/// Check if a field is known to be acceptable to differ
fn is_acceptable_field(field: &str) -> bool {
    matches!(
        field,
        "date" | "publisher" | "version" | "generator" | "timestamp" | "_generatedBy"
    )
}

/// Get reason why a field difference is acceptable
fn get_acceptable_reason(field: &str) -> String {
    match field {
        "date" => "Generation timestamp differs".to_string(),
        "publisher" => "Generator information (MAKI vs SUSHI)".to_string(),
        "version" => "Tool version differs".to_string(),
        "generator" => "Generator name differs".to_string(),
        "timestamp" => "Generation timestamp differs".to_string(),
        "_generatedBy" => "Generator metadata differs".to_string(),
        _ => "Acceptable metadata difference".to_string(),
    }
}

/// Calculate compatibility percentage from differences
pub fn calculate_compatibility(differences: &[Difference]) -> f64 {
    let total = differences.len();
    if total == 0 {
        return 100.0;
    }

    let acceptable = differences
        .iter()
        .filter(|d| matches!(d, Difference::AcceptableDifference { .. }))
        .count();

    ((total - (total - acceptable)) as f64 / total as f64) * 100.0
}

/// Format differences for human-readable output
pub fn format_differences(differences: &[Difference]) -> String {
    let mut output = String::new();

    for diff in differences {
        match diff {
            Difference::MissingInMaki(file) => {
                output.push_str(&format!("❌ Missing in MAKI: {}\n", file));
            }
            Difference::MissingInSushi(file) => {
                output.push_str(&format!("⚠️  Extra in MAKI: {}\n", file));
            }
            Difference::ContentDifference {
                file,
                path,
                maki_value,
                sushi_value,
            } => {
                output.push_str(&format!(
                    "❌ {} @ {}\n  MAKI:  {}\n  SUSHI: {}\n",
                    file, path, maki_value, sushi_value
                ));
            }
            Difference::AcceptableDifference { file, path, reason } => {
                output.push_str(&format!("✅ {} @ {} ({})\n", file, path, reason));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_identical_json() {
        let json = json!({"resourceType": "Patient", "id": "example"});
        let diffs = compare_json("test.json", &json, &json);
        assert_eq!(diffs.len(), 0);
    }

    #[test]
    fn test_acceptable_difference() {
        let maki = json!({"resourceType": "Patient", "date": "2025-01-01"});
        let sushi = json!({"resourceType": "Patient", "date": "2025-01-02"});

        let diffs = compare_json("test.json", &maki, &sushi);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], Difference::AcceptableDifference { .. }));
    }

    #[test]
    fn test_content_difference() {
        let maki = json!({"resourceType": "Patient", "name": "John"});
        let sushi = json!({"resourceType": "Patient", "name": "Jane"});

        let diffs = compare_json("test.json", &maki, &sushi);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], Difference::ContentDifference { .. }));
    }

    #[test]
    fn test_compatibility_calculation() {
        let diffs = vec![
            Difference::AcceptableDifference {
                file: "test.json".to_string(),
                path: "date".to_string(),
                reason: "timestamp".to_string(),
            },
            Difference::ContentDifference {
                file: "test.json".to_string(),
                path: "name".to_string(),
                maki_value: "John".to_string(),
                sushi_value: "Jane".to_string(),
            },
        ];

        let compat = calculate_compatibility(&diffs);
        assert!(compat < 100.0);
    }
}
