//! CodeSystem Exporter
//!
//! Exports FSH CodeSystem definitions to FHIR CodeSystem resources (JSON).
//!
//! # Overview
//!
//! CodeSystems define sets of codes with their meanings and relationships.
//! This module handles:
//! - Converting FSH CodeSystem metadata to FHIR CodeSystem resources
//! - Processing concept definitions
//! - Handling hierarchical concepts (parent-child relationships)
//! - Building complete concept trees
//!
//! # Status
//!
//! **Phase 1 (Current)**: Metadata export with foundation for concepts
//! - Exports id, url, title, description, status, publisher
//! - Creates skeleton CodeSystem resources with content="complete"
//! - Foundation for concept rules
//!
//! **Phase 2 (Future)**: Full concept parsing
//! - Parse concept rules from AST (* #code "Display" "Definition")
//! - Handle hierarchical concepts (* #parent #child)
//! - Support concept properties
//! - Build complete concept tree
//! - Note: Requires parser enhancements for CodeSystem concept syntax
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::CodeSystemExporter;
//! use maki_core::cst::ast::CodeSystem;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse FSH codesystem
//! let codesystem: CodeSystem = todo!();
//!
//! // Create exporter
//! let exporter = CodeSystemExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Export to FHIR JSON
//! let resource = exporter.export(&codesystem).await?;
//!
//! // Serialize
//! let json = serde_json::to_string_pretty(&resource)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

use super::{CodeSystemResource, ExportError};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{CodeSystem, FixedValueRule, Rule};
use std::sync::Arc;
use tracing::{debug, trace, warn};

// ============================================================================
// CodeSystem Exporter
// ============================================================================

/// Exports FSH CodeSystem definitions to FHIR CodeSystem resources
///
/// # Phase 1: Metadata Export
///
/// Currently exports:
/// - Basic metadata (id, url, name, title, description)
/// - Status, date, publisher, copyright
/// - Content type (defaults to "complete")
/// - Case sensitivity
///
/// # Future Enhancements
///
/// Phase 2 will add:
/// - Concept parsing from FSH rules
/// - Hierarchical concept support
/// - Concept properties
/// - Automatic concept counting
///
/// # Example
///
/// ```rust,no_run
/// # use maki_core::export::CodeSystemExporter;
/// # use maki_core::canonical::DefinitionSession;
/// # use std::sync::Arc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let exporter = CodeSystemExporter::new(
///     session,
///     "http://example.org/fhir".to_string(),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct CodeSystemExporter {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    session: Arc<DefinitionSession>,
    /// Base URL for codesystem canonical URLs
    base_url: String,
}

impl CodeSystemExporter {
    /// Create a new codesystem exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving dependencies
    /// * `base_url` - Base URL for generated codesystem identifiers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::CodeSystemExporter;
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = CodeSystemExporter::new(
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

    /// Export a CodeSystem to a FHIR resource (JSON)
    ///
    /// # Arguments
    ///
    /// * `codesystem` - FSH CodeSystem AST node
    ///
    /// # Returns
    ///
    /// A FHIR CodeSystem resource as JSON
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - CodeSystem name is missing
    /// - Required metadata fields are invalid
    /// - Rule application fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::CodeSystemExporter;
    /// # use maki_core::cst::ast::CodeSystem;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let exporter: CodeSystemExporter = todo!();
    /// # let codesystem: CodeSystem = todo!();
    /// let resource = exporter.export(&codesystem).await?;
    /// println!("{}", serde_json::to_string_pretty(&resource)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&self, codesystem: &CodeSystem) -> Result<CodeSystemResource, ExportError> {
        let name = codesystem
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("CodeSystem name".to_string()))?;

        debug!("Exporting CodeSystem: {}", name);

        // Generate canonical URL
        let url = format!("{}/CodeSystem/{}", self.base_url, name);

        // Create base resource with default status
        let mut resource = CodeSystemResource::new(url, name.clone(), "draft");

        // Set id from Id clause if present
        if let Some(id_clause) = codesystem.id()
            && let Some(id_value) = id_clause.value()
        {
            resource.id = Some(id_value);
        }

        // Set title from Title clause if present
        if let Some(title_clause) = codesystem.title()
            && let Some(title_value) = title_clause.value()
        {
            resource.title = Some(title_value);
        }

        // Set description from Description clause if present
        if let Some(desc_clause) = codesystem.description()
            && let Some(desc_value) = desc_clause.value()
        {
            resource.description = Some(desc_value);
        }

        // Track concept rules for Phase 2
        let mut has_concepts = false;
        let mut concept_count = 0;

        // Process rules
        for rule in codesystem.rules() {
            match rule {
                Rule::FixedValue(fixed_rule) => {
                    self.apply_metadata_rule(&mut resource, &fixed_rule)?;
                }
                Rule::Card(_) => {
                    // Card rules don't apply to codesystems
                    trace!("Skipping card rule in codesystem");
                }
                Rule::Flag(_) => {
                    // Flag rules don't apply to codesystems
                    trace!("Skipping flag rule in codesystem");
                }
                Rule::ValueSet(_) => {
                    // ValueSet rules don't apply to codesystem definitions
                    trace!("Skipping valueset rule in codesystem definition");
                }
                Rule::Path(_) => {
                    // Path rules are for concept definitions
                    // Phase 2: Parse and add to concept list
                    trace!("Concept rule detected (Phase 2 feature)");
                    has_concepts = true;
                    concept_count += 1;
                }
                Rule::AddElement(_)
                | Rule::Contains(_)
                | Rule::Only(_)
                | Rule::Obeys(_)
                | Rule::Mapping(_) => {
                    // These rules don't apply to codesystems
                    trace!("Skipping contains/only/obeys rule in codesystem");
                }
            }
        }

        // Log info about concepts if found
        if has_concepts {
            warn!(
                "CodeSystem {} has {} concept rules, but full parsing not yet implemented",
                name, concept_count
            );
            // For now, add a placeholder to indicate concepts should be here
            // Phase 2 will populate this properly
        }

        debug!("Successfully exported CodeSystem {}", name);
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
    /// - ^caseSensitive - Case sensitivity
    /// - ^content - Content type
    /// - ^experimental - Experimental flag
    fn apply_metadata_rule(
        &self,
        resource: &mut CodeSystemResource,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        // Get the path (property name)
        let path = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::InvalidPath {
                path: "<unknown>".to_string(),
                resource: "CodeSystem".to_string(),
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
            "caseSensitive" => {
                // Parse boolean
                let case_sensitive = value_str.trim() == "true";
                resource.case_sensitive = Some(case_sensitive);
            }
            "content" => {
                // Extract code value
                let content = if let Some(code) = value_str.strip_prefix('#') {
                    code.to_string()
                } else {
                    value_str.clone()
                };
                resource.content = content;
            }
            "experimental" => {
                // Parse boolean
                let experimental = value_str.trim() == "true";
                resource.experimental = Some(experimental);
            }
            _ => {
                // Unknown property - log warning but don't fail
                warn!("Unknown CodeSystem metadata property: {}", property);
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
    use crate::export::CodeSystemConcept;

    fn create_test_exporter() -> CodeSystemExporter {
        CodeSystemExporter {
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
    fn test_codesystem_resource_creation() {
        let cs = CodeSystemResource::new(
            "http://example.org/fhir/CodeSystem/test-cs",
            "TestCS",
            "draft",
        );

        assert_eq!(cs.resource_type, "CodeSystem");
        assert_eq!(cs.url, "http://example.org/fhir/CodeSystem/test-cs");
        assert_eq!(cs.name, "TestCS");
        assert_eq!(cs.status, "draft");
        assert_eq!(cs.content, "complete");
        assert!(cs.id.is_none());
        assert!(cs.concept.is_none());
    }

    #[test]
    fn test_codesystem_add_concept() {
        let mut cs = CodeSystemResource::new(
            "http://example.org/fhir/CodeSystem/test-cs",
            "TestCS",
            "draft",
        );

        cs.add_concept(CodeSystemConcept::with_display("active", "Active"));

        assert!(cs.concept.is_some());
        assert_eq!(cs.concept.as_ref().unwrap().len(), 1);
        assert_eq!(cs.concept.as_ref().unwrap()[0].code, "active");
        assert_eq!(
            cs.concept.as_ref().unwrap()[0].display.as_ref().unwrap(),
            "Active"
        );
    }

    #[test]
    fn test_codesystem_concept_new() {
        let concept = CodeSystemConcept::new("test-code");

        assert_eq!(concept.code, "test-code");
        assert!(concept.display.is_none());
        assert!(concept.definition.is_none());
    }

    #[test]
    fn test_codesystem_concept_with_display() {
        let concept = CodeSystemConcept::with_display("active", "Active");

        assert_eq!(concept.code, "active");
        assert_eq!(concept.display.as_ref().unwrap(), "Active");
        assert!(concept.definition.is_none());
    }

    #[test]
    fn test_codesystem_concept_with_definition() {
        let concept = CodeSystemConcept::with_definition(
            "active",
            "Active",
            "The resource is currently active",
        );

        assert_eq!(concept.code, "active");
        assert_eq!(concept.display.as_ref().unwrap(), "Active");
        assert_eq!(
            concept.definition.as_ref().unwrap(),
            "The resource is currently active"
        );
    }

    #[test]
    fn test_codesystem_concept_hierarchy() {
        let mut parent = CodeSystemConcept::with_display("active", "Active");
        let child = CodeSystemConcept::with_display("suspended", "Suspended");

        parent.add_child(child);

        assert!(parent.concept.is_some());
        assert_eq!(parent.concept.as_ref().unwrap().len(), 1);
        assert_eq!(parent.concept.as_ref().unwrap()[0].code, "suspended");
    }

    #[test]
    fn test_codesystem_update_count() {
        let mut cs = CodeSystemResource::new(
            "http://example.org/fhir/CodeSystem/test-cs",
            "TestCS",
            "draft",
        );

        // Add concepts
        cs.add_concept(CodeSystemConcept::new("code1"));
        cs.add_concept(CodeSystemConcept::new("code2"));

        // Add hierarchical concept
        let mut parent = CodeSystemConcept::new("parent");
        parent.add_child(CodeSystemConcept::new("child1"));
        parent.add_child(CodeSystemConcept::new("child2"));
        cs.add_concept(parent);

        cs.update_count();

        // Should count: code1(1) + code2(1) + parent(1) + child1(1) + child2(1) = 5
        assert_eq!(cs.count, Some(5));
    }

    #[test]
    fn test_codesystem_multiple_concepts() {
        let mut cs = CodeSystemResource::new(
            "http://example.org/fhir/CodeSystem/observation-status",
            "ObservationStatus",
            "active",
        );

        cs.add_concept(CodeSystemConcept::with_definition(
            "registered",
            "Registered",
            "The existence of the observation is registered",
        ));
        cs.add_concept(CodeSystemConcept::with_definition(
            "preliminary",
            "Preliminary",
            "This is an initial or interim observation",
        ));
        cs.add_concept(CodeSystemConcept::with_definition(
            "final",
            "Final",
            "The observation is complete",
        ));

        assert_eq!(cs.concept.as_ref().unwrap().len(), 3);
        cs.update_count();
        assert_eq!(cs.count, Some(3));
    }
}
