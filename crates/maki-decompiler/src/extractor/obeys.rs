//! Obeys rule extractor for invariants/constraints
//!
//! This extractor converts FHIR ElementDefinition constraints into FSH obeys rules.
//! Constraints represent invariants that must be satisfied for the element to be valid.

use super::RuleExtractor;
use crate::{
    Result,
    exportable::{ExportableRule, ObeysRule},
    processor::ProcessableElementDefinition,
};

/// Extractor for obeys rules (invariants/constraints)
///
/// In FHIR, constraints are defined in ElementDefinition.constraint array.
/// Each constraint has a key (identifier) that can be referenced in obeys rules.
///
/// In FSH, this becomes:
/// ```fsh
/// * element obeys invariant-1
/// * obeys resource-level-invariant
/// ```
pub struct ObeysExtractor;

impl ObeysExtractor {
    /// Extract obeys rules from an element's constraints
    fn extract_constraints(elem: &mut ProcessableElementDefinition) -> Result<Vec<ObeysRule>> {
        let mut rules = Vec::new();

        if let Some(ref constraints) = elem.element.constraint {
            for constraint in constraints {
                // Create obeys rule with path
                rules.push(ObeysRule {
                    path: Some(elem.element.fsh_path()),
                    invariant: constraint.key.clone(),
                });
            }

            // Mark constraints as processed
            elem.mark_processed("constraint");
        }

        Ok(rules)
    }

    /// Extract resource-level constraints (constraints on root element)
    ///
    /// These don't have a path in FSH:
    /// ```fsh
    /// * obeys inv-1
    /// ```
    pub fn extract_root_constraints(
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<ObeysRule>> {
        let mut rules = Vec::new();

        // Only extract for root element (path has no dots, like "Patient")
        if !elem.element.path.contains('.')
            && let Some(ref constraints) = elem.element.constraint
        {
            for constraint in constraints {
                // Create obeys rule without path (resource-level)
                rules.push(ObeysRule {
                    path: None,
                    invariant: constraint.key.clone(),
                });
            }

            elem.mark_processed("constraint");
        }

        Ok(rules)
    }
}

impl RuleExtractor for ObeysExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule + Send + Sync>>> {
        // Check if constraints have already been processed
        if elem.is_processed("constraint") {
            return Ok(vec![]);
        }

        let rules = Self::extract_constraints(elem)?;

        Ok(rules
            .into_iter()
            .map(|r| Box::new(r) as Box<dyn ExportableRule + Send + Sync>)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ElementDefinition, common::Constraint};

    fn create_test_element(path: &str) -> ProcessableElementDefinition {
        ProcessableElementDefinition::new(ElementDefinition {
            id: path.to_string(),
            path: path.to_string(),
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
        })
    }

    #[test]
    fn test_extract_single_constraint() {
        let mut elem = create_test_element("Patient.identifier");
        elem.element.constraint = Some(vec![Constraint {
            key: "us-core-1".to_string(),
            severity: Some("error".to_string()),
            human: "Must have system and value".to_string(),
            expression: Some("system.exists() and value.exists()".to_string()),
            xpath: None,
        }]);

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].to_fsh(), "identifier obeys us-core-1");
        assert!(elem.is_processed("constraint"));
    }

    #[test]
    fn test_extract_multiple_constraints() {
        let mut elem = create_test_element("Patient.name");
        elem.element.constraint = Some(vec![
            Constraint {
                key: "inv-1".to_string(),
                severity: Some("error".to_string()),
                human: "Must have family or given".to_string(),
                expression: Some("family.exists() or given.exists()".to_string()),
                xpath: None,
            },
            Constraint {
                key: "inv-2".to_string(),
                severity: Some("warning".to_string()),
                human: "Should have use".to_string(),
                expression: Some("use.exists()".to_string()),
                xpath: None,
            },
        ]);

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].to_fsh(), "name obeys inv-1");
        assert_eq!(rules[1].to_fsh(), "name obeys inv-2");
    }

    #[test]
    fn test_extract_no_constraints() {
        let mut elem = create_test_element("Patient.identifier");
        elem.element.constraint = None;

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_empty_constraints() {
        let mut elem = create_test_element("Patient.identifier");
        elem.element.constraint = Some(vec![]);

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_no_duplicate_processing() {
        let mut elem = create_test_element("Patient.identifier");
        elem.element.constraint = Some(vec![Constraint {
            key: "us-core-1".to_string(),
            severity: Some("error".to_string()),
            human: "Must have system".to_string(),
            expression: Some("system.exists()".to_string()),
            xpath: None,
        }]);

        let extractor = ObeysExtractor;

        // First extraction
        let rules1 = extractor.extract(&mut elem).unwrap();
        assert_eq!(rules1.len(), 1);

        // Second extraction should return empty (already processed)
        let rules2 = extractor.extract(&mut elem).unwrap();
        assert_eq!(rules2.len(), 0);
    }

    #[test]
    fn test_extract_root_constraints() {
        let mut elem = create_test_element("Patient");
        elem.element.constraint = Some(vec![Constraint {
            key: "pat-1".to_string(),
            severity: Some("error".to_string()),
            human: "Resource-level constraint".to_string(),
            expression: Some("name.exists() or identifier.exists()".to_string()),
            xpath: None,
        }]);

        let rules = ObeysExtractor::extract_root_constraints(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        // Root-level constraints don't have a path
        assert_eq!(rules[0].to_fsh(), "obeys pat-1");
        assert!(elem.is_processed("constraint"));
    }

    #[test]
    fn test_extract_root_constraints_non_root_element() {
        let mut elem = create_test_element("Patient.identifier");
        elem.element.constraint = Some(vec![Constraint {
            key: "inv-1".to_string(),
            severity: Some("error".to_string()),
            human: "Some constraint".to_string(),
            expression: Some("value.exists()".to_string()),
            xpath: None,
        }]);

        let rules = ObeysExtractor::extract_root_constraints(&mut elem).unwrap();

        // Non-root elements shouldn't generate root-level constraints
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_constraint_key_preserved() {
        let mut elem = create_test_element("Observation.value[x]");
        elem.element.constraint = Some(vec![Constraint {
            key: "obs-7".to_string(),
            severity: Some("error".to_string()),
            human: "Value is required if status is final".to_string(),
            expression: Some("(status = 'final') implies value.exists()".to_string()),
            xpath: None,
        }]);

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        let fsh = rules[0].to_fsh();
        assert!(fsh.contains("obs-7"));
        assert!(fsh.contains("value[x]"));
    }

    #[test]
    fn test_fsh_path_conversion() {
        let mut elem = create_test_element("Patient.identifier.value");
        elem.element.constraint = Some(vec![Constraint {
            key: "inv-1".to_string(),
            severity: Some("error".to_string()),
            human: "Value must not be empty".to_string(),
            expression: Some("$this.length() > 0".to_string()),
            xpath: None,
        }]);

        let extractor = ObeysExtractor;
        let rules = extractor.extract(&mut elem).unwrap();

        assert_eq!(rules.len(), 1);
        let fsh = rules[0].to_fsh();
        // Should use FSH path (without resource prefix)
        assert!(fsh.contains("identifier.value"));
        assert!(!fsh.contains("Patient.identifier"));
    }
}
