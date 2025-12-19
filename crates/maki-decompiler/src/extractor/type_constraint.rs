//! Type constraint extractor
//!
//! Extracts type constraints (only rules) from ElementDefinition

use super::RuleExtractor;
use crate::{
    Result,
    exportable::{ExportableRule, TypeReference, TypeRule},
    models::TypeRef,
    processor::ProcessableElementDefinition,
};
use log::debug;

/// Extracts type constraint rules (only Type1 or Type2)
pub struct TypeExtractor;

impl RuleExtractor for TypeExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule + Send + Sync>>> {
        let mut rules: Vec<Box<dyn ExportableRule + Send + Sync>> = Vec::new();

        // Check if type constraints exist
        if let Some(types) = &elem.element.type_ {
            if types.is_empty() {
                return Ok(rules);
            }

            // Skip if already processed
            if elem.is_processed("type") {
                return Ok(rules);
            }

            let fsh_path = Self::element_path_to_fsh(&elem.element.path);

            // Convert FHIR TypeRef to FSH TypeReference
            let type_references: Vec<TypeReference> =
                types.iter().map(|t| self.convert_type_ref(t)).collect();

            debug!(
                "Extracting type constraint for path {} with {} types",
                fsh_path,
                type_references.len()
            );

            rules.push(Box::new(TypeRule {
                path: fsh_path,
                types: type_references,
            }));

            elem.mark_processed("type");
        }

        Ok(rules)
    }
}

impl TypeExtractor {
    /// Convert FHIR TypeRef to FSH TypeReference
    fn convert_type_ref(&self, type_ref: &TypeRef) -> TypeReference {
        let profiles = type_ref.profile.clone().unwrap_or_default();
        let target_profiles = type_ref.target_profile.clone().unwrap_or_default();

        TypeReference {
            type_name: type_ref.code.clone(),
            profiles,
            target_profiles,
        }
    }

    /// Convert ElementDefinition path to FSH path
    fn element_path_to_fsh(path: &str) -> String {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() > 1 {
            parts[1..].join(".")
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ElementDefinition;

    fn create_test_element(path: &str, types: Option<Vec<TypeRef>>) -> ElementDefinition {
        ElementDefinition {
            id: path.to_string(),
            path: path.to_string(),
            slice_name: None,
            min: None,
            max: None,
            type_: types,
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
        }
    }

    #[test]
    fn test_extract_single_type() {
        let extractor = TypeExtractor;
        let types = vec![TypeRef {
            code: "string".to_string(),
            profile: None,
            target_profile: None,
        }];
        let elem = create_test_element("Patient.gender", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("type"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("only"));
        assert!(rule_fsh.contains("string"));
    }

    #[test]
    fn test_extract_multiple_types() {
        let extractor = TypeExtractor;
        let types = vec![
            TypeRef {
                code: "string".to_string(),
                profile: None,
                target_profile: None,
            },
            TypeRef {
                code: "code".to_string(),
                profile: None,
                target_profile: None,
            },
        ];
        let elem = create_test_element("Patient.value[x]", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("only"));
        assert!(rule_fsh.contains("or"));
    }

    #[test]
    fn test_extract_type_with_profile() {
        let extractor = TypeExtractor;
        let types = vec![TypeRef {
            code: "Reference".to_string(),
            profile: None,
            target_profile: Some(vec![
                "http://hl7.org/fhir/StructureDefinition/Patient".to_string(),
            ]),
        }];
        let elem = create_test_element("Observation.subject", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("only"));
        assert!(rule_fsh.contains("Reference"));
    }

    #[test]
    fn test_extract_type_with_multiple_profiles() {
        let extractor = TypeExtractor;
        let types = vec![TypeRef {
            code: "Reference".to_string(),
            profile: None,
            target_profile: Some(vec![
                "http://hl7.org/fhir/StructureDefinition/Patient".to_string(),
                "http://hl7.org/fhir/StructureDefinition/Organization".to_string(),
            ]),
        }];
        let elem = create_test_element("Observation.performer", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("Reference"));
    }

    #[test]
    fn test_extract_no_types() {
        let extractor = TypeExtractor;
        let elem = create_test_element("Patient.identifier", None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_empty_types() {
        let extractor = TypeExtractor;
        let elem = create_test_element("Patient.identifier", Some(vec![]));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_already_processed() {
        let extractor = TypeExtractor;
        let types = vec![TypeRef {
            code: "string".to_string(),
            profile: None,
            target_profile: None,
        }];
        let elem = create_test_element("Patient.gender", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        // Mark as already processed
        processable.mark_processed("type");

        let rules = extractor.extract(&mut processable).unwrap();

        // Should return no rules since already processed
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_codeable_concept_type() {
        let extractor = TypeExtractor;
        let types = vec![TypeRef {
            code: "CodeableConcept".to_string(),
            profile: None,
            target_profile: None,
        }];
        let elem = create_test_element("Observation.code", Some(types));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("CodeableConcept"));
    }
}
