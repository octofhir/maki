//! Remove extension URL assignment rules
//!
//! Removes extension.url assignment rules since SUSHI automatically sets these based on the extension definition.

use crate::{
    Result,
    exportable::{AssignmentRule, Exportable},
    lake::ResourceLake,
    optimizer::{OptimizationStats, Optimizer},
};
use log::debug;

/// Removes extension.url assignment rules that SUSHI handles automatically
///
/// This optimizer removes assignment rules for extension.url paths since:
/// - SUSHI automatically sets extension.url based on the extension definition
/// - These assignments are redundant in FSH
///
/// Example transformation:
/// ```fsh
/// // Before:
/// * extension[myExtension].url = "http://example.org/myExtension"
/// * extension[myExtension].valueString = "test"
///
/// // After:
/// * extension[myExtension].valueString = "test"
/// ```
pub struct RemoveExtensionURLAssignmentOptimizer;

impl Optimizer for RemoveExtensionURLAssignmentOptimizer {
    fn name(&self) -> &str {
        "remove-extension-url"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Find all extension.url assignment rules to remove
        let mut indices_to_remove = Vec::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(assign_rule) = rule.as_any().downcast_ref::<AssignmentRule>() {
                // Check if path ends with .url and contains "extension"
                if Self::is_extension_url_path(&assign_rule.path) {
                    debug!(
                        "Removing redundant extension.url assignment: {}",
                        assign_rule.path
                    );
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

impl RemoveExtensionURLAssignmentOptimizer {
    /// Check if path is an extension.url assignment
    fn is_extension_url_path(path: &str) -> bool {
        // Match patterns like:
        // - extension.url
        // - extension[name].url
        // - modifierExtension.url
        // - modifierExtension[name].url
        // - some.path.extension.url
        // - some.path.extension[name].url

        if !path.ends_with(".url") {
            return false;
        }

        // Check for extension patterns
        path.contains("extension[")
            || path.contains("modifierExtension[")
            || path.ends_with("extension.url")
            || path.ends_with("modifierExtension.url")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, FshValue};
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
    async fn test_remove_extension_url_assignment() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add extension.url assignment (should be removed)
        profile.add_rule(Box::new(AssignmentRule {
            path: "extension[myExtension].url".to_string(),
            value: FshValue::String("http://example.org/myExtension".to_string()),
            exactly: false,
        }));

        // Add actual extension value (should remain)
        profile.add_rule(Box::new(AssignmentRule {
            path: "extension[myExtension].valueString".to_string(),
            value: FshValue::String("test".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveExtensionURLAssignmentOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify remaining rule is the valueString assignment
        let remaining = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert_eq!(remaining.path, "extension[myExtension].valueString");
    }

    #[tokio::test]
    async fn test_remove_modifier_extension_url() {
        let mut profile =
            ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        // Add modifierExtension.url assignment (should be removed)
        profile.add_rule(Box::new(AssignmentRule {
            path: "modifierExtension[special].url".to_string(),
            value: FshValue::String("http://example.org/special".to_string()),
            exactly: false,
        }));

        // Add modifierExtension value (should remain)
        profile.add_rule(Box::new(AssignmentRule {
            path: "modifierExtension[special].valueBoolean".to_string(),
            value: FshValue::Boolean(true),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveExtensionURLAssignmentOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 1);

        let remaining = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert_eq!(remaining.path, "modifierExtension[special].valueBoolean");
    }

    #[tokio::test]
    async fn test_keep_non_extension_url_assignments() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add regular url field (NOT extension.url - should remain)
        profile.add_rule(Box::new(AssignmentRule {
            path: "photo.url".to_string(),
            value: FshValue::String("http://example.org/photo.jpg".to_string()),
            exactly: false,
        }));

        // Add another non-extension url (should remain)
        profile.add_rule(Box::new(AssignmentRule {
            path: "link.other.url".to_string(),
            value: FshValue::String("http://example.org/other".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveExtensionURLAssignmentOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 0);
        assert_eq!(profile.rules.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_nested_extension_url() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Bundle".to_string());

        // Add nested extension.url (should be removed)
        profile.add_rule(Box::new(AssignmentRule {
            path: "entry.resource.extension[outer].extension[inner].url".to_string(),
            value: FshValue::String("http://example.org/inner".to_string()),
            exactly: false,
        }));

        // Add nested extension value (should remain)
        profile.add_rule(Box::new(AssignmentRule {
            path: "entry.resource.extension[outer].extension[inner].valueCode".to_string(),
            value: FshValue::String("test".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveExtensionURLAssignmentOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 1);

        let remaining = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert!(remaining.path.ends_with(".valueCode"));
    }

    #[tokio::test]
    async fn test_remove_simple_extension_url() {
        let mut profile =
            ExportableProfile::new("TestProfile".to_string(), "DomainResource".to_string());

        // Add simple extension.url without slice name (should be removed)
        profile.add_rule(Box::new(AssignmentRule {
            path: "extension.url".to_string(),
            value: FshValue::String("http://example.org/ext".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = RemoveExtensionURLAssignmentOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.redundant_rules, 1);
        assert_eq!(profile.rules.len(), 0);
    }
}
