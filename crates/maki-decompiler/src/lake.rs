//! ResourceLake - central repository for LOCAL FHIR resources
//!
//! This module implements a storage and lookup system for FHIR resources loaded
//! from local files. It implements the Fishable trait to provide cascading lookups:
//! 1. Local resources (from input files)
//! 2. Installed packages (via DefinitionSession)
//!
//! The ResourceLake stores only resources from the current project/input files,
//! while external dependencies are resolved through the DefinitionSession.

use crate::error::Result;
use crate::models::*;
use async_trait::async_trait;
use indexmap::IndexMap;
use maki_core::canonical::{
    CanonicalResult, DefinitionResource, DefinitionSession,
    fishable::{FhirMetadata, FhirType, Fishable},
};
use std::collections::HashMap;
use std::sync::Arc;

/// Central repository for LOCAL FHIR resources
///
/// Stores resources loaded from input files and provides fast lookup by:
/// - Canonical URL (primary key)
/// - Resource ID
/// - Resource name
///
/// External dependencies are resolved via DefinitionSession (canonical manager).
pub struct ResourceLake {
    /// StructureDefinitions by canonical URL (preserves insertion order)
    structure_definitions: IndexMap<String, StructureDefinition>,

    /// ValueSets by canonical URL
    value_sets: IndexMap<String, ValueSet>,

    /// CodeSystems by canonical URL
    code_systems: IndexMap<String, CodeSystem>,

    /// All other resources (instances) by ID
    instances: IndexMap<String, serde_json::Value>,

    /// URL → ID mapping for quick lookup
    url_to_id: HashMap<String, String>,

    /// ID → URL mapping
    id_to_url: HashMap<String, String>,

    /// Name → URL mapping (for resolution by name)
    name_to_url: HashMap<String, String>,

    /// DefinitionSession for resolving external dependencies
    /// (Uses existing canonical manager infrastructure)
    session: Arc<DefinitionSession>,
}

impl ResourceLake {
    /// Create a new ResourceLake with a DefinitionSession for external lookups
    pub fn new(session: Arc<DefinitionSession>) -> Self {
        Self {
            structure_definitions: IndexMap::new(),
            value_sets: IndexMap::new(),
            code_systems: IndexMap::new(),
            instances: IndexMap::new(),
            url_to_id: HashMap::new(),
            id_to_url: HashMap::new(),
            name_to_url: HashMap::new(),
            session,
        }
    }

    /// Add a StructureDefinition to the lake
    pub fn add_structure_definition(&mut self, sd: StructureDefinition) -> Result<()> {
        let url = sd.url.clone();

        // Build indices
        if let Some(id) = &sd.id {
            self.url_to_id.insert(url.clone(), id.clone());
            self.id_to_url.insert(id.clone(), url.clone());
        }
        self.name_to_url.insert(sd.name.clone(), url.clone());

        // Store resource
        self.structure_definitions.insert(url, sd);

        Ok(())
    }

    /// Add a ValueSet to the lake
    pub fn add_value_set(&mut self, vs: ValueSet) -> Result<()> {
        let url = vs.url.clone();

        if let Some(id) = &vs.id {
            self.url_to_id.insert(url.clone(), id.clone());
            self.id_to_url.insert(id.clone(), url.clone());
        }
        self.name_to_url.insert(vs.name.clone(), url.clone());

        self.value_sets.insert(url, vs);

        Ok(())
    }

    /// Add a CodeSystem to the lake
    pub fn add_code_system(&mut self, cs: CodeSystem) -> Result<()> {
        let url = cs.url.clone();

        if let Some(id) = &cs.id {
            self.url_to_id.insert(url.clone(), id.clone());
            self.id_to_url.insert(id.clone(), url.clone());
        }
        self.name_to_url.insert(cs.name.clone(), url.clone());

        self.code_systems.insert(url, cs);

        Ok(())
    }

    /// Add a generic FHIR resource (instance)
    pub fn add_instance(&mut self, id: String, resource: serde_json::Value) -> Result<()> {
        self.instances.insert(id, resource);
        Ok(())
    }

    /// Get StructureDefinition by URL
    pub fn get_structure_definition(&self, url: &str) -> Option<&StructureDefinition> {
        self.structure_definitions.get(url)
    }

    /// Get ValueSet by URL
    pub fn get_value_set(&self, url: &str) -> Option<&ValueSet> {
        self.value_sets.get(url)
    }

    /// Get CodeSystem by URL
    pub fn get_code_system(&self, url: &str) -> Option<&CodeSystem> {
        self.code_systems.get(url)
    }

    /// Resolve URL to FSH name (from local resources only)
    pub fn resolve_url_to_name(&self, url: &str) -> Option<String> {
        // Try StructureDefinitions
        if let Some(sd) = self.structure_definitions.get(url) {
            return Some(sd.name.clone());
        }

        // Try ValueSets
        if let Some(vs) = self.value_sets.get(url) {
            return Some(vs.name.clone());
        }

        // Try CodeSystems
        if let Some(cs) = self.code_systems.get(url) {
            return Some(cs.name.clone());
        }

        None
    }

    /// Remove duplicate resources (same URL, keep first)
    pub fn deduplicate(&mut self) {
        // IndexMap already ensures unique keys
        // This method is mainly for logging duplicates

        log::debug!(
            "Deduplication: {} structure definitions",
            self.structure_definitions.len()
        );
        log::debug!("Deduplication: {} value sets", self.value_sets.len());
        log::debug!("Deduplication: {} code systems", self.code_systems.len());
    }

    /// Assign missing IDs based on name or URL
    pub fn assign_missing_ids(&mut self) {
        // Assign IDs to StructureDefinitions
        for (url, sd) in self.structure_definitions.iter_mut() {
            if sd.id.is_none() {
                let generated_id = generate_id_from_name(&sd.name);
                log::debug!("Assigning ID '{}' to {}", generated_id, url);
                sd.id = Some(generated_id.clone());
                self.url_to_id.insert(url.clone(), generated_id.clone());
                self.id_to_url.insert(generated_id, url.clone());
            }
        }

        // Similar for ValueSets
        for (url, vs) in self.value_sets.iter_mut() {
            if vs.id.is_none() {
                let generated_id = generate_id_from_name(&vs.name);
                vs.id = Some(generated_id.clone());
                self.url_to_id.insert(url.clone(), generated_id.clone());
                self.id_to_url.insert(generated_id, url.clone());
            }
        }

        // Similar for CodeSystems
        for (url, cs) in self.code_systems.iter_mut() {
            if cs.id.is_none() {
                let generated_id = generate_id_from_name(&cs.name);
                cs.id = Some(generated_id.clone());
                self.url_to_id.insert(url.clone(), generated_id.clone());
                self.id_to_url.insert(generated_id, url.clone());
            }
        }
    }

    /// Iterate over all StructureDefinitions
    pub fn structure_definitions(&self) -> impl Iterator<Item = (&String, &StructureDefinition)> {
        self.structure_definitions.iter()
    }

    /// Iterate over all ValueSets
    pub fn value_sets(&self) -> impl Iterator<Item = (&String, &ValueSet)> {
        self.value_sets.iter()
    }

    /// Iterate over all CodeSystems
    pub fn code_systems(&self) -> impl Iterator<Item = (&String, &CodeSystem)> {
        self.code_systems.iter()
    }

    /// Iterate over all instances
    pub fn instances(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.instances.iter()
    }

    /// Get statistics about the lake
    pub fn stats(&self) -> LakeStats {
        LakeStats {
            structure_definitions: self.structure_definitions.len(),
            value_sets: self.value_sets.len(),
            code_systems: self.code_systems.len(),
            instances: self.instances.len(),
        }
    }
}

/// Helper: Generate ID from name (kebab-case)
fn generate_id_from_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[derive(Debug)]
pub struct LakeStats {
    pub structure_definitions: usize,
    pub value_sets: usize,
    pub code_systems: usize,
    pub instances: usize,
}

/// Helper: Convert StructureDefinition to FhirMetadata
fn sd_to_metadata(sd: &StructureDefinition) -> FhirMetadata {
    FhirMetadata {
        resource_type: "StructureDefinition".to_string(),
        id: sd.id.clone(),
        url: Some(sd.url.clone()),
        name: Some(sd.name.clone()),
        version: sd.version.clone(),
        kind: sd.kind.as_ref().map(|k| format!("{:?}", k).to_lowercase()),
        derivation: sd
            .derivation
            .as_ref()
            .map(|d| format!("{:?}", d).to_lowercase()),
        base_definition: sd.base_definition.clone(),
        package_id: "local".to_string(), // Local resources don't have a package
    }
}

/// Helper: Convert ValueSet to FhirMetadata
fn vs_to_metadata(vs: &ValueSet) -> FhirMetadata {
    FhirMetadata {
        resource_type: "ValueSet".to_string(),
        id: vs.id.clone(),
        url: Some(vs.url.clone()),
        name: Some(vs.name.clone()),
        version: vs.version.clone(),
        kind: None,
        derivation: None,
        base_definition: None,
        package_id: "local".to_string(),
    }
}

/// Helper: Convert CodeSystem to FhirMetadata
fn cs_to_metadata(cs: &CodeSystem) -> FhirMetadata {
    FhirMetadata {
        resource_type: "CodeSystem".to_string(),
        id: cs.id.clone(),
        url: Some(cs.url.clone()),
        name: Some(cs.name.clone()),
        version: cs.version.clone(),
        kind: None,
        derivation: None,
        base_definition: None,
        package_id: "local".to_string(),
    }
}

/// Implement the Fishable trait from maki-core::canonical
/// This provides cascading lookups: local resources → DefinitionSession
#[async_trait]
impl Fishable for ResourceLake {
    /// Fish with 3-level cascading lookup:
    /// 1. Local resources by URL
    /// 2. Local resources by name/ID
    /// 3. DefinitionSession (installed packages)
    async fn fish(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Level 1: Try local resources by URL
        if let Some(sd) = self.structure_definitions.get(item) {
            let metadata = sd_to_metadata(sd);
            if metadata.matches_types(types) {
                // Convert to DefinitionResource
                return convert_sd_to_resource(sd);
            }
        }
        if let Some(vs) = self.value_sets.get(item) {
            let metadata = vs_to_metadata(vs);
            if metadata.matches_types(types) {
                return convert_vs_to_resource(vs);
            }
        }
        if let Some(cs) = self.code_systems.get(item) {
            let metadata = cs_to_metadata(cs);
            if metadata.matches_types(types) {
                return convert_cs_to_resource(cs);
            }
        }

        // Level 2: Try local resources by name
        if let Some(url) = self.name_to_url.get(item) {
            return self.fish_by_url(url).await;
        }

        // Level 2: Try local resources by ID
        if let Some(url) = self.id_to_url.get(item) {
            return self.fish_by_url(url).await;
        }

        // Level 3: Fall back to DefinitionSession
        self.session.fish(item, types).await
    }

    async fn fish_for_metadata(
        &self,
        item: &str,
        types: &[FhirType],
    ) -> CanonicalResult<Option<FhirMetadata>> {
        // Try local first
        if let Some(sd) = self.structure_definitions.get(item) {
            let metadata = sd_to_metadata(sd);
            if metadata.matches_types(types) {
                return Ok(Some(metadata));
            }
        }
        if let Some(vs) = self.value_sets.get(item) {
            let metadata = vs_to_metadata(vs);
            if metadata.matches_types(types) {
                return Ok(Some(metadata));
            }
        }
        if let Some(cs) = self.code_systems.get(item) {
            let metadata = cs_to_metadata(cs);
            if metadata.matches_types(types) {
                return Ok(Some(metadata));
            }
        }

        // Try by name
        if let Some(url) = self.name_to_url.get(item) {
            if let Some(sd) = self.structure_definitions.get(url) {
                let metadata = sd_to_metadata(sd);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
            if let Some(vs) = self.value_sets.get(url) {
                let metadata = vs_to_metadata(vs);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
            if let Some(cs) = self.code_systems.get(url) {
                let metadata = cs_to_metadata(cs);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
        }

        // Try by ID
        if let Some(url) = self.id_to_url.get(item) {
            if let Some(sd) = self.structure_definitions.get(url) {
                let metadata = sd_to_metadata(sd);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
            if let Some(vs) = self.value_sets.get(url) {
                let metadata = vs_to_metadata(vs);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
            if let Some(cs) = self.code_systems.get(url) {
                let metadata = cs_to_metadata(cs);
                if metadata.matches_types(types) {
                    return Ok(Some(metadata));
                }
            }
        }

        // Fall back to session
        self.session.fish_for_metadata(item, types).await
    }

    async fn fish_by_url(&self, url: &str) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Check local first
        if let Some(sd) = self.structure_definitions.get(url) {
            return convert_sd_to_resource(sd);
        }
        if let Some(vs) = self.value_sets.get(url) {
            return convert_vs_to_resource(vs);
        }
        if let Some(cs) = self.code_systems.get(url) {
            return convert_cs_to_resource(cs);
        }

        // Fall back to session
        self.session.fish_by_url(url).await
    }

    async fn fish_by_id(&self, id: &str) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Check local by ID
        if let Some(url) = self.id_to_url.get(id) {
            return self.fish_by_url(url).await;
        }

        // Fall back to session
        self.session.fish_by_id(id).await
    }

    async fn fish_by_name(&self, name: &str) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Check local by name
        if let Some(url) = self.name_to_url.get(name) {
            return self.fish_by_url(url).await;
        }

        // Fall back to session
        self.session.fish_by_name(name).await
    }

    async fn fish_by_type(
        &self,
        fhir_type: FhirType,
    ) -> CanonicalResult<Vec<Arc<DefinitionResource>>> {
        let mut results = Vec::new();

        // Collect local resources of the specified type
        match fhir_type {
            FhirType::StructureDefinition
            | FhirType::Profile
            | FhirType::Extension
            | FhirType::Logical
            | FhirType::Resource
            | FhirType::Any => {
                for sd in self.structure_definitions.values() {
                    let metadata = sd_to_metadata(sd);
                    if metadata.matches_types(&[fhir_type])
                        && let Ok(Some(resource)) = convert_sd_to_resource(sd)
                    {
                        results.push(resource);
                    }
                }
            }
            _ => {}
        }

        match fhir_type {
            FhirType::ValueSet | FhirType::Any => {
                for vs in self.value_sets.values() {
                    if let Ok(Some(resource)) = convert_vs_to_resource(vs) {
                        results.push(resource);
                    }
                }
            }
            _ => {}
        }

        match fhir_type {
            FhirType::CodeSystem | FhirType::Any => {
                for cs in self.code_systems.values() {
                    if let Ok(Some(resource)) = convert_cs_to_resource(cs) {
                        results.push(resource);
                    }
                }
            }
            _ => {}
        }

        // Also get from session
        let session_results = self.session.fish_by_type(fhir_type).await?;
        results.extend(session_results);

        Ok(results)
    }
}

// Helper functions to convert models to DefinitionResource

#[allow(clippy::result_large_err)]
fn convert_sd_to_resource(
    sd: &StructureDefinition,
) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
    let json = serde_json::to_value(sd)
        .map_err(|e| maki_core::canonical::CanonicalLoaderError::Config(e.to_string()))?;

    Ok(Some(Arc::new(DefinitionResource {
        canonical_url: sd.url.clone(),
        resource_type: sd
            .resource_type
            .clone()
            .unwrap_or_else(|| "StructureDefinition".to_string()),
        package_id: "local".to_string(),
        version: sd.version.clone(),
        content: Arc::new(json),
    })))
}

#[allow(clippy::result_large_err)]
fn convert_vs_to_resource(vs: &ValueSet) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
    let json = serde_json::to_value(vs)
        .map_err(|e| maki_core::canonical::CanonicalLoaderError::Config(e.to_string()))?;

    Ok(Some(Arc::new(DefinitionResource {
        canonical_url: vs.url.clone(),
        resource_type: vs
            .resource_type
            .clone()
            .unwrap_or_else(|| "ValueSet".to_string()),
        package_id: "local".to_string(),
        version: vs.version.clone(),
        content: Arc::new(json),
    })))
}

#[allow(clippy::result_large_err)]
fn convert_cs_to_resource(cs: &CodeSystem) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
    let json = serde_json::to_value(cs)
        .map_err(|e| maki_core::canonical::CanonicalLoaderError::Config(e.to_string()))?;

    Ok(Some(Arc::new(DefinitionResource {
        canonical_url: cs.url.clone(),
        resource_type: cs
            .resource_type
            .clone()
            .unwrap_or_else(|| "CodeSystem".to_string()),
        package_id: "local".to_string(),
        version: cs.version.clone(),
        content: Arc::new(json),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> DefinitionSession {
        // Create a minimal test session
        // In real usage, this would be properly initialized with canonical manager
        todo!("Implement test session creation")
    }

    #[test]
    fn test_generate_id_from_name() {
        assert_eq!(generate_id_from_name("MyProfile"), "myprofile");
        assert_eq!(generate_id_from_name("US Core Patient"), "us-core-patient");
        assert_eq!(
            generate_id_from_name("Test-Profile-123"),
            "test-profile-123"
        );
        assert_eq!(generate_id_from_name("___Test___"), "test");
    }

    #[test]
    fn test_add_and_get_structure_definition() {
        let session = Arc::new(create_test_session());
        let mut lake = ResourceLake::new(session);

        let sd = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/MyProfile".to_string(),
            name: "MyProfile".to_string(),
            status: "active".to_string(),
            id: Some("myprofile".to_string()),
            title: None,
            description: None,
            base_definition: None,
            derivation: None,
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        let url = sd.url.clone();
        lake.add_structure_definition(sd).unwrap();

        let retrieved = lake.get_structure_definition(&url).unwrap();
        assert_eq!(retrieved.name, "MyProfile");
    }

    #[test]
    fn test_resolve_url_to_name() {
        let session = Arc::new(create_test_session());
        let mut lake = ResourceLake::new(session);

        let sd = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/MyProfile".to_string(),
            name: "MyProfile".to_string(),
            status: "active".to_string(),
            id: None,
            title: None,
            description: None,
            base_definition: None,
            derivation: None,
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        let url = sd.url.clone();
        lake.add_structure_definition(sd).unwrap();

        let name = lake.resolve_url_to_name(&url);
        assert_eq!(name, Some("MyProfile".to_string()));
    }

    #[test]
    fn test_lake_stats() {
        let session = Arc::new(create_test_session());
        let mut lake = ResourceLake::new(session);

        let sd = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/MyProfile".to_string(),
            name: "MyProfile".to_string(),
            status: "active".to_string(),
            id: None,
            title: None,
            description: None,
            base_definition: None,
            derivation: None,
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        lake.add_structure_definition(sd).unwrap();

        let stats = lake.stats();
        assert_eq!(stats.structure_definitions, 1);
        assert_eq!(stats.value_sets, 0);
        assert_eq!(stats.code_systems, 0);
    }
}
