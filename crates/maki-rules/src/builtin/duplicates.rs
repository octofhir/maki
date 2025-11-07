//! Duplicate definition detection rules
//!
//! Detects duplicate resource definitions, aliases, and conflicting rules in FSH files.
//!
//! This module implements comprehensive duplicate detection:
//! - Duplicate entity names (Profiles, Extensions, ValueSets, CodeSystems)
//! - Duplicate entity IDs across all resource types
//! - Duplicate/conflicting rules within profiles
//! - Duplicate aliases with different values

use maki_core::cst::ast::{AstNode, Document, Rule};
use maki_core::cst::FshSyntaxNode;
use maki_core::{Diagnostic, SemanticModel, Severity};
use std::collections::HashMap;

/// Rule ID for duplicate definitions (blocking rule - detects name and ID duplicates)
pub const DUPLICATE_DEFINITION: &str = "blocking/duplicate-definition";

/// Rule ID for duplicate entity names specifically
pub const DUPLICATE_ENTITY_NAME: &str = "correctness/duplicate-entity-name";

/// Rule ID for duplicate entity IDs specifically
pub const DUPLICATE_ENTITY_ID: &str = "correctness/duplicate-entity-id";

/// Rule ID for conflicting rules within profiles
pub const DUPLICATE_RULE: &str = "correctness/duplicate-rule";

/// Rule ID for duplicate aliases
pub const DUPLICATE_ALIAS: &str = "correctness/duplicate-alias";

/// Check for duplicate resource definitions (main blocking rule)
/// This checks both entity names and IDs in a single pass
pub fn check_duplicates(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Track resource IDs and names with all their occurrences
    // In FHIR, all entity names share the same namespace, so we track them together
    let mut entity_names: HashMap<String, Vec<(String, FshSyntaxNode)>> = HashMap::new(); // name -> [(resource_type, location)]
    let mut ids: HashMap<String, Vec<(String, FshSyntaxNode)>> = HashMap::new(); // id -> [(resource_type, location)]

    // Collect all profiles
    for profile in document.profiles() {
        if let Some(name) = profile.name() {
            entity_names
                .entry(name.clone())
                .or_default()
                .push(("Profile".to_string(), profile.syntax().clone()));
        }

        if let Some(id_clause) = profile.id()
            && let Some(id) = id_clause.value()
        {
            ids.entry(id.clone())
                .or_default()
                .push(("Profile".to_string(), profile.syntax().clone()));
        }
    }

    // Collect all extensions
    for extension in document.extensions() {
        if let Some(name) = extension.name() {
            entity_names
                .entry(name.clone())
                .or_default()
                .push(("Extension".to_string(), extension.syntax().clone()));
        }

        if let Some(id_clause) = extension.id()
            && let Some(id) = id_clause.value()
        {
            ids.entry(id.clone())
                .or_default()
                .push(("Extension".to_string(), extension.syntax().clone()));
        }
    }

    // Collect all value sets
    for value_set in document.value_sets() {
        if let Some(name) = value_set.name() {
            entity_names
                .entry(name.clone())
                .or_default()
                .push(("ValueSet".to_string(), value_set.syntax().clone()));
        }

        if let Some(id_clause) = value_set.id()
            && let Some(id) = id_clause.value()
        {
            ids.entry(id.clone())
                .or_default()
                .push(("ValueSet".to_string(), value_set.syntax().clone()));
        }
    }

    // Collect all code systems
    for code_system in document.code_systems() {
        if let Some(name) = code_system.name() {
            entity_names
                .entry(name.clone())
                .or_default()
                .push(("CodeSystem".to_string(), code_system.syntax().clone()));
        }

        if let Some(id_clause) = code_system.id()
            && let Some(id) = id_clause.value()
        {
            ids.entry(id.clone())
                .or_default()
                .push(("CodeSystem".to_string(), code_system.syntax().clone()));
        }
    }

    // Collect all instances
    for instance in document.instances() {
        if let Some(name) = instance.name() {
            entity_names
                .entry(name.clone())
                .or_default()
                .push(("Instance".to_string(), instance.syntax().clone()));
        }

        if let Some(id_clause) = instance.id()
            && let Some(id) = id_clause.value()
        {
            ids.entry(id.clone())
                .or_default()
                .push(("Instance".to_string(), instance.syntax().clone()));
        }
    }

    // Report duplicate names (unified across all resource types)
    for (name, occurrences) in entity_names {
        if occurrences.len() > 1 {
            diagnostics.extend(create_unified_duplicate_name_diagnostics(
                model,
                &name,
                &occurrences,
            ));
        }
    }

    // Report duplicate IDs
    for (id, occurrences) in ids {
        if occurrences.len() > 1 {
            diagnostics.extend(create_duplicate_id_diagnostics(model, &id, &occurrences));
        }
    }

    diagnostics
}

/// Create diagnostics for duplicate entity names (unified across all types)
fn create_unified_duplicate_name_diagnostics(
    model: &SemanticModel,
    name: &str,
    occurrences: &[(String, FshSyntaxNode)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, (res_type, node)) in occurrences.iter().enumerate() {
        let location = model.source_map.node_to_diagnostic_location(
            node,
            &model.source,
            &model.source_file,
        );

        let message = if i == 0 {
            format!(
                "Duplicate entity name '{}' (used by {} {} entities)",
                name,
                occurrences.len(),
                if occurrences.iter().all(|(t, _)| t == res_type) {
                    res_type.as_str()
                } else {
                    "different"
                }
            )
        } else {
            format!(
                "Duplicate entity name '{}' (occurrence {} of {}, type: {})",
                name,
                i + 1,
                occurrences.len(),
                res_type
            )
        };

        diagnostics.push(
            Diagnostic::new(DUPLICATE_DEFINITION, Severity::Error, message, location)
                .with_code(format!("duplicate-{}-name", res_type.to_lowercase())),
        );
    }

    diagnostics
}

/// Create diagnostics for duplicate entity IDs
fn create_duplicate_id_diagnostics(
    model: &SemanticModel,
    id: &str,
    occurrences: &[(String, FshSyntaxNode)],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (i, (res_type, node)) in occurrences.iter().enumerate() {
        let location = model.source_map.node_to_diagnostic_location(
            node,
            &model.source,
            &model.source_file,
        );

        let message = if i == 0 {
            format!(
                "Duplicate resource ID '{}' (used by {} {} entities)",
                id,
                occurrences.len(),
                if occurrences.iter().all(|(t, _)| t == res_type) {
                    res_type.as_str()
                } else {
                    "different"
                }
            )
        } else {
            format!(
                "Duplicate resource ID '{}' (occurrence {} of {}, type: {})",
                id,
                i + 1,
                occurrences.len(),
                res_type
            )
        };

        diagnostics.push(
            Diagnostic::new(DUPLICATE_DEFINITION, Severity::Error, message, location)
                .with_code("duplicate-resource-id".to_string()),
        );
    }

    diagnostics
}

/// Check for conflicting rules within profiles and extensions
/// This detects when the same element path has multiple conflicting rules
pub fn check_duplicate_rules(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles for conflicting rules
    for profile in document.profiles() {
        diagnostics.extend(check_entity_duplicate_rules(model, profile.rules()));
    }

    // Check extensions for conflicting rules
    for extension in document.extensions() {
        diagnostics.extend(check_entity_duplicate_rules(model, extension.rules()));
    }

    diagnostics
}

/// Check for duplicate rules in a specific entity
fn check_entity_duplicate_rules(
    model: &SemanticModel,
    rules: impl Iterator<Item = Rule>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Group rules by their element path
    let mut rules_by_path: HashMap<String, Vec<(Rule, FshSyntaxNode)>> = HashMap::new();

    for rule in rules {
        // Extract path from different rule types
        let path_opt = match &rule {
            Rule::Card(card) => card.path().map(|p| p.syntax().text().to_string()),
            Rule::FixedValue(fixed) => fixed.path().map(|p| p.syntax().text().to_string()),
            Rule::Only(only) => only.path().map(|p| p.syntax().text().to_string()),
            Rule::ValueSet(vs) => vs.path().map(|p| p.syntax().text().to_string()),
            Rule::Flag(flag) => flag.path().map(|p| p.syntax().text().to_string()),
            Rule::Contains(contains) => contains.path().map(|p| p.syntax().text().to_string()),
            Rule::Obeys(obeys) => obeys.path().map(|p| p.syntax().text().to_string()),
            Rule::CaretValue(caret) => caret.element_path().map(|p| p.syntax().text().to_string()),
            _ => None,
        };

        if let Some(path) = path_opt {
            rules_by_path
                .entry(path)
                .or_default()
                .push((rule.clone(), rule.syntax().clone()));
        }
    }

    // Check for conflicts within each path group
    for (path, path_rules) in rules_by_path {
        if let Some(conflict_diags) = check_rule_conflicts(model, &path, &path_rules) {
            diagnostics.extend(conflict_diags);
        }
    }

    diagnostics
}

/// Check if rules on the same path conflict with each other
fn check_rule_conflicts(
    model: &SemanticModel,
    path: &str,
    rules: &[(Rule, FshSyntaxNode)],
) -> Option<Vec<Diagnostic>> {
    if rules.len() <= 1 {
        return None;
    }

    let mut diagnostics = Vec::new();

    // Check for conflicting cardinality rules
    let cardinality_rules: Vec<_> = rules
        .iter()
        .filter_map(|(r, n)| {
            if let Rule::Card(card) = r {
                Some((card, n))
            } else {
                None
            }
        })
        .collect();

    if cardinality_rules.len() > 1 {
        // Extract cardinality text from each rule
        let cardinalities: Vec<String> = cardinality_rules
            .iter()
            .map(|(card, _)| card.syntax().text().to_string().trim().to_string())
            .collect();

        // Check if all cardinalities are the same
        if !all_same(&cardinalities) {
            for (i, (_card, node)) in cardinality_rules.iter().enumerate() {
                let location = model.source_map.node_to_diagnostic_location(
                    node,
                    &model.source,
                    &model.source_file,
                );

                let message = if i == 0 {
                    format!(
                        "Conflicting cardinality rules for path '{}' ({} conflicting definitions)",
                        path,
                        cardinality_rules.len()
                    )
                } else {
                    format!(
                        "Conflicting cardinality rule for path '{}' (occurrence {} of {})",
                        path,
                        i + 1,
                        cardinality_rules.len()
                    )
                };

                diagnostics.push(
                    Diagnostic::new(DUPLICATE_RULE, Severity::Error, message, location)
                        .with_code("conflicting-cardinality".to_string()),
                );
            }
        }
    }

    // Check for conflicting type constraints (only rules)
    let only_rules: Vec<_> = rules
        .iter()
        .filter_map(|(r, n)| {
            if let Rule::Only(only) = r {
                Some((only, n))
            } else {
                None
            }
        })
        .collect();

    if only_rules.len() > 1 {
        // Extract type lists from each rule
        let types: Vec<Vec<String>> = only_rules
            .iter()
            .map(|(only, _)| only.types())
            .collect();

        // Check if all type lists are the same
        if !all_type_lists_same(&types) {
            for (i, (_only, node)) in only_rules.iter().enumerate() {
                let location = model.source_map.node_to_diagnostic_location(
                    node,
                    &model.source,
                    &model.source_file,
                );

                let message = if i == 0 {
                    format!(
                        "Conflicting type constraints for path '{}' ({} conflicting definitions)",
                        path,
                        only_rules.len()
                    )
                } else {
                    format!(
                        "Conflicting type constraint for path '{}' (occurrence {} of {})",
                        path,
                        i + 1,
                        only_rules.len()
                    )
                };

                diagnostics.push(
                    Diagnostic::new(DUPLICATE_RULE, Severity::Error, message, location)
                        .with_code("conflicting-type-constraint".to_string()),
                );
            }
        }
    }

    // Check for conflicting value set bindings
    let valueset_rules: Vec<_> = rules
        .iter()
        .filter_map(|(r, n)| {
            if let Rule::ValueSet(vs) = r {
                Some((vs, n))
            } else {
                None
            }
        })
        .collect();

    if valueset_rules.len() > 1 {
        // Extract value set names
        let valuesets: Vec<String> = valueset_rules
            .iter()
            .filter_map(|(vs, _)| vs.value_set())
            .collect();

        // Check if all value sets are the same
        if !all_same(&valuesets) {
            for (i, (_vs, node)) in valueset_rules.iter().enumerate() {
                let location = model.source_map.node_to_diagnostic_location(
                    node,
                    &model.source,
                    &model.source_file,
                );

                let message = if i == 0 {
                    format!(
                        "Conflicting value set bindings for path '{}' ({} conflicting definitions)",
                        path,
                        valueset_rules.len()
                    )
                } else {
                    format!(
                        "Conflicting value set binding for path '{}' (occurrence {} of {})",
                        path,
                        i + 1,
                        valueset_rules.len()
                    )
                };

                diagnostics.push(
                    Diagnostic::new(DUPLICATE_RULE, Severity::Error, message, location)
                        .with_code("conflicting-valueset-binding".to_string()),
                );
            }
        }
    }

    if diagnostics.is_empty() {
        None
    } else {
        Some(diagnostics)
    }
}

/// Check for duplicate alias definitions
pub fn check_duplicate_aliases(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect all aliases with their names and values
    let mut aliases_by_name: HashMap<String, Vec<(Option<String>, FshSyntaxNode)>> =
        HashMap::new();

    for alias in document.aliases() {
        if let Some(name) = alias.name() {
            let value = alias.value();
            aliases_by_name
                .entry(name)
                .or_default()
                .push((value, alias.syntax().clone()));
        }
    }

    // Check for duplicates
    for (name, occurrences) in aliases_by_name {
        if occurrences.len() > 1 {
            // Check if all values are the same
            let values: Vec<Option<String>> = occurrences.iter().map(|(v, _)| v.clone()).collect();

            let all_same_value = values.windows(2).all(|w| w[0] == w[1]);

            if !all_same_value {
                // Different values - this is an error
                for (i, (value, node)) in occurrences.iter().enumerate() {
                    let location = model.source_map.node_to_diagnostic_location(
                        node,
                        &model.source,
                        &model.source_file,
                    );

                    let value_str = value.as_deref().unwrap_or("(no value)");
                    let message = if i == 0 {
                        format!(
                            "Duplicate alias '{}' with different values (defined {} times)",
                            name,
                            occurrences.len()
                        )
                    } else {
                        format!(
                            "Duplicate alias '{}' = '{}' (occurrence {} of {})",
                            name,
                            value_str,
                            i + 1,
                            occurrences.len()
                        )
                    };

                    diagnostics.push(
                        Diagnostic::new(DUPLICATE_ALIAS, Severity::Error, message, location)
                            .with_code("duplicate-alias-different-values".to_string()),
                    );
                }
            } else {
                // Same value - this is a warning (redundant but not wrong)
                for (_i, (value, node)) in occurrences.iter().enumerate().skip(1) {
                    let location = model.source_map.node_to_diagnostic_location(
                        node,
                        &model.source,
                        &model.source_file,
                    );

                    let value_str = value.as_deref().unwrap_or("(no value)");
                    let message = format!(
                        "Redundant alias '{}' = '{}' (already defined with same value)",
                        name, value_str
                    );

                    diagnostics.push(
                        Diagnostic::new(DUPLICATE_ALIAS, Severity::Warning, message, location)
                            .with_code("redundant-alias".to_string()),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Helper function to check if all strings in a vec are the same
fn all_same(values: &[String]) -> bool {
    if values.is_empty() {
        return true;
    }
    values.windows(2).all(|w| w[0] == w[1])
}

/// Helper function to check if all type lists are the same
fn all_type_lists_same(type_lists: &[Vec<String>]) -> bool {
    if type_lists.is_empty() {
        return true;
    }

    let first = &type_lists[0];
    type_lists.iter().all(|list| {
        list.len() == first.len() && list.iter().zip(first.iter()).all(|(a, b)| a == b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::cst::parse_fsh;
    use std::path::PathBuf;

    fn create_test_model(source: &str) -> SemanticModel {
        let (cst, _, _) = parse_fsh(source);
        let source_map = maki_core::SourceMap::new(source);
        SemanticModel {
            cst,
            resources: Vec::new(),
            symbols: Default::default(),
            aliases: maki_core::semantic::AliasTable::new(),
            references: Vec::new(),
            source_file: PathBuf::from("test.fsh"),
            source_map,
            source: source.to_string(),
            deferred_rules: maki_core::DeferredRuleQueue::new(),
        }
    }

    #[test]
    fn test_duplicate_profile_names() {
        let source = r#"
Profile: MyProfile
Parent: Patient
Id: my-profile-1

Profile: MyProfile
Parent: Patient
Id: my-profile-2
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        assert!(!diagnostics.is_empty(), "Should detect duplicate profile names");
        assert!(diagnostics.iter().any(|d| d.message.contains("MyProfile")));
    }

    #[test]
    fn test_duplicate_entity_ids() {
        let source = r#"
Profile: FirstProfile
Parent: Patient
Id: same-id

Profile: SecondProfile
Parent: Patient
Id: same-id
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        assert!(!diagnostics.is_empty(), "Should detect duplicate IDs");
        assert!(diagnostics.iter().any(|d| d.message.contains("same-id")));
    }

    #[test]
    fn test_no_duplicates() {
        let source = r#"
Profile: Profile1
Parent: Patient
Id: profile-1

Profile: Profile2
Parent: Patient
Id: profile-2
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        assert_eq!(diagnostics.len(), 0, "Should not detect any duplicates");
    }

    #[test]
    fn test_conflicting_cardinality_rules() {
        let source = r#"
Profile: ConflictingProfile
Parent: Patient
* name 1..*
* name 0..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_rules(&model);

        assert!(!diagnostics.is_empty(), "Should detect conflicting cardinality rules");
        assert!(diagnostics.iter().any(|d| d.message.contains("Conflicting")));
    }

    #[test]
    fn test_compatible_rules_same_cardinality() {
        let source = r#"
Profile: CompatibleProfile
Parent: Patient
* name 1..*
* name 1..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_rules(&model);

        // Same cardinality is technically redundant but not conflicting
        // The current implementation treats same values as non-conflicting
        assert_eq!(diagnostics.len(), 0, "Same cardinality should not conflict");
    }

    #[test]
    fn test_duplicate_aliases_different_values() {
        let source = r#"
Alias: $SCT = http://snomed.info/sct
Alias: $SCT = http://different-url.org
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_aliases(&model);

        assert!(!diagnostics.is_empty(), "Should detect duplicate aliases with different values");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error && d.message.contains("SCT"))
        );
    }

    #[test]
    fn test_duplicate_aliases_same_value() {
        let source = r#"
Alias: $SCT = http://snomed.info/sct
Alias: $SCT = http://snomed.info/sct
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_aliases(&model);

        assert!(!diagnostics.is_empty(), "Should detect redundant aliases");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.severity == Severity::Warning && d.message.contains("Redundant"))
        );
    }

    #[test]
    fn test_multiple_entity_types_same_name() {
        let source = r#"
Profile: MyResource
Parent: Patient
Id: my-profile

ValueSet: MyResource
Id: my-valueset
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        // Different entity types can have the same name (different namespaces)
        // But our current implementation treats all entity names in same namespace
        // This is actually correct as FHIR treats all as resource names
        assert!(!diagnostics.is_empty(), "Should detect duplicate names across types");
    }

    #[test]
    fn test_duplicate_extension_names() {
        let source = r#"
Extension: MyExtension
Id: my-extension-1
* value[x] only string

Extension: MyExtension
Id: my-extension-2
* value[x] only integer
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        assert!(!diagnostics.is_empty(), "Should detect duplicate extension names");
        assert!(diagnostics.iter().any(|d| d.message.contains("MyExtension")));
    }

    #[test]
    fn test_conflicting_type_constraints() {
        let source = r#"
Profile: TypeConflict
Parent: Observation
* value[x] only string
* value[x] only integer
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_rules(&model);

        assert!(!diagnostics.is_empty(), "Should detect conflicting type constraints");
        assert!(diagnostics.iter().any(|d| d.message.contains("type")));
    }

    #[test]
    fn test_conflicting_valueset_bindings() {
        let source = r#"
Profile: BindingConflict
Parent: Observation
* code from http://example.org/vs1
* code from http://example.org/vs2
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicate_rules(&model);

        assert!(!diagnostics.is_empty(), "Should detect conflicting value set bindings");
        assert!(diagnostics.iter().any(|d| d.message.contains("value set")));
    }

    #[test]
    fn test_all_same_helper() {
        assert!(all_same(&[]));
        assert!(all_same(&["a".to_string()]));
        assert!(all_same(&["a".to_string(), "a".to_string()]));
        assert!(!all_same(&["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_all_type_lists_same_helper() {
        assert!(all_type_lists_same(&[]));
        assert!(all_type_lists_same(&[vec!["string".to_string()]]));
        assert!(all_type_lists_same(&[
            vec!["string".to_string()],
            vec!["string".to_string()]
        ]));
        assert!(!all_type_lists_same(&[
            vec!["string".to_string()],
            vec!["integer".to_string()]
        ]));
        assert!(!all_type_lists_same(&[
            vec!["string".to_string()],
            vec!["string".to_string(), "integer".to_string()]
        ]));
    }

    #[test]
    fn test_three_way_duplicate() {
        let source = r#"
Profile: TripleDuplicate
Parent: Patient
Id: id-1

Profile: TripleDuplicate
Parent: Patient
Id: id-2

Profile: TripleDuplicate
Parent: Patient
Id: id-3
"#;
        let model = create_test_model(source);
        let diagnostics = check_duplicates(&model);

        // Should report all 3 occurrences
        let name_duplicates: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("TripleDuplicate"))
            .collect();
        assert_eq!(
            name_duplicates.len(),
            3,
            "Should report all 3 duplicate occurrences"
        );
    }
}
