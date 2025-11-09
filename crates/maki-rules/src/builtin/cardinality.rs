//! Cardinality validation rules
//!
//! Validates cardinality constraints in FSH profiles and extensions.
//!
//! Rules:
//! - `blocking/valid-cardinality`: Validates min ≤ max and detects 0..0
//! - `blocking/invalid-cardinality`: Syntax validation for cardinality format

use maki_core::cst::ast::{AstNode, CardRule, Document};
use maki_core::{CodeSuggestion, Diagnostic, Location, SemanticModel, Severity};

/// Rule ID for cardinality validation
pub const VALID_CARDINALITY: &str = "blocking/valid-cardinality";
pub const INVALID_CARDINALITY: &str = "blocking/invalid-cardinality";
pub const CARDINALITY_CONFLICTS: &str = "correctness/cardinality-conflicts";
pub const CARDINALITY_TOO_RESTRICTIVE: &str = "correctness/cardinality-too-restrictive";

/// Detect cardinality conflict patterns
/// Identifies suspicious cardinality patterns that often indicate parent/child conflicts
fn detect_conflict_patterns(card: &Cardinality, location: Location) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Pattern 1: Unbounded on a typically bounded field with high minimum
    // This is suspicious because most FHIR elements have bounded parent cardinality
    if card.max.is_none() && card.min > 1 {
        diagnostics.push(
            Diagnostic::new(
                CARDINALITY_CONFLICTS,
                Severity::Warning,
                format!(
                    "Cardinality {}..* is unbounded with minimum {}. \
                     This may conflict with parent element cardinality. \
                     Verify this is intentional.",
                    card.min, card.min
                ),
                location,
            )
            .with_code("unbounded-high-min".to_string()),
        );
    }

    diagnostics
}

/// Detect overly restrictive cardinality changes
fn detect_overly_restrictive(card: &Cardinality, location: Location) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Pattern 1: Exactly 1 is very restrictive from 0..*
    if card.min == 1 && card.max == Some(1) {
        diagnostics.push(
            Diagnostic::new(
                CARDINALITY_TOO_RESTRICTIVE,
                Severity::Warning,
                "Cardinality 1..1 makes this element required and exactly once. \
                 This is very restrictive - ensure this is intentional."
                    .to_string(),
                location.clone(),
            )
            .with_code("required-exactly-one".to_string()),
        );
    }

    // Pattern 2: Multiple required instances
    if card.min >= 2 {
        diagnostics.push(
            Diagnostic::new(
                CARDINALITY_TOO_RESTRICTIVE,
                Severity::Warning,
                format!(
                    "Cardinality {}..{} requires at least {} instances. \
                     This is very restrictive - ensure this is intentional.",
                    card.min,
                    card.max.map(|m| m.to_string()).unwrap_or("*".to_string()),
                    card.min
                ),
                location,
            )
            .with_code("multiple-required".to_string()),
        );
    }

    diagnostics
}

/// Check if a cardinality change from parent to child is too restrictive.
///
/// This is when a child narrows the cardinality significantly from the parent.
/// Examples:
/// - Parent 0..*, child 1..1 - requires when parent is optional
/// - Parent 0..5, child 0..1 - significantly reduces upper bound
///
/// Returns (is_too_restrictive, message)
#[allow(dead_code)]
fn is_cardinality_too_restrictive(
    parent: &Cardinality,
    child: &Cardinality,
) -> (bool, Option<String>) {
    // If parent was unrestricted (0..*), and child requires at least 1,
    // this is a significant restriction
    if parent.min == 0 && child.min > 0 {
        return (
            true,
            Some(format!(
                "Making optional element required (parent: {}..*, child: {}..)",
                parent.min, child.min
            )),
        );
    }

    // If child's max is significantly lower than parent's max
    match (parent.max, child.max) {
        (Some(parent_max), Some(child_max)) if parent_max > 0 && child_max <= (parent_max / 2) => {
            return (
                true,
                Some(format!(
                    "Significantly reducing upper bound (parent: ..{}, child: ..{})",
                    parent_max, child_max
                )),
            );
        }
        _ => {}
    }

    (false, None)
}

/// Check cardinality rules (Phase 1: Syntax validation)
pub fn check_cardinality(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles
    for profile in document.profiles() {
        let card_rules: Vec<CardRule> = profile
            .rules()
            .filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            })
            .collect();

        diagnostics.extend(check_resource_cardinality_rules(
            card_rules.into_iter(),
            model,
        ));
    }

    // Check extensions
    for extension in document.extensions() {
        let card_rules: Vec<CardRule> = extension
            .rules()
            .filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            })
            .collect();

        diagnostics.extend(check_resource_cardinality_rules(
            card_rules.into_iter(),
            model,
        ));
    }

    diagnostics
}

/// Check cardinality conflicts against parent (Phase 2: Semantic validation)
/// This function validates cardinality patterns that often indicate conflicts.
///
/// When a DefinitionSession is provided (via the rule engine), it will validate
/// against actual FHIR parent definitions. Otherwise, it performs pattern-based validation.
pub async fn check_cardinality_conflicts(
    model: &SemanticModel,
    session: Option<&maki_core::canonical::DefinitionSession>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Collect profiles first to avoid holding CST iterators across await
    let profiles: Vec<_> = document.profiles().collect();

    for profile in profiles {
        diagnostics.extend(check_profile_cardinality_conflicts(&profile, model, session).await);
    }

    diagnostics
}

/// Check a profile for cardinality conflicts with parent
async fn check_profile_cardinality_conflicts(
    profile: &maki_core::cst::ast::Profile,
    model: &SemanticModel,
    session: Option<&maki_core::canonical::DefinitionSession>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect all cardinality rules and their data before any async operations
    let card_rules: Vec<_> = profile
        .rules()
        .filter_map(|r| match r {
            maki_core::cst::ast::Rule::Card(c) => {
                let text = c.syntax().text().to_string().trim().to_string();
                let range = c.syntax().text_range();
                Some((c, text, range))
            }
            _ => None,
        })
        .collect();

    // If we have a session and parent, try to resolve parent cardinality
    if let Some(session) = session
        && let Some(parent) = profile.parent()
        && let Some(parent_name) = parent.value()
    {
        // Resolve parent StructureDefinition asynchronously
        if let Ok(Some(parent_sd)) = session.resolve_structure_definition(&parent_name).await {
            // Check each cardinality rule against parent
            for (rule, cardinality_text, text_range) in card_rules {
                if let Some(child_card) = Cardinality::from_str(&cardinality_text) {
                    // Get element path
                    let path_str = rule
                        .path()
                        .map(|p| p.syntax().text().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    // Find parent element cardinality
                    if let Some(parent_card) = find_element_cardinality(&parent_sd, &path_str) {
                        // Check if child violates parent constraints
                        if !child_card.is_valid_refinement(&parent_card) {
                            let span = text_range.start().into()..text_range.end().into();
                            let location = model.source_map.span_to_diagnostic_location(
                                &span,
                                &model.source,
                                &model.source_file,
                            );

                            diagnostics.push(
                                Diagnostic::new(
                                    CARDINALITY_CONFLICTS,
                                    Severity::Error,
                                    format!(
                                        "Cardinality {} for element '{}' conflicts with parent '{}' cardinality {}",
                                        child_card.as_string(),
                                        path_str,
                                        parent_name,
                                        parent_card.as_string()
                                    ),
                                    location,
                                )
                                .with_code("cardinality-conflict".to_string()),
                            );
                        }
                    } else {
                        // No parent cardinality found - use heuristic pattern detection
                        let span = text_range.start().into()..text_range.end().into();
                        let location = model.source_map.span_to_diagnostic_location(
                            &span,
                            &model.source,
                            &model.source_file,
                        );
                        diagnostics.extend(detect_conflict_patterns(&child_card, location));
                    }
                }
            }
            return diagnostics;
        }
    }

    // Fallback: No session or couldn't resolve parent - use heuristic pattern detection
    for (_, cardinality_text, text_range) in card_rules {
        if let Some(child_card) = Cardinality::from_str(&cardinality_text) {
            let span = text_range.start().into()..text_range.end().into();
            let location = model.source_map.span_to_diagnostic_location(
                &span,
                &model.source,
                &model.source_file,
            );
            diagnostics.extend(detect_conflict_patterns(&child_card, location));
        }
    }

    diagnostics
}

/// Find cardinality for a specific element in a StructureDefinition
fn find_element_cardinality(
    sd: &maki_core::export::StructureDefinition,
    element_path: &str,
) -> Option<Cardinality> {
    let resource_type: &str = sd.resource_type.as_ref();
    let full_path = if element_path == "." || element_path.is_empty() {
        resource_type.to_string()
    } else {
        format!("{}.{}", resource_type, element_path)
    };

    // Look through snapshot elements first
    if let Some(snapshot) = &sd.snapshot {
        for element in &snapshot.element {
            if (element.path == full_path || element.path.ends_with(&format!(".{}", element_path)))
                && element.min.is_some()
            {
                return Some(Cardinality {
                    min: element.min.unwrap_or(0),
                    max: element
                        .max
                        .as_deref()
                        .and_then(|m| if m == "*" { None } else { m.parse().ok() }),
                });
            }
        }
    }

    // Fall back to differential
    if let Some(differential) = &sd.differential {
        for element in &differential.element {
            if (element.path == full_path || element.path.ends_with(&format!(".{}", element_path)))
                && element.min.is_some()
            {
                return Some(Cardinality {
                    min: element.min.unwrap_or(0),
                    max: element
                        .max
                        .as_deref()
                        .and_then(|m| if m == "*" { None } else { m.parse().ok() }),
                });
            }
        }
    }

    None
}

/// Check if a child cardinality is a valid refinement of parent cardinality
impl Cardinality {
    fn is_valid_refinement(&self, parent: &Cardinality) -> bool {
        // Child min must be >= parent min
        if self.min < parent.min {
            return false;
        }

        // Child max must be <= parent max
        match (self.max, parent.max) {
            (Some(child_max), Some(parent_max)) => child_max <= parent_max,
            (Some(_), None) => true,  // Child bounded, parent unbounded: OK
            (None, Some(_)) => false, // Child unbounded, parent bounded: NOT OK
            (None, None) => true,     // Both unbounded: OK
        }
    }
}

/// Check for cardinality being overly restrictive (Phase 2: Warning detection)
pub fn check_cardinality_too_restrictive(model: &SemanticModel) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(document) = Document::cast(model.cst.clone()) else {
        return diagnostics;
    };

    // Check profiles for overly restrictive cardinality
    for profile in document.profiles() {
        let card_rules: Vec<CardRule> = profile
            .rules()
            .filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            })
            .collect();

        for rule in card_rules {
            let cardinality_text = rule.syntax().text().to_string().trim().to_string();

            // Parse cardinality
            if let Some(child_card) = Cardinality::from_str(&cardinality_text) {
                let location = model.source_map.node_to_diagnostic_location(
                    rule.syntax(),
                    &model.source,
                    &model.source_file,
                );

                // Check if this cardinality is suspiciously restrictive
                // We'll use default parent assumptions for common scenarios
                diagnostics.extend(detect_overly_restrictive(&child_card, location));
            }
        }
    }

    diagnostics
}

/// Represents a cardinality range: (min, max)
/// max is None to represent "*" (unbounded)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cardinality {
    min: u32,
    max: Option<u32>,
}

impl Cardinality {
    /// Create a cardinality from min and max values
    #[allow(dead_code)]
    fn new(min: u32, max: Option<u32>) -> Self {
        Self { min, max }
    }

    /// Parse cardinality from string like "0..1" or "1..*"
    fn from_str(s: &str) -> Option<Self> {
        let (min_str, max_str) = s.trim().split_once("..")?;
        let min = min_str.trim().parse::<u32>().ok()?;
        let max = if max_str.trim() == "*" {
            None
        } else {
            Some(max_str.trim().parse::<u32>().ok()?)
        };
        Some(Self { min, max })
    }

    /// Check if this cardinality is a valid subset of parent cardinality
    /// In FHIR, child cardinality must be more restrictive than parent:
    /// - child_min >= parent_min
    /// - child_max <= parent_max (respecting unbounded)
    #[allow(dead_code)]
    fn is_subset_of(&self, parent: &Cardinality) -> bool {
        // Child minimum must be >= parent minimum
        if self.min < parent.min {
            return false;
        }

        // Child maximum must be <= parent maximum
        match (self.max, parent.max) {
            // Child is bounded, parent is unbounded: always valid
            (Some(_), None) => true,
            // Child is unbounded, parent is bounded: invalid (child is less restrictive)
            (None, Some(_)) => false,
            // Both bounded: child max must be <= parent max
            (Some(child_max), Some(parent_max)) => child_max <= parent_max,
            // Both unbounded: valid (same flexibility)
            (None, None) => true,
        }
    }

    /// Convert to string representation for error messages
    #[allow(dead_code)]
    fn as_string(&self) -> String {
        match self.max {
            Some(max) => format!("{}..{}", self.min, max),
            None => format!("{}.*", self.min),
        }
    }
}

/// Check cardinality rules in a resource
fn check_resource_cardinality_rules(
    rules: impl Iterator<Item = CardRule>,
    model: &SemanticModel,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in rules {
        // Parse cardinality from rule text directly
        let cardinality_text = rule.syntax().text().to_string().trim().to_string();

        if let Some((min_str, max_str)) = cardinality_text.split_once("..") {
            let min_result = min_str.trim().parse::<u32>();
            let max_str_trimmed = max_str.trim();
            let max_result = if max_str_trimmed == "*" {
                Ok("*".to_string())
            } else {
                max_str_trimmed.parse::<u32>().map(|n| n.to_string())
            };

            if let (Ok(min), Ok(max_str)) = (min_result, max_result) {
                let location = model.source_map.node_to_diagnostic_location(
                    rule.syntax(),
                    &model.source,
                    &model.source_file,
                );

                // Check for reversed cardinality (min > max)
                #[allow(clippy::collapsible_if)]
                if max_str != "*" {
                    if let Ok(max) = max_str.parse::<u32>() {
                        if max < min {
                            // Generate fixed version
                            let fixed_cardinality = format!("{}..{}", max, min);
                            diagnostics.push(
                                Diagnostic::new(
                                    VALID_CARDINALITY,
                                    Severity::Error,
                                    format!(
                                        "Invalid cardinality: minimum ({}) cannot be greater than maximum ({}). \
                                         Cardinality must be MIN..MAX where MIN ≤ MAX. \
                                         Valid examples: 0..1, 1..*, 0..0, 2..5",
                                        min, max
                                    ),
                                    location.clone(),
                                )
                                .with_code("reversed-cardinality".to_string())
                                .with_suggestion(CodeSuggestion::safe(
                                    format!("Swap to {}..{}", max, min),
                                    fixed_cardinality,
                                    location.clone(),
                                )),
                            );
                        }
                    }
                }

                // Check for 0..0 (prohibited element)
                if min == 0 && max_str == "0" {
                    diagnostics.push(
                        Diagnostic::new(
                            VALID_CARDINALITY,
                            Severity::Warning,
                            "Cardinality 0..0 explicitly prohibits this element. This is valid but unusual - confirm this is intentional.".to_string(),
                            location,
                        )
                        .with_code("prohibited-element".to_string()),
                    );
                }
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use maki_core::cst::parse_fsh;
    use std::path::PathBuf;

    fn create_test_model(source: &str) -> maki_core::SemanticModel {
        let (cst, _, _) = parse_fsh(source);
        let source_map = maki_core::SourceMap::new(source);
        maki_core::SemanticModel {
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
    fn test_valid_cardinality_detects_reversed() {
        let source = r#"Profile: MyProfile
Parent: Patient
Id: my-profile
Title: "My Profile"
* name 5..3
"#;
        let model = create_test_model(source);

        // Debug: Check if profile was parsed
        let Some(document) = Document::cast(model.cst.clone()) else {
            panic!("Failed to parse document");
        };
        let profiles: Vec<_> = document.profiles().collect();
        assert_eq!(profiles.len(), 1, "Should have one profile");

        // Debug: Check if card rules exist
        let card_rules: Vec<_> = profiles[0]
            .rules()
            .filter_map(|r| match r {
                maki_core::cst::ast::Rule::Card(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(
            card_rules.len(),
            1,
            "Profile should have one card rule, got {}",
            card_rules.len()
        );

        let diagnostics = check_cardinality(&model);
        assert!(
            !diagnostics.is_empty(),
            "Should detect reversed cardinality"
        );
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("minimum"));
        assert!(diagnostics[0].message.contains("greater than"));
    }

    #[test]
    fn test_valid_cardinality_allows_correct_range() {
        let source = r#"Profile: MyProfile
Parent: Patient
Id: my-profile
Title: "My Profile"
* name 0..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_cardinality_allows_unbounded() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 1..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_cardinality_detects_zero_zero() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* extension 0..0
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert!(diagnostics[0].message.contains("0..0"));
        assert!(diagnostics[0].message.contains("prohibit"));
    }

    #[test]
    fn test_cardinality_with_suggestion_has_autofix() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 5..3
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(!diagnostics.is_empty());
        assert!(!diagnostics[0].suggestions.is_empty());
        assert_eq!(diagnostics[0].suggestions[0].replacement, "3..5");
    }

    #[test]
    fn test_valid_cardinality_multiple_rules() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 0..1
* birthDate 1..1
* extension 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_cardinality_mixed_valid_invalid() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 0..1
* birthDate 5..2
* extension 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        // Should have one error for reversed cardinality
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        assert_eq!(errors, 1);
    }

    #[test]
    fn test_extension_cardinality() {
        let source = r#"
Extension: MyExtension
* value[x] 1..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_extension_invalid_cardinality() {
        let source = r#"
Extension: MyExtension
* value[x] 3..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn test_large_reversed_cardinality() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 100..10
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(!diagnostics.is_empty());
        let suggestions = &diagnostics[0].suggestions;
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].replacement, "10..100");
    }

    #[test]
    fn test_cardinality_with_zero_min() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 0..5
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_exact_cardinality_zero() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 5..5
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty());
    }

    // Tests for Cardinality struct
    #[test]
    fn test_cardinality_from_str_bounded() {
        let card = Cardinality::from_str("0..1");
        assert!(card.is_some());
        let card = card.unwrap();
        assert_eq!(card.min, 0);
        assert_eq!(card.max, Some(1));
    }

    #[test]
    fn test_cardinality_from_str_unbounded() {
        let card = Cardinality::from_str("1..*");
        assert!(card.is_some());
        let card = card.unwrap();
        assert_eq!(card.min, 1);
        assert_eq!(card.max, None);
    }

    #[test]
    fn test_cardinality_from_str_zero_unbounded() {
        let card = Cardinality::from_str("0..*");
        assert!(card.is_some());
        let card = card.unwrap();
        assert_eq!(card.min, 0);
        assert_eq!(card.max, None);
    }

    #[test]
    fn test_cardinality_from_str_with_spaces() {
        let card = Cardinality::from_str("  0  ..  1  ");
        assert!(card.is_some());
        let card = card.unwrap();
        assert_eq!(card.min, 0);
        assert_eq!(card.max, Some(1));
    }

    #[test]
    fn test_cardinality_from_str_invalid_format() {
        let card = Cardinality::from_str("0-1");
        assert!(card.is_none());
    }

    #[test]
    fn test_cardinality_from_str_invalid_min() {
        let card = Cardinality::from_str("abc..1");
        assert!(card.is_none());
    }

    #[test]
    fn test_cardinality_from_str_invalid_max() {
        let card = Cardinality::from_str("0..abc");
        assert!(card.is_none());
    }

    // Tests for is_subset_of() - exact matches
    #[test]
    fn test_is_subset_exact_match_bounded() {
        let child = Cardinality::new(0, Some(1));
        let parent = Cardinality::new(0, Some(1));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_exact_match_unbounded() {
        let child = Cardinality::new(1, None);
        let parent = Cardinality::new(1, None);
        assert!(child.is_subset_of(&parent));
    }

    // Tests for is_subset_of() - valid subsets (more restrictive)
    #[test]
    fn test_is_subset_narrower_min() {
        let child = Cardinality::new(1, Some(1));
        let parent = Cardinality::new(0, Some(1));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_narrower_max() {
        let child = Cardinality::new(0, Some(1));
        let parent = Cardinality::new(0, Some(5));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_narrower_both() {
        let child = Cardinality::new(1, Some(2));
        let parent = Cardinality::new(0, Some(5));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_child_bounded_parent_unbounded() {
        let child = Cardinality::new(0, Some(10));
        let parent = Cardinality::new(0, None);
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_both_unbounded() {
        let child = Cardinality::new(0, None);
        let parent = Cardinality::new(0, None);
        assert!(child.is_subset_of(&parent));
    }

    // Tests for is_subset_of() - invalid subsets (less restrictive)
    #[test]
    fn test_is_subset_min_too_low() {
        let child = Cardinality::new(0, Some(1));
        let parent = Cardinality::new(1, Some(1));
        assert!(!child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_max_too_high() {
        let child = Cardinality::new(0, Some(5));
        let parent = Cardinality::new(0, Some(1));
        assert!(!child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_child_unbounded_parent_bounded() {
        let child = Cardinality::new(0, None);
        let parent = Cardinality::new(0, Some(5));
        assert!(!child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_both_min_and_max_too_low() {
        let child = Cardinality::new(0, Some(1));
        let parent = Cardinality::new(1, Some(5));
        assert!(!child.is_subset_of(&parent));
    }

    // Edge cases
    #[test]
    fn test_is_subset_zero_to_zero() {
        let child = Cardinality::new(0, Some(0));
        let parent = Cardinality::new(0, Some(1));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_zero_to_zero_exact_match() {
        let child = Cardinality::new(0, Some(0));
        let parent = Cardinality::new(0, Some(0));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_large_numbers() {
        let child = Cardinality::new(100, Some(200));
        let parent = Cardinality::new(50, Some(500));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_is_subset_max_at_boundary() {
        let child = Cardinality::new(0, Some(5));
        let parent = Cardinality::new(0, Some(5));
        assert!(child.is_subset_of(&parent));
    }

    #[test]
    fn test_cardinality_to_string_bounded() {
        let card = Cardinality::new(0, Some(1));
        assert_eq!(card.as_string(), "0..1");
    }

    #[test]
    fn test_cardinality_to_string_unbounded() {
        let card = Cardinality::new(1, None);
        assert_eq!(card.as_string(), "1.*");
    }

    // Tests for is_cardinality_too_restrictive
    #[test]
    fn test_too_restrictive_makes_required() {
        let parent = Cardinality::new(0, None);
        let child = Cardinality::new(1, Some(5));
        let (is_restrictive, msg) = is_cardinality_too_restrictive(&parent, &child);
        assert!(is_restrictive);
        assert!(msg.is_some());
    }

    #[test]
    fn test_too_restrictive_reduces_max_by_half() {
        let parent = Cardinality::new(0, Some(10));
        let child = Cardinality::new(0, Some(5));
        let (is_restrictive, msg) = is_cardinality_too_restrictive(&parent, &child);
        assert!(is_restrictive);
        assert!(msg.is_some());
    }

    #[test]
    fn test_not_too_restrictive_when_both_optional() {
        let parent = Cardinality::new(0, None);
        let child = Cardinality::new(0, Some(5));
        let (is_restrictive, msg) = is_cardinality_too_restrictive(&parent, &child);
        assert!(!is_restrictive);
        assert!(msg.is_none());
    }

    #[test]
    fn test_not_too_restrictive_small_reduction() {
        let parent = Cardinality::new(0, Some(10));
        let child = Cardinality::new(0, Some(8));
        let (is_restrictive, msg) = is_cardinality_too_restrictive(&parent, &child);
        assert!(!is_restrictive);
        assert!(msg.is_none());
    }

    #[test]
    fn test_not_too_restrictive_same_bounds() {
        let parent = Cardinality::new(1, Some(5));
        let child = Cardinality::new(1, Some(5));
        let (is_restrictive, msg) = is_cardinality_too_restrictive(&parent, &child);
        assert!(!is_restrictive);
        assert!(msg.is_none());
    }

    // Integration tests for Phase 1 through Phase 2 flow
    #[test]
    fn test_cardinality_phase1_and_phase2_workflow() {
        let source = r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
Title: "Test Profile"
* name 1..1
* birthDate 0..1
* address 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        // All cardinalities are valid
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_phase1_catches_syntax_error_phase2_ready() {
        let source = r#"
Profile: TestProfile
Parent: Patient
Id: test-profile
Title: "Test Profile"
* name 5..2
* birthDate 3..7
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        // Phase 1: Should catch the reversed cardinality error
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(
            errors.len(),
            1,
            "Should detect one reversed cardinality error"
        );
        assert!(
            !errors[0].suggestions.is_empty(),
            "Should have autofix suggestion"
        );
    }

    #[test]
    fn test_cardinality_with_extension() {
        let source = r#"
Extension: MyExtension
* value[x] 1..1
* value[x] only string or CodeableConcept
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(
            diagnostics.is_empty(),
            "Extension cardinality should be valid"
        );
    }

    #[test]
    fn test_cardinality_complex_profile() {
        let source = r#"
Profile: ComplexProfile
Parent: Patient
* identifier 0..*
* identifier.system 1..1
* identifier.value 1..1
* name 0..*
* name.given 1..*
* name.family 0..1
* contact 0..*
* contact.telecom 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        assert!(diagnostics.is_empty(), "All cardinalities should be valid");
    }

    #[test]
    fn test_cardinality_boundary_conditions() {
        let source = r#"
Profile: BoundaryTest
Parent: Patient
* name 0..0
* birthDate 1..*
* address 2..100
* telecom 0..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        // Only 0..0 should generate a warning
        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1, "Should have one warning for 0..0");
    }

    #[test]
    fn test_cardinality_multiple_errors_and_warnings() {
        let source = r#"
Profile: MixedProfile
Parent: Patient
* name 5..2
* birthDate 0..0
* address 3..1
* telecom 0..1
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality(&model);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(
            errors.len(),
            2,
            "Should have two reversed cardinality errors"
        );
        assert_eq!(warnings.len(), 1, "Should have one 0..0 warning");
    }

    // Tests for Phase 2 rule functions
    #[tokio::test]
    async fn test_check_cardinality_conflicts_rule() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 2..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality_conflicts(&model, None).await;
        // Should detect unbounded with high minimum as suspicious
        let has_warning = diagnostics
            .iter()
            .any(|d| d.rule_id == CARDINALITY_CONFLICTS && d.severity == Severity::Warning);
        assert!(has_warning, "Should warn about unbounded high minimum");
    }

    #[test]
    fn test_check_cardinality_too_restrictive_rule() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 1..1
* identifier 2..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality_too_restrictive(&model);
        // Should detect 1..1 and multiple required as restrictive
        assert!(
            !diagnostics.is_empty(),
            "Should detect overly restrictive patterns"
        );
        let has_restrictive = diagnostics
            .iter()
            .any(|d| d.rule_id == CARDINALITY_TOO_RESTRICTIVE && d.severity == Severity::Warning);
        assert!(has_restrictive, "Should warn about restrictive cardinality");
    }

    #[tokio::test]
    async fn test_cardinality_conflicts_normal_cardinality() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 0..1
* birthDate 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality_conflicts(&model, None).await;
        // Normal cardinality should not trigger conflicts warning
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_cardinality_too_restrictive_normal() {
        let source = r#"
Profile: MyProfile
Parent: Patient
* name 0..1
* birthDate 0..*
"#;
        let model = create_test_model(source);
        let diagnostics = check_cardinality_too_restrictive(&model);
        // Normal cardinality should not trigger restrictive warnings
        assert!(diagnostics.is_empty());
    }

    #[tokio::test]
    async fn test_phase2_rules_integration() {
        let source = r#"
Profile: RestrictiveProfile
Parent: Patient
* name 1..1
* identifier 3..*
* birthDate 0..0
"#;
        let model = create_test_model(source);

        // Phase 1: Syntax validation
        let phase1_diags = check_cardinality(&model);
        let phase1_warnings: Vec<_> = phase1_diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(phase1_warnings.len(), 1, "Phase 1: Should warn about 0..0");

        // Phase 2a: Conflict detection
        let conflict_diags = check_cardinality_conflicts(&model, None).await;
        let _has_conflicts = conflict_diags
            .iter()
            .any(|d| d.rule_id == CARDINALITY_CONFLICTS);
        // May or may not have conflicts depending on patterns

        // Phase 2b: Restrictive detection
        let restrictive_diags = check_cardinality_too_restrictive(&model);
        let restrictive_count = restrictive_diags
            .iter()
            .filter(|d| d.rule_id == CARDINALITY_TOO_RESTRICTIVE)
            .count();
        assert!(
            restrictive_count >= 2,
            "Phase 2b: Should detect multiple restrictive patterns"
        );
    }
}
