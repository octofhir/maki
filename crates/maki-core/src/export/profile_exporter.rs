//! Profile Exporter
//!
//! Exports FSH Profile definitions to FHIR StructureDefinition resources.
//! This is the core of the FSH-to-FHIR transformation pipeline.
//!

#![allow(dead_code)]
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
//! let alias_table = maki_core::semantic::AliasTable::default();
//! let package = Arc::new(tokio::sync::RwLock::new(maki_core::semantic::Package::new()));
//! let exporter = ProfileExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//!     Some("1.0.0".to_string()),
//!     Some("draft".to_string()),
//!     Some("Example Org".to_string()),
//!     alias_table,
//!     package,
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
    CardRule, CaretValueRule, ContainsRule, FixedValueRule, FlagRule, FlagValue, ObeysRule,
    OnlyRule, Profile, Rule, ValueSetRule,
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

/// Check if a type name is a FHIR primitive or complex type (not a profile)
///
/// FHIR primitive types: string, boolean, integer, decimal, uri, url, canonical, etc.
/// FHIR complex types: CodeableConcept, Identifier, HumanName, Address, Quantity, etc.
/// Common FSH code system aliases mapped to canonical URLs
const CODE_SYSTEM_ALIASES: &[(&str, &str)] = &[
    ("LNC", "http://loinc.org"),
    ("LOINC", "http://loinc.org"),
    ("SCT", "http://snomed.info/sct"),
    ("SNOMED", "http://snomed.info/sct"),
    ("ICD10CM", "http://hl7.org/fhir/sid/icd-10-cm"),
    ("ICD10", "http://hl7.org/fhir/sid/icd-10"),
    ("UCUM", "http://unitsofmeasure.org"),
    ("RXNORM", "http://www.nlm.nih.gov/research/umls/rxnorm"),
    ("CPT", "http://www.ama-assn.org/go/cpt"),
    ("CVX", "http://hl7.org/fhir/sid/cvx"),
    ("HGNC", "http://www.genenames.org"),
    ("HGVS", "http://varnomen.hgvs.org"),
    ("ISO3166", "urn:iso:std:iso:3166"),
    ("NUCC", "http://nucc.org/provider-taxonomy"),
    ("NDC", "http://hl7.org/fhir/sid/ndc"),
];

/// Resolve a code system alias to its canonical URL
fn resolve_code_system_alias(alias: &str) -> Option<&'static str> {
    for (name, url) in CODE_SYSTEM_ALIASES {
        if *name == alias {
            return Some(url);
        }
    }
    None
}

/// Parsed FSH code value
#[derive(Debug)]
struct FshCodeValue {
    system: Option<String>,
    code: String,
    display: Option<String>,
}

/// Parse FSH code syntax into components
///
/// Handles formats:
/// - `#code` - code only (no system)
/// - `SYSTEM#code` - alias and code (e.g., `LNC#31206-6`)
/// - `http://system.url#code` - full URL and code
/// - `SYSTEM#code "display"` - with display text
fn parse_fsh_code_value(value: &str) -> Option<FshCodeValue> {
    let trimmed = value.trim();

    // Check if it has the code marker '#'
    if !trimmed.contains('#') {
        return None;
    }

    // Split display text if present (ends with "quoted string")
    let (code_part, display) = if let Some(quote_start) = trimmed.rfind('"') {
        // Find the matching opening quote
        let before_end_quote = &trimmed[..quote_start];
        if let Some(open_quote) = before_end_quote.rfind('"') {
            let display_text = trimmed[open_quote + 1..quote_start].to_string();
            let code_part = trimmed[..open_quote].trim();
            (code_part, Some(display_text))
        } else {
            (trimmed, None)
        }
    } else {
        (trimmed, None)
    };

    // Split on '#' to get system and code
    if let Some(hash_pos) = code_part.find('#') {
        let system_part = code_part[..hash_pos].trim();
        let code = code_part[hash_pos + 1..].trim().to_string();

        let system = if system_part.is_empty() {
            // Just `#code` - no system
            None
        } else if system_part.starts_with("http://") || system_part.starts_with("https://") {
            // Full URL system
            Some(system_part.to_string())
        } else {
            // Alias - resolve to URL
            if let Some(url) = resolve_code_system_alias(system_part) {
                Some(url.to_string())
            } else {
                // Unknown alias - treat as-is (might be defined elsewhere)
                Some(system_part.to_string())
            }
        };

        Some(FshCodeValue {
            system,
            code,
            display,
        })
    } else {
        None
    }
}

/// Determine the element type from an ElementDefinition
fn get_element_type(element: &ElementDefinition) -> Option<String> {
    element
        .type_
        .as_ref()
        .and_then(|types| types.first().map(|t| t.code.clone()))
}

fn is_fhir_primitive_or_complex_type(type_name: &str) -> bool {
    // Primitive types
    const PRIMITIVE_TYPES: &[&str] = &[
        "boolean",
        "integer",
        "integer64",
        "string",
        "decimal",
        "uri",
        "url",
        "canonical",
        "base64Binary",
        "instant",
        "date",
        "dateTime",
        "time",
        "code",
        "oid",
        "id",
        "markdown",
        "unsignedInt",
        "positiveInt",
        "uuid",
        "xhtml",
    ];

    // Common complex types (not exhaustive, but covers most common ones)
    const COMPLEX_TYPES: &[&str] = &[
        "Address",
        "Age",
        "Annotation",
        "Attachment",
        "CodeableConcept",
        "CodeableReference",
        "Coding",
        "ContactDetail",
        "ContactPoint",
        "Contributor",
        "Count",
        "DataRequirement",
        "Distance",
        "Dosage",
        "Duration",
        "Expression",
        "Extension",
        "HumanName",
        "Identifier",
        "Meta",
        "Money",
        "MoneyQuantity",
        "Narrative",
        "ParameterDefinition",
        "Period",
        "Quantity",
        "Range",
        "Ratio",
        "RatioRange",
        "Reference",
        "RelatedArtifact",
        "SampledData",
        "Signature",
        "SimpleQuantity",
        "Timing",
        "TriggerDefinition",
        "UsageContext",
        // Resource types commonly used in element types
        "Resource",
        "DomainResource",
        "BackboneElement",
        "Element",
    ];

    PRIMITIVE_TYPES.contains(&type_name) || COMPLEX_TYPES.contains(&type_name)
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
    #[allow(dead_code)]
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
    /// Alias table for resolving profile/extension aliases
    alias_table: crate::semantic::AliasTable,
    /// Package for finding locally exported profiles
    package: Arc<tokio::sync::RwLock<crate::semantic::Package>>,
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
    /// use maki_core::export::ProfileExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let alias_table = maki_core::semantic::AliasTable::default();
    /// let package = Arc::new(tokio::sync::RwLock::new(maki_core::semantic::Package::new()));
    /// let exporter = ProfileExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    ///     Some("1.0.0".to_string()),
    ///     Some("draft".to_string()),
    ///     Some("Example Org".to_string()),
    ///     alias_table,
    ///     package,
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
        alias_table: crate::semantic::AliasTable,
        package: Arc<tokio::sync::RwLock<crate::semantic::Package>>,
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
            alias_table,
            package,
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

        // FIX: Set baseDefinition to the parent's actual canonical URL (not a hardcoded format)
        // The resolved StructureDefinition has the correct URL - use it directly
        let base_definition_url = structure_def.url.clone();
        eprintln!(
            "[PROFILE EXPORT] Step 4a: Using parent's canonical URL as baseDefinition: {}",
            base_definition_url
        );
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
        // Helper to decide if a resolved StructureDefinition is an unexpected Extension parent
        let is_unexpected_extension = |sd: &StructureDefinition| -> bool {
            sd.type_field == "Extension" && parent != "Extension"
        };

        eprintln!(
            "[GET_BASE_SD] Step 1: Starting resolution for parent: {}",
            parent
        );
        debug!("Resolving parent: {}", parent);

        // 0a. Check if parent is in locally exported Package (by name)
        // This allows child profiles to find parent profiles exported earlier in the same build
        {
            let package = self.package.read().await;
            for (canonical_url, resource_json) in package.all_resources().iter() {
                if let Ok(sd) =
                    serde_json::from_value::<StructureDefinition>((**resource_json).clone())
                {
                    // Check if name matches parent
                    if sd.name == parent {
                        eprintln!(
                            "[GET_BASE_SD] Step 0: Found parent '{}' in local Package by name → {}",
                            parent, canonical_url
                        );
                        debug!("Found parent '{}' in local Package", parent);
                        return Ok(sd);
                    }
                }
            }
        }

        // 0b. Fast path: Use find_profile_parent for case-insensitive search with Extension exclusion
        // This is optimized to search by id/name/url with proper package priority ordering
        // Skip this for Extension parents (they should match Extensions)
        if parent != "Extension" {
            eprintln!(
                "[GET_BASE_SD] Step 0b: Trying find_profile_parent for '{}'",
                parent
            );
            if let Ok(Some(resource)) = self.session.find_profile_parent(parent).await {
                eprintln!(
                    "[GET_BASE_SD] Step 0b: Found parent '{}' via find_profile_parent → {}",
                    parent, resource.canonical_url
                );
                if let Ok(sd) =
                    serde_json::from_value::<StructureDefinition>((*resource.content).clone())
                {
                    debug!("Resolved parent '{}' via find_profile_parent", parent);
                    return Ok(sd);
                }
            }
        }

        // 0c. Check if parent is an alias and resolve it to canonical URL
        let parent_to_resolve = if let Some(resolved_url) = self.alias_table.resolve(parent) {
            eprintln!(
                "[GET_BASE_SD] Step 1a: Parent '{}' is an alias, resolved to canonical URL: {}",
                parent, resolved_url
            );
            debug!("Resolved alias '{}' → '{}'", parent, resolved_url);
            resolved_url
        } else {
            parent
        };

        // 1. If canonical provided explicitly (or resolved from alias), resolve immediately
        if parent_to_resolve.starts_with("http://") || parent_to_resolve.starts_with("https://") {
            eprintln!("[GET_BASE_SD] Step 2: Parent is a canonical URL, resolving...");
            if let Some(sd) = self
                .resolve_structure_definition_from_canonical(parent_to_resolve)
                .await?
            {
                // If we resolved to an Extension but the FSH parent wasn't Extension, keep searching
                if is_unexpected_extension(&sd) {
                    eprintln!(
                        "[GET_BASE_SD] Step 2a: Resolved to Extension for parent '{}', continuing search",
                        parent_to_resolve
                    );
                } else {
                    eprintln!(
                        "[GET_BASE_SD] Step 3: Resolved parent via canonical URL {} → {}",
                        parent_to_resolve, sd.url
                    );
                    debug!(
                        "Resolved parent via canonical URL {} → {}",
                        parent_to_resolve, sd.url
                    );
                    return Ok(sd);
                }
            }

            // If the canonical URL gave us an unexpected extension, keep searching
            if parent_to_resolve.starts_with("http://") || parent_to_resolve.starts_with("https://")
            {
                eprintln!(
                    "[GET_BASE_SD] Step 2b: Continuing search for parent '{}' after extension match",
                    parent_to_resolve
                );
            }

            return Err(ExportError::ParentNotFound(format!(
                "{}: canonical URL {} not found",
                parent, parent_to_resolve
            )));
        }

        // 1b. Try resolving by name via installed canonical packages (preferred over core fallback)
        eprintln!(
            "[GET_BASE_SD] Step 3: Trying to resolve parent '{}' by name via canonical packages",
            parent_to_resolve
        );
        if let Ok(Some(resource)) = self
            .session
            .resource_by_type_and_name("StructureDefinition", parent_to_resolve)
            .await
        {
            eprintln!(
                "[GET_BASE_SD] Step 3a: Found parent '{}' in canonical packages by name",
                parent_to_resolve
            );
            if let Ok(sd) =
                serde_json::from_value::<StructureDefinition>((*resource.content).clone())
            {
                if is_unexpected_extension(&sd) {
                    eprintln!(
                        "[GET_BASE_SD] Step 3a-ext: '{}' resolved to Extension ({}), continuing search",
                        parent_to_resolve, sd.url
                    );
                } else {
                    debug!(
                        "Resolved parent '{}' from canonical packages by name",
                        parent_to_resolve
                    );
                    return Ok(sd);
                }
            }
        }

        // 1c. Try well-known dependency canonical bases (e.g., genomics-reporting Variant)
        // Some IGs reference profiles by simple name (Variant) that live in dependency packages.
        // Try a few canonical patterns before falling back to generic core URL.
        let dependency_candidates = [
            format!(
                "http://hl7.org/fhir/uv/genomics-reporting/StructureDefinition/{}",
                parent_to_resolve
            ),
            format!(
                "http://hl7.org/fhir/uv/genomics-reporting/StructureDefinition/{}",
                parent_to_resolve.to_ascii_lowercase()
            ),
            format!(
                "http://hl7.org/fhir/uv/genomics-reporting/StructureDefinition/{}",
                camel_to_kebab(parent_to_resolve)
            ),
        ];

        for candidate in dependency_candidates {
            eprintln!(
                "[GET_BASE_SD] Step 3b: Trying dependency canonical candidate: {}",
                candidate
            );
            if let Some(sd) = self
                .resolve_structure_definition_from_canonical(&candidate)
                .await?
            {
                if is_unexpected_extension(&sd) {
                    eprintln!(
                        "[GET_BASE_SD] Step 3c-ext: Candidate {} resolved to Extension, continuing search",
                        candidate
                    );
                } else {
                    eprintln!(
                        "[GET_BASE_SD] Step 3c: Resolved parent '{}' via dependency canonical {}",
                        parent_to_resolve, candidate
                    );
                    return Ok(sd);
                }
            }
        }

        // 2. Try resolving as FHIR resource by name via canonical manager
        // This handles cases like "USCorePatient" which may be defined as an alias
        // or exists in a loaded package with that name
        eprintln!(
            "[GET_BASE_SD] Step 4: Trying to resolve parent '{}' via canonical manager by name",
            parent_to_resolve
        );
        debug!(
            "Trying to resolve parent '{}' via canonical manager by name",
            parent_to_resolve
        );

        eprintln!(
            "[DEBUG] >>> CRITICAL: About to call session.resolve() for: {}",
            parent_to_resolve
        );
        let resolve_start = std::time::Instant::now();

        let resolve_result = self.session.resolve(parent_to_resolve).await;
        let resolve_elapsed = resolve_start.elapsed();

        eprintln!(
            "[DEBUG] <<< session.resolve() returned after {:?} for: {}",
            resolve_elapsed, parent_to_resolve
        );

        if let Ok(resource) = resolve_result {
            eprintln!(
                "[GET_BASE_SD] Step 6: Found parent '{}' in canonical packages by name",
                parent_to_resolve
            );
            debug!(
                "Found parent '{}' in canonical packages by name",
                parent_to_resolve
            );
            let sd_json = (*resource.content).clone();
            match serde_json::from_value::<StructureDefinition>(sd_json) {
                Ok(sd) => {
                    debug!(
                        "Successfully parsed StructureDefinition for parent: {}",
                        parent_to_resolve
                    );
                    return Ok(sd);
                }
                Err(e) => {
                    debug!(
                        "Failed to parse StructureDefinition for parent {}: {}",
                        parent_to_resolve, e
                    );
                }
            }
        }

        // 3. Try resolving as a FHIR core profile (fast path)
        eprintln!(
            "[DEBUG] >>> Trying FHIR core canonical for: {}",
            parent_to_resolve
        );
        let start_core = std::time::Instant::now();
        let core_candidate = format!(
            "http://hl7.org/fhir/StructureDefinition/{}",
            parent_to_resolve
        );
        if let Some(sd) = self
            .resolve_structure_definition_from_canonical(&core_candidate)
            .await?
        {
            eprintln!(
                "[DEBUG] <<< Found via FHIR core after {:?}",
                start_core.elapsed()
            );
            debug!(
                "Resolved parent {} using FHIR core canonical {}",
                parent_to_resolve, core_candidate
            );
            return Ok(sd);
        }
        eprintln!(
            "[DEBUG] <<< FHIR core lookup failed after {:?}",
            start_core.elapsed()
        );

        // 4. Fallback to searching by ID/name via canonical manager index
        eprintln!("[DEBUG] >>> Searching by ID: {}", parent_to_resolve);
        let start_search = std::time::Instant::now();
        debug!(
            "Searching for StructureDefinition with name/id: {}",
            parent_to_resolve
        );

        // Try multiple search strategies:
        // 1. Search by exact ID match (e.g., "us-core-patient")
        // 2. Search by name match (e.g., "USCorePatient")
        // 3. Convert camelCase to kebab-case and search (e.g., "USCorePatient" -> "us-core-patient")

        let canonical_from_search = if let Ok(Some(resource)) = self
            .session
            .resource_by_type_and_id("StructureDefinition", parent_to_resolve)
            .await
        {
            eprintln!("[DEBUG] <<< Found by ID after {:?}", start_search.elapsed());
            debug!("Found parent by exact ID: {}", parent_to_resolve);
            resource
                .content
                .as_object()
                .and_then(|obj| obj.get("url"))
                .and_then(|url| url.as_str())
                .ok_or_else(|| {
                    ExportError::CanonicalError(format!(
                        "Found resource but missing url field: {}",
                        parent_to_resolve
                    ))
                })?
                .to_string()
        } else {
            // Try by exact name first (SUSHI-compatible: USCoreVitalSignsProfile)
            eprintln!("[DEBUG] Trying by exact name: {}", parent_to_resolve);
            if let Ok(Some(resource)) = self
                .session
                .resource_by_type_and_name("StructureDefinition", parent_to_resolve)
                .await
            {
                eprintln!(
                    "[DEBUG] <<< Found by exact name after {:?}",
                    start_search.elapsed()
                );
                debug!("Found parent by exact name: {}", parent_to_resolve);
                resource.canonical_url.clone()
            } else {
                // Try with "Profile" suffix by name (common US Core pattern: USCoreMedicationRequestProfile)
                let with_profile_suffix = format!("{}Profile", parent_to_resolve);
                debug!(
                    "Trying with Profile suffix by name: {}",
                    with_profile_suffix
                );

                if let Ok(Some(resource)) = self
                    .session
                    .resource_by_type_and_name("StructureDefinition", &with_profile_suffix)
                    .await
                {
                    debug!(
                        "Found parent with Profile suffix by name: {}",
                        with_profile_suffix
                    );
                    resource.canonical_url.clone()
                } else if parent_to_resolve.ends_with("Profile") {
                    // Try WITHOUT "Profile" suffix (USCoreVitalSignsProfile -> USCoreVitalSigns)
                    let without_profile_suffix = &parent_to_resolve[..parent_to_resolve.len() - 7];
                    debug!(
                        "Trying without Profile suffix by name: {}",
                        without_profile_suffix
                    );

                    if let Ok(Some(resource)) = self
                        .session
                        .resource_by_type_and_name("StructureDefinition", without_profile_suffix)
                        .await
                    {
                        debug!(
                            "Found parent without Profile suffix by name: {}",
                            without_profile_suffix
                        );
                        resource.canonical_url.clone()
                    } else {
                        // Try kebab-case variant
                        let kebab_case = camel_to_kebab(parent_to_resolve);
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
                                parent_to_resolve
                            );
                            core_candidate.clone()
                        }
                    }
                } else {
                    let kebab_case = camel_to_kebab(parent_to_resolve);
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
                            parent_to_resolve
                        );
                        core_candidate
                    }
                }
            }
        };

        if let Some(sd) = self
            .resolve_structure_definition_from_canonical(&canonical_from_search)
            .await?
        {
            debug!(
                "Resolved parent {} via canonical {}",
                parent_to_resolve, canonical_from_search
            );
            Ok(sd)
        } else {
            Err(ExportError::ParentNotFound(format!(
                "{}: canonical resolution failed for {}",
                parent_to_resolve, canonical_from_search
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

    /// Resolve a type name to its canonical URL
    ///
    /// Resolution order:
    /// 1. Check local Package for exported StructureDefinitions
    /// 2. Check alias table
    /// 3. Query canonical manager
    /// 4. Try FHIR core canonical URL as fallback
    async fn resolve_type_to_canonical(&self, type_name: &str) -> Option<String> {
        // If already a URL, return as-is
        if type_name.starts_with("http://") || type_name.starts_with("https://") {
            return Some(type_name.to_string());
        }

        // 1. Check local Package for exported StructureDefinitions
        {
            let package = self.package.read().await;
            for (_canonical_url, resource_json) in package.all_resources().iter() {
                if let Ok(sd) =
                    serde_json::from_value::<StructureDefinition>((**resource_json).clone())
                {
                    // Check if name matches
                    if sd.name == type_name {
                        return Some(sd.url.clone());
                    }
                }
            }
        }

        // 2. Check alias table
        if let Some(resolved_url) = self.alias_table.resolve(type_name) {
            return Some(resolved_url.to_string());
        }

        // 3. Try canonical manager by name
        if let Ok(resource) = self.session.resolve(type_name).await {
            if let Ok(sd) =
                serde_json::from_value::<StructureDefinition>((*resource.content).clone())
            {
                return Some(sd.url.clone());
            }
        }

        // 4. Try FHIR core canonical URL format
        let core_candidate = format!("http://hl7.org/fhir/StructureDefinition/{}", type_name);
        if let Ok(Some(_)) = self
            .resolve_structure_definition_from_canonical(&core_candidate)
            .await
        {
            return Some(core_candidate);
        }

        // 5. Fallback: try kebab-case variant
        let kebab_name = camel_to_kebab(type_name);
        if let Ok(Some(resource)) = self
            .session
            .resource_by_type_and_id("StructureDefinition", &kebab_name)
            .await
        {
            if let Ok(sd) =
                serde_json::from_value::<StructureDefinition>((*resource.content).clone())
            {
                return Some(sd.url.clone());
            }
        }

        None
    }

    /// Parse a type constraint string and resolve it to an ElementDefinitionType
    ///
    /// Handles:
    /// - `Reference(TypeName)` → code: "Reference", targetProfile: [resolved URL]
    /// - `Reference(Type1 or Type2)` → code: "Reference", targetProfile: [urls]
    /// - `canonical(TypeName)` → code: "canonical", targetProfile: [resolved URL]
    /// - `CodeableReference(TypeName)` → code: "CodeableReference", targetProfile: [resolved URL]
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

        // For simple types or profile constraints (e.g., "string", "Quantity", "USCorePatient")
        // Check if it's a profile name that should go in the profile field
        if !is_fhir_primitive_or_complex_type(type_str) {
            // This might be a profile name - try to resolve it
            if let Some(canonical_url) = self.resolve_type_to_canonical(type_str).await {
                // It's a profile - the code should be the base type and profile should be the URL
                // For now, just put the resolved URL as the code since we don't know the base type
                // TODO: Look up the profile's type from its StructureDefinition
                return ElementDefinitionType {
                    code: type_str.to_string(),
                    profile: Some(vec![canonical_url]),
                    target_profile: None,
                };
            }
        }

        // Simple type - just use as code
        ElementDefinitionType {
            code: type_str.to_string(),
            profile: None,
            target_profile: None,
        }
    }

    /// Resolve a ValueSet name to its canonical URL
    ///
    /// Resolution order:
    /// 1. Check if already a URL
    /// 2. Check local Package for exported ValueSets
    /// 3. Check alias table
    /// 4. Query canonical manager
    /// 5. Fallback: construct URL with base_url and kebab-case id
    async fn resolve_valueset_url(&self, value_set_name: &str) -> String {
        // 1. If already a URL, return as-is
        if value_set_name.starts_with("http://") || value_set_name.starts_with("https://") {
            return value_set_name.to_string();
        }

        // 2. Check local Package for exported ValueSets
        {
            let package = self.package.read().await;
            for (_canonical_url, resource_json) in package.all_resources().iter() {
                // Try to parse as ValueSet
                if let Some(resource_type) =
                    resource_json.get("resourceType").and_then(|v| v.as_str())
                {
                    if resource_type == "ValueSet" {
                        // Check if name matches
                        if let Some(name) = resource_json.get("name").and_then(|v| v.as_str()) {
                            if name == value_set_name {
                                if let Some(url) = resource_json.get("url").and_then(|v| v.as_str())
                                {
                                    return url.to_string();
                                }
                            }
                        }
                        // Also check if id matches (for cases where id is used instead of name)
                        if let Some(id) = resource_json.get("id").and_then(|v| v.as_str()) {
                            if id == value_set_name || id == camel_to_kebab(value_set_name) {
                                if let Some(url) = resource_json.get("url").and_then(|v| v.as_str())
                                {
                                    return url.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Check alias table
        if let Some(resolved_url) = self.alias_table.resolve(value_set_name) {
            return resolved_url.to_string();
        }

        // 4. Try canonical manager by name
        if let Ok(resource) = self.session.resolve(value_set_name).await {
            if let Some(url) = resource.content.get("url").and_then(|v| v.as_str()) {
                return url.to_string();
            }
        }

        // 5. Try FHIR core ValueSet URL format
        let core_candidate = format!("http://hl7.org/fhir/ValueSet/{}", value_set_name);
        if let Ok(_) = self.session.resolve(&core_candidate).await {
            return core_candidate;
        }

        // 6. Fallback: construct URL with base_url and kebab-case id
        // This matches SUSHI behavior of using the ValueSet's id (typically kebab-case)
        let id = camel_to_kebab(value_set_name);
        format!("{}/ValueSet/{}", self.base_url, id)
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

        // Use id for URL if present, otherwise convert name to kebab-case (SUSHI behavior)
        let profile_id = profile
            .id()
            .and_then(|id_clause| id_clause.value())
            .unwrap_or_else(|| camel_to_kebab(&profile_name));
        structure_def.url = format!("{}/StructureDefinition/{}", self.base_url, profile_id);

        // Set derivation
        structure_def.derivation = Some("constraint".to_string());

        // base_definition should point to parent, not self
        // (this will be set correctly when getting base definition)

        // ALWAYS set the id - either from explicit Id: clause or derived from name
        // BUG FIX: Previously only set if Id: clause was present, leaving parent's id
        structure_def.id = Some(profile_id.clone());

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

        let cardinality_node = rule
            .cardinality()
            .ok_or_else(|| ExportError::InvalidCardinality("missing".to_string()))?;

        trace!(
            "Applying cardinality rule: {} {}",
            path_str, cardinality_node
        );

        // Use structured cardinality access instead of string parsing
        let min = cardinality_node
            .min()
            .ok_or_else(|| ExportError::InvalidCardinality("missing min".to_string()))?;
        let max = cardinality_node
            .max()
            .ok_or_else(|| ExportError::InvalidCardinality("missing max".to_string()))?;

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
        for flag in rule.flags_as_strings() {
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

        for flag in rule.flags_as_strings() {
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

        // Resolve ValueSet to its canonical URL
        let value_set_url = self.resolve_valueset_url(&value_set).await;

        element.binding = Some(ElementDefinitionBinding {
            strength,
            description: None,
            value_set: Some(value_set_url),
        });

        Ok(())
    }

    /// Apply fixed value rule
    ///
    /// Handles FSH value syntax including:
    /// - String values: `"some text"`
    /// - Code values: `#active`, `LNC#31206-6`, `http://loinc.org#31206-6`
    /// - Numeric values: `42`, `3.14`
    /// - Boolean values: `true`, `false`
    ///
    /// Generates correct pattern type based on element type:
    /// - `code` element → `patternCode`
    /// - `Coding` element → `patternCoding` (with system, code, display)
    /// - `CodeableConcept` element → `patternCodeableConcept` (with coding array)
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

        // Handle caret-prefixed paths that were parsed as FixedValueRule
        // These should be applied to the StructureDefinition itself, not elements
        // Example: * ^extension[FMM].valueInteger = 5
        if path_str.starts_with('^') {
            let field = path_str.trim_start_matches('^');
            debug!(
                "Redirecting caret-prefixed FixedValueRule to structure: ^{} = {}",
                field, value
            );
            return self.apply_field_to_structure(structure_def, field, &value);
        }

        let full_path = self.resolve_full_path(structure_def, &path_str).await?;
        let profile_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: profile_name,
            }
        })?;

        // Get the element type to determine correct pattern type
        let element_type = get_element_type(element);

        // Parse value and generate appropriate pattern
        let mut pattern_map = std::collections::HashMap::new();

        // Check if it's a FSH code value (contains #)
        if let Some(fsh_code) = parse_fsh_code_value(&value) {
            // Generate pattern based on element type
            match element_type.as_deref() {
                Some("CodeableConcept") => {
                    // Generate patternCodeableConcept with coding array
                    let mut coding = serde_json::Map::new();
                    if let Some(system) = &fsh_code.system {
                        coding.insert("system".to_string(), JsonValue::String(system.clone()));
                    }
                    coding.insert("code".to_string(), JsonValue::String(fsh_code.code.clone()));
                    if let Some(display) = &fsh_code.display {
                        coding.insert("display".to_string(), JsonValue::String(display.clone()));
                    }

                    let codeable_concept = serde_json::json!({
                        "coding": [JsonValue::Object(coding)]
                    });
                    pattern_map.insert("patternCodeableConcept".to_string(), codeable_concept);
                }
                Some("Coding") => {
                    // Generate patternCoding
                    let mut coding = serde_json::Map::new();
                    if let Some(system) = &fsh_code.system {
                        coding.insert("system".to_string(), JsonValue::String(system.clone()));
                    }
                    coding.insert("code".to_string(), JsonValue::String(fsh_code.code.clone()));
                    if let Some(display) = &fsh_code.display {
                        coding.insert("display".to_string(), JsonValue::String(display.clone()));
                    }
                    pattern_map.insert("patternCoding".to_string(), JsonValue::Object(coding));
                }
                Some("code") | None => {
                    // Generate patternCode (just the code string)
                    pattern_map.insert(
                        "patternCode".to_string(),
                        JsonValue::String(fsh_code.code.clone()),
                    );
                }
                _ => {
                    // Unknown type - default to patternCodeableConcept for safety
                    // as it's the most common use case
                    let mut coding = serde_json::Map::new();
                    if let Some(system) = &fsh_code.system {
                        coding.insert("system".to_string(), JsonValue::String(system.clone()));
                    }
                    coding.insert("code".to_string(), JsonValue::String(fsh_code.code.clone()));
                    if let Some(display) = &fsh_code.display {
                        coding.insert("display".to_string(), JsonValue::String(display.clone()));
                    }

                    let codeable_concept = serde_json::json!({
                        "coding": [JsonValue::Object(coding)]
                    });
                    pattern_map.insert("patternCodeableConcept".to_string(), codeable_concept);
                }
            }
        } else if value.starts_with('"') && value.ends_with('"') {
            // String value
            let string_val = &value[1..value.len() - 1];
            pattern_map.insert(
                "patternString".to_string(),
                JsonValue::String(string_val.to_string()),
            );
        } else if value == "true" || value == "false" {
            // Boolean value
            pattern_map.insert(
                "patternBoolean".to_string(),
                JsonValue::Bool(value == "true"),
            );
        } else if let Ok(int_val) = value.parse::<i64>() {
            // Integer value
            pattern_map.insert(
                "patternInteger".to_string(),
                JsonValue::Number(serde_json::Number::from(int_val)),
            );
        } else if let Ok(float_val) = value.parse::<f64>() {
            // Decimal value
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                pattern_map.insert("patternDecimal".to_string(), JsonValue::Number(num));
            }
        } else {
            // Fallback: treat as code if it looks like one, otherwise string
            if value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                pattern_map.insert("patternCode".to_string(), JsonValue::String(value));
            } else {
                pattern_map.insert("patternString".to_string(), JsonValue::String(value));
            }
        }

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
            return Ok(structure_def.type_field.clone());
        }

        // If path already includes resource type, use as-is
        if path.contains('.') {
            // Use structured path parsing instead of string splitting
            let first_segment = path.split('.').next().unwrap_or("");
            if first_segment == structure_def.type_field {
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
            // Set the type constraint - parse each type to handle Reference, canonical, etc.
            let mut parsed_types = Vec::new();
            for type_str in &types {
                let element_type = self.parse_type_constraint(type_str).await;
                parsed_types.push(element_type);
            }
            element.type_ = Some(parsed_types);

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
        // Handle extension paths: extension[FMM].valueInteger = 5
        if field.starts_with("extension[") {
            return self.apply_extension_to_structure(structure_def, field, value);
        }

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

    /// Apply an extension to the StructureDefinition
    /// Handles paths like: extension[FMM].valueInteger = 5
    fn apply_extension_to_structure(
        &self,
        structure_def: &mut StructureDefinition,
        field: &str,
        value: &str,
    ) -> Result<(), ExportError> {
        // Parse extension path: extension[NAME].valueTYPE
        let re = regex::Regex::new(r"extension\[([^\]]+)\]\.(\w+)").unwrap();
        if let Some(caps) = re.captures(field) {
            let extension_name = caps.get(1).map_or("", |m| m.as_str());
            let value_field = caps.get(2).map_or("", |m| m.as_str());

            // Resolve extension URL from alias table
            let extension_url = self
                .alias_table
                .resolve(extension_name)
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    // If not an alias, assume it's already a URL or construct one
                    if extension_name.starts_with("http://")
                        || extension_name.starts_with("https://")
                    {
                        extension_name.to_string()
                    } else {
                        format!(
                            "{}/StructureDefinition/{}",
                            self.base_url,
                            camel_to_kebab(extension_name)
                        )
                    }
                });

            // Parse the value based on the value field type
            let typed_value: serde_json::Value = match value_field {
                "valueInteger" => {
                    let int_val: i64 = value.parse().map_err(|_| {
                        ExportError::InvalidValue(format!(
                            "Invalid integer value '{}' for {}",
                            value, field
                        ))
                    })?;
                    serde_json::json!(int_val)
                }
                "valueString" => {
                    serde_json::json!(value.trim_matches('"'))
                }
                "valueBoolean" => {
                    serde_json::json!(value == "true")
                }
                "valueCode" => {
                    serde_json::json!(value.trim_start_matches('#'))
                }
                "valueUri" | "valueUrl" | "valueCanonical" => {
                    serde_json::json!(value.trim_matches('"'))
                }
                _ => {
                    // Default to string for unknown types
                    serde_json::json!(value.trim_matches('"'))
                }
            };

            // Create extension object
            let extension = serde_json::json!({
                "url": extension_url,
                value_field: typed_value
            });

            // Add to structure_def.extension
            if structure_def.extension.is_none() {
                structure_def.extension = Some(Vec::new());
            }

            if let Some(ref mut extensions) = structure_def.extension {
                // Check if extension with this URL already exists
                if let Some(existing) = extensions
                    .iter_mut()
                    .find(|e| e.get("url").and_then(|u| u.as_str()) == Some(&extension_url))
                {
                    // Update existing extension
                    if let Some(obj) = existing.as_object_mut() {
                        obj.insert(value_field.to_string(), typed_value);
                    }
                } else {
                    // Add new extension
                    extensions.push(extension);
                }
            }

            debug!(
                "Applied extension: {} = {} (URL: {})",
                field, value, extension_url
            );
        } else {
            warn!("Could not parse extension path: {}", field);
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

                        // Use structured cardinality access
                        if let Some(cardinality_node) = card_rule.cardinality() {
                            if let Some(min) = cardinality_node.min() {
                                element.min = Some(min);
                            }
                            if let Some(max) = cardinality_node.max() {
                                element.max = Some(max);
                            }
                        }

                        // Check for MS flag
                        if card_rule.flags().contains(&FlagValue::MustSupport) {
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
                            if flag_rule.flags().contains(&FlagValue::MustSupport) {
                                existing.must_support = Some(true);
                            }
                        } else {
                            // Create new element
                            let mut element = ElementDefinition::new(full_path.clone());

                            if flag_rule.flags().contains(&FlagValue::MustSupport) {
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
        let _base_url = "http://test.org".to_string();
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
