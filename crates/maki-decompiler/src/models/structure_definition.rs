//! StructureDefinition model - represents FHIR profiles, extensions, logical models, and resources

use serde::{Deserialize, Serialize};
use super::element_definition::ElementList;
use super::common::ContactDetail;

/// FHIR StructureDefinition resource
///
/// Maps to Profile, Extension, Logical Model, or Resource in FSH
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<String>, // Optional for tagged enum deserialization
    pub id: Option<String>,
    pub url: String,
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub base_definition: Option<String>,
    pub derivation: Option<Derivation>,
    pub kind: Option<StructureDefinitionKind>,
    #[serde(rename = "abstract")]
    pub abstract_: Option<bool>,
    pub context: Option<Vec<ContextDefinition>>,
    pub differential: Option<ElementList>,
    pub snapshot: Option<ElementList>,

    // Additional metadata
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
    pub copyright: Option<String>,
}

/// Derivation type (constraint or specialization)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Derivation {
    Constraint,      // Profile or Extension
    Specialization,  // Logical Model or Resource
}

/// StructureDefinition kind
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructureDefinitionKind {
    Resource,
    ComplexType,
    PrimitiveType,
    Logical,
}

/// Context definition for extensions
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDefinition {
    #[serde(rename = "type")]
    pub type_: String,
    pub expression: String,
}

impl StructureDefinition {
    /// Determine if this is a Profile
    pub fn is_profile(&self) -> bool {
        matches!(self.derivation, Some(Derivation::Constraint)) && !self.is_extension()
    }

    /// Determine if this is an Extension
    pub fn is_extension(&self) -> bool {
        self.base_definition
            .as_ref()
            .map(|url| url.contains("/Extension"))
            .unwrap_or(false)
    }

    /// Determine if this is a Logical Model
    pub fn is_logical(&self) -> bool {
        matches!(self.kind, Some(StructureDefinitionKind::Logical))
            && matches!(self.derivation, Some(Derivation::Specialization))
    }

    /// Determine if this is a Resource definition
    pub fn is_resource(&self) -> bool {
        matches!(self.kind, Some(StructureDefinitionKind::Resource))
            && matches!(self.derivation, Some(Derivation::Specialization))
    }

    /// Get FSH type name (Profile, Extension, Logical, Resource)
    pub fn fsh_type(&self) -> &'static str {
        if self.is_extension() {
            "Extension"
        } else if self.is_logical() {
            "Logical"
        } else if self.is_resource() {
            "Resource"
        } else {
            "Profile"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_structure_definition() {
        let json = r#"{
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/MyProfile",
            "name": "MyProfile",
            "status": "active",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Patient",
            "derivation": "constraint"
        }"#;

        let sd: StructureDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(sd.name, "MyProfile");
        assert_eq!(sd.derivation, Some(Derivation::Constraint));
        assert!(sd.is_profile());
        assert!(!sd.is_extension());
    }

    #[test]
    fn test_is_extension() {
        let sd = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/MyExtension".to_string(),
            name: "MyExtension".to_string(),
            status: "active".to_string(),
            base_definition: Some(
                "http://hl7.org/fhir/StructureDefinition/Extension".to_string(),
            ),
            derivation: Some(Derivation::Constraint),
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            id: None,
            title: None,
            description: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        assert!(sd.is_extension());
        assert!(!sd.is_profile());
        assert_eq!(sd.fsh_type(), "Extension");
    }

    #[test]
    fn test_is_logical() {
        let sd = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/MyLogical".to_string(),
            name: "MyLogical".to_string(),
            status: "active".to_string(),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Element".to_string()),
            derivation: Some(Derivation::Specialization),
            kind: Some(StructureDefinitionKind::Logical),
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            id: None,
            title: None,
            description: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        assert!(sd.is_logical());
        assert!(!sd.is_profile());
        assert!(!sd.is_extension());
        assert_eq!(sd.fsh_type(), "Logical");
    }

    #[test]
    fn test_fsh_type() {
        // Test Profile
        let profile = StructureDefinition {
            resource_type: Some("StructureDefinition".to_string()),
            url: "http://example.org/StructureDefinition/Test".to_string(),
            name: "Test".to_string(),
            status: "active".to_string(),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
            derivation: Some(Derivation::Constraint),
            kind: None,
            abstract_: None,
            context: None,
            differential: None,
            snapshot: None,
            id: None,
            title: None,
            description: None,
            version: None,
            publisher: None,
            contact: None,
            copyright: None,
        };

        assert_eq!(profile.fsh_type(), "Profile");
    }
}
