//! Assignment combination optimization
//!
//! Combines multi-path assignments into single composite values:
//! - Coding: system + code + display
//! - Quantity: value + unit + system + code
//!
//! Matches GoFSH's CombineCodingAndQuantityValuesOptimizer behavior

use crate::{
    Result,
    exportable::{AssignmentRule, Exportable, ExportableRule, FshCoding, FshQuantity, FshValue},
    lake::ResourceLake,
    optimizer::{OptimizationStats, Optimizer},
};
use log::debug;
use std::collections::HashMap;

/// Combines related assignment rules into composite values
///
/// This optimizer looks for patterns like:
/// * code.system = "http://example.org"
/// * code.code = #active
///
/// And combines them into:
/// * code = http://example.org#active
///
/// Based on GoFSH's CombineAssignmentsOptimizer
pub struct CombineAssignmentsOptimizer;

impl Optimizer for CombineAssignmentsOptimizer {
    fn name(&self) -> &str {
        "combine-assignments"
    }

    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        _lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        let mut stats = OptimizationStats::new();

        // Get all rules
        let rules = exportable.get_rules_mut();

        // Find combinable component groups
        let component_groups = Self::find_component_groups(rules);

        // Collect indices to remove and new rules to add
        let mut indices_to_remove = Vec::new();
        let mut rules_to_add = Vec::new();

        for (base_path, components) in component_groups.iter() {
            // Try combining as Quantity first (if value is present, it's a Quantity)
            if Self::can_combine_quantity(components) {
                debug!("Combining quantity components for path: {}", base_path);

                if let Some(combined_rule) =
                    Self::combine_quantity_assignments(base_path, components, rules)
                {
                    rules_to_add.push(combined_rule);

                    for &idx in components.values() {
                        indices_to_remove.push(idx);
                    }

                    stats.record_simplification();
                }
            }
            // Try combining as Coding
            else if Self::can_combine_coding(components) {
                debug!("Combining coding components for path: {}", base_path);

                if let Some(combined_rule) =
                    Self::combine_coding_assignments(base_path, components, rules)
                {
                    rules_to_add.push(combined_rule);

                    for &idx in components.values() {
                        indices_to_remove.push(idx);
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

        // Add new combined rules
        for rule in rules_to_add {
            rules.push(rule);
            stats.record_addition();
        }

        Ok(stats)
    }
}

impl CombineAssignmentsOptimizer {
    /// Find potential component groups (Coding/Quantity fields)
    fn find_component_groups(
        rules: &[Box<dyn ExportableRule + Send + Sync>],
    ) -> HashMap<String, HashMap<String, usize>> {
        let mut groups: HashMap<String, HashMap<String, usize>> = HashMap::new();

        for (idx, rule) in rules.iter().enumerate() {
            if let Some(assignment) = rule.as_any().downcast_ref::<AssignmentRule>() {
                // Check for component patterns
                if let Some(dot_pos) = assignment.path.rfind('.') {
                    let base_path = &assignment.path[..dot_pos];
                    let field = &assignment.path[dot_pos + 1..];

                    // Coding fields: system, code, display
                    // Quantity fields: value, unit, system, code
                    if matches!(field, "system" | "code" | "display" | "value" | "unit") {
                        groups
                            .entry(base_path.to_string())
                            .or_default()
                            .insert(field.to_string(), idx);
                    }
                }
            }
        }

        groups
    }

    /// Check if coding components can be combined
    fn can_combine_coding(components: &HashMap<String, usize>) -> bool {
        // Need at least system and code to combine
        components.contains_key("system") && components.contains_key("code")
    }

    /// Combine coding assignments into a single rule
    fn combine_coding_assignments(
        base_path: &str,
        components: &HashMap<String, usize>,
        rules: &[Box<dyn ExportableRule + Send + Sync>],
    ) -> Option<Box<dyn ExportableRule + Send + Sync>> {
        // Extract system value
        let system_idx = components.get("system")?;
        let system_rule = rules.get(*system_idx)?;
        let system_assignment = system_rule.as_any().downcast_ref::<AssignmentRule>()?;
        let system = match &system_assignment.value {
            FshValue::String(s) => Some(s.clone()),
            _ => None,
        }?;

        // Extract code value
        let code_idx = components.get("code")?;
        let code_rule = rules.get(*code_idx)?;
        let code_assignment = code_rule.as_any().downcast_ref::<AssignmentRule>()?;
        let code = match &code_assignment.value {
            FshValue::Code(c) => c.code.clone(),
            FshValue::String(s) => s.clone(),
            _ => return None,
        };

        // Extract optional display value
        let display = components.get("display").and_then(|&idx| {
            rules.get(idx).and_then(|rule| {
                rule.as_any()
                    .downcast_ref::<AssignmentRule>()
                    .and_then(|assignment| match &assignment.value {
                        FshValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
            })
        });

        // Create combined Coding
        let combined_coding = FshCoding {
            system: Some(system),
            code,
            display,
        };

        // Create assignment rule with the combined coding
        Some(Box::new(AssignmentRule {
            path: base_path.to_string(),
            value: FshValue::Coding(combined_coding),
            exactly: false,
        }))
    }

    /// Check if quantity components can be combined
    fn can_combine_quantity(components: &HashMap<String, usize>) -> bool {
        // Need at least value to combine as quantity
        components.contains_key("value")
    }

    /// Combine quantity assignments into a single rule
    fn combine_quantity_assignments(
        base_path: &str,
        components: &HashMap<String, usize>,
        rules: &[Box<dyn ExportableRule + Send + Sync>],
    ) -> Option<Box<dyn ExportableRule + Send + Sync>> {
        // Extract value
        let value_idx = components.get("value")?;
        let value_rule = rules.get(*value_idx)?;
        let value_assignment = value_rule.as_any().downcast_ref::<AssignmentRule>()?;
        let value = match &value_assignment.value {
            FshValue::Decimal(d) => Some(*d),
            FshValue::Integer(i) => Some(*i as f64),
            _ => None,
        };

        // Extract optional unit
        let unit = components.get("unit").and_then(|&idx| {
            rules.get(idx).and_then(|rule| {
                rule.as_any()
                    .downcast_ref::<AssignmentRule>()
                    .and_then(|assignment| match &assignment.value {
                        FshValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
            })
        });

        // Extract optional system
        let system = components.get("system").and_then(|&idx| {
            rules.get(idx).and_then(|rule| {
                rule.as_any()
                    .downcast_ref::<AssignmentRule>()
                    .and_then(|assignment| match &assignment.value {
                        FshValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
            })
        });

        // Extract optional code
        let code = components.get("code").and_then(|&idx| {
            rules.get(idx).and_then(|rule| {
                rule.as_any()
                    .downcast_ref::<AssignmentRule>()
                    .and_then(|assignment| match &assignment.value {
                        FshValue::Code(c) => Some(c.code.clone()),
                        FshValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
            })
        });

        // Create combined Quantity
        let combined_quantity = FshQuantity {
            value,
            unit,
            system,
            code,
        };

        // Create assignment rule with the combined quantity
        Some(Box::new(AssignmentRule {
            path: base_path.to_string(),
            value: FshValue::Quantity(combined_quantity),
            exactly: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableProfile, FshCode};
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
    async fn test_combine_system_and_code() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add separate assignments for system and code
        profile.add_rule(Box::new(AssignmentRule {
            path: "status.system".to_string(),
            value: FshValue::String("http://hl7.org/fhir/status".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "status.code".to_string(),
            value: FshValue::Code(FshCode {
                system: None,
                code: "active".to_string(),
            }),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should combine the two rules into one
        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 2); // Removed system and code
        assert_eq!(stats.rules_added, 1); // Added combined coding
        assert_eq!(profile.rules.len(), 1); // One combined rule remains

        // Verify the combined rule
        let combined_rule = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert_eq!(combined_rule.path, "status");
        match &combined_rule.value {
            FshValue::Coding(coding) => {
                assert_eq!(
                    coding.system,
                    Some("http://hl7.org/fhir/status".to_string())
                );
                assert_eq!(coding.code, "active");
                assert_eq!(coding.display, None);
            }
            _ => panic!("Expected FshValue::Coding"),
        }
    }

    #[tokio::test]
    async fn test_no_combinable_assignments() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add unrelated assignments
        profile.add_rule(Box::new(AssignmentRule {
            path: "active".to_string(),
            value: FshValue::Boolean(true),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 0);
    }

    #[tokio::test]
    async fn test_partial_coding_not_combined() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add only system (missing code - cannot combine)
        profile.add_rule(Box::new(AssignmentRule {
            path: "status.system".to_string(),
            value: FshValue::String("http://hl7.org/fhir/status".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Cannot combine without both system and code
        assert_eq!(stats.simplified_rules, 0);
    }

    #[tokio::test]
    async fn test_combine_with_display() {
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        // Add system, code, and display
        profile.add_rule(Box::new(AssignmentRule {
            path: "gender.system".to_string(),
            value: FshValue::String("http://hl7.org/fhir/administrative-gender".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "gender.code".to_string(),
            value: FshValue::Code(FshCode {
                system: None,
                code: "male".to_string(),
            }),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "gender.display".to_string(),
            value: FshValue::String("Male".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        // Should combine all three
        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 3);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        // Verify the combined rule includes display
        let combined_rule = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        match &combined_rule.value {
            FshValue::Coding(coding) => {
                assert_eq!(
                    coding.system,
                    Some("http://hl7.org/fhir/administrative-gender".to_string())
                );
                assert_eq!(coding.code, "male");
                assert_eq!(coding.display, Some("Male".to_string()));
            }
            _ => panic!("Expected FshValue::Coding"),
        }
    }

    #[tokio::test]
    async fn test_combine_quantity_value_and_unit() {
        let mut profile =
            ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.value".to_string(),
            value: FshValue::Decimal(5.5),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.unit".to_string(),
            value: FshValue::String("mg".to_string()),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 2);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        let combined_rule = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert_eq!(combined_rule.path, "valueQuantity");
        match &combined_rule.value {
            FshValue::Quantity(qty) => {
                assert_eq!(qty.value, Some(5.5));
                assert_eq!(qty.unit, Some("mg".to_string()));
                assert_eq!(qty.system, None);
                assert_eq!(qty.code, None);
            }
            _ => panic!("Expected FshValue::Quantity"),
        }
    }

    #[tokio::test]
    async fn test_combine_quantity_with_system_and_code() {
        let mut profile =
            ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.value".to_string(),
            value: FshValue::Decimal(100.0),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.system".to_string(),
            value: FshValue::String("http://unitsofmeasure.org".to_string()),
            exactly: false,
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.code".to_string(),
            value: FshValue::Code(FshCode {
                system: None,
                code: "mg".to_string(),
            }),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 3);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        let combined_rule = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        assert_eq!(combined_rule.path, "valueQuantity");
        match &combined_rule.value {
            FshValue::Quantity(qty) => {
                assert_eq!(qty.value, Some(100.0));
                assert_eq!(qty.system, Some("http://unitsofmeasure.org".to_string()));
                assert_eq!(qty.code, Some("mg".to_string()));
                assert_eq!(qty.unit, None);
            }
            _ => panic!("Expected FshValue::Quantity"),
        }
    }

    #[tokio::test]
    async fn test_quantity_value_only_not_combined() {
        let mut profile =
            ExportableProfile::new("TestProfile".to_string(), "Observation".to_string());

        profile.add_rule(Box::new(AssignmentRule {
            path: "valueQuantity.value".to_string(),
            value: FshValue::Decimal(42.0),
            exactly: false,
        }));

        let lake = create_test_lake().await;
        let optimizer = CombineAssignmentsOptimizer;

        let stats = optimizer.optimize(&mut profile, &lake).unwrap();

        assert_eq!(stats.simplified_rules, 1);
        assert_eq!(stats.rules_removed, 1);
        assert_eq!(stats.rules_added, 1);
        assert_eq!(profile.rules.len(), 1);

        let combined_rule = profile.rules[0]
            .as_any()
            .downcast_ref::<AssignmentRule>()
            .unwrap();
        match &combined_rule.value {
            FshValue::Quantity(qty) => {
                assert_eq!(qty.value, Some(42.0));
                assert_eq!(qty.unit, None);
                assert_eq!(qty.system, None);
                assert_eq!(qty.code, None);
            }
            _ => panic!("Expected FshValue::Quantity"),
        }
    }
}
