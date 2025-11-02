//! RuleSet integration for build pipeline
//!
//! This module integrates the semantic::ruleset expander with the build pipeline,
//! extracting RuleSets from parsed FSH files and expanding insert statements.

use crate::cst::FshSyntaxNode;
use crate::cst::ast::{AstNode, CodeSystem, Document, Extension, Instance, Profile, ValueSet};
use crate::semantic::ruleset::{RuleSet, RuleSetError, RuleSetExpander, RuleSetInsert};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn};

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
        // Iterate through all children looking for RuleSet definitions
        for child in document.syntax().children() {
            // Check if this is a RuleSet definition
            if let Some(ruleset_text) = self.try_parse_ruleset(&child, file_path) {
                // For now, we'll store the raw text and parse parameters/rules later
                // This is a simplified implementation
                debug!("Found RuleSet in {:?}", file_path);
                self.rulesets_collected += 1;
            }
        }

        Ok(())
    }

    /// Try to parse a RuleSet from a syntax node
    fn try_parse_ruleset(&self, node: &FshSyntaxNode, file_path: &PathBuf) -> Option<String> {
        // This is a placeholder - actual implementation would:
        // 1. Check if node is a RuleSet definition (RulesetKw)
        // 2. Extract name and parameters
        // 3. Extract rules
        // 4. Create RuleSet struct
        // 5. Register with expander

        // For now, return None as we need CST parser support
        None
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
                if let Ok(expanded) = self.expand_entity_inserts(&profile.syntax(), file_path) {
                    if !expanded.is_empty() {
                        expanded_rules_map.insert(name.clone(), expanded);
                        self.inserts_expanded += 1;
                    }
                }
            }

            // Process extensions
            for extension in document.extensions() {
                let name = extension.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(&extension.syntax(), file_path) {
                    if !expanded.is_empty() {
                        expanded_rules_map.insert(name.clone(), expanded);
                        self.inserts_expanded += 1;
                    }
                }
            }

            // Process instances
            for instance in document.instances() {
                let name = instance.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(&instance.syntax(), file_path) {
                    if !expanded.is_empty() {
                        expanded_rules_map.insert(name.clone(), expanded);
                        self.inserts_expanded += 1;
                    }
                }
            }

            // Process value sets
            for valueset in document.value_sets() {
                let name = valueset.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(&valueset.syntax(), file_path) {
                    if !expanded.is_empty() {
                        expanded_rules_map.insert(name.clone(), expanded);
                        self.inserts_expanded += 1;
                    }
                }
            }

            // Process code systems
            for codesystem in document.code_systems() {
                let name = codesystem.name().unwrap_or_else(|| "Unknown".to_string());
                if let Ok(expanded) = self.expand_entity_inserts(&codesystem.syntax(), file_path) {
                    if !expanded.is_empty() {
                        expanded_rules_map.insert(name.clone(), expanded);
                        self.inserts_expanded += 1;
                    }
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
        // This is a placeholder - actual implementation would:
        // 1. Check if node contains InsertKw
        // 2. Extract RuleSet name
        // 3. Extract arguments
        // 4. Create RuleSetInsert struct

        // For now, return None as we need CST parser support
        None
    }

    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.rulesets_collected, self.inserts_expanded)
    }
}

impl Default for RuleSetProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
