//! Binding strength validation rules
//!
//! Validates that bindings to value sets have proper strength specifications,
//! don't weaken parent bindings, and are used consistently across profiles.

use maki_core::cst::ast::{AstNode, Document, Extension, Profile, ValueSetRule};
use maki_core::{CodeSuggestion, Diagnostic, SemanticModel, Severity};
use std::collections::HashMap;

/// Rule ID for binding strength required (blocking rule)
/// Previously named BINDING_STRENGTH_PRESENT for backwards compatibility
pub const BINDING_STRENGTH_PRESENT: &str = "blocking/binding-strength-present";
pub const BINDING_STRENGTH_REQUIRED: &str = "blocking/binding-strength-required";

/// Rule ID for binding strength weakening (semantic validation)
pub const BINDING_STRENGTH_WEAKENING: &str = "correctness/binding-strength-weakening";

/// Rule ID for inconsistent binding strengths (best practice)
pub const BINDING_STRENGTH_INCONSISTENT: &str = "suspicious/binding-strength-inconsistent";

/// Rule ID for binding without valueset (correctness)
pub const BINDING_WITHOUT_VALUESET: &str = "correctness/binding-without-valueset";

/// FHIR binding strength hierarchy (from strongest to weakest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingStrength {
    /// Required: Must use a code from the ValueSet
    Required = 4,
    /// Extensible: Should use a code from the ValueSet if applicable
    Extensible = 3,
    /// Preferred: Suggested ValueSet, but not enforced
    Preferred = 2,
    /// Example: Example ValueSet for guidance only
    Example = 1,
}

impl BindingStrength {
    /// Parse binding strength from string (case-insensitive)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "required" => Some(BindingStrength::Required),
            "extensible" => Some(BindingStrength::Extensible),
            "preferred" => Some(BindingStrength::Preferred),
            "example" => Some(BindingStrength::Example),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            BindingStrength::Required => "required",
            BindingStrength::Extensible => "extensible",
            BindingStrength::Preferred => "preferred",
            BindingStrength::Example => "example",
        }
    }

    /// Check if this strength is valid
    pub fn is_valid(s: &str) -> bool {
        Self::parse(s).is_some()
    }
}

impl std::str::FromStr for BindingStrength {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid binding strength: '{}'", s))
    }
}

impl std::fmt::Display for BindingStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Valid binding strengths in FHIR
const VALID_BINDING_STRENGTHS: &[&str] = &["required", "extensible", "preferred", "example"];

/// Check for missing or invalid binding strengths in FSH document
/// This is the main BLOCKING rule that runs first
pub fn check_binding_strength_required(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        diagnostics.extend(check_value_set_bindings_required(
            profile.rules().filter_map(|r| match r {
                maki_core::cst::ast::Rule::ValueSet(vs) => Some(vs),
                _ => None,
            }),
            model,
        ));
    }

    // Check extensions
    for extension in document.extensions() {
        diagnostics.extend(check_value_set_bindings_required(
            extension.rules().filter_map(|r| match r {
                maki_core::cst::ast::Rule::ValueSet(vs) => Some(vs),
                _ => None,
            }),
            model,
        ));
    }

    diagnostics
}

/// Check value set binding rules for proper strength specification
fn check_value_set_bindings_required(
    rules: impl Iterator<Item = ValueSetRule>,
    model: &SemanticModel,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in rules {
        match rule.strength() {
            Some(strength) => {
                // Check if the strength is valid
                let strength_lower = strength.to_lowercase();
                if !VALID_BINDING_STRENGTHS.contains(&strength_lower.as_str()) {
                    let location = model.source_map.node_to_diagnostic_location(
                        rule.syntax(),
                        &model.source,
                        &model.source_file,
                    );
                    diagnostics.push(
                        Diagnostic::new(
                            BINDING_STRENGTH_PRESENT,
                            Severity::Error,
                            format!(
                                "Invalid binding strength '{}'. Must be one of: required, extensible, preferred, example",
                                strength
                            ),
                            location,
                        )
                        .with_code("invalid-binding-strength".to_string()),
                    );
                }
            }
            None => {
                // Missing binding strength - provide autofix
                if let Some(value_set_name) = rule.value_set() {
                    let location = model.source_map.node_to_diagnostic_location(
                        rule.syntax(),
                        &model.source,
                        &model.source_file,
                    );

                    // Create autofix to add (required) strength
                    let text = rule.syntax().text().to_string();
                    let suggestion = CodeSuggestion::unsafe_fix(
                        "Add (required) binding strength",
                        format!("{} (required)", text.trim()),
                        location.clone(),
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            BINDING_STRENGTH_PRESENT,
                            Severity::Error,
                            format!(
                                "Binding to ValueSet '{}' must specify a strength (required, extensible, preferred, or example)",
                                value_set_name
                            ),
                            location,
                        )
                        .with_code("missing-binding-strength".to_string())
                        .with_suggestion(suggestion),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Check for binding strength weakening (child using weaker strength than parent)
/// This requires FHIR definitions to be loaded via DefinitionSession
pub async fn check_binding_strength_weakening(
    model: &SemanticModel,
    session: Option<&maki_core::canonical::DefinitionSession>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect profiles and extensions first to avoid holding CST iterators across await points
    let profiles: Vec<_> = document.profiles().collect();
    let extensions: Vec<_> = document.extensions().collect();

    // Check profiles for weakening
    for profile in profiles {
        diagnostics.extend(check_profile_binding_weakening(&profile, model, session).await);
    }

    // Check extensions for weakening
    for extension in extensions {
        diagnostics.extend(check_extension_binding_weakening(&extension, model, session).await);
    }

    diagnostics
}

/// Check a profile for binding strength weakening
async fn check_profile_binding_weakening(
    profile: &Profile,
    model: &SemanticModel,
    session: Option<&maki_core::canonical::DefinitionSession>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Get parent profile name
    let Some(parent) = profile.parent() else {
        return diagnostics;
    };

    let Some(parent_name) = parent.value() else {
        return diagnostics;
    };

    // If no session, we can't look up FHIR definitions - skip this rule
    let Some(session) = session else {
        return diagnostics;
    };

    // Collect all binding information first to avoid holding CST iterators across await
    let mut bindings = Vec::new();
    for rule in profile.rules() {
        if let maki_core::cst::ast::Rule::ValueSet(vs_rule) = rule {
            // Get the strength of this binding
            let Some(strength_str) = vs_rule.strength() else {
                continue;
            };

            let Some(child_strength) = BindingStrength::parse(&strength_str) else {
                continue;
            };

            // Get the element path
            let path_str = vs_rule
                .path()
                .map(|p| p.syntax().text().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Store syntax node text range for diagnostic location
            let text_range = vs_rule.syntax().text_range();
            bindings.push((child_strength, path_str, text_range));
        }
    }

    // Now do async lookup - no CST references held
    if let Ok(Some(parent_sd)) = session.resolve_structure_definition(&parent_name).await {
        for (child_strength, path_str, text_range) in bindings {
            // Find the element in parent that matches this path
            if let Some(parent_binding_strength) =
                find_element_binding_strength(&parent_sd, &path_str)
            {
                // Check if child weakens parent binding
                if child_strength < parent_binding_strength {
                    let span = text_range.start().into()..text_range.end().into();
                    let location = model.source_map.span_to_diagnostic_location(
                        &span,
                        &model.source,
                        &model.source_file,
                    );

                    diagnostics.push(
                        Diagnostic::new(
                            BINDING_STRENGTH_WEAKENING,
                            Severity::Error,
                            format!(
                                "Binding strength '{}' for element '{}' is weaker than parent '{}' binding strength '{}'",
                                child_strength.as_str(),
                                path_str,
                                parent_name,
                                parent_binding_strength.as_str()
                            ),
                            location,
                        )
                        .with_code("binding-strength-weakening".to_string()),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Check an extension for binding strength weakening
async fn check_extension_binding_weakening(
    _extension: &Extension,
    _model: &SemanticModel,
    _session: Option<&maki_core::canonical::DefinitionSession>,
) -> Vec<Diagnostic> {
    // Extensions typically define their own bindings, so weakening is less relevant
    // However, if an extension extends another extension, this should be checked
    // TODO: Implement extension parent binding checking when needed

    Vec::new()
}

/// Find binding strength for a specific element in a StructureDefinition
fn find_element_binding_strength(
    sd: &maki_core::export::StructureDefinition,
    element_path: &str,
) -> Option<BindingStrength> {
    // The element path in FSH is relative to the resource (e.g., "code", "component.code")
    // We need to convert it to the full FHIR path format
    let resource_type: &str = sd.resource_type.as_ref();
    let full_path = if element_path == "." || element_path.is_empty() {
        resource_type.to_string()
    } else {
        format!("{}.{}", resource_type, element_path)
    };

    // Look through snapshot elements first (most reliable)
    if let Some(snapshot) = &sd.snapshot {
        for element in &snapshot.element {
            if (element.path == full_path || element.path.ends_with(&format!(".{}", element_path)))
                && let Some(binding) = &element.binding
            {
                // binding.strength is a BindingStrength enum, not Option<String>
                // We need to convert it to our BindingStrength
                return match binding.strength {
                    maki_core::export::BindingStrength::Required => Some(BindingStrength::Required),
                    maki_core::export::BindingStrength::Extensible => {
                        Some(BindingStrength::Extensible)
                    }
                    maki_core::export::BindingStrength::Preferred => {
                        Some(BindingStrength::Preferred)
                    }
                    maki_core::export::BindingStrength::Example => Some(BindingStrength::Example),
                };
            }
        }
    }

    // Fall back to differential if snapshot not available
    if let Some(differential) = &sd.differential {
        for element in &differential.element {
            if (element.path == full_path || element.path.ends_with(&format!(".{}", element_path)))
                && let Some(binding) = &element.binding
            {
                return match binding.strength {
                    maki_core::export::BindingStrength::Required => Some(BindingStrength::Required),
                    maki_core::export::BindingStrength::Extensible => {
                        Some(BindingStrength::Extensible)
                    }
                    maki_core::export::BindingStrength::Preferred => {
                        Some(BindingStrength::Preferred)
                    }
                    maki_core::export::BindingStrength::Example => Some(BindingStrength::Example),
                };
            }
        }
    }

    None
}

/// Check for inconsistent binding strengths across similar elements
pub fn check_binding_strength_inconsistent(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles for inconsistent bindings
    for profile in document.profiles() {
        diagnostics.extend(check_profile_binding_consistency(&profile, model));
    }

    diagnostics
}

/// Check a profile for binding strength consistency
fn check_profile_binding_consistency(profile: &Profile, model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Group bindings by element name pattern
    let mut element_groups: HashMap<String, Vec<(String, BindingStrength, ValueSetRule)>> =
        HashMap::new();

    for rule in profile.rules() {
        if let maki_core::cst::ast::Rule::ValueSet(vs_rule) = rule
            && let Some(strength_str) = vs_rule.strength()
            && let Some(strength) = BindingStrength::parse(&strength_str)
        {
            let path_str = vs_rule
                .path()
                .map(|p| {
                    // Convert path to string by getting its syntax text
                    p.syntax().text().to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());

            // Extract element name pattern (e.g., "code" from "component.code")
            let element_name = extract_element_name(&path_str);

            element_groups
                .entry(element_name.clone())
                .or_default()
                .push((path_str, strength, vs_rule));
        }
    }

    // Check each group for inconsistencies
    for (element_name, bindings) in element_groups.iter() {
        if bindings.len() >= 2 {
            // Check if there are multiple different strengths
            let strengths: Vec<_> = bindings.iter().map(|(_, s, _)| s).collect();
            let unique_strengths: std::collections::HashSet<_> = strengths.iter().collect();

            if unique_strengths.len() > 1 && element_name != "unknown" {
                // Found inconsistent strengths
                let location = model.source_map.node_to_diagnostic_location(
                    profile.syntax(),
                    &model.source,
                    &model.source_file,
                );

                diagnostics.push(
                    Diagnostic::new(
                        BINDING_STRENGTH_INCONSISTENT,
                        Severity::Info,
                        format!(
                            "Inconsistent binding strengths for '{}' elements in profile {}",
                            element_name,
                            profile.name().unwrap_or_else(|| "unknown".to_string())
                        ),
                        location,
                    )
                    .with_code("inconsistent-binding-strengths".to_string()),
                );
            }
        }
    }

    diagnostics
}

/// Extract element name from path (e.g., "code" from "component.code")
fn extract_element_name(path: &str) -> String {
    path.split('.')
        .next_back()
        .unwrap_or(path)
        .split('[')
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Check for bindings that reference non-existent ValueSets
pub fn check_binding_without_valueset(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect all defined ValueSets in the document
    let defined_valuesets: std::collections::HashSet<_> =
        document.value_sets().filter_map(|vs| vs.name()).collect();

    // Check profiles for undefined ValueSet references
    for profile in document.profiles() {
        for rule in profile.rules() {
            if let maki_core::cst::ast::Rule::ValueSet(vs_rule) = rule
                && let Some(vs_name) = vs_rule.value_set()
            {
                // Check if ValueSet is defined locally
                if !defined_valuesets.contains(&vs_name) {
                    // ValueSet might be from FHIR spec or external package
                    // TODO: Check against FHIR definitions when available
                    // For now, we'll only warn about likely typos (no URL format)

                    if !vs_name.starts_with("http://") && !vs_name.starts_with("https://") {
                        let location = model.source_map.node_to_diagnostic_location(
                            vs_rule.syntax(),
                            &model.source,
                            &model.source_file,
                        );

                        diagnostics.push(
                            Diagnostic::new(
                                BINDING_WITHOUT_VALUESET,
                                Severity::Warning,
                                format!(
                                    "Binding references ValueSet '{}' which is not defined in this project",
                                    vs_name
                                ),
                                location,
                            )
                            .with_code("undefined-valueset".to_string()),
                        );
                    }
                }
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_strength_ordering() {
        assert!(BindingStrength::Required > BindingStrength::Extensible);
        assert!(BindingStrength::Extensible > BindingStrength::Preferred);
        assert!(BindingStrength::Preferred > BindingStrength::Example);
    }

    #[test]
    fn test_binding_strength_parse() {
        assert_eq!(
            BindingStrength::parse("required"),
            Some(BindingStrength::Required)
        );
        assert_eq!(
            BindingStrength::parse("REQUIRED"),
            Some(BindingStrength::Required)
        );
        assert_eq!(
            BindingStrength::parse("extensible"),
            Some(BindingStrength::Extensible)
        );
        assert_eq!(
            BindingStrength::parse("preferred"),
            Some(BindingStrength::Preferred)
        );
        assert_eq!(
            BindingStrength::parse("example"),
            Some(BindingStrength::Example)
        );
        assert_eq!(BindingStrength::parse("invalid"), None);
    }

    #[test]
    fn test_binding_strength_from_str_trait() {
        use std::str::FromStr;
        assert_eq!(
            BindingStrength::from_str("required").ok(),
            Some(BindingStrength::Required)
        );
        assert!(BindingStrength::from_str("invalid").is_err());
    }

    #[test]
    fn test_binding_strength_as_str() {
        assert_eq!(BindingStrength::Required.as_str(), "required");
        assert_eq!(BindingStrength::Extensible.as_str(), "extensible");
        assert_eq!(BindingStrength::Preferred.as_str(), "preferred");
        assert_eq!(BindingStrength::Example.as_str(), "example");
    }

    #[test]
    fn test_binding_strength_is_valid() {
        assert!(BindingStrength::is_valid("required"));
        assert!(BindingStrength::is_valid("EXTENSIBLE"));
        assert!(BindingStrength::is_valid("Preferred"));
        assert!(!BindingStrength::is_valid("invalid"));
        assert!(!BindingStrength::is_valid(""));
    }

    #[test]
    fn test_extract_element_name() {
        assert_eq!(extract_element_name("code"), "code");
        assert_eq!(extract_element_name("component.code"), "code");
        assert_eq!(extract_element_name("identifier[type].system"), "system");
        assert_eq!(extract_element_name("extension[url].value"), "value");
    }
}
