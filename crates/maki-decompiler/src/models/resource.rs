//! FHIR resource enum for deserialization

use super::{CodeSystem, StructureDefinition, ValueSet};
use serde::{Deserialize, Serialize};

/// FHIR resource discriminated union
///
/// Uses Serde's tag-based deserialization to determine resource type
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "resourceType")]
pub enum FhirResource {
    StructureDefinition(StructureDefinition),
    ValueSet(ValueSet),
    CodeSystem(CodeSystem),
    #[serde(other)]
    Other,
}

impl FhirResource {
    /// Get resource type as string
    pub fn resource_type(&self) -> &'static str {
        match self {
            FhirResource::StructureDefinition(_) => "StructureDefinition",
            FhirResource::ValueSet(_) => "ValueSet",
            FhirResource::CodeSystem(_) => "CodeSystem",
            FhirResource::Other => "Other",
        }
    }

    /// Check if this is a StructureDefinition
    pub fn is_structure_definition(&self) -> bool {
        matches!(self, FhirResource::StructureDefinition(_))
    }

    /// Check if this is a ValueSet
    pub fn is_value_set(&self) -> bool {
        matches!(self, FhirResource::ValueSet(_))
    }

    /// Check if this is a CodeSystem
    pub fn is_code_system(&self) -> bool {
        matches!(self, FhirResource::CodeSystem(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_structure_definition() {
        let json = r#"{
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/Test",
            "name": "Test",
            "status": "active"
        }"#;

        let resource: FhirResource = serde_json::from_str(json).unwrap();
        assert!(resource.is_structure_definition());
        assert_eq!(resource.resource_type(), "StructureDefinition");

        match resource {
            FhirResource::StructureDefinition(sd) => {
                assert_eq!(sd.name, "Test");
            }
            _ => panic!("Expected StructureDefinition"),
        }
    }

    #[test]
    fn test_deserialize_value_set() {
        let json = r#"{
            "resourceType": "ValueSet",
            "url": "http://example.org/ValueSet/Test",
            "name": "Test",
            "status": "active"
        }"#;

        let resource: FhirResource = serde_json::from_str(json).unwrap();
        assert!(resource.is_value_set());
        assert_eq!(resource.resource_type(), "ValueSet");
    }

    #[test]
    fn test_deserialize_code_system() {
        let json = r#"{
            "resourceType": "CodeSystem",
            "url": "http://example.org/CodeSystem/Test",
            "name": "Test",
            "status": "active",
            "content": "complete"
        }"#;

        let resource: FhirResource = serde_json::from_str(json).unwrap();
        assert!(resource.is_code_system());
        assert_eq!(resource.resource_type(), "CodeSystem");
    }

    #[test]
    fn test_deserialize_other_resource() {
        let json = r#"{
            "resourceType": "Patient",
            "id": "example"
        }"#;

        let resource: FhirResource = serde_json::from_str(json).unwrap();
        assert!(!resource.is_structure_definition());
        assert!(!resource.is_value_set());
        assert!(!resource.is_code_system());
        assert_eq!(resource.resource_type(), "Other");
    }
}
