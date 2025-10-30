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

    /// Extension context (for extensions only) - where can this extension be used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<StructureDefinitionContext>>,

    /// Snapshot view of the structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<StructureDefinitionSnapshot>,

    /// Differential view of the structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub differential: Option<StructureDefinitionDifferential>,
}

impl StructureDefinition {
    /// Create a new StructureDefinition with required fields
    pub fn new(
        url: String,
        name: String,
        type_field: String,
        kind: StructureDefinitionKind,
    ) -> Self {
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
            context: None,
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

// ============================================================================
// StructureDefinition Context (for Extensions)
// ============================================================================

/// Context where an extension can be used
///
/// See: <https://www.hl7.org/fhir/extensibility.html#context>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinitionContext {
    /// element | extension | fhirpath
    #[serde(rename = "type")]
    pub type_: String,

    /// Where the extension can be used (e.g., "Patient", "Observation.status")
    pub expression: String,
}

impl StructureDefinitionContext {
    /// Create a new extension context
    pub fn new(type_: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            expression: expression.into(),
        }
    }

    /// Create an element context
    pub fn element(expression: impl Into<String>) -> Self {
        Self::new("element", expression)
    }

    /// Create an extension context
    pub fn extension(expression: impl Into<String>) -> Self {
        Self::new("extension", expression)
    }

    /// Create a fhirpath context
    pub fn fhirpath(expression: impl Into<String>) -> Self {
        Self::new("fhirpath", expression)
    }
}

// ============================================================================
// ValueSet
// ============================================================================

/// FHIR ValueSet resource
///
/// Represents a set of codes drawn from one or more code systems.
/// See: <https://www.hl7.org/fhir/valueset.html>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetResource {
    /// Resource type (always "ValueSet")
    pub resource_type: String,

    /// Logical id of this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Canonical identifier for this value set
    pub url: String,

    /// Business version of the value set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Name for this value set (computer friendly)
    pub name: String,

    /// Name for this value set (human friendly)
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

    /// Natural language description of the value set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The context that the content is intended to support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_context: Option<Vec<serde_json::Value>>,

    /// Intended jurisdiction for value set (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Vec<serde_json::Value>>,

    /// Immutable | Extensible | Complete | Supplement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immutable: Option<bool>,

    /// Purpose and use of the value set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    /// Use and/or publishing restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,

    /// Content logical definition of the value set (CLD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compose: Option<ValueSetCompose>,

    /// Used when the value set is "expanded"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expansion: Option<serde_json::Value>,
}

impl ValueSetResource {
    /// Create a new ValueSet resource
    pub fn new(url: impl Into<String>, name: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            resource_type: "ValueSet".to_string(),
            id: None,
            url: url.into(),
            version: None,
            name: name.into(),
            title: None,
            status: status.into(),
            date: None,
            publisher: None,
            description: None,
            use_context: None,
            jurisdiction: None,
            immutable: None,
            purpose: None,
            copyright: None,
            compose: None,
            expansion: None,
        }
    }
}

/// Content logical definition of the value set (CLD)
///
/// Defines what codes are in the value set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetCompose {
    /// Fixed date for references with no specified version (transitive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_date: Option<String>,

    /// Whether inactive codes are in the value set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inactive: Option<bool>,

    /// Include one or more codes from a code system or other value set(s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<ValueSetInclude>>,

    /// Explicitly exclude codes from a code system or other value set(s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<ValueSetInclude>>,
}

impl ValueSetCompose {
    /// Create a new empty compose
    pub fn new() -> Self {
        Self {
            locked_date: None,
            inactive: None,
            include: None,
            exclude: None,
        }
    }

    /// Add an include entry
    pub fn add_include(&mut self, include: ValueSetInclude) {
        if let Some(ref mut includes) = self.include {
            includes.push(include);
        } else {
            self.include = Some(vec![include]);
        }
    }

    /// Add an exclude entry
    pub fn add_exclude(&mut self, exclude: ValueSetInclude) {
        if let Some(ref mut excludes) = self.exclude {
            excludes.push(exclude);
        } else {
            self.exclude = Some(vec![exclude]);
        }
    }
}

impl Default for ValueSetCompose {
    fn default() -> Self {
        Self::new()
    }
}

/// Include or exclude codes from a code system or value set
///
/// Specifies a concept to be included or excluded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetInclude {
    /// The system the codes come from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Specific version of the code system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// A concept defined in the system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<Vec<ValueSetConcept>>,

    /// Select codes/concepts by their properties (including relationships)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<ValueSetFilter>>,

    /// Select the contents included in this value set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_set: Option<Vec<String>>,
}

impl ValueSetInclude {
    /// Create a new include for a specific system
    pub fn from_system(system: impl Into<String>) -> Self {
        Self {
            system: Some(system.into()),
            version: None,
            concept: None,
            filter: None,
            value_set: None,
        }
    }

    /// Create a new include from another value set
    pub fn from_valueset(value_set_url: impl Into<String>) -> Self {
        Self {
            system: None,
            version: None,
            concept: None,
            filter: None,
            value_set: Some(vec![value_set_url.into()]),
        }
    }

    /// Add a concept
    pub fn add_concept(&mut self, concept: ValueSetConcept) {
        if let Some(ref mut concepts) = self.concept {
            concepts.push(concept);
        } else {
            self.concept = Some(vec![concept]);
        }
    }

    /// Add a filter
    pub fn add_filter(&mut self, filter: ValueSetFilter) {
        if let Some(ref mut filters) = self.filter {
            filters.push(filter);
        } else {
            self.filter = Some(vec![filter]);
        }
    }
}

/// A concept from a code system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetConcept {
    /// Code or expression from system
    pub code: String,

    /// Text to display for this code for this value set in this valueset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    /// Additional representations for this concept
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<Vec<serde_json::Value>>,
}

impl ValueSetConcept {
    /// Create a new concept with just a code
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            display: None,
            designation: None,
        }
    }

    /// Create a new concept with code and display
    pub fn with_display(code: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            display: Some(display.into()),
            designation: None,
        }
    }
}

/// Filter to select codes from a code system
///
/// Selects codes based on properties defined by the code system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValueSetFilter {
    /// A property/filter defined by the code system
    pub property: String,

    /// = | is-a | descendent-of | is-not-a | regex | in | not-in | generalizes | exists
    pub op: String,

    /// Code from the system, or regex criteria, or boolean value for exists
    pub value: String,
}

impl ValueSetFilter {
    /// Create a new filter
    pub fn new(
        property: impl Into<String>,
        op: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            property: property.into(),
            op: op.into(),
            value: value.into(),
        }
    }

    /// Create an "is-a" filter (concept is a child of the given parent)
    pub fn is_a(parent_code: impl Into<String>) -> Self {
        Self::new("concept", "is-a", parent_code)
    }

    /// Create a "descendent-of" filter
    pub fn descendent_of(parent_code: impl Into<String>) -> Self {
        Self::new("concept", "descendent-of", parent_code)
    }

    /// Create a "regex" filter
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self::new("concept", "regex", pattern)
    }

    /// Create an "exists" filter
    pub fn exists(property: impl Into<String>) -> Self {
        Self::new(property, "exists", "true")
    }
}

// ============================================================================
// CodeSystem
// ============================================================================

/// FHIR CodeSystem resource
///
/// Represents a set of codes with definitions and relationships.
/// See: <https://www.hl7.org/fhir/codesystem.html>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemResource {
    /// Resource type (always "CodeSystem")
    pub resource_type: String,

    /// Logical id of this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Canonical identifier for this code system
    pub url: String,

    /// Business version of the code system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Name for this code system (computer friendly)
    pub name: String,

    /// Name for this code system (human friendly)
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

    /// Natural language description of the code system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The context that the content is intended to support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_context: Option<Vec<serde_json::Value>>,

    /// Intended jurisdiction for code system (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<Vec<serde_json::Value>>,

    /// Why this code system is defined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    /// Use and/or publishing restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,

    /// If code comparison is case sensitive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,

    /// not-present | example | fragment | complete | supplement
    pub content: String,

    /// Total concepts in the code system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,

    /// If definitions are not stable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<bool>,

    /// Concepts in the code system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<Vec<CodeSystemConcept>>,

    /// Additional information supplied about each concept
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<Vec<CodeSystemProperty>>,
}

impl CodeSystemResource {
    /// Create a new CodeSystem resource
    pub fn new(url: impl Into<String>, name: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            resource_type: "CodeSystem".to_string(),
            id: None,
            url: url.into(),
            version: None,
            name: name.into(),
            title: None,
            status: status.into(),
            date: None,
            publisher: None,
            description: None,
            use_context: None,
            jurisdiction: None,
            purpose: None,
            copyright: None,
            case_sensitive: None,
            content: "complete".to_string(),
            count: None,
            experimental: None,
            concept: None,
            property: None,
        }
    }

    /// Add a concept to the code system
    pub fn add_concept(&mut self, concept: CodeSystemConcept) {
        if let Some(ref mut concepts) = self.concept {
            concepts.push(concept);
        } else {
            self.concept = Some(vec![concept]);
        }
    }

    /// Add a property definition
    pub fn add_property(&mut self, property: CodeSystemProperty) {
        if let Some(ref mut properties) = self.property {
            properties.push(property);
        } else {
            self.property = Some(vec![property]);
        }
    }

    /// Update the count of concepts
    pub fn update_count(&mut self) {
        if let Some(ref concepts) = self.concept {
            self.count = Some(self.count_concepts_recursive(concepts));
        }
    }

    /// Recursively count all concepts including children
    fn count_concepts_recursive(&self, concepts: &[CodeSystemConcept]) -> u32 {
        concepts
            .iter()
            .map(|c| {
                let mut count = 1;
                if let Some(ref children) = c.concept {
                    count += self.count_concepts_recursive(children);
                }
                count
            })
            .sum()
    }
}

/// A concept defined in the code system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemConcept {
    /// Code that identifies concept
    pub code: String,

    /// Text to display to the user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,

    /// Formal definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,

    /// Additional representations for the concept
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designation: Option<Vec<serde_json::Value>>,

    /// Property value for the concept
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<Vec<CodeSystemConceptProperty>>,

    /// Child Concepts (is-a/contains/categorizes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concept: Option<Vec<CodeSystemConcept>>,
}

impl CodeSystemConcept {
    /// Create a new concept with just a code
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            display: None,
            definition: None,
            designation: None,
            property: None,
            concept: None,
        }
    }

    /// Create a concept with code and display
    pub fn with_display(code: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            display: Some(display.into()),
            definition: None,
            designation: None,
            property: None,
            concept: None,
        }
    }

    /// Create a concept with code, display, and definition
    pub fn with_definition(
        code: impl Into<String>,
        display: impl Into<String>,
        definition: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            display: Some(display.into()),
            definition: Some(definition.into()),
            designation: None,
            property: None,
            concept: None,
        }
    }

    /// Add a child concept
    pub fn add_child(&mut self, child: CodeSystemConcept) {
        if let Some(ref mut children) = self.concept {
            children.push(child);
        } else {
            self.concept = Some(vec![child]);
        }
    }

    /// Add a property value
    pub fn add_property(&mut self, property: CodeSystemConceptProperty) {
        if let Some(ref mut properties) = self.property {
            properties.push(property);
        } else {
            self.property = Some(vec![property]);
        }
    }
}

/// A property value for a concept
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemConceptProperty {
    /// Reference to CodeSystem.property.code
    pub code: String,

    /// Value of the property for this concept
    #[serde(flatten)]
    pub value: CodeSystemPropertyValue,
}

impl CodeSystemConceptProperty {
    /// Create a new concept property
    pub fn new(code: impl Into<String>, value: CodeSystemPropertyValue) -> Self {
        Self {
            code: code.into(),
            value,
        }
    }
}

/// Value of a concept property (can be various types)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CodeSystemPropertyValue {
    #[serde(rename = "valueCode")]
    Code(String),
    #[serde(rename = "valueString")]
    String(String),
    #[serde(rename = "valueInteger")]
    Integer(i32),
    #[serde(rename = "valueBoolean")]
    Boolean(bool),
}

/// Additional information about a property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodeSystemProperty {
    /// Identifies the property on the concepts
    pub code: String,

    /// Formal identifier for the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// Why the property is defined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// code | Coding | string | integer | boolean | dateTime | decimal
    #[serde(rename = "type")]
    pub type_: String,
}

impl CodeSystemProperty {
    /// Create a new property definition
    pub fn new(code: impl Into<String>, type_: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            uri: None,
            description: None,
            type_: type_.into(),
        }
    }

    /// Create a property with description
    pub fn with_description(
        code: impl Into<String>,
        type_: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            uri: None,
            description: Some(description.into()),
            type_: type_.into(),
        }
    }
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
        assert_eq!(
            sd.url,
            "http://example.org/fhir/StructureDefinition/TestProfile"
        );
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
        assert_eq!(
            BindingStrength::from_str("required"),
            Some(BindingStrength::Required)
        );
        assert_eq!(
            BindingStrength::from_str("REQUIRED"),
            Some(BindingStrength::Required)
        );
        assert_eq!(
            BindingStrength::from_str("extensible"),
            Some(BindingStrength::Extensible)
        );
        assert_eq!(BindingStrength::from_str("invalid"), None);
    }

    #[test]
    fn test_element_definition_type() {
        let type_def = ElementDefinitionType::new("string");
        assert_eq!(type_def.code, "string");
        assert!(type_def.profile.is_none());
    }
}
