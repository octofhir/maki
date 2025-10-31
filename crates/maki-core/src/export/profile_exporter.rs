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

use super::fhir_types::*;
use crate::canonical::DefinitionSession;
use crate::cst::ast::{
    CardRule, ContainsRule, FixedValueRule, FlagRule, ObeysRule, OnlyRule, Profile, Rule,
    ValueSetRule,
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
}

/// Profile exporter
///
/// Transforms FSH Profile AST nodes into FHIR StructureDefinition resources.
pub struct ProfileExporter {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
    /// Path resolver for finding elements
    path_resolver: Arc<PathResolver>,
    /// Base URL for generated profiles
    base_url: String,
    /// Whether to generate snapshots (default: false to match SUSHI)
    generate_snapshots: bool,
}

impl ProfileExporter {
    /// Create a new profile exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving base definitions
    /// * `base_url` - Base URL for generated profile canonical URLs
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
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
    ) -> Result<Self, ExportError> {
        let path_resolver = Arc::new(PathResolver::new(session.clone()));

        Ok(Self {
            session,
            path_resolver,
            base_url,
            generate_snapshots: false, // Default OFF to match SUSHI
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

        debug!("Exporting profile: {}", profile_name);

        // 1. Get parent type
        let parent = profile
            .parent()
            .and_then(|p| p.value())
            .ok_or_else(|| ExportError::MissingRequiredField("parent".to_string()))?;

        debug!("Parent type: {}", parent);

        // 2. Get base StructureDefinition
        let mut structure_def = self.get_base_structure_definition(&parent).await?;

        // 3. Apply metadata
        self.apply_metadata(&mut structure_def, profile)?;

        // 4. Make a copy of the base snapshot for comparison
        let base_snapshot = structure_def.snapshot.clone();

        if let Some(ref snap) = base_snapshot {
            debug!("Base snapshot has {} elements", snap.element.len());
        } else {
            debug!("Base snapshot is None");
        }

        // 5. Apply all rules to snapshot
        for rule in profile.rules() {
            if let Err(e) = self.apply_rule(&mut structure_def, &rule).await {
                warn!("Failed to apply rule: {}", e);
                // Continue with other rules instead of failing completely
            }
        }

        if let Some(ref snap) = structure_def.snapshot {
            debug!("Modified snapshot has {} elements", snap.element.len());
        }

        // 6. Generate differential directly from FSH rules (SUSHI-style)
        structure_def.differential =
            Some(self.generate_differential_from_rules(profile, &structure_def.type_field));

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
        debug!("Resolving parent: {}", parent);

        // Try to resolve as a canonical URL first, then by type name
        let canonical_url = if parent.starts_with("http://") || parent.starts_with("https://") {
            parent.to_string()
        } else {
            // Assume it's a FHIR core resource type
            format!("http://hl7.org/fhir/StructureDefinition/{}", parent)
        };

        let resource = self
            .session
            .resolve(&canonical_url)
            .await
            .map_err(|e| ExportError::ParentNotFound(format!("{}: {}", parent, e)))?;

        // Parse JSON into StructureDefinition
        let structure_def: StructureDefinition =
            serde_json::from_value((*resource.content).clone()).map_err(|e| {
                ExportError::CanonicalError(format!(
                    "Failed to parse StructureDefinition for {}: {}",
                    parent, e
                ))
            })?;

        debug!("Resolved parent: {} ({})", parent, structure_def.url);
        Ok(structure_def)
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

        // Note: Fields like contact, use_context, jurisdiction, purpose, copyright, keyword
        // don't exist in our simplified struct yet. They would be cleared in a full implementation.

        // STEP 3: Extension filtering would happen here
        // Note: Our StructureDefinition doesn't have an extension field yet.
        // In full implementation, we would filter out UNINHERITED_EXTENSIONS here.

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

        // Set status from config
        structure_def.status = "draft".to_string(); // TODO: Get from config

        // Note: version is typically not set in differential, IG Publisher handles it
        structure_def.version = None;

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
            Rule::Path(_) => {
                // PathRule is for type constraints - not implemented yet
                Ok(())
            }
            Rule::Contains(contains_rule) => {
                self.apply_contains_rule(structure_def, contains_rule).await
            }
            Rule::Only(only_rule) => self.apply_only_rule(structure_def, only_rule).await,
            Rule::Obeys(obeys_rule) => self.apply_obeys_rule(structure_def, obeys_rule).await,
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
            snapshot.element.push(ElementDefinition {
                path: full_path.clone(),
                min: None,
                max: None,
                type_: None,
                short: None,
                definition: None,
                comment: None,
                must_support: None,
                is_modifier: None,
                is_summary: None,
                binding: None,
                constraint: None,
                pattern: None,
                fixed: None,
            });
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
            let mut slice_element = ElementDefinition {
                path: slice_path.clone(),
                min: None,
                max: None,
                type_: None,
                short: Some(format!("Slice: {}", item)),
                definition: None,
                comment: None,
                must_support: None,
                is_modifier: None,
                is_summary: None,
                binding: None,
                constraint: None,
                pattern: None,
                fixed: None,
            };

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

                        let mut element = ElementDefinition {
                            path: full_path.clone(),
                            min: None,
                            max: None,
                            type_: None,
                            short: None,
                            definition: None,
                            comment: None,
                            must_support: None,
                            is_summary: None,
                            is_modifier: None,
                            binding: None,
                            constraint: None,
                            fixed: None,
                            pattern: None,
                        };

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
                            let mut element = ElementDefinition {
                                path: full_path.clone(),
                                min: None,
                                max: None,
                                type_: None,
                                short: None,
                                definition: None,
                                comment: None,
                                must_support: None,
                                is_summary: None,
                                is_modifier: None,
                                binding: None,
                                constraint: None,
                                fixed: None,
                                pattern: None,
                            };

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
