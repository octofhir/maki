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

use super::{ExportError, ValueSetCompose, ValueSetConcept, ValueSetInclude, ValueSetResource};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{AstNode, Document, FixedValueRule, PathRule, Rule, ValueSet};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

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

        // Group codes by system for includes
        let mut system_includes: HashMap<String, Vec<ValueSetConcept>> = HashMap::new();

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
                    // Parse: * SPTY#AMN "Amniotic fluid"
                    if let Some((system, concept)) = self.parse_component_rule(&path_rule, &alias_map, &name)? {
                        system_includes.entry(system).or_insert_with(Vec::new).push(concept);
                    }
                }
                Rule::AddElement(_)
                | Rule::Contains(_)
                | Rule::Only(_)
                | Rule::Obeys(_)
                | Rule::Mapping(_)
                | Rule::CaretValue(_) => {
                    // These rules don't apply to valuesets
                    trace!("Skipping contains/only/obeys rule in valueset");
                }
            }
        }

        // Build compose.include from grouped codes
        if !system_includes.is_empty() {
            for (system, concepts) in system_includes {
                let mut include = ValueSetInclude::from_system(system);
                for concept in concepts {
                    include.add_concept(concept);
                }
                compose.add_include(include);
            }
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

    /// Parse a component rule (PathRule) into a system and concept
    ///
    /// Handles syntax like:
    /// - `* SPTY#AMN "Amniotic fluid"` - code with display
    /// - `* SPTY#BLD` - code without display
    ///
    /// Returns: (resolved_system_url, ValueSetConcept)
    fn parse_component_rule(
        &self,
        path_rule: &PathRule,
        alias_map: &HashMap<String, String>,
        valueset_name: &str,
    ) -> Result<Option<(String, ValueSetConcept)>, ExportError> {
        // Get the path (e.g., "SPTY#AMN")
        let path = match path_rule.path() {
            Some(p) => p.as_string(),
            None => {
                warn!("PathRule without path in ValueSet {}", valueset_name);
                return Ok(None);
            }
        };

        // Parse: CodeSystemPrefix#Code
        let parts: Vec<&str> = path.split('#').collect();
        if parts.len() != 2 {
            warn!(
                "Invalid component rule format '{}' in ValueSet {}. Expected format: SYSTEM#CODE",
                path, valueset_name
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

        // Create concept
        let concept = if let Some(display_text) = display {
            ValueSetConcept::with_display(code, display_text)
        } else {
            ValueSetConcept::new(code)
        };

        trace!(
            "Parsed component rule: system={}, code={}, display={:?}",
            system_url,
            code,
            concept.display
        );

        Ok(Some((system_url, concept)))
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
