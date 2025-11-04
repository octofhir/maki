//! Value Rules Implementation
//!
//! Implements FSH value rules for setting properties and metadata:
//! - BindingRule: Bind elements to ValueSets
//! - AssignmentRule: Set property values with path traversal
//! - CaretValueRule: Set metadata with ^ syntax
//! - ObeysRule: Add invariants

use crate::export::fhir_types::{
    BindingStrength, ElementDefinition, ElementDefinitionBinding, ElementDefinitionConstraint,
    StructureDefinition,
};
use serde_json::Value as JsonValue;
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Debug, Error)]
pub enum ValueRuleError {
    #[error("Invalid rule: {0}")]
    InvalidRule(String),

    #[error("Path parsing error: {0}")]
    PathError(String),

    #[error("Value conversion error: {0}")]
    ConversionError(String),
}

type Result<T> = std::result::Result<T, ValueRuleError>;

/// Binding rule data
#[derive(Debug, Clone)]
pub struct BindingRule {
    pub path: String,
    pub valueset: String,
    pub strength: BindingStrength,
}

/// Assignment rule data
#[derive(Debug, Clone)]
pub struct AssignmentRule {
    pub path: String,
    pub value: FshValue,
}

/// FSH value types
#[derive(Debug, Clone)]
pub enum FshValue {
    String(String),
    Code(String),
    Boolean(bool),
    Integer(i64),
    Decimal(f64),
    Canonical(String),
    Reference(String),
}

/// Caret value rule data
#[derive(Debug, Clone)]
pub struct CaretValueRule {
    pub path: String,
    pub property: String,
    pub value: FshValue,
}

/// Obeys rule data
#[derive(Debug, Clone)]
pub struct ObeysRule {
    pub path: String,
    pub invariant_id: String,
}

/// Apply binding rule
pub fn apply_binding_rule(sd: &mut StructureDefinition, rule: &BindingRule) -> Result<()> {
    debug!("Applying BindingRule to {}", rule.path);

    let element = find_or_create_element(sd, &rule.path)?;

    let binding = ElementDefinitionBinding {
        strength: rule.strength,
        description: None,
        value_set: Some(rule.valueset.clone()),
    };

    element.binding = Some(binding);
    trace!("  Set binding: {} ({:?})", rule.valueset, rule.strength);

    Ok(())
}

/// Apply assignment rule to a JSON resource
pub fn apply_assignment_rule(resource: &mut JsonValue, rule: &AssignmentRule) -> Result<()> {
    debug!("Applying AssignmentRule to path: {}", rule.path);

    let segments = parse_assignment_path(&rule.path)?;
    let target = navigate_to_path(resource, &segments)?;
    *target = convert_fsh_value_to_json(&rule.value)?;

    trace!("  Set value at {}", rule.path);
    Ok(())
}

/// Apply caret value rule (metadata)
pub fn apply_caret_rule(sd: &mut StructureDefinition, rule: &CaretValueRule) -> Result<()> {
    debug!(
        "Applying CaretValueRule: {} = {:?}",
        rule.property, rule.value
    );

    if rule.path.is_empty() {
        // Root-level metadata
        apply_root_caret(sd, &rule.property, &rule.value)?;
    } else {
        // Element-level metadata
        let element = find_or_create_element(sd, &rule.path)?;
        apply_element_caret(element, &rule.property, &rule.value)?;
    }

    Ok(())
}

/// Apply obeys rule (add invariant)
pub fn apply_obeys_rule(sd: &mut StructureDefinition, rule: &ObeysRule) -> Result<()> {
    debug!("Applying ObeysRule to {}: {}", rule.path, rule.invariant_id);

    let element = find_or_create_element(sd, &rule.path)?;

    // Create constraint for the invariant
    let constraint = ElementDefinitionConstraint {
        key: rule.invariant_id.clone(),
        severity: Some("error".to_string()),
        human: format!("Constraint {}", rule.invariant_id),
        expression: None,
    };

    if element.constraint.is_none() {
        element.constraint = Some(Vec::new());
    }

    element.constraint.as_mut().unwrap().push(constraint);
    trace!("  Added invariant: {}", rule.invariant_id);

    Ok(())
}

// Helper functions

fn find_or_create_element<'a>(
    sd: &'a mut StructureDefinition,
    path: &str,
) -> Result<&'a mut ElementDefinition> {
    if sd.differential.is_none() {
        sd.differential = Some(crate::export::fhir_types::StructureDefinitionDifferential {
            element: Vec::new(),
        });
    }
    let diff = sd.differential.as_mut().unwrap();

    let exists = diff.element.iter().any(|e| e.path == path);

    if !exists {
        diff.element.push(ElementDefinition::new(path.to_string()));
    }

    Ok(diff.element.iter_mut().find(|e| e.path == path).unwrap())
}

fn parse_assignment_path(path: &str) -> Result<Vec<PathSegment>> {
    let mut segments = Vec::new();

    for part in path.split('.') {
        if let Some(bracket_pos) = part.find('[') {
            // Array element: name[index]
            let name = part[..bracket_pos].to_string();
            let index_str = &part[bracket_pos + 1..part.len() - 1];
            let index = index_str.parse::<usize>().map_err(|_| {
                ValueRuleError::PathError(format!("Invalid array index: {}", index_str))
            })?;
            segments.push(PathSegment::ArrayElement { name, index });
        } else {
            // Simple property
            segments.push(PathSegment::Property(part.to_string()));
        }
    }

    Ok(segments)
}

#[derive(Debug)]
enum PathSegment {
    Property(String),
    ArrayElement { name: String, index: usize },
}

fn navigate_to_path<'a>(
    resource: &'a mut JsonValue,
    segments: &[PathSegment],
) -> Result<&'a mut JsonValue> {
    let mut current = resource;

    for segment in segments {
        match segment {
            PathSegment::Property(name) => {
                if !current.is_object() {
                    return Err(ValueRuleError::PathError(format!(
                        "Cannot navigate property '{}' on non-object",
                        name
                    )));
                }
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(name) {
                    obj.insert(name.clone(), JsonValue::Null);
                }
                current = obj.get_mut(name).unwrap();
            }
            PathSegment::ArrayElement { name, index } => {
                if !current.is_object() {
                    return Err(ValueRuleError::PathError(format!(
                        "Cannot navigate property '{}' on non-object",
                        name
                    )));
                }
                let obj = current.as_object_mut().unwrap();
                if !obj.contains_key(name) {
                    obj.insert(name.clone(), JsonValue::Array(Vec::new()));
                }

                let arr = obj.get_mut(name).unwrap();
                if !arr.is_array() {
                    return Err(ValueRuleError::PathError(format!(
                        "Property '{}' is not an array",
                        name
                    )));
                }

                let arr_vec = arr.as_array_mut().unwrap();
                while arr_vec.len() <= *index {
                    arr_vec.push(JsonValue::Object(serde_json::Map::new()));
                }
                current = &mut arr_vec[*index];
            }
        }
    }

    Ok(current)
}

fn convert_fsh_value_to_json(value: &FshValue) -> Result<JsonValue> {
    Ok(match value {
        FshValue::String(s) => JsonValue::String(s.clone()),
        FshValue::Code(c) => JsonValue::String(c.clone()),
        FshValue::Boolean(b) => JsonValue::Bool(*b),
        FshValue::Integer(i) => JsonValue::Number((*i).into()),
        FshValue::Decimal(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .ok_or_else(|| ValueRuleError::ConversionError(format!("Invalid decimal: {}", f)))?,
        FshValue::Canonical(c) => JsonValue::String(c.clone()),
        FshValue::Reference(r) => JsonValue::String(r.clone()),
    })
}

fn apply_root_caret(sd: &mut StructureDefinition, property: &str, value: &FshValue) -> Result<()> {
    match property {
        "status" => {
            if let FshValue::Code(status) = value {
                sd.status = status.clone();
            }
        }
        "version" => {
            if let FshValue::String(version) = value {
                sd.version = Some(version.clone());
            }
        }
        "experimental" => {
            if let FshValue::Boolean(exp) = value {
                sd.experimental = Some(*exp);
            }
        }
        "publisher" => {
            if let FshValue::String(pub_) = value {
                sd.publisher = Some(pub_.clone());
            }
        }
        "description" => {
            if let FshValue::String(desc) = value {
                sd.description = Some(desc.clone());
            }
        }
        _ => {
            trace!("Skipping unsupported root caret: {}", property);
        }
    }
    Ok(())
}

fn apply_element_caret(
    element: &mut ElementDefinition,
    property: &str,
    value: &FshValue,
) -> Result<()> {
    match property {
        "short" => {
            if let FshValue::String(s) = value {
                element.short = Some(s.clone());
            }
        }
        "definition" => {
            if let FshValue::String(s) = value {
                element.definition = Some(s.clone());
            }
        }
        "comment" => {
            if let FshValue::String(s) = value {
                element.comment = Some(s.clone());
            }
        }
        _ => {
            trace!("Skipping unsupported element caret: {}", property);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sd() -> StructureDefinition {
        StructureDefinition {
            resource_type: "StructureDefinition".to_string(),
            id: None,
            url: "http://example.com/test".to_string(),
            version: None,
            name: "TestProfile".to_string(),
            title: None,
            status: "draft".to_string(),
            date: None,
            publisher: None,
            description: None,
            experimental: None,
            extension: None,
            fhir_version: None,
            kind: crate::export::fhir_types::StructureDefinitionKind::Resource,
            is_abstract: false,
            type_field: "Patient".to_string(),
            base_definition: Some("http://hl7.org/fhir/StructureDefinition/Patient".to_string()),
            derivation: Some("constraint".to_string()),
            context: None,
            snapshot: None,
            differential: Some(crate::export::fhir_types::StructureDefinitionDifferential {
                element: Vec::new(),
            }),
            mapping: None,
        }
    }

    #[test]
    fn test_binding_rule() {
        let mut sd = create_test_sd();

        let rule = BindingRule {
            path: "Patient.gender".to_string(),
            valueset: "http://hl7.org/fhir/ValueSet/administrative-gender".to_string(),
            strength: BindingStrength::Required,
        };

        let result = apply_binding_rule(&mut sd, &rule);
        assert!(result.is_ok());

        let element = sd
            .differential
            .unwrap()
            .element
            .into_iter()
            .find(|e| e.path == "Patient.gender")
            .unwrap();

        assert!(element.binding.is_some());
        assert_eq!(
            element.binding.unwrap().value_set.unwrap(),
            "http://hl7.org/fhir/ValueSet/administrative-gender"
        );
    }

    #[test]
    fn test_parse_assignment_path() {
        let path = "name[0].family";
        let segments = parse_assignment_path(path).unwrap();

        assert_eq!(segments.len(), 2);
        match &segments[0] {
            PathSegment::ArrayElement { name, index } => {
                assert_eq!(name, "name");
                assert_eq!(*index, 0);
            }
            _ => panic!("Expected ArrayElement"),
        }
        match &segments[1] {
            PathSegment::Property(name) => {
                assert_eq!(name, "family");
            }
            _ => panic!("Expected Property"),
        }
    }

    #[test]
    fn test_caret_rule_root() {
        let mut sd = create_test_sd();

        let rule = CaretValueRule {
            path: String::new(),
            property: "version".to_string(),
            value: FshValue::String("2.0.0".to_string()),
        };

        let result = apply_caret_rule(&mut sd, &rule);
        assert!(result.is_ok());
        assert_eq!(sd.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_obeys_rule() {
        let mut sd = create_test_sd();

        let rule = ObeysRule {
            path: "Patient.identifier".to_string(),
            invariant_id: "us-core-1".to_string(),
        };

        let result = apply_obeys_rule(&mut sd, &rule);
        assert!(result.is_ok());

        let element = sd
            .differential
            .unwrap()
            .element
            .into_iter()
            .find(|e| e.path == "Patient.identifier")
            .unwrap();

        assert!(element.constraint.is_some());
        assert_eq!(element.constraint.unwrap()[0].key, "us-core-1");
    }
}
