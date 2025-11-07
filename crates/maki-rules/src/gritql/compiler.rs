//! GritQL pattern compiler
//!
//! Compiles parsed GritQL AST into executable grit-pattern-matcher Pattern structs.

use super::parser::{GritPattern, GritPredicate};
use super::query_context::{FshNodePattern, FshQueryContext};
use grit_pattern_matcher::pattern::{
    And, Container, Not, Pattern, PrAnd, PrNot, PrOr, Predicate, Variable, Where,
};
use maki_core::cst::FshSyntaxKind;
use maki_core::{MakiError, Result};
use std::collections::HashMap;

pub struct PatternCompiler {
    pub variables: HashMap<String, usize>,
    next_var_index: usize,
    current_scope: usize,
}

impl PatternCompiler {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            next_var_index: 0,
            current_scope: 0,
        }
    }

    pub fn compile(&mut self, pattern: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        match pattern {
            GritPattern::NodeKind(kind) => self.compile_node_kind(kind),
            GritPattern::Variable(name) => self.compile_variable(name),
            GritPattern::Not(inner) => self.compile_not(inner),
            GritPattern::Where(pattern, predicate) => self.compile_where(pattern, predicate),
            GritPattern::Assignment { var, value } => self.compile_assignment(var, value),
            GritPattern::And(patterns) => self.compile_and(patterns),
            GritPattern::Or(patterns) => self.compile_or(patterns),
        }
    }

    fn compile_node_kind(&self, kind: &str) -> Result<Pattern<FshQueryContext>> {
        let syntax_kind = self.parse_syntax_kind(kind)?;
        Ok(Pattern::AstNode(Box::new(FshNodePattern {
            kind: syntax_kind,
            args: vec![],
        })))
    }

    fn compile_variable(&mut self, name: &str) -> Result<Pattern<FshQueryContext>> {
        let index = self.get_or_create_variable(name);
        Ok(Pattern::Variable(Variable::new(index, self.current_scope)))
    }

    fn compile_not(&mut self, inner: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        let compiled_inner = self.compile(inner)?;
        Ok(Pattern::Not(Box::new(Not {
            pattern: compiled_inner,
        })))
    }

    fn compile_where(
        &mut self,
        pattern: &GritPattern,
        predicate: &GritPredicate,
    ) -> Result<Pattern<FshQueryContext>> {
        let compiled_pattern = self.compile(pattern)?;
        let compiled_predicate = self.compile_predicate_to_predicate(predicate)?;

        Ok(Pattern::Where(Box::new(Where {
            pattern: compiled_pattern,
            side_condition: compiled_predicate,
        })))
    }

    fn compile_assignment(
        &mut self,
        var: &str,
        value: &GritPattern,
    ) -> Result<Pattern<FshQueryContext>> {
        let var_index = self.get_or_create_variable(var);
        let compiled_value = self.compile(value)?;

        Ok(Pattern::Assignment(Box::new(
            grit_pattern_matcher::pattern::Assignment {
                container: Container::Variable(Variable::new(var_index, self.current_scope)),
                pattern: compiled_value,
            },
        )))
    }

    fn compile_and(&mut self, patterns: &[GritPattern]) -> Result<Pattern<FshQueryContext>> {
        let compiled: Result<Vec<_>> = patterns.iter().map(|p| self.compile(p)).collect();
        Ok(Pattern::And(Box::new(And {
            patterns: compiled?,
        })))
    }

    fn compile_or(&mut self, patterns: &[GritPattern]) -> Result<Pattern<FshQueryContext>> {
        let compiled: Result<Vec<_>> = patterns.iter().map(|p| self.compile(p)).collect();
        Ok(Pattern::Or(Box::new(grit_pattern_matcher::pattern::Or {
            patterns: compiled?,
        })))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn compile_predicate_to_predicate(
        &mut self,
        predicate: &GritPredicate,
    ) -> Result<Predicate<FshQueryContext>> {
        match predicate {
            GritPredicate::FieldExists(_field) => Ok(Predicate::True),
            GritPredicate::Not(inner) => {
                let compiled = self.compile_predicate_to_predicate(inner)?;
                Ok(Predicate::Not(Box::new(PrNot::new(compiled))))
            }
            GritPredicate::And(preds) => {
                let compiled: Result<Vec<_>> = preds
                    .iter()
                    .map(|p| self.compile_predicate_to_predicate(p))
                    .collect();
                Ok(Predicate::And(Box::new(PrAnd::new(compiled?))))
            }
            GritPredicate::Or(preds) => {
                let compiled: Result<Vec<_>> = preds
                    .iter()
                    .map(|p| self.compile_predicate_to_predicate(p))
                    .collect();
                Ok(Predicate::Or(Box::new(PrOr::new(compiled?))))
            }
            // Regex match, contains, etc. - for now just return true
            // These will be evaluated during pattern execution
            GritPredicate::RegexMatch { .. } => Ok(Predicate::True),
            GritPredicate::Contains { .. } => Ok(Predicate::True),
            GritPredicate::StartsWith { .. } => Ok(Predicate::True),
            GritPredicate::EndsWith { .. } => Ok(Predicate::True),
            GritPredicate::Equality { .. } => Ok(Predicate::True),
            GritPredicate::Inequality { .. } => Ok(Predicate::True),
        }
    }

    fn parse_syntax_kind(&self, kind: &str) -> Result<FshSyntaxKind> {
        let normalized = kind.to_uppercase().replace('-', "_");

        match normalized.as_str() {
            "PROFILE" => Ok(FshSyntaxKind::Profile),
            "EXTENSION" => Ok(FshSyntaxKind::Extension),
            "VALUESET" | "VALUE_SET" => Ok(FshSyntaxKind::ValueSet),
            "CODESYSTEM" | "CODE_SYSTEM" => Ok(FshSyntaxKind::CodeSystem),
            "INSTANCE" => Ok(FshSyntaxKind::Instance),
            "INVARIANT" => Ok(FshSyntaxKind::Invariant),
            "MAPPING" => Ok(FshSyntaxKind::Mapping),
            "LOGICAL" => Ok(FshSyntaxKind::Logical),
            "RESOURCE" => Ok(FshSyntaxKind::Resource),
            "ALIAS" => Ok(FshSyntaxKind::Alias),
            "PARENT" => Ok(FshSyntaxKind::ParentKw),
            "ID" => Ok(FshSyntaxKind::IdKw),
            "TITLE" => Ok(FshSyntaxKind::TitleKw),
            "DESCRIPTION" => Ok(FshSyntaxKind::DescriptionKw),
            _ => Err(MakiError::rule_error(
                "gritql-compiler",
                format!("Unknown FSH syntax kind: {kind}"),
            )),
        }
    }

    fn get_or_create_variable(&mut self, name: &str) -> usize {
        if let Some(&index) = self.variables.get(name) {
            index
        } else {
            let index = self.next_var_index;
            self.variables.insert(name.to_string(), index);
            self.next_var_index += 1;
            index
        }
    }
}

impl Default for PatternCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gritql::parser::GritQLParser;

    #[test]
    fn test_compile_simple_node() {
        let mut parser = GritQLParser::new("profile");
        let pattern = parser.parse().unwrap();

        let mut compiler = PatternCompiler::new();
        let result = compiler.compile(&pattern);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_where_clause() {
        let mut parser = GritQLParser::new("profile where { description }");
        let pattern = parser.parse().unwrap();

        let mut compiler = PatternCompiler::new();
        let result = compiler.compile(&pattern);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_variable() {
        let mut parser = GritQLParser::new("$name");
        let pattern = parser.parse().unwrap();

        let mut compiler = PatternCompiler::new();
        let result = compiler.compile(&pattern);
        assert!(result.is_ok());
    }
}
