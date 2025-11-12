//! Contains rule extractor for slicing definitions
//!
//! This extractor handles FHIR slicing, converting slicing entries and their related
//! slices into FSH contains rules. Slicing is one of the most complex aspects of FHIR
//! profiling, allowing profiles to subdivide repeating elements into named slices.

use crate::{
    processor::ProcessableElementDefinition,
    exportable::{ExportableRule, ContainsRule, ContainsItem},
    Result,
};
use super::RuleExtractor;

/// Extractor for contains rules (slicing)
///
/// In FHIR, slicing is represented by:
/// 1. A "slicing entry" element - has slicing definition, no sliceName
/// 2. One or more "slice" elements - have sliceName, same path as entry
///
/// In FSH, this becomes:
/// ```fsh
/// * element contains slice1 0..1 and slice2 0..*
/// ```
pub struct ContainsExtractor;

impl ContainsExtractor {
    /// Extract slicing rules from a collection of elements
    ///
    /// This requires analyzing multiple elements together to group slicing
    /// entries with their related slices.
    pub fn extract_slicing(
        elements: &mut [ProcessableElementDefinition],
    ) -> Result<Vec<ContainsRule>> {
        let mut rules = Vec::new();

        // Find all slicing entries
        for i in 0..elements.len() {
            if elements[i].element.is_slicing_entry() && !elements[i].is_processed("slicing") {
                let path = elements[i].element.path.clone();

                // Find all related slices (same path, with slice names)
                let items = Self::find_related_slices(&path, elements);

                if !items.is_empty() {
                    rules.push(ContainsRule {
                        path: elements[i].element.fsh_path(),
                        items,
                    });

                    // Mark slicing entry as processed
                    elements[i].mark_processed("slicing");
                }
            }
        }

        Ok(rules)
    }

    /// Find all slices related to a slicing entry
    fn find_related_slices(
        path: &str,
        elements: &mut [ProcessableElementDefinition],
    ) -> Vec<ContainsItem> {
        let mut items = Vec::new();

        for elem in elements.iter_mut() {
            // Look for elements with same path and a slice name
            if elem.element.path == path && elem.element.is_slice() {
                if let Some(slice_name) = &elem.element.slice_name {
                    // Check if this is an extension slice with a URL
                    let type_name = Self::get_extension_url(elem);

                    items.push(ContainsItem {
                        name: slice_name.clone(),
                        type_name,
                        min: elem.element.min.unwrap_or(0),
                        max: elem.element.max.clone().unwrap_or_else(|| "*".to_string()),
                    });

                    // Mark the slice element's cardinality as processed since
                    // it's included in the contains rule
                    elem.mark_processed("min");
                    elem.mark_processed("max");
                }
            }
        }

        items
    }

    /// Get extension URL for extension slices
    ///
    /// Extension slices in FHIR have a type with a profile pointing to the extension URL.
    /// In FSH, this becomes: `* extension contains http://example.org/Extension named myExt 0..1`
    fn get_extension_url(elem: &ProcessableElementDefinition) -> Option<String> {
        // Check if this is an extension element
        if !elem.element.path.ends_with(".extension") {
            return None;
        }

        // Look for type with profile
        if let Some(types) = &elem.element.type_ {
            for type_ref in types {
                if type_ref.code == "Extension" {
                    // Get the first profile URL if it exists
                    if let Some(profiles) = &type_ref.profile {
                        if let Some(profile_url) = profiles.first() {
                            return Some(profile_url.clone());
                        }
                    }
                }
            }
        }

        None
    }
}

impl RuleExtractor for ContainsExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        // ContainsExtractor requires analyzing multiple elements together
        // So this single-element extract method is not suitable
        // Use extract_slicing() instead with a collection of elements
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ElementDefinition, common::{Slicing, TypeRef}};

    fn create_test_element(path: &str, slice_name: Option<String>) -> ProcessableElementDefinition {
        ProcessableElementDefinition::new(ElementDefinition {
            id: if let Some(ref name) = slice_name {
                format!("{}:{}", path, name)
            } else {
                path.to_string()
            },
            path: path.to_string(),
            slice_name,
            min: Some(0),
            max: Some("*".to_string()),
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
    fn test_extract_simple_slicing() {
        let mut elements = vec![
            // Slicing entry
            {
                let mut elem = create_test_element("Patient.identifier", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
            // Slice 1
            {
                let mut elem = create_test_element("Patient.identifier", Some("mrn".to_string()));
                elem.element.min = Some(1);
                elem.element.max = Some("1".to_string());
                elem
            },
            // Slice 2
            {
                let mut elem = create_test_element("Patient.identifier", Some("ssn".to_string()));
                elem.element.min = Some(0);
                elem.element.max = Some("1".to_string());
                elem
            },
        ];

        let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].path, "identifier");
        assert_eq!(rules[0].items.len(), 2);
        assert_eq!(rules[0].items[0].name, "mrn");
        assert_eq!(rules[0].items[0].min, 1);
        assert_eq!(rules[0].items[0].max, "1");
        assert_eq!(rules[0].items[1].name, "ssn");

        // Verify FSH output
        let fsh = rules[0].to_fsh();
        assert!(fsh.contains("identifier contains"));
        assert!(fsh.contains("mrn 1..1"));
        assert!(fsh.contains("ssn 0..1"));
    }

    #[test]
    fn test_extract_extension_slicing() {
        let mut elements = vec![
            // Slicing entry
            {
                let mut elem = create_test_element("Patient.extension", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
            // Extension slice with URL
            {
                let mut elem = create_test_element("Patient.extension", Some("race".to_string()));
                elem.element.min = Some(0);
                elem.element.max = Some("1".to_string());
                elem.element.type_ = Some(vec![TypeRef {
                    code: "Extension".to_string(),
                    profile: Some(vec![
                        "http://hl7.org/fhir/us/core/StructureDefinition/us-core-race".to_string(),
                    ]),
                    target_profile: None,
                }]);
                elem
            },
        ];

        let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].items.len(), 1);
        assert_eq!(rules[0].items[0].name, "race");
        assert_eq!(
            rules[0].items[0].type_name,
            Some("http://hl7.org/fhir/us/core/StructureDefinition/us-core-race".to_string())
        );

        // Verify FSH output includes URL
        let fsh = rules[0].to_fsh();
        assert!(fsh.contains("http://hl7.org/fhir/us/core/StructureDefinition/us-core-race"));
        assert!(fsh.contains("named race"));
    }

    #[test]
    fn test_no_slicing_entry() {
        let mut elements = vec![
            // Just a slice without slicing entry (shouldn't happen, but handle gracefully)
            create_test_element("Patient.identifier", Some("mrn".to_string())),
        ];

        let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

        // Should return empty since no slicing entry found
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_slicing_entry_without_slices() {
        let mut elements = vec![
            // Slicing entry but no slices (shouldn't generate rule)
            {
                let mut elem = create_test_element("Patient.identifier", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
        ];

        let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

        // Should return empty since no slices found
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_multiple_slicing_groups() {
        let mut elements = vec![
            // First slicing group: identifier
            {
                let mut elem = create_test_element("Patient.identifier", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
            {
                let mut elem = create_test_element("Patient.identifier", Some("mrn".to_string()));
                elem.element.min = Some(1);
                elem.element.max = Some("1".to_string());
                elem
            },
            // Second slicing group: telecom
            {
                let mut elem = create_test_element("Patient.telecom", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
            {
                let mut elem = create_test_element("Patient.telecom", Some("phone".to_string()));
                elem.element.min = Some(0);
                elem.element.max = Some("*".to_string());
                elem
            },
        ];

        let rules = ContainsExtractor::extract_slicing(&mut elements).unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].path, "identifier");
        assert_eq!(rules[1].path, "telecom");
    }

    #[test]
    fn test_cardinality_marked_processed() {
        let mut elements = vec![
            {
                let mut elem = create_test_element("Patient.identifier", None);
                elem.element.slicing = Some(Slicing {
                    discriminator: None,
                    description: None,
                    ordered: None,
                    rules: None,
                });
                elem
            },
            create_test_element("Patient.identifier", Some("mrn".to_string())),
        ];

        ContainsExtractor::extract_slicing(&mut elements).unwrap();

        // The slice element should have min/max marked as processed
        assert!(elements[1].is_processed("min"));
        assert!(elements[1].is_processed("max"));
    }

    #[test]
    fn test_get_extension_url_for_non_extension() {
        let elem = create_test_element("Patient.identifier", None);
        let url = ContainsExtractor::get_extension_url(&elem);
        assert_eq!(url, None);
    }

    #[test]
    fn test_get_extension_url_with_profile() {
        let mut elem = create_test_element("Patient.extension", Some("race".to_string()));
        elem.element.type_ = Some(vec![TypeRef {
            code: "Extension".to_string(),
            profile: Some(vec!["http://example.org/Extension".to_string()]),
            target_profile: None,
        }]);

        let url = ContainsExtractor::get_extension_url(&elem);
        assert_eq!(url, Some("http://example.org/Extension".to_string()));
    }
}
