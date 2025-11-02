//! Fishing - FHIR Resource Resolution with Priority
//!
//! Implements SUSHI's three-tier fishing pattern for resolving FHIR resources:
//! 1. **Package** - Local exports (resources already exported in this build)
//! 2. **Tank** - FSH definitions (in-memory parsed FSH resources)
//! 3. **FHIRDefs** - External FHIR packages via CanonicalFacade
//!
//! The key insight is that if a resource is found in the Tank (tier 2), we return
//! None rather than falling through to tier 3. This prevents external definitions
//! from being used when a local FSH definition exists but hasn't been exported yet.

use crate::canonical::{CanonicalResult, DefinitionSession};
use crate::export::fhir_types::StructureDefinition;
use crate::semantic::{FhirResource, ResourceType};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock; // Use async-aware RwLock
use tracing::{debug, trace};

/// Lightweight metadata for fast resource lookups without full export
///
/// This struct contains only the essential fields needed for dependency resolution
/// and quick lookups, making it much faster than full FHIR export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishableMetadata {
    /// Resource ID (e.g., "patient-profile")
    pub id: String,
    /// Resource name (e.g., "PatientProfile")
    pub name: String,
    /// Canonical URL (e.g., "http://example.org/fhir/StructureDefinition/patient-profile")
    pub url: String,
    /// FHIR resource type (e.g., "StructureDefinition", "ValueSet")
    pub resource_type: String,
    /// For StructureDefinitions: "Profile", "Extension", "Logical"
    pub sd_type: Option<String>,
    /// Parent resource (for profiles and extensions)
    pub parent: Option<String>,
    /// Instance usage type (for instances)
    pub instance_usage: Option<String>,
}

/// FSH Tank - In-memory collection of parsed FSH resources
///
/// This represents all FSH definitions parsed from source files.
/// In SUSHI, this is called the "Tank" and contains the raw FSH definitions
/// before they're fully exported to FHIR JSON.
#[derive(Debug, Clone, Default)]
pub struct FshTank {
    /// Resources indexed by ID
    resources_by_id: HashMap<String, FhirResource>,
    /// Resources indexed by canonical URL (if available)
    resources_by_url: HashMap<String, FhirResource>,
    /// Resources indexed by name (for flexible lookup)
    resources_by_name: HashMap<String, FhirResource>,
}

impl FshTank {
    /// Create a new empty tank
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a resource to the tank
    pub fn add_resource(&mut self, resource: FhirResource) {
        let id = resource.id.clone();

        // Index by ID
        self.resources_by_id.insert(id.clone(), resource.clone());

        // Index by name if available
        if let Some(name) = &resource.name {
            self.resources_by_name
                .insert(name.clone(), resource.clone());
        }

        // For Profiles, Extensions, ValueSets, CodeSystems - index by canonical URL
        // The canonical URL is typically constructed from metadata
        if let Some(url) = self.construct_canonical_url(&resource) {
            self.resources_by_url.insert(url, resource.clone());
        }
    }

    /// Construct canonical URL for a resource based on metadata
    fn construct_canonical_url(&self, resource: &FhirResource) -> Option<String> {
        // This is a simplified version - in reality, this would use
        // the configured canonical URL base from the IG configuration
        match resource.resource_type {
            ResourceType::Profile
            | ResourceType::Extension
            | ResourceType::ValueSet
            | ResourceType::CodeSystem => {
                // In a full implementation, this would come from sushi-config.yaml
                // For now, we'll use a placeholder
                Some(format!(
                    "http://example.org/fhir/{}/{}",
                    resource.resource_type.as_str(),
                    resource.id
                ))
            }
            _ => None,
        }
    }

    /// Check if a resource exists in the tank by identifier
    ///
    /// Returns true if found, which signals that we should NOT use external definitions
    pub fn contains(&self, identifier: &str, resource_types: &[ResourceType]) -> bool {
        // Try ID lookup
        if let Some(resource) = self.resources_by_id.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return true;
            }
        }

        // Try URL lookup
        if let Some(resource) = self.resources_by_url.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return true;
            }
        }

        // Try name lookup
        if let Some(resource) = self.resources_by_name.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return true;
            }
        }

        false
    }

    /// Fish for a resource in the tank
    pub fn fish(&self, identifier: &str, resource_types: &[ResourceType]) -> Option<&FhirResource> {
        // Try ID lookup
        if let Some(resource) = self.resources_by_id.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return Some(resource);
            }
        }

        // Try URL lookup
        if let Some(resource) = self.resources_by_url.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return Some(resource);
            }
        }

        // Try name lookup
        if let Some(resource) = self.resources_by_name.get(identifier) {
            if resource_types.is_empty() || resource_types.contains(&resource.resource_type) {
                return Some(resource);
            }
        }

        None
    }

    /// Get all resources of a specific type
    pub fn get_resources_by_type(&self, resource_type: ResourceType) -> Vec<&FhirResource> {
        self.resources_by_id
            .values()
            .filter(|r| r.resource_type == resource_type)
            .collect()
    }

    /// Get all resources
    pub fn all_resources(&self) -> Vec<&FhirResource> {
        self.resources_by_id.values().collect()
    }
}

impl ResourceType {
    fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Profile => "StructureDefinition",
            ResourceType::Extension => "StructureDefinition",
            ResourceType::ValueSet => "ValueSet",
            ResourceType::CodeSystem => "CodeSystem",
            ResourceType::Instance => "Instance",
            ResourceType::Invariant => "Invariant",
            ResourceType::RuleSet => "RuleSet",
            ResourceType::Mapping => "Mapping",
            ResourceType::Logical => "StructureDefinition",
        }
    }
}

/// Package - Exported FHIR resources
///
/// This represents resources that have been fully exported to FHIR JSON format.
/// These are the highest priority in fishing lookups.
#[derive(Debug, Default)]
pub struct Package {
    /// Exported resources indexed by canonical URL
    resources: HashMap<String, Arc<JsonValue>>,
}

impl Package {
    /// Create a new empty package
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an exported resource to the package
    pub fn add_resource(&mut self, canonical_url: String, resource: JsonValue) {
        self.resources.insert(canonical_url, Arc::new(resource));
    }

    /// Fish for a resource in the package
    pub fn fish(&self, identifier: &str) -> Option<Arc<JsonValue>> {
        self.resources.get(identifier).cloned()
    }
}

/// Fishing Context - Coordinates all three tiers of resource resolution
///
/// Implements the SUSHI fishing pattern:
/// 1. Check Package (exported resources)
/// 2. Check Tank (FSH definitions) - returns None if found!
/// 3. Check Canonical (external FHIR packages)
pub struct FishingContext {
    /// Canonical facade for external FHIR package resolution
    canonical_session: Arc<DefinitionSession>,
    /// Tank containing parsed FSH resources
    tank: Arc<RwLock<FshTank>>,
    /// Package containing exported FHIR resources
    package: Arc<RwLock<Package>>,
}

impl FishingContext {
    /// Create a new fishing context
    pub fn new(
        canonical_session: Arc<DefinitionSession>,
        tank: Arc<RwLock<FshTank>>,
        package: Arc<RwLock<Package>>,
    ) -> Self {
        Self {
            canonical_session,
            tank,
            package,
        }
    }

    /// Fish for a FHIR resource following the three-tier priority
    ///
    /// # Arguments
    ///
    /// * `identifier` - Resource ID, canonical URL, or name
    /// * `resource_types` - Filter by resource types (empty = any type)
    ///
    /// # Returns
    ///
    /// * `Ok(Some(resource))` - Found in package or canonical
    /// * `Ok(None)` - Found in tank (blocking external lookup) OR not found anywhere
    /// * `Err(...)` - Error during canonical resolution
    pub async fn fish(
        &self,
        identifier: &str,
        resource_types: &[ResourceType],
    ) -> CanonicalResult<Option<Arc<JsonValue>>> {
        trace!("Fishing for {} (types: {:?})", identifier, resource_types);

        // Tier 1: Check package (already exported)
        if let Some(resource) = self.fish_in_package(identifier).await? {
            debug!("Found {} in package", identifier);
            return Ok(Some(resource));
        }

        // Tier 2: Check tank (FSH definitions)
        // If found here, return None to block external lookup
        if self.is_in_tank(identifier, resource_types).await {
            debug!("Found {} in tank - blocking external lookup", identifier);
            return Ok(None);
        }

        // Tier 3: Check canonical (external FHIR packages)
        self.fish_in_canonical(identifier).await
    }

    /// Fish for a StructureDefinition specifically
    pub async fn fish_structure_definition(
        &self,
        identifier: &str,
    ) -> CanonicalResult<Option<StructureDefinition>> {
        trace!("Fishing for StructureDefinition: {}", identifier);

        // Check package first
        if let Some(json) = self.fish_in_package(identifier).await? {
            if let Ok(sd) = serde_json::from_value((*json).clone()) {
                return Ok(Some(sd));
            }
        }

        // Check tank
        let resource_types = vec![
            ResourceType::Profile,
            ResourceType::Extension,
            ResourceType::Logical,
        ];
        if self.is_in_tank(identifier, &resource_types).await {
            debug!(
                "Found StructureDefinition {} in tank - blocking external lookup",
                identifier
            );
            return Ok(None);
        }

        // Check canonical
        match self
            .canonical_session
            .resolve_structure_definition(identifier)
            .await
        {
            Ok(Some(sd)) => {
                debug!("Found StructureDefinition {} in canonical", identifier);
                Ok(Some(sd))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                debug!("Error resolving StructureDefinition {}: {}", identifier, e);
                Err(e)
            }
        }
    }

    /// Check if a resource is in the package
    async fn fish_in_package(&self, identifier: &str) -> CanonicalResult<Option<Arc<JsonValue>>> {
        let package = self.package.read().await;
        Ok(package.fish(identifier))
    }

    /// Check if a resource is in the tank
    async fn is_in_tank(&self, identifier: &str, resource_types: &[ResourceType]) -> bool {
        let tank = self.tank.read().await;
        tank.contains(identifier, resource_types)
    }

    /// Fish in canonical (external FHIR packages)
    async fn fish_in_canonical(&self, identifier: &str) -> CanonicalResult<Option<Arc<JsonValue>>> {
        match self.canonical_session.resolve(identifier).await {
            Ok(resource) => {
                debug!("Found {} in canonical", identifier);
                Ok(Some(Arc::new((*resource.content).clone())))
            }
            Err(_) => {
                // Not found in canonical
                Ok(None)
            }
        }
    }

    /// Get access to the tank (for adding resources during parsing)
    pub fn tank(&self) -> Arc<RwLock<FshTank>> {
        Arc::clone(&self.tank)
    }

    /// Get access to the package (for adding exported resources)
    pub fn package(&self) -> Arc<RwLock<Package>> {
        Arc::clone(&self.package)
    }

    /// Get access to the canonical session
    pub fn canonical_session(&self) -> Arc<DefinitionSession> {
        Arc::clone(&self.canonical_session)
    }

    /// Extract lightweight metadata from a tank resource without full export
    ///
    /// This is much faster than full FHIR export and is sufficient for dependency
    /// resolution and quick lookups. Returns None if the resource is not found.
    ///
    /// # Arguments
    ///
    /// * `identifier` - Resource ID, canonical URL, or name
    /// * `resource_types` - Optional filter by resource types
    ///
    /// # Returns
    ///
    /// * `Some(metadata)` - Found resource with lightweight metadata
    /// * `None` - Resource not found in tank
    pub async fn fish_metadata(
        &self,
        identifier: &str,
        resource_types: &[ResourceType],
    ) -> Option<FishableMetadata> {
        let tank = self.tank.read().await;

        // Try to find the resource in the tank
        let resource = tank.fish(identifier, resource_types)?;

        // Extract lightweight metadata
        Some(Self::extract_metadata(resource))
    }

    /// Extract lightweight metadata from a FhirResource
    ///
    /// This is an internal helper that converts a full FhirResource into
    /// lightweight FishableMetadata for fast lookups.
    fn extract_metadata(resource: &FhirResource) -> FishableMetadata {
        // Determine the FHIR resource type string
        let resource_type = match resource.resource_type {
            ResourceType::Profile | ResourceType::Extension | ResourceType::Logical => {
                "StructureDefinition".to_string()
            }
            ResourceType::ValueSet => "ValueSet".to_string(),
            ResourceType::CodeSystem => "CodeSystem".to_string(),
            ResourceType::Instance => "Instance".to_string(),
            ResourceType::Invariant => "Invariant".to_string(),
            ResourceType::RuleSet => "RuleSet".to_string(),
            ResourceType::Mapping => "Mapping".to_string(),
        };

        // Determine the StructureDefinition type if applicable
        let sd_type = match resource.resource_type {
            ResourceType::Profile => Some("Profile".to_string()),
            ResourceType::Extension => Some("Extension".to_string()),
            ResourceType::Logical => Some("Logical".to_string()),
            _ => None,
        };

        // Construct canonical URL
        // Note: In a full implementation, this would use the canonical base from sushi-config.yaml
        let url = format!("http://example.org/fhir/{}/{}", resource_type, resource.id);

        FishableMetadata {
            id: resource.id.clone(),
            name: resource.name.clone().unwrap_or_else(|| resource.id.clone()),
            url,
            resource_type,
            sd_type,
            parent: resource.parent.clone(),
            instance_usage: None, // TODO: Extract from Instance metadata when available
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    fn create_test_resource(id: &str, name: &str, resource_type: ResourceType) -> FhirResource {
        FhirResource {
            resource_type,
            id: id.to_string(),
            name: Some(name.to_string()),
            title: None,
            description: None,
            parent: None,
            elements: Vec::new(),
            location: Location::default(),
            metadata: crate::semantic::ResourceMetadata::default(),
        }
    }

    #[test]
    fn test_tank_add_and_lookup() {
        let mut tank = FshTank::new();
        let resource =
            create_test_resource("patient-profile", "PatientProfile", ResourceType::Profile);

        tank.add_resource(resource);

        // Lookup by ID
        assert!(tank.contains("patient-profile", &[]));
        assert!(tank.fish("patient-profile", &[]).is_some());

        // Lookup by name
        assert!(tank.contains("PatientProfile", &[]));
        assert!(tank.fish("PatientProfile", &[]).is_some());

        // Lookup by URL
        assert!(tank.contains(
            "http://example.org/fhir/StructureDefinition/patient-profile",
            &[]
        ));
    }

    #[test]
    fn test_tank_type_filtering() {
        let mut tank = FshTank::new();
        tank.add_resource(create_test_resource(
            "patient-profile",
            "PatientProfile",
            ResourceType::Profile,
        ));
        tank.add_resource(create_test_resource(
            "patient-vs",
            "PatientVS",
            ResourceType::ValueSet,
        ));

        // Should find with matching type
        assert!(tank.contains("patient-profile", &[ResourceType::Profile]));
        assert!(!tank.contains("patient-profile", &[ResourceType::ValueSet]));

        // Should find with empty type filter
        assert!(tank.contains("patient-profile", &[]));
    }

    #[test]
    fn test_package_add_and_lookup() {
        let mut package = Package::new();
        let resource = serde_json::json!({
            "resourceType": "StructureDefinition",
            "id": "patient-profile",
            "name": "PatientProfile"
        });

        package.add_resource(
            "http://example.org/fhir/StructureDefinition/patient-profile".to_string(),
            resource,
        );

        let found = package.fish("http://example.org/fhir/StructureDefinition/patient-profile");
        assert!(found.is_some());
        assert_eq!(found.unwrap()["id"], "patient-profile");
    }

    #[test]
    fn test_fishing_priority_package_first() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));

        // Add to both package and tank
        {
            let mut pkg = package.write().unwrap();
            pkg.add_resource(
                "http://example.org/test".to_string(),
                serde_json::json!({"source": "package"}),
            );
        }
        {
            let mut t = tank.write().unwrap();
            t.add_resource(create_test_resource("test", "Test", ResourceType::Profile));
        }

        // Test package lookup directly
        let pkg = package.read().unwrap();
        let result = pkg.fish("http://example.org/test");
        assert!(result.is_some());
        assert_eq!(result.unwrap()["source"], "package");
    }

    #[test]
    fn test_fishing_tank_blocks_external() {
        let tank = Arc::new(RwLock::new(FshTank::new()));

        // Add only to tank
        {
            let mut t = tank.write().unwrap();
            t.add_resource(create_test_resource("test", "Test", ResourceType::Profile));
        }

        // Test tank lookup directly
        let t = tank.read().unwrap();
        let result = t.fish("test", &[ResourceType::Profile]);
        assert!(result.is_some());

        // Verify it's in the tank (would block external lookup)
        assert!(t.contains("test", &[ResourceType::Profile]));
    }

    // ===== Metadata Extraction Tests =====

    #[test]
    fn test_extract_metadata_profile() {
        let resource =
            create_test_resource("patient-profile", "PatientProfile", ResourceType::Profile);
        let metadata = FishingContext::extract_metadata(&resource);

        assert_eq!(metadata.id, "patient-profile");
        assert_eq!(metadata.name, "PatientProfile");
        assert_eq!(metadata.resource_type, "StructureDefinition");
        assert_eq!(metadata.sd_type, Some("Profile".to_string()));
        assert_eq!(metadata.parent, None);
        assert!(metadata.url.contains("patient-profile"));
    }

    #[test]
    fn test_extract_metadata_extension() {
        let mut resource =
            create_test_resource("my-extension", "MyExtension", ResourceType::Extension);
        resource.parent = Some("Extension".to_string());

        let metadata = FishingContext::extract_metadata(&resource);

        assert_eq!(metadata.id, "my-extension");
        assert_eq!(metadata.name, "MyExtension");
        assert_eq!(metadata.resource_type, "StructureDefinition");
        assert_eq!(metadata.sd_type, Some("Extension".to_string()));
        assert_eq!(metadata.parent, Some("Extension".to_string()));
    }

    #[test]
    fn test_extract_metadata_valueset() {
        let resource =
            create_test_resource("diagnosis-codes", "DiagnosisCodes", ResourceType::ValueSet);
        let metadata = FishingContext::extract_metadata(&resource);

        assert_eq!(metadata.id, "diagnosis-codes");
        assert_eq!(metadata.name, "DiagnosisCodes");
        assert_eq!(metadata.resource_type, "ValueSet");
        assert_eq!(metadata.sd_type, None);
        assert_eq!(metadata.parent, None);
    }

    #[test]
    fn test_extract_metadata_codesystem() {
        let resource = create_test_resource("color-codes", "ColorCodes", ResourceType::CodeSystem);
        let metadata = FishingContext::extract_metadata(&resource);

        assert_eq!(metadata.id, "color-codes");
        assert_eq!(metadata.name, "ColorCodes");
        assert_eq!(metadata.resource_type, "CodeSystem");
        assert_eq!(metadata.sd_type, None);
    }

    #[test]
    fn test_extract_metadata_logical() {
        let resource = create_test_resource("my-logical", "MyLogical", ResourceType::Logical);
        let metadata = FishingContext::extract_metadata(&resource);

        assert_eq!(metadata.id, "my-logical");
        assert_eq!(metadata.name, "MyLogical");
        assert_eq!(metadata.resource_type, "StructureDefinition");
        assert_eq!(metadata.sd_type, Some("Logical".to_string()));
    }

    #[test]
    fn test_fish_metadata_by_id() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(DefinitionSession::for_testing());

        // Add resource to tank
        {
            let mut t = tank.write().unwrap();
            let mut resource =
                create_test_resource("patient-profile", "PatientProfile", ResourceType::Profile);
            resource.parent = Some("Patient".to_string());
            t.add_resource(resource);
        }

        let ctx = FishingContext::new(session, tank, package);

        // Fish for metadata by ID
        let metadata = ctx.fish_metadata("patient-profile", &[]);
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.id, "patient-profile");
        assert_eq!(metadata.name, "PatientProfile");
        assert_eq!(metadata.parent, Some("Patient".to_string()));
    }

    #[test]
    fn test_fish_metadata_by_name() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(DefinitionSession::for_testing());

        // Add resource to tank
        {
            let mut t = tank.write().unwrap();
            t.add_resource(create_test_resource(
                "patient-profile",
                "PatientProfile",
                ResourceType::Profile,
            ));
        }

        let ctx = FishingContext::new(session, tank, package);

        // Fish for metadata by name
        let metadata = ctx.fish_metadata("PatientProfile", &[]);
        assert!(metadata.is_some());

        let metadata = metadata.unwrap();
        assert_eq!(metadata.id, "patient-profile");
        assert_eq!(metadata.name, "PatientProfile");
    }

    #[test]
    fn test_fish_metadata_with_type_filter() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(DefinitionSession::for_testing());

        // Add resources of different types
        {
            let mut t = tank.write().unwrap();
            t.add_resource(create_test_resource(
                "patient-profile",
                "PatientProfile",
                ResourceType::Profile,
            ));
            t.add_resource(create_test_resource(
                "patient-vs",
                "PatientVS",
                ResourceType::ValueSet,
            ));
        }

        let ctx = FishingContext::new(session, tank, package);

        // Should find profile when filtering by Profile
        let metadata = ctx.fish_metadata("patient-profile", &[ResourceType::Profile]);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().sd_type, Some("Profile".to_string()));

        // Should not find profile when filtering by ValueSet
        let metadata = ctx.fish_metadata("patient-profile", &[ResourceType::ValueSet]);
        assert!(metadata.is_none());

        // Should find valueset when filtering by ValueSet
        let metadata = ctx.fish_metadata("patient-vs", &[ResourceType::ValueSet]);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().resource_type, "ValueSet");
    }

    #[test]
    fn test_fish_metadata_not_found() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(DefinitionSession::for_testing());

        let ctx = FishingContext::new(session, tank, package);

        // Should return None for non-existent resource
        let metadata = ctx.fish_metadata("nonexistent", &[]);
        assert!(metadata.is_none());
    }

    #[test]
    fn test_fish_metadata_url_format() {
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(DefinitionSession::for_testing());

        {
            let mut t = tank.write().unwrap();
            t.add_resource(create_test_resource(
                "patient-profile",
                "PatientProfile",
                ResourceType::Profile,
            ));
            t.add_resource(create_test_resource(
                "my-vs",
                "MyVS",
                ResourceType::ValueSet,
            ));
        }

        let ctx = FishingContext::new(session, tank, package);

        // Check URL format for profile
        let metadata = ctx.fish_metadata("patient-profile", &[]).unwrap();
        assert_eq!(
            metadata.url,
            "http://example.org/fhir/StructureDefinition/patient-profile"
        );

        // Check URL format for valueset
        let metadata = ctx.fish_metadata("my-vs", &[]).unwrap();
        assert_eq!(metadata.url, "http://example.org/fhir/ValueSet/my-vs");
    }
}
