//! ValueSet Exporter
//!
//! Exports FSH ValueSet definitions to FHIR ValueSet resources (JSON).
//!
//! # Overview
//!
//! ValueSets define allowed sets of codes from code systems. This module handles:
//! - Converting FSH ValueSet metadata to FHIR ValueSet resources
//! - Processing include/exclude rules
//! - Handling filters (is-a, regex, exists, etc.)
//! - Building compose.include and compose.exclude structures
//!
//! # Status
//!
//! **Phase 1 (Current)**: Basic metadata export with FixedValueRule support
//! - Exports id, url, title, description, status
//! - Creates skeleton ValueSet resources
//! - Foundation for component rules
//!
//! **Phase 2 (Future)**: Full component rule support
//! - Parse and export include/exclude components
//! - Handle filters (is-a, descendent-of, regex, exists)
//! - Support valueset references
//! - Note: Requires parser enhancements for ValueSet component syntax
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::ValueSetExporter;
//! use maki_core::cst::ast::ValueSet;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse FSH valueset
//! let valueset: ValueSet = todo!();
//!
//! // Create exporter
//! let exporter = ValueSetExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Export to FHIR JSON
//! let resource = exporter.export(&valueset).await?;
//!
//! // Serialize
//! let json = serde_json::to_string_pretty(&resource)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

use super::{
    ExportError, ValueSetCompose, ValueSetConcept, ValueSetConceptDesignation,
    ValueSetConceptProperty, ValueSetFilter, ValueSetInclude, ValueSetResource,
};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{AstNode, Document, FixedValueRule, PathRule, Rule, ValueSet};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

// ============================================================================
// Component Rule Types
// ============================================================================

/// Represents different types of ValueSet component rules
#[derive(Debug, Clone)]
enum ComponentRule {
    /// Include a specific concept: * SYSTEM#CODE "Display"
    IncludeConcept {
        system: String,
        concept: ValueSetConcept,
        version: Option<String>,
    },
    /// Exclude a specific concept: * exclude SYSTEM#CODE
    ExcludeConcept {
        system: String,
        concept: ValueSetConcept,
        version: Option<String>,
    },
    /// Include with filter: * SYSTEM where property op value
    IncludeFilter {
        system: String,
        filter: ValueSetFilter,
        version: Option<String>,
    },
    /// Exclude with filter: * exclude SYSTEM where property op value
    ExcludeFilter {
        system: String,
        filter: ValueSetFilter,
        version: Option<String>,
    },
    /// Include from another ValueSet: * include codes from valueset "http://..."
    IncludeValueSet { value_set_url: String },
    /// Exclude from another ValueSet: * exclude codes from valueset "http://..."
    ExcludeValueSet { value_set_url: String },
}

// ============================================================================
// ValueSet Exporter
// ============================================================================

/// Exports FSH ValueSet definitions to FHIR ValueSet resources
///
/// # Phase 1: Metadata Export
///
/// Currently exports:
/// - Basic metadata (id, url, name, title, description)
/// - Status and date
/// - Publisher and copyright
///
/// # Future Enhancements
///
/// Phase 2 will add:
/// - Include/exclude component parsing
/// - Filter handling (is-a, regex, exists)
/// - ValueSet reference support
///
/// # Example
///
/// ```rust,no_run
/// # use maki_core::export::ValueSetExporter;
/// # use maki_core::canonical::DefinitionSession;
/// # use std::sync::Arc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let exporter = ValueSetExporter::new(
///     session,
///     "http://example.org/fhir".to_string(),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct ValueSetExporter {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    session: Arc<DefinitionSession>,
    /// Base URL for valueset canonical URLs
    base_url: String,
    /// Version from config
    version: Option<String>,
}

impl ValueSetExporter {
    /// Create a new valueset exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving code systems and other valuesets
    /// * `base_url` - Base URL for generated valueset identifiers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::ValueSetExporter;
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = ValueSetExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
        version: Option<String>,
    ) -> Result<Self, ExportError> {
        Ok(Self {
            session,
            base_url,
            version,
        })
    }

    /// Export a ValueSet to a FHIR resource (JSON)
    ///
    /// # Arguments
    ///
    /// * `valueset` - FSH ValueSet AST node
    ///
    /// # Returns
    ///
    /// A FHIR ValueSet resource as JSON
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - ValueSet name is missing
    /// - Required metadata fields are invalid
    /// - Rule application fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::ValueSetExporter;
    /// # use maki_core::cst::ast::ValueSet;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let exporter: ValueSetExporter = todo!();
    /// # let valueset: ValueSet = todo!();
    /// let resource = exporter.export(&valueset).await?;
    /// println!("{}", serde_json::to_string_pretty(&resource)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&self, valueset: &ValueSet) -> Result<ValueSetResource, ExportError> {
        let name = valueset
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("ValueSet name".to_string()))?;

        debug!("Exporting ValueSet: {}", name);

        // Generate canonical URL
        let url = format!("{}/ValueSet/{}", self.base_url, name);

        // Create base resource with default status
        let mut resource = ValueSetResource::new(url, name.clone(), "draft");

        // Set id from Id clause if present
        if let Some(id_clause) = valueset.id()
            && let Some(id_value) = id_clause.value()
        {
            resource.id = Some(id_value);
        }

        // Set title from Title clause if present
        if let Some(title_clause) = valueset.title()
            && let Some(title_value) = title_clause.value()
        {
            resource.title = Some(title_value);
        }

        // Set description from Description clause if present
        if let Some(desc_clause) = valueset.description()
            && let Some(desc_value) = desc_clause.value()
        {
            resource.description = Some(desc_value);
        }

        // Set version from config if available (SUSHI parity)
        resource.version = self.version.clone();

        // Build alias map from parent document
        let alias_map = self.build_alias_map(&valueset);
        trace!("Built alias map with {} entries", alias_map.len());

        // Initialize compose for component rules
        let mut compose = ValueSetCompose::new();

        // Group includes and excludes by system
        let mut system_includes: HashMap<
            String,
            (Vec<ValueSetConcept>, Vec<ValueSetFilter>, Option<String>),
        > = HashMap::new();
        let mut system_excludes: HashMap<
            String,
            (Vec<ValueSetConcept>, Vec<ValueSetFilter>, Option<String>),
        > = HashMap::new();
        let mut valueset_includes: Vec<String> = Vec::new();
        let mut valueset_excludes: Vec<String> = Vec::new();

        // Process rules
        for rule in valueset.rules() {
            match rule {
                Rule::FixedValue(fixed_rule) => {
                    self.apply_metadata_rule(&mut resource, &fixed_rule)?;
                }
                Rule::Card(_) => {
                    // Card rules don't apply to valuesets
                    trace!("Skipping card rule in valueset");
                }
                Rule::Flag(_) => {
                    // Flag rules don't apply to valuesets
                    trace!("Skipping flag rule in valueset");
                }
                Rule::ValueSet(_) => {
                    // ValueSet binding rules don't apply to valueset definitions
                    trace!("Skipping valueset rule in valueset definition");
                }
                Rule::Path(path_rule) => {
                    // Path rules are for component includes/excludes
                    if let Some(component_rule) =
                        self.parse_component_rule(&path_rule, &alias_map, &name)?
                    {
                        match component_rule {
                            ComponentRule::IncludeConcept {
                                system,
                                concept,
                                version,
                            } => {
                                let entry = system_includes
                                    .entry(system)
                                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                                entry.0.push(concept);
                            }
                            ComponentRule::ExcludeConcept {
                                system,
                                concept,
                                version,
                            } => {
                                let entry = system_excludes
                                    .entry(system)
                                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                                entry.0.push(concept);
                            }
                            ComponentRule::IncludeFilter {
                                system,
                                filter,
                                version,
                            } => {
                                let entry = system_includes
                                    .entry(system)
                                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                                entry.1.push(filter);
                            }
                            ComponentRule::ExcludeFilter {
                                system,
                                filter,
                                version,
                            } => {
                                let entry = system_excludes
                                    .entry(system)
                                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                                entry.1.push(filter);
                            }
                            ComponentRule::IncludeValueSet { value_set_url } => {
                                valueset_includes.push(value_set_url);
                            }
                            ComponentRule::ExcludeValueSet { value_set_url } => {
                                valueset_excludes.push(value_set_url);
                            }
                        }
                    }
                }
                Rule::AddElement(_)
                | Rule::Contains(_)
                | Rule::Only(_)
                | Rule::Obeys(_)
                | Rule::Mapping(_)
                | Rule::CaretValue(_)
                | Rule::CodeCaretValue(_)
                | Rule::CodeInsert(_) => {
                    // These rules don't apply to valuesets
                    trace!("Skipping contains/only/obeys rule in valueset");
                }
            }
        }

        // Build compose.include from grouped rules
        let mut has_content = false;

        // Add system-based includes
        for (system, (concepts, filters, version)) in system_includes {
            let mut include = ValueSetInclude::from_system(system);
            if let Some(v) = version {
                include.version = Some(v);
            }
            for concept in concepts {
                include.add_concept(concept);
            }
            for filter in filters {
                include.add_filter(filter);
            }
            compose.add_include(include);
            has_content = true;
        }

        // Add ValueSet-based includes
        for value_set_url in valueset_includes {
            let include = ValueSetInclude::from_valueset(value_set_url);
            compose.add_include(include);
            has_content = true;
        }

        // Build compose.exclude from grouped rules
        for (system, (concepts, filters, version)) in system_excludes {
            let mut exclude = ValueSetInclude::from_system(system);
            if let Some(v) = version {
                exclude.version = Some(v);
            }
            for concept in concepts {
                exclude.add_concept(concept);
            }
            for filter in filters {
                exclude.add_filter(filter);
            }
            compose.add_exclude(exclude);
            has_content = true;
        }

        // Add ValueSet-based excludes
        for value_set_url in valueset_excludes {
            let exclude = ValueSetInclude::from_valueset(value_set_url);
            compose.add_exclude(exclude);
            has_content = true;
        }

        if has_content {
            // Validate exclude rules don't conflict with include rules
            self.validate_exclude_conflicts(&compose, &name)?;
            resource.compose = Some(compose);
        }

        debug!("Successfully exported ValueSet {}", name);
        Ok(resource)
    }

    /// Apply a metadata rule (^property = value)
    ///
    /// Handles common metadata properties:
    /// - ^url - Canonical URL
    /// - ^version - Business version
    /// - ^status - Publication status
    /// - ^date - Last changed date
    /// - ^publisher - Publisher name
    /// - ^description - Description
    /// - ^purpose - Purpose
    /// - ^copyright - Copyright notice
    fn apply_metadata_rule(
        &self,
        resource: &mut ValueSetResource,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        // Get the path (property name)
        let path = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::InvalidPath {
                path: "<unknown>".to_string(),
                resource: "ValueSet".to_string(),
            })?;

        // Get the value
        let value_str = rule.value().ok_or_else(|| {
            ExportError::InvalidValue("Missing value in metadata rule".to_string())
        })?;

        trace!("Applying metadata: {} = {}", path, value_str);

        // Strip ^ prefix if present
        let property = path.strip_prefix('^').unwrap_or(&path);

        // Apply based on property name
        match property {
            "url" => {
                // Remove quotes from string value
                resource.url = self.extract_string_value(&value_str);
            }
            "version" => {
                resource.version = Some(self.extract_string_value(&value_str));
            }
            "status" => {
                // Extract code value (remove # prefix)
                let status = if let Some(code) = value_str.strip_prefix('#') {
                    code.to_string()
                } else {
                    value_str.clone()
                };
                resource.status = status;
            }
            "date" => {
                resource.date = Some(self.extract_string_value(&value_str));
            }
            "publisher" => {
                resource.publisher = Some(self.extract_string_value(&value_str));
            }
            "description" => {
                resource.description = Some(self.extract_string_value(&value_str));
            }
            "purpose" => {
                resource.purpose = Some(self.extract_string_value(&value_str));
            }
            "copyright" => {
                resource.copyright = Some(self.extract_string_value(&value_str));
            }
            "immutable" => {
                // Parse boolean
                let immutable = value_str.trim() == "true";
                resource.immutable = Some(immutable);
            }
            _ => {
                // Unknown property - log warning but don't fail
                warn!("Unknown ValueSet metadata property: {}", property);
            }
        }

        Ok(())
    }

    /// Extract string value from FSH value (removes quotes)
    fn extract_string_value(&self, value_str: &str) -> String {
        let trimmed = value_str.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Build alias map from parent document
    ///
    /// Traverses up to the parent Document node and extracts all aliases
    /// into a HashMap for quick lookup during component rule parsing.
    fn build_alias_map(&self, valueset: &ValueSet) -> HashMap<String, String> {
        let mut alias_map = HashMap::new();

        // Get parent document by traversing up the tree
        if let Some(parent) = valueset.syntax().parent() {
            if let Some(document) = Document::cast(parent) {
                for alias in document.aliases() {
                    if let Some(name) = alias.name() {
                        if let Some(value) = alias.value() {
                            alias_map.insert(name, value);
                        }
                    }
                }
            }
        }

        trace!("Built alias map with {} entries", alias_map.len());
        alias_map
    }

    /// Parse a component rule (PathRule) into system includes/excludes
    ///
    /// Handles syntax like:
    /// - `* SPTY#AMN "Amniotic fluid"` - code with display
    /// - `* SPTY#BLD` - code without display
    /// - `* SNOMED_CT where concept is-a #123456` - filter rule
    /// - `* exclude LOINC#12345-6` - exclude rule
    /// - `* LOINC version "2.72"` - version-specific include
    ///
    /// Returns: ComponentRule enum indicating the type of rule parsed
    fn parse_component_rule(
        &self,
        path_rule: &PathRule,
        alias_map: &HashMap<String, String>,
        valueset_name: &str,
    ) -> Result<Option<ComponentRule>, ExportError> {
        // Get the full text of the path rule to parse complex syntax
        let rule_text = path_rule.syntax().text().to_string();
        let rule_text = rule_text.trim();

        trace!("Parsing component rule: '{}'", rule_text);

        // Check for exclude prefix
        let (is_exclude, remaining_text) = if rule_text.starts_with("exclude ") {
            (true, rule_text.strip_prefix("exclude ").unwrap().trim())
        } else {
            (false, rule_text)
        };

        // Check for "codes from valueset" syntax
        if remaining_text.contains("codes from valueset") {
            return self.parse_valueset_reference(remaining_text, is_exclude);
        }

        // Check for "where" clause (filter syntax)
        if remaining_text.contains(" where ") {
            return self.parse_filter_rule(remaining_text, alias_map, is_exclude, valueset_name);
        }

        // Check for version specification
        let (system_and_code, version) = if remaining_text.contains(" version ") {
            let parts: Vec<&str> = remaining_text.splitn(2, " version ").collect();
            if parts.len() == 2 {
                let version_part = parts[1].trim();
                let version = self.extract_string_value(version_part);
                (parts[0].trim(), Some(version))
            } else {
                (remaining_text, None)
            }
        } else {
            (remaining_text, None)
        };

        // Parse: CodeSystemPrefix#Code
        let parts: Vec<&str> = system_and_code.split('#').collect();
        if parts.len() != 2 {
            warn!(
                "Invalid component rule format '{}' in ValueSet {}. Expected format: SYSTEM#CODE",
                system_and_code, valueset_name
            );
            return Ok(None);
        }

        let system_prefix = parts[0].trim();
        let code = parts[1].trim();

        // Resolve alias to full URL
        let system_url = match alias_map.get(system_prefix) {
            Some(url) => url.clone(),
            None => {
                warn!(
                    "Unresolved alias '{}' in ValueSet {}. Using as-is.",
                    system_prefix, valueset_name
                );
                system_prefix.to_string()
            }
        };

        // Try to get display text from following String token
        let display = self.get_display_text(path_rule);

        // Create concept with enhanced parsing for properties and designations
        let mut concept = if let Some(display_text) = display {
            ValueSetConcept::with_display(code, display_text)
        } else {
            ValueSetConcept::new(code)
        };

        // Parse additional concept metadata from the rule text
        self.parse_concept_metadata(&mut concept, path_rule)?;

        trace!(
            "Parsed component rule: system={}, code={}, display={:?}, exclude={}, version={:?}",
            system_url, code, concept.display, is_exclude, version
        );

        let component_rule = if is_exclude {
            ComponentRule::ExcludeConcept {
                system: system_url,
                concept,
                version,
            }
        } else {
            ComponentRule::IncludeConcept {
                system: system_url,
                concept,
                version,
            }
        };

        Ok(Some(component_rule))
    }

    /// Parse a filter rule: SYSTEM where property op value
    ///
    /// Examples:
    /// - `SNOMED_CT where concept is-a #123456`
    /// - `LOINC where STATUS = ACTIVE`
    /// - `ICD10 where concept regex "^A.*"`
    fn parse_filter_rule(
        &self,
        rule_text: &str,
        alias_map: &HashMap<String, String>,
        is_exclude: bool,
        valueset_name: &str,
    ) -> Result<Option<ComponentRule>, ExportError> {
        let parts: Vec<&str> = rule_text.splitn(2, " where ").collect();
        if parts.len() != 2 {
            warn!(
                "Invalid filter rule format in ValueSet {}: {}",
                valueset_name, rule_text
            );
            return Ok(None);
        }

        let system_part = parts[0].trim();
        let filter_part = parts[1].trim();

        // Parse version if present
        let (system_prefix, version) = if system_part.contains(" version ") {
            let version_parts: Vec<&str> = system_part.splitn(2, " version ").collect();
            if version_parts.len() == 2 {
                let version = self.extract_string_value(version_parts[1].trim());
                (version_parts[0].trim(), Some(version))
            } else {
                (system_part, None)
            }
        } else {
            (system_part, None)
        };

        // Resolve system alias
        let system_url = match alias_map.get(system_prefix) {
            Some(url) => url.clone(),
            None => {
                warn!(
                    "Unresolved alias '{}' in ValueSet {}. Using as-is.",
                    system_prefix, valueset_name
                );
                system_prefix.to_string()
            }
        };

        // Parse filter expression: property op value
        let filter = self.parse_filter_expression(filter_part)?;

        trace!(
            "Parsed filter rule: system={}, filter={:?}, exclude={}, version={:?}",
            system_url, filter, is_exclude, version
        );

        let component_rule = if is_exclude {
            ComponentRule::ExcludeFilter {
                system: system_url,
                filter,
                version,
            }
        } else {
            ComponentRule::IncludeFilter {
                system: system_url,
                filter,
                version,
            }
        };

        Ok(Some(component_rule))
    }

    /// Parse a filter expression: property op value
    ///
    /// Examples:
    /// - `concept is-a #123456`
    /// - `STATUS = ACTIVE`
    /// - `concept regex "^A.*"`
    /// - `inactive exists true`
    /// - `concept generalizes #123456`
    /// - `STATUS in "ACTIVE,INACTIVE"`
    /// - `concept not-in "123456,789012"`
    fn parse_filter_expression(&self, filter_text: &str) -> Result<ValueSetFilter, ExportError> {
        let filter_text = filter_text.trim();

        // Validate filter expression syntax
        self.validate_filter_syntax(filter_text)?;

        // Enhanced patterns to match more complex expressions
        let patterns = [
            // Concept-based filters
            (r"concept\s+is-a\s+#?(\S+)", "concept", "is-a"),
            (
                r"concept\s+descendent-of\s+#?(\S+)",
                "concept",
                "descendent-of",
            ),
            (r"concept\s+is-not-a\s+#?(\S+)", "concept", "is-not-a"),
            (r"concept\s+generalizes\s+#?(\S+)", "concept", "generalizes"),
            (r"concept\s+regex\s+(.+)", "concept", "regex"),
            // Property-based filters with various operators
            (r"(\w+)\s+exists\s+(true|false)", "", "exists"),
            (r"(\w+)\s+=\s+(.+)", "", "="),
            (r"(\w+)\s+!=\s+(.+)", "", "!="),
            (r"(\w+)\s+in\s+(.+)", "", "in"),
            (r"(\w+)\s+not-in\s+(.+)", "", "not-in"),
            // Special SNOMED CT filters
            (r"(\w+)\s+child-of\s+#?(\S+)", "", "child-of"),
            (r"(\w+)\s+parent-of\s+#?(\S+)", "", "parent-of"),
            (r"(\w+)\s+ancestor-of\s+#?(\S+)", "", "ancestor-of"),
        ];

        for (pattern, default_property, op) in patterns {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if let Some(captures) = regex.captures(filter_text) {
                    let property = if default_property.is_empty() {
                        captures.get(1).map(|m| m.as_str()).unwrap_or("concept")
                    } else {
                        default_property
                    };

                    let value_index = if default_property.is_empty() { 2 } else { 1 };
                    let value = captures
                        .get(value_index)
                        .map(|m| m.as_str().trim())
                        .unwrap_or("");

                    // Clean up value (remove quotes, # prefix)
                    let clean_value = self.clean_filter_value(value);

                    // Validate the operator is supported
                    self.validate_filter_operator(op, property)?;

                    return Ok(ValueSetFilter::new(property, op, clean_value));
                }
            }
        }

        // Fallback: try to parse as "property op value" with whitespace
        let parts: Vec<&str> = filter_text.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            let property = parts[0];
            let op = parts[1];
            let value = parts[2..].join(" ");
            let clean_value = self.clean_filter_value(&value);

            // Validate the operator
            self.validate_filter_operator(op, property)?;

            return Ok(ValueSetFilter::new(property, op, clean_value));
        }

        Err(ExportError::InvalidValue(format!(
            "Unable to parse filter expression: {}",
            filter_text
        )))
    }

    /// Validate filter expression syntax
    fn validate_filter_syntax(&self, filter_text: &str) -> Result<(), ExportError> {
        if filter_text.is_empty() {
            return Err(ExportError::InvalidValue(
                "Empty filter expression".to_string(),
            ));
        }

        // Check for balanced quotes
        let quote_count = filter_text.chars().filter(|&c| c == '"').count();
        if quote_count % 2 != 0 {
            return Err(ExportError::InvalidValue(format!(
                "Unbalanced quotes in filter expression: {}",
                filter_text
            )));
        }

        // Check for basic structure (property operator value)
        let parts: Vec<&str> = filter_text.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(ExportError::InvalidValue(format!(
                "Filter expression must have at least property and operator: {}",
                filter_text
            )));
        }

        Ok(())
    }

    /// Validate that the filter operator is supported
    fn validate_filter_operator(&self, op: &str, property: &str) -> Result<(), ExportError> {
        let valid_operators = [
            "=",
            "!=",
            "is-a",
            "descendent-of",
            "is-not-a",
            "regex",
            "in",
            "not-in",
            "generalizes",
            "exists",
            "child-of",
            "parent-of",
            "ancestor-of",
        ];

        if !valid_operators.contains(&op) {
            return Err(ExportError::InvalidValue(format!(
                "Unsupported filter operator '{}' for property '{}'",
                op, property
            )));
        }

        // Validate operator-property combinations
        match (property, op) {
            ("concept", "exists") => {
                return Err(ExportError::InvalidValue(
                    "The 'exists' operator is not valid for the 'concept' property".to_string(),
                ));
            }
            (_, "is-a" | "descendent-of" | "is-not-a" | "generalizes") if property != "concept" => {
                warn!(
                    "Hierarchical operator '{}' used with property '{}' - may not be supported by all systems",
                    op, property
                );
            }
            _ => {} // Valid combination
        }

        Ok(())
    }

    /// Clean up filter value (remove quotes, # prefix, etc.)
    fn clean_filter_value(&self, value: &str) -> String {
        let value = value.trim();

        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            // Remove surrounding quotes
            value[1..value.len() - 1].to_string()
        } else if value.starts_with('#') {
            // Remove # prefix for codes
            value[1..].to_string()
        } else {
            value.to_string()
        }
    }

    /// Parse a ValueSet reference: codes from valueset "URL"
    fn parse_valueset_reference(
        &self,
        rule_text: &str,
        is_exclude: bool,
    ) -> Result<Option<ComponentRule>, ExportError> {
        // Extract the ValueSet URL after "codes from valueset"
        if let Some(url_start) = rule_text.find("codes from valueset") {
            let url_part = rule_text[url_start + "codes from valueset".len()..].trim();
            let value_set_url = self.extract_string_value(url_part);

            let component_rule = if is_exclude {
                ComponentRule::ExcludeValueSet { value_set_url }
            } else {
                ComponentRule::IncludeValueSet { value_set_url }
            };

            return Ok(Some(component_rule));
        }

        Ok(None)
    }

    /// Validate that exclude rules don't conflict with include rules
    ///
    /// Checks for potential conflicts where the same system/concept is both included and excluded
    fn validate_exclude_conflicts(
        &self,
        compose: &ValueSetCompose,
        valueset_name: &str,
    ) -> Result<(), ExportError> {
        let empty_vec = Vec::new();
        let includes = compose.include.as_ref().unwrap_or(&empty_vec);
        let excludes = compose.exclude.as_ref().unwrap_or(&empty_vec);

        for exclude in excludes {
            for include in includes {
                // Check for same system conflicts
                if let (Some(exclude_system), Some(include_system)) =
                    (&exclude.system, &include.system)
                {
                    if exclude_system == include_system {
                        // Check for specific concept conflicts
                        if let (Some(exclude_concepts), Some(include_concepts)) =
                            (&exclude.concept, &include.concept)
                        {
                            for exclude_concept in exclude_concepts {
                                for include_concept in include_concepts {
                                    if exclude_concept.code == include_concept.code {
                                        warn!(
                                            "ValueSet {}: Concept {}#{} is both included and excluded",
                                            valueset_name, exclude_system, exclude_concept.code
                                        );
                                    }
                                }
                            }
                        }

                        // Check for filter conflicts (more complex, just warn for now)
                        if exclude.filter.is_some() && include.filter.is_some() {
                            warn!(
                                "ValueSet {}: System {} has both include and exclude filters - potential conflicts",
                                valueset_name, exclude_system
                            );
                        }
                    }
                }

                // Check for ValueSet reference conflicts
                if let (Some(exclude_valuesets), Some(include_valuesets)) =
                    (&exclude.value_set, &include.value_set)
                {
                    for exclude_vs in exclude_valuesets {
                        for include_vs in include_valuesets {
                            if exclude_vs == include_vs {
                                warn!(
                                    "ValueSet {}: ValueSet {} is both included and excluded",
                                    valueset_name, exclude_vs
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse concept metadata like properties and designations
    ///
    /// Handles extended syntax like:
    /// - `* LOINC#12345-6 "Blood pressure" ^property[status] = #active`
    /// - `* SNOMED#123456 "Condition" ^designation[0].language = #en ^designation[0].value = "English term"`
    fn parse_concept_metadata(
        &self,
        concept: &mut ValueSetConcept,
        path_rule: &PathRule,
    ) -> Result<(), ExportError> {
        let rule_text = path_rule.syntax().text().to_string();

        // Look for property assignments: ^property[name] = value
        if rule_text.contains("^property[") {
            self.parse_concept_properties(concept, &rule_text)?;
        }

        // Look for designation assignments: ^designation[index].field = value
        if rule_text.contains("^designation[") {
            self.parse_concept_designations(concept, &rule_text)?;
        }

        Ok(())
    }

    /// Parse concept properties from rule text
    ///
    /// Examples:
    /// - `^property[status] = #active`
    /// - `^property[inactive] = false`
    /// - `^property[parent] = #123456`
    fn parse_concept_properties(
        &self,
        concept: &mut ValueSetConcept,
        rule_text: &str,
    ) -> Result<(), ExportError> {
        // Simple regex to find property assignments
        if let Ok(regex) = regex::Regex::new(r"\^property\[([^\]]+)\]\s*=\s*([^,\s]+)") {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(property_name), Some(property_value)) =
                    (captures.get(1), captures.get(2))
                {
                    let name = property_name.as_str();
                    let value = property_value.as_str();

                    // Determine property type and create appropriate property
                    let property = if value.starts_with('#') {
                        // Code value
                        ValueSetConceptProperty::code(name, &value[1..])
                    } else if value == "true" || value == "false" {
                        // Boolean value
                        ValueSetConceptProperty::boolean(name, value == "true")
                    } else if let Ok(int_val) = value.parse::<i32>() {
                        // Integer value
                        ValueSetConceptProperty::integer(name, int_val)
                    } else if value.starts_with('"') && value.ends_with('"') {
                        // String value
                        ValueSetConceptProperty::string(name, &value[1..value.len() - 1])
                    } else {
                        // Default to string
                        ValueSetConceptProperty::string(name, value)
                    };

                    concept.add_property(property);
                }
            }
        }

        Ok(())
    }

    /// Parse concept designations from rule text
    ///
    /// Examples:
    /// - `^designation[0].language = #en`
    /// - `^designation[0].value = "English term"`
    /// - `^designation[1].language = #es ^designation[1].value = "Término español"`
    fn parse_concept_designations(
        &self,
        concept: &mut ValueSetConcept,
        rule_text: &str,
    ) -> Result<(), ExportError> {
        use std::collections::HashMap;

        // Collect designation assignments by index
        let mut designations: HashMap<usize, (Option<String>, Option<String>)> = HashMap::new();

        // Parse language assignments
        if let Ok(regex) = regex::Regex::new(r"\^designation\[(\d+)\]\.language\s*=\s*#?([^\s,]+)")
        {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(index_match), Some(lang_match)) = (captures.get(1), captures.get(2)) {
                    if let Ok(index) = index_match.as_str().parse::<usize>() {
                        let entry = designations.entry(index).or_insert((None, None));
                        entry.0 = Some(lang_match.as_str().to_string());
                    }
                }
            }
        }

        // Parse value assignments
        if let Ok(regex) = regex::Regex::new(r#"\^designation\[(\d+)\]\.value\s*=\s*"([^"]+)""#) {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(index_match), Some(value_match)) = (captures.get(1), captures.get(2)) {
                    if let Ok(index) = index_match.as_str().parse::<usize>() {
                        let entry = designations.entry(index).or_insert((None, None));
                        entry.1 = Some(value_match.as_str().to_string());
                    }
                }
            }
        }

        // Create designation objects
        for (_index, (language, value)) in designations {
            if let Some(designation_value) = value {
                let mut designation = ValueSetConceptDesignation::new(designation_value);
                if let Some(lang) = language {
                    designation.language = Some(lang);
                }
                concept.add_designation(designation);
            }
        }

        Ok(())
    }

    /// Extract display text from a PathRule
    ///
    /// Looks for a String token following the path in the syntax tree.
    /// Example: `* SPTY#AMN "Amniotic fluid"` -> Some("Amniotic fluid")
    fn get_display_text(&self, path_rule: &PathRule) -> Option<String> {
        use crate::cst::FshSyntaxKind;

        // Look for a String token following the PathRule node
        // We iterate through all descendants with tokens to find a String
        for child in path_rule.syntax().descendants_with_tokens() {
            if let rowan::NodeOrToken::Token(token) = child {
                if token.kind() == FshSyntaxKind::String {
                    let text = token.text();
                    // Remove surrounding quotes
                    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                        return Some(text[1..text.len() - 1].to_string());
                    } else {
                        return Some(text.to_string());
                    }
                }
            }
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{ValueSetConcept, ValueSetFilter, ValueSetInclude};

    fn create_test_exporter() -> ValueSetExporter {
        ValueSetExporter {
            session: Arc::new(crate::canonical::DefinitionSession::for_testing()),
            base_url: "http://example.org/fhir".to_string(),
            version: None,
        }
    }

    #[test]
    fn test_exporter_creation() {
        let exporter = create_test_exporter();
        assert_eq!(exporter.base_url, "http://example.org/fhir");
    }

    #[test]
    fn test_extract_string_value() {
        let exporter = create_test_exporter();

        assert_eq!(
            exporter.extract_string_value("\"Hello World\""),
            "Hello World"
        );
        assert_eq!(
            exporter.extract_string_value("'Hello World'"),
            "Hello World"
        );
        assert_eq!(exporter.extract_string_value("NoQuotes"), "NoQuotes");
        assert_eq!(exporter.extract_string_value("  \"Spaces\"  "), "Spaces");
    }

    #[test]
    fn test_valueset_resource_creation() {
        let vs = ValueSetResource::new(
            "http://example.org/fhir/ValueSet/test-vs",
            "TestVS",
            "draft",
        );

        assert_eq!(vs.resource_type, "ValueSet");
        assert_eq!(vs.url, "http://example.org/fhir/ValueSet/test-vs");
        assert_eq!(vs.name, "TestVS");
        assert_eq!(vs.status, "draft");
        assert!(vs.id.is_none());
        assert!(vs.compose.is_none());
    }

    #[test]
    fn test_valueset_compose_add_include() {
        let mut compose = ValueSetCompose::new();
        let include = ValueSetInclude::from_system("http://loinc.org");

        compose.add_include(include);

        assert!(compose.include.is_some());
        assert_eq!(compose.include.as_ref().unwrap().len(), 1);
        assert_eq!(
            compose.include.as_ref().unwrap()[0]
                .system
                .as_ref()
                .unwrap(),
            "http://loinc.org"
        );
    }

    #[test]
    fn test_valueset_include_with_concept() {
        let mut include = ValueSetInclude::from_system("http://loinc.org");
        include.add_concept(ValueSetConcept::with_display("12345-6", "Blood pressure"));

        assert!(include.concept.is_some());
        assert_eq!(include.concept.as_ref().unwrap().len(), 1);
        assert_eq!(include.concept.as_ref().unwrap()[0].code, "12345-6");
        assert_eq!(
            include.concept.as_ref().unwrap()[0]
                .display
                .as_ref()
                .unwrap(),
            "Blood pressure"
        );
    }

    #[test]
    fn test_valueset_filter_is_a() {
        let filter = ValueSetFilter::is_a("123456");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "is-a");
        assert_eq!(filter.value, "123456");
    }

    #[test]
    fn test_valueset_filter_regex() {
        let filter = ValueSetFilter::regex("^(A|B).*");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "regex");
        assert_eq!(filter.value, "^(A|B).*");
    }

    #[test]
    fn test_valueset_filter_descendent_of() {
        let filter = ValueSetFilter::descendent_of("12345");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "descendent-of");
        assert_eq!(filter.value, "12345");
    }

    #[test]
    fn test_valueset_include_with_filter() {
        let mut include = ValueSetInclude::from_system("http://snomed.info/sct");
        include.add_filter(ValueSetFilter::is_a("123456"));

        assert!(include.filter.is_some());
        assert_eq!(include.filter.as_ref().unwrap().len(), 1);
        assert_eq!(include.filter.as_ref().unwrap()[0].op, "is-a");
    }

    #[test]
    fn test_valueset_compose_add_exclude() {
        let mut compose = ValueSetCompose::new();
        let exclude = ValueSetInclude::from_system("http://loinc.org");

        compose.add_exclude(exclude);

        assert!(compose.exclude.is_some());
        assert_eq!(compose.exclude.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_valueset_include_from_valueset() {
        let include = ValueSetInclude::from_valueset("http://example.org/vs/vital-signs");

        assert!(include.value_set.is_some());
        assert_eq!(
            include.value_set.as_ref().unwrap()[0],
            "http://example.org/vs/vital-signs"
        );
        assert!(include.system.is_none());
    }
}
