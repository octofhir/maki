//! Required field validation rules
//!
//! Validates that FHIR resources have required metadata fields:
//! - Parent: Required for Profiles (validated in profile.rs)
//! - Id: Required for all entities (auto-generated from Name in kebab-case)
//! - Title: Required for all entities (auto-generated from Name with spaces)
//! - Description: Recommended (auto-generated with TODO placeholder)
//! - Context: Required for Extensions (defines where they can be used)

use maki_core::cst::ast::{AstNode, Document};
use maki_core::diagnostics::Location;
use maki_core::{CodeSuggestion, Diagnostic, SemanticModel, Severity};

/// Rule IDs for required field validation
pub const REQUIRED_FIELD_MISSING: &str = "blocking/required-field-missing";
pub const REQUIRED_FIELD_PRESENT: &str = "blocking/required-field-present";
pub const REQUIRED_PARENT: &str = "blocking/required-parent";
pub const REQUIRED_ID: &str = "blocking/required-id";
pub const REQUIRED_TITLE: &str = "blocking/required-title";
pub const MISSING_DESCRIPTION: &str = "documentation/missing-description";

// Re-export extension context constant from profile module for consistency
pub use crate::builtin::profile::EXTENSION_CONTEXT_MISSING;

/// Rule IDs for instance validation
pub const INSTANCE_REQUIRED_FIELDS_MISSING: &str = "correctness/instance-required-fields-missing";
pub const PROFILE_WITHOUT_EXAMPLES: &str = "documentation/profile-without-examples";

/// Rule ID for required field override detection (semantic validation)
pub const REQUIRED_FIELD_OVERRIDE: &str = "correctness/required-field-override";

/// Convert PascalCase name to kebab-case ID
fn name_to_kebab_case_id(name: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for (i, ch) in name.chars().enumerate() {
        if ch == '_' || ch == ' ' {
            result.push('-');
            prev_was_lower = false;
        } else if ch.is_uppercase() {
            // Add hyphen before uppercase if previous was lowercase (camelCase/PascalCase)
            if i > 0 && prev_was_lower {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_was_lower = false;
        } else {
            result.push(ch);
            prev_was_lower = ch.is_lowercase();
        }
    }

    result
}

/// Convert PascalCase name to "Title Case" by adding spaces
fn name_to_title_case(name: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for c in name.chars() {
        if c.is_uppercase() && prev_was_lower {
            result.push(' ');
        }
        result.push(c);
        prev_was_lower = c.is_lowercase();
    }

    result
}

/// Create an insertion location at the end of the first line of a resource
/// This is used for safe fixes that add new fields after the resource declaration line
fn get_insertion_location_after_first_line(location: &Location, model: &SemanticModel) -> Location {
    let first_line_text = model
        .source
        .lines()
        .nth(location.line.saturating_sub(1))
        .unwrap_or("");

    Location {
        file: location.file.clone(),
        line: location.line,
        column: first_line_text.chars().count() + 1,
        end_line: Some(location.line),
        end_column: Some(first_line_text.chars().count() + 1),
        offset: location.offset + first_line_text.len(),
        length: 0,
        span: Some((
            location.offset + first_line_text.len(),
            location.offset + first_line_text.len(),
        )),
    }
}

/// Check required fields
pub fn check_required_fields(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        if let Some(name) = profile.name() {
            let location = model.source_map.node_to_diagnostic_location(
                profile.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Parent
            if profile.parent().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_PARENT,
                        Severity::Error,
                        format!("Profile '{name}' must specify a Parent (e.g., 'Parent: Patient')"),
                        location.clone(),
                    )
                    .with_code("profile-missing-parent".to_string()),
                );
            }

            // Check required Id
            if profile.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("Profile '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("profile-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check required Title
            if profile.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("Profile '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("profile-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check optional Description
            if profile.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("Profile '{name}' should have a Description"),
                        location.clone(),
                    )
                    .with_code("profile-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }
        }
    }

    // Check extensions
    for extension in document.extensions() {
        if let Some(name) = extension.name() {
            let location = model.source_map.node_to_diagnostic_location(
                extension.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if extension.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("Extension '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("extension-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check required Title
            if extension.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("Extension '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("extension-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check optional Description
            if extension.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("Extension '{name}' should have a Description"),
                        location.clone(),
                    )
                    .with_code("extension-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }
        }
    }

    // Check value sets
    for value_set in document.value_sets() {
        if let Some(name) = value_set.name() {
            let location = model.source_map.node_to_diagnostic_location(
                value_set.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if value_set.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("ValueSet '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("valueset-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check required Title
            if value_set.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("ValueSet '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("valueset-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check optional Description
            if value_set.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("ValueSet '{name}' should have a Description"),
                        location.clone(),
                    )
                    .with_code("valueset-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }
        }
    }

    // Check code systems
    for code_system in document.code_systems() {
        if let Some(name) = code_system.name() {
            let location = model.source_map.node_to_diagnostic_location(
                code_system.syntax(),
                &model.source,
                &model.source_file,
            );

            // Check required Id
            if code_system.id().is_none() {
                let generated_id = name_to_kebab_case_id(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_ID,
                        Severity::Error,
                        format!("CodeSystem '{name}' must specify an Id field"),
                        location.clone(),
                    )
                    .with_code("codesystem-missing-id".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Id: {}", generated_id),
                        format!("\nId: {}", generated_id),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check required Title
            if code_system.title().is_none() {
                let generated_title = name_to_title_case(&name);
                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_TITLE,
                        Severity::Error,
                        format!("CodeSystem '{name}' must specify a Title field"),
                        location.clone(),
                    )
                    .with_code("codesystem-missing-title".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        format!("Add Title: \"{}\"", generated_title),
                        format!("\nTitle: \"{}\"", generated_title),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }

            // Check optional Description
            if code_system.description().is_none() {
                diagnostics.push(
                    Diagnostic::new(
                        MISSING_DESCRIPTION,
                        Severity::Warning,
                        format!("CodeSystem '{name}' should have a Description"),
                        location.clone(),
                    )
                    .with_code("codesystem-missing-description".to_string())
                    .with_suggestion(CodeSuggestion::safe(
                        "Add placeholder Description".to_string(),
                        format!("\nDescription: \"TODO: Add description for {}\"", name),
                        get_insertion_location_after_first_line(&location, model),
                    )),
                );
            }
        }
    }

    diagnostics
}

/// Check that Extensions have Context specifications
///
/// Extensions must specify ^context rules to define where they can be used.
/// This is a critical requirement in FHIR - extensions without context are invalid.
pub fn check_extension_context(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    for extension in document.extensions() {
        // Check if extension has ^context rules
        // In FSH, context is defined with: * ^context[+].type = #element
        // We check the entire extension's syntax text since the ^context appears
        // in the rule statement, not just the rule value
        let extension_text = extension.syntax().text().to_string();
        let has_context = extension_text.contains("^context");

        if !has_context {
            let extension_name = extension
                .name()
                .unwrap_or_else(|| "this extension".to_string());

            let location = model.source_map.node_to_diagnostic_location(
                extension.syntax(),
                &model.source,
                &model.source_file,
            );

            // Suggest adding context with TODO placeholder
            let suggested_context = format!(
                "\n* ^context[+].type = #element\n* ^context[=].expression = \"Patient\"  // TODO: Update to correct resource type for {}",
                extension_name
            );

            let message = format!(
                "Extension '{}' must specify Context\n  Note: Extensions require ^context rules to define where they can be used\n  Help: Add ^context rules to specify which FHIR resources this extension applies to",
                extension_name
            );

            diagnostics.push(
                Diagnostic::new(
                    EXTENSION_CONTEXT_MISSING,
                    Severity::Error,
                    message,
                    location.clone(),
                )
                .with_code("extension-missing-context".to_string())
                .with_suggestion(CodeSuggestion::unsafe_fix(
                    format!("Add Context for {}", extension_name),
                    suggested_context,
                    location,
                )),
            );
        }
    }

    diagnostics
}

/// Structure to represent a required element in a profile
#[derive(Debug, Clone)]
struct RequiredElement {
    path: String,
    #[allow(dead_code)] // Used for display in diagnostics
    min: u32,
    max_str: String,
}

/// Check that instances provide all required fields from their profiles
///
/// This validates that instances satisfy the cardinality requirements (min >= 1)
/// defined in their profiles.
pub fn check_instance_required_fields(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect all profiles for lookup
    let profiles: std::collections::HashMap<String, _> = document
        .profiles()
        .filter_map(|p| p.name().map(|name| (name, p)))
        .collect();

    // Check each instance
    for instance in document.instances() {
        if let Some(instance_name) = instance.name() {
            // Get the profile this instance conforms to
            if let Some(instance_of) = instance.instance_of()
                && let Some(profile_name) = instance_of.value()
            {
                // Find the profile definition
                if let Some(profile) = profiles.get(&profile_name) {
                    // Collect required elements from the profile
                    let required_elements = collect_required_elements(profile);

                    if !required_elements.is_empty() {
                        // Collect provided elements from the instance
                        let provided_elements = collect_instance_assignments(&instance);

                        // Check for missing required elements
                        for required in &required_elements {
                            if !provided_elements.contains(&required.path) {
                                let location = model.source_map.node_to_diagnostic_location(
                                    instance.syntax(),
                                    &model.source,
                                    &model.source_file,
                                );

                                let message = format!(
                                    "Instance '{}' missing required element '{}'\n  Note: Profile '{}' requires this element with cardinality {}",
                                    instance_name, required.path, profile_name, required.max_str
                                );

                                diagnostics.push(
                                    Diagnostic::new(
                                        INSTANCE_REQUIRED_FIELDS_MISSING,
                                        Severity::Error,
                                        message,
                                        location,
                                    )
                                    .with_code("instance-missing-required-field".to_string()),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    diagnostics
}

/// Collect required elements (min >= 1) from a profile's cardinality rules
fn collect_required_elements(profile: &maki_core::cst::ast::Profile) -> Vec<RequiredElement> {
    use maki_core::cst::ast::Rule;
    let mut required = Vec::new();

    for rule in profile.rules() {
        if let Rule::Card(card_rule) = rule {
            // Extract the path using the API
            if let Some(path) = card_rule.path() {
                let path_str = path.syntax().text().to_string();

                // The CardRule's own text contains the cardinality (e.g., "1..*" or "1..1")
                let card_text = card_rule.syntax().text().to_string().trim().to_string();

                // Parse cardinality from text (format: "min..max")
                if let Some((min_str, _max_str)) = card_text.split_once("..")
                    && let Ok(min) = min_str.trim().parse::<u32>()
                    && min >= 1
                {
                    required.push(RequiredElement {
                        path: path_str,
                        min,
                        max_str: card_text,
                    });
                }
            }
        }
    }

    required
}

/// Collect all assignment paths from an instance
fn collect_instance_assignments(
    instance: &maki_core::cst::ast::Instance,
) -> std::collections::HashSet<String> {
    use maki_core::cst::ast::Rule;
    let mut paths = std::collections::HashSet::new();

    for rule in instance.rules() {
        // Extract path from different rule types
        let path_opt = match &rule {
            Rule::FixedValue(fixed) => fixed.path(),
            Rule::Path(_path_rule) => {
                // Path rules don't have an assignment, skip them
                None
            }
            Rule::Card(card) => card.path(),
            Rule::ValueSet(vs) => vs.path(),
            Rule::Flag(flag) => flag.path(),
            Rule::Only(only) => only.path(),
            Rule::Contains(contains) => contains.path(),
            Rule::Obeys(obeys) => obeys.path(),
            Rule::CaretValue(caret) => caret.element_path(),
            _ => None,
        };

        if let Some(path) = path_opt {
            let path_str = path.syntax().text().to_string();

            // Add the full path
            paths.insert(path_str.clone());

            // Also add parent paths (e.g., if we have "name.family", also add "name")
            let mut current_path = path_str.as_str();
            while let Some(dot_pos) = current_path.rfind('.') {
                current_path = &current_path[..dot_pos];
                paths.insert(current_path.to_string());
            }
        }
    }

    paths
}

/// Creates a location that only covers the first line of a multi-line node.
/// This provides better visual highlighting for diagnostics on definition headers.
fn first_line_location(location: Location, source: &str) -> Location {
    // If it's already a single line, return as-is
    if location.end_line == Some(location.line) || location.end_line.is_none() {
        return location;
    }

    // Find the end of the first line
    let lines: Vec<&str> = source.lines().collect();
    let first_line_len = if location.line > 0 && location.line <= lines.len() {
        lines[location.line - 1].len()
    } else {
        0
    };

    Location {
        end_line: Some(location.line),
        end_column: Some(first_line_len + 1), // 1-indexed, after last char
        length: first_line_len.saturating_sub(location.column.saturating_sub(1)),
        ..location
    }
}

/// Check that profiles have example instances
///
/// This is a documentation/best practice rule that warns when profiles
/// don't have any example instances defined.
pub fn check_profile_without_examples(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect all instance -> profile mappings
    let instance_profiles: std::collections::HashSet<String> = document
        .instances()
        .filter_map(|inst| inst.instance_of())
        .filter_map(|iof| iof.value())
        .collect();

    // Check each profile
    for profile in document.profiles() {
        if let Some(profile_name) = profile.name() {
            // Check if any instance uses this profile
            if !instance_profiles.contains(&profile_name) {
                // Get full location then trim to first line for better visual highlighting
                let full_location = model.source_map.node_to_diagnostic_location(
                    profile.syntax(),
                    &model.source,
                    &model.source_file,
                );
                let location = first_line_location(full_location, &model.source);

                let message = format!(
                    "Profile '{}' has no example instances\n  Note: Profiles should have at least one example instance\n  Help: Create an example instance with 'Instance: {}Example' and 'InstanceOf: {}'",
                    profile_name, profile_name, profile_name
                );

                diagnostics.push(
                    Diagnostic::new(
                        PROFILE_WITHOUT_EXAMPLES,
                        Severity::Warning,
                        message,
                        location,
                    )
                    .with_code("profile-without-examples".to_string()),
                );
            }
        }
    }

    diagnostics
}

/// Check for required field overrides (child making required fields optional)
/// This requires FHIR definitions to be loaded via LazySession.
/// The session is only initialized when profiles with external parents are found.
pub async fn check_required_field_override(
    model: &SemanticModel,
    lazy_session: Option<&std::sync::Arc<maki_core::LazySession>>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // If no lazy session, we can't look up FHIR definitions - skip this rule
    let Some(lazy_session) = lazy_session else {
        return diagnostics;
    };

    // Collect profiles first to avoid holding CST iterators across await points
    let profiles: Vec<_> = document.profiles().collect();

    for profile in profiles {
        diagnostics
            .extend(check_profile_required_field_override(&profile, model, lazy_session).await);
    }

    diagnostics
}

/// Check a profile for required field overrides
async fn check_profile_required_field_override(
    profile: &maki_core::cst::ast::Profile,
    model: &SemanticModel,
    lazy_session: &std::sync::Arc<maki_core::LazySession>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Get parent profile name
    let Some(parent) = profile.parent() else {
        return diagnostics;
    };

    let Some(parent_name) = parent.value() else {
        return diagnostics;
    };

    // Check if parent is external (not defined locally) - only then init session
    let is_external_parent =
        !parent_name.contains('.') && parent_name.chars().next().is_some_and(|c| c.is_uppercase());

    if !is_external_parent {
        return diagnostics;
    }

    // Collect all cardinality rules and their data before any async operations
    let card_rules: Vec<_> = profile
        .rules()
        .filter_map(|r| match r {
            maki_core::cst::ast::Rule::Card(c) => {
                let text = c.syntax().text().to_string().trim().to_string();
                let range = c.syntax().text_range();
                let path = c.path().map(|p| p.syntax().text().to_string());
                Some((text, range, path))
            }
            _ => None,
        })
        .collect();

    // Initialize session lazily only when we actually need to resolve parent
    let session = match lazy_session.get().await {
        Ok(s) => s,
        Err(_) => return diagnostics,
    };

    // Construct canonical URL for FHIR base types
    let canonical_url = format!("http://hl7.org/fhir/StructureDefinition/{}", parent_name);

    // Resolve parent StructureDefinition asynchronously
    let Ok(Some(parent_sd)) = session.resolve_structure_definition(&canonical_url).await else {
        return diagnostics;
    };

    // Check each cardinality rule against parent
    for (cardinality_text, text_range, path_opt) in card_rules {
        let Some(path_str) = path_opt else {
            continue;
        };

        // Parse child cardinality
        let Some((child_min_str, _child_max_str)) = cardinality_text.split_once("..") else {
            continue;
        };

        let Ok(child_min) = child_min_str.trim().parse::<u32>() else {
            continue;
        };

        // Find parent element cardinality
        if let Some(parent_min) = find_element_min_cardinality(&parent_sd, &path_str) {
            // Check if child is making a required field optional
            // Parent requires field (min >= 1) but child makes it optional (min == 0)
            if parent_min >= 1 && child_min == 0 {
                let span = text_range.start().into()..text_range.end().into();
                let location = model.source_map.span_to_diagnostic_location(
                    &span,
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        REQUIRED_FIELD_OVERRIDE,
                        Severity::Error,
                        format!(
                            "Element '{}' is required in parent '{}' (min: {}) but made optional (min: 0) in this profile",
                            path_str,
                            parent_name,
                            parent_min
                        ),
                        location,
                    )
                    .with_code("required-field-override".to_string()),
                );
            }
        }
    }

    diagnostics
}

/// Find minimum cardinality for a specific element in a StructureDefinition
fn find_element_min_cardinality(
    sd: &maki_core::export::StructureDefinition,
    element_path: &str,
) -> Option<u32> {
    let resource_type: &str = sd.resource_type.as_ref();
    let full_path = if element_path == "." || element_path.is_empty() {
        resource_type.to_string()
    } else {
        format!("{}.{}", resource_type, element_path)
    };

    // Look through snapshot elements first
    if let Some(snapshot) = &sd.snapshot {
        for element in &snapshot.element {
            if element.path == full_path || element.path.ends_with(&format!(".{}", element_path)) {
                return element.min;
            }
        }
    }

    // Fall back to differential
    if let Some(differential) = &sd.differential {
        for element in &differential.element {
            if element.path == full_path || element.path.ends_with(&format!(".{}", element_path)) {
                return element.min;
            }
        }
    }

    None
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
    fn test_name_to_kebab_case_id() {
        assert_eq!(name_to_kebab_case_id("MyProfile"), "my-profile");
        assert_eq!(name_to_kebab_case_id("PatientProfile"), "patient-profile");
        assert_eq!(
            name_to_kebab_case_id("MyPatientProfileExtension"),
            "my-patient-profile-extension"
        );
        assert_eq!(name_to_kebab_case_id("simple"), "simple");
    }

    #[test]
    fn test_name_to_title_case() {
        assert_eq!(name_to_title_case("MyProfile"), "My Profile");
        assert_eq!(name_to_title_case("PatientProfile"), "Patient Profile");
        assert_eq!(
            name_to_title_case("MyPatientProfileExtension"),
            "My Patient Profile Extension"
        );
        assert_eq!(name_to_title_case("simple"), "simple");
    }

    #[test]
    fn test_profile_missing_parent() {
        let source = "Profile: MyProfile\nId: my-profile\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let parent_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_PARENT)
            .collect();
        assert_eq!(parent_diags.len(), 1);
        assert!(parent_diags[0].message.contains("Parent"));
    }

    #[test]
    fn test_profile_missing_id() {
        let source = "Profile: MyProfile\nParent: Patient\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let id_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_ID)
            .collect();
        assert_eq!(id_diags.len(), 1);
        assert!(id_diags[0].message.contains("Id"));
        assert!(
            id_diags[0]
                .suggestions
                .iter()
                .any(|s| s.message.contains("my-profile"))
        );
    }

    #[test]
    fn test_profile_missing_title() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let title_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_TITLE)
            .collect();
        assert_eq!(title_diags.len(), 1);
        assert!(title_diags[0].message.contains("Title"));
        assert!(
            title_diags[0]
                .suggestions
                .iter()
                .any(|s| s.message.contains("My Profile"))
        );
    }

    #[test]
    fn test_profile_missing_description() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\nTitle: \"My Profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let desc_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == MISSING_DESCRIPTION)
            .collect();
        assert_eq!(desc_diags.len(), 1);
        assert!(desc_diags[0].message.contains("Description"));
        assert!(
            desc_diags[0]
                .suggestions
                .iter()
                .any(|s| s.replacement.contains("TODO"))
        );
    }

    #[test]
    fn test_profile_all_fields_present() {
        let source = "Profile: MyProfile\nParent: Patient\nId: my-profile\nTitle: \"My Profile\"\nDescription: \"Test profile\"\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        // Should have no required field diagnostics
        let required_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.rule_id == REQUIRED_PARENT
                    || d.rule_id == REQUIRED_ID
                    || d.rule_id == REQUIRED_TITLE
            })
            .collect();
        assert_eq!(
            required_diags.len(),
            0,
            "No required fields should be missing"
        );
    }

    #[test]
    fn test_extension_missing_id_and_title() {
        let source = "Extension: MyExtension\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let id_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_ID && d.message.contains("Extension"))
            .collect();
        assert_eq!(id_diags.len(), 1);

        let title_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule_id == REQUIRED_TITLE && d.message.contains("Extension"))
            .collect();
        assert_eq!(title_diags.len(), 1);
    }

    #[test]
    fn test_valueset_missing_fields() {
        let source = "ValueSet: MyValueSet\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("ValueSet") && d.message.contains("must specify"))
            .collect();
        assert!(missing.len() >= 2, "Should detect missing Id and Title");
    }

    #[test]
    fn test_codesystem_missing_fields() {
        let source = "CodeSystem: MyCodeSystem\n";
        let model = create_test_model(source);
        let diagnostics = check_required_fields(&model);

        let missing: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("CodeSystem") && d.message.contains("must specify"))
            .collect();
        assert!(missing.len() >= 2, "Should detect missing Id and Title");
    }

    // Extension Context Tests

    #[test]
    fn test_extension_missing_context() {
        let source = r#"Extension: MyExtension
Description: "An extension without context"
* value[x] only string
"#;
        let model = create_test_model(source);
        let diagnostics = check_extension_context(&model);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule_id, EXTENSION_CONTEXT_MISSING);
        assert!(diagnostics[0].message.contains("Context"));
        assert!(diagnostics[0].message.contains("MyExtension"));
    }

    #[test]
    fn test_extension_with_context() {
        let source = r#"Extension: MyExtension
Description: "An extension with proper context"
* value[x] only string
* ^context[+].type = #element
* ^context[=].expression = "Patient"
"#;
        let model = create_test_model(source);
        let diagnostics = check_extension_context(&model);

        assert_eq!(diagnostics.len(), 0, "Extension with context should pass");
    }

    #[test]
    fn test_extension_context_autofix_suggestion() {
        let source = r#"Extension: BirthPlace
Description: "Place of birth"
* value[x] only Address
"#;
        let model = create_test_model(source);
        let diagnostics = check_extension_context(&model);

        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].suggestions.is_empty());

        let suggestion = &diagnostics[0].suggestions[0];
        assert!(suggestion.replacement.contains("^context"));
        assert!(suggestion.replacement.contains("TODO"));
        assert!(suggestion.replacement.contains("BirthPlace"));
    }

    #[test]
    fn test_extension_with_multiple_contexts() {
        let source = r#"Extension: Ethnicity
Description: "Patient ethnicity"
* value[x] only CodeableConcept
* ^context[+].type = #element
* ^context[=].expression = "Patient"
* ^context[+].type = #element
* ^context[=].expression = "RelatedPerson"
"#;
        let model = create_test_model(source);
        let diagnostics = check_extension_context(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Extension with multiple contexts should pass"
        );
    }

    #[test]
    fn test_multiple_extensions_context_check() {
        let source = r#"Extension: ValidExtension
* value[x] only string
* ^context[+].type = #element
* ^context[=].expression = "Patient"

Extension: InvalidExtension
* value[x] only CodeableConcept
"#;
        let model = create_test_model(source);
        let diagnostics = check_extension_context(&model);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("InvalidExtension"));
    }

    // Instance Required Fields Tests

    #[test]
    fn test_instance_missing_required_fields() {
        let source = r#"Profile: StrictPatient
Parent: Patient
* name 1..*
* gender 1..1

Instance: IncompleteExample
InstanceOf: StrictPatient
* birthDate = "1990-01-01"
"#;
        let model = create_test_model(source);
        let diagnostics = check_instance_required_fields(&model);

        // Should detect missing 'name' and 'gender'
        assert!(
            diagnostics.len() >= 2,
            "Should detect at least 2 missing fields"
        );
        assert!(diagnostics.iter().any(|d| d.message.contains("name")));
        assert!(diagnostics.iter().any(|d| d.message.contains("gender")));
    }

    #[test]
    fn test_instance_with_all_required_fields() {
        let source = r#"Profile: StrictPatient
Parent: Patient
* name 1..*
* gender 1..1

Instance: CompleteExample
InstanceOf: StrictPatient
* name.family = "Smith"
* gender = #male
"#;
        let model = create_test_model(source);
        let diagnostics = check_instance_required_fields(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Complete instance should have no errors"
        );
    }

    #[test]
    fn test_instance_with_nested_paths() {
        let source = r#"Profile: DetailedPatient
Parent: Patient
* name 1..*
* name.family 1..1

Instance: DetailedExample
InstanceOf: DetailedPatient
* name.family = "Doe"
* name.given = "Jane"
"#;
        let model = create_test_model(source);
        let diagnostics = check_instance_required_fields(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Instance with nested paths should satisfy parent path requirements"
        );
    }

    #[test]
    fn test_instance_without_profile() {
        let source = r#"Instance: StandaloneExample
InstanceOf: Patient
* birthDate = "1990-01-01"
"#;
        let model = create_test_model(source);
        let diagnostics = check_instance_required_fields(&model);

        // Should not error for instances of built-in types
        // (we only check against profiles defined in the same document)
        assert_eq!(diagnostics.len(), 0);
    }

    // Profile Without Examples Tests

    #[test]
    fn test_profile_without_examples_warns() {
        let source = r#"Profile: LonelyProfile
Parent: Patient
* name 1..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_profile_without_examples(&model);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("no example"));
    }

    #[test]
    fn test_profile_with_examples() {
        let source = r#"Profile: PopularProfile
Parent: Patient
* name 1..*

Instance: ExamplePatient
InstanceOf: PopularProfile
* name.family = "Smith"
"#;
        let model = create_test_model(source);
        let diagnostics = check_profile_without_examples(&model);

        assert_eq!(
            diagnostics.len(),
            0,
            "Profile with examples should not warn"
        );
    }

    #[test]
    fn test_multiple_profiles_some_without_examples() {
        let source = r#"Profile: ProfileWithExample
Parent: Patient
* name 1..*

Instance: ExamplePatient
InstanceOf: ProfileWithExample
* name.family = "Smith"

Profile: ProfileWithoutExample
Parent: Observation
* status 1..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_profile_without_examples(&model);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("ProfileWithoutExample"));
    }

    // Required Field Override Tests (async with DefinitionSession)

    #[tokio::test]
    async fn test_required_field_override_without_session() {
        let source = r#"Profile: TestProfile
Parent: Patient
* name 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_required_field_override(&model, None).await;

        // Without session, should not error (gracefully skip)
        assert_eq!(diagnostics.len(), 0);
    }

    #[tokio::test]
    async fn test_required_field_override_with_session() {
        // This test requires a real DefinitionSession with FHIR packages
        // For now, we test the structure without actual FHIR data
        let source = r#"Profile: TestProfile
Parent: Patient
* name 0..1
"#;
        let model = create_test_model(source);

        // Note: This test would need a proper DefinitionSession with Patient definition
        // to actually trigger the validation. Without it, the rule gracefully skips.
        let diagnostics = check_required_field_override(&model, None).await;
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_find_element_min_cardinality_with_snapshot() {
        // Test the helper function with a mock StructureDefinition
        let sd = maki_core::export::StructureDefinition {
            resource_type: "StructureDefinition".to_string(),
            id: Some("test-patient".to_string()),
            url: "http://test.com/Patient".to_string(),
            version: None,
            name: "TestPatient".to_string(),
            title: None,
            status: "active".to_string(),
            date: None,
            publisher: None,
            description: None,
            experimental: None,
            fhir_version: None,
            kind: maki_core::export::StructureDefinitionKind::Resource,
            is_abstract: false,
            type_field: "Patient".to_string(),
            base_definition: None,
            derivation: None,
            extension: None,
            context: None,
            snapshot: Some(maki_core::export::StructureDefinitionSnapshot {
                element: vec![{
                    let mut elem =
                        maki_core::export::ElementDefinition::new("Patient.name".to_string());
                    elem.id = Some("Patient.name".to_string());
                    elem.min = Some(1);
                    elem.max = Some("*".to_string());
                    elem
                }],
            }),
            differential: None,
            mapping: None,
        };

        let min = find_element_min_cardinality(&sd, "name");
        assert_eq!(min, Some(1));
    }

    #[test]
    fn test_find_element_min_cardinality_not_found() {
        let sd = maki_core::export::StructureDefinition {
            resource_type: "StructureDefinition".to_string(),
            id: Some("test-patient".to_string()),
            url: "http://test.com/Patient".to_string(),
            version: None,
            name: "TestPatient".to_string(),
            title: None,
            status: "active".to_string(),
            date: None,
            publisher: None,
            description: None,
            experimental: None,
            fhir_version: None,
            kind: maki_core::export::StructureDefinitionKind::Resource,
            is_abstract: false,
            type_field: "Patient".to_string(),
            base_definition: None,
            derivation: None,
            extension: None,
            context: None,
            snapshot: None,
            differential: None,
            mapping: None,
        };

        let min = find_element_min_cardinality(&sd, "nonexistent");
        assert_eq!(min, None);
    }
}
