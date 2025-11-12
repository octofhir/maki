//! Combine cardinality and flag rules
//!
//! Merges separate cardinality and flag rules for the same path into a single
//! combined rule: element 0..1 + element MS → element 0..1 MS

use crate::{
    Result,
    exportable::{CardinalityFlagRule, CardinalityRule, Exportable, FlagRule},
    lake::ResourceLake,
    optimizer::{OptimizationStats, Optimizer},
};
use log::debug;
use std::collections::HashMap;

/// Combines cardinality and flag rules for the same path
///
/// This optimizer looks for patterns like:
/// * element 0..1
/// * element MS
///
/// And combines them into:
/// * element 0..1 MS
///
/// Based on GoFSH's CombineCardAndFlagRulesOptimizer
pub struct CombineCardAndFlagRulesOptimizer;

impl Optimizer for CombineCardAndFlagRulesOptimizer {
    fn name(&self) -> &str {
        "combine-card-and-flag"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Build maps of path -> index for cardinality and flag rules
        let mut cardinality_map: HashMap<String, usize> = HashMap::new();
        let mut flag_map: HashMap<String, usize> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(card_rule) = rule.as_any().downcast_ref::<CardinalityRule>() {
                cardinality_map.insert(card_rule.path.clone(), idx);
            } else if let Some(flag_rule) = rule.as_any().downcast_ref::<FlagRule>() {
                flag_map.insert(flag_rule.path.clone(), idx);
            }
        }

        // Find paths that have both cardinality and flag rules
        let mut indices_to_remove = Vec::new();
        let mut rules_to_add = Vec::new();

        for (path, card_idx) in cardinality_map.iter() {
            if let Some(&flag_idx) = flag_map.get(path) {
                debug!("Combining cardinality and flag rules for path: {}", path);

                // Extract cardinality and flag details
                let card_rule = rules[*card_idx]
                    .as_any()
                    .downcast_ref::<CardinalityRule>()
                    .unwrap();
                let flag_rule = rules[flag_idx].as_any().downcast_ref::<FlagRule>().unwrap();

                // Create combined rule
                let combined = Box::new(CardinalityFlagRule {
                    path: path.clone(),
                    min: card_rule.min,
                    max: card_rule.max.clone(),
                    flags: flag_rule.flags.clone(),
                });

                rules_to_add.push(combined);
                indices_to_remove.push(*card_idx);
                indices_to_remove.push(flag_idx);

                stats.record_simplification();
            }
        }

        // Remove old rules in reverse order
        indices_to_remove.sort_unstable();
        indices_to_remove.dedup(); // In case of duplicates
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
            stats.record_removal();
        }

        // Add new combined rules
        for rule in rules_to_add {
            rules.push(rule);
            stats.record_addition();
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, Flag};
    use crate::lake::ResourceLake;
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
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
    async fn test_combine_card_and_flag() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add cardinality and flag for same path
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineCardAndFlagRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should combine into one rule
        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 2);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify the combined rule
        let combined = profile.rules[0]
            .as_any()
            .downcast_ref::<CardinalityFlagRule>()
            .unwrap();
        assert_eq!(combined.path, "identifier");
        assert_eq!(combined.min, 1);
        assert_eq!(combined.max, "*");
        assert_eq!(combined.flags, vec![Flag::MustSupport]);
    }

    #[tokio::test]
    async fn test_no_matching_pairs() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add cardinality only
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        // Add flag for different path
        profile.add_rule(Box::new(FlagRule {
            path: "name".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineCardAndFlagRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should not combine anything
        assert_eq!(stats.simplified_rules, 0);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_combinations() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // First pair
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        // Second pair
        profile.add_rule(Box::new(CardinalityRule {
            path: "name".to_string(),
            min: 0,
            max: "1".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "name".to_string(),
            flags: vec![Flag::Summary],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineCardAndFlagRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should combine both pairs
        assert_eq!(stats.simplified_rules, 2);
        assert_eq!(stats.rules_removed, 4);
        assert_eq!(stats.rules_added, 2);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_flags() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add cardinality and flag with multiple flags
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport, Flag::Summary],
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineCardAndFlagRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify both flags are preserved
        let combined = profile.rules[0]
            .as_any()
            .downcast_ref::<CardinalityFlagRule>()
            .unwrap();
        assert_eq!(combined.flags.len(), 2);
        assert!(combined.flags.contains(&Flag::MustSupport));
        assert!(combined.flags.contains(&Flag::Summary));
    }
}
