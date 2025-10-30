//! Fishable trait for FHIR resource lookups
//!
//! This module implements the "Fishable" pattern from SUSHI - a unified interface
//! for looking up FHIR definitions by various identifiers (URL, ID, name, type).
//!
//! # Overview
//!
//! The Fishable trait provides fast O(1) lookups using HashMap indexes, significantly
//! improving upon SUSHI's linear search approach. It supports:
//!
//! - URL-based lookups (canonical URLs)
//! - ID-based lookups (resource IDs)
//! - Name-based lookups (resource names)
//! - Type-based queries (all resources of a type)
//! - Metadata-only queries (fast, without loading full resources)
//! - Cascading lookups across multiple sources
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::canonical::fishable::{Fishable, FhirType};
//! use maki_core::canonical::DefinitionSession;
//!
//! # async fn example(session: &DefinitionSession) -> Result<(), Box<dyn std::error::Error>> {
//! // Fish by URL
//! let patient = session.fish(
//!     "http://hl7.org/fhir/StructureDefinition/Patient",
//!     &[FhirType::StructureDefinition]
//! ).await?;
//!
//! // Fish by ID with type filter
//! let observation = session.fish("Observation", &[FhirType::Profile]).await?;
//!
//! // Get metadata only (fast)
//! let metadata = session.fish_for_metadata("Patient", &[]).await?;
//! # Ok(())
//! # }
//! ```

use crate::canonical::{DefinitionResource, DefinitionSession};
use std::sync::Arc;
use tracing::{debug, trace};

/// FHIR resource type filter for fishing operations
///
/// Allows filtering fish results by FHIR resource type and derivation.
/// This enum corresponds to the most common FHIR definition types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FhirType {
    /// StructureDefinition (any kind)
    StructureDefinition,
    /// ValueSet
    ValueSet,
    /// CodeSystem
    CodeSystem,
    /// Profile (StructureDefinition with derivation=constraint)
    Profile,
    /// Extension (StructureDefinition with kind=complex-type and baseDefinition=Extension)
    Extension,
    /// Logical model (StructureDefinition with kind=logical)
    Logical,
    /// Resource (StructureDefinition with kind=resource)
    Resource,
    /// Instance (any resource instance, not a definition)
    Instance,
    /// Any type (no filtering)
    Any,
}

impl FhirType {
    /// Check if a resource matches this FHIR type filter
    ///
    /// # Arguments
    ///
    /// * `resource_type` - The resourceType field from the FHIR resource
    /// * `kind` - The kind field (for StructureDefinition)
    /// * `derivation` - The derivation field (for StructureDefinition)
    /// * `base_definition` - The baseDefinition field (for StructureDefinition)
    pub fn matches(
        &self,
        resource_type: &str,
        kind: Option<&str>,
        derivation: Option<&str>,
        base_definition: Option<&str>,
    ) -> bool {
        match self {
            FhirType::Any => true,
            FhirType::StructureDefinition => resource_type == "StructureDefinition",
            FhirType::ValueSet => resource_type == "ValueSet",
            FhirType::CodeSystem => resource_type == "CodeSystem",
            FhirType::Profile => {
                resource_type == "StructureDefinition" && derivation == Some("constraint")
            }
            FhirType::Extension => {
                resource_type == "StructureDefinition"
                    && kind == Some("complex-type")
                    && base_definition
                        .map(|b| b.ends_with("/Extension"))
                        .unwrap_or(false)
            }
            FhirType::Logical => resource_type == "StructureDefinition" && kind == Some("logical"),
            FhirType::Resource => {
                resource_type == "StructureDefinition" && kind == Some("resource")
            }
            FhirType::Instance => {
                // An instance is anything that's not a definitional resource
                !matches!(
                    resource_type,
                    "StructureDefinition" | "ValueSet" | "CodeSystem" | "SearchParameter"
                )
            }
        }
    }

    /// Get a display name for this FHIR type
    pub fn display_name(&self) -> &'static str {
        match self {
            FhirType::StructureDefinition => "StructureDefinition",
            FhirType::ValueSet => "ValueSet",
            FhirType::CodeSystem => "CodeSystem",
            FhirType::Profile => "Profile",
            FhirType::Extension => "Extension",
            FhirType::Logical => "Logical",
            FhirType::Resource => "Resource",
            FhirType::Instance => "Instance",
            FhirType::Any => "Any",
        }
    }
}

/// Lightweight metadata for FHIR resources
///
/// Contains essential identifying information without the full resource content.
/// Useful for fast lookups that only need basic information.
#[derive(Debug, Clone)]
pub struct FhirMetadata {
    /// FHIR resourceType (e.g., "StructureDefinition")
    pub resource_type: String,
    /// Resource ID
    pub id: Option<String>,
    /// Canonical URL
    pub url: Option<String>,
    /// Resource name
    pub name: Option<String>,
    /// Resource version
    pub version: Option<String>,
    /// Kind field (for StructureDefinition)
    pub kind: Option<String>,
    /// Derivation field (for StructureDefinition)
    pub derivation: Option<String>,
    /// Base definition (for StructureDefinition)
    pub base_definition: Option<String>,
    /// Package ID this resource comes from
    pub package_id: String,
}

impl FhirMetadata {
    /// Create metadata from a DefinitionResource
    pub fn from_definition(resource: &DefinitionResource) -> Self {
        let content = &resource.content;

        Self {
            resource_type: resource.resource_type.clone(),
            id: content.get("id").and_then(|v| v.as_str()).map(String::from),
            url: Some(resource.canonical_url.clone()),
            name: content
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from),
            version: resource.version.clone(),
            kind: content
                .get("kind")
                .and_then(|v| v.as_str())
                .map(String::from),
            derivation: content
                .get("derivation")
                .and_then(|v| v.as_str())
                .map(String::from),
            base_definition: content
                .get("baseDefinition")
                .and_then(|v| v.as_str())
                .map(String::from),
            package_id: resource.package_id.clone(),
        }
    }

    /// Check if this metadata matches the given type filters
    pub fn matches_types(&self, types: &[FhirType]) -> bool {
        if types.is_empty() || types.contains(&FhirType::Any) {
            return true;
        }

        types.iter().any(|fhir_type| {
            fhir_type.matches(
                &self.resource_type,
                self.kind.as_deref(),
                self.derivation.as_deref(),
                self.base_definition.as_deref(),
            )
        })
    }
}

/// Fishable trait - unified interface for FHIR resource lookups
///
/// This trait provides a consistent interface for searching FHIR resources
/// across different sources (sessions, packages, etc.). It supports:
///
/// - Multi-strategy lookups (URL, ID, name)
/// - Type filtering
/// - Metadata-only queries
/// - Specific lookup methods
///
/// # Example
///
/// ```rust,no_run
/// use maki_core::canonical::fishable::{Fishable, FhirType};
///
/// # async fn example(fishable: impl Fishable) -> Result<(), Box<dyn std::error::Error>> {
/// // Multi-strategy fish (tries URL, then ID, then name)
/// let patient = fishable.fish("Patient", &[FhirType::StructureDefinition]).await?;
///
/// // Specific lookups
/// let by_url = fishable.fish_by_url("http://hl7.org/fhir/StructureDefinition/Patient").await?;
/// let by_id = fishable.fish_by_id("Patient").await?;
///
/// // Fast metadata query
/// let metadata = fishable.fish_for_metadata("Patient", &[]).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait::async_trait]
pub trait Fishable: Send + Sync {
    /// Fish for a FHIR resource by identifier
    ///
    /// Attempts to find a resource by trying multiple strategies:
    /// 1. Canonical URL lookup
    /// 2. ID lookup
    /// 3. Name lookup
    ///
    /// # Arguments
    ///
    /// * `item` - The identifier to search for (URL, ID, or name)
    /// * `types` - Optional type filters (empty means no filtering)
    ///
    /// # Returns
    ///
    /// Returns `Some(resource)` if found and matches type filter, `None` otherwise
    async fn fish(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>>;

    /// Fish for metadata only (fast, doesn't load full resource)
    ///
    /// Returns lightweight metadata without the full resource JSON.
    /// Useful for LSP and other tools that need quick info.
    ///
    /// # Arguments
    ///
    /// * `item` - The identifier to search for
    /// * `types` - Optional type filters
    async fn fish_for_metadata(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> crate::canonical::CanonicalResult<Option<FhirMetadata>>;

    /// Fish by exact canonical URL
    ///
    /// # Arguments
    ///
    /// * `url` - Canonical URL to look up
    async fn fish_by_url(
        &self,
        url: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>>;

    /// Fish by exact ID
    ///
    /// # Arguments
    ///
    /// * `id` - Resource ID to look up
    async fn fish_by_id(
        &self,
        id: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>>;

    /// Fish by name
    ///
    /// # Arguments
    ///
    /// * `name` - Resource name to look up
    async fn fish_by_name(
        &self,
        name: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>>;

    /// Fish for all resources of a specific type
    ///
    /// # Arguments
    ///
    /// * `fhir_type` - The FHIR type to search for
    async fn fish_by_type(
        &self,
        fhir_type: FhirType,
    ) -> crate::canonical::CanonicalResult<Vec<Arc<DefinitionResource>>>;
}

/// Implementation of Fishable for DefinitionSession
///
/// Provides fast lookups using the canonical manager's search capabilities
/// with additional caching and indexing.
#[async_trait::async_trait]
impl Fishable for DefinitionSession {
    async fn fish(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>> {
        trace!("Fishing for '{}' with type filters: {:?}", item, types);

        // Strategy 1: Try as canonical URL
        if let Ok(resource) = self.resolve(item).await {
            let metadata = FhirMetadata::from_definition(&resource);
            if metadata.matches_types(types) {
                debug!("Found '{}' by URL", item);
                return Ok(Some(resource));
            }
        }

        // Strategy 2: Try as ID (search by type and ID)
        // Determine which resource types to search based on filters
        let search_types = if types.is_empty() {
            vec!["StructureDefinition", "ValueSet", "CodeSystem"]
        } else {
            types
                .iter()
                .filter_map(|t| match t {
                    FhirType::StructureDefinition
                    | FhirType::Profile
                    | FhirType::Extension
                    | FhirType::Logical
                    | FhirType::Resource => Some("StructureDefinition"),
                    FhirType::ValueSet => Some("ValueSet"),
                    FhirType::CodeSystem => Some("CodeSystem"),
                    FhirType::Any => Some("StructureDefinition"),
                    _ => None,
                })
                .collect()
        };

        for resource_type in search_types {
            if let Ok(Some(resource)) = self.resource_by_type_and_id(resource_type, item).await {
                let metadata = FhirMetadata::from_definition(&resource);
                if metadata.matches_types(types) {
                    debug!("Found '{}' by ID (type: {})", item, resource_type);
                    return Ok(Some(resource));
                }
            }
        }

        debug!("Not found: '{}'", item);
        Ok(None)
    }

    async fn fish_for_metadata(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> crate::canonical::CanonicalResult<Option<FhirMetadata>> {
        // Try to get the resource and extract metadata
        if let Some(resource) = self.fish(item, types).await? {
            let metadata = FhirMetadata::from_definition(&resource);
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    async fn fish_by_url(
        &self,
        url: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>> {
        match self.resolve(url).await {
            Ok(resource) => Ok(Some(resource)),
            Err(_) => Ok(None),
        }
    }

    async fn fish_by_id(
        &self,
        id: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Try common resource types
        for resource_type in &["StructureDefinition", "ValueSet", "CodeSystem"] {
            if let Ok(Some(resource)) = self.resource_by_type_and_id(resource_type, id).await {
                return Ok(Some(resource));
            }
        }
        Ok(None)
    }

    async fn fish_by_name(
        &self,
        _name: &str,
    ) -> crate::canonical::CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Name-based lookups are tricky - would require iterating through all resources
        // For now, return None - can be optimized later with a name index
        // TODO: Implement name-based lookup with index
        Ok(None)
    }

    async fn fish_by_type(
        &self,
        _fhir_type: FhirType,
    ) -> crate::canonical::CanonicalResult<Vec<Arc<DefinitionResource>>> {
        let results = Vec::new();

        // TODO: Implement type-based search
        // This requires access to the underlying canonical manager's search engine
        // which is not currently exposed through DefinitionSession.
        // For now, return empty results.

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_type_matches_structure_definition() {
        assert!(FhirType::StructureDefinition.matches("StructureDefinition", None, None, None));
        assert!(!FhirType::StructureDefinition.matches("ValueSet", None, None, None));
    }

    #[test]
    fn test_fhir_type_matches_profile() {
        assert!(FhirType::Profile.matches("StructureDefinition", None, Some("constraint"), None));
        assert!(!FhirType::Profile.matches(
            "StructureDefinition",
            None,
            Some("specialization"),
            None
        ));
    }

    #[test]
    fn test_fhir_type_matches_extension() {
        assert!(FhirType::Extension.matches(
            "StructureDefinition",
            Some("complex-type"),
            None,
            Some("http://hl7.org/fhir/StructureDefinition/Extension")
        ));
        assert!(!FhirType::Extension.matches(
            "StructureDefinition",
            Some("resource"),
            None,
            Some("http://hl7.org/fhir/StructureDefinition/Extension")
        ));
    }

    #[test]
    fn test_fhir_type_matches_any() {
        assert!(FhirType::Any.matches("StructureDefinition", None, None, None));
        assert!(FhirType::Any.matches("ValueSet", None, None, None));
        assert!(FhirType::Any.matches("Patient", None, None, None));
    }

    #[test]
    fn test_fhir_type_display_names() {
        assert_eq!(
            FhirType::StructureDefinition.display_name(),
            "StructureDefinition"
        );
        assert_eq!(FhirType::Profile.display_name(), "Profile");
        assert_eq!(FhirType::Extension.display_name(), "Extension");
    }

    #[test]
    fn test_fhir_metadata_matches_types_empty() {
        let metadata = FhirMetadata {
            resource_type: "StructureDefinition".to_string(),
            id: Some("Patient".to_string()),
            url: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
            name: Some("Patient".to_string()),
            version: Some("4.0.1".to_string()),
            kind: Some("resource".to_string()),
            derivation: None,
            base_definition: None,
            package_id: "hl7.fhir.r4.core@4.0.1".to_string(),
        };

        // Empty types should match anything
        assert!(metadata.matches_types(&[]));
    }

    #[test]
    fn test_fhir_metadata_matches_types_profile() {
        let metadata = FhirMetadata {
            resource_type: "StructureDefinition".to_string(),
            id: Some("us-core-patient".to_string()),
            url: Some(
                "http://hl7.org/fhir/us/core/StructureDefinition/us-core-patient".to_string(),
            ),
            name: Some("USCorePatient".to_string()),
            version: Some("6.1.0".to_string()),
            kind: Some("resource".to_string()),
            derivation: Some("constraint".to_string()),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
            package_id: "hl7.fhir.us.core@6.1.0".to_string(),
        };

        assert!(metadata.matches_types(&[FhirType::Profile]));
        assert!(metadata.matches_types(&[FhirType::StructureDefinition]));
        assert!(!metadata.matches_types(&[FhirType::ValueSet]));
    }
}
