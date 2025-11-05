//! JSON comparison logic for SUSHI compatibility testing
//!
//! This module provides utilities for comparing FHIR JSON outputs
//! from MAKI and SUSHI, identifying differences, and determining
//! if differences are acceptable.

use serde_json::Value;

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

/// Semantic comparison result
#[derive(Debug, Clone)]
pub struct SemanticComparisonResult {
    pub file: String,
    pub is_equivalent: bool,
    pub equivalence_score: f64,
    pub semantic_issues: Vec<SemanticIssue>,
    pub raw_differences: Vec<Difference>,
}

/// Semantic issue found during comparison
#[derive(Debug, Clone)]
pub struct SemanticIssue {
    pub path: String,
    pub issue_type: SemanticIssueType,
    pub description: String,
    pub severity: SemanticSeverity,
}

/// Types of semantic issues
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticIssueType {
    ContentMismatch,
    MissingResource,
    ExtraResource,
    StructuralDifference,
    TypeMismatch,
    CardinalityMismatch,
}

/// Severity levels for semantic issues
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticSeverity {
    Low,    // Minor differences that don't affect functionality
    Medium, // Differences that might affect some use cases
    High,   // Critical differences that affect core functionality
}

/// Compare two JSON values, returning list of differences
pub fn compare_json(file: &str, maki: &Value, sushi: &Value) -> Vec<Difference> {
    let mut differences = Vec::new();
    compare_json_recursive(file, "", maki, sushi, &mut differences);
    differences
}

/// Compare two JSON values for semantic equivalence
pub fn compare_semantic_equivalence(file: &str, maki: &Value, sushi: &Value) -> SemanticComparisonResult {
    let mut differences = Vec::new();
    let mut semantic_issues = Vec::new();
    
    compare_json_recursive(file, "", maki, sushi, &mut differences);
    
    // Analyze differences for semantic significance
    for diff in &differences {
        match diff {
            Difference::AcceptableDifference { .. } => {
                // These don't affect semantic equivalence
            }
            Difference::ContentDifference { path, maki_value, sushi_value, .. } => {
                if is_semantically_significant(path, maki_value, sushi_value) {
                    semantic_issues.push(SemanticIssue {
                        path: path.clone(),
                        issue_type: SemanticIssueType::ContentMismatch,
                        description: format!("Semantic difference: {} vs {}", maki_value, sushi_value),
                        severity: get_semantic_severity(path),
                    });
                }
            }
            Difference::MissingInMaki(file) => {
                semantic_issues.push(SemanticIssue {
                    path: file.clone(),
                    issue_type: SemanticIssueType::MissingResource,
                    description: "Resource missing in Maki output".to_string(),
                    severity: SemanticSeverity::High,
                });
            }
            Difference::MissingInSushi(file) => {
                semantic_issues.push(SemanticIssue {
                    path: file.clone(),
                    issue_type: SemanticIssueType::ExtraResource,
                    description: "Extra resource in Maki output".to_string(),
                    severity: SemanticSeverity::Medium,
                });
            }
        }
    }
    
    let equivalence_score = calculate_semantic_equivalence_score(&semantic_issues);
    
    SemanticComparisonResult {
        file: file.to_string(),
        is_equivalent: semantic_issues.is_empty(),
        equivalence_score,
        semantic_issues,
        raw_differences: differences,
    }
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

/// Check if a difference is semantically significant
fn is_semantically_significant(path: &str, _maki_value: &str, _sushi_value: &str) -> bool {
    // Skip metadata fields that don't affect semantic meaning
    if is_acceptable_field(path.split('.').last().unwrap_or(path)) {
        return false;
    }
    
    // Check for FHIR-specific semantic equivalence
    match path {
        // Resource type must match exactly
        p if p.ends_with("resourceType") => true,
        // IDs must match for references to work
        p if p.ends_with("id") => true,
        // URLs must match for canonical references
        p if p.ends_with("url") => true,
        // Status affects resource validity
        p if p.ends_with("status") => true,
        // Cardinality affects validation
        p if p.contains("min") || p.contains("max") => true,
        // Type definitions affect structure
        p if p.contains("type") => true,
        // Binding affects terminology validation
        p if p.contains("binding") => true,
        // Slicing affects element structure
        p if p.contains("slicing") => true,
        // Extensions affect functionality
        p if p.contains("extension") => true,
        // Default to significant if unsure
        _ => true,
    }
}

/// Get semantic severity for a path
fn get_semantic_severity(path: &str) -> SemanticSeverity {
    match path {
        // Critical paths that affect core functionality
        p if p.ends_with("resourceType") => SemanticSeverity::High,
        p if p.ends_with("id") => SemanticSeverity::High,
        p if p.ends_with("url") => SemanticSeverity::High,
        p if p.contains("type") => SemanticSeverity::High,
        
        // Important paths that affect validation
        p if p.ends_with("status") => SemanticSeverity::Medium,
        p if p.contains("min") || p.contains("max") => SemanticSeverity::Medium,
        p if p.contains("binding") => SemanticSeverity::Medium,
        p if p.contains("slicing") => SemanticSeverity::Medium,
        
        // Less critical paths
        _ => SemanticSeverity::Low,
    }
}

/// Calculate semantic equivalence score (0.0 = not equivalent, 1.0 = fully equivalent)
fn calculate_semantic_equivalence_score(issues: &[SemanticIssue]) -> f64 {
    if issues.is_empty() {
        return 1.0;
    }
    
    let total_weight = issues.len() as f64;
    let weighted_issues: f64 = issues.iter().map(|issue| {
        match issue.severity {
            SemanticSeverity::High => 1.0,
            SemanticSeverity::Medium => 0.6,
            SemanticSeverity::Low => 0.2,
        }
    }).sum();
    
    (total_weight - weighted_issues) / total_weight
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

/// Format semantic comparison results
pub fn format_semantic_results(results: &[SemanticComparisonResult]) -> String {
    let mut output = String::new();
    
    let total = results.len();
    let equivalent = results.iter().filter(|r| r.is_equivalent).count();
    let avg_score = if total > 0 {
        results.iter().map(|r| r.equivalence_score).sum::<f64>() / total as f64
    } else {
        1.0
    };
    
    output.push_str(&format!(
        "Semantic Equivalence Summary\n\
         ============================\n\
         Total Files: {}\n\
         Semantically Equivalent: {}\n\
         Average Equivalence Score: {:.2}\n\n",
        total, equivalent, avg_score
    ));
    
    // Show files with semantic issues
    for result in results.iter().filter(|r| !r.is_equivalent) {
        output.push_str(&format!("File: {} (Score: {:.2})\n", result.file, result.equivalence_score));
        
        for issue in &result.semantic_issues {
            let severity_icon = match issue.severity {
                SemanticSeverity::High => "🔴",
                SemanticSeverity::Medium => "🟡", 
                SemanticSeverity::Low => "🟢",
            };
            
            output.push_str(&format!(
                "  {} {} @ {}: {}\n",
                severity_icon, issue.issue_type_str(), issue.path, issue.description
            ));
        }
        output.push('\n');
    }
    
    output
}

impl SemanticIssue {
    fn issue_type_str(&self) -> &str {
        match self.issue_type {
            SemanticIssueType::ContentMismatch => "Content Mismatch",
            SemanticIssueType::MissingResource => "Missing Resource",
            SemanticIssueType::ExtraResource => "Extra Resource",
            SemanticIssueType::StructuralDifference => "Structural Difference",
            SemanticIssueType::TypeMismatch => "Type Mismatch",
            SemanticIssueType::CardinalityMismatch => "Cardinality Mismatch",
        }
    }
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
