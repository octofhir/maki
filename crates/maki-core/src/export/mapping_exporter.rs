//! Mapping Exporter
//!
//! Applies FSH Mapping definitions to StructureDefinitions by adding mapping metadata.
//!
//! # Overview
//!
//! Unlike other FSH constructs, Mappings do NOT create standalone FHIR resources.
//! Instead, they:
//! 1. Add `StructureDefinition.mapping` metadata entries
//! 2. Apply mapping rules to individual `ElementDefinition.mapping` entries
//!
//! This follows SUSHI's implementation and the FHIR specification.
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::MappingExporter;
//! use maki_core::cst::ast::Mapping;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Parse FSH mapping
//! let mapping: Mapping = todo!();
//!
//! // Create exporter
//! let exporter = MappingExporter::new(session).await?;
//!
//! // Apply mapping to source StructureDefinition
//! exporter.apply_mapping(&mapping).await?;
//! # Ok(())
//! # }
//! ```

use super::ExportError;
use crate::canonical::DefinitionSession;
use crate::cst::ast::{Mapping, Rule};
use crate::export::fhir_types::{
    ElementDefinitionMapping, StructureDefinition, StructureDefinitionMapping,
};
use crate::semantic::FishingContext;
use std::sync::Arc;
use tracing::{debug, trace, warn};

// ============================================================================
// Mapping Exporter
// ============================================================================

/// Applies FSH Mapping definitions to StructureDefinitions
///
/// # Architecture
///
/// Following SUSHI's implementation, the MappingExporter:
/// 1. Finds the source StructureDefinition (Profile/Extension/Logical/Resource)
/// 2. Adds StructureDefinition.mapping metadata entry
/// 3. Applies mapping rules to ElementDefinition.mapping entries
///
/// # Example
///
/// ```rust,no_run
/// # use maki_core::export::MappingExporter;
/// # use maki_core::canonical::DefinitionSession;
/// # use std::sync::Arc;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let fishing_context: Arc<FishingContext> = todo!();
/// let exporter = MappingExporter::new(session, fishing_context).await?;
/// # Ok(())
/// # }
/// ```
pub struct MappingExporter {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
    /// Fishing context for resolving source StructureDefinitions
    fishing_context: Option<Arc<FishingContext>>,
}

impl MappingExporter {
    /// Create a new mapping exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving FHIR definitions
    /// * `fishing_context` - Optional FishingContext for finding source resources
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::MappingExporter;
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let fishing_context = None;
    /// let exporter = MappingExporter::new(session, fishing_context).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        fishing_context: Option<Arc<FishingContext>>,
    ) -> Result<Self, ExportError> {
        Ok(Self {
            session,
            fishing_context,
        })
    }

    /// Apply a Mapping to its source StructureDefinition
    ///
    /// # Arguments
    ///
    /// * `mapping` - FSH Mapping AST node
    /// * `structure_defs` - Mutable map of StructureDefinitions to modify
    ///
    /// # Returns
    ///
    /// Ok(()) if mapping was successfully applied
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Mapping name is missing
    /// - Source StructureDefinition not found
    /// - Element path resolution fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::MappingExporter;
    /// # use maki_core::cst::ast::Mapping;
    /// # use std::collections::HashMap;
    /// # async fn example(exporter: MappingExporter, mapping: Mapping) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut structure_defs = HashMap::new();
    /// exporter.apply_mapping(&mapping, &mut structure_defs).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn apply_mapping(
        &self,
        mapping: &Mapping,
        structure_defs: &mut std::collections::HashMap<String, StructureDefinition>,
    ) -> Result<(), ExportError> {
        let name = mapping.name().ok_or_else(|| {
            ExportError::MissingRequiredField("Mapping name is required".to_string())
        })?;

        debug!("Applying mapping: {}", name);

        // Get id from id clause or use name
        let id = mapping
            .id()
            .and_then(|id| id.value())
            .unwrap_or_else(|| name.clone());

        // Get source resource name
        let source_name = mapping.source().and_then(|s| s.value()).ok_or_else(|| {
            ExportError::MissingRequiredField(format!("Mapping {} requires Source clause", name))
        })?;

        debug!("Looking for source StructureDefinition: {}", source_name);

        // Find source StructureDefinition
        let source_sd = structure_defs.get_mut(&source_name).ok_or_else(|| {
            ExportError::ParentNotFound(format!(
                "Source StructureDefinition '{}' not found for Mapping '{}'",
                source_name, name
            ))
        })?;

        trace!("Found source StructureDefinition: {}", source_sd.name);

        // Create StructureDefinition.mapping entry
        let sd_mapping = StructureDefinitionMapping {
            identity: id.clone(),
            uri: mapping.target().and_then(|t| t.value()),
            name: mapping.title().and_then(|t| t.value()),
            comment: mapping.description().and_then(|d| d.value()),
        };

        // Add to StructureDefinition.mapping array
        if source_sd.mapping.is_none() {
            source_sd.mapping = Some(Vec::new());
        }
        source_sd.mapping.as_mut().unwrap().push(sd_mapping);

        debug!("Added StructureDefinition.mapping for identity: {}", id);

        // Apply mapping rules to elements
        for rule in mapping.rules() {
            if let Rule::Mapping(mapping_rule) = rule {
                self.apply_mapping_rule(source_sd, &mapping_rule, &id)?;
            }
        }

        debug!("Successfully applied mapping: {}", name);
        Ok(())
    }

    /// Apply a single mapping rule to an element
    fn apply_mapping_rule(
        &self,
        structure_def: &mut StructureDefinition,
        mapping_rule: &crate::cst::ast::MappingRule,
        identity: &str,
    ) -> Result<(), ExportError> {
        // Get the path
        let path =
            mapping_rule
                .path()
                .map(|p| p.as_string())
                .ok_or_else(|| ExportError::InvalidPath {
                    path: "(missing)".to_string(),
                    resource: structure_def.name.clone(),
                })?;

        trace!("Applying mapping rule for path: {}", path);

        // Get the map expression
        let map = mapping_rule.map().ok_or_else(|| {
            ExportError::MissingRequiredField(format!(
                "Mapping rule for '{}' requires target expression",
                path
            ))
        })?;

        // Find element in differential or snapshot
        let element = structure_def
            .differential
            .as_mut()
            .and_then(|diff| diff.element.iter_mut().find(|e| e.path == path))
            .or_else(|| {
                structure_def
                    .snapshot
                    .as_mut()
                    .and_then(|snap| snap.element.iter_mut().find(|e| e.path == path))
            })
            .ok_or_else(|| ExportError::ElementNotFound {
                path: path.clone(),
                profile: structure_def.name.clone(),
            })?;

        // Create ElementDefinition.mapping entry
        let elem_mapping = ElementDefinitionMapping {
            identity: identity.to_string(),
            language: mapping_rule.language(),
            map,
            comment: mapping_rule.comment(),
        };

        // Add to ElementDefinition.mapping array
        if element.mapping.is_none() {
            element.mapping = Some(Vec::new());
        }
        element.mapping.as_mut().unwrap().push(elem_mapping);

        trace!("Added ElementDefinition.mapping for path: {}", path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::DefinitionSession;
    use crate::cst::{
        ast::{AstNode, Document},
        parse_fsh,
    };
    use crate::export::fhir_types::{
        StructureDefinition, StructureDefinitionDifferential, StructureDefinitionKind,
    };
    use std::collections::HashMap;

    fn create_test_exporter() -> MappingExporter {
        MappingExporter {
            session: Arc::new(DefinitionSession::for_testing()),
            fishing_context: None,
        }
    }

    #[test]
    fn test_apply_mapping_metadata() {
        let source = r#"
Mapping: PatientToV2
Id: patient-to-v2
Source: Patient
Target: "HL7 V2 PID segment"
Title: "FHIR Patient to V2 PID Mapping"
Description: "Maps FHIR Observation to HL7 V2 OBX segment"
"#;

        let (syntax, _errors) = parse_fsh(source);
        let doc = Document::cast(syntax).expect("Expected document");
        let mapping = doc.mappings().next().expect("Expected mapping");

        let exporter = create_test_exporter();

        // Create a test StructureDefinition
        let mut structure_defs = HashMap::new();
        let mut patient_sd = StructureDefinition::new(
            "http://hl7.org/fhir/StructureDefinition/Patient".to_string(),
            "Patient".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        );
        patient_sd.differential = Some(StructureDefinitionDifferential { element: vec![] });
        structure_defs.insert("Patient".to_string(), patient_sd);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(exporter.apply_mapping(&mapping, &mut structure_defs))
            .unwrap();

        let patient_sd = structure_defs.get("Patient").unwrap();
        assert!(patient_sd.mapping.is_some());
        let mappings = patient_sd.mapping.as_ref().unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].identity, "patient-to-v2");
        assert_eq!(mappings[0].uri, Some("HL7 V2 PID segment".to_string()));
        assert_eq!(
            mappings[0].name,
            Some("FHIR Patient to V2 PID Mapping".to_string())
        );
    }

    #[test]
    fn test_apply_mapping_with_rules() {
        let source = r#"
Mapping: PatientToV2
Id: patient-to-v2
Source: TestPatient
Target: "HL7 V2 PID segment"
* name -> "PID-5"
* status -> "OBX-11" "Observation result status"
"#;

        let (syntax, _errors) = parse_fsh(source);
        let doc = Document::cast(syntax).expect("Expected document");
        let mapping = doc.mappings().next().expect("Expected mapping");

        let exporter = create_test_exporter();

        // Create a test StructureDefinition with elements
        let mut structure_defs = HashMap::new();
        let mut patient_sd = StructureDefinition::new(
            "http://example.org/StructureDefinition/TestPatient".to_string(),
            "TestPatient".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        );

        let mut name_element =
            crate::export::fhir_types::ElementDefinition::new("name".to_string());
        let mut status_element =
            crate::export::fhir_types::ElementDefinition::new("status".to_string());

        patient_sd.differential = Some(StructureDefinitionDifferential {
            element: vec![name_element, status_element],
        });
        structure_defs.insert("TestPatient".to_string(), patient_sd);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(exporter.apply_mapping(&mapping, &mut structure_defs))
            .unwrap();

        let patient_sd = structure_defs.get("TestPatient").unwrap();
        let diff = patient_sd.differential.as_ref().unwrap();

        // Check name element mapping
        let name_elem = diff.element.iter().find(|e| e.path == "name").unwrap();
        assert!(name_elem.mapping.is_some());
        let name_mappings = name_elem.mapping.as_ref().unwrap();
        assert_eq!(name_mappings.len(), 1);
        assert_eq!(name_mappings[0].identity, "patient-to-v2");
        assert_eq!(name_mappings[0].map, "PID-5");
        assert_eq!(name_mappings[0].comment, None);

        // Check status element mapping
        let status_elem = diff.element.iter().find(|e| e.path == "status").unwrap();
        assert!(status_elem.mapping.is_some());
        let status_mappings = status_elem.mapping.as_ref().unwrap();
        assert_eq!(status_mappings.len(), 1);
        assert_eq!(status_mappings[0].identity, "patient-to-v2");
        assert_eq!(status_mappings[0].map, "OBX-11");
        assert_eq!(
            status_mappings[0].comment,
            Some("Observation result status".to_string())
        );
    }
}
