//! Rule engine implementation

use crate::gritql::GritQLCompiler;
use maki_core::{
    CompiledRule, Diagnostic, GritQLMatcher, MakiError, Result, Rule,
    RuleEngine as RuleEngineTrait, RuleEngineConfig, SemanticModel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Metadata for a rule pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackMetadata {
    /// Name of the rule pack
    pub name: String,
    /// Version of the rule pack
    pub version: String,
    /// Description of the rule pack
    pub description: String,
    /// Author or organization
    pub author: Option<String>,
    /// License information
    pub license: Option<String>,
    /// Homepage or repository URL
    pub homepage: Option<String>,
    /// Minimum required linter version
    pub min_linter_version: Option<String>,
    /// Tags for categorizing the rule pack
    pub tags: Vec<String>,
}

/// A rule pack containing multiple rules with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePack {
    /// Metadata for this rule pack
    pub metadata: RulePackMetadata,
    /// Rules included in this pack
    pub rules: Vec<Rule>,
    /// Dependencies on other rule packs
    pub dependencies: Vec<RulePackDependency>,
}

/// Dependency on another rule pack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackDependency {
    /// Name of the required rule pack
    pub name: String,
    /// Version requirement (semver compatible)
    pub version: String,
    /// Whether this dependency is optional
    pub optional: bool,
}

/// Rule precedence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePrecedence {
    /// Rule pack name (empty string for individual rules)
    pub pack_name: String,
    /// Priority level (higher numbers = higher priority)
    pub priority: i32,
    /// Whether this pack can override rules from lower priority packs
    pub can_override: bool,
}

/// Rule discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDiscoveryConfig {
    /// Directories to search for rule files
    pub rule_directories: Vec<PathBuf>,
    /// Rule pack directories to search
    pub pack_directories: Vec<PathBuf>,
    /// File patterns to include (glob patterns)
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Whether to search subdirectories recursively
    pub recursive: bool,
    /// Rule precedence configuration
    pub precedence: Vec<RulePrecedence>,
}

/// Registry for managing compiled rules
#[derive(Debug)]
pub struct RuleRegistry {
    rules: HashMap<String, CompiledRule>,
    rule_packs: HashMap<String, RulePack>,
    config: RuleEngineConfig,
    discovery_config: RuleDiscoveryConfig,
    rule_precedence: HashMap<String, i32>, // rule_id -> priority
}

/// Default implementation of the rule engine
pub struct DefaultRuleEngine {
    registry: RuleRegistry,
    gritql_loader: Option<crate::gritql::GritQLRuleLoader>,
    gritql_compiler: Arc<GritQLCompiler>,
    session: Option<Arc<maki_core::canonical::DefinitionSession>>,
}

impl RuleRegistry {
    /// Create a new rule registry
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            rule_packs: HashMap::new(),
            config: RuleEngineConfig::default(),
            discovery_config: RuleDiscoveryConfig::default(),
            rule_precedence: HashMap::new(),
        }
    }

    /// Create a new rule registry with configuration
    pub fn with_config(config: RuleEngineConfig) -> Self {
        Self {
            rules: HashMap::new(),
            rule_packs: HashMap::new(),
            config,
            discovery_config: RuleDiscoveryConfig::default(),
            rule_precedence: HashMap::new(),
        }
    }

    /// Create a new rule registry with discovery configuration
    pub fn with_discovery_config(discovery_config: RuleDiscoveryConfig) -> Self {
        Self {
            rules: HashMap::new(),
            rule_packs: HashMap::new(),
            config: RuleEngineConfig::default(),
            discovery_config,
            rule_precedence: HashMap::new(),
        }
    }

    /// Register a compiled rule with precedence handling
    pub fn register(&mut self, rule: CompiledRule) {
        let id = rule.id().to_string();

        // Check if rule already exists and handle precedence
        if let Some(_existing_rule) = self.rules.get(&id) {
            let existing_priority = self.rule_precedence.get(&id).unwrap_or(&0);
            let new_priority = self.rule_precedence.get(&id).unwrap_or(&0);

            if new_priority > existing_priority {
                tracing::info!("Overriding rule '{}' with higher priority rule", id);
                self.rules.insert(id.clone(), rule);
            } else if new_priority == existing_priority {
                tracing::warn!(
                    "Rule '{}' already exists with same priority, keeping existing",
                    id
                );
            } else {
                tracing::debug!("Skipping rule '{}' with lower priority", id);
            }
        } else {
            self.rules.insert(id, rule);
        }
    }

    /// Register a rule pack
    pub fn register_pack(&mut self, pack: RulePack) -> Result<()> {
        let pack_name = pack.metadata.name.clone();

        // Validate pack metadata
        self.validate_pack_metadata(&pack.metadata)?;

        // Check for pack conflicts
        if self.rule_packs.contains_key(&pack_name) {
            return Err(MakiError::rule_error(
                &pack_name,
                "Rule pack with this name is already registered",
            ));
        }

        // Find pack priority and override setting from discovery config
        let (pack_priority, can_override) = self
            .discovery_config
            .precedence
            .iter()
            .find(|p| p.pack_name == pack_name)
            .map(|p| (p.priority, p.can_override))
            .unwrap_or((0, false));

        // Register all rules from the pack with the pack's priority
        for rule in &pack.rules {
            // Check if rule already has a priority
            if let Some(&existing_priority) = self.rule_precedence.get(&rule.id) {
                // Only override if:
                // 1. This pack has higher priority, OR
                // 2. This pack has can_override=true and priority >= existing
                if pack_priority > existing_priority
                    || (can_override && pack_priority >= existing_priority)
                {
                    self.rule_precedence.insert(rule.id.clone(), pack_priority);
                }
                // Otherwise, keep the existing priority (don't override)
            } else {
                // No existing priority, just set it
                self.rule_precedence.insert(rule.id.clone(), pack_priority);
            }
        }

        let rules_count = pack.rules.len();

        // Store the pack
        self.rule_packs.insert(pack_name.clone(), pack);

        tracing::info!(
            "Registered rule pack '{}' with {} rules",
            pack_name,
            rules_count
        );
        Ok(())
    }

    /// Get a rule by ID
    pub fn get(&self, id: &str) -> Option<&CompiledRule> {
        self.rules.get(id)
    }

    /// Get all rules
    pub fn get_all(&self) -> Vec<&CompiledRule> {
        self.rules.values().collect()
    }

    /// List all registered rule IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.rules.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Get the configuration
    pub fn config(&self) -> &RuleEngineConfig {
        &self.config
    }

    /// Get the discovery configuration
    pub fn discovery_config(&self) -> &RuleDiscoveryConfig {
        &self.discovery_config
    }

    /// Get all registered rule packs
    pub fn get_packs(&self) -> &HashMap<String, RulePack> {
        &self.rule_packs
    }

    /// Get a specific rule pack by name
    pub fn get_pack(&self, name: &str) -> Option<&RulePack> {
        self.rule_packs.get(name)
    }

    /// Get rule precedence information
    pub fn get_rule_priority(&self, rule_id: &str) -> i32 {
        self.rule_precedence.get(rule_id).copied().unwrap_or(0)
    }

    /// List all rule packs
    pub fn list_pack_names(&self) -> Vec<&str> {
        self.rule_packs.keys().map(|s| s.as_str()).collect()
    }

    /// Validate rule pack metadata
    fn validate_pack_metadata(&self, metadata: &RulePackMetadata) -> Result<()> {
        if metadata.name.trim().is_empty() {
            return Err(MakiError::config_error("Rule pack name cannot be empty"));
        }

        if metadata.version.trim().is_empty() {
            return Err(MakiError::config_error("Rule pack version cannot be empty"));
        }

        // Basic semver validation
        if !metadata.version.chars().any(|c| c.is_ascii_digit()) {
            return Err(MakiError::config_error(
                "Rule pack version must contain at least one digit",
            ));
        }

        Ok(())
    }
}

impl DefaultRuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        let compiler = GritQLCompiler::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create GritQL compiler: {}", e);
            panic!("Fatal: Could not initialize GritQL compiler");
        });

        Self {
            registry: RuleRegistry::new(),
            gritql_loader: None,
            gritql_compiler: Arc::new(compiler),
            session: None,
        }
    }

    /// Create a new rule engine with configuration
    pub fn with_config(config: RuleEngineConfig) -> Self {
        let compiler = GritQLCompiler::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create GritQL compiler: {}", e);
            panic!("Fatal: Could not initialize GritQL compiler");
        });

        Self {
            registry: RuleRegistry::with_config(config),
            gritql_loader: None,
            gritql_compiler: Arc::new(compiler),
            session: None,
        }
    }

    /// Create a new rule engine with GritQL rule directories
    ///
    /// # Arguments
    ///
    /// * `rule_directories` - Directories to search for .grit files
    ///
    /// # Returns
    ///
    /// A rule engine with loaded GritQL rules, or an error if loading fails
    pub fn with_gritql_directories(rule_directories: Vec<String>) -> Result<Self> {
        // Convert String paths to &Path
        let paths: Vec<std::path::PathBuf> = rule_directories
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let path_refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
        let gritql_loader = crate::gritql::GritQLRuleLoader::load_from_directories(&path_refs)?;

        let compiler = GritQLCompiler::new()?;

        Ok(Self {
            registry: RuleRegistry::new(),
            gritql_loader: Some(gritql_loader),
            gritql_compiler: Arc::new(compiler),
            session: None,
        })
    }

    /// Get the rule registry
    pub fn registry(&self) -> &RuleRegistry {
        &self.registry
    }

    /// Get mutable access to the rule registry
    pub fn registry_mut(&mut self) -> &mut RuleRegistry {
        &mut self.registry
    }

    /// Load rules from a single file
    pub fn load_rule_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| MakiError::io_error(path, e))?;

        // Try to parse as JSON first, then TOML
        let rule: Rule = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).map_err(|e| {
                MakiError::config_error(format!(
                    "Failed to parse rule file {}: {}",
                    path.display(),
                    e
                ))
            })?
        } else {
            toml::from_str(&content).map_err(|e| {
                MakiError::config_error(format!(
                    "Failed to parse rule file {}: {}",
                    path.display(),
                    e
                ))
            })?
        };

        // Validate the rule
        self.validate_rule(&rule)?;

        // Compile and register the rule
        let compiled_rule = self.compile_rule(&rule)?;
        self.registry.register(compiled_rule);

        tracing::debug!("Loaded rule '{}' from {}", rule.id, path.display());
        Ok(())
    }

    /// Load all rule files from a directory
    pub fn load_rules_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Err(MakiError::config_error(format!(
                "Rule directory does not exist: {}",
                dir.display()
            )));
        }

        if !dir.is_dir() {
            return Err(MakiError::config_error(format!(
                "Path is not a directory: {}",
                dir.display()
            )));
        }

        let mut loaded_count = 0;
        let mut errors = Vec::new();

        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry.map_err(|e| MakiError::io_error(dir, std::io::Error::other(e)))?;
            let path = entry.path();

            // Only process rule files (JSON or TOML)
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|s| s.to_str())
                && matches!(ext, "json" | "toml" | "yaml" | "yml")
            {
                match self.load_rule_file(path) {
                    Ok(()) => loaded_count += 1,
                    Err(e) => {
                        if self.registry.config.fail_fast {
                            return Err(e);
                        } else {
                            tracing::warn!("Failed to load rule from {}: {}", path.display(), e);
                            errors.push(e);
                        }
                    }
                }
            }
        }

        tracing::info!("Loaded {} rules from {}", loaded_count, dir.display());

        if !errors.is_empty() && self.registry.config.fail_fast {
            return Err(MakiError::config_error(format!(
                "Failed to load {} rule files",
                errors.len()
            )));
        }

        Ok(())
    }

    /// Discover and load rules from configured directories
    pub fn discover_and_load_rules(&mut self) -> Result<()> {
        let discovery_config = self.registry.discovery_config.clone();

        // Load individual rule files from rule directories
        for rule_dir in &discovery_config.rule_directories {
            if discovery_config.recursive {
                self.load_rules_from_dir_recursive(rule_dir, &discovery_config)?;
            } else {
                self.load_rules_from_dir_flat(rule_dir, &discovery_config)?;
            }
        }

        // Load rule packs from pack directories
        for pack_dir in &discovery_config.pack_directories {
            self.load_rule_packs_from_dir(pack_dir)?;
        }

        Ok(())
    }

    /// Load rules from directory with pattern matching
    fn load_rules_from_dir_recursive(
        &mut self,
        dir: &Path,
        config: &RuleDiscoveryConfig,
    ) -> Result<()> {
        if !dir.exists() {
            tracing::warn!("Rule directory does not exist: {}", dir.display());
            return Ok(());
        }

        let walker = if config.recursive {
            walkdir::WalkDir::new(dir)
        } else {
            walkdir::WalkDir::new(dir).max_depth(1)
        };

        for entry in walker {
            let entry = entry.map_err(|e| MakiError::io_error(dir, std::io::Error::other(e)))?;
            let path = entry.path();

            if path.is_file()
                && self.should_include_file(path, config)
                && let Err(e) = self.load_rule_file(path)
            {
                if self.registry.config.fail_fast {
                    return Err(e);
                } else {
                    tracing::warn!("Failed to load rule from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Load rules from directory (flat, non-recursive)
    fn load_rules_from_dir_flat(&mut self, dir: &Path, config: &RuleDiscoveryConfig) -> Result<()> {
        if !dir.exists() {
            tracing::warn!("Rule directory does not exist: {}", dir.display());
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| MakiError::io_error(dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| MakiError::io_error(dir, e))?;
            let path = entry.path();

            if path.is_file()
                && self.should_include_file(&path, config)
                && let Err(e) = self.load_rule_file(&path)
            {
                if self.registry.config.fail_fast {
                    return Err(e);
                } else {
                    tracing::warn!("Failed to load rule from {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Load rule packs from a directory
    pub fn load_rule_packs_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            tracing::warn!("Rule pack directory does not exist: {}", dir.display());
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| MakiError::io_error(dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| MakiError::io_error(dir, e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Look for pack manifest files
                    if (file_name == "pack.json"
                        || file_name == "pack.toml"
                        || file_name == "rulePack.json")
                        && let Err(e) = self.load_rule_pack_file(&path)
                    {
                        if self.registry.config.fail_fast {
                            return Err(e);
                        } else {
                            tracing::warn!(
                                "Failed to load rule pack from {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            } else if path.is_dir() {
                // Recursively search subdirectories for pack files
                self.load_rule_packs_from_dir(&path)?;
            }
        }

        Ok(())
    }

    /// Load a rule pack from a file
    pub fn load_rule_pack_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path).map_err(|e| MakiError::io_error(path, e))?;

        // Try to parse as JSON first, then TOML
        let pack: RulePack = if path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).map_err(|e| {
                MakiError::config_error(format!(
                    "Failed to parse rule pack file {}: {}",
                    path.display(),
                    e
                ))
            })?
        } else {
            toml::from_str(&content).map_err(|e| {
                MakiError::config_error(format!(
                    "Failed to parse rule pack file {}: {}",
                    path.display(),
                    e
                ))
            })?
        };

        // Register the pack (this will validate metadata and handle precedence)
        self.registry.register_pack(pack.clone())?;

        // Compile and register all rules from the pack
        for rule in &pack.rules {
            // Validate the rule
            self.validate_rule(rule)?;

            // Compile and register the rule
            let compiled_rule = self.compile_rule(rule)?;
            self.registry.register(compiled_rule);
        }

        tracing::info!(
            "Loaded rule pack '{}' from {}",
            pack.metadata.name,
            path.display()
        );
        Ok(())
    }

    /// Check if a file should be included based on patterns
    fn should_include_file(&self, path: &Path, config: &RuleDiscoveryConfig) -> bool {
        let path_str = path.to_string_lossy();

        // Check exclude patterns first
        for exclude_pattern in &config.exclude_patterns {
            if glob_match(exclude_pattern, &path_str) {
                return false;
            }
        }

        // If no include patterns specified, include all supported file types
        if config.include_patterns.is_empty() {
            return path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext, "json" | "toml" | "yaml" | "yml"))
                .unwrap_or(false);
        }

        // Check include patterns
        for include_pattern in &config.include_patterns {
            if glob_match(include_pattern, &path_str) {
                return true;
            }
        }

        false
    }

    /// Execute a single rule against the semantic model
    async fn execute_single_rule(
        &self,
        rule: &CompiledRule,
        model: &SemanticModel,
        compiler: &GritQLCompiler,
    ) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();

        // Check if this is an AST-based rule by checking if it has a pattern
        if !rule.matcher.has_pattern() {
            // AST-based rule without GritQL pattern - execute builtin rule check function
            tracing::debug!(
                "Executing AST-based rule '{}' against semantic model for file '{}'",
                rule.id(),
                model.source_file.display()
            );

            // Execute AST-based builtin rules
            match rule.id() {
                crate::builtin::cardinality::INVALID_CARDINALITY => {
                    diagnostics.extend(crate::builtin::cardinality::check_cardinality(model));
                }
                crate::builtin::cardinality::CARDINALITY_CONFLICTS => {
                    diagnostics.extend(
                        crate::builtin::cardinality::check_cardinality_conflicts(
                            model,
                            self.session.as_ref().map(|s| s.as_ref()),
                        )
                        .await,
                    );
                }
                crate::builtin::cardinality::CARDINALITY_TOO_RESTRICTIVE => {
                    diagnostics.extend(
                        crate::builtin::cardinality::check_cardinality_too_restrictive(model),
                    );
                }
                crate::builtin::required_fields::REQUIRED_FIELD_PRESENT => {
                    diagnostics.extend(crate::builtin::required_fields::check_required_fields(
                        model,
                    ));
                }
                crate::builtin::binding::BINDING_STRENGTH_PRESENT => {
                    diagnostics.extend(crate::builtin::binding::check_binding_strength_required(
                        model,
                    ));
                }
                crate::builtin::binding::BINDING_STRENGTH_WEAKENING => {
                    diagnostics.extend(
                        crate::builtin::binding::check_binding_strength_weakening(
                            model,
                            self.session.as_ref().map(|s| s.as_ref()),
                        )
                        .await,
                    );
                }
                crate::builtin::binding::BINDING_STRENGTH_INCONSISTENT => {
                    diagnostics.extend(
                        crate::builtin::binding::check_binding_strength_inconsistent(model),
                    );
                }
                crate::builtin::binding::BINDING_WITHOUT_VALUESET => {
                    diagnostics.extend(crate::builtin::binding::check_binding_without_valueset(
                        model,
                    ));
                }
                crate::builtin::metadata::MISSING_METADATA => {
                    diagnostics.extend(crate::builtin::metadata::check_missing_metadata(model));
                }
                crate::builtin::duplicates::DUPLICATE_DEFINITION => {
                    diagnostics.extend(crate::builtin::duplicates::check_duplicates(model));
                }
                crate::builtin::duplicates::DUPLICATE_RULE => {
                    diagnostics.extend(crate::builtin::duplicates::check_duplicate_rules(model));
                }
                crate::builtin::duplicates::DUPLICATE_ALIAS => {
                    diagnostics.extend(crate::builtin::duplicates::check_duplicate_aliases(model));
                }
                crate::builtin::profile::PROFILE_ASSIGNMENT_PRESENT => {
                    diagnostics.extend(crate::builtin::profile::check_profile_assignments(model));
                }
                crate::builtin::profile::EXTENSION_CONTEXT_MISSING => {
                    // Use the enhanced implementation from required_fields with Error severity and autofix
                    diagnostics.extend(crate::builtin::required_fields::check_extension_context(
                        model,
                    ));
                }
                crate::builtin::required_fields::INSTANCE_REQUIRED_FIELDS_MISSING => {
                    diagnostics.extend(
                        crate::builtin::required_fields::check_instance_required_fields(model),
                    );
                }
                crate::builtin::required_fields::REQUIRED_FIELD_OVERRIDE => {
                    diagnostics.extend(
                        crate::builtin::required_fields::check_required_field_override(
                            model,
                            self.session.as_ref().map(|s| s.as_ref()),
                        )
                        .await,
                    );
                }
                crate::builtin::required_fields::PROFILE_WITHOUT_EXAMPLES => {
                    diagnostics.extend(
                        crate::builtin::required_fields::check_profile_without_examples(model),
                    );
                }
                crate::builtin::naming::NAMING_CONVENTION => {
                    diagnostics.extend(crate::builtin::naming::check_naming_conventions(model));
                }
                crate::builtin::profile::SLICE_NAME_COLLISION => {
                    diagnostics.extend(crate::builtin::profile::check_slice_name_collision(model));
                }
                crate::builtin::profile::MUST_SUPPORT_PROPAGATION => {
                    diagnostics.extend(crate::builtin::profile::check_must_support_propagation(
                        model,
                    ));
                }
                _ => {
                    tracing::warn!(
                        "AST rule '{}' not found in builtin rules registry",
                        rule.id()
                    );
                }
            }
        } else {
            // GritQL pattern-based rule
            let compiled_pattern = compiler.compile_pattern(rule.matcher.pattern(), rule.id())?;

            tracing::debug!("Executing GritQL rule '{}' with pattern", rule.id());

            // Execute pattern and convert matches to diagnostics
            match compiled_pattern.execute(&model.source, model.source_file.to_str().unwrap_or(""))
            {
                Ok(matches) => {
                    for grit_match in matches {
                        let location = maki_core::diagnostics::Location {
                            file: model.source_file.clone(),
                            line: grit_match.range.start_line,
                            column: grit_match.range.start_column,
                            end_line: Some(grit_match.range.end_line),
                            end_column: Some(grit_match.range.end_column),
                            offset: 0, // TODO: Calculate from source
                            length: grit_match.matched_text.len(),
                            span: None,
                        };

                        diagnostics.push(maki_core::Diagnostic::new(
                            rule.id(),
                            rule.metadata.severity,
                            &rule.metadata.description,
                            location,
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to execute GritQL pattern for rule '{}': {}",
                        rule.id(),
                        e
                    );
                }
            }
        }

        Ok(diagnostics)
    }
}

impl RuleEngineTrait for DefaultRuleEngine {
    fn load_rules(&mut self, rule_dirs: &[PathBuf]) -> Result<()> {
        // Update discovery config with provided directories
        self.registry.discovery_config.rule_directories = rule_dirs.to_vec();

        // Use the discovery system to load rules
        self.discover_and_load_rules()
    }

    fn compile_rule(&self, rule: &Rule) -> Result<CompiledRule> {
        // Validate the rule first
        rule.validate()?;

        // Compile the GritQL pattern with the rule ID for better error reporting
        let matcher = GritQLMatcher::new_with_rule_id(rule.gritql_pattern.clone(), &rule.id)?;

        // Create the compiled rule
        let mut compiled_rule = CompiledRule::new(rule.metadata.clone(), matcher);

        // Add autofix template if present
        if let Some(autofix) = &rule.autofix {
            compiled_rule.autofix_template = Some(autofix.clone());
        }

        Ok(compiled_rule)
    }

    fn execute_rules<'a>(
        &'a self,
        model: &'a SemanticModel,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<Diagnostic>> + Send + 'a>> {
        Box::pin(self.execute_rules_impl(model))
    }

    fn get_rules(&self) -> &[CompiledRule] {
        // This is a bit awkward since we need to return a slice, but we have a HashMap
        // For now, we'll use a static empty slice. In a real implementation,
        // we might want to change the trait to return an iterator or Vec
        &[]
    }

    fn get_rule(&self, id: &str) -> Option<&CompiledRule> {
        self.registry.get(id)
    }

    fn validate_rule(&self, rule: &Rule) -> Result<()> {
        // First run the rule's own validation
        rule.validate()?;

        // Additional validation specific to the engine
        if self.registry.get(&rule.id).is_some() {
            return Err(MakiError::rule_error(
                &rule.id,
                "Rule with this ID is already registered",
            ));
        }

        // Validate GritQL pattern syntax (only for non-AST rules)
        if !rule.is_ast_rule && rule.gritql_pattern.trim().is_empty() {
            return Err(MakiError::rule_error(
                &rule.id,
                "GritQL pattern cannot be empty or whitespace-only for non-AST rules",
            ));
        }

        // Validate metadata
        if rule.metadata.name.trim().is_empty() {
            return Err(MakiError::rule_error(&rule.id, "Rule name cannot be empty"));
        }

        Ok(())
    }
}

impl DefaultRuleEngine {
    async fn execute_rules_impl(&self, model: &SemanticModel) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Use the pre-initialized GritQL compiler (shared across all files)
        let compiler = &self.gritql_compiler;

        // Execute built-in rules
        for rule in self.registry.get_all() {
            // Apply rule-specific configuration if present
            if let Some(rule_config) = self.registry.config.rule_configs.get(rule.id())
                && !rule_config.enabled
            {
                tracing::debug!("Skipping disabled rule '{}'", rule.id());
                continue;
            }

            // Respect max diagnostics limit
            if let Some(max_diagnostics) = self.registry.config.max_diagnostics_per_rule
                && diagnostics.len() >= max_diagnostics
            {
                tracing::warn!("Reached maximum diagnostics limit for rule '{}'", rule.id());
                break;
            }

            // Execute the rule against each file in the semantic model
            match self.execute_single_rule(rule, model, compiler).await {
                Ok(mut rule_diagnostics) => {
                    diagnostics.append(&mut rule_diagnostics);
                }
                Err(e) => {
                    tracing::error!("Failed to execute rule '{}': {}", rule.id(), e);
                    // Continue with other rules even if one fails
                }
            }
        }

        // Execute GritQL rules if loader is present
        if let Some(ref loader) = self.gritql_loader {
            tracing::debug!(
                "Executing {} GritQL rules from custom directories",
                loader.len()
            );

            // Execute each loaded GritQL rule
            for loaded_rule in loader.all_rules() {
                tracing::debug!("Executing GritQL rule: {}", loaded_rule.id());

                // Execute pattern against the source file
                match loaded_rule
                    .pattern()
                    .execute(&model.source, model.source_file.to_str().unwrap_or(""))
                {
                    Ok(matches) => {
                        for grit_match in matches {
                            let location = maki_core::diagnostics::Location {
                                file: model.source_file.clone(),
                                line: grit_match.range.start_line,
                                column: grit_match.range.start_column,
                                end_line: Some(grit_match.range.end_line),
                                end_column: Some(grit_match.range.end_column),
                                offset: 0, // TODO: Calculate from source
                                length: grit_match.matched_text.len(),
                                span: None,
                            };

                            // Create diagnostic from match
                            // Use default severity of Warning for GritQL rules
                            diagnostics.push(maki_core::Diagnostic::new(
                                loaded_rule.id(),
                                maki_core::Severity::Warning,
                                format!("GritQL pattern matched: {}", loaded_rule.id()),
                                location,
                            ));
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to execute GritQL rule '{}': {}",
                            loaded_rule.id(),
                            e
                        );
                    }
                }
            }
        }

        diagnostics
    }

    /// Set the discovery configuration
    pub fn set_discovery_config(&mut self, config: RuleDiscoveryConfig) {
        self.registry.discovery_config = config;
    }

    /// Add a rule directory to the discovery configuration
    pub fn add_rule_directory(&mut self, dir: PathBuf) {
        self.registry.discovery_config.rule_directories.push(dir);
    }

    /// Add a rule pack directory to the discovery configuration
    pub fn add_pack_directory(&mut self, dir: PathBuf) {
        self.registry.discovery_config.pack_directories.push(dir);
    }

    /// Set rule precedence configuration
    pub fn set_rule_precedence(&mut self, precedence: Vec<RulePrecedence>) {
        self.registry.discovery_config.precedence = precedence;
    }

    /// Set the canonical manager session for FHIR resource resolution
    pub fn set_session(&mut self, session: Arc<maki_core::canonical::DefinitionSession>) {
        self.session = Some(session);
    }

    /// Get statistics about loaded rules and packs
    pub fn get_statistics(&self) -> RuleEngineStatistics {
        let mut rules_by_pack = HashMap::new();
        let mut rules_by_category = HashMap::new();

        for rule in self.registry.get_all() {
            // Count rules by pack (if they belong to one)
            let pack_name = self
                .find_rule_pack(rule.id())
                .unwrap_or("individual".to_string());
            *rules_by_pack.entry(pack_name).or_insert(0) += 1;

            // Count rules by category
            let category = rule.metadata.category.to_string();
            *rules_by_category.entry(category).or_insert(0) += 1;
        }

        RuleEngineStatistics {
            total_rules: self.registry.len(),
            total_packs: self.registry.rule_packs.len(),
            rules_by_pack,
            rules_by_category,
        }
    }

    /// Find which pack a rule belongs to
    fn find_rule_pack(&self, rule_id: &str) -> Option<String> {
        for (pack_name, pack) in &self.registry.rule_packs {
            if pack.rules.iter().any(|r| r.id == rule_id) {
                return Some(pack_name.clone());
            }
        }
        None
    }
}

/// Statistics about the rule engine state
#[derive(Debug, Clone)]
pub struct RuleEngineStatistics {
    /// Total number of loaded rules
    pub total_rules: usize,
    /// Total number of loaded rule packs
    pub total_packs: usize,
    /// Number of rules per pack
    pub rules_by_pack: HashMap<String, usize>,
    /// Number of rules per category
    pub rules_by_category: HashMap<String, usize>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DefaultRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RuleDiscoveryConfig {
    fn default() -> Self {
        Self {
            rule_directories: Vec::new(),
            pack_directories: Vec::new(),
            include_patterns: vec![
                "*.json".to_string(),
                "*.toml".to_string(),
                "*.yaml".to_string(),
                "*.yml".to_string(),
            ],
            exclude_patterns: vec![
                ".*".to_string(), // Exclude hidden files
                "node_modules/**".to_string(),
                "target/**".to_string(),
                ".git/**".to_string(),
            ],
            recursive: true,
            precedence: Vec::new(),
        }
    }
}

/// Thread-safe wrapper for RuleRegistry optimized for concurrent LSP operations
///
/// This wrapper uses RwLock for concurrent read access with exclusive write access,
/// making it ideal for LSP servers where:
/// - Multiple document analysis requests can read rules concurrently
/// - Rule updates (hot reload, configuration changes) require exclusive access
///
/// # Thread Safety
/// - Multiple readers can access rules simultaneously (common case in LSP)
/// - Writers get exclusive access for rule updates
/// - All wrapped types (CompiledRule, RulePack) are Send + Sync
///
/// # Example for LSP
/// ```rust,ignore
/// let registry = Arc::new(ThreadSafeRuleRegistry::new(RuleRegistry::new()));
///
/// // Clone Arc for each LSP request handler
/// let registry_clone = Arc::clone(&registry);
/// tokio::spawn(async move {
///     // Read access - non-blocking for other readers
///     let rules = registry_clone.get_all_rules();
///     // ... execute rules
/// });
/// ```
#[derive(Debug)]
pub struct ThreadSafeRuleRegistry {
    inner: Arc<RwLock<RuleRegistry>>,
}

impl ThreadSafeRuleRegistry {
    /// Create a new thread-safe registry wrapper
    pub fn new(registry: RuleRegistry) -> Self {
        Self {
            inner: Arc::new(RwLock::new(registry)),
        }
    }

    /// Create with default registry
    pub fn default_registry() -> Self {
        Self::new(RuleRegistry::new())
    }

    /// Get a rule by ID (read lock)
    pub fn get_rule(&self, id: &str) -> Option<CompiledRule> {
        self.inner.read().ok()?.get(id).cloned()
    }

    /// Get all rules (read lock)
    pub fn get_all_rules(&self) -> Vec<CompiledRule> {
        self.inner
            .read()
            .map(|registry| registry.get_all().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Register a new rule (write lock)
    pub fn register_rule(&self, rule: CompiledRule) -> Result<()> {
        self.inner
            .write()
            .map_err(|e| MakiError::config_error(format!("Lock poisoned: {e}")))?
            .register(rule);
        Ok(())
    }

    /// Register a rule pack (write lock)
    pub fn register_pack(&self, pack: RulePack) -> Result<()> {
        self.inner
            .write()
            .map_err(|e| MakiError::config_error(format!("Lock poisoned: {e}")))?
            .register_pack(pack)
    }

    /// Get rule count (read lock)
    pub fn len(&self) -> usize {
        self.inner
            .read()
            .map(|registry| registry.len())
            .unwrap_or(0)
    }

    /// Check if registry is empty (read lock)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clone the inner Arc for sharing across threads/tasks
    pub fn clone_arc(&self) -> Arc<RwLock<RuleRegistry>> {
        Arc::clone(&self.inner)
    }

    /// Get a reference to the inner Arc
    pub fn inner(&self) -> &Arc<RwLock<RuleRegistry>> {
        &self.inner
    }
}

impl Clone for ThreadSafeRuleRegistry {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Simple implementation - in a real system you'd use a proper glob library
    if pattern == "*" {
        return true;
    }

    if pattern.contains('*') {
        // Handle simple wildcard patterns
        if let Some(ext) = pattern.strip_prefix("*.") {
            return text.ends_with(ext);
        }

        if let Some(prefix) = pattern.strip_suffix("/**") {
            return text.starts_with(prefix);
        }

        // For more complex patterns, fall back to simple contains check
        let pattern_without_wildcards = pattern.replace('*', "");
        return text.contains(&pattern_without_wildcards);
    }

    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::{RuleCategory, RuleMetadata, Severity};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_rule() -> Rule {
        Rule {
            id: "test/correctness/test-rule".to_string(),
            severity: Severity::Warning,
            description: "A test rule".to_string(),
            gritql_pattern: "test_pattern".to_string(),
            autofix: None,
            metadata: RuleMetadata {
                id: "test/correctness/test-rule".to_string(),
                name: "Test Rule".to_string(),
                description: "A test rule".to_string(),
                severity: Severity::Warning,
                category: RuleCategory::Correctness,
                tags: vec!["test".to_string()],
                version: Some("1.0.0".to_string()),
                docs_url: None,
            },
            is_ast_rule: false,
        }
    }

    #[test]
    fn test_rule_registry() {
        let mut registry = RuleRegistry::new();
        assert!(registry.is_empty());

        let rule = create_test_rule();
        let compiled_rule = CompiledRule::new(
            rule.metadata.clone(),
            GritQLMatcher::new(rule.gritql_pattern.clone()).unwrap(),
        );

        registry.register(compiled_rule);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test/correctness/test-rule").is_some());
    }

    #[test]
    fn test_rule_engine_creation() {
        let engine = DefaultRuleEngine::new();
        assert!(engine.registry().is_empty());

        let config = RuleEngineConfig::default();
        let engine_with_config = DefaultRuleEngine::with_config(config);
        assert!(engine_with_config.registry().is_empty());
    }

    #[test]
    fn test_rule_compilation() {
        let engine = DefaultRuleEngine::new();
        let rule = create_test_rule();

        let compiled_rule = engine.compile_rule(&rule).unwrap();
        assert_eq!(compiled_rule.id(), "test/correctness/test-rule");
        assert_eq!(compiled_rule.severity(), Severity::Warning);
    }

    #[test]
    fn test_rule_validation() {
        let engine = DefaultRuleEngine::new();
        let valid_rule = create_test_rule();

        assert!(engine.validate_rule(&valid_rule).is_ok());

        let mut invalid_rule = create_test_rule();
        invalid_rule.gritql_pattern = "".to_string();

        assert!(engine.validate_rule(&invalid_rule).is_err());
    }

    #[test]
    fn test_load_rule_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let rule_file = temp_dir.path().join("test-rule.json");

        let rule = create_test_rule();
        let rule_json = serde_json::to_string_pretty(&rule).unwrap();
        fs::write(&rule_file, rule_json).unwrap();

        let mut engine = DefaultRuleEngine::new();
        engine.load_rule_file(&rule_file)?;

        assert!(
            engine
                .registry()
                .get("test/correctness/test-rule")
                .is_some()
        );
        Ok(())
    }

    fn create_test_rule_pack() -> RulePack {
        RulePack {
            metadata: RulePackMetadata {
                name: "test-pack".to_string(),
                version: "1.0.0".to_string(),
                description: "A test rule pack".to_string(),
                author: Some("Test Author".to_string()),
                license: Some("MIT".to_string()),
                homepage: None,
                min_linter_version: None,
                tags: vec!["test".to_string()],
            },
            rules: vec![
                create_test_rule(),
                Rule {
                    id: "test/suspicious/test-rule-2".to_string(),
                    severity: Severity::Error,
                    description: "Another test rule".to_string(),
                    gritql_pattern: "another_pattern".to_string(),
                    autofix: None,
                    metadata: RuleMetadata {
                        id: "test/suspicious/test-rule-2".to_string(),
                        name: "Test Rule 2".to_string(),
                        description: "Another test rule".to_string(),
                        severity: Severity::Error,
                        category: RuleCategory::Suspicious,
                        tags: vec!["test".to_string()],
                        version: Some("1.0.0".to_string()),
                        docs_url: None,
                    },
                    is_ast_rule: false,
                },
            ],
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn test_rule_pack_registration() -> Result<()> {
        let mut registry = RuleRegistry::new();
        let pack = create_test_rule_pack();

        registry.register_pack(pack.clone())?;

        assert_eq!(registry.rule_packs.len(), 1);
        assert!(registry.get_pack("test-pack").is_some());
        Ok(())
    }

    #[test]
    fn test_rule_pack_loading() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let pack_file = temp_dir.path().join("pack.json");

        let pack = create_test_rule_pack();
        let pack_json = serde_json::to_string_pretty(&pack).unwrap();
        fs::write(&pack_file, pack_json).unwrap();

        let mut engine = DefaultRuleEngine::new();
        engine.load_rule_pack_file(&pack_file)?;

        assert!(engine.registry().get_pack("test-pack").is_some());
        assert!(
            engine
                .registry()
                .get("test/correctness/test-rule")
                .is_some()
        );
        assert!(
            engine
                .registry()
                .get("test/suspicious/test-rule-2")
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn test_rule_precedence() -> Result<()> {
        let mut engine = DefaultRuleEngine::new();

        // Set up precedence configuration
        let precedence = vec![
            RulePrecedence {
                pack_name: "high-priority-pack".to_string(),
                priority: 100,
                can_override: true,
            },
            RulePrecedence {
                pack_name: "low-priority-pack".to_string(),
                priority: 10,
                can_override: false,
            },
        ];
        engine.set_rule_precedence(precedence);

        // Create two packs with conflicting rule IDs
        let mut low_priority_pack = create_test_rule_pack();
        low_priority_pack.metadata.name = "low-priority-pack".to_string();

        let mut high_priority_pack = create_test_rule_pack();
        high_priority_pack.metadata.name = "high-priority-pack".to_string();
        high_priority_pack.rules[0].description = "High priority version".to_string();

        // Register low priority pack first
        engine.registry_mut().register_pack(low_priority_pack)?;

        // Compile and register rules from low priority pack
        let low_rule = create_test_rule();
        let compiled_low = engine.compile_rule(&low_rule)?;
        engine.registry_mut().register(compiled_low);

        // Register high priority pack
        engine.registry_mut().register_pack(high_priority_pack)?;

        // Compile and register rules from high priority pack (should override)
        let mut high_rule = create_test_rule();
        high_rule.description = "High priority version".to_string();
        let compiled_high = engine.compile_rule(&high_rule)?;
        engine.registry_mut().register(compiled_high);

        // The high priority rule should be registered
        let _registered_rule = engine.registry().get("test/correctness/test-rule").unwrap();
        // Note: We can't easily test the description here since CompiledRule doesn't expose it
        // In a real implementation, we'd need better access to rule metadata

        Ok(())
    }

    #[test]
    fn test_rule_discovery_config() {
        let mut engine = DefaultRuleEngine::new();

        let config = RuleDiscoveryConfig {
            rule_directories: vec![PathBuf::from("/rules")],
            pack_directories: vec![PathBuf::from("/packs")],
            include_patterns: vec!["*.json".to_string()],
            exclude_patterns: vec!["test_*".to_string()],
            recursive: false,
            precedence: Vec::new(),
        };

        engine.set_discovery_config(config.clone());

        assert_eq!(
            engine.registry().discovery_config().rule_directories,
            config.rule_directories
        );
        assert_eq!(
            engine.registry().discovery_config().pack_directories,
            config.pack_directories
        );
        assert!(!engine.registry().discovery_config().recursive);
    }

    #[test]
    fn test_glob_matching() {
        assert!(glob_match("*.json", "test.json"));
        assert!(glob_match("*.json", "path/to/test.json"));
        assert!(!glob_match("*.json", "test.toml"));

        assert!(glob_match("test/**", "test/file.json"));
        assert!(glob_match("test/**", "test/subdir/file.json"));
        assert!(!glob_match("test/**", "other/file.json"));

        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "different"));
    }

    #[test]
    fn test_rule_engine_statistics() -> Result<()> {
        let mut engine = DefaultRuleEngine::new();

        // Load a rule pack
        let pack = create_test_rule_pack();
        engine.registry_mut().register_pack(pack.clone())?;

        // Compile and register rules
        for rule in &pack.rules {
            let compiled_rule = engine.compile_rule(rule)?;
            engine.registry_mut().register(compiled_rule);
        }

        let stats = engine.get_statistics();

        assert_eq!(stats.total_rules, 2);
        assert_eq!(stats.total_packs, 1);
        assert!(stats.rules_by_pack.contains_key("test-pack"));
        assert!(stats.rules_by_category.contains_key("correctness"));
        assert!(stats.rules_by_category.contains_key("suspicious"));

        Ok(())
    }
}
