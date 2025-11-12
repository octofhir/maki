//! Remove implied 0..0 cardinality rules
//!
//! Removes value[x]/extension 0..0 rules that SUSHI automatically applies in Extension definitions.
//! EXACT implementation matching GoFSH's RemoveImpliedZeroZeroCardRulesOptimizer.ts

use crate::{
    exportable::{Exportable, CardinalityRule},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;

/// Removes 0..0 cardinality rules that SUSHI applies automatically
///
/// This optimizer removes cardinality rules for extension and value[x] paths when:
/// - An extension path is 0..0 AND there are sibling value[x] paths that aren't 0..0
/// - A value[x] path is 0..0 AND there are sibling extension paths that aren't 0..0
///
/// SUSHI automatically constrains extension to 0..0 when value rules exist, and vice versa.
///
/// EXACT implementation matching GoFSH's RemoveImpliedZeroZeroCardRulesOptimizer
pub struct RemoveZeroZeroCardRulesOptimizer;

impl Optimizer for RemoveZeroZeroCardRulesOptimizer {
    fn name(&self) -> &str {
        "remove-zero-zero-card"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Collect all cardinality rules for analysis
        let card_rules: Vec<(usize, String, String)> = rules
            .iter()
            .enumerate()
            .filter_map(|(idx, rule)| {
                rule.as_any().downcast_ref::<CardinalityRule>().map(|cr| {
                    (idx, cr.path.clone(), cr.max.clone())
                })
            })
            .collect();

        // Find rules to remove based on GoFSH logic
        let mut indices_to_remove = Vec::new();

        for (idx, path, max) in card_rules.iter() {
            // Only consider 0..0 rules
            if max != "0" {
                continue;
            }

            if Self::should_remove_rule(path, &card_rules) {
                debug!("Removing implied 0..0 cardinality rule for path: {}", path);
                indices_to_remove.push(*idx);
                stats.record_redundant();
            }
        }

        // Remove in reverse order to maintain correct indices
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
        }

        Ok(stats)
    }
}

impl RemoveZeroZeroCardRulesOptimizer {
    /// Determine if a 0..0 rule should be removed (EXACT GoFSH logic)
    fn should_remove_rule(path: &str, all_card_rules: &[(usize, String, String)]) -> bool {
        // Case 1: rule is extension path with sibling value paths that aren't 0..0
        if path.ends_with("extension") {
            let value_prefix = path.replace("extension", "value");
            if all_card_rules.iter().any(|(_, r_path, r_max)| {
                r_path.starts_with(&value_prefix) && r_max != "0"
            }) {
                return true;
            }
        }

        // Case 2: rule is value[x] path with sibling extension paths that aren't 0..0
        if path.ends_with("value[x]") {
            let extension_prefix = path.replace("value[x]", "extension");
            if all_card_rules.iter().any(|(_, r_path, r_max)| {
                r_path.starts_with(&extension_prefix) && r_max != "0"
            }) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::ExportableProfile;
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
    async fn test_remove_extension_zero_zero_with_value_sibling() {
        let mut extension = ExportableProfile::new("TestExtension".to_string(), "Extension".to_string());

        // extension 0..0 (should be removed because value[x] is NOT 0..0)
        extension.add_rule(Box::new(CardinalityRule {
            path: "extension".to_string(),
            min: 0,
            max: "0".to_string(),
        }));

        // value[x] 1..1 (sibling that's not 0..0)
        extension.add_rule(Box::new(CardinalityRule {
            path: "value[x]".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveZeroZeroCardRulesOptimizer;

        let stats = optimizer.optimize(&mut extension, &lake).unwrap();

        // Should remove extension 0..0 because value[x] exists and isn't 0..0
        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(extension.rules.len(), 1);

        // Remaining rule should be value[x]
        let remaining = extension.rules[0].as_any().downcast_ref::<CardinalityRule>().unwrap();
        assert_eq!(remaining.path, "value[x]");
    }

    #[tokio::test]
    async fn test_remove_value_zero_zero_with_extension_sibling() {
        let mut extension = ExportableProfile::new("TestExtension".to_string(), "Extension".to_string());

        // value[x] 0..0 (should be removed because extension is NOT 0..0)
        extension.add_rule(Box::new(CardinalityRule {
            path: "value[x]".to_string(),
            min: 0,
            max: "0".to_string(),
        }));

        // extension contains something (sibling that's not 0..0)
        extension.add_rule(Box::new(CardinalityRule {
            path: "extension[custom]".to_string(),
            min: 0,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveZeroZeroCardRulesOptimizer;

        let stats = optimizer.optimize(&mut extension, &lake).unwrap();

        // Should remove value[x] 0..0 because extension exists and isn't 0..0
        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(extension.rules.len(), 1);

        // Remaining rule should be extension[custom]
        let remaining = extension.rules[0].as_any().downcast_ref::<CardinalityRule>().unwrap();
        assert_eq!(remaining.path, "extension[custom]");
    }

    #[tokio::test]
    async fn test_keep_zero_zero_without_non_zero_sibling() {
        let mut extension = ExportableProfile::new("TestExtension".to_string(), "Extension".to_string());

        // Both extension and value[x] are 0..0 - keep both (no non-zero sibling)
        extension.add_rule(Box::new(CardinalityRule {
            path: "extension".to_string(),
            min: 0,
            max: "0".to_string(),
        }));

        extension.add_rule(Box::new(CardinalityRule {
            path: "value[x]".to_string(),
            min: 0,
            max: "0".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveZeroZeroCardRulesOptimizer;

        let stats = optimizer.optimize(&mut extension, &lake).unwrap();

        // Should NOT remove either (no non-zero siblings)
        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(extension.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_keep_zero_zero_on_non_extension_value_paths() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // 0..0 on regular path (not extension/value[x]) - keep it
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 0,
            max: "0".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveZeroZeroCardRulesOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should NOT remove (not an extension/value[x] path)
        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 1);
    }
}
