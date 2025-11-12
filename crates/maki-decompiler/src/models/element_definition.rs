//! ElementDefinition model - represents FHIR element constraints

use super::common::*;
use serde::{Deserialize, Serialize};

/// List of element definitions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ElementList {
    pub element: Vec<ElementDefinition>,
}

/// FHIR ElementDefinition - represents a constraint on a FHIR element
///
/// Maps to multiple FSH rules (cardinality, binding, type, assignment, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefinition {
    // Core identification
    pub id: String,
    pub path: String,
    pub slice_name: Option<String>,

    // Cardinality
    pub min: Option<u32>,
    pub max: Option<String>, // Can be "*"

    // Type constraints
    #[serde(rename = "type")]
    pub type_: Option<Vec<TypeRef>>,

    // Flags
    pub must_support: Option<bool>,
    pub is_modifier: Option<bool>,
    pub is_summary: Option<bool>,

    // Binding
    pub binding: Option<Binding>,

    // Constraints
    pub constraint: Option<Vec<Constraint>>,

    // Slicing
    pub slicing: Option<Slicing>,

    // Fixed values (20+ polymorphic properties - most common ones)
    pub fixed_boolean: Option<bool>,
    pub fixed_integer: Option<i32>,
    pub fixed_decimal: Option<f64>,
    pub fixed_string: Option<String>,
    pub fixed_uri: Option<String>,
    pub fixed_url: Option<String>,
    pub fixed_canonical: Option<String>,
    pub fixed_code: Option<String>,
    pub fixed_date: Option<String>,
    pub fixed_date_time: Option<String>,
    pub fixed_instant: Option<String>,
    pub fixed_time: Option<String>,
    pub fixed_id: Option<String>,
    pub fixed_oid: Option<String>,
    pub fixed_uuid: Option<String>,
    pub fixed_codeable_concept: Option<CodeableConcept>,
    pub fixed_coding: Option<Coding>,
    pub fixed_quantity: Option<Quantity>,
    pub fixed_identifier: Option<Identifier>,
    pub fixed_reference: Option<Reference>,

    // Pattern values (10+ polymorphic properties - most common ones)
    pub pattern_boolean: Option<bool>,
    pub pattern_integer: Option<i32>,
    pub pattern_decimal: Option<f64>,
    pub pattern_string: Option<String>,
    pub pattern_code: Option<String>,
    pub pattern_codeable_concept: Option<CodeableConcept>,
    pub pattern_coding: Option<Coding>,
    pub pattern_quantity: Option<Quantity>,
    pub pattern_identifier: Option<Identifier>,
    pub pattern_reference: Option<Reference>,

    // Metadata (for caret rules)
    pub short: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
    pub requirements: Option<String>,
    pub alias: Option<Vec<String>>,
    pub example: Option<Vec<Example>>,
}

impl ElementDefinition {
    /// Get path without resource prefix (e.g., "Patient.identifier" → "identifier")
    pub fn fsh_path(&self) -> String {
        self.path.split('.').skip(1).collect::<Vec<_>>().join(".")
    }

    /// Check if this is a slicing entry (has slicing definition, no slice name)
    pub fn is_slicing_entry(&self) -> bool {
        self.slicing.is_some() && self.slice_name.is_none()
    }

    /// Check if this is a slice (has slice name)
    pub fn is_slice(&self) -> bool {
        self.slice_name.is_some()
    }

    /// Get the number of fixed[x] properties that are set
    pub fn count_fixed_values(&self) -> usize {
        let mut count = 0;
        if self.fixed_boolean.is_some() {
            count += 1;
        }
        if self.fixed_integer.is_some() {
            count += 1;
        }
        if self.fixed_decimal.is_some() {
            count += 1;
        }
        if self.fixed_string.is_some() {
            count += 1;
        }
        if self.fixed_code.is_some() {
            count += 1;
        }
        if self.fixed_codeable_concept.is_some() {
            count += 1;
        }
        if self.fixed_coding.is_some() {
            count += 1;
        }
        if self.fixed_quantity.is_some() {
            count += 1;
        }
        if self.fixed_identifier.is_some() {
            count += 1;
        }
        if self.fixed_reference.is_some() {
            count += 1;
        }
        count
    }

    /// Get the number of pattern[x] properties that are set
    pub fn count_pattern_values(&self) -> usize {
        let mut count = 0;
        if self.pattern_boolean.is_some() {
            count += 1;
        }
        if self.pattern_integer.is_some() {
            count += 1;
        }
        if self.pattern_string.is_some() {
            count += 1;
        }
        if self.pattern_code.is_some() {
            count += 1;
        }
        if self.pattern_codeable_concept.is_some() {
            count += 1;
        }
        if self.pattern_coding.is_some() {
            count += 1;
        }
        if self.pattern_quantity.is_some() {
            count += 1;
        }
        if self.pattern_identifier.is_some() {
            count += 1;
        }
        if self.pattern_reference.is_some() {
            count += 1;
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsh_path() {
        let elem = ElementDefinition {
            id: "Patient.identifier".to_string(),
            path: "Patient.identifier".to_string(),
            slice_name: None,
            min: None,
            max: None,
            type_: None,
            must_support: None,
            is_modifier: None,
            is_summary: None,
            binding: None,
            constraint: None,
            slicing: None,
            fixed_boolean: None,
            fixed_integer: None,
            fixed_decimal: None,
            fixed_string: None,
            fixed_uri: None,
            fixed_url: None,
            fixed_canonical: None,
            fixed_code: None,
            fixed_date: None,
            fixed_date_time: None,
            fixed_instant: None,
            fixed_time: None,
            fixed_id: None,
            fixed_oid: None,
            fixed_uuid: None,
            fixed_codeable_concept: None,
            fixed_coding: None,
            fixed_quantity: None,
            fixed_identifier: None,
            fixed_reference: None,
            pattern_boolean: None,
            pattern_integer: None,
            pattern_decimal: None,
            pattern_string: None,
            pattern_code: None,
            pattern_codeable_concept: None,
            pattern_coding: None,
            pattern_quantity: None,
            pattern_identifier: None,
            pattern_reference: None,
            short: None,
            definition: None,
            comment: None,
            requirements: None,
            alias: None,
            example: None,
        };

        assert_eq!(elem.fsh_path(), "identifier");
    }

    #[test]
    fn test_is_slicing_entry() {
        let mut elem = ElementDefinition {
            id: "Patient.identifier".to_string(),
            path: "Patient.identifier".to_string(),
            slice_name: None,
            slicing: Some(Slicing {
                discriminator: None,
                description: None,
                ordered: None,
                rules: None,
            }),
            // ... rest of fields omitted for brevity
            min: None,
            max: None,
            type_: None,
            must_support: None,
            is_modifier: None,
            is_summary: None,
            binding: None,
            constraint: None,
            fixed_boolean: None,
            fixed_integer: None,
            fixed_decimal: None,
            fixed_string: None,
            fixed_uri: None,
            fixed_url: None,
            fixed_canonical: None,
            fixed_code: None,
            fixed_date: None,
            fixed_date_time: None,
            fixed_instant: None,
            fixed_time: None,
            fixed_id: None,
            fixed_oid: None,
            fixed_uuid: None,
            fixed_codeable_concept: None,
            fixed_coding: None,
            fixed_quantity: None,
            fixed_identifier: None,
            fixed_reference: None,
            pattern_boolean: None,
            pattern_integer: None,
            pattern_decimal: None,
            pattern_string: None,
            pattern_code: None,
            pattern_codeable_concept: None,
            pattern_coding: None,
            pattern_quantity: None,
            pattern_identifier: None,
            pattern_reference: None,
            short: None,
            definition: None,
            comment: None,
            requirements: None,
            alias: None,
            example: None,
        };

        assert!(elem.is_slicing_entry());

        elem.slice_name = Some("mrn".to_string());
        assert!(!elem.is_slicing_entry());
        assert!(elem.is_slice());
    }

    #[test]
    fn test_deserialize_element_with_fixed_value() {
        let json = r#"{
            "id": "Patient.active",
            "path": "Patient.active",
            "fixedBoolean": true
        }"#;

        let elem: ElementDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(elem.fixed_boolean, Some(true));
        assert_eq!(elem.count_fixed_values(), 1);
    }

    #[test]
    fn test_deserialize_element_with_pattern() {
        let json = r#"{
            "id": "Patient.gender",
            "path": "Patient.gender",
            "patternCode": "male"
        }"#;

        let elem: ElementDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(elem.pattern_code, Some("male".to_string()));
        assert_eq!(elem.count_pattern_values(), 1);
    }
}
