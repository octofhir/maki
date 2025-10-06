//! GritQL pattern matching for FSH AST using CST

use fsh_lint_core::cst::FshSyntaxNode;
use fsh_lint_core::cst::ast::{AstNode, *};
use fsh_lint_core::{Diagnostic, Location, Severity};
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
            let mut message = message_template.to_string();
            for (var, value) in &m.captures {
                message = message.replace(&format!("${var}"), value);
                message = message.replace(&format!("${{{var}}}"), value);
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
    pub node_type: NodeType,
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
    Any,
}

/// Predicates for filtering matched nodes
#[derive(Debug, Clone)]
pub enum Predicate {
    FieldEquals { field: String, value: String },
    FieldContains { field: String, substring: String },
    FieldMatches { field: String, pattern: String },
    FieldMissing { field: String },
    FieldPresent { field: String },
    Or(Vec<Predicate>),
    And(Vec<Predicate>),
    Not(Box<Predicate>),
}

/// A match result from pattern execution
#[derive(Debug, Clone)]
pub struct AstMatch {
    pub node_type: NodeType,
    pub location: Location,
    pub captures: HashMap<String, String>,
    pub snippet: String,
}

/// Execute a pattern against an FSH document using CST
pub fn execute_pattern(
    pattern: &AstPattern,
    document: &Document,
    file_path: &PathBuf,
    source: &str,
) -> fsh_lint_core::Result<Vec<AstMatch>> {
    let mut matches = Vec::new();

    // Match profiles
    if matches_node_type(&pattern.node_type, &NodeType::Profile) {
        for profile in document.profiles() {
            if evaluate_predicates(&pattern.predicates, &NodeContext::Profile(&profile))? {
                matches.push(create_match(
                    NodeType::Profile,
                    profile.syntax(),
                    file_path,
                    source,
                    &pattern.predicates,
                    &NodeContext::Profile(&profile),
                )?);
            }
        }
    }

    // Match extensions
    if matches_node_type(&pattern.node_type, &NodeType::Extension) {
        for extension in document.extensions() {
            if evaluate_predicates(&pattern.predicates, &NodeContext::Extension(&extension))? {
                matches.push(create_match(
                    NodeType::Extension,
                    extension.syntax(),
                    file_path,
                    source,
                    &pattern.predicates,
                    &NodeContext::Extension(&extension),
                )?);
            }
        }
    }

    // Match value sets
    if matches_node_type(&pattern.node_type, &NodeType::ValueSet) {
        for value_set in document.value_sets() {
            if evaluate_predicates(&pattern.predicates, &NodeContext::ValueSet(&value_set))? {
                matches.push(create_match(
                    NodeType::ValueSet,
                    value_set.syntax(),
                    file_path,
                    source,
                    &pattern.predicates,
                    &NodeContext::ValueSet(&value_set),
                )?);
            }
        }
    }

    // Match code systems
    if matches_node_type(&pattern.node_type, &NodeType::CodeSystem) {
        for code_system in document.code_systems() {
            if evaluate_predicates(&pattern.predicates, &NodeContext::CodeSystem(&code_system))? {
                matches.push(create_match(
                    NodeType::CodeSystem,
                    code_system.syntax(),
                    file_path,
                    source,
                    &pattern.predicates,
                    &NodeContext::CodeSystem(&code_system),
                )?);
            }
        }
    }

    // TODO: Instance and Invariant not yet implemented in CST AST
    // if matches_node_type(&pattern.node_type, &NodeType::Instance) {
    //     for instance in document.instances() {
    //         ...
    //     }
    // }

    // TODO: Invariant not yet implemented in CST AST
    // if matches_node_type(&pattern.node_type, &NodeType::Invariant) {
    //     for invariant in document.invariants() {
    //         ...
    //     }
    // }

    // Match aliases
    if matches_node_type(&pattern.node_type, &NodeType::Alias) {
        for alias in document.aliases() {
            if evaluate_predicates(&pattern.predicates, &NodeContext::Alias(&alias))? {
                matches.push(create_match(
                    NodeType::Alias,
                    alias.syntax(),
                    file_path,
                    source,
                    &pattern.predicates,
                    &NodeContext::Alias(&alias),
                )?);
            }
        }
    }

    // TODO: Add rule matching (CardRule, FlagRule, etc.) by traversing profiles/extensions

    Ok(matches)
}

/// Context wrapper for different node types
enum NodeContext<'a> {
    Profile(&'a Profile),
    Extension(&'a Extension),
    ValueSet(&'a ValueSet),
    CodeSystem(&'a CodeSystem),
    // TODO: Instance and Invariant not yet in CST AST
    // Instance(&'a Instance),
    // Invariant(&'a Invariant),
    Alias(&'a Alias),
    Rule(&'a Rule),
}

/// Check if pattern node type matches target
fn matches_node_type(pattern: &NodeType, target: &NodeType) -> bool {
    pattern == &NodeType::Any || pattern == target
}

/// Evaluate all predicates against a node
fn evaluate_predicates(
    predicates: &[Predicate],
    context: &NodeContext,
) -> fsh_lint_core::Result<bool> {
    for predicate in predicates {
        if !evaluate_predicate(predicate, context)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Evaluate a single predicate
fn evaluate_predicate(predicate: &Predicate, context: &NodeContext) -> fsh_lint_core::Result<bool> {
    match predicate {
        Predicate::FieldEquals { field, value } => {
            let field_value = get_field_value(field, context)?;
            Ok(field_value.as_deref() == Some(value.as_str()))
        }
        Predicate::FieldContains { field, substring } => {
            let field_value = get_field_value(field, context)?;
            Ok(field_value.is_some_and(|v| v.contains(substring)))
        }
        Predicate::FieldMatches { field, pattern } => {
            let field_value = get_field_value(field, context)?;
            if let Some(value) = field_value {
                let re = regex::Regex::new(pattern).map_err(|e| {
                    fsh_lint_core::FshLintError::rule_error(
                        "gritql-pattern",
                        format!("Invalid regex: {e}"),
                    )
                })?;
                Ok(re.is_match(&value))
            } else {
                Ok(false)
            }
        }
        Predicate::FieldMissing { field } => {
            let field_value = get_field_value(field, context)?;
            Ok(field_value.is_none())
        }
        Predicate::FieldPresent { field } => {
            let field_value = get_field_value(field, context)?;
            Ok(field_value.is_some())
        }
        Predicate::Or(predicates) => {
            for pred in predicates {
                if evaluate_predicate(pred, context)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Predicate::And(predicates) => {
            for pred in predicates {
                if !evaluate_predicate(pred, context)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Predicate::Not(pred) => Ok(!evaluate_predicate(pred, context)?),
    }
}

/// Get field value from a node context
fn get_field_value(field: &str, context: &NodeContext) -> fsh_lint_core::Result<Option<String>> {
    match context {
        NodeContext::Profile(profile) => match field {
            "name" => Ok(profile.name()),
            "id" => Ok(profile.id().and_then(|c| c.value())),
            "title" => Ok(profile.title().and_then(|c| c.value())),
            "description" => Ok(profile.description().and_then(|c| c.value())),
            "parent" => Ok(profile.parent().and_then(|c| c.value())),
            _ => Ok(None),
        },
        NodeContext::Extension(extension) => match field {
            "name" => Ok(extension.name()),
            "id" => Ok(extension.id().and_then(|c| c.value())),
            "title" => Ok(extension.title().and_then(|c| c.value())),
            "description" => Ok(extension.description().and_then(|c| c.value())),
            _ => Ok(None),
        },
        NodeContext::ValueSet(value_set) => match field {
            "name" => Ok(value_set.name()),
            "id" => Ok(value_set.id().and_then(|c| c.value())),
            "title" => Ok(value_set.title().and_then(|c| c.value())),
            "description" => Ok(value_set.description().and_then(|c| c.value())),
            _ => Ok(None),
        },
        NodeContext::CodeSystem(code_system) => match field {
            "name" => Ok(code_system.name()),
            "id" => Ok(code_system.id().and_then(|c| c.value())),
            "title" => Ok(code_system.title().and_then(|c| c.value())),
            "description" => Ok(code_system.description().and_then(|c| c.value())),
            _ => Ok(None),
        },
        // TODO: Instance and Invariant not yet in CST AST
        // NodeContext::Instance(instance) => match field {
        //     "name" => Ok(instance.name()),
        //     _ => Ok(None),
        // },
        // NodeContext::Invariant(invariant) => match field {
        //     "name" => Ok(invariant.name()),
        //     _ => Ok(None),
        // },
        NodeContext::Alias(alias) => match field {
            "name" => Ok(alias.name()),
            "value" => Ok(alias.value()),
            _ => Ok(None),
        },
        NodeContext::Rule(_rule) => {
            // TODO: Add rule field matching
            Ok(None)
        }
    }
}

/// Create a match result with captures
fn create_match(
    node_type: NodeType,
    node: &FshSyntaxNode,
    file_path: &PathBuf,
    source: &str,
    predicates: &[Predicate],
    context: &NodeContext,
) -> fsh_lint_core::Result<AstMatch> {
    let range = node.text_range();
    let span = usize::from(range.start())..usize::from(range.end());

    let location = span_to_location(file_path, &span, source);
    let snippet = source[span.clone()].to_string();

    // Extract captures from predicates
    let mut captures = HashMap::new();
    extract_captures(predicates, context, &mut captures)?;

    Ok(AstMatch {
        node_type,
        location,
        captures,
        snippet,
    })
}

/// Extract variable captures from predicates
fn extract_captures(
    predicates: &[Predicate],
    context: &NodeContext,
    captures: &mut HashMap<String, String>,
) -> fsh_lint_core::Result<()> {
    for predicate in predicates {
        match predicate {
            Predicate::FieldEquals { field, .. }
            | Predicate::FieldContains { field, .. }
            | Predicate::FieldMatches { field, .. }
            | Predicate::FieldPresent { field } => {
                if let Some(value) = get_field_value(field, context)? {
                    captures.insert(field.clone(), value);
                }
            }
            Predicate::Or(preds) | Predicate::And(preds) => {
                extract_captures(preds, context, captures)?;
            }
            Predicate::Not(pred) => {
                extract_captures(&[(**pred).clone()], context, captures)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Convert span to location (helper function)
fn span_to_location(file_path: &PathBuf, span: &std::ops::Range<usize>, source: &str) -> Location {
    let line = source[..span.start].lines().count();
    let column = source[..span.start]
        .lines()
        .last()
        .map_or(0, |line| line.len());

    let end_line = source[..span.end].lines().count();
    let end_column = source[..span.end]
        .lines()
        .last()
        .map_or(0, |line| line.len());

    Location {
        file: file_path.clone(),
        line,
        column,
        end_line: Some(end_line),
        end_column: Some(end_column),
        offset: span.start,
        length: span.end - span.start,
        span: Some((span.start, span.end)),
    }
}
