//! Combine contains rules optimization
//!
//! Combines separate contains rules on the same path into a single contains rule with multiple items.

use crate::{
    exportable::{Exportable, ExportableRule, ContainsRule},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;
use std::collections::HashMap;

/// Combines multiple contains rules for the same path into one
///
/// This optimizer looks for patterns like:
/// * extension contains SliceA 0..1
/// * extension contains SliceB 0..*
///
/// And combines them into:
/// * extension contains SliceA 0..1 and SliceB 0..*
pub struct CombineContainsRulesOptimizer;

impl Optimizer for CombineContainsRulesOptimizer {
    fn name(&self) -> &str {
        "combine-contains"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Build map of path -> indices of contains rules
        let mut contains_map: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(contains_rule) = rule.as_any().downcast_ref::<ContainsRule>() {
                contains_map
                    .entry(contains_rule.path.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }

        // Find paths with multiple contains rules and collect items to merge
        let mut indices_to_remove = Vec::new();
        let mut items_to_add: HashMap<usize, Vec<crate::exportable::ContainsItem>> = HashMap::new();

        for (path, indices) in contains_map.iter() {
            if indices.len() <= 1 {
                continue;
            }

            debug!("Combining {} contains rules for path: {}", indices.len(), path);

            // Keep the first rule, merge others into it
            let first_idx = indices[0];
            let mut collected_items = Vec::new();

            // Collect items from all other rules
            for &other_idx in &indices[1..] {
                if let Some(other_rule) = rules.get(other_idx) {
                    if let Some(other_contains) = other_rule.as_any().downcast_ref::<ContainsRule>() {
                        collected_items.extend(other_contains.items.clone());
                    }
                }

                indices_to_remove.push(other_idx);
                stats.record_removal();
            }

            items_to_add.insert(first_idx, collected_items);
            stats.record_simplification();
        }

        // Now append collected items to first rules
        for (first_idx, items) in items_to_add {
            if let Some(first_rule) = rules.get_mut(first_idx) {
                if let Some(first_contains) = first_rule.as_any_mut().downcast_mut::<ContainsRule>() {
                    first_contains.items.extend(items);
                }
            }
        }

        // Remove duplicate rules in reverse order
        indices_to_remove.sort_unstable();
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, ContainsItem};
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
    async fn test_combine_multiple_contains_same_path() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Add multiple contains rules for same path
        profile.add_rule(Box::new(ContainsRule {
            path: "extension".to_string(),
            items: vec![ContainsItem {
                name: "SliceA".to_string(),
                type_name: Some("http://example.org/extension-a".to_string()),
                min: 0,
                max: "1".to_string(),
            }],
        }));

        profile.add_rule(Box::new(ContainsRule {
            path: "extension".to_string(),
            items: vec![ContainsItem {
                name: "SliceB".to_string(),
                type_name: Some("http://example.org/extension-b".to_string()),
                min: 0,
                max: "*".to_string(),
            }],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineContainsRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify combined rule has both items
        let combined_rule = profile.rules[0].as_any().downcast_ref::<ContainsRule>().unwrap();
        assert_eq!(combined_rule.path, "extension");
        assert_eq!(combined_rule.items.len(), 2);
        assert_eq!(combined_rule.items[0].name, "SliceA");
        assert_eq!(combined_rule.items[1].name, "SliceB");
    }

    #[tokio::test]
    async fn test_no_combination_different_paths() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Add contains rules for different paths
        profile.add_rule(Box::new(ContainsRule {
            path: "extension".to_string(),
            items: vec![ContainsItem {
                name: "SliceA".to_string(),
                type_name: Some("http://example.org/extension-a".to_string()),
                min: 0,
                max: "1".to_string(),
            }],
        }));

        profile.add_rule(Box::new(ContainsRule {
            path: "modifierExtension".to_string(),
            items: vec![ContainsItem {
                name: "SliceB".to_string(),
                type_name: Some("http://example.org/extension-b".to_string()),
                min: 0,
                max: "*".to_string(),
            }],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineContainsRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 0);
        assert_eq!(stats.rules_removed, 0);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_combine_three_contains_rules() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Bundle".to_string());

        // Add three contains rules for same path
        for (name, url) in [
            ("Entry1", "http://example.org/entry-1"),
            ("Entry2", "http://example.org/entry-2"),
            ("Entry3", "http://example.org/entry-3"),
        ] {
            profile.add_rule(Box::new(ContainsRule {
                path: "entry".to_string(),
                items: vec![ContainsItem {
                    name: name.to_string(),
                    type_name: Some(url.to_string()),
                    min: 0,
                    max: "1".to_string(),
                }],
            }));
        }

        let lake = create_test_lake().await;
        let optimizer = CombineContainsRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 2);
        assert_eq!(profile.rules.len(), 1);

        // Verify all three items combined
        let combined_rule = profile.rules[0].as_any().downcast_ref::<ContainsRule>().unwrap();
        assert_eq!(combined_rule.items.len(), 3);
        assert_eq!(combined_rule.items[0].name, "Entry1");
        assert_eq!(combined_rule.items[1].name, "Entry2");
        assert_eq!(combined_rule.items[2].name, "Entry3");
    }
}
