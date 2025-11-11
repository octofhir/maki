//! Processable wrappers for FHIR elements
//!
//! Tracks which properties have been processed to avoid duplicate rule extraction

use crate::models::ElementDefinition;
use std::collections::HashSet;

/// Wrapper around ElementDefinition that tracks processed properties
#[derive(Debug)]
pub struct ProcessableElementDefinition {
    pub element: ElementDefinition,
    processed: HashSet<String>,
}

impl ProcessableElementDefinition {
    /// Create a new processable element definition
    pub fn new(element: ElementDefinition) -> Self {
        Self {
            element,
            processed: HashSet::new(),
        }
    }

    /// Mark a property as processed
    pub fn mark_processed(&mut self, property: &str) {
        self.processed.insert(property.to_string());
    }

    /// Check if a property has been processed
    pub fn is_processed(&self, property: &str) -> bool {
        self.processed.contains(property)
    }

    /// Get all processed properties
    pub fn processed_properties(&self) -> &HashSet<String> {
        &self.processed
    }

    /// Clear processed properties
    pub fn clear_processed(&mut self) {
        self.processed.clear();
    }
}

impl AsRef<ElementDefinition> for ProcessableElementDefinition {
    fn as_ref(&self) -> &ElementDefinition {
        &self.element
    }
}

impl From<ElementDefinition> for ProcessableElementDefinition {
    fn from(element: ElementDefinition) -> Self {
        Self::new(element)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_element() -> ElementDefinition {
        ElementDefinition {
            id: "Patient.identifier".to_string(),
            path: "Patient.identifier".to_string(),
            slice_name: None,
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
        }
    }

    #[test]
    fn test_new_processable() {
        let element = create_test_element();
        let processable = ProcessableElementDefinition::new(element);

        assert_eq!(processable.element.id, "Patient.identifier");
        assert_eq!(processable.processed.len(), 0);
    }

    #[test]
    fn test_mark_processed() {
        let element = create_test_element();
        let mut processable = ProcessableElementDefinition::new(element);

        processable.mark_processed("min");
        processable.mark_processed("max");

        assert!(processable.is_processed("min"));
        assert!(processable.is_processed("max"));
        assert!(!processable.is_processed("type"));
    }

    #[test]
    fn test_clear_processed() {
        let element = create_test_element();
        let mut processable = ProcessableElementDefinition::new(element);

        processable.mark_processed("min");
        processable.mark_processed("max");
        assert_eq!(processable.processed.len(), 2);

        processable.clear_processed();
        assert_eq!(processable.processed.len(), 0);
    }

    #[test]
    fn test_from_element_definition() {
        let element = create_test_element();
        let processable: ProcessableElementDefinition = element.into();

        assert_eq!(processable.element.id, "Patient.identifier");
    }
}
