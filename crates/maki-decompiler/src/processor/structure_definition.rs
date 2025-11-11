//! StructureDefinition processor
//!
//! Converts FHIR StructureDefinitions into Exportable objects (Profile, Extension, Logical, Resource)

use crate::{
    models::{Derivation, StructureDefinition, StructureDefinitionKind},
    exportable::*,
    lake::ResourceLake,
    Error, Result,
};
use log::{debug, warn};
use maki_core::canonical::fishable::Fishable;

/// Type of definition being processed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionType {
    Profile,
    Extension,
    Logical,
    Resource,
}

/// StructureDefinition processor
pub struct StructureDefinitionProcessor<'a> {
    lake: &'a ResourceLake,
}

impl<'a> StructureDefinitionProcessor<'a> {
    /// Create a new StructureDefinition processor
    pub fn new(lake: &'a ResourceLake) -> Self {
        Self { lake }
    }

    /// Process a StructureDefinition into an Exportable
    pub async fn process(&self, sd: &StructureDefinition) -> Result<Box<dyn Exportable>> {
        let def_type = self.determine_type(sd)?;

        debug!(
            "Processing StructureDefinition '{}' as {:?}",
            sd.name, def_type
        );

        match def_type {
            DefinitionType::Profile => {
                let profile = self.process_profile(sd).await?;
                Ok(Box::new(profile) as Box<dyn Exportable>)
            }
            DefinitionType::Extension => {
                let extension = self.process_extension(sd).await?;
                Ok(Box::new(extension) as Box<dyn Exportable>)
            }
            DefinitionType::Logical => {
                let logical = self.process_logical(sd).await?;
                Ok(Box::new(logical) as Box<dyn Exportable>)
            }
            DefinitionType::Resource => {
                let resource = self.process_resource(sd).await?;
                Ok(Box::new(resource) as Box<dyn Exportable>)
            }
        }
    }

    /// Determine the type of StructureDefinition
    pub fn determine_type(&self, sd: &StructureDefinition) -> Result<DefinitionType> {
        // Extension: baseDefinition = .../Extension
        if let Some(base) = &sd.base_definition {
            if base.contains("/Extension") {
                return Ok(DefinitionType::Extension);
            }
        }

        // Logical: kind=logical
        if matches!(sd.kind, Some(StructureDefinitionKind::Logical)) {
            return Ok(DefinitionType::Logical);
        }

        // Resource: kind=resource AND derivation=specialization
        if matches!(sd.kind, Some(StructureDefinitionKind::Resource))
            && matches!(sd.derivation, Some(Derivation::Specialization))
        {
            return Ok(DefinitionType::Resource);
        }

        // Default: Profile
        Ok(DefinitionType::Profile)
    }

    /// Process a Profile
    async fn process_profile(&self, sd: &StructureDefinition) -> Result<ExportableProfile> {
        let parent = self.resolve_parent(sd).await?;

        let mut profile = ExportableProfile::new(sd.name.clone(), parent);

        // Set optional fields
        if let Some(id) = &sd.id {
            profile.id = Some(id.clone());
        }

        if let Some(title) = &sd.title {
            profile.title = Some(title.clone());
        }

        if let Some(desc) = &sd.description {
            profile.description = Some(desc.clone());
        }

        // Note: Rules will be extracted in Phase 3 (Tasks 09-12)
        // For now, we just create the profile structure

        debug!(
            "Created ExportableProfile '{}' with parent '{}'",
            profile.name(),
            &profile.parent
        );

        Ok(profile)
    }

    /// Process an Extension
    async fn process_extension(&self, sd: &StructureDefinition) -> Result<ExportableExtension> {
        let mut extension = ExportableExtension::new(sd.name.clone());

        // Set optional fields
        if let Some(id) = &sd.id {
            extension.id = Some(id.clone());
        }

        if let Some(title) = &sd.title {
            extension.title = Some(title.clone());
        }

        if let Some(desc) = &sd.description {
            extension.description = Some(desc.clone());
        }

        // Extract contexts
        if let Some(contexts) = &sd.context {
            for ctx in contexts {
                // Parse context type
                let context_type = match ctx.type_.as_str() {
                    "element" => ContextType::Element,
                    "extension" => ContextType::Extension,
                    "fhirpath" => ContextType::Fhirpath,
                    _ => {
                        warn!("Unknown context type: {}", ctx.type_);
                        ContextType::Element
                    }
                };

                extension.add_context(Context {
                    type_: context_type,
                    expression: ctx.expression.clone(),
                });
            }
        }

        // Note: Rules will be extracted in Phase 3
        debug!("Created ExportableExtension '{}'", extension.name());

        Ok(extension)
    }

    /// Process a Logical Model
    async fn process_logical(&self, sd: &StructureDefinition) -> Result<ExportableLogical> {
        let mut logical = ExportableLogical::new(sd.name.clone());

        // Set optional fields
        if let Some(id) = &sd.id {
            logical.id = Some(id.clone());
        }

        // Parent is optional for logical models
        if let Some(base) = &sd.base_definition {
            if !base.contains("/Element") && !base.contains("/Base") {
                let parent = self.resolve_parent(sd).await?;
                logical.parent = Some(parent);
            }
        }

        if let Some(title) = &sd.title {
            logical.title = Some(title.clone());
        }

        if let Some(desc) = &sd.description {
            logical.description = Some(desc.clone());
        }

        // Note: Characteristics extraction will be added in Phase 3
        // Note: Rules will be extracted in Phase 3

        debug!("Created ExportableLogical '{}'", logical.name());

        Ok(logical)
    }

    /// Process a Resource
    async fn process_resource(&self, sd: &StructureDefinition) -> Result<ExportableResource> {
        let mut resource = ExportableResource::new(sd.name.clone());

        // Set optional fields
        if let Some(id) = &sd.id {
            resource.id = Some(id.clone());
        }

        // Parent is optional for resources
        if let Some(base) = &sd.base_definition {
            if !base.contains("/Resource") && !base.contains("/DomainResource") {
                let parent = self.resolve_parent(sd).await?;
                resource.parent = Some(parent);
            } else {
                // Extract just the resource name from URL
                if let Some(name) = base.split('/').last() {
                    resource.parent = Some(name.to_string());
                }
            }
        }

        if let Some(title) = &sd.title {
            resource.title = Some(title.clone());
        }

        if let Some(desc) = &sd.description {
            resource.description = Some(desc.clone());
        }

        // Note: Rules will be extracted in Phase 3

        debug!("Created ExportableResource '{}'", resource.name());

        Ok(resource)
    }

    /// Resolve parent name using Fishable
    async fn resolve_parent(&self, sd: &StructureDefinition) -> Result<String> {
        let parent_url = sd
            .base_definition
            .as_ref()
            .ok_or_else(|| Error::MissingBaseDefinition(sd.name.clone()))?;

        debug!("Resolving parent for '{}': {}", sd.name, parent_url);

        // Try to fish the parent from the lake
        match self.lake.fish_by_url(parent_url).await {
            Ok(Some(parent_resource)) => {
                // Parse the parent resource to extract the name
                // The content is a serde_json::Value, convert it to StructureDefinition
                if let Ok(parent_sd) = serde_json::from_value::<StructureDefinition>((*parent_resource.content).clone()) {
                    debug!("Found parent in lake: {}", parent_sd.name);
                    Ok(parent_sd.name)
                } else {
                    // If parsing fails, extract name from URL
                    let name = parent_url
                        .split('/')
                        .last()
                        .unwrap_or(parent_url)
                        .to_string();
                    debug!("Could not parse parent, using URL-derived name: {}", name);
                    Ok(name)
                }
            }
            Ok(None) => {
                // Parent not found, extract name from URL
                let name = parent_url
                    .split('/')
                    .last()
                    .unwrap_or(parent_url)
                    .to_string();

                debug!(
                    "Parent not found in lake, using URL-derived name: {}",
                    name
                );
                Ok(name)
            }
            Err(e) => {
                warn!(
                    "Error fishing parent for '{}': {}. Using URL-derived name",
                    sd.name, e
                );

                // Fallback to URL-derived name
                let name = parent_url
                    .split('/')
                    .last()
                    .unwrap_or(parent_url)
                    .to_string();
                Ok(name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ContextDefinition, StructureDefinition};
    use std::sync::Arc;

    async fn create_test_lake() -> ResourceLake {
        // Create a test lake without requiring real canonical manager
        // For testing purposes, we use a minimal session
        // In integration tests, a real session would be used
        use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};

        let options = CanonicalOptions {
            quick_init: true,  // Fast initialization for tests
            auto_install_core: false,  // Don't install packages in tests
            ..Default::default()
        };

        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session(vec![FhirRelease::R4]).await.unwrap();

        ResourceLake::new(Arc::new(session))
    }

    fn create_test_profile() -> StructureDefinition {
        StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            id: Some("my-patient".to_string()),
            url: "http://example.org/StructureDefinition/MyPatient".to_string(),
            name: "MyPatient".to_string(),
            title: Some("My Patient Profile".to_string()),
            status: "active".to_string(),
            description: Some("A custom patient profile".to_string()),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
            derivation: Some(Derivation::Constraint),
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        }
    }

    fn create_test_extension() -> StructureDefinition {
        StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            id: Some("my-extension".to_string()),
            url: "http://example.org/StructureDefinition/MyExtension".to_string(),
            name: "MyExtension".to_string(),
            title: Some("My Extension".to_string()),
            status: "active".to_string(),
            description: Some("A custom extension".to_string()),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Extension".to_string()),
            derivation: Some(Derivation::Constraint),
            kind: None,
            abstract_: None,
            context: Some(vec![ContextDefinition {
                type_: "element".to_string(),
                expression: "Patient".to_string(),
            }]),
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        }
    }

    fn create_test_logical() -> StructureDefinition {
        StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            id: Some("my-model".to_string()),
            url: "http://example.org/StructureDefinition/MyModel".to_string(),
            name: "MyModel".to_string(),
            title: Some("My Logical Model".to_string()),
            status: "active".to_string(),
            description: Some("A custom logical model".to_string()),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Element".to_string()),
            derivation: Some(Derivation::Specialization),
            kind: Some(StructureDefinitionKind::Logical),
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        }
    }

    #[tokio::test]
    async fn test_determine_type_profile() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_profile();
        let def_type = processor.determine_type(&sd).unwrap();

        assert_eq!(def_type, DefinitionType::Profile);
    }

    #[tokio::test]
    async fn test_determine_type_extension() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_extension();
        let def_type = processor.determine_type(&sd).unwrap();

        assert_eq!(def_type, DefinitionType::Extension);
    }

    #[tokio::test]
    async fn test_determine_type_logical() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_logical();
        let def_type = processor.determine_type(&sd).unwrap();

        assert_eq!(def_type, DefinitionType::Logical);
    }

    #[tokio::test]
    async fn test_process_profile() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_profile();
        let profile = processor.process_profile(&sd).await.unwrap();

        assert_eq!(profile.name(), "MyPatient");
        assert_eq!(profile.parent, "Patient");
        assert_eq!(profile.id, Some("my-patient".to_string()));
        assert_eq!(profile.title, Some("My Patient Profile".to_string()));
    }

    #[tokio::test]
    async fn test_process_extension() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_extension();
        let extension = processor.process_extension(&sd).await.unwrap();

        assert_eq!(extension.name(), "MyExtension");
        assert_eq!(extension.id, Some("my-extension".to_string()));
        assert_eq!(extension.contexts.len(), 1);
        assert_eq!(extension.contexts[0].type_, ContextType::Element);
        assert_eq!(extension.contexts[0].expression, "Patient");
    }

    #[tokio::test]
    async fn test_process_logical() {
        let lake = create_test_lake().await;
        let processor = StructureDefinitionProcessor::new(&lake);

        let sd = create_test_logical();
        let logical = processor.process_logical(&sd).await.unwrap();

        assert_eq!(logical.name(), "MyModel");
        assert_eq!(logical.id, Some("my-model".to_string()));
        assert_eq!(logical.parent, None); // Element is skipped
    }
}
