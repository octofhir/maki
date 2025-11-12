//! Caret value rule extractor
//!
//! Extracts metadata rules (^short, ^definition, ^comment) from ElementDefinition

use super::RuleExtractor;
use crate::{
    Result,
    exportable::{CaretValueRule, ExportableRule, FshValue},
    processor::ProcessableElementDefinition,
};
use log::debug;

/// Extracts caret value rules (metadata)
pub struct CaretValueExtractor;

impl RuleExtractor for CaretValueExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        let mut rules: Vec<Box<dyn ExportableRule>> = Vec::new();

        let fsh_path = Self::element_path_to_fsh(&elem.element.path);

        // Extract ^short
        if let Some(short) = &elem.element.short
            && !elem.is_processed("short")
        {
            debug!("Extracting ^short for path {}", fsh_path);

            rules.push(Box::new(CaretValueRule {
                path: if fsh_path.is_empty() {
                    None
                } else {
                    Some(fsh_path.clone())
                },
                caret_path: "short".to_string(),
                value: FshValue::String(short.clone()),
            }));

            elem.mark_processed("short");
        }

        // Extract ^definition
        if let Some(definition) = &elem.element.definition
            && !elem.is_processed("definition")
        {
            debug!("Extracting ^definition for path {}", fsh_path);

            rules.push(Box::new(CaretValueRule {
                path: if fsh_path.is_empty() {
                    None
                } else {
                    Some(fsh_path.clone())
                },
                caret_path: "definition".to_string(),
                value: FshValue::String(definition.clone()),
            }));

            elem.mark_processed("definition");
        }

        // Extract ^comment
        if let Some(comment) = &elem.element.comment
            && !elem.is_processed("comment")
        {
            debug!("Extracting ^comment for path {}", fsh_path);

            rules.push(Box::new(CaretValueRule {
                path: if fsh_path.is_empty() {
                    None
                } else {
                    Some(fsh_path.clone())
                },
                caret_path: "comment".to_string(),
                value: FshValue::String(comment.clone()),
            }));

            elem.mark_processed("comment");
        }

        // Extract ^requirements
        if let Some(requirements) = &elem.element.requirements
            && !elem.is_processed("requirements")
        {
            debug!("Extracting ^requirements for path {}", fsh_path);

            rules.push(Box::new(CaretValueRule {
                path: if fsh_path.is_empty() {
                    None
                } else {
                    Some(fsh_path.clone())
                },
                caret_path: "requirements".to_string(),
                value: FshValue::String(requirements.clone()),
            }));

            elem.mark_processed("requirements");
        }

        Ok(rules)
    }
}

impl CaretValueExtractor {
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

    fn create_test_element(
        path: &str,
        short: Option<String>,
        definition: Option<String>,
        comment: Option<String>,
        requirements: Option<String>,
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
            short,
            definition,
            comment,
            requirements,
            alias: None,
            example: None,
        }
    }

    #[test]
    fn test_extract_short() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient.identifier",
            Some("Patient identifier".to_string()),
            None,
            None,
            None,
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("short"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("^short"));
        assert!(rule_fsh.contains("Patient identifier"));
    }

    #[test]
    fn test_extract_definition() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient.identifier",
            None,
            Some("A unique identifier for this patient".to_string()),
            None,
            None,
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("definition"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("^definition"));
    }

    #[test]
    fn test_extract_comment() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient.active",
            None,
            None,
            Some("This element is a modifier".to_string()),
            None,
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("comment"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("^comment"));
    }

    #[test]
    fn test_extract_requirements() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient.name",
            None,
            None,
            None,
            Some("Need to track patient name for identification".to_string()),
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("requirements"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("^requirements"));
    }

    #[test]
    fn test_extract_multiple_metadata() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient.identifier",
            Some("Patient identifier".to_string()),
            Some("A unique identifier for this patient".to_string()),
            Some("This is required".to_string()),
            None,
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        // Should have 3 rules (short, definition, comment)
        assert_eq!(rules.len(), 3);
        assert!(processable.is_processed("short"));
        assert!(processable.is_processed("definition"));
        assert!(processable.is_processed("comment"));
    }

    #[test]
    fn test_extract_no_metadata() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element("Patient.identifier", None, None, None, None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_root_element_metadata() {
        let extractor = CaretValueExtractor;
        let elem = create_test_element(
            "Patient",
            Some("Patient resource".to_string()),
            None,
            None,
            None,
        );
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        // Root element should have None for path
        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.starts_with("^short"));
    }
}
