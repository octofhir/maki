//! Cardinality rule extractor
//!
//! Extracts min..max cardinality constraints from ElementDefinition

use super::RuleExtractor;
use crate::{
    Result,
    exportable::{CardinalityRule, ExportableRule},
    processor::ProcessableElementDefinition,
};
use log::debug;

/// Extracts cardinality rules (min..max)
pub struct CardinalityExtractor;

impl RuleExtractor for CardinalityExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        let mut rules: Vec<Box<dyn ExportableRule>> = Vec::new();

        // Check if cardinality has been set and is different from parent
        if let (Some(min), Some(max)) = (elem.element.min, &elem.element.max) {
            // Skip if already processed
            if elem.is_processed("min") || elem.is_processed("max") {
                return Ok(rules);
            }

            // Create FSH path from element path
            let fsh_path = Self::element_path_to_fsh(&elem.element.path);

            debug!(
                "Extracting cardinality {}..{} for path {}",
                min, max, fsh_path
            );

            rules.push(Box::new(CardinalityRule {
                path: fsh_path,
                min,
                max: max.clone(),
            }));

            // Mark properties as processed
            elem.mark_processed("min");
            elem.mark_processed("max");
        }

        Ok(rules)
    }
}

impl CardinalityExtractor {
    /// Convert ElementDefinition path to FSH path
    ///
    /// Examples:
    /// - "Patient.identifier" → "identifier"
    /// - "Patient.name.given" → "name.given"
    /// - "Patient" → (skip root)
    fn element_path_to_fsh(path: &str) -> String {
        // Split by '.' and skip first element (resource type)
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() > 1 {
            parts[1..].join(".")
        } else {
            // Root element - no path
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ElementDefinition;

    fn create_test_element(path: &str, min: Option<u32>, max: Option<String>) -> ElementDefinition {
        ElementDefinition {
            id: path.to_string(),
            path: path.to_string(),
            slice_name: None,
            min,
            max,
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
        }
    }

    #[test]
    fn test_extract_cardinality_0_to_1() {
        let extractor = CardinalityExtractor;
        let elem = create_test_element("Patient.identifier", Some(0), Some("1".to_string()));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("min"));
        assert!(processable.is_processed("max"));
    }

    #[test]
    fn test_extract_cardinality_1_to_many() {
        let extractor = CardinalityExtractor;
        let elem = create_test_element("Patient.name", Some(1), Some("*".to_string()));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        // Verify it's a cardinality rule
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("1..*"));
    }

    #[test]
    fn test_extract_no_cardinality() {
        let extractor = CardinalityExtractor;
        let elem = create_test_element("Patient.identifier", None, None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_path_conversion() {
        assert_eq!(
            CardinalityExtractor::element_path_to_fsh("Patient.identifier"),
            "identifier"
        );
        assert_eq!(
            CardinalityExtractor::element_path_to_fsh("Patient.name.given"),
            "name.given"
        );
        assert_eq!(CardinalityExtractor::element_path_to_fsh("Patient"), "");
    }

    #[test]
    fn test_extract_already_processed() {
        let extractor = CardinalityExtractor;
        let elem = create_test_element("Patient.identifier", Some(0), Some("1".to_string()));
        let mut processable = ProcessableElementDefinition::new(elem);

        // Mark as already processed
        processable.mark_processed("min");
        processable.mark_processed("max");

        let rules = extractor.extract(&mut processable).unwrap();

        // Should return no rules since already processed
        assert_eq!(rules.len(), 0);
    }
}
