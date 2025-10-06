//! Load GritQL rules from .grit files - CST-based
//!
//! This module handles discovering and loading GritQL patterns from directories.

use super::executor::{CompiledGritQLPattern, GritQLCompiler};
use super::registry::GritQLRegistry;
use fsh_lint_core::{FshLintError, Result};
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Loader for GritQL rules from filesystem
pub struct GritQLRuleLoader {
    rules: Vec<LoadedRule>,
    registry: GritQLRegistry,
}

/// A loaded GritQL rule with metadata
#[derive(Debug, Clone)]
pub struct LoadedRule {
    id: String,
    pattern: CompiledGritQLPattern,
    source_path: PathBuf,
}

impl LoadedRule {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pattern(&self) -> &CompiledGritQLPattern {
        &self.pattern
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
}

impl GritQLRuleLoader {
    /// Create a new empty loader
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            registry: GritQLRegistry::new(),
        }
    }

    /// Load all .grit files from specified directories
    pub fn load_from_directories(dirs: &[&Path]) -> Result<Self> {
        let mut loader = Self::new();

        for dir in dirs {
            debug!("Scanning directory for .grit files: {}", dir.display());
            let pattern_path = format!("{}/**/*.grit", dir.display());

            match glob(&pattern_path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        if let Err(e) = loader.load_rule(&entry) {
                            warn!("Failed to load GritQL rule from {}: {}", entry.display(), e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to glob pattern {}: {}", pattern_path, e);
                }
            }
        }

        info!("Loaded {} GritQL rules", loader.rules.len());
        Ok(loader)
    }

    /// Load a single .grit file
    fn load_rule(&mut self, path: &Path) -> Result<()> {
        debug!("Loading GritQL rule from: {}", path.display());

        let source = fs::read_to_string(path).map_err(|e| {
            FshLintError::rule_error(
                "gritql-loader",
                format!("Failed to read file {}: {}", path.display(), e),
            )
        })?;

        // Generate rule ID from filename
        let rule_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| format!("gritql/{s}"))
            .ok_or_else(|| {
                FshLintError::rule_error("gritql-loader", "Invalid filename for .grit file")
            })?;

        // Compile pattern
        let compiler = GritQLCompiler::new()?;
        let pattern = compiler.compile_pattern(&source, &rule_id)?;

        let loaded_rule = LoadedRule {
            id: rule_id.clone(),
            pattern: pattern.clone(),
            source_path: path.to_path_buf(),
        };

        self.registry.register(rule_id.clone(), pattern);

        debug!("Successfully loaded rule: {}", loaded_rule.id);
        self.rules.push(loaded_rule);
        Ok(())
    }

    /// Get all loaded rules
    pub fn all_rules(&self) -> &[LoadedRule] {
        &self.rules
    }

    /// Get the rule registry
    pub fn registry(&self) -> &GritQLRegistry {
        &self.registry
    }

    /// Get the number of loaded rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if loader has no rules
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

impl Default for GritQLRuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_creation() {
        let loader = GritQLRuleLoader::new();
        assert_eq!(loader.all_rules().len(), 0);
    }
}
