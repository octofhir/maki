//! RuleSet integration for build pipeline
//!
//! This module integrates the semantic::ruleset expander with the build pipeline,
//! extracting RuleSets from parsed FSH files and expanding insert statements.

use crate::cst::FshSyntaxNode;
use crate::cst::ast::{AstNode, Document, InsertRule, Rule};
use crate::semantic::ruleset::{RuleSet, RuleSetError, RuleSetExpander, RuleSetInsert};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{info, warn};

/// RuleSet integration errors
#[derive(Debug, Error)]
pub enum RuleSetIntegrationError {
    #[error("RuleSet expansion error: {0}")]
    ExpansionError(#[from] RuleSetError),

    #[error("Failed to parse RuleSet: {0}")]
    ParseError(String),
}

/// RuleSet collector and expander for build pipeline
pub struct RuleSetProcessor {
    expander: RuleSetExpander,
    rulesets_collected: usize,
    inserts_expanded: usize,
}

impl RuleSetProcessor {
    /// Create a new RuleSet processor
    pub fn new() -> Self {
        Self {
            expander: RuleSetExpander::new(),
            rulesets_collected: 0,
            inserts_expanded: 0,
        }
    }

    /// Consume the processor and return the underlying expander
    pub fn into_expander(self) -> RuleSetExpander {
        self.expander
    }

    /// Collect all RuleSets from parsed FSH files (Phase 1a)
    ///
    /// This scans all parsed files and extracts RuleSet definitions.
    pub fn collect_rulesets(
        &mut self,
        parsed_files: &[(PathBuf, FshSyntaxNode)],
    ) -> Result<(), RuleSetIntegrationError> {
        info!("🔄 Phase 1a: Collecting RuleSets...");

        for (file_path, root) in parsed_files {
            let document = match Document::cast(root.clone()) {
                Some(doc) => doc,
                None => {
                    warn!("Skipping file {:?}: not a valid document", file_path);
                    continue;
                }
            };

            // Extract RuleSets from the document
            self.extract_rulesets_from_document(&document, file_path)?;
        }

        info!("  Found {} RuleSets", self.rulesets_collected);
        Ok(())
    }

    /// Extract RuleSets from a document
    fn extract_rulesets_from_document(
        &mut self,
        document: &Document,
        file_path: &PathBuf,
    ) -> Result<(), RuleSetIntegrationError> {
        for rs in document.rule_sets() {
            let Some(name) = rs.name() else {
                continue;
            };

            let params = rs.parameters();
            let rules: Vec<String> = rs.rules().map(|r| Self::serialize_rule(&r)).collect();
            let range = rs.syntax().text_range();

            let ruleset = RuleSet {
                name: name.clone(),
                parameters: params,
                rules,
                source_file: file_path.clone(),
                source_range: (range.start().into()..range.end().into()),
            };

            self.expander.register_ruleset(ruleset);
            self.rulesets_collected += 1;
        }

        Ok(())
    }

    fn serialize_rule(rule: &Rule) -> String {
        match rule {
            Rule::FixedValue(fv) => {
                if let Some(path) = fv.path().map(|p| p.as_string())
                    && let Some(value) = fv.value()
                {
                    format!("* {} = {}", path, value)
                } else {
                    rule.syntax().text().to_string()
                }
            }
            _ => rule.syntax().text().to_string(),
        }
    }

    /// Expand insert rules in all entities (Phase 1b)
    ///
    /// This processes all entities and expands any insert statements.
    pub fn expand_all_inserts(
        &mut self,
        parsed_files: &[(PathBuf, FshSyntaxNode)],
    ) -> Result<HashMap<String, Vec<String>>, RuleSetIntegrationError> {
        info!("🔄 Phase 1b: Expanding InsertRules...");

        let mut expanded_rules_map = HashMap::new();

        for (file_path, root) in parsed_files {
            let document = match Document::cast(root.clone()) {
                Some(doc) => doc,
                None => continue,
            };

            // Process profiles
            for profile in document.profiles() {
                let name = profile.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(profile.syntax(), file_path)
                    && !expanded.is_empty()
                {
                    expanded_rules_map.insert(name.clone(), expanded);
                    self.inserts_expanded += 1;
                }
            }

            // Process extensions
            for extension in document.extensions() {
                let name = extension.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(extension.syntax(), file_path)
                    && !expanded.is_empty()
                {
                    expanded_rules_map.insert(name.clone(), expanded);
                    self.inserts_expanded += 1;
                }
            }

            // Process instances
            for instance in document.instances() {
                let name = instance.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(instance.syntax(), file_path)
                    && !expanded.is_empty()
                {
                    expanded_rules_map.insert(name.clone(), expanded);
                    self.inserts_expanded += 1;
                }
            }

            // Process value sets
            for valueset in document.value_sets() {
                let name = valueset.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(valueset.syntax(), file_path)
                    && !expanded.is_empty()
                {
                    expanded_rules_map.insert(name.clone(), expanded);
                    self.inserts_expanded += 1;
                }
            }

            // Process code systems
            for codesystem in document.code_systems() {
                let name = codesystem.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(codesystem.syntax(), file_path)
                    && !expanded.is_empty()
                {
                    expanded_rules_map.insert(name.clone(), expanded);
                    self.inserts_expanded += 1;
                }
            }
        }

        info!("  Expanded {} InsertRules", self.inserts_expanded);
        Ok(expanded_rules_map)
    }

    /// Expand insert rules in a single entity
    fn expand_entity_inserts(
        &self,
        entity_node: &FshSyntaxNode,
        file_path: &PathBuf,
    ) -> Result<Vec<String>, RuleSetIntegrationError> {
        let mut expanded_rules = Vec::new();

        // Iterate through rules in the entity
        for rule_node in entity_node.children() {
            // Check if this is an insert statement
            if let Some(insert) = self.try_parse_insert(&rule_node) {
                // Expand the insert using the expander
                match self.expander.expand(&insert) {
                    Ok(rules) => {
                        expanded_rules.extend(rules);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to expand insert '{}' in {:?}: {}",
                            insert.ruleset_name, file_path, e
                        );
                        // Continue with other rules
                    }
                }
            }
        }

        Ok(expanded_rules)
    }

    /// Try to parse an insert statement from a syntax node
    fn try_parse_insert(&self, node: &FshSyntaxNode) -> Option<RuleSetInsert> {
        if let Some(insert) = InsertRule::cast(node.clone()) {
            let name = insert.ruleset_reference()?;
            let args = insert.arguments();
            let range = insert.syntax().text_range();
            return Some(RuleSetInsert {
                ruleset_name: name,
                arguments: args,
                source_range: (range.start().into()..range.end().into()),
            });
        }

        // Handle code insert (used in CodeSystem)
        if let Some(code_insert) = crate::cst::ast::CodeInsertRule::cast(node.clone()) {
            let name = code_insert.ruleset_reference()?;
            let args = code_insert.arguments();
            let range = code_insert.syntax().text_range();
            return Some(RuleSetInsert {
                ruleset_name: name,
                arguments: args,
                source_range: (range.start().into()..range.end().into()),
            });
        }

        None
    }

    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.rulesets_collected, self.inserts_expanded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::parse_fsh;
    use crate::semantic::ruleset::RuleSetExpander;

    #[test]
    fn collect_ruleset_from_file() {
        let content = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../mcode-ig/input/fsh/EX_Staging_Other.fsh"),
        )
        .unwrap();
        let (cst, lex, parse) = parse_fsh(&content);
        assert!(lex.is_empty() && parse.is_empty());

        let parsed_files = vec![(PathBuf::from("EX_Staging_Other.fsh"), cst)];

        let mut processor = RuleSetProcessor::new();
        processor.collect_rulesets(&parsed_files).unwrap();

        let (rulesets_found, _) = processor.stats();
        assert!(rulesets_found > 0);

        let expander: RuleSetExpander = processor.into_expander();
        assert!(expander.has_ruleset("StagingInstanceRuleSet"));
    }

    #[test]
    fn test_ruleset_processor_creation() {
        let processor = RuleSetProcessor::new();
        let (collected, expanded) = processor.stats();
        assert_eq!(collected, 0);
        assert_eq!(expanded, 0);
    }

    #[test]
    fn test_collect_rulesets_empty() {
        let mut processor = RuleSetProcessor::new();
        let parsed_files: Vec<(PathBuf, FshSyntaxNode)> = vec![];
        let result = processor.collect_rulesets(&parsed_files);
        assert!(result.is_ok());
    }
}

impl Default for RuleSetProcessor {
    fn default() -> Self {
        Self::new()
    }
}
