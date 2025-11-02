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

use super::{ExportError, ValueSetCompose, ValueSetResource};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{FixedValueRule, Rule, ValueSet};
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
    ) -> Result<Self, ExportError> {
        Ok(Self { session, base_url })
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

        // Initialize compose for component rules
        let compose = ValueSetCompose::new();
        let mut has_components = false;

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
                Rule::Path(_) => {
                    // Path rules are for component includes/excludes
                    // Phase 2: Parse and add to compose
                    trace!("Component rule detected (Phase 2 feature)");
                    has_components = true;
                }
                Rule::AddElement(_)
                | Rule::Contains(_)
                | Rule::Only(_)
                | Rule::Obeys(_)
                | Rule::Mapping(_) => {
                    // These rules don't apply to valuesets
                    trace!("Skipping contains/only/obeys rule in valueset");
                }
            }
        }

        // Add compose if we found any component rules
        if has_components {
            // For now, just add empty compose structure
            // Phase 2 will populate this with actual includes/excludes
            warn!(
                "ValueSet {} has component rules, but full parsing not yet implemented",
                name
            );
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
