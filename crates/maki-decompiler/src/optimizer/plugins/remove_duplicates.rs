//! Remove duplicate rules
//!
//! Identifies and removes duplicate rules that have identical paths and values.
//! This can happen when extracting rules from FHIR resources with redundant constraints.

use crate::{
    exportable::{Exportable, ExportableRule},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;
use std::collections::HashSet;

/// Removes duplicate rules from exportables
///
/// This optimizer identifies rules with identical FSH representation and removes
/// duplicates, keeping only the first occurrence.
pub struct RemoveDuplicateRulesOptimizer;

impl Optimizer for RemoveDuplicateRulesOptimizer {
    fn name(&self) -> &str {
        "remove-duplicates"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Track seen rules by their FSH representation
        let mut seen = HashSet::new();
        let mut indices_to_remove = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            let fsh = rule.to_fsh();

            if seen.contains(&fsh) {
                debug!("Removing duplicate rule: {}", fsh);
                indices_to_remove.push(idx);
                stats.record_redundant();
            } else {
                seen.insert(fsh);
            }
        }

        // Remove duplicates in reverse order
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
        }

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, CardinalityRule, FlagRule, Flag};
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
    async fn test_remove_duplicate_cardinality() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add duplicate cardinality rules
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(CardinalityRule {
            path: "name".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveDuplicateRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1); // One duplicate removed
        assert_eq!(profile.rules.len(), 2);   // Two unique rules remain
    }

    #[tokio::test]
    async fn test_no_duplicates() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

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
        let optimizer = RemoveDuplicateRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_duplicates() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add same rule three times
        for _ in 0..3 {
            profile.add_rule(Box::new(FlagRule {
                path: "name".to_string(),
                flags: vec![Flag::MustSupport],
            }));
        }

        let lake = create_test_lake().await;
        let optimizer = RemoveDuplicateRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 2); // Two duplicates removed
        assert_eq!(profile.rules.len(), 1);   // One remains
    }
}
