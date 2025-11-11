//! Config processor
//!
//! Extracts sushi-config.yaml from ImplementationGuide resources

use crate::{
    models::ImplementationGuide,
    exportable::{ExportableConfiguration, FhirDependency, ConfigParameter},
    lake::ResourceLake,
    Result,
};
use log::debug;

/// Config processor for ImplementationGuide resources
pub struct ConfigProcessor<'a> {
    lake: &'a ResourceLake,
}

impl<'a> ConfigProcessor<'a> {
    /// Create a new Config processor
    pub fn new(lake: &'a ResourceLake) -> Self {
        Self { lake }
    }

    /// Extract sushi-config from an ImplementationGuide resource
    pub fn process(&self, ig: &ImplementationGuide) -> Result<ExportableConfiguration> {
        debug!("Processing ImplementationGuide '{}' to config", ig.name);

        let mut config = ExportableConfiguration::new(
            ig.package_id.clone().unwrap_or_else(|| ig.name.clone()),
            ig.url.clone(),
            ig.name.clone(),
            ig.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        );

        // Set optional fields
        config.title = ig.title.clone();
        config.description = ig.description.clone();
        config.status = ig.status.clone();

        // Set FHIR version
        if let Some(fhir_versions) = &ig.fhir_version {
            config.fhir_version = fhir_versions.clone();
        }

        // Extract dependencies
        if let Some(depends_on) = &ig.depends_on {
            for dep in depends_on {
                let package_id = dep
                    .package_id
                    .clone()
                    .unwrap_or_else(|| extract_package_id_from_uri(&dep.uri));

                let version = dep.version.clone().unwrap_or_else(|| "latest".to_string());

                config.dependencies.push(FhirDependency {
                    package_id,
                    version,
                });
            }
        }

        // Extract parameters from definition
        if let Some(definition) = &ig.definition {
            if let Some(parameters) = &definition.parameter {
                for p in parameters {
                    config.parameters.push(ConfigParameter {
                        code: p.code.clone(),
                        value: p.value.clone(),
                    });
                }
            }
        }

        debug!("Created ExportableConfiguration for '{}'", config.name);

        Ok(config)
    }
}

/// Extract package ID from IG URI
fn extract_package_id_from_uri(uri: &str) -> String {
    // Try to extract from URI like:
    // http://hl7.org/fhir/us/core/ImplementationGuide/hl7.fhir.us.core
    if let Some(package_id) = uri.split('/').last() {
        if package_id.contains('.') {
            return package_id.to_string();
        }
    }

    // Fallback: use the URI itself
    uri.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ImplementationGuideDependsOn, ContactDetail};
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
    use std::sync::Arc;

    async fn create_test_lake() -> ResourceLake {
        let options = CanonicalOptions {
            quick_init: true,
            auto_install_core: false,
            ..Default::default()
        };

        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session(vec![FhirRelease::R4]).await.unwrap();

        ResourceLake::new(Arc::new(session))
    }

    fn create_test_ig() -> ImplementationGuide {
        ImplementationGuide {
            resource_type: Some("ImplementationGuide".to_string()),
            id: Some("example-ig".to_string()),
            url: "http://example.org/ImplementationGuide/example".to_string(),
            name: "ExampleIG".to_string(),
            title: Some("Example Implementation Guide".to_string()),
            status: "active".to_string(),
            description: Some("An example IG for testing".to_string()),
            version: Some("1.0.0".to_string()),
            publisher: Some("Example Publisher".to_string()),
            contact: Some(vec![ContactDetail {
                name: Some("Example Contact".to_string()),
                telecom: None,
            }]),
            package_id: Some("example.fhir.ig".to_string()),
            license: None,
            fhir_version: Some(vec!["4.0.1".to_string()]),
            depends_on: None,
            definition: None,
        }
    }

    #[tokio::test]
    async fn test_process_basic_ig() {
        let lake = create_test_lake().await;
        let processor = ConfigProcessor::new(&lake);

        let ig = create_test_ig();
        let config = processor.process(&ig).unwrap();

        assert_eq!(
            config.canonical,
            "http://example.org/ImplementationGuide/example"
        );
        assert_eq!(config.name, "ExampleIG");
        assert_eq!(config.id, "example.fhir.ig");
        assert_eq!(
            config.title,
            Some("Example Implementation Guide".to_string())
        );
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.fhir_version, vec!["4.0.1".to_string()]);
    }

    #[tokio::test]
    async fn test_process_ig_with_dependencies() {
        let lake = create_test_lake().await;
        let processor = ConfigProcessor::new(&lake);

        let mut ig = create_test_ig();
        ig.depends_on = Some(vec![
            ImplementationGuideDependsOn {
                uri: "http://hl7.org/fhir/us/core/ImplementationGuide/hl7.fhir.us.core"
                    .to_string(),
                package_id: Some("hl7.fhir.us.core".to_string()),
                version: Some("3.1.0".to_string()),
            },
            ImplementationGuideDependsOn {
                uri: "http://hl7.org/fhir/uv/ips/ImplementationGuide/hl7.fhir.uv.ips".to_string(),
                package_id: Some("hl7.fhir.uv.ips".to_string()),
                version: Some("1.0.0".to_string()),
            },
        ]);

        let config = processor.process(&ig).unwrap();

        assert_eq!(config.dependencies.len(), 2);
        assert_eq!(config.dependencies[0].package_id, "hl7.fhir.us.core");
        assert_eq!(config.dependencies[0].version, "3.1.0");
        assert_eq!(config.dependencies[1].package_id, "hl7.fhir.uv.ips");
        assert_eq!(config.dependencies[1].version, "1.0.0");
    }

    #[tokio::test]
    async fn test_process_ig_without_package_id() {
        let lake = create_test_lake().await;
        let processor = ConfigProcessor::new(&lake);

        let mut ig = create_test_ig();
        ig.package_id = None;

        let config = processor.process(&ig).unwrap();

        // Should use name as fallback
        assert_eq!(config.id, "ExampleIG");
    }

    #[tokio::test]
    async fn test_extract_package_id_from_uri() {
        assert_eq!(
            extract_package_id_from_uri(
                "http://hl7.org/fhir/us/core/ImplementationGuide/hl7.fhir.us.core"
            ),
            "hl7.fhir.us.core"
        );

        assert_eq!(
            extract_package_id_from_uri("http://example.org/ImplementationGuide/simple"),
            "http://example.org/ImplementationGuide/simple"
        );
    }
}
