//! Pattern parser for FSH GritQL-like syntax
//!
//! This module parses user-friendly pattern strings into AstPattern structs.
//!
//! # Pattern Syntax
//!
//! Basic pattern:
//! ```text
//! Profile where missing(parent)
//! Extension where missing(context)
//! Alias where name matches r"[^a-zA-Z0-9_-]"
//! ```
//!
//! Complex patterns:
//! ```text
//! Profile where {
//!     missing(parent) or missing(id)
//!     and name contains "Patient"
//! }
//! ```

use crate::gritql_ast::{AstPattern, NodeType, Predicate};
use fsh_lint_core::{FshLintError, Result};

/// Parse a pattern string into an AstPattern
pub fn parse_pattern(pattern_text: &str) -> Result<AstPattern> {
    let pattern_text = pattern_text.trim();

    // Handle empty patterns
    if pattern_text.is_empty() {
        return Err(FshLintError::config_error("Pattern cannot be empty"));
    }

    // Split on "where" keyword
    let parts: Vec<&str> = pattern_text.splitn(2, " where ").collect();

    if parts.len() != 2 {
        // No "where" clause - just node type
        let node_type = parse_node_type(parts[0].trim())?;
        return Ok(AstPattern {
            node_type,
            predicates: Vec::new(),
        });
    }

    let node_type = parse_node_type(parts[0].trim())?;
    let predicates_text = parts[1].trim();

    let predicates = parse_predicates(predicates_text)?;

    Ok(AstPattern {
        node_type,
        predicates,
    })
}

/// Parse node type from string
fn parse_node_type(type_str: &str) -> Result<NodeType> {
    match type_str {
        "Profile" => Ok(NodeType::Profile),
        "Extension" => Ok(NodeType::Extension),
        "ValueSet" => Ok(NodeType::ValueSet),
        "CodeSystem" => Ok(NodeType::CodeSystem),
        "Instance" => Ok(NodeType::Instance),
        "Invariant" => Ok(NodeType::Invariant),
        "Alias" => Ok(NodeType::Alias),
        "CardRule" | "Cardinality" => Ok(NodeType::CardRule),
        "FlagRule" | "Flag" => Ok(NodeType::FlagRule),
        "ValueSetRule" | "Binding" => Ok(NodeType::ValueSetRule),
        "FixedValueRule" | "Assignment" => Ok(NodeType::FixedValueRule),
        "OnlyRule" | "Type" => Ok(NodeType::OnlyRule),
        "ObeysRule" | "Constraint" => Ok(NodeType::ObeysRule),
        "CaretValueRule" | "Caret" => Ok(NodeType::CaretValueRule),
        "InsertRule" | "Insert" => Ok(NodeType::InsertRule),
        "PathRule" | "Path" => Ok(NodeType::PathRule),
        "*" | "Any" => Ok(NodeType::Any),
        other => Err(FshLintError::config_error(format!(
            "Unknown node type: {}",
            other
        ))),
    }
}

/// Parse predicates from string
fn parse_predicates(pred_text: &str) -> Result<Vec<Predicate>> {
    let pred_text = pred_text.trim();

    // Handle block syntax: { ... }
    if pred_text.starts_with('{') && pred_text.ends_with('}') {
        let inner = &pred_text[1..pred_text.len() - 1].trim();
        return parse_predicate_block(inner);
    }

    // Single predicate expression (may include and/or/not)
    Ok(vec![parse_predicate_expression(pred_text)?])
}

/// Parse a block of predicates (multiple lines with and/or)
fn parse_predicate_block(block: &str) -> Result<Vec<Predicate>> {
    // Split by newlines and parse each as a predicate expression
    let lines: Vec<&str> = block.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .collect();

    if lines.is_empty() {
        return Ok(Vec::new());
    }

    // For now, treat each line as an AND condition
    // TODO: Implement proper and/or parsing
    let mut predicates = Vec::new();
    for line in lines {
        predicates.push(parse_predicate_expression(line)?);
    }

    // If multiple predicates, wrap in And
    if predicates.len() > 1 {
        Ok(vec![Predicate::And(predicates)])
    } else {
        Ok(predicates)
    }
}

/// Parse a predicate expression (may contain and/or)
fn parse_predicate_expression(expr: &str) -> Result<Predicate> {
    let expr = expr.trim();

    // Check for negation first (highest precedence)
    if expr.starts_with("not ") {
        let negated = &expr[4..].trim();
        return Ok(Predicate::Not(Box::new(parse_predicate_expression(negated)?)));
    }

    if expr.starts_with("!") {
        let negated = &expr[1..].trim();
        return Ok(Predicate::Not(Box::new(parse_predicate_expression(negated)?)));
    }

    // Check for logical operators (OR has lower precedence than AND)
    if expr.contains(" or ") {
        return parse_or_expression(expr);
    }

    if expr.contains(" and ") {
        return parse_and_expression(expr);
    }

    // Single predicate
    parse_single_predicate(expr)
}

/// Parse OR expression
fn parse_or_expression(expr: &str) -> Result<Predicate> {
    let parts: Vec<&str> = expr.split(" or ").collect();
    let mut predicates = Vec::new();

    for part in parts {
        let part = part.trim();
        // Each part can be an AND expression or a single predicate
        if part.contains(" and ") {
            predicates.push(parse_and_expression(part)?);
        } else if part.starts_with("not ") || part.starts_with("!") {
            let negated = if part.starts_with("not ") {
                &part[4..]
            } else {
                &part[1..]
            };
            predicates.push(Predicate::Not(Box::new(parse_single_predicate(negated.trim())?)));
        } else {
            predicates.push(parse_single_predicate(part)?);
        }
    }

    Ok(Predicate::Or(predicates))
}

/// Parse AND expression
fn parse_and_expression(expr: &str) -> Result<Predicate> {
    let parts: Vec<&str> = expr.split(" and ").collect();
    let mut predicates = Vec::new();

    for part in parts {
        let part = part.trim();
        // Each part can be a NOT expression or a single predicate
        if part.starts_with("not ") || part.starts_with("!") {
            let negated = if part.starts_with("not ") {
                &part[4..]
            } else {
                &part[1..]
            };
            predicates.push(Predicate::Not(Box::new(parse_single_predicate(negated.trim())?)));
        } else {
            predicates.push(parse_single_predicate(part)?);
        }
    }

    Ok(Predicate::And(predicates))
}

/// Parse a single predicate function
fn parse_single_predicate(pred: &str) -> Result<Predicate> {
    let pred = pred.trim();

    // missing(field)
    if pred.starts_with("missing(") && pred.ends_with(')') {
        let field = extract_function_arg(pred, "missing")?;
        return Ok(Predicate::FieldMissing { field });
    }

    // present(field)
    if pred.starts_with("present(") && pred.ends_with(')') {
        let field = extract_function_arg(pred, "present")?;
        return Ok(Predicate::FieldPresent { field });
    }

    // field = "value" or field equals "value"
    if let Some(equals_pos) = pred.find(" = ") {
        let field = pred[..equals_pos].trim().to_string();
        let value = extract_string_value(&pred[equals_pos + 3..])?;
        return Ok(Predicate::FieldEquals { field, value });
    }

    if let Some(equals_pos) = pred.find(" equals ") {
        let field = pred[..equals_pos].trim().to_string();
        let value = extract_string_value(&pred[equals_pos + 8..])?;
        return Ok(Predicate::FieldEquals { field, value });
    }

    // field contains "substring"
    if let Some(contains_pos) = pred.find(" contains ") {
        let field = pred[..contains_pos].trim().to_string();
        let substring = extract_string_value(&pred[contains_pos + 10..])?;
        return Ok(Predicate::FieldContains { field, substring });
    }

    // field matches r"pattern" or field matches "pattern"
    if let Some(matches_pos) = pred.find(" matches ") {
        let field = pred[..matches_pos].trim().to_string();
        let pattern_str = &pred[matches_pos + 9..].trim();

        // Handle r"..." or "..." syntax
        let pattern = if pattern_str.starts_with("r\"") && pattern_str.ends_with('"') {
            pattern_str[2..pattern_str.len() - 1].to_string()
        } else {
            extract_string_value(pattern_str)?
        };

        return Ok(Predicate::FieldMatches { field, pattern });
    }

    Err(FshLintError::config_error(format!(
        "Invalid predicate syntax: {}",
        pred
    )))
}

/// Extract argument from function call like "missing(parent)"
fn extract_function_arg(expr: &str, func_name: &str) -> Result<String> {
    let start = func_name.len() + 1; // +1 for '('

    // Find the matching closing parenthesis
    let mut depth = 1;
    let mut end = start;
    let chars: Vec<char> = expr.chars().collect();

    while end < chars.len() && depth > 0 {
        if chars[end] == '(' {
            depth += 1;
        } else if chars[end] == ')' {
            depth -= 1;
        }
        if depth > 0 {
            end += 1;
        }
    }

    if depth != 0 {
        return Err(FshLintError::config_error(format!(
            "Unmatched parentheses in {}()",
            func_name
        )));
    }

    if start >= end {
        return Err(FshLintError::config_error(format!(
            "Empty argument in {}()",
            func_name
        )));
    }

    Ok(chars[start..end].iter().collect::<String>().trim().to_string())
}

/// Extract string value from quoted string
fn extract_string_value(s: &str) -> Result<String> {
    let s = s.trim();

    // Handle "..." or '...' quotes
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        if s.len() < 2 {
            return Err(FshLintError::config_error("Empty string value"));
        }
        return Ok(s[1..s.len() - 1].to_string());
    }

    // No quotes - return as is
    Ok(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pattern() {
        let pattern = parse_pattern("Profile where missing(parent)").unwrap();
        assert_eq!(pattern.node_type, NodeType::Profile);
        assert_eq!(pattern.predicates.len(), 1);

        match &pattern.predicates[0] {
            Predicate::FieldMissing { field } => assert_eq!(field, "parent"),
            _ => panic!("Expected FieldMissing predicate"),
        }
    }

    #[test]
    fn test_parse_node_type_only() {
        let pattern = parse_pattern("Profile").unwrap();
        assert_eq!(pattern.node_type, NodeType::Profile);
        assert_eq!(pattern.predicates.len(), 0);
    }

    #[test]
    fn test_parse_field_equals() {
        let pattern = parse_pattern("Profile where name = \"TestProfile\"").unwrap();
        assert_eq!(pattern.predicates.len(), 1);

        match &pattern.predicates[0] {
            Predicate::FieldEquals { field, value } => {
                assert_eq!(field, "name");
                assert_eq!(value, "TestProfile");
            }
            _ => panic!("Expected FieldEquals predicate"),
        }
    }

    #[test]
    fn test_parse_field_contains() {
        let pattern = parse_pattern("Profile where name contains \"Patient\"").unwrap();

        match &pattern.predicates[0] {
            Predicate::FieldContains { field, substring } => {
                assert_eq!(field, "name");
                assert_eq!(substring, "Patient");
            }
            _ => panic!("Expected FieldContains predicate"),
        }
    }

    #[test]
    fn test_parse_field_matches() {
        let pattern = parse_pattern(r#"Alias where name matches r"[^a-zA-Z0-9_-]""#).unwrap();

        match &pattern.predicates[0] {
            Predicate::FieldMatches { field, pattern } => {
                assert_eq!(field, "name");
                assert_eq!(pattern, "[^a-zA-Z0-9_-]");
            }
            _ => panic!("Expected FieldMatches predicate"),
        }
    }

    #[test]
    fn test_parse_or_expression() {
        let pattern = parse_pattern("Profile where missing(parent) or missing(id)").unwrap();

        match &pattern.predicates[0] {
            Predicate::Or(preds) => {
                assert_eq!(preds.len(), 2);
            }
            _ => panic!("Expected Or predicate"),
        }
    }

    #[test]
    fn test_parse_and_expression() {
        let pattern = parse_pattern("Profile where missing(parent) and missing(id)").unwrap();
        eprintln!("AND pattern predicates: {:?}", pattern.predicates);

        match &pattern.predicates[0] {
            Predicate::And(preds) => {
                assert_eq!(preds.len(), 2);
            }
            pred => panic!("Expected And predicate, got {:?}", pred),
        }
    }

    #[test]
    fn test_parse_not_expression() {
        let pattern = parse_pattern("Profile where not missing(parent)").unwrap();

        match &pattern.predicates[0] {
            Predicate::Not(pred) => {
                match **pred {
                    Predicate::FieldMissing { ref field } => assert_eq!(field, "parent"),
                    _ => panic!("Expected FieldMissing inside Not"),
                }
            }
            _ => panic!("Expected Not predicate"),
        }
    }

    #[test]
    fn test_parse_block_syntax() {
        let pattern = parse_pattern(r#"Profile where {
            missing(parent)
            missing(id)
        }"#).unwrap();

        // Should be wrapped in And
        match &pattern.predicates[0] {
            Predicate::And(preds) => {
                assert_eq!(preds.len(), 2);
            }
            _ => panic!("Expected And predicate for block"),
        }
    }

    #[test]
    fn test_parse_various_node_types() {
        assert_eq!(parse_node_type("Profile").unwrap(), NodeType::Profile);
        assert_eq!(parse_node_type("Extension").unwrap(), NodeType::Extension);
        assert_eq!(parse_node_type("ValueSet").unwrap(), NodeType::ValueSet);
        assert_eq!(parse_node_type("Alias").unwrap(), NodeType::Alias);
        assert_eq!(parse_node_type("CardRule").unwrap(), NodeType::CardRule);
        assert_eq!(parse_node_type("Cardinality").unwrap(), NodeType::CardRule);
        assert_eq!(parse_node_type("*").unwrap(), NodeType::Any);
    }

    #[test]
    fn test_invalid_patterns() {
        assert!(parse_pattern("").is_err());
        assert!(parse_pattern("UnknownType").is_err());
        assert!(parse_pattern("Profile where invalid_syntax").is_err());
    }
}
