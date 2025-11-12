//! Instance processor
//!
//! Converts generic FHIR resources into ExportableInstance objects

use crate::{
    Result,
    exportable::{Exportable, ExportableInstance, FshValue, InstanceUsage},
    lake::ResourceLake,
};
use log::{debug, warn};
use serde_json::Value;

/// Instance processor for generic FHIR resources
pub struct InstanceProcessor<'a> {
    _lake: &'a ResourceLake,
}

impl<'a> InstanceProcessor<'a> {
    /// Create a new Instance processor
    pub fn new(lake: &'a ResourceLake) -> Self {
        Self { _lake: lake }
    }

    /// Process a generic FHIR resource into an ExportableInstance
    pub fn process(&self, resource: &Value) -> Result<ExportableInstance> {
        // Extract resourceType and id
        let resource_type = resource
            .get("resourceType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::Error::Other(anyhow::anyhow!("Missing resourceType in resource"))
            })?
            .to_string();

        let id = resource
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        debug!(
            "Processing instance of type '{}' with id '{:?}'",
            resource_type, id
        );

        // Determine instance name (use id or generate from resourceType)
        let name = id
            .clone()
            .unwrap_or_else(|| format!("{}Instance", resource_type));

        let mut instance = ExportableInstance::new(name, resource_type.clone());

        // Set usage (default to Example for now, can be refined later)
        instance.usage = Some(InstanceUsage::Example);

        // Extract title from resource if available
        if let Some(title) = resource.get("title").and_then(|v| v.as_str()) {
            instance.title = Some(title.to_string());
        }

        // Extract description if available
        if let Some(desc) = resource.get("description").and_then(|v| v.as_str()) {
            instance.description = Some(desc.to_string());
        }

        // Process assignments (convert JSON values to rules)
        // Note: Full assignment extraction will be implemented in Phase 3
        // For now, we just track that we've seen the resource

        debug!(
            "Created ExportableInstance '{}' of type '{}'",
            instance.name(),
            instance.instance_of
        );

        Ok(instance)
    }

    /// Extract assignments from JSON value (simplified version)
    /// Full implementation will be in Phase 3 (Task 11)
    #[allow(dead_code)]
    fn extract_assignments(&self, _resource: &Value, _instance: &mut ExportableInstance) {
        // TODO: Implement in Phase 3
        // This will recursively walk the JSON and create AssignmentRule for each field
        warn!("Assignment extraction not yet implemented");
    }

    /// Convert a JSON value to FshValue
    #[allow(dead_code)]
    fn json_to_fsh_value(&self, value: &Value) -> Option<FshValue> {
        match value {
            Value::Bool(b) => Some(FshValue::Boolean(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(FshValue::Integer(i as i32))
                } else {
                    n.as_f64().map(FshValue::Decimal)
                }
            }
            Value::String(s) => Some(FshValue::String(s.clone())),
            // Complex types would be handled here
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_process_patient_instance() {
        let lake = create_test_lake().await;
        let processor = InstanceProcessor::new(&lake);

        let json = serde_json::json!({
            "resourceType": "Patient",
            "id": "example-patient",
            "active": true,
            "name": [{
                "family": "Smith",
                "given": ["John"]
            }]
        });

        let instance = processor.process(&json).unwrap();

        assert_eq!(instance.name(), "example-patient");
        assert_eq!(instance.instance_of, "Patient");
    }

    #[tokio::test]
    async fn test_process_observation_instance() {
        let lake = create_test_lake().await;
        let processor = InstanceProcessor::new(&lake);

        let json = serde_json::json!({
            "resourceType": "Observation",
            "id": "example-obs",
            "status": "final",
            "code": {
                "coding": [{
                    "system": "http://loinc.org",
                    "code": "15074-8",
                    "display": "Glucose"
                }]
            }
        });

        let instance = processor.process(&json).unwrap();

        assert_eq!(instance.name(), "example-obs");
        assert_eq!(instance.instance_of, "Observation");
    }

    #[tokio::test]
    async fn test_process_instance_without_id() {
        let lake = create_test_lake().await;
        let processor = InstanceProcessor::new(&lake);

        let json = serde_json::json!({
            "resourceType": "Organization",
            "name": "Example Org"
        });

        let instance = processor.process(&json).unwrap();

        assert_eq!(instance.name(), "OrganizationInstance");
        assert_eq!(instance.instance_of, "Organization");
    }

    #[tokio::test]
    async fn test_process_instance_with_title() {
        let lake = create_test_lake().await;
        let processor = InstanceProcessor::new(&lake);

        let json = serde_json::json!({
            "resourceType": "StructureDefinition",
            "id": "my-profile",
            "title": "My Custom Profile",
            "description": "A custom profile for testing"
        });

        let instance = processor.process(&json).unwrap();

        assert_eq!(instance.name(), "my-profile");
        assert_eq!(instance.title, Some("My Custom Profile".to_string()));
        assert_eq!(
            instance.description,
            Some("A custom profile for testing".to_string())
        );
    }

    #[tokio::test]
    async fn test_process_instance_missing_resource_type() {
        let lake = create_test_lake().await;
        let processor = InstanceProcessor::new(&lake);

        let json = serde_json::json!({
            "id": "example"
        });

        let result = processor.process(&json);
        assert!(result.is_err());
    }
}
