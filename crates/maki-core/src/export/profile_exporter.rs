//! Profile Exporter
//!
//! Exports FSH Profile definitions to FHIR StructureDefinition resources.
//! This is the core of the FSH-to-FHIR transformation pipeline.
//!
//! # Algorithm
//!
//! 1. **Get Base Definition**: Resolve parent StructureDefinition from FHIR packages
//! 2. **Apply Metadata**: Set profile metadata (name, title, description, etc.)
//! 3. **Apply Rules**: Transform each FSH rule into FHIR ElementDefinition modifications
//! 4. **Generate Differential**: Compare modified snapshot with base to create differential
//! 5. **Validate**: Ensure exported profile is valid FHIR
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::ProfileExporter;
//! use maki_core::cst::ast::Profile;
//! use maki_core::canonical::DefinitionSession;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let session: Arc<DefinitionSession> = todo!();
//! let exporter = ProfileExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Parse profile from FSH
//! let profile: Profile = todo!();
//!
//! // Export to StructureDefinition
//! let structure_def = exporter.export(&profile).await?;
//! # Ok(())
//! # }
//! ```

use super::differential_generator::DifferentialGenerator;
use super::fhir_types::*;
use crate::canonical::{CanonicalLoaderError, DefinitionSession};
use crate::cst::ast::{
    CardRule, CaretValueRule, ContainsRule, FixedValueRule, FlagRule, ObeysRule, OnlyRule, Profile,
    Rule, ValueSetRule,
};
use crate::semantic::path_resolver::PathResolver;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use thiserror::Error;

/// Extensions that should not be inherited from parent StructureDefinition
/// These are removed when creating a new profile (SUSHI compatible behavior)
const UNINHERITED_EXTENSIONS: &[&str] = &[
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-fmm",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-fmm-no-warnings",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-hierarchy",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-interface",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-normative-version",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-applicable-version",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-category",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-codegen-super",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-security-category",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-standards-status",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-summary",
    "http://hl7.org/fhir/StructureDefinition/structuredefinition-wg",
    "http://hl7.org/fhir/StructureDefinition/replaces",
    "http://hl7.org/fhir/StructureDefinition/resource-approvalDate",
    "http://hl7.org/fhir/StructureDefinition/resource-effectivePeriod",
    "http://hl7.org/fhir/StructureDefinition/resource-lastReviewDate",
];
use tracing::{debug, trace, warn};

/// Convert CamelCase to kebab-case (e.g., "USCorePatient" -> "us-core-patient")
fn camel_to_kebab(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_lower = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && prev_was_lower {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
            prev_was_lower = false;
        } else {
            result.push(ch);
            prev_was_lower = ch.is_lowercase();
        }
    }

    result
}

/// Profile export errors
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Parent not found: {0}")]
    ParentNotFound(String),

    #[error("Element not found: {path} in {profile}")]
    ElementNotFound { path: String, profile: String },

    #[error("Invalid cardinality: {0}")]
    InvalidCardinality(String),

    #[error("Invalid type: {0}")]
    InvalidType(String),

    #[error("Invalid binding strength: {0}")]
    InvalidBindingStrength(String),

    #[error("Rule application failed: {rule} on {path}: {reason}")]
    RuleApplicationFailed {
        rule: String,
        path: String,
        reason: String,
    },

    #[error("Path resolution failed: {0}")]
    PathResolutionFailed(String),

    #[error("Canonical resolution error: {0}")]
    CanonicalError(String),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    #[error("Invalid path: {path} in {resource}")]
    InvalidPath { path: String, resource: String },

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Invalid reference: {reference} - {reason}")]
    InvalidReference { reference: String, reason: String },

    #[error("Invalid context expression: {0}")]
    InvalidContextExpression(String),
}

/// Profile exporter
///
/// Transforms FSH Profile AST nodes into FHIR StructureDefinition resources.
pub struct ProfileExporter {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
    /// Path resolver for finding elements
    path_resolver: Arc<PathResolver>,
    /// Differential generator for rule-based differential creation
    differential_generator: DifferentialGenerator,
    /// Base URL for generated profiles
    base_url: String,
    /// Whether to generate snapshots (default: false to match SUSHI)
    generate_snapshots: bool,
    /// Version from config
    version: Option<String>,
    /// Status from config (draft | active | retired | unknown)
    status: Option<String>,
    /// Publisher from config
    publisher: Option<String>,
}

impl ProfileExporter {
    /// Create a new profile exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving base definitions
    /// * `base_url` - Base URL for generated profile canonical URLs
    /// * `version` - Version from configuration
    /// * `status` - Status from configuration (draft | active | retired | unknown)
    /// * `publisher` - Publisher from configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use maki_core::export::ProfileExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = ProfileExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    ///     Some("1.0.0".to_string()),
    ///     Some("draft".to_string()),
    ///     Some("Example Org".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
        version: Option<String>,
        status: Option<String>,
        publisher: Option<String>,
    ) -> Result<Self, ExportError> {
        let path_resolver = Arc::new(PathResolver::new(session.clone()));
        let differential_generator =
            DifferentialGenerator::new(session.clone(), path_resolver.clone(), base_url.clone());

        Ok(Self {
            session,
            path_resolver,
            differential_generator,
            base_url,
            generate_snapshots: false, // Default OFF to match SUSHI
            version,
            status,
            publisher,
        })
    }

    /// Set whether to generate snapshots
    pub fn set_generate_snapshots(&mut self, generate: bool) {
        self.generate_snapshots = generate;
    }

    /// Export a Profile to StructureDefinition
    ///
    /// # Arguments
    ///
    /// * `profile` - FSH Profile AST node
    ///
    /// # Returns
    ///
    /// A FHIR StructureDefinition with differential populated
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Parent definition not found
    /// - Rule application fails
    /// - Required fields missing
    pub async fn export(&self, profile: &Profile) -> Result<StructureDefinition, ExportError> {
        let profile_name = profile
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("profile name".to_string()))?;

        eprintln!(
            "[PROFILE EXPORT] Step 1: Starting export for profile: {}",
            profile_name
        );
        debug!("Exporting profile: {}", profile_name);

        // 1. Get parent type
        let parent = profile
            .parent()
            .and_then(|p| p.value())
            .ok_or_else(|| ExportError::MissingRequiredField("parent".to_string()))?;

        eprintln!("[PROFILE EXPORT] Step 2: Got parent type: {}", parent);
        debug!("Parent type: {}", parent);

        // 2. Get base StructureDefinition
        eprintln!(
            "[PROFILE EXPORT] Step 3: About to call get_base_structure_definition for: {}",
            parent
        );
        let mut structure_def = self.get_base_structure_definition(&parent).await?;
        eprintln!("[PROFILE EXPORT] Step 4: Successfully got base structure definition");

        // FIX Bug 2: Set baseDefinition to point to the parent, not parent's parent
        let base_definition_url = format!("http://hl7.org/fhir/StructureDefinition/{}", parent);
        structure_def.base_definition = Some(base_definition_url);

        // 3. Apply metadata
        self.apply_metadata(&mut structure_def, profile)?;

        // 4. CAPTURE ORIGINAL STATE (SUSHI approach)
        // After metadata is applied but BEFORE rules are applied,
        // capture the snapshot. This is what we'll compare against to generate differential.
        let original_snapshot = structure_def.snapshot.clone();
        let original_mappings = structure_def.mapping.clone();

        if let Some(ref snap) = original_snapshot {
            debug!("Original snapshot has {} elements", snap.element.len());
        } else {
            debug!("Original snapshot is None");
        }

        // 5. Apply all rules to snapshot (modifying in-place)
        let all_rules: Vec<_> = profile.rules().collect();
        debug!("Profile has {} rules", all_rules.len());
        for (i, rule) in all_rules.iter().enumerate() {
            debug!("  Rule {}: {:?}", i, std::mem::discriminant(rule));
            if let Err(e) = self.apply_rule(&mut structure_def, rule).await {
                warn!("Failed to apply rule: {}", e);
                // Continue with other rules instead of failing completely
            }
        }

        if let Some(ref snap) = structure_def.snapshot {
            debug!("Modified snapshot has {} elements", snap.element.len());
        }

        // 6. Generate differential using the new rule-based approach
        match self
            .differential_generator
            .generate_from_rules(profile, &structure_def)
            .await
        {
            Ok(differential) => {
                structure_def.differential = Some(differential);
            }
            Err(e) => {
                warn!("Failed to generate differential using new approach: {}", e);
                // Fallback to old snapshot comparison approach
                structure_def.differential = Some(self.generate_differential_from_snapshot(
                    &original_snapshot,
                    &structure_def.snapshot,
                    &original_mappings,
                    &structure_def.mapping,
                ));
            }
        }

        // 7. Clear snapshot if not requested (to match SUSHI default behavior)
        if !self.generate_snapshots {
            structure_def.snapshot = None;
        }

        // 8. Validate exported structure
        self.validate_structure_definition(&structure_def)?;

        debug!("Successfully exported profile: {}", profile_name);
        Ok(structure_def)
    }

    /// Validate exported StructureDefinition
    fn validate_structure_definition(
        &self,
        structure_def: &StructureDefinition,
    ) -> Result<(), ExportError> {
        // Check required fields
        if structure_def.url.is_empty() {
            return Err(ExportError::MissingRequiredField("url".to_string()));
        }
        if structure_def.name.is_empty() {
            return Err(ExportError::MissingRequiredField("name".to_string()));
        }
        if structure_def.type_field.is_empty() {
            return Err(ExportError::MissingRequiredField("type".to_string()));
        }

        // Validate differential elements if present
        if let Some(differential) = &structure_def.differential {
            for element in &differential.element {
                self.validate_element_definition(element)?;
            }
        }

        Ok(())
    }

    /// Validate an ElementDefinition
    fn validate_element_definition(&self, element: &ElementDefinition) -> Result<(), ExportError> {
        // Check path is not empty
        if element.path.is_empty() {
            return Err(ExportError::MissingRequiredField(
                "element.path".to_string(),
            ));
        }

        // Validate cardinality if present
        if let (Some(min), Some(max)) = (&element.min, &element.max) {
            // Check that max is valid
            if max != "*"
                && let Ok(max_val) = max.parse::<u32>()
                && *min > max_val
            {
                return Err(ExportError::InvalidCardinality(format!("{}..{}", min, max)));
            }
        }

        // Validate binding strength if binding present
        if let Some(binding) = &element.binding
            && binding.value_set.is_none()
        {
            return Err(ExportError::InvalidBindingStrength(
                "Binding must have a value_set".to_string(),
            ));
        }

        Ok(())
    }

    /// Get base StructureDefinition from parent type
    async fn get_base_structure_definition(
        &self,
        parent: &str,
    ) -> Result<StructureDefinition, ExportError> {
        eprintln!(
            "[GET_BASE_SD] Step 1: Starting resolution for parent: {}",
            parent
        );
        debug!("Resolving parent: {}", parent);

        // 1. If canonical provided explicitly, resolve immediately
        if parent.starts_with("http://") || parent.starts_with("https://") {
            eprintln!("[GET_BASE_SD] Step 2: Parent is a canonical URL, resolving...");
            if let Some(sd) = self
                .resolve_structure_definition_from_canonical(parent)
                .await?
            {
                eprintln!(
                    "[GET_BASE_SD] Step 3: Resolved parent via canonical URL {} → {}",
                    parent, sd.url
                );
                debug!("Resolved parent via canonical URL {} → {}", parent, sd.url);
                return Ok(sd);
            }

            return Err(ExportError::ParentNotFound(format!(
                "{}: canonical URL {} not found",
                parent, parent
            )));
        }

        // 2. Try resolving as FHIR resource by name via canonical manager
        // This handles cases like "USCorePatient" which may be defined as an alias
        // or exists in a loaded package with that name
        eprintln!(
            "[GET_BASE_SD] Step 4: Trying to resolve parent '{}' via canonical manager by name",
            parent
        );
        debug!(
            "Trying to resolve parent '{}' via canonical manager by name",
            parent
        );

        eprintln!("[GET_BASE_SD] Step 5: About to call session.resolve()");
        if let Ok(resource) = self.session.resolve(parent).await {
            eprintln!(
                "[GET_BASE_SD] Step 6: Found parent '{}' in canonical packages by name",
                parent
            );
            debug!("Found parent '{}' in canonical packages by name", parent);
            let sd_json = (*resource.content).clone();
            match serde_json::from_value::<StructureDefinition>(sd_json) {
                Ok(sd) => {
                    debug!(
                        "Successfully parsed StructureDefinition for parent: {}",
                        parent
                    );
                    return Ok(sd);
                }
                Err(e) => {
                    debug!(
                        "Failed to parse StructureDefinition for parent {}: {}",
                        parent, e
                    );
                }
            }
        }

        // 3. Try resolving as a FHIR core profile (fast path)
        let core_candidate = format!("http://hl7.org/fhir/StructureDefinition/{}", parent);
        if let Some(sd) = self
            .resolve_structure_definition_from_canonical(&core_candidate)
            .await?
        {
            debug!(
                "Resolved parent {} using FHIR core canonical {}",
                parent, core_candidate
            );
            return Ok(sd);
        }

        // 4. Fallback to searching by ID/name via canonical manager index
        debug!("Searching for StructureDefinition with name/id: {}", parent);

        // Try multiple search strategies:
        // 1. Search by exact ID match (e.g., "us-core-patient")
        // 2. Search by name match (e.g., "USCorePatient")
        // 3. Convert camelCase to kebab-case and search (e.g., "USCorePatient" -> "us-core-patient")

        let canonical_from_search = if let Ok(Some(resource)) = self
            .session
            .resource_by_type_and_id("StructureDefinition", parent)
            .await
        {
            debug!("Found parent by exact ID: {}", parent);
            resource
                .content
                .as_object()
                .and_then(|obj| obj.get("url"))
                .and_then(|url| url.as_str())
                .ok_or_else(|| {
                    ExportError::CanonicalError(format!(
                        "Found resource but missing url field: {}",
                        parent
                    ))
                })?
                .to_string()
        } else {
            let kebab_case = camel_to_kebab(parent);
            debug!("Trying kebab-case variant: {}", kebab_case);

            if let Ok(Some(resource)) = self
                .session
                .resource_by_type_and_id("StructureDefinition", &kebab_case)
                .await
            {
                debug!("Found parent by kebab-case ID: {}", kebab_case);
                resource
                    .content
                    .as_object()
                    .and_then(|obj| obj.get("url"))
                    .and_then(|url| url.as_str())
                    .ok_or_else(|| {
                        ExportError::CanonicalError(format!(
                            "Found resource but missing url field: {}",
                            kebab_case
                        ))
                    })?
                    .to_string()
            } else {
                debug!(
                    "No match found via search for {}, assuming FHIR core canonical",
                    parent
                );
                core_candidate
            }
        };

        if let Some(sd) = self
            .resolve_structure_definition_from_canonical(&canonical_from_search)
            .await?
        {
            debug!(
                "Resolved parent {} via canonical {}",
                parent, canonical_from_search
            );
            Ok(sd)
        } else {
            Err(ExportError::ParentNotFound(format!(
                "{}: canonical resolution failed for {}",
                parent, canonical_from_search
            )))
        }
    }

    async fn resolve_structure_definition_from_canonical(
        &self,
        canonical_url: &str,
    ) -> Result<Option<StructureDefinition>, ExportError> {
        match self.session.resolve(canonical_url).await {
            Ok(resource) => {
                let structure_def: StructureDefinition =
                    serde_json::from_value((*resource.content).clone()).map_err(|e| {
                        ExportError::CanonicalError(format!(
                            "Failed to parse StructureDefinition {}: {}",
                            canonical_url, e
                        ))
                    })?;
                Ok(Some(structure_def))
            }
            Err(CanonicalLoaderError::Resolution { .. }) => Ok(None),
            Err(e) => Err(ExportError::CanonicalError(format!(
                "Failed resolving {}: {}",
                canonical_url, e
            ))),
        }
    }

    /// Apply profile metadata (SUSHI compatible)
    ///
    /// This function implements SUSHI's metadata clearing strategy:
    /// 1. Clear all inherited metadata fields
    /// 2. Remove uninherited extensions
    /// 3. Set new profile metadata from FSH
    fn apply_metadata(
        &self,
        structure_def: &mut StructureDefinition,
        profile: &Profile,
    ) -> Result<(), ExportError> {
        let profile_name = profile
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("name".to_string()))?;

        // STEP 1: Clear inherited metadata fields (SUSHI behavior)
        // Note: Our StructureDefinition struct has limited fields.
        // We clear what we have. Full FHIR StructureDefinition would have:
        // meta, implicit_rules, language, text, contained, etc.

        // STEP 2: Clear inherited fields that exist in our struct
        structure_def.experimental = None;
        structure_def.date = None;
        structure_def.publisher = None;

        // NOTE: Mappings should NOT be cleared - they are inherited from parent
        // (SUSHI behavior: mappings stay in snapshot, only NEW mappings in differential)

        // Note: Fields like contact, use_context, jurisdiction, purpose, copyright, keyword
        // don't exist in our simplified struct yet. They would be cleared in a full implementation.

        // STEP 3: Filter out uninherited extensions (SUSHI parity)
        // These HL7-specific extensions should not be inherited from parent
        if let Some(extensions) = &structure_def.extension {
            let filtered: Vec<serde_json::Value> = extensions
                .iter()
                .filter(|ext| {
                    // Extract URL from extension object
                    if let Some(url) = ext.get("url").and_then(|u| u.as_str()) {
                        !UNINHERITED_EXTENSIONS.contains(&url)
                    } else {
                        true // Keep extensions without URL
                    }
                })
                .cloned()
                .collect();

            structure_def.extension = if filtered.is_empty() {
                None
            } else {
                Some(filtered)
            };
        }

        // STEP 4: Set new metadata from profile
        structure_def.name = profile_name.clone();

        // Use id for URL if present, otherwise name
        let url_id = profile
            .id()
            .and_then(|id_clause| id_clause.value())
            .unwrap_or_else(|| profile_name.clone());
        structure_def.url = format!("{}/StructureDefinition/{}", self.base_url, url_id);

        // Set derivation
        structure_def.derivation = Some("constraint".to_string());

        // base_definition should point to parent, not self
        // (this will be set correctly when getting base definition)

        // Set id
        if let Some(id_clause) = profile.id()
            && let Some(id) = id_clause.value()
        {
            structure_def.id = Some(id);
        }

        // Set title
        if let Some(title_clause) = profile.title() {
            if let Some(title) = title_clause.value() {
                structure_def.title = Some(title);
            }
        } else {
            structure_def.title = None;
        }

        // Set description
        if let Some(desc_clause) = profile.description() {
            if let Some(desc) = desc_clause.value() {
                structure_def.description = Some(desc);
            }
        } else {
            structure_def.description = None;
        }

        // Set status from config (defaults to "draft" if not specified)
        structure_def.status = self.status.clone().unwrap_or_else(|| "draft".to_string());

        // Set publisher from config if available
        structure_def.publisher = self.publisher.clone();

        // Set version from config if available (SUSHI parity)
        structure_def.version = self.version.clone();

        Ok(())
    }

    /// Apply a single rule to the StructureDefinition
    async fn apply_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &Rule,
    ) -> Result<(), ExportError> {
        match rule {
            Rule::Card(card_rule) => self.apply_cardinality_rule(structure_def, card_rule).await,
            Rule::Flag(flag_rule) => self.apply_flag_rule(structure_def, flag_rule).await,
            Rule::ValueSet(valueset_rule) => {
                self.apply_binding_rule(structure_def, valueset_rule).await
            }
            Rule::FixedValue(fixed_rule) => {
                self.apply_fixed_value_rule(structure_def, fixed_rule).await
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
            Rule::Path(_) => {
                // PathRule is for type constraints - not implemented yet
                Ok(())
            }
            Rule::Contains(contains_rule) => {
                self.apply_contains_rule(structure_def, contains_rule).await
            }
            Rule::Only(only_rule) => self.apply_only_rule(structure_def, only_rule).await,
            Rule::Obeys(obeys_rule) => self.apply_obeys_rule(structure_def, obeys_rule).await,
            Rule::CaretValue(caret_rule) => {
                self.apply_caret_value_rule(structure_def, caret_rule).await
            }
            Rule::CodeCaretValue(_) => {
                // Code caret rules are handled when exporting CodeSystems/ValueSets
                Ok(())
            }
            Rule::CodeInsert(_) => {
                // Code insert rules are not applicable for StructureDefinition export
                Ok(())
            }
        }
    }

    /// Apply cardinality rule (min..max)
    async fn apply_cardinality_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &CardRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let cardinality = rule
            .cardinality()
            .ok_or_else(|| ExportError::InvalidCardinality("missing".to_string()))?;

        trace!("Applying cardinality rule: {} {}", path_str, cardinality);

        // Parse cardinality (e.g., "1..1", "0..*")
        let parts: Vec<&str> = cardinality.split("..").collect();
        if parts.len() != 2 {
            return Err(ExportError::InvalidCardinality(cardinality));
        }

        let min = parts[0]
            .parse::<u32>()
            .map_err(|_| ExportError::InvalidCardinality(cardinality.clone()))?;
        let max = parts[1].to_string();

        // Resolve path to element
        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let profile_name = structure_def.name.clone();

        // Find and update element
        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            warn!("Element not found in snapshot: {}", full_path);
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: profile_name,
            }
        })?;

        debug!("Applying cardinality {}..{} to {}", min, max, full_path);
        element.min = Some(min);
        element.max = Some(max);

        // Also apply flags if present
        for flag in rule.flags() {
            self.apply_flag_to_element(element, &flag)?;
        }

        Ok(())
    }

    /// Apply flag rule (MS, SU, etc.)
    async fn apply_flag_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &FlagRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        trace!("Applying flag rule: {}", path_str);

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let profile_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: profile_name,
            }
        })?;

        for flag in rule.flags() {
            self.apply_flag_to_element(element, &flag)?;
        }

        Ok(())
    }

    /// Apply a flag to an element
    fn apply_flag_to_element(
        &self,
        element: &mut ElementDefinition,
        flag: &str,
    ) -> Result<(), ExportError> {
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

    /// Apply binding rule (ValueSet binding)
    async fn apply_binding_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &ValueSetRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let value_set = rule
            .value_set()
            .ok_or_else(|| ExportError::MissingRequiredField("value set".to_string()))?;

        let strength_str = rule.strength().unwrap_or_else(|| "required".to_string());

        trace!(
            "Applying binding rule: {} from {} ({})",
            path_str, value_set, strength_str
        );

        let strength = BindingStrength::from_str(&strength_str)
            .ok_or_else(|| ExportError::InvalidBindingStrength(strength_str.clone()))?;

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let profile_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: profile_name,
            }
        })?;

        // Create canonical URL for ValueSet
        let value_set_url = if value_set.starts_with("http://") || value_set.starts_with("https://")
        {
            value_set
        } else {
            format!("{}/ValueSet/{}", self.base_url, value_set)
        };

        element.binding = Some(ElementDefinitionBinding {
            strength,
            description: None,
            value_set: Some(value_set_url),
        });

        Ok(())
    }

    /// Apply fixed value rule
    async fn apply_fixed_value_rule(
        &self,
        structure_def: &mut StructureDefinition,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        let path_str = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::MissingRequiredField("path".to_string()))?;

        let value = rule
            .value()
            .ok_or_else(|| ExportError::MissingRequiredField("value".to_string()))?;

        trace!("Applying fixed value rule: {} = {}", path_str, value);

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let profile_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: profile_name,
            }
        })?;

        // Parse value and determine type
        // For now, we'll store as pattern (less strict than fixed)
        let mut pattern_map = std::collections::HashMap::new();

        // Determine the type from the element or infer from value
        if value.starts_with('"') {
            // String value
            let parsed_value: JsonValue = serde_json::from_str(&value)?;
            pattern_map.insert("patternString".to_string(), parsed_value);
        } else if value.starts_with('#') {
            // Code value
            let code = value.trim_start_matches('#');
            pattern_map.insert(
                "patternCode".to_string(),
                JsonValue::String(code.to_string()),
            );
        } else if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            // Numeric value
            let parsed_value: JsonValue = serde_json::from_str(&value)?;
            pattern_map.insert("patternInteger".to_string(), parsed_value);
        } else {
            // Treat as identifier/code
            pattern_map.insert("patternCode".to_string(), JsonValue::String(value));
        };

        element.pattern = Some(pattern_map);

        Ok(())
    }

    /// Resolve a path string to full element path
    ///
    /// Handles:
    /// - Simple paths: "name" → "Patient.name"
    /// - Nested paths: "name.given" → "Patient.name.given"
    /// - Slicing syntax: "identifier[system]" → "Patient.identifier:system"
    /// - Extension paths: "extension[myExtension]" → "Patient.extension:myExtension"
    /// - Choice types: "value[x]" stays as "Patient.value[x]" (type constraint handled elsewhere)
    async fn resolve_full_path(
        &self,
        structure_def: &StructureDefinition,
        path: &str,
    ) -> Result<String, ExportError> {
        // If path already includes resource type, use as-is
        if path.contains('.') {
            let parts: Vec<&str> = path.split('.').collect();
            if parts[0] == structure_def.type_field {
                return Ok(path.to_string());
            }
        }

        // Handle bracket notation for slicing: "identifier[system]" → "identifier:system"
        let normalized_path = if path.contains('[') && path.contains(']') {
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
        } else {
            path.to_string()
        };

        // Prepend resource type
        Ok(format!("{}.{}", structure_def.type_field, normalized_path))
    }

    /// Apply contains rule (for slicing)
    /// Example: * extension contains myExtension 0..1
    ///
    /// This creates slices for the specified elements. For extensions,
    /// it automatically uses `url` as the discriminator.
    async fn apply_contains_rule(
        &self,
        structure_def: &mut StructureDefinition,
        contains_rule: &ContainsRule,
    ) -> Result<(), ExportError> {
        let path = contains_rule.path().ok_or_else(|| {
            ExportError::MissingRequiredField("path for contains rule".to_string())
        })?;
        let path_str = path.as_string();
        let items = contains_rule.items();

        debug!("Applying contains rule: {} contains {:?}", path_str, items);

        // Resolve full path
        let full_path = self.resolve_full_path(structure_def, &path_str).await?;

        // Find or create the element to be sliced
        let snapshot = structure_def.get_or_create_snapshot();

        // Check if this is an extension path (special handling)
        let is_extension = path_str == "extension"
            || path_str == "modifierExtension"
            || path_str.ends_with(".extension")
            || path_str.ends_with(".modifierExtension");

        // Find the base element
        let base_element_index = snapshot.element.iter().position(|e| e.path == full_path);

        if base_element_index.is_none() {
            warn!("Base element not found for slicing: {}", full_path);
            // Create base element if it doesn't exist
            snapshot
                .element
                .push(ElementDefinition::new(full_path.clone()));
        }

        // TODO: Add slicing discriminator for extensions
        // This requires adding slicing field to ElementDefinition struct
        // For now, we just create the slice elements
        if is_extension {
            debug!("Extension slicing on {}", full_path);
            // base_elem.slicing would be set here in full implementation
        }

        // Create slice elements for each item
        for item in items {
            let slice_path = format!("{}:{}", full_path, item);

            // Check if slice already exists
            if snapshot.element.iter().any(|e| e.path == slice_path) {
                debug!("Slice already exists: {}", slice_path);
                continue;
            }

            // Create the slice element
            let mut slice_element = ElementDefinition::new(slice_path.clone());
            slice_element.short = Some(format!("Slice: {}", item));

            // For extension slices, try to resolve the extension and set its profile
            if is_extension {
                // Try to resolve extension URL
                let extension_url = format!("{}/StructureDefinition/{}", self.base_url, item);

                // Set type with profile
                slice_element.type_ = Some(vec![ElementDefinitionType {
                    code: "Extension".to_string(),
                    profile: Some(vec![extension_url.clone()]),
                    target_profile: None,
                }]);

                debug!(
                    "Created extension slice {} with profile {}",
                    slice_path, extension_url
                );
            }

            snapshot.element.push(slice_element);
        }

        Ok(())
    }

    /// Apply only rule (type constraint)
    /// Example: * value[x] only Quantity
    ///
    /// Constrains the allowed types for an element.
    async fn apply_only_rule(
        &self,
        structure_def: &mut StructureDefinition,
        only_rule: &OnlyRule,
    ) -> Result<(), ExportError> {
        let path = only_rule
            .path()
            .ok_or_else(|| ExportError::MissingRequiredField("path for only rule".to_string()))?;
        let path_str = path.as_string();
        let types = only_rule.types();

        debug!("Applying only rule: {} only {:?}", path_str, types);

        // Resolve full path
        let full_path = self.resolve_full_path(structure_def, &path_str).await?;

        // Find the element in snapshot
        let snapshot = structure_def.get_or_create_snapshot();

        if let Some(element) = snapshot.element.iter_mut().find(|e| e.path == full_path) {
            // Set the type constraint
            element.type_ = Some(
                types
                    .iter()
                    .map(|t| ElementDefinitionType {
                        code: t.clone(),
                        profile: None,
                        target_profile: None,
                    })
                    .collect(),
            );

            debug!("Constrained {} to types: {:?}", full_path, types);
        } else {
            warn!("Element not found for only rule: {}", full_path);
        }

        Ok(())
    }

    /// Apply obeys rule (invariant constraint)
    /// Example: * obeys inv-1
    ///
    /// Adds invariant constraints to an element.
    async fn apply_obeys_rule(
        &self,
        structure_def: &mut StructureDefinition,
        obeys_rule: &ObeysRule,
    ) -> Result<(), ExportError> {
        let path = obeys_rule
            .path()
            .ok_or_else(|| ExportError::MissingRequiredField("path for obeys rule".to_string()))?;
        let path_str = path.as_string();
        let invariants = obeys_rule.invariants();

        debug!("Applying obeys rule: {} obeys {:?}", path_str, invariants);

        // Resolve full path
        let full_path = self.resolve_full_path(structure_def, &path_str).await?;

        // Find the element in snapshot
        let snapshot = structure_def.get_or_create_snapshot();

        if let Some(element) = snapshot.element.iter_mut().find(|e| e.path == full_path) {
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
        } else {
            warn!("Element not found for obeys rule: {}", full_path);
        }

        Ok(())
    }

    /// Apply caret value rule (metadata assignment)
    ///
    /// Example: * identifier ^short = "Patient identifier"
    /// Example: * ^version = "1.0.0"
    ///
    /// Applies metadata fields to either elements or the profile itself.
    async fn apply_caret_value_rule(
        &self,
        structure_def: &mut StructureDefinition,
        caret_rule: &CaretValueRule,
    ) -> Result<(), ExportError> {
        let field = caret_rule
            .field()
            .ok_or_else(|| ExportError::MissingRequiredField("caret field".to_string()))?;
        let value = caret_rule
            .value()
            .ok_or_else(|| ExportError::MissingRequiredField("caret value".to_string()))?;

        // Check if this is an element-level caret rule or profile-level
        if let Some(element_path) = caret_rule.element_path() {
            // Element-level: * identifier ^short = "Patient identifier"
            let path_str = element_path.as_string();
            debug!(
                "Applying element-level caret rule: {} ^{} = {}",
                path_str, field, value
            );

            // Resolve full path
            let full_path = self.resolve_full_path(structure_def, &path_str).await?;

            // Find the element in snapshot
            let snapshot = structure_def.get_or_create_snapshot();

            if let Some(element) = snapshot.element.iter_mut().find(|e| e.path == full_path) {
                // Apply the field to the element
                self.apply_field_to_element(element, &field, &value)?;
            } else {
                warn!("Element not found for caret rule: {}", full_path);
            }
        } else {
            // Profile-level: * ^version = "1.0.0"
            debug!("Applying profile-level caret rule: ^{} = {}", field, value);

            // Apply the field to the StructureDefinition itself
            self.apply_field_to_structure(structure_def, &field, &value)?;
        }

        Ok(())
    }

    /// Apply a field to an ElementDefinition
    fn apply_field_to_element(
        &self,
        element: &mut ElementDefinition,
        field: &str,
        value: &str,
    ) -> Result<(), ExportError> {
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

    /// Apply a field to the StructureDefinition
    fn apply_field_to_structure(
        &self,
        structure_def: &mut StructureDefinition,
        field: &str,
        value: &str,
    ) -> Result<(), ExportError> {
        match field {
            "version" => {
                structure_def.version = Some(value.trim_matches('"').to_string());
            }
            "status" => {
                // Remove # prefix if present
                let status_value = value.trim_start_matches('#').trim_matches('"');
                structure_def.status = status_value.to_string();
            }
            "experimental" => {
                structure_def.experimental = Some(value == "true");
            }
            "publisher" => {
                structure_def.publisher = Some(value.trim_matches('"').to_string());
            }
            "description" => {
                structure_def.description = Some(value.trim_matches('"').to_string());
            }
            _ => {
                debug!("Unhandled structure field in caret rule: {}", field);
                // Note: purpose, copyright, and other fields may not be
                // available in the current StructureDefinition struct
            }
        }
        Ok(())
    }

    /// Generate differential directly from FSH rules (SUSHI-style approach)
    /// Creates one differential element per FSH rule
    fn generate_differential_from_rules(
        &self,
        profile: &Profile,
        resource_type: &str,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        // Process each rule and create differential elements
        for rule in profile.rules() {
            match rule {
                Rule::Card(card_rule) => {
                    // Create element for cardinality constraint
                    if let Some(path_str) = card_rule.path().map(|p| p.as_string()) {
                        let full_path = if path_str.contains('.') {
                            path_str
                        } else {
                            format!("{}.{}", resource_type, path_str)
                        };

                        let mut element = ElementDefinition::new(full_path.clone());

                        // Parse cardinality
                        if let Some(card_str) = card_rule.cardinality() {
                            let parts: Vec<&str> = card_str.split("..").collect();
                            if parts.len() == 2 {
                                if let Ok(min) = parts[0].parse::<u32>() {
                                    element.min = Some(min);
                                }
                                element.max = Some(parts[1].to_string());
                            }
                        }

                        // Check for MS flag
                        if card_rule.flags().contains(&"MS".to_string()) {
                            element.must_support = Some(true);
                        }

                        differential_elements.push(element);
                    }
                }
                Rule::Flag(flag_rule) => {
                    // Create element for flag rule (like MS)
                    if let Some(path_str) = flag_rule.path().map(|p| p.as_string()) {
                        let full_path = if path_str.contains('.') {
                            path_str
                        } else {
                            format!("{}.{}", resource_type, path_str)
                        };

                        // Check if we already have this element from a cardinality rule
                        if let Some(existing) = differential_elements
                            .iter_mut()
                            .find(|e| e.path == full_path)
                        {
                            // Update existing element
                            if flag_rule.flags().contains(&"MS".to_string()) {
                                existing.must_support = Some(true);
                            }
                        } else {
                            // Create new element
                            let mut element = ElementDefinition::new(full_path.clone());

                            if flag_rule.flags().contains(&"MS".to_string()) {
                                element.must_support = Some(true);
                            }

                            differential_elements.push(element);
                        }
                    }
                }
                Rule::FixedValue(fixed_rule) => {
                    // Handle ^short metadata
                    if let Some(path_str) = fixed_rule.path().map(|p| p.as_string())
                        && path_str.starts_with('^')
                    {
                        // For now, skip metadata rules - they're handled differently
                        continue;
                    }
                }
                _ => {
                    // Other rule types handled separately
                }
            }
        }

        debug!(
            "Generated {} differential elements from FSH rules",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }

    /// Generate differential by comparing current snapshot with original (SUSHI approach)
    ///
    /// This compares the modified snapshot (after rules applied) with the original snapshot
    /// (before rules applied) to determine what changed. Only changed elements appear in differential.
    fn generate_differential_from_snapshot(
        &self,
        original_snapshot: &Option<StructureDefinitionSnapshot>,
        current_snapshot: &Option<StructureDefinitionSnapshot>,
        _original_mappings: &Option<Vec<StructureDefinitionMapping>>,
        _current_mappings: &Option<Vec<StructureDefinitionMapping>>,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        // Get elements from both snapshots
        let empty_vec = vec![];
        let original_elements = original_snapshot
            .as_ref()
            .map(|s| &s.element)
            .unwrap_or(&empty_vec);

        let current_elements = current_snapshot
            .as_ref()
            .map(|s| &s.element)
            .unwrap_or(&empty_vec);

        // Compare each element in current snapshot with original
        for current_elem in current_elements {
            // Find matching element in original snapshot
            let original_elem = original_elements
                .iter()
                .find(|e| e.id == current_elem.id || e.path == current_elem.path);

            if let Some(orig) = original_elem {
                // Element exists in both - check if it changed
                if self.element_has_diff(orig, current_elem) {
                    // Create differential element with only changed fields
                    let diff_elem = self.create_diff_element(orig, current_elem);
                    differential_elements.push(diff_elem);
                }
            } else {
                // Element is new (e.g., a slice) - include entire element
                differential_elements.push(current_elem.clone());
            }
        }

        debug!(
            "Generated {} differential elements from snapshot comparison",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }

    /// Check if an element has differences between original and current
    fn element_has_diff(&self, original: &ElementDefinition, current: &ElementDefinition) -> bool {
        // Check all fields that can be constrained
        original.min != current.min
            || original.max != current.max
            || original.must_support != current.must_support
            || original.short != current.short
            || original.definition != current.definition
            || original.comment != current.comment
            || original.type_ != current.type_
            || original.binding != current.binding
            || original.constraint != current.constraint
            || original.mapping != current.mapping
            || original.fixed != current.fixed
            || original.pattern != current.pattern
    }

    /// Create a differential element containing only changed fields
    fn create_diff_element(
        &self,
        original: &ElementDefinition,
        current: &ElementDefinition,
    ) -> ElementDefinition {
        let mut diff_elem = ElementDefinition::new(current.path.clone());

        // Always include id and path
        diff_elem.id = current.id.clone();

        // Include only fields that changed
        if original.min != current.min {
            diff_elem.min = current.min;
        }
        if original.max != current.max {
            diff_elem.max = current.max.clone();
        }
        if original.must_support != current.must_support {
            diff_elem.must_support = current.must_support;
        }
        if original.short != current.short {
            diff_elem.short = current.short.clone();
        }
        if original.definition != current.definition {
            diff_elem.definition = current.definition.clone();
        }
        if original.comment != current.comment {
            diff_elem.comment = current.comment.clone();
        }
        if original.type_ != current.type_ {
            diff_elem.type_ = current.type_.clone();
        }
        if original.binding != current.binding {
            diff_elem.binding = current.binding.clone();
        }
        if original.constraint != current.constraint {
            diff_elem.constraint = current.constraint.clone();
        }
        if original.mapping != current.mapping {
            diff_elem.mapping = current.mapping.clone();
        }
        if original.fixed != current.fixed {
            diff_elem.fixed = current.fixed.clone();
        }
        if original.pattern != current.pattern {
            diff_elem.pattern = current.pattern.clone();
        }

        diff_elem
    }

    /// Generate differential by comparing snapshot with base (static version for testing)
    #[cfg(test)]
    fn generate_differential_static(
        base: &StructureDefinitionSnapshot,
        modified: &StructureDefinition,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        if let Some(modified_snapshot) = &modified.snapshot {
            for modified_elem in &modified_snapshot.element {
                // Find corresponding base element
                if let Some(base_elem) = base.element.iter().find(|e| e.path == modified_elem.path)
                {
                    // Check if element was modified
                    if modified_elem.is_modified_from(base_elem) {
                        differential_elements.push(modified_elem.clone());
                    }
                } else {
                    // New element (not in base) - always include
                    differential_elements.push(modified_elem.clone());
                }
            }
        }

        trace!(
            "Generated differential with {} elements",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }

    /// Generate differential by comparing snapshot with base
    fn generate_differential(
        &self,
        base: &StructureDefinitionSnapshot,
        modified: &StructureDefinition,
    ) -> StructureDefinitionDifferential {
        let mut differential_elements = Vec::new();

        if let Some(modified_snapshot) = &modified.snapshot {
            for modified_elem in &modified_snapshot.element {
                // Find corresponding base element
                if let Some(base_elem) = base.element.iter().find(|e| e.path == modified_elem.path)
                {
                    // Check if element was modified
                    if modified_elem.is_modified_from(base_elem) {
                        differential_elements.push(modified_elem.clone());
                    }
                } else {
                    // New element (not in base) - always include
                    differential_elements.push(modified_elem.clone());
                }
            }
        }

        trace!(
            "Generated differential with {} elements",
            differential_elements.len()
        );

        StructureDefinitionDifferential {
            element: differential_elements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: These tests are disabled because they require a real session
    // They should be moved to integration tests

    // #[test]
    // fn test_resolve_full_path() { ... }

    // #[test]
    // fn test_apply_flag_to_element() { ... }

    #[test]
    fn test_generate_differential_no_changes() {
        // Note: We don't use create_test_exporter() for this test since
        // generate_differential doesn't access session/path_resolver

        let base_elem = ElementDefinition::new("Patient.id".to_string());
        let base = StructureDefinitionSnapshot {
            element: vec![base_elem.clone()],
        };

        let mut modified = StructureDefinition::new(
            "http://test.org/Test".to_string(),
            "Test".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        );
        modified.snapshot = Some(StructureDefinitionSnapshot {
            element: vec![base_elem],
        });

        // Create a minimal exporter just for testing differential generation
        // This is safe because generate_differential doesn't use session/path_resolver
        let base_url = "http://test.org".to_string();
        let diff = ProfileExporter::generate_differential_static(&base, &modified);
        assert_eq!(diff.element.len(), 0);
    }

    #[test]
    fn test_generate_differential_with_changes() {
        let base_elem = ElementDefinition::new("Patient.name".to_string());
        let base = StructureDefinitionSnapshot {
            element: vec![base_elem.clone()],
        };

        let mut modified_elem = base_elem;
        modified_elem.min = Some(1);
        modified_elem.must_support = Some(true);

        let mut modified = StructureDefinition::new(
            "http://test.org/Test".to_string(),
            "Test".to_string(),
            "Patient".to_string(),
            StructureDefinitionKind::Resource,
        );
        modified.snapshot = Some(StructureDefinitionSnapshot {
            element: vec![modified_elem.clone()],
        });

        let diff = ProfileExporter::generate_differential_static(&base, &modified);
        assert_eq!(diff.element.len(), 1);
        assert_eq!(diff.element[0].path, "Patient.name");
        assert_eq!(diff.element[0].min, Some(1));
        assert_eq!(diff.element[0].must_support, Some(true));
    }
}
