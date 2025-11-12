//! Remove choice slicing rules optimization
//!
//! Removes redundant slicing rules for choice types (value[x]) since SUSHI handles these automatically.

use crate::{
    exportable::{Exportable, ExportableRule, ContainsRule},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;

/// Removes redundant slicing rules for choice types
///
/// This optimizer removes ContainsRule definitions for choice type (value[x]) slices since:
/// - SUSHI automatically creates slicing for choice types
/// - When you constrain value[x] to a specific type like valueString, SUSHI handles the slicing
/// - Explicit slicing rules for choice types are redundant
///
/// Example transformation:
/// ```fsh
/// // Before:
/// * value[x] contains valueString 0..1
/// * valueString = "test"
///
/// // After:
/// * valueString = "test"
/// ```
pub struct RemoveChoiceSlicingRulesOptimizer;

impl Optimizer for RemoveChoiceSlicingRulesOptimizer {
    fn name(&self) -> &str {
        "remove-choice-slicing"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Find all choice slicing rules to remove
        let mut indices_to_remove = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(contains_rule) = rule.as_any().downcast_ref::<ContainsRule>() {
                // Check if this is a choice type slicing rule
                if Self::is_choice_type_slicing(&contains_rule.path, &contains_rule.items) {
                    debug!("Removing choice type slicing rule for path: {}", contains_rule.path);
                    indices_to_remove.push(idx);
                    stats.record_redundant();
                }
            }
        }

        // Remove in reverse order to maintain correct indices
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
        }

        Ok(stats)
    }
}

impl RemoveChoiceSlicingRulesOptimizer {
    /// Check if this is a choice type slicing rule
    fn is_choice_type_slicing(path: &str, items: &[crate::exportable::ContainsItem]) -> bool {
        // Choice types end with [x]
        if !path.ends_with("[x]") {
            return false;
        }

        // Get the base name without [x]
        let base_name = path.strip_suffix("[x]").unwrap_or(path);
        let base_name = base_name.split('.').last().unwrap_or(base_name);

        // Check if all items are choice type variants
        // e.g., for value[x], valid slices are valueString, valueInteger, valueBoolean, etc.
        items.iter().all(|item| {
            // Choice slice names start with the base name followed by a capital letter
            // e.g., value[x] slices: valueString, valueInteger, valueCode
            item.name.starts_with(base_name) &&
            item.name.len() > base_name.len() &&
            item.name.chars().nth(base_name.len()).map_or(false, |c| c.is_uppercase())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, ContainsItem, AssignmentRule, FshValue};
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
    use crate::lake::ResourceLake;
    use std::sync::Arc;

    async fn create_test_lake() -> ResourceLake {
        let options = CanonicalOptions {
            quick_init: true,
            auto_install_core: false,
            ..Default::default()
        };
        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session(vec![FhirRelease::R4]).await.unwrap();
        ResourceLake::new(Arc::new(session))
    }

    #[tokio::test]
    async fn test_remove_value_choice_slicing() {
        let mut extension = ExportableProfile::new("TestExtension".to_string(), "Extension".to_string());

        // Add choice type slicing (should be removed)
        extension.add_rule(Box::new(ContainsRule {
            path: "value[x]".to_string(),
            items: vec![
                ContainsItem {
                    name: "valueString".to_string(),
                    type_name: Some("string".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
                ContainsItem {
                    name: "valueInteger".to_string(),
                    type_name: Some("integer".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
            ],
        }));

        // Add actual value assignment (should remain)
        extension.add_rule(Box::new(AssignmentRule {
            path: "valueString".to_string(),
            value: FshValue::String("test".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveChoiceSlicingRulesOptimizer;

        let stats = optimizer.optimize(&mut extension, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(extension.rules.len(), 1);

        // Verify remaining rule is the value assignment
        let remaining = extension.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(remaining.path, "valueString");
    }

    #[tokio::test]
    async fn test_keep_non_choice_slicing() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Add non-choice slicing (should NOT be removed)
        profile.add_rule(Box::new(ContainsRule {
            path: "component".to_string(),
            items: vec![
                ContainsItem {
                    name: "systolic".to_string(),
                    type_name: None,
                    min: 1,
                    max: "1".to_string(),
                },
                ContainsItem {
                    name: "diastolic".to_string(),
                    type_name: None,
                    min: 1,
                    max: "1".to_string(),
                },
            ],
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveChoiceSlicingRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 1);
    }

    #[tokio::test]
    async fn test_keep_mixed_slicing() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Extension".to_string());

        // Add slicing with non-standard names for value[x] (should NOT be removed)
        profile.add_rule(Box::new(ContainsRule {
            path: "value[x]".to_string(),
            items: vec![
                ContainsItem {
                    name: "customName".to_string(), // Not a standard choice variant
                    type_name: Some("string".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
            ],
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveChoiceSlicingRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should NOT remove because slice name doesn't follow choice pattern
        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_nested_choice_slicing() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Add nested choice type slicing (should be removed)
        profile.add_rule(Box::new(ContainsRule {
            path: "component.value[x]".to_string(),
            items: vec![
                ContainsItem {
                    name: "valueQuantity".to_string(),
                    type_name: Some("Quantity".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
                ContainsItem {
                    name: "valueCodeableConcept".to_string(),
                    type_name: Some("CodeableConcept".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
            ],
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveChoiceSlicingRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 0);
    }

    #[tokio::test]
    async fn test_remove_single_choice_variant() {
        let mut extension = ExportableProfile::new("TestExtension".to_string(), "Extension".to_string());

        // Add choice slicing with single variant (should be removed)
        extension.add_rule(Box::new(ContainsRule {
            path: "value[x]".to_string(),
            items: vec![
                ContainsItem {
                    name: "valueBoolean".to_string(),
                    type_name: Some("boolean".to_string()),
                    min: 0,
                    max: "1".to_string(),
                },
            ],
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveChoiceSlicingRulesOptimizer;

        let stats = optimizer.optimize(&mut extension, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(extension.rules.len(), 0);
    }
}
