//! FHIR Type Definitions for Export
//!
//! This module contains simplified FHIR type definitions used for exporting
//! FSH resources to FHIR JSON. These types are focused on the fields needed
//! for profile export and differential generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// StructureDefinition
// ============================================================================

/// FHIR StructureDefinition resource
///
/// Represents a FHIR Profile, Extension, or Logical model.
/// This is a simplified version containing the fields most commonly used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinition {
    /// Resource type (always "StructureDefinition")
    pub resource_type: String,

    /// Logical id of this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Canonical identifier for this structure definition
    pub url: String,

    /// Business version of the structure definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Name for this structure definition (computer friendly)
    pub name: String,

    /// Name for this structure definition (human friendly)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// draft | active | retired | unknown
    pub status: String,

    /// Date last changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,

    /// Name of the publisher
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    /// Natural language description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// For testing purposes, not real usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,

    /// FHIR Version this StructureDefinition targets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fhir_version: Option<String>,

    /// primitive-type | complex-type | resource | logical
    pub kind: StructureDefinitionKind,

    /// Whether the structure is abstract
    #[serde(rename = "abstract")]
    pub is_abstract: bool,

    /// Type defined or constrained by this structure
    #[serde(rename = "type")]
    pub type_field: String,

    /// Definition that this type is constrained/specialized from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_definition: Option<String>,

    /// specialization | constraint - How this type relates to baseDefinition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation: Option<String>,

    /// Snapshot view of the structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<StructureDefinitionSnapshot>,

    /// Differential view of the structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differential: Option<StructureDefinitionDifferential>,
}

impl StructureDefinition {
    /// Create a new StructureDefinition with required fields
    pub fn new(url: String, name: String, type_field: String, kind: StructureDefinitionKind) -> Self {
        Self {
            resource_type: "StructureDefinition".to_string(),
            id: None,
            url,
            version: None,
            name,
            title: None,
            status: "draft".to_string(),
            date: None,
            publisher: None,
            description: None,
            experimental: None,
            fhir_version: None,
            kind,
            is_abstract: false,
            type_field,
            base_definition: None,
            derivation: Some("constraint".to_string()),
            snapshot: None,
            differential: None,
        }
    }

    /// Find an element by path in snapshot
    pub fn find_element(&self, path: &str) -> Option<&ElementDefinition> {
        self.snapshot
            .as_ref()?
            .element
            .iter()
            .find(|e| e.path == path)
    }

    /// Find an element by path in snapshot (mutable)
    pub fn find_element_mut(&mut self, path: &str) -> Option<&mut ElementDefinition> {
        self.snapshot
            .as_mut()?
            .element
            .iter_mut()
            .find(|e| e.path == path)
    }

    /// Get or create snapshot
    pub fn get_or_create_snapshot(&mut self) -> &mut StructureDefinitionSnapshot {
        if self.snapshot.is_none() {
            self.snapshot = Some(StructureDefinitionSnapshot {
                element: Vec::new(),
            });
        }
        self.snapshot.as_mut().unwrap()
    }
}

/// Kind of structure definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructureDefinitionKind {
    PrimitiveType,
    ComplexType,
    Resource,
    Logical,
}

/// Snapshot view of structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureDefinitionSnapshot {
    pub element: Vec<ElementDefinition>,
}

/// Differential view of structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureDefinitionDifferential {
    pub element: Vec<ElementDefinition>,
}

// ============================================================================
// ElementDefinition
// ============================================================================

/// Definition of an element in a resource or data type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinition {
    /// Path of the element in the hierarchy of elements
    pub path: String,

    /// Minimum Cardinality
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,

    /// Maximum Cardinality ("*" for unbounded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,

    /// Data type(s) for this element
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_: Option<Vec<ElementDefinitionType>>,

    /// Short description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,

    /// Full formal definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,

    /// Comments about the use of the element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,

    /// Include when support is essential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_support: Option<bool>,

    /// If the element must be supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_modifier: Option<bool>,

    /// Include in summaries
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_summary: Option<bool>,

    /// ValueSet binding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<ElementDefinitionBinding>,

    /// Condition that must evaluate to true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraint: Option<Vec<ElementDefinitionConstraint>>,

    /// Fixed value
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub fixed: Option<HashMap<String, serde_json::Value>>,

    /// Pattern value
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub pattern: Option<HashMap<String, serde_json::Value>>,
}

impl ElementDefinition {
    /// Create a new ElementDefinition with just a path
    pub fn new(path: String) -> Self {
        Self {
            path,
            min: None,
            max: None,
            type_: None,
            short: None,
            definition: None,
            comment: None,
            must_support: None,
            is_modifier: None,
            is_summary: None,
            binding: None,
            constraint: None,
            fixed: None,
            pattern: None,
        }
    }

    /// Check if this element has been modified from defaults
    pub fn has_modifications(&self) -> bool {
        self.min.is_some()
            || self.max.is_some()
            || self.type_.is_some()
            || self.short.is_some()
            || self.definition.is_some()
            || self.comment.is_some()
            || self.must_support.is_some()
            || self.is_modifier.is_some()
            || self.is_summary.is_some()
            || self.binding.is_some()
            || self.constraint.is_some()
            || self.fixed.is_some()
            || self.pattern.is_some()
    }

    /// Compare with another element to check if modified
    pub fn is_modified_from(&self, base: &ElementDefinition) -> bool {
        self.min != base.min
            || self.max != base.max
            || self.type_ != base.type_
            || self.short != base.short
            || self.definition != base.definition
            || self.comment != base.comment
            || self.must_support != base.must_support
            || self.is_modifier != base.is_modifier
            || self.is_summary != base.is_summary
            || self.binding != base.binding
            || self.constraint != base.constraint
            || self.fixed != base.fixed
            || self.pattern != base.pattern
    }
}

/// Data type for an element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinitionType {
    /// Data type or Resource (reference target)
    pub code: String,

    /// Profile (StructureDefinition or IG) on type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Vec<String>>,

    /// Profile (StructureDefinition or IG) for target resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_profile: Option<Vec<String>>,
}

impl ElementDefinitionType {
    /// Create a simple type with just a code
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            profile: None,
            target_profile: None,
        }
    }
}

/// ValueSet binding for an element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinitionBinding {
    /// required | extensible | preferred | example
    pub strength: BindingStrength,

    /// Description of the binding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Source of value set (canonical URL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_set: Option<String>,
}

/// Binding strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindingStrength {
    Required,
    Extensible,
    Preferred,
    Example,
}

impl BindingStrength {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "required" => Some(Self::Required),
            "extensible" => Some(Self::Extensible),
            "preferred" => Some(Self::Preferred),
            "example" => Some(Self::Example),
            _ => None,
        }
    }
}

/// Constraint on an element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinitionConstraint {
    /// Target of 'condition' reference
    pub key: String,

    /// error | warning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,

    /// Human description of constraint
    pub human: String,

    /// FHIRPath expression of constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_definition_new() {
        let sd = StructureDefinition::new(
            "http://example.org/fhir/StructureDefinition/TestProfile".to_string(),
            "TestProfile".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        );

        assert_eq!(sd.resource_type, "StructureDefinition");
        assert_eq!(sd.url, "http://example.org/fhir/StructureDefinition/TestProfile");
        assert_eq!(sd.name, "TestProfile");
        assert_eq!(sd.type_field, "Patient");
        assert_eq!(sd.kind, StructureDefinitionKind::Resource);
        assert_eq!(sd.status, "draft");
    }

    #[test]
    fn test_element_definition_new() {
        let elem = ElementDefinition::new("Patient.name".to_string());
        assert_eq!(elem.path, "Patient.name");
        assert!(!elem.has_modifications());
    }

    #[test]
    fn test_element_definition_modifications() {
        let mut elem = ElementDefinition::new("Patient.name".to_string());
        assert!(!elem.has_modifications());

        elem.min = Some(1);
        assert!(elem.has_modifications());
    }

    #[test]
    fn test_binding_strength_from_str() {
        assert_eq!(BindingStrength::from_str("required"), Some(BindingStrength::Required));
        assert_eq!(BindingStrength::from_str("REQUIRED"), Some(BindingStrength::Required));
        assert_eq!(BindingStrength::from_str("extensible"), Some(BindingStrength::Extensible));
        assert_eq!(BindingStrength::from_str("invalid"), None);
    }

    #[test]
    fn test_element_definition_type() {
        let type_def = ElementDefinitionType::new("string");
        assert_eq!(type_def.code, "string");
        assert!(type_def.profile.is_none());
    }
}
