//! Flag rule extractor
//!
//! Extracts flag rules (MS, ?!, SU, D, N, TU) from ElementDefinition

use super::RuleExtractor;
use crate::{
    Result,
    exportable::{ExportableRule, Flag, FlagRule},
    processor::ProcessableElementDefinition,
};
use log::debug;

/// Extracts flag rules (MS, ?!, SU, etc.)
pub struct FlagExtractor;

impl RuleExtractor for FlagExtractor {
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>> {
        let mut rules: Vec<Box<dyn ExportableRule>> = Vec::new();
        let mut flags = Vec::new();

        // MustSupport (MS)
        if let Some(true) = elem.element.must_support
            && !elem.is_processed("mustSupport")
        {
            flags.push(Flag::MustSupport);
            elem.mark_processed("mustSupport");
        }

        // Is Modifier (?!)
        if let Some(true) = elem.element.is_modifier
            && !elem.is_processed("isModifier")
        {
            flags.push(Flag::Modifier);
            elem.mark_processed("isModifier");
        }

        // Is Summary (SU)
        if let Some(true) = elem.element.is_summary
            && !elem.is_processed("isSummary")
        {
            flags.push(Flag::Summary);
            elem.mark_processed("isSummary");
        }

        // Note: D, N, TU flags come from StandardsStatus which we'd need to add to ElementDefinition
        // For now, we only handle MS, ?!, SU

        // Create rule if we have any flags
        if !flags.is_empty() {
            let fsh_path = Self::element_path_to_fsh(&elem.element.path);

            debug!("Extracting flags {:?} for path {}", flags, fsh_path);

            rules.push(Box::new(FlagRule {
                path: fsh_path,
                flags,
            }));
        }

        Ok(rules)
    }
}

impl FlagExtractor {
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
        must_support: Option<bool>,
        is_modifier: Option<bool>,
        is_summary: Option<bool>,
    ) -> ElementDefinition {
        ElementDefinition {
            id: path.to_string(),
            path: path.to_string(),
            slice_name: None,
            min: None,
            max: None,
            type_: None,
            must_support,
            is_modifier,
            is_summary,
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
    fn test_extract_must_support() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.identifier", Some(true), None, None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("mustSupport"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("MS"));
    }

    #[test]
    fn test_extract_modifier() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.active", None, Some(true), None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("isModifier"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("?!"));
    }

    #[test]
    fn test_extract_summary() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.name", None, None, Some(true));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("isSummary"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("SU"));
    }

    #[test]
    fn test_extract_multiple_flags() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.identifier", Some(true), None, Some(true));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(processable.is_processed("mustSupport"));
        assert!(processable.is_processed("isSummary"));

        let rule_fsh = rules[0].to_fsh();
        assert!(rule_fsh.contains("MS"));
        assert!(rule_fsh.contains("SU"));
    }

    #[test]
    fn test_extract_no_flags() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.identifier", None, None, None);
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_extract_false_flags() {
        let extractor = FlagExtractor;
        let elem = create_test_element("Patient.identifier", Some(false), Some(false), Some(false));
        let mut processable = ProcessableElementDefinition::new(elem);

        let rules = extractor.extract(&mut processable).unwrap();

        // False flags should not generate rules
        assert_eq!(rules.len(), 0);
    }
}
