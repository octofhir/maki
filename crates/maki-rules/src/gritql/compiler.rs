//! GritQL pattern compiler
//!
//! Compiles parsed GritQL AST into executable grit-pattern-matcher Pattern structs.

use super::parser::{GritMatchValue, GritPattern, GritPredicate};
use super::query_context::{FshNodePattern, FshQueryContext};
use grit_pattern_matcher::pattern::Match as MatchPredicate;
use grit_pattern_matcher::pattern::{
    And, Container, Equal, Not, Pattern, PrAnd, PrNot, PrOr, Predicate, RegexLike, RegexPattern,
    StringConstant, Variable, Where,
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
            GritPattern::Maybe(inner) => self.compile_maybe(inner),
            GritPattern::Any(patterns) => self.compile_any(patterns),
            GritPattern::Contains(inner) => self.compile_contains(inner),
            GritPattern::Within(inner) => self.compile_within(inner),
            GritPattern::After(inner) => self.compile_after(inner),
            GritPattern::Bubble { pattern, args } => self.compile_bubble(pattern, args),
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

    fn compile_maybe(&mut self, inner: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        let compiled_inner = self.compile(inner)?;
        Ok(Pattern::Maybe(Box::new(
            grit_pattern_matcher::pattern::Maybe::new(compiled_inner),
        )))
    }

    fn compile_any(&mut self, patterns: &[GritPattern]) -> Result<Pattern<FshQueryContext>> {
        let compiled: Result<Vec<_>> = patterns.iter().map(|p| self.compile(p)).collect();
        Ok(Pattern::Any(Box::new(
            grit_pattern_matcher::pattern::Any::new(compiled?),
        )))
    }

    fn compile_contains(&mut self, inner: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        let compiled_inner = self.compile(inner)?;
        Ok(Pattern::Contains(Box::new(
            grit_pattern_matcher::pattern::Contains::new(
                compiled_inner,
                None, // until
            ),
        )))
    }

    fn compile_within(&mut self, inner: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        let compiled_inner = self.compile(inner)?;
        Ok(Pattern::Within(Box::new(
            grit_pattern_matcher::pattern::Within::new(
                compiled_inner,
                None, // until
            ),
        )))
    }

    fn compile_after(&mut self, inner: &GritPattern) -> Result<Pattern<FshQueryContext>> {
        let compiled_inner = self.compile(inner)?;
        Ok(Pattern::After(Box::new(
            grit_pattern_matcher::pattern::After::new(compiled_inner),
        )))
    }

    fn compile_bubble(
        &mut self,
        _pattern: &GritPattern,
        _args: &[String],
    ) -> Result<Pattern<FshQueryContext>> {
        // Bubble is complex and requires PatternDefinition
        // For now, just return Underscore (matches anything)
        // Will be properly implemented in Phase 2
        Ok(Pattern::Underscore)
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
            GritPredicate::Match { var, value } => {
                let var_index = self.get_or_create_variable(var);
                self.compile_match_predicate(var_index, value)
            }
            GritPredicate::Contains { var, value } => {
                let var_index = self.get_or_create_variable(var);
                self.compile_contains_predicate(var_index, value)
            }
            GritPredicate::StartsWith { var, value } => {
                let var_index = self.get_or_create_variable(var);
                self.compile_starts_with_predicate(var_index, value)
            }
            GritPredicate::EndsWith { var, value } => {
                let var_index = self.get_or_create_variable(var);
                self.compile_ends_with_predicate(var_index, value)
            }
            GritPredicate::Equality { left, right } => {
                let var_index = self.get_or_create_variable(left);
                Ok(Predicate::Equal(Box::new(Equal::new(
                    Variable::new(var_index, self.current_scope),
                    Pattern::StringConstant(StringConstant::new(right.clone())),
                ))))
            }
            GritPredicate::Inequality { left, right } => {
                let var_index = self.get_or_create_variable(left);
                Ok(Predicate::Not(Box::new(PrNot::new(Predicate::Equal(
                    Box::new(Equal::new(
                        Variable::new(var_index, self.current_scope),
                        Pattern::StringConstant(StringConstant::new(right.clone())),
                    )),
                )))))
            }
            GritPredicate::VariablePattern { var, constraint } => {
                // For now, just compile the constraint and track the variable
                // The variable binding will be handled in the executor
                let _var_index = self.get_or_create_variable(var);
                self.compile_predicate_to_predicate(constraint)
            }
        }
    }

    /// Compile a match predicate (regex or string match)
    fn compile_match_predicate(
        &mut self,
        var_index: usize,
        value: &GritMatchValue,
    ) -> Result<Predicate<FshQueryContext>> {
        match value {
            GritMatchValue::String(s) => Ok(Predicate::Equal(Box::new(Equal::new(
                Variable::new(var_index, self.current_scope),
                Pattern::StringConstant(StringConstant::new(s.clone())),
            )))),
            GritMatchValue::Regex(pattern) => Ok(Predicate::Match(Box::new(MatchPredicate::new(
                Container::Variable(Variable::new(var_index, self.current_scope)),
                Some(Pattern::Regex(Box::new(RegexPattern::new(
                    RegexLike::Regex(pattern.clone()),
                    vec![],
                )))),
            )))),
            GritMatchValue::Or(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_match_predicate(var_index, v))
                    .collect();
                Ok(Predicate::Or(Box::new(PrOr::new(preds?))))
            }
            GritMatchValue::And(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_match_predicate(var_index, v))
                    .collect();
                Ok(Predicate::And(Box::new(PrAnd::new(preds?))))
            }
        }
    }

    /// Compile a contains predicate using regex
    fn compile_contains_predicate(
        &mut self,
        var_index: usize,
        value: &GritMatchValue,
    ) -> Result<Predicate<FshQueryContext>> {
        match value {
            GritMatchValue::String(s) => {
                // Convert contains "foo" to regex .*foo.*
                let regex_pattern = format!(".*{}.*", regex::escape(s));
                Ok(Predicate::Match(Box::new(MatchPredicate::new(
                    Container::Variable(Variable::new(var_index, self.current_scope)),
                    Some(Pattern::Regex(Box::new(RegexPattern::new(
                        RegexLike::Regex(regex_pattern),
                        vec![],
                    )))),
                ))))
            }
            GritMatchValue::Regex(pattern) => Ok(Predicate::Match(Box::new(MatchPredicate::new(
                Container::Variable(Variable::new(var_index, self.current_scope)),
                Some(Pattern::Regex(Box::new(RegexPattern::new(
                    RegexLike::Regex(pattern.clone()),
                    vec![],
                )))),
            )))),
            GritMatchValue::Or(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_contains_predicate(var_index, v))
                    .collect();
                Ok(Predicate::Or(Box::new(PrOr::new(preds?))))
            }
            GritMatchValue::And(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_contains_predicate(var_index, v))
                    .collect();
                Ok(Predicate::And(Box::new(PrAnd::new(preds?))))
            }
        }
    }

    /// Compile a starts with predicate using regex
    fn compile_starts_with_predicate(
        &mut self,
        var_index: usize,
        value: &GritMatchValue,
    ) -> Result<Predicate<FshQueryContext>> {
        match value {
            GritMatchValue::String(s) => {
                // Convert startsWith "foo" to regex ^foo
                let regex_pattern = format!("^{}", regex::escape(s));
                Ok(Predicate::Match(Box::new(MatchPredicate::new(
                    Container::Variable(Variable::new(var_index, self.current_scope)),
                    Some(Pattern::Regex(Box::new(RegexPattern::new(
                        RegexLike::Regex(regex_pattern),
                        vec![],
                    )))),
                ))))
            }
            GritMatchValue::Regex(pattern) => Ok(Predicate::Match(Box::new(MatchPredicate::new(
                Container::Variable(Variable::new(var_index, self.current_scope)),
                Some(Pattern::Regex(Box::new(RegexPattern::new(
                    RegexLike::Regex(pattern.clone()),
                    vec![],
                )))),
            )))),
            GritMatchValue::Or(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_starts_with_predicate(var_index, v))
                    .collect();
                Ok(Predicate::Or(Box::new(PrOr::new(preds?))))
            }
            GritMatchValue::And(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_starts_with_predicate(var_index, v))
                    .collect();
                Ok(Predicate::And(Box::new(PrAnd::new(preds?))))
            }
        }
    }

    /// Compile an ends with predicate using regex
    fn compile_ends_with_predicate(
        &mut self,
        var_index: usize,
        value: &GritMatchValue,
    ) -> Result<Predicate<FshQueryContext>> {
        match value {
            GritMatchValue::String(s) => {
                // Convert endsWith "foo" to regex foo$
                let regex_pattern = format!("{}$", regex::escape(s));
                Ok(Predicate::Match(Box::new(MatchPredicate::new(
                    Container::Variable(Variable::new(var_index, self.current_scope)),
                    Some(Pattern::Regex(Box::new(RegexPattern::new(
                        RegexLike::Regex(regex_pattern),
                        vec![],
                    )))),
                ))))
            }
            GritMatchValue::Regex(pattern) => Ok(Predicate::Match(Box::new(MatchPredicate::new(
                Container::Variable(Variable::new(var_index, self.current_scope)),
                Some(Pattern::Regex(Box::new(RegexPattern::new(
                    RegexLike::Regex(pattern.clone()),
                    vec![],
                )))),
            )))),
            GritMatchValue::Or(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_ends_with_predicate(var_index, v))
                    .collect();
                Ok(Predicate::Or(Box::new(PrOr::new(preds?))))
            }
            GritMatchValue::And(values) => {
                let preds: Result<Vec<_>> = values
                    .iter()
                    .map(|v| self.compile_ends_with_predicate(var_index, v))
                    .collect();
                Ok(Predicate::And(Box::new(PrAnd::new(preds?))))
            }
        }
    }

    fn parse_syntax_kind(&self, kind: &str) -> Result<FshSyntaxKind> {
        let normalized = kind.to_uppercase().replace('-', "_");

        match normalized.as_str() {
            "PROFILE" | "PROFILE_DECLARATION" => Ok(FshSyntaxKind::Profile),
            "EXTENSION" | "EXTENSION_DECLARATION" => Ok(FshSyntaxKind::Extension),
            "VALUESET" | "VALUE_SET" | "VALUESET_DECLARATION" => Ok(FshSyntaxKind::ValueSet),
            "CODESYSTEM" | "CODE_SYSTEM" | "CODESYSTEM_DECLARATION" => {
                Ok(FshSyntaxKind::CodeSystem)
            }
            "INSTANCE" | "INSTANCE_DECLARATION" => Ok(FshSyntaxKind::Instance),
            "INVARIANT" | "INVARIANT_DECLARATION" => Ok(FshSyntaxKind::Invariant),
            "MAPPING" | "MAPPING_DECLARATION" => Ok(FshSyntaxKind::Mapping),
            "LOGICAL" | "LOGICAL_DECLARATION" => Ok(FshSyntaxKind::Logical),
            "RESOURCE" | "RESOURCE_DECLARATION" => Ok(FshSyntaxKind::Resource),
            "ALIAS" | "ALIAS_DECLARATION" => Ok(FshSyntaxKind::Alias),
            "PARENT" => Ok(FshSyntaxKind::ParentKw),
            "ID" => Ok(FshSyntaxKind::IdKw),
            "TITLE" => Ok(FshSyntaxKind::TitleKw),
            "DESCRIPTION" => Ok(FshSyntaxKind::DescriptionKw),
            // Generic node types - map to closest equivalent
            "IDENTIFIER" | "STATUS_FIELD" | "FIELD" => Ok(FshSyntaxKind::Ident),
            "SLICING_RULE" | "SLICING" => Ok(FshSyntaxKind::PathRule), // closest match
            "CARET_RULE" | "CARET_PATH" => Ok(FshSyntaxKind::CardRule),
            "ASSIGNMENT_RULE" => Ok(FshSyntaxKind::PathRule), // closest match
            "BINDING_RULE" => Ok(FshSyntaxKind::PathRule),    // closest match
            "CARDINALITY_RULE" | "CARDINALITY" => Ok(FshSyntaxKind::CardRule),
            "FLAG_RULE" => Ok(FshSyntaxKind::PathRule), // closest match
            "VALUE_SET_COMPONENT" => Ok(FshSyntaxKind::ValueSet),
            "CODE_CARDINALITY_RULE" => Ok(FshSyntaxKind::CardRule),
            "CONTAINS_RULE" => Ok(FshSyntaxKind::ContainsRule),
            "ONLY_RULE" => Ok(FshSyntaxKind::OnlyRule),
            "OBEYS_RULE" => Ok(FshSyntaxKind::ObeysRule),
            "INSERT_RULE" => Ok(FshSyntaxKind::InsertRule),
            "PATH_RULE" => Ok(FshSyntaxKind::PathRule),
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
