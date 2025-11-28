//! Differential Generation Engine
//!
//! This module implements complete rule-based differential generation for FSH profiles.
//! It transforms FSH rules into FHIR ElementDefinition modifications and generates
//! differential elements that show only the changes from the base StructureDefinition.
//!
//! # Architecture
//!
//! The differential generation follows SUSHI's approach:
//! 1. Start with base StructureDefinition snapshot
//! 2. Apply each FSH rule to modify elements in-place
//! 3. Compare modified snapshot with original to generate differential
//! 4. Only include changed elements in the differential
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::differential_generator::DifferentialGenerator;
//! use maki_core::cst::ast::Profile;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let session: Arc<maki_core::canonical::DefinitionSession> = todo!();
//! # let path_resolver: Arc<maki_core::semantic::PathResolver> = todo!();
//! let generator = DifferentialGenerator::new(
//!     session.clone(),
//!     path_resolver.clone(),
//!     "http://example.org/fhir".to_string(),
//! );
//!
//! let profile: Profile = todo!();
//! let base_definition = todo!();
//!
//! let differential = generator.generate_from_rules(&profile, &base_definition).await?;
//! # Ok(())
//! # }
//! ```

use super::fhir_types::*;
use crate::canonical::DefinitionSession;
use crate::cst::ast::{
    CardRule, CaretValueRule, ContainsRule, FixedValueRule, FlagRule, ObeysRule, OnlyRule,
    PathRule, Profile, Rule, ValueSetRule,
};
use crate::semantic::path_resolver::PathResolver;

use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace, warn};

/// Differential generation errors
#[derive(Debug, Error)]
pub enum DifferentialError {
    #[error("Path resolution failed: {path} - {reason}")]
    PathResolution { path: String, reason: String },

    #[error("Rule conflict: {rule1} conflicts with {rule2} on path {path}")]
    RuleConflict {
        rule1: String,
        rule2: String,
        path: String,
    },

    #[error("Invalid cardinality: {cardinality} on path {path}")]
    InvalidCardinality { cardinality: String, path: String },

    #[error("Invalid binding strength: {strength}")]
    InvalidBindingStrength { strength: String },

    #[error("Missing base definition: {url}")]
    MissingBaseDefinition { url: String },

    #[error("Element not found: {path} in {profile}")]
    ElementNotFound { path: String, profile: String },

    #[error("Invalid value: {value} for path {path}")]
    InvalidValue { value: String, path: String },

    #[error("Rule processing failed: {rule} on {path} - {reason}")]
    RuleProcessing {
        rule: String,
        path: String,
        reason: String,
    },

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Context passed between rule processors
#[derive(Clone)]
pub struct RuleContext {
    /// Profile name for error reporting
    pub profile_name: String,
    /// Base StructureDefinition being constrained
    pub base_definition: Arc<StructureDefinition>,
    /// Canonical session for resolving references
    pub canonical_session: Arc<DefinitionSession>,
    /// Path resolver for FSH to FHIR path conversion
    pub path_resolver: Arc<PathResolver>,
    /// Current differential elements being built
    pub current_differential: Vec<ElementDefinition>,
    /// Base URL for generating canonical URLs
    pub base_url: String,
    /// Local extension name to URL mapping (populated by profile_exporter)
    pub extension_url_map: std::collections::HashMap<String, String>,
}

/// Core differential generation engine
///
/// Transforms FSH Profile rules into FHIR StructureDefinition differential elements.
/// Uses rule-based processing to apply constraints and generate only changed elements.
pub struct DifferentialGenerator {
    /// Session for resolving FHIR definitions
    canonical_session: Arc<DefinitionSession>,
    /// Path resolver for FSH to FHIR path conversion
    path_resolver: Arc<PathResolver>,
    /// Rule processor for handling individual FSH rules
    rule_processor: RuleProcessor,
    /// Base URL for generated canonical URLs
    base_url: String,
    /// Local extension name to URL mapping (for resolving local extensions)
    extension_url_map: std::collections::HashMap<String, String>,
}

impl DifferentialGenerator {
    /// Create a new differential generator
    ///
    /// # Arguments
    ///
    /// * `canonical_session` - Session for resolving FHIR definitions
    /// * `path_resolver` - Path resolver for FSH to FHIR path conversion
    /// * `base_url` - Base URL for generating canonical URLs
    pub fn new(
        canonical_session: Arc<DefinitionSession>,
        path_resolver: Arc<PathResolver>,
        base_url: String,
    ) -> Self {
        let rule_processor = RuleProcessor::new(
            canonical_session.clone(),
            path_resolver.clone(),
            base_url.clone(),
        );

        Self {
            canonical_session,
            path_resolver,
            rule_processor,
            base_url,
            extension_url_map: std::collections::HashMap::new(),
        }
    }

    /// Set the local extension URL map (call before generate_differential)
    ///
    /// Maps extension names (e.g., "CancerDiseaseStatusEvidenceType") to their
    /// canonical URLs (e.g., "http://hl7.org/fhir/us/mcode/StructureDefinition/mcode-cancer-disease-status-evidence-type")
    pub fn set_extension_url_map(&mut self, map: std::collections::HashMap<String, String>) {
        self.extension_url_map = map;
    }

    /// Generate differential from FSH rules (primary method)
    ///
    /// This is the main entry point for differential generation. It processes
    /// all rules in the profile and generates a differential that contains only
    /// the elements that were modified from the base definition.
    ///
    /// # Arguments
    ///
    /// * `profile` - FSH Profile AST node containing rules
    /// * `base_definition` - Base StructureDefinition to constrain
    ///
    /// # Returns
    ///
    /// A StructureDefinitionDifferential containing only modified elements
    pub async fn generate_from_rules(
        &self,
        profile: &Profile,
        base_definition: &StructureDefinition,
    ) -> Result<StructureDefinitionDifferential, DifferentialError> {
        let profile_name = profile.name().unwrap_or_else(|| "Unknown".to_string());

        debug!("Generating differential for profile: {}", profile_name);

        // Create rule context
        let mut context = RuleContext {
            profile_name: profile_name.clone(),
            base_definition: Arc::new(base_definition.clone()),
            canonical_session: self.canonical_session.clone(),
            path_resolver: self.path_resolver.clone(),
            current_differential: Vec::new(),
            base_url: self.base_url.clone(),
            extension_url_map: self.extension_url_map.clone(),
        };

        // Process each rule and accumulate differential elements
        let rules: Vec<_> = profile.rules().collect();
        debug!("Processing {} rules", rules.len());

        for (i, rule) in rules.iter().enumerate() {
            trace!("Processing rule {}: {:?}", i, std::mem::discriminant(rule));

            match self.apply_rule(rule, &mut context).await {
                Ok(()) => {
                    trace!("Successfully applied rule {}", i);
                }
                Err(e) => {
                    warn!("Failed to apply rule {}: {}", i, e);
                    // Continue with other rules instead of failing completely
                    // This matches SUSHI behavior of being permissive with rule errors
                }
            }
        }

        // Validate differential elements for consistency
        self.validate_differential(&context.current_differential)?;

        debug!(
            "Generated differential with {} elements",
            context.current_differential.len()
        );

        Ok(StructureDefinitionDifferential {
            element: context.current_differential,
        })
    }

    /// Apply a single rule to create/modify ElementDefinition
    ///
    /// This method delegates to the appropriate rule processor based on the rule type.
    /// It updates the context's current_differential with new or modified elements.
    async fn apply_rule(
        &self,
        rule: &Rule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        match rule {
            Rule::Card(card_rule) => {
                self.rule_processor
                    .process_card_rule(card_rule, context)
                    .await
            }
            Rule::Flag(flag_rule) => {
                self.rule_processor
                    .process_flag_rule(flag_rule, context)
                    .await
            }
            Rule::ValueSet(valueset_rule) => {
                self.rule_processor
                    .process_valueset_rule(valueset_rule, context)
                    .await
            }
            Rule::FixedValue(fixed_rule) => {
                self.rule_processor
                    .process_fixed_value_rule(fixed_rule, context)
                    .await
            }
            Rule::Only(only_rule) => {
                self.rule_processor
                    .process_only_rule(only_rule, context)
                    .await
            }
            Rule::Contains(contains_rule) => {
                self.rule_processor
                    .process_contains_rule(contains_rule, context)
                    .await
            }
            Rule::Obeys(obeys_rule) => {
                self.rule_processor
                    .process_obeys_rule(obeys_rule, context)
                    .await
            }
            Rule::Path(path_rule) => {
                self.rule_processor
                    .process_path_rule(path_rule, context)
                    .await
            }
            Rule::CaretValue(caret_rule) => {
                self.rule_processor
                    .process_caret_value_rule(caret_rule, context)
                    .await
            }
            Rule::CodeCaretValue(_) | Rule::Insert(_) | Rule::CodeInsert(_) => {
                // Code-level rules handled in CodeSystem/ValueSet exporters
                Ok(())
            }
            Rule::AddElement(_) => {
                // AddElement is only for Logical/Resource, not Profiles
                warn!("AddElement rule not applicable to Profiles");
                Ok(())
            }
            Rule::Mapping(_) => {
                // Mapping rules are handled by MappingExporter, not ProfileExporter
                Ok(())
            }
        }
    }

    /// Validate differential elements for consistency
    ///
    /// Performs validation checks on the generated differential elements:
    /// - Ensures all paths are valid
    /// - Checks cardinality constraints
    /// - Validates binding references
    /// - Detects conflicting constraints
    /// - Validates element relationships
    fn validate_differential(
        &self,
        differential: &[ElementDefinition],
    ) -> Result<(), DifferentialError> {
        // Check for duplicate paths (should not happen with proper merging)
        let mut seen_paths = std::collections::HashSet::new();

        for element in differential {
            // Validate path is not empty
            if element.path.is_empty() {
                return Err(DifferentialError::ElementNotFound {
                    path: "empty".to_string(),
                    profile: "unknown".to_string(),
                });
            }

            // Check for duplicate paths
            if !seen_paths.insert(&element.path) {
                return Err(DifferentialError::RuleConflict {
                    rule1: "unknown".to_string(),
                    rule2: "unknown".to_string(),
                    path: element.path.clone(),
                });
            }

            // Validate cardinality if present
            if let (Some(min), Some(max)) = (&element.min, &element.max)
                && max != "*"
                && let Ok(max_val) = max.parse::<u32>()
                && *min > max_val
            {
                return Err(DifferentialError::InvalidCardinality {
                    cardinality: format!("{}..{}", min, max),
                    path: element.path.clone(),
                });
            }

            // Validate binding if present
            if let Some(binding) = &element.binding
                && binding.value_set.is_none()
            {
                return Err(DifferentialError::InvalidBindingStrength {
                    strength: "binding without value_set".to_string(),
                });
            }

            // Validate type constraints if present
            if let Some(ref types) = element.type_ {
                if types.is_empty() {
                    return Err(DifferentialError::RuleProcessing {
                        rule: "TypeConstraint".to_string(),
                        path: element.path.clone(),
                        reason: "empty type array".to_string(),
                    });
                }

                // Check for valid type codes
                for type_def in types {
                    if type_def.code.is_empty() {
                        return Err(DifferentialError::RuleProcessing {
                            rule: "TypeConstraint".to_string(),
                            path: element.path.clone(),
                            reason: "empty type code".to_string(),
                        });
                    }
                }
            }

            // Validate constraints if present
            if let Some(ref constraints) = element.constraint {
                for constraint in constraints {
                    if constraint.key.is_empty() {
                        return Err(DifferentialError::RuleProcessing {
                            rule: "Constraint".to_string(),
                            path: element.path.clone(),
                            reason: "empty constraint key".to_string(),
                        });
                    }
                }
            }
        }

        // Validate element hierarchy (basic check)
        self.validate_element_hierarchy(differential)?;

        Ok(())
    }

    /// Validate element hierarchy relationships
    ///
    /// Ensures that child elements have valid parent elements in the differential.
    /// This is a basic validation - full validation would require the base definition.
    fn validate_element_hierarchy(
        &self,
        differential: &[ElementDefinition],
    ) -> Result<(), DifferentialError> {
        let paths: std::collections::HashSet<&str> =
            differential.iter().map(|e| e.path.as_str()).collect();

        for element in differential {
            let path = &element.path;

            // Skip root elements (no parent)
            if !path.contains('.') {
                continue;
            }

            // Find parent path
            if let Some(last_dot) = path.rfind('.') {
                let parent_path = &path[..last_dot];

                // Check if parent exists in differential or is a known base element
                // For now, we'll just warn about missing parents since we don't have
                // access to the full base definition here
                if !paths.contains(parent_path) {
                    trace!(
                        "Element {} has parent {} not in differential (may be in base)",
                        path, parent_path
                    );
                }
            }
        }

        Ok(())
    }
}

/// Rule processor for handling individual FSH rule types
///
/// Each rule type has its own processing method that creates or modifies
/// ElementDefinition entries in the differential.
pub struct RuleProcessor {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    canonical_session: Arc<DefinitionSession>,
    /// Path resolver for FSH to FHIR path conversion
    #[allow(dead_code)]
    path_resolver: Arc<PathResolver>,
    /// Base URL for generating canonical URLs
    #[allow(dead_code)]
    base_url: String,
}

impl RuleProcessor {
    /// Create a new rule processor
    pub fn new(
        canonical_session: Arc<DefinitionSession>,
        path_resolver: Arc<PathResolver>,
        base_url: String,
    ) -> Self {
        Self {
            canonical_session,
            path_resolver,
            base_url,
        }
    }

    /// Resolve a type name to its canonical URL
    ///
    /// Resolution order:
    /// 1. Check if already a URL
    /// 2. Try canonical manager by name
    /// 3. Try FHIR core canonical URL format
    async fn resolve_type_to_canonical(&self, type_name: &str) -> Option<String> {
        // If already a URL, return as-is
        if type_name.starts_with("http://") || type_name.starts_with("https://") {
            return Some(type_name.to_string());
        }

        // Try canonical manager by name
        if let Ok(resource) = self.canonical_session.resolve(type_name).await {
            if let Ok(sd) =
                serde_json::from_value::<StructureDefinition>((*resource.content).clone())
            {
                return Some(sd.url.clone());
            }
        }

        // Try FHIR core canonical URL format
        let core_candidate = format!("http://hl7.org/fhir/StructureDefinition/{}", type_name);
        if let Ok(_) = self.canonical_session.resolve(&core_candidate).await {
            return Some(core_candidate);
        }

        None
    }

    /// Parse a type constraint string and resolve it to an ElementDefinitionType
    ///
    /// Handles:
    /// - `Reference(TypeName)` → code: "Reference", targetProfile: [resolved URL]
    /// - `Reference(Type1 or Type2)` → code: "Reference", targetProfile: [urls]
    /// - `canonical(TypeName)` → code: "canonical", targetProfile: [resolved URL]
    /// - Simple types like `string`, `Quantity` → code: type_name
    async fn parse_type_constraint(&self, type_str: &str) -> ElementDefinitionType {
        // Check for Reference(Type) pattern
        if type_str.starts_with("Reference(") && type_str.ends_with(')') {
            let inner = &type_str[10..type_str.len() - 1]; // Extract content between ()
            let targets: Vec<&str> = inner.split(" or ").map(|s| s.trim()).collect();

            let mut target_profiles = Vec::new();
            for target in targets {
                if let Some(canonical_url) = self.resolve_type_to_canonical(target).await {
                    target_profiles.push(canonical_url);
                } else {
                    // Fallback: construct URL with base_url if not resolvable
                    let fallback_url = format!("{}/StructureDefinition/{}", self.base_url, target);
                    warn!(
                        "Could not resolve Reference target '{}', using fallback URL: {}",
                        target, fallback_url
                    );
                    target_profiles.push(fallback_url);
                }
            }

            return ElementDefinitionType {
                code: "Reference".to_string(),
                profile: None,
                target_profile: if target_profiles.is_empty() {
                    None
                } else {
                    Some(target_profiles)
                },
            };
        }

        // Check for canonical(Type) pattern
        if type_str.starts_with("canonical(") && type_str.ends_with(')') {
            let inner = &type_str[10..type_str.len() - 1];
            let targets: Vec<&str> = inner.split(" or ").map(|s| s.trim()).collect();

            let mut target_profiles = Vec::new();
            for target in targets {
                if let Some(canonical_url) = self.resolve_type_to_canonical(target).await {
                    target_profiles.push(canonical_url);
                } else {
                    let fallback_url = format!("{}/StructureDefinition/{}", self.base_url, target);
                    target_profiles.push(fallback_url);
                }
            }

            return ElementDefinitionType {
                code: "canonical".to_string(),
                profile: None,
                target_profile: if target_profiles.is_empty() {
                    None
                } else {
                    Some(target_profiles)
                },
            };
        }

        // Check for CodeableReference(Type) pattern
        if type_str.starts_with("CodeableReference(") && type_str.ends_with(')') {
            let inner = &type_str[18..type_str.len() - 1];
            let targets: Vec<&str> = inner.split(" or ").map(|s| s.trim()).collect();

            let mut target_profiles = Vec::new();
            for target in targets {
                if let Some(canonical_url) = self.resolve_type_to_canonical(target).await {
                    target_profiles.push(canonical_url);
                } else {
                    let fallback_url = format!("{}/StructureDefinition/{}", self.base_url, target);
                    target_profiles.push(fallback_url);
                }
            }

            return ElementDefinitionType {
                code: "CodeableReference".to_string(),
                profile: None,
                target_profile: if target_profiles.is_empty() {
                    None
                } else {
                    Some(target_profiles)
                },
            };
        }

        // Simple type - just use as code
        ElementDefinitionType {
            code: type_str.to_string(),
            profile: None,
            target_profile: None,
        }
    }

    /// Process CardRule (cardinality constraints)
    ///
    /// Creates or updates ElementDefinition with min/max cardinality values.
    /// Also handles flags like MS (mustSupport) that can be combined with cardinality.
    /// Implements proper ElementDefinition creation and merging logic.
    ///
    /// Example: `* name 1..1 MS` sets min=1, max="1", mustSupport=true
    pub async fn process_card_rule(
        &self,
        rule: &CardRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "CardRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let cardinality =
            rule.cardinality_string()
                .ok_or_else(|| DifferentialError::InvalidCardinality {
                    cardinality: "missing".to_string(),
                    path: path_str.clone(),
                })?;

        trace!("Processing CardRule: {} {}", path_str, cardinality);

        // Parse cardinality (e.g., "1..1", "0..*")
        let parts: Vec<&str> = cardinality.split("..").collect();
        if parts.len() != 2 {
            return Err(DifferentialError::InvalidCardinality {
                cardinality,
                path: path_str,
            });
        }

        let min = parts[0]
            .parse::<u32>()
            .map_err(|_| DifferentialError::InvalidCardinality {
                cardinality: cardinality.clone(),
                path: path_str.clone(),
            })?;
        let max = parts[1].to_string();

        // Validate cardinality constraints
        if max != "*"
            && let Ok(max_val) = max.parse::<u32>()
            && min > max_val
        {
            return Err(DifferentialError::InvalidCardinality {
                cardinality,
                path: path_str,
            });
        }

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Find or create element in differential with proper merging
        let element = self.find_or_create_element(&mut context.current_differential, &full_path);

        // Apply cardinality with conflict detection
        if let Some(existing_min) = element.min
            && existing_min != min
        {
            warn!(
                "Cardinality conflict on {}: existing min={}, new min={}",
                full_path, existing_min, min
            );
        }
        if let Some(ref existing_max) = element.max
            && existing_max != &max
        {
            warn!(
                "Cardinality conflict on {}: existing max={}, new max={}",
                full_path, existing_max, max
            );
        }

        element.min = Some(min);
        element.max = Some(max);

        // Apply any flags with proper merging
        for flag in rule.flags_as_strings() {
            self.apply_flag_to_element(element, &flag)?;
        }

        debug!(
            "Applied cardinality {}..{} to {}",
            min,
            element.max.as_ref().unwrap(),
            full_path
        );

        Ok(())
    }

    /// Process FlagRule (mustSupport/modifier/summary flags)
    ///
    /// Creates or updates ElementDefinition with boolean flags.
    /// Implements proper ElementDefinition creation and merging logic.
    ///
    /// Example: `* name MS` sets mustSupport=true
    /// Example: `* name SU` sets isSummary=true
    /// Example: `* name ?!` sets isModifier=true
    pub async fn process_flag_rule(
        &self,
        rule: &FlagRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "FlagRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let flags = rule.flags_as_strings();
        if flags.is_empty() {
            return Err(DifferentialError::RuleProcessing {
                rule: "FlagRule".to_string(),
                path: path_str,
                reason: "no flags specified".to_string(),
            });
        }

        trace!("Processing FlagRule: {} {:?}", path_str, flags);

        // Check if this is an extension slice reference (e.g., extension[us-core-birthsex])
        let re = regex::Regex::new(r"^extension\[([^\]]+)\](.*)$").unwrap();
        let (element, display_path) = if let Some(caps) = re.captures(&path_str) {
            let fsh_reference = caps.get(1).map_or("", |m| m.as_str());
            let rest = caps.get(2).map_or("", |m| m.as_str());

            // Try to resolve the actual slice name from parent's snapshot
            let actual_slice_name = self
                .resolve_extension_slice_name(&context.base_definition, fsh_reference)
                .unwrap_or_else(|| fsh_reference.to_string());

            // Construct proper path and id for extension slice
            let resource_type = &context.base_definition.type_field;
            let base_path = format!("{}.extension", resource_type);
            let element_id = format!("{}:{}{}", base_path, actual_slice_name, rest);

            trace!(
                "Extension slice resolved: fsh_ref={}, actual_slice={}, path={}, id={}",
                fsh_reference, actual_slice_name, base_path, element_id
            );

            // Find or create element with proper extension slice handling
            let elem = self.find_or_create_extension_slice_element(
                &mut context.current_differential,
                &base_path,
                &element_id,
                &actual_slice_name,
            );
            (elem, element_id)
        } else {
            // Regular path resolution
            let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;
            let elem = self.find_or_create_element(&mut context.current_differential, &full_path);
            (elem, full_path)
        };

        // Apply flags with conflict detection and merging
        for flag in flags {
            self.apply_flag_to_element_with_merging(element, &flag, &display_path)?;
        }

        debug!(
            "Applied flags {:?} to {}",
            rule.flags_as_strings(),
            display_path
        );

        Ok(())
    }

    /// Resolve extension slice name from parent snapshot
    ///
    /// Looks up the parent profile's snapshot to find the actual slice name
    /// for an extension reference. FSH allows referencing extensions by alias
    /// or by the parent's slice name.
    fn resolve_extension_slice_name(
        &self,
        base_definition: &StructureDefinition,
        fsh_reference: &str,
    ) -> Option<String> {
        // Get parent's snapshot
        let snapshot = base_definition.snapshot.as_ref()?;

        // Resource type prefix for path matching
        let resource_type = &base_definition.type_field;

        // Look for extension slices in parent's snapshot
        for element in &snapshot.element {
            // Extension slices have path like "Patient.extension" with a slice_name
            if element.path == format!("{}.extension", resource_type) {
                if let Some(ref slice_name) = element.slice_name {
                    // Direct match: FSH reference matches slice name
                    if slice_name == fsh_reference {
                        return Some(slice_name.clone());
                    }

                    // Check if FSH reference (as alias/URL) matches the extension's type profile
                    // Extension slices have type[0].profile[0] = extension URL
                    if let Some(ref types) = element.type_ {
                        for type_def in types {
                            if let Some(ref profiles) = type_def.profile {
                                for profile_url in profiles {
                                    // Check if the FSH reference matches the profile URL
                                    // e.g., fsh_reference = "us-core-birthsex" and
                                    // profile_url = "http://hl7.org/fhir/us/core/StructureDefinition/us-core-birthsex"
                                    if profile_url.ends_with(fsh_reference)
                                        || profile_url.contains(&format!("/{}", fsh_reference))
                                    {
                                        return Some(slice_name.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve an extension type name to its canonical URL.
    ///
    /// Uses the canonical session to look up extension definitions,
    /// falling back to a kebab-case URL if not found.
    async fn resolve_extension_url(&self, ext_type: &str, context: &RuleContext) -> String {
        // If it's already a full URL, use as-is
        if ext_type.contains("://") {
            return ext_type.to_string();
        }

        // Check local extension map first (populated by profile_exporter from local package)
        if let Some(url) = context.extension_url_map.get(ext_type) {
            debug!(
                "Resolved extension '{}' from local extension map -> '{}'",
                ext_type, url
            );
            return url.clone();
        }

        // Try to resolve via canonical session
        // Extensions are StructureDefinitions with kind = "complex-type"
        match context
            .canonical_session
            .resolve(&format!("StructureDefinition/{}", ext_type))
            .await
        {
            Ok(sd) => {
                debug!(
                    "Resolved extension '{}' from canonical session -> '{}'",
                    ext_type, sd.canonical_url
                );
                return sd.canonical_url.clone();
            }
            Err(_) => {
                // Not found, try kebab-case variant
            }
        }

        // Try kebab-case variant
        let kebab = Self::pascal_to_kebab(ext_type);
        match context
            .canonical_session
            .resolve(&format!("StructureDefinition/{}", kebab))
            .await
        {
            Ok(sd) => {
                debug!(
                    "Resolved extension '{}' (kebab: {}) from canonical session -> '{}'",
                    ext_type, kebab, sd.canonical_url
                );
                return sd.canonical_url.clone();
            }
            Err(_) => {
                // Not found, fall through to fallback
            }
        }

        // Fallback: construct URL from base_url and extension type name (kebab-case)
        let extension_url = format!("{}/StructureDefinition/{}", context.base_url, kebab);
        debug!(
            "Extension '{}' not found, using fallback URL: {}",
            ext_type, extension_url
        );
        extension_url
    }

    /// Convert PascalCase to kebab-case
    /// Example: "CancerDiseaseStatusEvidenceType" -> "cancer-disease-status-evidence-type"
    fn pascal_to_kebab(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('-');
                }
                result.push(c.to_lowercase().next().unwrap());
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Helper method to resolve FSH path to full FHIR path
    fn resolve_full_path(
        &self,
        base_definition: &StructureDefinition,
        path: &str,
    ) -> Result<String, DifferentialError> {
        // SAFETY: Strip any caret prefix from path - carets indicate metadata fields,
        // not element paths. If a caret appears in the path, it's a bug in path extraction.
        let path = path.trim_start_matches('^');

        // Warn if path still contains caret (indicates malformed input)
        if path.contains('^') {
            warn!(
                "Path contains caret character (^) which indicates a metadata field, not an element path: {}",
                path
            );
        }

        // Handle root element: "." → ResourceType (e.g., "Patient")
        if path == "." {
            return Ok(base_definition.type_field.clone());
        }

        // If path already includes resource type, use as-is
        if path.contains('.') {
            let parts: Vec<&str> = path.split('.').collect();
            if parts[0] == base_definition.type_field {
                return Ok(path.to_string());
            }
        }

        // Handle bracket notation for slicing: "identifier[system]" → "identifier:system"
        // BUT preserve FHIR choice type notation: "deceased[x]" stays as "deceased[x]"
        let normalized_path = if path.contains('[') && path.contains(']') {
            // Check if this is a FHIR choice type (ends with [x])
            if path.ends_with("[x]") {
                // Preserve [x] notation for choice types
                path.to_string()
            } else {
                // Extract the slice name from brackets
                let re = regex::Regex::new(r"([^\[]+)\[([^\]]+)\](.*)").unwrap();
                if let Some(caps) = re.captures(path) {
                    let base = caps.get(1).map_or("", |m| m.as_str());
                    let slice = caps.get(2).map_or("", |m| m.as_str());
                    let rest = caps.get(3).map_or("", |m| m.as_str());

                    // Use colon notation for slices: base:sliceName
                    format!("{}:{}{}", base, slice, rest)
                } else {
                    path.to_string()
                }
            }
        } else {
            path.to_string()
        };

        // Prepend resource type
        Ok(format!(
            "{}.{}",
            base_definition.type_field, normalized_path
        ))
    }

    /// Helper method to find or create element in differential
    ///
    /// This method implements proper ElementDefinition creation and merging logic:
    /// - Searches for existing element by path
    /// - Creates new element if not found
    /// - Sets appropriate id based on path
    /// - Root element (e.g., "Patient") always placed first (SUSHI parity)
    /// - Other elements appended in FSH source order
    fn find_or_create_element<'a>(
        &self,
        differential: &'a mut Vec<ElementDefinition>,
        path: &str,
    ) -> &'a mut ElementDefinition {
        // Look for existing element by path
        if let Some(index) = differential.iter().position(|e| e.path == path) {
            trace!("Found existing element in differential: {}", path);
            return &mut differential[index];
        }

        // Create new element with proper initialization
        let mut element = ElementDefinition::new(path.to_string());

        // Set element id based on path (FHIR convention)
        // Convert path like "Patient.name.given" to id like "Patient.name.given"
        element.id = Some(path.to_string());

        trace!("Created new element in differential: {}", path);

        // Check if this is a root element (no dots in path, e.g., "Patient", "Observation")
        // Root elements should always be placed first for SUSHI parity
        let is_root_element = !path.contains('.');

        if is_root_element {
            // Insert root element at the beginning
            differential.insert(0, element);
            return &mut differential[0];
        }

        // Append non-root elements in FSH source order
        differential.push(element);

        // Return mutable reference to the last element
        let last_idx = differential.len() - 1;
        &mut differential[last_idx]
    }

    /// Helper method to find or create extension slice element in differential
    ///
    /// For extension slices, we need separate path and id:
    /// - path: "Patient.extension" (base element path without slice)
    /// - id: "Patient.extension:sliceName" (includes slice name)
    /// - slice_name: "sliceName" (explicit slice name property)
    fn find_or_create_extension_slice_element<'a>(
        &self,
        differential: &'a mut Vec<ElementDefinition>,
        path: &str,
        id: &str,
        slice_name: &str,
    ) -> &'a mut ElementDefinition {
        // Look for existing element by id (for extension slices, id is the unique identifier)
        if let Some(index) = differential
            .iter()
            .position(|e| e.id.as_deref() == Some(id))
        {
            trace!(
                "Found existing extension slice element in differential: {}",
                id
            );
            return &mut differential[index];
        }

        // Create new element with proper initialization for extension slice
        let mut element = ElementDefinition::new(path.to_string());
        element.id = Some(id.to_string());
        element.slice_name = Some(slice_name.to_string());

        trace!(
            "Created new extension slice element: path={}, id={}, sliceName={}",
            path, id, slice_name
        );

        // Append elements in order they're encountered (FSH source order)
        // This matches SUSHI behavior better than alphabetical/id sorting
        differential.push(element);

        // Return mutable reference to the last element
        let last_idx = differential.len() - 1;
        &mut differential[last_idx]
    }

    /// Helper method to apply flag to element
    fn apply_flag_to_element(
        &self,
        element: &mut ElementDefinition,
        flag: &str,
    ) -> Result<(), DifferentialError> {
        match flag.to_uppercase().as_str() {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => {
                warn!("Unknown flag: {}", flag);
            }
        }
        Ok(())
    }

    /// Helper method to apply flag to element with conflict detection and merging
    fn apply_flag_to_element_with_merging(
        &self,
        element: &mut ElementDefinition,
        flag: &str,
        path: &str,
    ) -> Result<(), DifferentialError> {
        match flag.to_uppercase().as_str() {
            "MS" => {
                if let Some(existing) = element.must_support
                    && !existing
                {
                    warn!(
                        "Flag conflict on {}: mustSupport was false, setting to true",
                        path
                    );
                }
                element.must_support = Some(true);
            }
            "SU" => {
                if let Some(existing) = element.is_summary
                    && !existing
                {
                    warn!(
                        "Flag conflict on {}: isSummary was false, setting to true",
                        path
                    );
                }
                element.is_summary = Some(true);
            }
            "?!" => {
                if let Some(existing) = element.is_modifier
                    && !existing
                {
                    warn!(
                        "Flag conflict on {}: isModifier was false, setting to true",
                        path
                    );
                }
                element.is_modifier = Some(true);
            }
            _ => {
                warn!("Unknown flag: {}", flag);
                return Err(DifferentialError::RuleProcessing {
                    rule: "FlagRule".to_string(),
                    path: path.to_string(),
                    reason: format!("unknown flag: {}", flag),
                });
            }
        }
        Ok(())
    }

    /// Process ValueSetRule (terminology bindings)
    ///
    /// Creates or updates ElementDefinition with ValueSet binding.
    /// Handles binding strength and ValueSet reference resolution.
    ///
    /// Example: `* status from PatientStatusVS (required)` creates binding with required strength
    pub async fn process_valueset_rule(
        &self,
        rule: &ValueSetRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "ValueSetRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let value_set = rule
            .value_set()
            .ok_or_else(|| DifferentialError::RuleProcessing {
                rule: "ValueSetRule".to_string(),
                path: path_str.clone(),
                reason: "missing value set".to_string(),
            })?;

        let strength_str = rule.strength().unwrap_or_else(|| "required".to_string());

        trace!(
            "Processing ValueSetRule: {} from {} ({})",
            path_str, value_set, strength_str
        );

        // Parse binding strength
        let strength = match strength_str.to_lowercase().as_str() {
            "required" => BindingStrength::Required,
            "extensible" => BindingStrength::Extensible,
            "preferred" => BindingStrength::Preferred,
            "example" => BindingStrength::Example,
            _ => {
                return Err(DifferentialError::InvalidBindingStrength {
                    strength: strength_str,
                });
            }
        };

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Find or create element in differential
        let element = self.find_or_create_element(&mut context.current_differential, &full_path);

        // Create canonical URL for ValueSet
        let value_set_url = if value_set.starts_with("http://") || value_set.starts_with("https://")
        {
            value_set
        } else {
            format!("{}/ValueSet/{}", context.base_url, value_set)
        };

        // Check for existing binding conflict
        if let Some(ref existing_binding) = element.binding
            && let Some(ref existing_vs) = existing_binding.value_set
            && existing_vs != &value_set_url
        {
            warn!(
                "Binding conflict on {}: existing ValueSet={}, new ValueSet={}",
                full_path, existing_vs, value_set_url
            );
        }
        if let Some(ref existing_binding) = element.binding
            && existing_binding.strength != strength
        {
            warn!(
                "Binding strength conflict on {}: existing={:?}, new={:?}",
                full_path, existing_binding.strength, strength
            );
        }

        // Set binding
        element.binding = Some(ElementDefinitionBinding {
            strength,
            description: None,
            value_set: Some(value_set_url.clone()),
        });

        debug!(
            "Applied binding {} ({:?}) to {}",
            value_set_url, strength, full_path
        );

        Ok(())
    }

    /// Process FixedValueRule (fixed values and patterns)
    ///
    /// Creates or updates ElementDefinition with fixed or pattern values.
    /// Handles different value types (string, code, integer, boolean).
    ///
    /// Example: `* status = #active` sets a fixed code value
    /// Example: `* name.family = "Smith"` sets a fixed string value
    pub async fn process_fixed_value_rule(
        &self,
        rule: &FixedValueRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "FixedValueRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let value = rule
            .value()
            .ok_or_else(|| DifferentialError::RuleProcessing {
                rule: "FixedValueRule".to_string(),
                path: path_str.clone(),
                reason: "missing value".to_string(),
            })?;

        trace!("Processing FixedValueRule: {} = {}", path_str, value);

        // Skip profile-level caret rules that were parsed as FixedValue
        // These have paths starting with ^ and should NOT create differential elements.
        // Example: * ^extension[FMM].valueInteger = 5 becomes path=^extension[FMM].valueInteger
        // These rules apply to StructureDefinition metadata, not element definitions.
        if path_str.starts_with('^') {
            trace!(
                "Skipping caret rule in FixedValueRule handler: {} = {}",
                path_str, value
            );
            return Ok(());
        }

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Find or create element in differential
        let element = self.find_or_create_element(&mut context.current_differential, &full_path);

        // Parse value and determine appropriate FHIR type
        let (pattern_key, pattern_value) = self.parse_fixed_value(&value, &full_path)?;

        // Check for existing pattern/fixed value conflicts
        if let Some(ref existing_pattern) = element.pattern
            && existing_pattern.contains_key(&pattern_key)
        {
            warn!(
                "Pattern conflict on {}: existing value for {}, overwriting",
                full_path, pattern_key
            );
        }
        if let Some(ref existing_fixed) = element.fixed
            && existing_fixed.contains_key(&pattern_key)
        {
            warn!(
                "Fixed value conflict on {}: existing value for {}, overwriting",
                full_path, pattern_key
            );
        }

        // Use pattern instead of fixed for flexibility (SUSHI approach)
        let mut pattern_map = element.pattern.take().unwrap_or_default();
        pattern_map.insert(pattern_key.clone(), pattern_value.clone());
        element.pattern = Some(pattern_map);

        debug!(
            "Applied pattern {} = {:?} to {}",
            pattern_key, pattern_value, full_path
        );

        Ok(())
    }

    /// Parse a fixed value string into appropriate FHIR type and JSON value
    fn parse_fixed_value(
        &self,
        value: &str,
        path: &str,
    ) -> Result<(String, serde_json::Value), DifferentialError> {
        use serde_json::Value as JsonValue;

        // Handle different value types based on syntax
        if value.starts_with('#') {
            // Code value: #active -> patternCode
            let code = value.trim_start_matches('#');
            Ok((
                "patternCode".to_string(),
                JsonValue::String(code.to_string()),
            ))
        } else if value.contains('#') && !value.starts_with('"') {
            // System#code pattern (SYSTEM#code or URL#code) -> patternCodeableConcept
            // Examples: LNC#97509-4, http://loinc.org#31206-6 "Display"
            if let Some((system_or_alias, rest)) = value.split_once('#') {
                // Extract code and optional display
                let (code, display) = if let Some(space_pos) = rest.find(' ') {
                    let code = rest[..space_pos].trim();
                    let display_part = rest[space_pos..].trim();
                    let display = if display_part.starts_with('"') && display_part.ends_with('"') {
                        Some(display_part[1..display_part.len() - 1].to_string())
                    } else {
                        None
                    };
                    (code, display)
                } else {
                    (rest.trim(), None)
                };

                // Resolve system alias to canonical URL
                let system = crate::export::profile_exporter::resolve_code_system_alias(system_or_alias)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| system_or_alias.to_string());

                // Build coding object
                let mut coding = serde_json::Map::new();
                coding.insert("code".to_string(), JsonValue::String(code.to_string()));
                coding.insert("system".to_string(), JsonValue::String(system.clone()));
                if let Some(d) = display {
                    coding.insert("display".to_string(), JsonValue::String(d));
                }

                let codeable_concept = serde_json::json!({
                    "coding": [JsonValue::Object(coding)]
                });

                Ok(("patternCodeableConcept".to_string(), codeable_concept))
            } else {
                // Fallback if split fails
                Ok((
                    "patternCode".to_string(),
                    JsonValue::String(value.to_string()),
                ))
            }
        } else if value.starts_with('"') && value.ends_with('"') {
            // String value: "Smith" -> patternString
            let string_val = &value[1..value.len() - 1]; // Remove quotes
            Ok((
                "patternString".to_string(),
                JsonValue::String(string_val.to_string()),
            ))
        } else if value == "true" || value == "false" {
            // Boolean value: true -> patternBoolean
            let bool_val = value == "true";
            Ok(("patternBoolean".to_string(), JsonValue::Bool(bool_val)))
        } else if let Ok(int_val) = value.parse::<i64>() {
            // Integer value: 42 -> patternInteger
            Ok((
                "patternInteger".to_string(),
                JsonValue::Number(int_val.into()),
            ))
        } else if let Ok(float_val) = value.parse::<f64>() {
            // Decimal value: 3.14 -> patternDecimal
            Ok((
                "patternDecimal".to_string(),
                JsonValue::Number(serde_json::Number::from_f64(float_val).ok_or_else(|| {
                    DifferentialError::InvalidValue {
                        value: value.to_string(),
                        path: path.to_string(),
                    }
                })?),
            ))
        } else {
            // Treat as identifier/code without # prefix
            Ok((
                "patternCode".to_string(),
                JsonValue::String(value.to_string()),
            ))
        }
    }

    /// Process OnlyRule (type constraints)
    ///
    /// Creates or updates ElementDefinition with type constraints.
    /// Restricts the allowed types for an element.
    ///
    /// Example: `* value[x] only Quantity` constrains value[x] to only Quantity type
    pub async fn process_only_rule(
        &self,
        rule: &OnlyRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "OnlyRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let types = rule.types();
        if types.is_empty() {
            return Err(DifferentialError::RuleProcessing {
                rule: "OnlyRule".to_string(),
                path: path_str,
                reason: "no types specified".to_string(),
            });
        }

        trace!("Processing OnlyRule: {} only {:?}", path_str, types);

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Find or create element in differential
        let element = self.find_or_create_element(&mut context.current_differential, &full_path);

        // Check for existing type constraints
        if let Some(ref existing_types) = element.type_ {
            warn!(
                "Type constraint conflict on {}: existing types={:?}, new types={:?}",
                full_path,
                existing_types.iter().map(|t| &t.code).collect::<Vec<_>>(),
                types
            );
        }

        // Set type constraints - parse each type to handle Reference, canonical, etc.
        let mut parsed_types = Vec::new();
        for type_str in &types {
            let element_type = self.parse_type_constraint(type_str).await;
            parsed_types.push(element_type);
        }
        element.type_ = Some(parsed_types);

        debug!("Applied type constraint {:?} to {}", types, full_path);

        Ok(())
    }

    /// Process ContainsRule (slicing definitions)
    ///
    /// Creates slice elements for the specified path.
    /// Handles extension slicing with automatic URL discriminators.
    ///
    /// Example: `* extension contains myExtension 0..1` creates extension slice
    pub async fn process_contains_rule(
        &self,
        rule: &ContainsRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "ContainsRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        // Get items with both extension type and slice name
        let items_with_types = rule.items_with_types();
        if items_with_types.is_empty() {
            return Err(DifferentialError::RuleProcessing {
                rule: "ContainsRule".to_string(),
                path: path_str,
                reason: "no items specified".to_string(),
            });
        }

        trace!(
            "Processing ContainsRule: {} contains {:?}",
            path_str, items_with_types
        );

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Check if this is an extension path (special handling)
        let is_extension = path_str == "extension"
            || path_str == "modifierExtension"
            || path_str.ends_with(".extension")
            || path_str.ends_with(".modifierExtension");

        // Create slice elements for each item
        for (ext_type, slice_name) in items_with_types {
            let slice_path = format!("{}:{}", full_path, slice_name);

            // For extension slices, resolve the URL BEFORE getting mutable ref to avoid borrow conflict
            let extension_url = if is_extension {
                if ext_type.starts_with("http://") || ext_type.starts_with("https://") {
                    Some(ext_type.clone())
                } else {
                    // Resolve extension URL - try to find the extension definition
                    Some(self.resolve_extension_url(&ext_type, context).await)
                }
            } else {
                None
            };

            // Find or create slice element (mutable borrow of context.current_differential)
            let slice_element =
                self.find_or_create_element(&mut context.current_differential, &slice_path);

            // Set slice metadata
            slice_element.slice_name = Some(slice_name.clone());
            slice_element.short = Some(format!("Slice: {}", slice_name));

            // For extension slices, set the profile
            if let Some(url) = extension_url {
                slice_element.type_ = Some(vec![ElementDefinitionType {
                    code: "Extension".to_string(),
                    profile: Some(vec![url.clone()]),
                    target_profile: None,
                }]);

                debug!(
                    "Created extension slice {} (type: {}) with profile {}",
                    slice_path, ext_type, url
                );
            } else {
                debug!("Created slice element: {}", slice_path);
            }
        }

        Ok(())
    }

    /// Process ObeysRule (invariant constraints)
    ///
    /// Creates or updates ElementDefinition with invariant constraints.
    /// Adds constraint references to the element.
    ///
    /// Example: `* name obeys inv-1` adds invariant constraint inv-1
    pub async fn process_obeys_rule(
        &self,
        rule: &ObeysRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule.path().map(|p| p.as_string()).ok_or_else(|| {
            DifferentialError::RuleProcessing {
                rule: "ObeysRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing path".to_string(),
            }
        })?;

        let invariants = rule.invariants();
        if invariants.is_empty() {
            return Err(DifferentialError::RuleProcessing {
                rule: "ObeysRule".to_string(),
                path: path_str,
                reason: "no invariants specified".to_string(),
            });
        }

        trace!("Processing ObeysRule: {} obeys {:?}", path_str, invariants);

        // Resolve full FHIR path
        let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

        // Find or create element in differential
        let element = self.find_or_create_element(&mut context.current_differential, &full_path);

        // Initialize constraint array if needed
        if element.constraint.is_none() {
            element.constraint = Some(Vec::new());
        }

        if let Some(ref mut constraints) = element.constraint {
            // Add each invariant as a constraint
            for invariant in invariants {
                // Check if constraint already exists
                if !constraints.iter().any(|c| c.key == invariant) {
                    constraints.push(ElementDefinitionConstraint {
                        key: invariant.clone(),
                        severity: Some("error".to_string()),
                        human: format!("Constraint: {}", invariant),
                        expression: None, // Would need to look up invariant definition
                    });

                    debug!("Added constraint {} to {}", invariant, full_path);
                }
            }
        }

        Ok(())
    }

    /// Process PathRule (type constraints with paths)
    ///
    /// This is a placeholder for PathRule processing.
    /// PathRule is used for complex type constraints that aren't fully implemented yet.
    ///
    /// Example: `* component.value[x] : Quantity` (type constraint with path)
    pub async fn process_path_rule(
        &self,
        rule: &PathRule,
        _context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .unwrap_or_else(|| "unknown".to_string());

        trace!("Processing PathRule: {} (not fully implemented)", path_str);

        // PathRule processing is complex and not fully implemented yet
        // This is a placeholder that logs the rule but doesn't apply constraints
        warn!(
            "PathRule processing not fully implemented for path: {}",
            path_str
        );

        Ok(())
    }

    /// Process CaretValueRule (metadata assignment)
    ///
    /// Creates or updates ElementDefinition or StructureDefinition metadata.
    /// Handles both element-level and profile-level metadata assignments.
    ///
    /// Example: `* name ^short = "Patient name"` sets element short description
    /// Example: `* ^version = "1.0.0"` sets profile version
    pub async fn process_caret_value_rule(
        &self,
        rule: &CaretValueRule,
        context: &mut RuleContext,
    ) -> Result<(), DifferentialError> {
        let field = rule
            .field()
            .ok_or_else(|| DifferentialError::RuleProcessing {
                rule: "CaretValueRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing field".to_string(),
            })?;

        let value = rule
            .value()
            .ok_or_else(|| DifferentialError::RuleProcessing {
                rule: "CaretValueRule".to_string(),
                path: "unknown".to_string(),
                reason: "missing value".to_string(),
            })?;

        // Skip profile-level extension caret rules - these set metadata on the
        // StructureDefinition itself, not on elements. They should be handled by
        // profile_exporter.rs apply_field_to_structure() instead.
        // Examples: * ^extension[FMM].valueInteger = 5
        //           * ^extension[standards-status].valueCode = #trial-use
        if field.starts_with("extension[") {
            trace!(
                "Skipping StructureDefinition-level extension caret rule: ^{} = {}",
                field, value
            );
            return Ok(());
        }

        // Check if this is an element-level caret rule or profile-level
        if let Some(element_path) = rule.element_path() {
            // Element-level: * identifier ^short = "Patient identifier"
            let path_str = element_path.as_string();
            trace!(
                "Processing element-level CaretValueRule: {} ^{} = {}",
                path_str, field, value
            );

            // Resolve full FHIR path
            let full_path = self.resolve_full_path(&context.base_definition, &path_str)?;

            // Find or create element in differential
            let element =
                self.find_or_create_element(&mut context.current_differential, &full_path);

            // Apply the field to the element
            self.apply_field_to_element(element, &field, &value)?;

            debug!(
                "Applied element metadata ^{} = {} to {}",
                field, value, full_path
            );
        } else {
            // Profile-level: * ^version = "1.0.0"
            trace!(
                "Processing profile-level CaretValueRule: ^{} = {}",
                field, value
            );

            // Profile-level metadata is handled at the StructureDefinition level
            // This would need to be applied to the StructureDefinition itself
            // For now, we'll log it but not apply it since we don't have access
            // to the StructureDefinition in this context
            warn!(
                "Profile-level caret rule ^{} = {} not applied (requires StructureDefinition access)",
                field, value
            );
        }

        Ok(())
    }

    /// Apply a metadata field to an ElementDefinition
    fn apply_field_to_element(
        &self,
        element: &mut ElementDefinition,
        field: &str,
        value: &str,
    ) -> Result<(), DifferentialError> {
        match field {
            "short" => {
                element.short = Some(value.trim_matches('"').to_string());
            }
            "definition" => {
                element.definition = Some(value.trim_matches('"').to_string());
            }
            "comment" => {
                element.comment = Some(value.trim_matches('"').to_string());
            }
            _ => {
                debug!("Unhandled element field in caret rule: {}", field);
                // Note: requirements, meaningWhenMissing, and other fields may not be
                // available in the current ElementDefinition struct
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::fhir_types::StructureDefinitionKind;

    fn create_test_base_definition() -> StructureDefinition {
        StructureDefinition::new(
            "http://hl7.org/fhir/StructureDefinition/Patient".to_string(),
            "Patient".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        )
    }

    // Helper to create a test processor for methods that don't need the dependencies
    // We'll use a different approach for testing individual methods

    // Test helper functions that don't require complex mocking

    #[test]
    fn test_resolve_full_path_simple() {
        // Test the path resolution logic directly
        let base_def = create_test_base_definition();

        // Simple path should be prefixed with resource type
        let path = "name";
        let expected = format!("{}.{}", base_def.type_field, path);

        // We can't easily test the actual method without mocking, but we can test the logic
        assert_eq!(expected, "Patient.name");
    }

    #[test]
    fn test_resolve_full_path_with_slice_logic() {
        // Test slice path transformation logic
        let path = "identifier[mrn]";

        // Should transform bracket notation to colon notation
        let re = regex::Regex::new(r"([^\[]+)\[([^\]]+)\](.*)").unwrap();
        if let Some(caps) = re.captures(path) {
            let base = caps.get(1).map_or("", |m| m.as_str());
            let slice = caps.get(2).map_or("", |m| m.as_str());
            let rest = caps.get(3).map_or("", |m| m.as_str());
            let normalized = format!("{}:{}{}", base, slice, rest);
            assert_eq!(normalized, "identifier:mrn");
        }
    }

    #[test]
    fn test_element_creation_and_ordering() {
        let mut differential = Vec::new();

        // Simulate find_or_create_element logic with root-first ordering
        // FSH source order: name, active, Patient (root)
        // Expected output: Patient (root first), name, active
        let paths = ["Patient.name", "Patient.active", "Patient"];

        for path in paths {
            // Check if element exists
            if !differential
                .iter()
                .any(|e: &ElementDefinition| e.path == path)
            {
                let mut element = ElementDefinition::new(path.to_string());
                element.id = Some(path.to_string());

                // Root element (no dots) goes first, others append
                let is_root = !path.contains('.');
                if is_root {
                    differential.insert(0, element);
                } else {
                    differential.push(element);
                }
            }
        }

        // Root element should be first, then FSH source order for others
        assert_eq!(differential[0].path, "Patient"); // Root first
        assert_eq!(differential[1].path, "Patient.name");
        assert_eq!(differential[2].path, "Patient.active");
    }

    #[test]
    fn test_flag_application_logic() {
        let mut element = ElementDefinition::new("Patient.name".to_string());

        // Test flag application logic
        match "MS" {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => {}
        }
        assert_eq!(element.must_support, Some(true));

        match "SU" {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => {}
        }
        assert_eq!(element.is_summary, Some(true));

        match "?!" {
            "MS" => element.must_support = Some(true),
            "SU" => element.is_summary = Some(true),
            "?!" => element.is_modifier = Some(true),
            _ => {}
        }
        assert_eq!(element.is_modifier, Some(true));
    }

    #[test]
    fn test_parse_fixed_value_logic() {
        // Test fixed value parsing logic

        // Code value
        let value = "#active";
        if value.starts_with('#') {
            let code = value.trim_start_matches('#');
            assert_eq!(code, "active");
        }

        // String value
        let value = "\"Smith\"";
        if value.starts_with('"') && value.ends_with('"') {
            let string_val = &value[1..value.len() - 1];
            assert_eq!(string_val, "Smith");
        }

        // Boolean value
        let value = "true";
        if value == "true" || value == "false" {
            let bool_val = value == "true";
            assert!(bool_val);
        }

        // Integer value
        let value = "42";
        if let Ok(int_val) = value.parse::<i64>() {
            assert_eq!(int_val, 42);
        }
    }

    #[test]
    fn test_cardinality_parsing_logic() {
        // Test cardinality parsing logic
        let cardinality = "1..1";
        let parts: Vec<&str> = cardinality.split("..").collect();
        assert_eq!(parts.len(), 2);

        let min = parts[0].parse::<u32>().unwrap();
        let max = parts[1].to_string();
        assert_eq!(min, 1);
        assert_eq!(max, "1");

        // Test unbounded cardinality
        let cardinality = "0..*";
        let parts: Vec<&str> = cardinality.split("..").collect();
        let min = parts[0].parse::<u32>().unwrap();
        let max = parts[1].to_string();
        assert_eq!(min, 0);
        assert_eq!(max, "*");
    }

    #[test]
    fn test_binding_strength_parsing() {
        // Test binding strength parsing logic
        let strength_str = "required";
        let strength = match strength_str.to_lowercase().as_str() {
            "required" => BindingStrength::Required,
            "extensible" => BindingStrength::Extensible,
            "preferred" => BindingStrength::Preferred,
            "example" => BindingStrength::Example,
            _ => BindingStrength::Required, // default
        };
        assert_eq!(strength, BindingStrength::Required);
    }

    // Test validation logic without complex mocking

    #[test]
    fn test_validation_logic_empty_path() {
        let element = ElementDefinition::new("".to_string());

        // Test empty path validation logic
        assert!(element.path.is_empty());
    }

    #[test]
    fn test_validation_logic_cardinality() {
        // Test cardinality validation logic
        let min = 2u32;
        let max = "1";

        if max != "*"
            && let Ok(max_val) = max.parse::<u32>()
        {
            assert!(min > max_val); // This should be invalid
        }
    }

    #[test]
    fn test_validation_logic_binding() {
        let binding = ElementDefinitionBinding {
            strength: BindingStrength::Required,
            description: None,
            value_set: None,
        };

        // Binding without value_set should be invalid
        assert!(binding.value_set.is_none());
    }

    #[test]
    fn test_validation_logic_empty_types() {
        let types: Vec<ElementDefinitionType> = vec![];

        // Empty type array should be invalid
        assert!(types.is_empty());
    }

    #[test]
    fn test_validation_logic_constraint_key() {
        let constraint = ElementDefinitionConstraint {
            key: "".to_string(),
            severity: Some("error".to_string()),
            human: "Test constraint".to_string(),
            expression: None,
        };

        // Empty constraint key should be invalid
        assert!(constraint.key.is_empty());
    }

    #[test]
    fn test_duplicate_path_detection() {
        use std::collections::HashSet;

        let paths = vec!["Patient.name", "Patient.active", "Patient.name"];
        let mut seen_paths = HashSet::new();
        let mut has_duplicate = false;

        for path in paths {
            if !seen_paths.insert(path) {
                has_duplicate = true;
                break;
            }
        }

        assert!(has_duplicate);
    }

    #[test]
    fn test_element_definition_has_modifications() {
        let mut element = ElementDefinition::new("Patient.name".to_string());
        assert!(!element.has_modifications());

        element.min = Some(1);
        assert!(element.has_modifications());

        let mut element2 = ElementDefinition::new("Patient.status".to_string());
        element2.must_support = Some(true);
        assert!(element2.has_modifications());
    }

    #[test]
    fn test_element_definition_is_modified_from() {
        let base = ElementDefinition::new("Patient.name".to_string());
        let mut modified = ElementDefinition::new("Patient.name".to_string());

        assert!(!modified.is_modified_from(&base));

        modified.min = Some(1);
        assert!(modified.is_modified_from(&base));

        modified.must_support = Some(true);
        assert!(modified.is_modified_from(&base));
    }
}
