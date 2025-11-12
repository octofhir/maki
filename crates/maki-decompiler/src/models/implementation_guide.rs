//! ImplementationGuide model for FHIR IG metadata

use super::common::ContactDetail;
use serde::{Deserialize, Serialize};

/// FHIR ImplementationGuide resource
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationGuide {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>,
    pub id: Option<String>,
    pub url: String,
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
    pub package_id: Option<String>,
    pub license: Option<String>,
    pub fhir_version: Option<Vec<String>>,
    pub depends_on: Option<Vec<ImplementationGuideDependsOn>>,
    pub definition: Option<ImplementationGuideDefinition>,
}

/// ImplementationGuide dependency
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationGuideDependsOn {
    pub uri: String,
    pub package_id: Option<String>,
    pub version: Option<String>,
}

/// ImplementationGuide definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImplementationGuideDefinition {
    pub resource: Option<Vec<ImplementationGuideResource>>,
    pub page: Option<ImplementationGuidePage>,
    pub parameter: Option<Vec<ImplementationGuideParameter>>,
}

/// ImplementationGuide resource reference
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationGuideResource {
    pub reference: ImplementationGuideResourceReference,
    pub name: Option<String>,
    pub description: Option<String>,
    pub example_boolean: Option<bool>,
    pub example_canonical: Option<String>,
}

/// Resource reference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImplementationGuideResourceReference {
    pub reference: Option<String>,
}

/// ImplementationGuide page
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationGuidePage {
    pub name_url: Option<String>,
    pub title: String,
    pub generation: String,
    pub page: Option<Vec<ImplementationGuidePage>>,
}

/// ImplementationGuide parameter
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImplementationGuideParameter {
    pub code: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_implementation_guide() {
        let json = r#"{
            "resourceType": "ImplementationGuide",
            "id": "example-ig",
            "url": "http://example.org/ImplementationGuide/example",
            "name": "ExampleIG",
            "title": "Example Implementation Guide",
            "status": "active",
            "version": "1.0.0",
            "publisher": "Example Publisher",
            "packageId": "example.fhir.ig",
            "fhirVersion": ["4.0.1"]
        }"#;

        let ig: ImplementationGuide = serde_json::from_str(json).unwrap();
        assert_eq!(ig.name, "ExampleIG");
        assert_eq!(ig.version, Some("1.0.0".to_string()));
        assert_eq!(ig.package_id, Some("example.fhir.ig".to_string()));
        assert_eq!(ig.fhir_version, Some(vec!["4.0.1".to_string()]));
    }

    #[test]
    fn test_deserialize_ig_with_dependencies() {
        let json = r#"{
            "resourceType": "ImplementationGuide",
            "url": "http://example.org/ImplementationGuide/example",
            "name": "ExampleIG",
            "status": "active",
            "dependsOn": [
                {
                    "uri": "http://hl7.org/fhir/us/core/ImplementationGuide/hl7.fhir.us.core",
                    "packageId": "hl7.fhir.us.core",
                    "version": "3.1.0"
                }
            ]
        }"#;

        let ig: ImplementationGuide = serde_json::from_str(json).unwrap();
        assert!(ig.depends_on.is_some());
        let deps = ig.depends_on.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].package_id, Some("hl7.fhir.us.core".to_string()));
        assert_eq!(deps[0].version, Some("3.1.0".to_string()));
    }

    #[test]
    fn test_deserialize_ig_with_definition() {
        let json = r#"{
            "resourceType": "ImplementationGuide",
            "url": "http://example.org/ImplementationGuide/example",
            "name": "ExampleIG",
            "status": "active",
            "definition": {
                "resource": [
                    {
                        "reference": {
                            "reference": "StructureDefinition/my-patient"
                        },
                        "name": "My Patient Profile",
                        "description": "A custom patient profile",
                        "exampleBoolean": false
                    }
                ]
            }
        }"#;

        let ig: ImplementationGuide = serde_json::from_str(json).unwrap();
        assert!(ig.definition.is_some());
        let def = ig.definition.unwrap();
        assert!(def.resource.is_some());
        let resources = def.resource.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(
            resources[0].reference.reference,
            Some("StructureDefinition/my-patient".to_string())
        );
    }
}
