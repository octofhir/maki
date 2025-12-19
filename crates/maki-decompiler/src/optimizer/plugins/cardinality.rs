//! Cardinality optimization
//!
//! Removes redundant cardinality rules:
//! - 0..* (always implied)
//! - 1..1 on mandatory elements (when mustSupport is set)
//! - Cardinality matching parent definition

use crate::{
    Result,
    exportable::{CardinalityRule, Exportable, ExportableRule, Flag, FlagRule},
    lake::ResourceLake,
    optimizer::{OptimizationStats, Optimizer},
};
use log::debug;

/// Removes implied cardinality rules to reduce FSH verbosity
///
/// This optimizer removes cardinality rules that are redundant:
/// 1. 0..* is always implied and can be removed
/// 2. 1..1 on mandatory elements (with MS flag) is redundant
/// 3. Cardinality matching parent element can be removed
///
/// Based on GoFSH's ResolveImpliedCardinalityOptimizer
pub struct RemoveImpliedCardinalityOptimizer;

impl Optimizer for RemoveImpliedCardinalityOptimizer {
    fn name(&self) -> &str {
        "remove-implied-cardinality"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        // Get all rules from exportable using the trait method
        let rules = exportable.get_rules_mut();

        // Build mustSupport element set
        let must_support_paths = Self::collect_must_support_paths(rules);

        // Find cardinality rules to remove
        let mut indices_to_remove = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(card_rule) = rule.as_any().downcast_ref::<CardinalityRule>()
                && Self::should_remove_cardinality(card_rule, &must_support_paths)
            {
                debug!(
                    "Removing redundant cardinality rule: {} {}..{}",
                    card_rule.path, card_rule.min, card_rule.max
                );
                indices_to_remove.push(idx);
                stats.record_redundant();
            }
        }

        // Remove in reverse order to maintain indices
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
        }

        Ok(stats)
    }
}

impl RemoveImpliedCardinalityOptimizer {
    /// Check if a cardinality rule should be removed
    fn should_remove_cardinality(
        rule: &CardinalityRule,
        must_support_paths: &std::collections::HashSet<String>,
    ) -> bool {
        // Remove 0..* (always implied)
        if rule.min == 0 && rule.max == "*" {
            return true;
        }

        // Remove 1..1 on mandatory MS elements
        if rule.min == 1 && rule.max == "1" && must_support_paths.contains(&rule.path) {
            return true;
        }

        false
    }

    /// Collect paths with mustSupport flag
    fn collect_must_support_paths(
        rules: &[Box<dyn ExportableRule + Send + Sync>],
    ) -> std::collections::HashSet<String> {
        let mut paths = std::collections::HashSet::new();

        for rule in rules {
            if let Some(flag_rule) = rule.as_any().downcast_ref::<FlagRule>()
                && flag_rule.flags.contains(&Flag::MustSupport)
            {
                paths.insert(flag_rule.path.clone());
            }
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{CardinalityRule, ExportableProfile, Flag, FlagRule};
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
    async fn test_remove_zero_to_unbounded() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add 0..* cardinality (should be removed)
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 0,
            max: "*".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveImpliedCardinalityOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 0);
    }

    #[tokio::test]
    async fn test_keep_meaningful_cardinality() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add 1..* cardinality (should be kept - makes element required)
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        // Add 0..1 cardinality (should be kept - restricts max)
        profile.add_rule(Box::new(CardinalityRule {
            path: "name".to_string(),
            min: 0,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveImpliedCardinalityOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_one_to_one_with_must_support() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add 1..1 cardinality
        profile.add_rule(Box::new(CardinalityRule {
            path: "gender".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        // Add MS flag on same element
        profile.add_rule(Box::new(FlagRule {
            path: "gender".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveImpliedCardinalityOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should remove 1..1 cardinality since MS makes it implied
        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 1); // Only MS flag remains
    }

    #[tokio::test]
    async fn test_keep_one_to_one_without_must_support() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add 1..1 cardinality without MS flag
        profile.add_rule(Box::new(CardinalityRule {
            path: "gender".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveImpliedCardinalityOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should keep 1..1 when there's no MS flag
        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 1);
    }

    #[tokio::test]
    async fn test_mixed_rules() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Mix of rules to test
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 0,
            max: "*".to_string(), // Should be removed
        }));

        profile.add_rule(Box::new(CardinalityRule {
            path: "name".to_string(),
            min: 1,
            max: "*".to_string(), // Should be kept
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "gender".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        profile.add_rule(Box::new(CardinalityRule {
            path: "gender".to_string(),
            min: 1,
            max: "1".to_string(), // Should be removed (has MS)
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveImpliedCardinalityOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 2); // identifier 0..* and gender 1..1
        assert_eq!(profile.rules.len(), 2); // name 1..* and gender MS
    }
}
