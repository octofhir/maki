//! GritQL pattern matching for FSH Chumsky AST
//!
//! This module provides a simplified GritQL-like pattern matching system
//! designed specifically for our Chumsky-based FSH AST.
//!
//! Unlike full GritQL which works with tree-sitter, this implementation
//! provides pattern matching capabilities tailored to our FSH AST structure.

use fsh_lint_core::ast::*;
use fsh_lint_core::{Diagnostic, FshLintError, Location, Result, Severity};
use std::collections::HashMap;
use std::path::PathBuf;

/// Convert pattern matches to diagnostics
pub fn matches_to_diagnostics(
    matches: Vec<AstMatch>,
    rule_id: &str,
    severity: Severity,
    message_template: &str,
) -> Vec<Diagnostic> {
    matches
        .into_iter()
        .map(|m| {
            // Replace variables in message template with captured values
            let mut message = message_template.to_string();
            for (var, value) in &m.captures {
                message = message.replace(&format!("${}", var), value);
                message = message.replace(&format!("${{{}}}", var), value);
            }

            Diagnostic {
                rule_id: rule_id.to_string(),
                severity,
                message,
                location: m.location,
                suggestions: Vec::new(),
                code_snippet: Some(m.snippet),
                code: None,
                source: Some("fsh-lint".to_string()),
                category: None,
            }
        })
        .collect()
}

/// A pattern query for FSH AST nodes
#[derive(Debug, Clone)]
pub struct AstPattern {
    /// The node type to match (e.g., "Profile", "CardRule", "Alias")
    pub node_type: NodeType,
    /// Predicates that must be satisfied
    pub predicates: Vec<Predicate>,
}

/// FSH AST node types that can be matched
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Profile,
    Extension,
    ValueSet,
    CodeSystem,
    Instance,
    Invariant,
    Alias,
    CardRule,
    FlagRule,
    ValueSetRule,
    FixedValueRule,
    OnlyRule,
    ObeysRule,
    CaretValueRule,
    InsertRule,
    PathRule,
    Any, // Matches any node type
}

/// Predicates for filtering matched nodes
#[derive(Debug, Clone)]
pub enum Predicate {
    /// Check if a field matches a value (e.g., name = "MyProfile")
    FieldEquals { field: String, value: String },
    /// Check if a field contains a substring
    FieldContains { field: String, substring: String },
    /// Check if a field matches a regex pattern
    FieldMatches { field: String, pattern: String },
    /// Check if a field is missing/None
    FieldMissing { field: String },
    /// Check if a field is present/Some
    FieldPresent { field: String },
    /// Logical OR of multiple predicates
    Or(Vec<Predicate>),
    /// Logical AND of multiple predicates
    And(Vec<Predicate>),
    /// Logical NOT of a predicate
    Not(Box<Predicate>),
}

/// A match result from pattern execution
#[derive(Debug, Clone)]
pub struct AstMatch {
    /// The type of node that matched
    pub node_type: NodeType,
    /// The location in the source file
    pub location: Location,
    /// Captured variables from the pattern
    pub captures: HashMap<String, String>,
    /// The matched text snippet
    pub snippet: String,
}

/// Execute a pattern against an FSH document
pub fn execute_pattern(
    pattern: &AstPattern,
    document: &FSHDocument,
    file_path: &PathBuf,
    source: &str,
) -> Result<Vec<AstMatch>> {
    let mut matches = Vec::new();

    // Match based on node type
    match &pattern.node_type {
        NodeType::Profile => {
            for profile in &document.profiles {
                if evaluate_predicates_on_profile(&pattern.predicates, profile) {
                    matches.push(create_match_from_profile(profile, file_path, source)?);
                }
            }
        }
        NodeType::Extension => {
            for extension in &document.extensions {
                if evaluate_predicates_on_extension(&pattern.predicates, extension) {
                    matches.push(create_match_from_extension(extension, file_path, source)?);
                }
            }
        }
        NodeType::ValueSet => {
            for value_set in &document.value_sets {
                if evaluate_predicates_on_value_set(&pattern.predicates, value_set) {
                    matches.push(create_match_from_value_set(value_set, file_path, source)?);
                }
            }
        }
        NodeType::Alias => {
            for alias in &document.aliases {
                if evaluate_predicates_on_alias(&pattern.predicates, alias) {
                    matches.push(create_match_from_alias(alias, file_path, source)?);
                }
            }
        }
        NodeType::CardRule => {
            // Search all profiles/extensions for cardinality rules
            for profile in &document.profiles {
                for rule in &profile.rules {
                    if let SDRule::Card(card_rule) = rule {
                        if evaluate_predicates_on_card_rule(&pattern.predicates, card_rule) {
                            matches.push(create_match_from_card_rule(
                                card_rule, file_path, source,
                            )?);
                        }
                    }
                }
            }
            for extension in &document.extensions {
                for rule in &extension.rules {
                    if let SDRule::Card(card_rule) = rule {
                        if evaluate_predicates_on_card_rule(&pattern.predicates, card_rule) {
                            matches.push(create_match_from_card_rule(
                                card_rule, file_path, source,
                            )?);
                        }
                    }
                }
            }
        }
        NodeType::Any => {
            // Match all nodes in the document
            // This is more complex and we'll implement it as needed
            todo!("Implement Any node type matching")
        }
        _ => {
            // Implement other node types as needed
            todo!("Implement matching for {:?}", pattern.node_type)
        }
    }

    Ok(matches)
}

// Predicate evaluation functions

fn evaluate_predicates_on_profile(predicates: &[Predicate], profile: &Profile) -> bool {
    predicates.iter().all(|pred| match pred {
        Predicate::FieldMissing { field } => match field.as_str() {
            "parent" => profile.parent.is_none(),
            "id" => profile.id.is_none(),
            "title" => profile.title.is_none(),
            "description" => profile.description.is_none(),
            _ => false,
        },
        Predicate::FieldPresent { field } => match field.as_str() {
            "parent" => profile.parent.is_some(),
            "id" => profile.id.is_some(),
            "title" => profile.title.is_some(),
            "description" => profile.description.is_some(),
            _ => false,
        },
        Predicate::FieldEquals { field, value } => match field.as_str() {
            "name" => &profile.name.value == value,
            "parent" => profile
                .parent
                .as_ref()
                .map(|p| &p.value == value)
                .unwrap_or(false),
            _ => false,
        },
        Predicate::FieldContains { field, substring } => match field.as_str() {
            "name" => profile.name.value.contains(substring),
            _ => false,
        },
        Predicate::And(preds) => preds
            .iter()
            .all(|p| evaluate_predicate_on_profile(p, profile)),
        Predicate::Or(preds) => preds
            .iter()
            .any(|p| evaluate_predicate_on_profile(p, profile)),
        Predicate::Not(pred) => !evaluate_predicate_on_profile(pred, profile),
        _ => false,
    })
}

fn evaluate_predicate_on_profile(predicate: &Predicate, profile: &Profile) -> bool {
    evaluate_predicates_on_profile(&[predicate.clone()], profile)
}

fn evaluate_predicates_on_extension(predicates: &[Predicate], extension: &Extension) -> bool {
    predicates.iter().all(|pred| match pred {
        Predicate::FieldMissing { field } => match field.as_str() {
            "parent" => extension.parent.is_none(),
            "id" => extension.id.is_none(),
            "title" => extension.title.is_none(),
            "description" => extension.description.is_none(),
            "context" | "contexts" => extension.contexts.is_empty(),
            _ => false,
        },
        Predicate::FieldPresent { field } => match field.as_str() {
            "parent" => extension.parent.is_some(),
            "id" => extension.id.is_some(),
            "title" => extension.title.is_some(),
            "description" => extension.description.is_some(),
            "context" | "contexts" => !extension.contexts.is_empty(),
            _ => false,
        },
        _ => false,
    })
}

fn evaluate_predicates_on_value_set(predicates: &[Predicate], value_set: &ValueSet) -> bool {
    predicates.iter().all(|pred| match pred {
        Predicate::FieldMissing { field } => match field.as_str() {
            "id" => value_set.id.is_none(),
            "title" => value_set.title.is_none(),
            "description" => value_set.description.is_none(),
            _ => false,
        },
        Predicate::FieldPresent { field } => match field.as_str() {
            "id" => value_set.id.is_some(),
            "title" => value_set.title.is_some(),
            "description" => value_set.description.is_some(),
            _ => false,
        },
        _ => false,
    })
}

fn evaluate_predicates_on_alias(predicates: &[Predicate], alias: &Alias) -> bool {
    predicates.iter().all(|pred| match pred {
        Predicate::FieldEquals { field, value } => match field.as_str() {
            "name" => &alias.name.value == value,
            "value" => &alias.value.value == value,
            _ => false,
        },
        Predicate::FieldContains { field, substring } => match field.as_str() {
            "name" => alias.name.value.contains(substring),
            "value" => alias.value.value.contains(substring),
            _ => false,
        },
        Predicate::FieldMatches { field, pattern } => match field.as_str() {
            "value" => {
                // Simple regex matching
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(&alias.value.value)
                } else {
                    false
                }
            }
            _ => false,
        },
        _ => false,
    })
}

fn evaluate_predicates_on_card_rule(predicates: &[Predicate], card_rule: &CardRule) -> bool {
    predicates.iter().all(|pred| match pred {
        Predicate::FieldEquals { field, value } => match field.as_str() {
            "path" => &card_rule.path.value == value,
            _ => false,
        },
        Predicate::FieldContains { field, substring } => match field.as_str() {
            "path" => card_rule.path.value.contains(substring),
            _ => false,
        },
        _ => false,
    })
}

// Match creation functions

fn create_match_from_profile(
    profile: &Profile,
    file_path: &PathBuf,
    source: &str,
) -> Result<AstMatch> {
    let location = span_to_location(&profile.span, file_path);
    let snippet = extract_snippet(source, &profile.span);

    Ok(AstMatch {
        node_type: NodeType::Profile,
        location,
        captures: {
            let mut map = HashMap::new();
            map.insert("name".to_string(), profile.name.value.clone());
            if let Some(parent) = &profile.parent {
                map.insert("parent".to_string(), parent.value.clone());
            }
            map
        },
        snippet,
    })
}

fn create_match_from_extension(
    extension: &Extension,
    file_path: &PathBuf,
    source: &str,
) -> Result<AstMatch> {
    let location = span_to_location(&extension.span, file_path);
    let snippet = extract_snippet(source, &extension.span);

    Ok(AstMatch {
        node_type: NodeType::Extension,
        location,
        captures: {
            let mut map = HashMap::new();
            map.insert("name".to_string(), extension.name.value.clone());
            map
        },
        snippet,
    })
}

fn create_match_from_value_set(
    value_set: &ValueSet,
    file_path: &PathBuf,
    source: &str,
) -> Result<AstMatch> {
    let location = span_to_location(&value_set.span, file_path);
    let snippet = extract_snippet(source, &value_set.span);

    Ok(AstMatch {
        node_type: NodeType::ValueSet,
        location,
        captures: {
            let mut map = HashMap::new();
            map.insert("name".to_string(), value_set.name.value.clone());
            map
        },
        snippet,
    })
}

fn create_match_from_alias(
    alias: &Alias,
    file_path: &PathBuf,
    source: &str,
) -> Result<AstMatch> {
    let location = span_to_location(&alias.span, file_path);
    let snippet = extract_snippet(source, &alias.span);

    Ok(AstMatch {
        node_type: NodeType::Alias,
        location,
        captures: {
            let mut map = HashMap::new();
            map.insert("name".to_string(), alias.name.value.clone());
            map.insert("value".to_string(), alias.value.value.clone());
            map
        },
        snippet,
    })
}

fn create_match_from_card_rule(
    card_rule: &CardRule,
    file_path: &PathBuf,
    source: &str,
) -> Result<AstMatch> {
    let location = span_to_location(&card_rule.span, file_path);
    let snippet = extract_snippet(source, &card_rule.span);

    Ok(AstMatch {
        node_type: NodeType::CardRule,
        location,
        captures: {
            let mut map = HashMap::new();
            map.insert("path".to_string(), card_rule.path.value.clone());
            map
        },
        snippet,
    })
}

// Utility functions

fn span_to_location(span: &Span, file_path: &PathBuf) -> Location {
    // For now, create a basic location
    // We'll need to enhance this to calculate line/column from span offsets
    Location {
        file: file_path.clone(),
        line: 0, // TODO: Calculate from span
        column: 0,
        end_line: None,
        end_column: None,
        offset: span.start,
        length: span.end - span.start,
        span: Some((span.start, span.end)),
    }
}

fn extract_snippet(source: &str, span: &Span) -> String {
    source
        .get(span.start..span.end)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_field_missing() {
        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: None,
            id: None,
            title: None,
            description: None,
            rules: Vec::new(),
            span: 0..100,
        };

        let pred = Predicate::FieldMissing {
            field: "parent".to_string(),
        };
        assert!(evaluate_predicates_on_profile(&[pred], &profile));
    }

    #[test]
    fn test_predicate_field_present() {
        let profile = Profile {
            name: Spanned::new("TestProfile".to_string(), 0..11),
            parent: Some(Spanned::new("Patient".to_string(), 20..27)),
            id: None,
            title: None,
            description: None,
            rules: Vec::new(),
            span: 0..100,
        };

        let pred = Predicate::FieldPresent {
            field: "parent".to_string(),
        };
        assert!(evaluate_predicates_on_profile(&[pred], &profile));
    }
}
