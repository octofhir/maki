//! Binding rule extractor
//!
//! Extracts ValueSet binding rules from ElementDefinition

use crate::{
    processor::ProcessableElementDefinition,
    exportable::{ExportableRule, BindingRule, BindingStrength as FshBindingStrength},
    models::BindingStrength as FhirBindingStrength,
    Result,
};
use super::RuleExtractor;
use log::debug;

/// Extracts binding rules (from ValueSet)
pub struct BindingExtractor;

impl RuleExtractor for BindingExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        let mut rules: Vec<Box<dyn ExportableRule>> = Vec::new();

        // Check if binding exists
        if let Some(binding) = &elem.element.binding {
            // Skip if already processed
            if elem.is_processed("binding") {
                return Ok(rules);
            }

            // Must have a value set
            if let Some(value_set) = &binding.value_set {
                let fsh_path = Self::element_path_to_fsh(&elem.element.path);

                // Convert FHIR binding strength to FSH binding strength
                let strength = match binding.strength {
                    FhirBindingStrength::Required => FshBindingStrength::Required,
                    FhirBindingStrength::Extensible => FshBindingStrength::Extensible,
                    FhirBindingStrength::Preferred => FshBindingStrength::Preferred,
                    FhirBindingStrength::Example => FshBindingStrength::Example,
                };

                debug!(
                    "Extracting binding to {} ({:?}) for path {}",
                    value_set, strength, fsh_path
                );

                rules.push(Box::new(BindingRule {
                    path: fsh_path,
                    value_set: value_set.clone(),
                    strength,
                }));

                elem.mark_processed("binding");
            }
        }

        Ok(rules)
    }
}

impl BindingExtractor {
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
    use crate::models::{ElementDefinition, Binding};

    fn create_test_element(
        path: &str,
        binding: Option<Binding>,
    ) -> ElementDefinition {
        ElementDefinition {
            id: path.to_string(),
            path: path.to_string(),
            slice_name: None,
            min: None,
            max: None,
            type_: None,
            must_support: None,
            is_modifier: None,
            is_summary: None,
            binding,
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
    fn test_extract_required_binding() {
        let extractor = BindingExtractor;
        let binding = Binding {
            strength: FhirBindingStrength::Required,
            value_set: Some("http://hl7.org/fhir/ValueSet/administrative-gender".to_string()),
            description: None,
        };
        let elem = create_test_element("Patient.gender", Some(binding));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("binding"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("from"));
        assert!(rule_fsh.contains("required"));
    }

    #[test]
    fn test_extract_extensible_binding() {
        let extractor = BindingExtractor;
        let binding = Binding {
            strength: FhirBindingStrength::Extensible,
            value_set: Some("http://hl7.org/fhir/ValueSet/languages".to_string()),
            description: None,
        };
        let elem = create_test_element("Patient.communication.language", Some(binding));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("extensible"));
    }

    #[test]
    fn test_extract_preferred_binding() {
        let extractor = BindingExtractor;
        let binding = Binding {
            strength: FhirBindingStrength::Preferred,
            value_set: Some("http://hl7.org/fhir/ValueSet/contact-point-use".to_string()),
            description: None,
        };
        let elem = create_test_element("Patient.telecom.use", Some(binding));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("preferred"));
    }

    #[test]
    fn test_extract_example_binding() {
        let extractor = BindingExtractor;
        let binding = Binding {
            strength: FhirBindingStrength::Example,
            value_set: Some("http://hl7.org/fhir/ValueSet/example".to_string()),
            description: None,
        };
        let elem = create_test_element("Patient.identifier.type", Some(binding));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("example"));
    }

    #[test]
    fn test_extract_no_binding() {
        let extractor = BindingExtractor;
        let elem = create_test_element("Patient.identifier", None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_binding_without_value_set() {
        let extractor = BindingExtractor;
        let binding = Binding {
            strength: FhirBindingStrength::Required,
            value_set: None,
            description: Some("A binding without a value set".to_string()),
        };
        let elem = create_test_element("Patient.identifier", Some(binding));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        // Should not create a rule without a value set
        assert_eq!(rules.len(), 0);
    }
}
