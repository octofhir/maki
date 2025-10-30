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
    FieldExists(String),
    Not(Box<GritPredicate>),
    And(Vec<GritPredicate>),
    Or(Vec<GritPredicate>),
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

        if self.peek_word("not") {
            self.consume_word("not")?;
            self.skip_whitespace();
            let pred = self.parse_predicate()?;
            self.skip_whitespace();
            self.expect_char('}')?;
            return Ok(GritPredicate::Not(Box::new(pred)));
        }

        let field = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect_char('}')?;

        Ok(GritPredicate::FieldExists(field))
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
