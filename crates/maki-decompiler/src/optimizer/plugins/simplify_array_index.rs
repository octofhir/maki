//! Simplify array indexing optimization
//!
//! Simplifies array notation by removing [0] indices since they're the default.

use crate::{
    exportable::{Exportable, ExportableRule},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;
use regex::Regex;
use std::sync::OnceLock;

/// Simplifies array indexing by removing [0] since it's the default
///
/// This optimizer simplifies paths that use [0] array indexing since:
/// - In FHIR and FSH, [0] is the implicit default
/// - `element[0]` and `element` refer to the same thing
///
/// Example transformation:
/// ```fsh
/// // Before:
/// * name[0].given = "John"
/// * name[0].family = "Doe"
///
/// // After:
/// * name.given = "John"
/// * name.family = "Doe"
/// ```
pub struct SimplifyArrayIndexingOptimizer;

impl Optimizer for SimplifyArrayIndexingOptimizer {
    fn name(&self) -> &str {
        "simplify-array-index"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Simplify paths in all rules
        for rule in rules.iter_mut() {
            if let Some(simplified) = Self::simplify_path(rule) {
                debug!("Simplified array indexing in rule");
                stats.record_simplification();
            }
        }

        Ok(stats)
    }
}

impl SimplifyArrayIndexingOptimizer {
    /// Simplify path by removing [0] indices
    fn simplify_path_string(path: &str) -> Option<String> {
        static REGEX: OnceLock<Regex> = OnceLock::new();
        let re = REGEX.get_or_init(|| {
            // Match [0] but not inside slice names like element[slice0]
            // Only match pure numeric [0] indices
            Regex::new(r"\[0\]").unwrap()
        });

        let simplified = re.replace_all(path, "");
        if simplified != path {
            Some(simplified.to_string())
        } else {
            None
        }
    }

    /// Simplify paths in a rule by removing [0] indices
    fn simplify_path(rule: &mut Box<dyn ExportableRule>) -> Option<()> {
        use crate::exportable::{
            AssignmentRule, CardinalityRule, FlagRule, BindingRule, TypeRule,
            ContainsRule, CaretValueRule, ObeysRule, CardinalityFlagRule,
            MappingRule, AddElementRule,
        };

        let any = rule.as_any_mut();

        // Handle different rule types that have paths
        if let Some(r) = any.downcast_mut::<AssignmentRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<CardinalityRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<FlagRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<BindingRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<TypeRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<ContainsRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<CaretValueRule>() {
            if let Some(path) = &r.path {
                if let Some(simplified) = Self::simplify_path_string(path) {
                    r.path = Some(simplified);
                    return Some(());
                }
            }
            if let Some(simplified) = Self::simplify_path_string(&r.caret_path) {
                r.caret_path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<ObeysRule>() {
            // ObeysRule has Option<String> path
            if let Some(path) = &r.path {
                if let Some(simplified) = Self::simplify_path_string(path) {
                    r.path = Some(simplified);
                    return Some(());
                }
            }
        } else if let Some(r) = any.downcast_mut::<CardinalityFlagRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        } else if let Some(r) = any.downcast_mut::<MappingRule>() {
            // MappingRule has Option<String> path
            if let Some(path) = &r.path {
                if let Some(simplified) = Self::simplify_path_string(path) {
                    r.path = Some(simplified);
                    return Some(());
                }
            }
        } else if let Some(r) = any.downcast_mut::<AddElementRule>() {
            if let Some(simplified) = Self::simplify_path_string(&r.path) {
                r.path = simplified;
                return Some(());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, AssignmentRule, CardinalityRule, FshValue};
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
    async fn test_simplify_array_zero_index() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add rules with [0] indexing
        profile.add_rule(Box::new(AssignmentRule {
            path: "name[0].given".to_string(),
            value: FshValue::String("John".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "name[0].family".to_string(),
            value: FshValue::String("Doe".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = SimplifyArrayIndexingOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 2);

        // Verify paths were simplified
        let rule1 = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule1.path, "name.given");

        let rule2 = profile.rules[1].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule2.path, "name.family");
    }

    #[tokio::test]
    async fn test_keep_non_zero_indices() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add rules with [1] and [2] indexing (should not be simplified)
        profile.add_rule(Box::new(AssignmentRule {
            path: "name[1].given".to_string(),
            value: FshValue::String("Jane".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "name[2].family".to_string(),
            value: FshValue::String("Smith".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = SimplifyArrayIndexingOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 0);

        // Verify paths were NOT changed
        let rule1 = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule1.path, "name[1].given");

        let rule2 = profile.rules[1].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule2.path, "name[2].family");
    }

    #[tokio::test]
    async fn test_simplify_multiple_zero_indices() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Bundle".to_string());

        // Add rule with multiple [0] indices
        profile.add_rule(Box::new(AssignmentRule {
            path: "entry[0].resource.name[0].given".to_string(),
            value: FshValue::String("Test".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = SimplifyArrayIndexingOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);

        // Verify both [0] indices were removed
        let rule = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule.path, "entry.resource.name.given");
    }

    #[tokio::test]
    async fn test_simplify_cardinality_rule() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add cardinality rule with [0]
        profile.add_rule(Box::new(CardinalityRule {
            path: "name[0].given".to_string(),
            min: 1,
            max: "1".to_string(),
        }));

        let lake = create_test_lake().await;
        let optimizer = SimplifyArrayIndexingOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);

        let rule = profile.rules[0].as_any().downcast_ref::<CardinalityRule>().unwrap();
        assert_eq!(rule.path, "name.given");
    }

    #[tokio::test]
    async fn test_keep_slice_names_with_zero() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Slice name containing "0" should NOT be simplified
        profile.add_rule(Box::new(AssignmentRule {
            path: "component[slice0].code.coding.code".to_string(),
            value: FshValue::String("12345".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = SimplifyArrayIndexingOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should not simplify since [slice0] is a slice name, not [0] index
        assert_eq!(stats.simplified_rules, 0);

        let rule = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(rule.path, "component[slice0].code.coding.code");
    }
}
