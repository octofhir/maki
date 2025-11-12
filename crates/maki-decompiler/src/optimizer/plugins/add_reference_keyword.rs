//! Add Reference keyword optimization
//!
//! Converts string-based reference assignments into FSH Reference syntax:
//! - subject.reference = "Patient/example" → subject = Reference(Patient/example)
//! - Combines display field if present
//! - Removes "#" prefix for contained resources

use crate::{
    exportable::{Exportable, ExportableRule, AssignmentRule, FshValue, FshReference},
    lake::ResourceLake,
    optimizer::{Optimizer, OptimizationStats},
    Result,
};
use log::debug;
use std::collections::HashMap;

/// Adds Reference keyword to reference assignments
///
/// This optimizer looks for patterns like:
/// * subject.reference = "Patient/example"
/// * subject.display = "John Doe"
///
/// And converts them to:
/// * subject = Reference(Patient/example) "John Doe"
///
/// Based on GoFSH's AddReferenceKeywordOptimizer
pub struct AddReferenceKeywordOptimizer;

impl Optimizer for AddReferenceKeywordOptimizer {
    fn name(&self) -> &str {
        "add-reference-keyword"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        let rules = exportable.get_rules_mut();

        // Find reference assignments (*.reference = "...")
        let reference_groups = Self::find_reference_groups(rules);

        // Collect indices to remove and new rules to add
        let mut indices_to_remove = Vec::new();
        let mut rules_to_add = Vec::new();

        for (base_path, components) in reference_groups.iter() {
            if let Some(ref_idx) = components.get("reference") {
                debug!("Converting reference assignment for path: {}", base_path);

                // Extract reference value and optional display
                if let Some(reference_rule) = Self::create_reference_rule(base_path, components, rules) {
                    rules_to_add.push(reference_rule);

                    // Mark original rules for removal
                    indices_to_remove.push(*ref_idx);
                    if let Some(&display_idx) = components.get("display") {
                        indices_to_remove.push(display_idx);
                    }

                    stats.record_simplification();
                }
            }
        }

        // Remove old rules in reverse order
        indices_to_remove.sort_unstable();
        for idx in indices_to_remove.into_iter().rev() {
            rules.remove(idx);
            stats.record_removal();
        }

        // Add new reference rules
        for rule in rules_to_add {
            rules.push(rule);
            stats.record_addition();
        }

        Ok(stats)
    }
}

impl AddReferenceKeywordOptimizer {
    /// Find potential reference groups (reference, display fields)
    fn find_reference_groups(
        rules: &[Box<dyn ExportableRule>],
    ) -> HashMap<String, HashMap<String, usize>> {
        let mut groups: HashMap<String, HashMap<String, usize>> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(assignment) = rule.as_any().downcast_ref::<AssignmentRule>() {
                // Check for reference field patterns (e.g., "subject.reference", "subject.display")
                if let Some(dot_pos) = assignment.path.rfind('.') {
                    let base_path = &assignment.path[..dot_pos];
                    let field = &assignment.path[dot_pos + 1..];

                    if matches!(field, "reference" | "display") {
                        groups
                            .entry(base_path.to_string())
                            .or_insert_with(HashMap::new)
                            .insert(field.to_string(), idx);
                    }
                }
            }
        }

        groups
    }

    /// Create a Reference rule from reference assignment(s)
    fn create_reference_rule(
        base_path: &str,
        components: &HashMap<String, usize>,
        rules: &[Box<dyn ExportableRule>],
    ) -> Option<Box<dyn ExportableRule>> {
        // Extract reference value
        let ref_idx = components.get("reference")?;
        let ref_rule = rules.get(*ref_idx)?;
        let ref_assignment = ref_rule.as_any().downcast_ref::<AssignmentRule>()?;

        let reference = match &ref_assignment.value {
            FshValue::String(s) => {
                // Remove "#" prefix for contained resources
                s.strip_prefix('#').unwrap_or(s).to_string()
            }
            _ => return None,
        };

        // Extract optional display value
        let display = components.get("display").and_then(|&idx| {
            rules.get(idx).and_then(|rule| {
                rule.as_any().downcast_ref::<AssignmentRule>().and_then(|assignment| {
                    match &assignment.value {
                        FshValue::String(s) => Some(s.clone()),
                        _ => None,
                    }
                })
            })
        });

        // Create Reference value
        let reference_value = FshReference {
            reference,
            display,
        };

        // Create assignment rule with the Reference
        Some(Box::new(AssignmentRule {
            path: base_path.to_string(),
            value: FshValue::Reference(reference_value),
            exactly: false,
        }))
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
    async fn test_convert_reference_without_display() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add reference assignment
        profile.add_rule(Box::new(AssignmentRule {
            path: "subject.reference".to_string(),
            value: FshValue::String("Patient/example".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = AddReferenceKeywordOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should convert to Reference
        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 1);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify the reference rule
        let ref_rule = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(ref_rule.path, "subject");
        match &ref_rule.value {
            FshValue::Reference(reference) => {
                assert_eq!(reference.reference, "Patient/example");
                assert_eq!(reference.display, None);
            }
            _ => panic!("Expected FshValue::Reference"),
        }
    }

    #[tokio::test]
    async fn test_convert_reference_with_display() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add reference and display assignments
        profile.add_rule(Box::new(AssignmentRule {
            path: "subject.reference".to_string(),
            value: FshValue::String("Patient/example".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "subject.display".to_string(),
            value: FshValue::String("John Doe".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = AddReferenceKeywordOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should combine both into Reference
        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 2);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify the reference rule with display
        let ref_rule = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        assert_eq!(ref_rule.path, "subject");
        match &ref_rule.value {
            FshValue::Reference(reference) => {
                assert_eq!(reference.reference, "Patient/example");
                assert_eq!(reference.display, Some("John Doe".to_string()));
            }
            _ => panic!("Expected FshValue::Reference"),
        }
    }

    #[tokio::test]
    async fn test_remove_hash_prefix_for_contained() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add contained reference (with # prefix)
        profile.add_rule(Box::new(AssignmentRule {
            path: "subject.reference".to_string(),
            value: FshValue::String("#contained-patient".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = AddReferenceKeywordOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify # prefix is removed
        let ref_rule = profile.rules[0].as_any().downcast_ref::<AssignmentRule>().unwrap();
        match &ref_rule.value {
            FshValue::Reference(reference) => {
                assert_eq!(reference.reference, "contained-patient");
            }
            _ => panic!("Expected FshValue::Reference"),
        }
    }

    #[tokio::test]
    async fn test_no_reference_assignments() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add non-reference assignment
        profile.add_rule(Box::new(AssignmentRule {
            path: "active".to_string(),
            value: FshValue::Boolean(true),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = AddReferenceKeywordOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should not modify anything
        assert_eq!(stats.simplified_rules, 0);
        assert_eq!(profile.rules.len(), 1);
    }
}
