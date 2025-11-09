//! GritQL pattern parser
//!
//! Parses GritQL syntax into an AST that can be compiled to Pattern structs.

use maki_core::{MakiError, Result};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GritPattern {
    NodeKind(String),
    Where(Box<GritPattern>, Box<GritPredicate>),
    Variable(String),
    Not(Box<GritPattern>),
    And(Vec<GritPattern>),
    Or(Vec<GritPattern>),
    Assignment {
        var: String,
        value: Box<GritPattern>,
    },
    Maybe(Box<GritPattern>),
    Any(Vec<GritPattern>),
    Contains(Box<GritPattern>),
    Within(Box<GritPattern>),
    After(Box<GritPattern>),
    Bubble {
        pattern: Box<GritPattern>,
        args: Vec<String>,
    },
}

/// Value expressions used in predicates (e.g., $var <: or { "a", "b" })
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GritMatchValue {
    /// String literal: "value"
    String(String),
    /// Regex pattern: r"pattern"
    Regex(String),
    /// Or expression: or { "a", "b", "c" }
    Or(Vec<GritMatchValue>),
    /// And expression: and { r"^A", r"Z$" }
    And(Vec<GritMatchValue>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GritPredicate {
    /// Check if a field exists
    FieldExists(String),
    /// Not predicate
    Not(Box<GritPredicate>),
    /// And predicate (all must be true)
    And(Vec<GritPredicate>),
    /// Or predicate (any must be true)
    Or(Vec<GritPredicate>),
    /// Match operator: $var <: value_expression
    Match { var: String, value: GritMatchValue },
    /// Contains: $var contains value_expression
    Contains { var: String, value: GritMatchValue },
    /// Starts with: $var startsWith value_expression
    StartsWith { var: String, value: GritMatchValue },
    /// Ends with: $var endsWith value_expression
    EndsWith { var: String, value: GritMatchValue },
    /// Equality: $a == "value"
    Equality { left: String, right: String },
    /// Inequality: $a != "value"
    Inequality { left: String, right: String },
    /// Variable pattern with constraints: $var where { predicates }
    VariablePattern {
        var: String,
        constraint: Box<GritPredicate>,
    },
}

pub struct GritQLParser {
    input: String,
    pos: usize,
}

impl GritQLParser {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            pos: 0,
        }
    }

    pub fn parse(&mut self) -> Result<GritPattern> {
        self.skip_whitespace();
        self.parse_pattern()
    }

    fn parse_pattern(&mut self) -> Result<GritPattern> {
        self.skip_whitespace();

        if self.peek_word("not") {
            self.consume_word("not")?;
            self.skip_whitespace();
            let pattern = self.parse_pattern()?;
            return Ok(GritPattern::Not(Box::new(pattern)));
        }

        if self.current_char() == Some('$') {
            return self.parse_variable();
        }

        let base_pattern = self.parse_base_pattern()?;

        self.skip_whitespace();
        if self.peek_word("where") {
            self.consume_word("where")?;
            self.skip_whitespace();
            let predicate = self.parse_predicate()?;
            return Ok(GritPattern::Where(
                Box::new(base_pattern),
                Box::new(predicate),
            ));
        }

        Ok(base_pattern)
    }

    fn parse_base_pattern(&mut self) -> Result<GritPattern> {
        let ident = self.parse_identifier()?;

        // Check for ": $variable" syntax (e.g., "Profile: $name")
        self.skip_whitespace();
        if self.current_char() == Some(':') {
            self.advance(); // consume ':'
            self.skip_whitespace();

            // Expect a variable ($name)
            if self.current_char() == Some('$') {
                self.advance(); // consume '$'
                let var_name = self.parse_identifier()?;

                // Return an Assignment: Profile: $name means assign the matched Profile to $name
                return Ok(GritPattern::Assignment {
                    var: var_name,
                    value: Box::new(GritPattern::NodeKind(ident)),
                });
            } else {
                return Err(MakiError::rule_error(
                    "gritql-parser",
                    format!(
                        "Expected variable after '{}:', found {:?}",
                        ident,
                        self.current_char()
                    ),
                ));
            }
        }

        Ok(GritPattern::NodeKind(ident))
    }

    fn parse_variable(&mut self) -> Result<GritPattern> {
        self.expect_char('$')?;
        let name = self.parse_identifier()?;

        self.skip_whitespace();
        if self.current_char() == Some('=') {
            self.advance();
            self.skip_whitespace();
            let value = self.parse_pattern()?;
            return Ok(GritPattern::Assignment {
                var: name,
                value: Box::new(value),
            });
        }

        Ok(GritPattern::Variable(name))
    }

    fn parse_predicate(&mut self) -> Result<GritPredicate> {
        self.skip_whitespace();
        self.expect_char('{')?;
        self.skip_whitespace();

        // Check for pattern-level 'or {' or 'and {' or 'not' blocks
        if self.peek_word("or") && self.peek_ahead_for_brace() {
            let pred = self.parse_or_predicate_block()?;
            self.skip_whitespace();
            self.expect_char('}')?;
            return Ok(pred);
        }

        if self.peek_word("and") && self.peek_ahead_for_brace() {
            let pred = self.parse_and_predicate_block()?;
            self.skip_whitespace();
            self.expect_char('}')?;
            return Ok(pred);
        }

        // Parse: not inner
        if self.peek_word("not") {
            self.consume_word("not")?;
            self.skip_whitespace();
            let pred = self.parse_predicate_inner()?;
            self.skip_whitespace();
            self.expect_char('}')?;
            return Ok(GritPredicate::Not(Box::new(pred)));
        }

        let pred = self.parse_predicate_inner()?;
        self.skip_whitespace();
        self.expect_char('}')?;

        Ok(pred)
    }

    /// Parse: or { predicate1, predicate2, ... } (pattern-level composition)
    fn parse_or_predicate_block(&mut self) -> Result<GritPredicate> {
        self.consume_word("or")?;
        self.skip_whitespace();
        self.expect_char('{')?;
        self.skip_whitespace();

        let mut preds = Vec::new();

        // Parse first predicate
        if self.current_char() != Some('}') {
            preds.push(self.parse_predicate_inner()?);
            self.skip_whitespace();

            // Parse remaining predicates (comma-separated)
            while self.current_char() == Some(',') {
                self.advance(); // consume comma
                self.skip_whitespace();

                // Allow trailing comma
                if self.current_char() == Some('}') {
                    break;
                }

                preds.push(self.parse_predicate_inner()?);
                self.skip_whitespace();
            }
        }

        self.expect_char('}')?;
        Ok(GritPredicate::Or(preds))
    }

    /// Parse: and { predicate1, predicate2, ... } (pattern-level composition)
    fn parse_and_predicate_block(&mut self) -> Result<GritPredicate> {
        self.consume_word("and")?;
        self.skip_whitespace();
        self.expect_char('{')?;
        self.skip_whitespace();

        let mut preds = Vec::new();

        // Parse first predicate
        if self.current_char() != Some('}') {
            preds.push(self.parse_predicate_inner()?);
            self.skip_whitespace();

            // Parse remaining predicates (comma-separated)
            while self.current_char() == Some(',') {
                self.advance(); // consume comma
                self.skip_whitespace();

                // Allow trailing comma
                if self.current_char() == Some('}') {
                    break;
                }

                preds.push(self.parse_predicate_inner()?);
                self.skip_whitespace();
            }
        }

        self.expect_char('}')?;
        Ok(GritPredicate::And(preds))
    }

    fn parse_predicate_inner(&mut self) -> Result<GritPredicate> {
        self.skip_whitespace();

        // Check for nested 'or {' or 'and {' blocks
        if self.peek_word("or") && self.peek_ahead_for_brace() {
            return self.parse_or_predicate_block();
        }

        if self.peek_word("and") && self.peek_ahead_for_brace() {
            return self.parse_and_predicate_block();
        }

        // Check for 'not' prefix
        if self.peek_word("not") {
            self.consume_word("not")?;
            self.skip_whitespace();
            let pred = self.parse_simple_predicate()?;
            return Ok(GritPredicate::Not(Box::new(pred)));
        }

        // NEW: Check for variable pattern with constraints: $var where { predicates }
        // IMPORTANT: Must have braces after 'where' to distinguish from regular predicates
        if self.current_char() == Some('$') {
            let start_pos = self.pos;
            self.advance(); // consume '$'
            let var_name = self.parse_identifier()?;
            self.skip_whitespace();

            // Check if followed by 'where {'
            if self.peek_word("where") {
                let after_where_pos = self.pos + 5; // "where".len()

                // Look ahead to check for '{' after 'where'
                let mut temp_pos = after_where_pos;
                while temp_pos < self.input.len() {
                    match self.input.chars().nth(temp_pos) {
                        Some('{') => {
                            // This is a variable pattern: $var where { ... }
                            self.consume_word("where")?;
                            self.skip_whitespace();

                            // Parse nested where clause predicates
                            let constraint = self.parse_predicate()?;

                            return Ok(GritPredicate::VariablePattern {
                                var: var_name,
                                constraint: Box::new(constraint),
                            });
                        }
                        Some(c) if c.is_whitespace() => {
                            temp_pos += 1;
                        }
                        _ => {
                            // Not a variable pattern, it's $var where <something else>
                            break;
                        }
                    }
                }
            }

            // Not a variable pattern, rewind and parse as simple predicate
            self.pos = start_pos;
        }

        let first = self.parse_simple_predicate()?;

        self.skip_whitespace();

        // Check for infix 'and' or 'or' operators (without braces)
        if self.peek_word("and") && !self.peek_ahead_for_brace() {
            let mut preds = vec![first];
            while self.peek_word("and") && !self.peek_ahead_for_brace() {
                self.consume_word("and")?;
                self.skip_whitespace();
                preds.push(self.parse_simple_predicate()?);
                self.skip_whitespace();
            }
            return Ok(GritPredicate::And(preds));
        }

        if self.peek_word("or") && !self.peek_ahead_for_brace() {
            let mut preds = vec![first];
            while self.peek_word("or") && !self.peek_ahead_for_brace() {
                self.consume_word("or")?;
                self.skip_whitespace();
                preds.push(self.parse_simple_predicate()?);
                self.skip_whitespace();
            }
            return Ok(GritPredicate::Or(preds));
        }

        Ok(first)
    }

    fn parse_simple_predicate(&mut self) -> Result<GritPredicate> {
        self.skip_whitespace();

        // Check for implicit context operators (contains/startsWith/endsWith without variable)
        if self.peek_word("contains") {
            self.consume_word("contains")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::Contains {
                var: "_".to_string(), // implicit context
                value,
            });
        }

        if self.peek_word("startsWith") {
            self.consume_word("startsWith")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::StartsWith {
                var: "_".to_string(), // implicit context
                value,
            });
        }

        if self.peek_word("endsWith") {
            self.consume_word("endsWith")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::EndsWith {
                var: "_".to_string(), // implicit context
                value,
            });
        }

        // Parse variable or field name (might start with $)
        let has_dollar = self.current_char() == Some('$');
        if has_dollar {
            self.advance();
        }

        let field_or_var = self.parse_identifier()?;
        self.skip_whitespace();

        // Check for operators
        // <: (match operator)
        if self.peek_word("<:") {
            self.consume_word("<:")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::Match {
                var: field_or_var,
                value,
            });
        }

        // contains (with variable prefix)
        if self.peek_word("contains") {
            self.consume_word("contains")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::Contains {
                var: field_or_var,
                value,
            });
        }

        // startsWith (with variable prefix)
        if self.peek_word("startsWith") {
            self.consume_word("startsWith")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::StartsWith {
                var: field_or_var,
                value,
            });
        }

        // endsWith (with variable prefix)
        if self.peek_word("endsWith") {
            self.consume_word("endsWith")?;
            self.skip_whitespace();
            let value = self.parse_match_value()?;
            return Ok(GritPredicate::EndsWith {
                var: field_or_var,
                value,
            });
        }

        // == (equality)
        if self.peek_word("==") {
            self.consume_word("==")?;
            self.skip_whitespace();
            let right = self.parse_quoted_string()?;
            return Ok(GritPredicate::Equality {
                left: field_or_var,
                right,
            });
        }

        // != (inequality)
        if self.peek_word("!=") {
            self.consume_word("!=")?;
            self.skip_whitespace();
            let right = self.parse_quoted_string()?;
            return Ok(GritPredicate::Inequality {
                left: field_or_var,
                right,
            });
        }

        // No operator found, treat as field exists
        Ok(GritPredicate::FieldExists(field_or_var))
    }

    /// Parse a match value expression (string, regex, or, and)
    fn parse_match_value(&mut self) -> Result<GritMatchValue> {
        self.skip_whitespace();

        // Check for 'or {' pattern
        if self.peek_word("or") && self.peek_ahead_for_brace() {
            return self.parse_or_value();
        }

        // Check for 'and {' pattern
        if self.peek_word("and") && self.peek_ahead_for_brace() {
            return self.parse_and_value();
        }

        // Default: string or regex
        self.parse_simple_match_value()
    }

    /// Parse a simple match value (string or regex)
    fn parse_simple_match_value(&mut self) -> Result<GritMatchValue> {
        self.skip_whitespace();

        // Check for r"..." regex
        if self.peek_word("r\"") {
            self.consume_word("r")?;
            let pattern = self.parse_regex_string()?;
            return Ok(GritMatchValue::Regex(pattern));
        }

        // Otherwise just a regular quoted string
        let s = self.parse_quoted_string()?;
        Ok(GritMatchValue::String(s))
    }

    /// Parse: or { value1, value2, ... }
    fn parse_or_value(&mut self) -> Result<GritMatchValue> {
        self.consume_word("or")?;
        self.skip_whitespace();
        self.expect_char('{')?;
        self.skip_whitespace();

        let mut values = Vec::new();

        // Parse first value
        if self.current_char() != Some('}') {
            values.push(self.parse_simple_match_value()?);
            self.skip_whitespace();

            // Parse remaining values (comma-separated)
            while self.current_char() == Some(',') {
                self.advance(); // consume comma
                self.skip_whitespace();

                // Allow trailing comma
                if self.current_char() == Some('}') {
                    break;
                }

                values.push(self.parse_simple_match_value()?);
                self.skip_whitespace();
            }
        }

        self.expect_char('}')?;
        Ok(GritMatchValue::Or(values))
    }

    /// Parse: and { value1, value2, ... }
    fn parse_and_value(&mut self) -> Result<GritMatchValue> {
        self.consume_word("and")?;
        self.skip_whitespace();
        self.expect_char('{')?;
        self.skip_whitespace();

        let mut values = Vec::new();

        // Parse first value
        if self.current_char() != Some('}') {
            values.push(self.parse_simple_match_value()?);
            self.skip_whitespace();

            // Parse remaining values (comma-separated)
            while self.current_char() == Some(',') {
                self.advance(); // consume comma
                self.skip_whitespace();

                // Allow trailing comma
                if self.current_char() == Some('}') {
                    break;
                }

                values.push(self.parse_simple_match_value()?);
                self.skip_whitespace();
            }
        }

        self.expect_char('}')?;
        Ok(GritMatchValue::And(values))
    }

    /// Check if there's a '{' ahead after skipping whitespace (without consuming)
    fn peek_ahead_for_brace(&self) -> bool {
        let mut temp_pos = self.pos;

        // Skip the current word (or/and)
        while temp_pos < self.input.len() {
            if let Some(ch) = self.input[temp_pos..].chars().next() {
                if ch.is_alphabetic() {
                    temp_pos += ch.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Skip whitespace
        while temp_pos < self.input.len() {
            if let Some(ch) = self.input[temp_pos..].chars().next() {
                if ch.is_whitespace() {
                    temp_pos += ch.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Check for '{'
        self.input[temp_pos..].starts_with('{')
    }

    fn parse_quoted_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        self.expect_char('"')?;

        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance();
                return Ok(result);
            }
            if ch == '\\' {
                self.advance();
                if let Some(escaped_ch) = self.current_char() {
                    result.push(escaped_ch);
                    self.advance();
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(MakiError::rule_error(
            "gritql-parser",
            "Unterminated string in predicate",
        ))
    }

    /// Parse a regex string - similar to parse_quoted_string but preserves backslashes
    /// for regex escape sequences like \s, \d, etc.
    fn parse_regex_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        self.expect_char('"')?;

        let mut result = String::new();
        while let Some(ch) = self.current_char() {
            if ch == '"' {
                self.advance();
                return Ok(result);
            }
            if ch == '\\' {
                // For regex strings, preserve the backslash
                result.push(ch);
                self.advance();
                if let Some(escaped_ch) = self.current_char() {
                    result.push(escaped_ch);
                    self.advance();
                }
            } else {
                result.push(ch);
                self.advance();
            }
        }

        Err(MakiError::rule_error(
            "gritql-parser",
            "Unterminated regex string",
        ))
    }

    fn parse_identifier(&mut self) -> Result<String> {
        self.skip_whitespace();
        let start = self.pos;

        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                self.advance();
            } else {
                break;
            }
        }

        if start == self.pos {
            return Err(MakiError::rule_error(
                "gritql-parser",
                "Expected identifier",
            ));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<()> {
        if self.current_char() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(MakiError::rule_error(
                "gritql-parser",
                format!("Expected '{}', found {:?}", expected, self.current_char()),
            ))
        }
    }

    fn peek_word(&self, word: &str) -> bool {
        self.input[self.pos..].starts_with(word)
    }

    fn consume_word(&mut self, word: &str) -> Result<()> {
        if self.peek_word(word) {
            self.pos += word.len();
            Ok(())
        } else {
            Err(MakiError::rule_error(
                "gritql-parser",
                format!("Expected word '{word}'"),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_node() {
        let mut parser = GritQLParser::new("profile");
        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            GritPattern::NodeKind("profile".to_string())
        );
    }

    #[test]
    fn test_parse_profile_with_colon_syntax() {
        // This test demonstrates what "Profile: $name" parses to after Phase 3.1 enhancement
        let mut parser = GritQLParser::new(r#"Profile: $name where { $name <: r"^[a-z]" }"#);
        let result = parser.parse();
        println!("Parsed pattern: {:#?}", result);
        assert!(result.is_ok(), "Should parse without error");

        // After Phase 3.1: Profile: $name creates an Assignment pattern
        if let Ok(GritPattern::Where(base, predicate)) = result {
            println!("Base pattern: {:#?}", base);
            println!("Predicate: {:#?}", predicate);
            // The base should now be Assignment { var: "name", value: NodeKind("Profile") }
            match *base {
                GritPattern::Assignment { ref var, .. } => {
                    assert_eq!(var, "name");
                }
                _ => panic!("Expected Assignment pattern, got {:?}", base),
            }
        } else {
            panic!("Expected Where pattern");
        }
    }

    #[test]
    fn test_parse_where_clause() {
        let mut parser = GritQLParser::new("profile where { description }");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_variable() {
        let mut parser = GritQLParser::new("$name");
        let result = parser.parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), GritPattern::Variable("name".to_string()));
    }

    #[test]
    fn test_parse_assignment() {
        let mut parser = GritQLParser::new("$name = profile");
        let result = parser.parse();
        assert!(result.is_ok());
    }

    // ===== Phase 1 Enhancement Tests =====

    #[test]
    fn test_parse_match_value_or_expression() {
        let mut parser = GritQLParser::new(r#"profile where { $name <: or { "a", "b", "c" } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse or value expression");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { var, value } => {
                    assert_eq!(var, "name");
                    assert!(matches!(value, GritMatchValue::Or(_)));
                    if let GritMatchValue::Or(values) = value {
                        assert_eq!(values.len(), 3);
                    }
                }
                _ => panic!("Expected Match predicate"),
            }
        }
    }

    #[test]
    fn test_parse_match_value_and_expression() {
        let mut parser = GritQLParser::new(r#"profile where { $name <: and { r"^A", r"Z$" } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse and value expression");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { value, .. } => {
                    assert!(matches!(value, GritMatchValue::And(_)));
                    if let GritMatchValue::And(values) = value {
                        assert_eq!(values.len(), 2);
                        assert!(matches!(values[0], GritMatchValue::Regex(_)));
                    }
                }
                _ => panic!("Expected Match predicate"),
            }
        }
    }

    #[test]
    fn test_parse_or_predicate_block() {
        let mut parser =
            GritQLParser::new(r#"profile where { or { contains "=", startsWith "Alias" } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse or predicate block");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Or(preds) => {
                    assert_eq!(preds.len(), 2);
                    assert!(matches!(preds[0], GritPredicate::Contains { .. }));
                    assert!(matches!(preds[1], GritPredicate::StartsWith { .. }));
                }
                _ => panic!("Expected Or predicate, got {:?}", predicate),
            }
        }
    }

    #[test]
    fn test_parse_and_predicate_block() {
        let mut parser =
            GritQLParser::new(r#"profile where { and { contains "Profile", endsWith ">" } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse and predicate block");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::And(preds) => {
                    assert_eq!(preds.len(), 2);
                    assert!(matches!(preds[0], GritPredicate::Contains { .. }));
                    assert!(matches!(preds[1], GritPredicate::EndsWith { .. }));
                }
                _ => panic!("Expected And predicate"),
            }
        }
    }

    #[test]
    fn test_parse_implicit_context_contains() {
        let mut parser = GritQLParser::new(r#"profile where { contains "test" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse implicit contains");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Contains { var, .. } => {
                    assert_eq!(var, "_", "Should use implicit context");
                }
                _ => panic!("Expected Contains predicate"),
            }
        }
    }

    #[test]
    fn test_parse_implicit_context_starts_with() {
        let mut parser = GritQLParser::new(r#"profile where { startsWith "Prefix" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse implicit startsWith");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::StartsWith { var, .. } => {
                    assert_eq!(var, "_", "Should use implicit context");
                }
                _ => panic!("Expected StartsWith predicate"),
            }
        }
    }

    #[test]
    fn test_parse_implicit_context_ends_with() {
        let mut parser = GritQLParser::new(r#"profile where { endsWith "Suffix" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse implicit endsWith");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::EndsWith { var, .. } => {
                    assert_eq!(var, "_", "Should use implicit context");
                }
                _ => panic!("Expected EndsWith predicate"),
            }
        }
    }

    #[test]
    fn test_parse_nested_or_blocks() {
        let mut parser = GritQLParser::new(
            r#"profile where { or { or { contains "a", contains "b" }, contains "c" } }"#,
        );
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse nested or blocks");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Or(preds) => {
                    assert_eq!(preds.len(), 2);
                    // First should be nested Or
                    assert!(matches!(preds[0], GritPredicate::Or(_)));
                }
                _ => panic!("Expected Or predicate"),
            }
        }
    }

    #[test]
    fn test_parse_trailing_comma_in_or_block() {
        let mut parser =
            GritQLParser::new(r#"profile where { or { contains "a", contains "b", } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse or block with trailing comma");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Or(preds) => {
                    assert_eq!(preds.len(), 2, "Trailing comma should be ignored");
                }
                _ => panic!("Expected Or predicate"),
            }
        }
    }

    #[test]
    fn test_parse_complex_builtin_pattern() {
        // Test pattern similar to invalid-keyword rule
        let pattern = r#"identifier where { $identifier <: or { "Profil", "profil", "PROFILE" } }"#;
        let mut parser = GritQLParser::new(pattern);
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Should parse complex builtin pattern: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_parse_complex_malformed_alias_pattern() {
        // Test pattern similar to malformed-alias rule
        let pattern = r#"alias_declaration where { or { not contains "=", contains "==" } }"#;
        let mut parser = GritQLParser::new(pattern);
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Should parse malformed-alias pattern: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_peek_ahead_for_brace() {
        let parser = GritQLParser::new("or { something }");
        assert!(
            parser.peek_ahead_for_brace(),
            "Should detect brace after 'or'"
        );

        let parser2 = GritQLParser::new("or not something");
        assert!(
            !parser2.peek_ahead_for_brace(),
            "Should not detect brace when none exists"
        );
    }

    #[test]
    fn test_parse_regex_value() {
        let mut parser = GritQLParser::new(r#"profile where { $name <: r"^[A-Z]" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse regex value");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { value, .. } => {
                    assert!(matches!(value, GritMatchValue::Regex(_)));
                    if let GritMatchValue::Regex(pattern) = value {
                        assert_eq!(pattern, "^[A-Z]");
                    }
                }
                _ => panic!("Expected Match predicate"),
            }
        }
    }

    #[test]
    fn test_parse_string_value() {
        let mut parser = GritQLParser::new(r#"profile where { $name <: "test" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse string value");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { value, .. } => {
                    assert!(matches!(value, GritMatchValue::String(_)));
                    if let GritMatchValue::String(s) = value {
                        assert_eq!(s, "test");
                    }
                }
                _ => panic!("Expected Match predicate"),
            }
        }
    }

    #[test]
    fn test_parse_mixed_value_expression() {
        let mut parser =
            GritQLParser::new(r#"profile where { $name <: or { "string", r"regex" } }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse mixed value expression");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { value, .. } => {
                    if let GritMatchValue::Or(values) = value {
                        assert_eq!(values.len(), 2);
                        assert!(matches!(values[0], GritMatchValue::String(_)));
                        assert!(matches!(values[1], GritMatchValue::Regex(_)));
                    } else {
                        panic!("Expected Or value expression");
                    }
                }
                _ => panic!("Expected Match predicate"),
            }
        }
    }

    // ===== Phase 3.1 Enhancement Tests =====

    #[test]
    fn test_parse_variable_pattern_with_constraint() {
        // Test pattern: $profile_name where { $profile_name <: r"^[A-Z]" }
        let mut parser = GritQLParser::new(
            r#"profile where { $profile_name where { $profile_name <: r"^[A-Z]" } }"#,
        );
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Should parse variable pattern with constraint: {:?}",
            result.err()
        );

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::VariablePattern { var, constraint } => {
                    assert_eq!(var, "profile_name");
                    // Check that the constraint is a Match predicate
                    match *constraint {
                        GritPredicate::Match { var: inner_var, .. } => {
                            assert_eq!(inner_var, "profile_name");
                        }
                        _ => panic!("Expected Match predicate in constraint"),
                    }
                }
                _ => panic!("Expected VariablePattern, got {:?}", predicate),
            }
        } else {
            panic!("Expected Where pattern");
        }
    }

    #[test]
    fn test_parse_nested_variable_pattern() {
        // Test more complex nested pattern similar to builtin rules
        let pattern = r#"profile_declaration where {
            $profile_name where {
                and {
                    $profile_name <: r"Profile:\s*([a-zA-Z][a-zA-Z0-9_-]*)",
                    or {
                        not $profile_name <: r"^[A-Z]",
                        $profile_name <: r"[a-z][A-Z]"
                    }
                }
            }
        }"#;
        let mut parser = GritQLParser::new(pattern);
        let result = parser.parse();
        assert!(
            result.is_ok(),
            "Should parse nested variable pattern: {:?}",
            result.err()
        );

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::VariablePattern { var, constraint } => {
                    assert_eq!(var, "profile_name");
                    // Check that the constraint is an And predicate
                    match *constraint {
                        GritPredicate::And(preds) => {
                            assert_eq!(preds.len(), 2, "Should have 2 predicates in and block");
                        }
                        _ => panic!("Expected And predicate in constraint"),
                    }
                }
                _ => panic!("Expected VariablePattern, got {:?}", predicate),
            }
        }
    }

    #[test]
    fn test_parse_simple_variable_without_where() {
        // Ensure $var without 'where' still works as before
        let mut parser = GritQLParser::new(r#"profile where { $name <: r"^[A-Z]" }"#);
        let result = parser.parse();
        assert!(result.is_ok(), "Should parse simple variable predicate");

        if let Ok(GritPattern::Where(_, predicate)) = result {
            match *predicate {
                GritPredicate::Match { var, .. } => {
                    assert_eq!(var, "name");
                }
                _ => panic!("Expected Match predicate, got {:?}", predicate),
            }
        }
    }
}
