//! Common FHIR types used across multiple resources

use serde::{Deserialize, Serialize};

/// Type reference in ElementDefinition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypeRef {
    pub code: String,
    pub profile: Option<Vec<String>>,
    #[serde(rename = "targetProfile")]
    pub target_profile: Option<Vec<String>>,
}

/// Value set binding
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub strength: BindingStrength,
    pub value_set: Option<String>,
    pub description: Option<String>,
}

/// Binding strength enumeration
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingStrength {
    Required,
    Extensible,
    Preferred,
    Example,
}

/// Slicing definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Slicing {
    pub discriminator: Option<Vec<Discriminator>>,
    pub description: Option<String>,
    pub ordered: Option<bool>,
    pub rules: Option<SlicingRules>,
}

/// Slicing discriminator
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Discriminator {
    #[serde(rename = "type")]
    pub type_: DiscriminatorType,
    pub path: String,
}

/// Discriminator type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscriminatorType {
    Value,
    Exists,
    Pattern,
    Type,
    Profile,
}

/// Slicing rules enumeration
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SlicingRules {
    Closed,
    Open,
    #[serde(rename = "openAtEnd")]
    OpenAtEnd,
}

/// Element constraint/invariant
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Constraint {
    pub key: String,
    pub severity: Option<String>,
    pub human: String,
    pub expression: Option<String>,
    pub xpath: Option<String>,
}

/// Example value
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub label: String,
    pub value_string: Option<String>,
    // Additional value[x] types can be added as needed
}

// FHIR complex data types

/// CodeableConcept type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodeableConcept {
    pub coding: Option<Vec<Coding>>,
    pub text: Option<String>,
}

/// Coding type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Coding {
    pub system: Option<String>,
    pub version: Option<String>,
    pub code: Option<String>,
    pub display: Option<String>,
}

/// Quantity type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Quantity {
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}

/// Identifier type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Identifier {
    pub system: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<CodeableConcept>,
}

/// Reference type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Reference {
    pub reference: Option<String>,
    pub display: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

/// ContactDetail type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactDetail {
    pub name: Option<String>,
    pub telecom: Option<Vec<ContactPoint>>,
}

/// ContactPoint type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactPoint {
    pub system: Option<String>,
    pub value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_strength_deserialization() {
        let json = r#"{
            "strength": "required",
            "valueSet": "http://example.org/ValueSet/example"
        }"#;

        let binding: Binding = serde_json::from_str(json).unwrap();
        assert_eq!(binding.strength, BindingStrength::Required);
        assert_eq!(
            binding.value_set,
            Some("http://example.org/ValueSet/example".to_string())
        );
    }

    #[test]
    fn test_discriminator_type_deserialization() {
        let json = r#"{
            "type": "value",
            "path": "code"
        }"#;

        let discriminator: Discriminator = serde_json::from_str(json).unwrap();
        assert_eq!(discriminator.type_, DiscriminatorType::Value);
        assert_eq!(discriminator.path, "code");
    }

    #[test]
    fn test_slicing_rules_deserialization() {
        assert_eq!(
            serde_json::from_str::<SlicingRules>(r#""open""#).unwrap(),
            SlicingRules::Open
        );
        assert_eq!(
            serde_json::from_str::<SlicingRules>(r#""openAtEnd""#).unwrap(),
            SlicingRules::OpenAtEnd
        );
    }
}
