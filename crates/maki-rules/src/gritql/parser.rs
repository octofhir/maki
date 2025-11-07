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
    /// Regex match: $var <: r"pattern"
    RegexMatch { var: String, regex: String },
    /// Contains: $var contains "substring"
    Contains { var: String, substring: String },
    /// Starts with: $var startsWith "prefix"
    StartsWith { var: String, prefix: String },
    /// Ends with: $var endsWith "suffix"
    EndsWith { var: String, suffix: String },
    /// Equality: $a == "value"
    Equality { left: String, right: String },
    /// Inequality: $a != "value"
    Inequality { left: String, right: String },
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

    fn parse_predicate_inner(&mut self) -> Result<GritPredicate> {
        let first = self.parse_simple_predicate()?;

        self.skip_whitespace();

        // Check for 'and' or 'or' operators
        if self.peek_word("and") {
            let mut preds = vec![first];
            while self.peek_word("and") {
                self.consume_word("and")?;
                self.skip_whitespace();
                preds.push(self.parse_simple_predicate()?);
                self.skip_whitespace();
            }
            return Ok(GritPredicate::And(preds));
        }

        if self.peek_word("or") {
            let mut preds = vec![first];
            while self.peek_word("or") {
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

        // Parse variable or field name (might start with $)
        if self.current_char() == Some('$') {
            self.advance();
        }

        let field_or_var = self.parse_identifier()?;
        self.skip_whitespace();

        // Check for operators
        // <: (regex match)
        if self.peek_word("<:") {
            self.consume_word("<:")?;
            self.skip_whitespace();
            let regex = self.parse_string_or_regex()?;
            return Ok(GritPredicate::RegexMatch {
                var: field_or_var,
                regex,
            });
        }

        // contains
        if self.peek_word("contains") {
            self.consume_word("contains")?;
            self.skip_whitespace();
            let substring = self.parse_quoted_string()?;
            return Ok(GritPredicate::Contains {
                var: field_or_var,
                substring,
            });
        }

        // startsWith
        if self.peek_word("startsWith") {
            self.consume_word("startsWith")?;
            self.skip_whitespace();
            let prefix = self.parse_quoted_string()?;
            return Ok(GritPredicate::StartsWith {
                var: field_or_var,
                prefix,
            });
        }

        // endsWith
        if self.peek_word("endsWith") {
            self.consume_word("endsWith")?;
            self.skip_whitespace();
            let suffix = self.parse_quoted_string()?;
            return Ok(GritPredicate::EndsWith {
                var: field_or_var,
                suffix,
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

    fn parse_string_or_regex(&mut self) -> Result<String> {
        self.skip_whitespace();

        // Check for r"..." regex
        if self.peek_word("r\"") {
            self.consume_word("r")?;
            return self.parse_quoted_string();
        }

        // Otherwise just a regular quoted string
        self.parse_quoted_string()
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
}
