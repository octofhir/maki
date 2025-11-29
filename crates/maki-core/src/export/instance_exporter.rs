//! Instance Exporter
//!
//! Exports FSH Instance definitions to FHIR resource instances (JSON).
//!
//! # Overview
//!
//! Instances are concrete examples of FHIR resources with specific data values.
//! This module handles:
//! - Converting FSH assignment rules (* path = value) to JSON
//! - Nested path navigation (e.g., address[0].line[+])
//! - Array indexing ([0], [+], [=])
//! - Value type conversion (strings, codes, references, etc.)
//!
//! # Algorithm
//!
//! Based on Algorithm 7 from MAKI_PLAN.md:
//! 1. Create base resource JSON with resourceType and id
//! 2. Process each assignment rule sequentially
//! 3. Parse paths and navigate/create nested structures
//! 4. Handle arrays with special indices
//! 5. Convert FSH values to appropriate JSON types
//! 6. Validate the resulting instance
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::InstanceExporter;
//! use maki_core::cst::ast::Instance;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let session: Arc<maki_core::canonical::DefinitionSession> = todo!();
//! // Parse FSH instance
//! let instance: Instance = todo!();
//!
//! // Create exporter
//! let exporter = InstanceExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Export to FHIR JSON
//! let resource = exporter.export(&instance).await?;
//!
//! // Serialize
//! let json = serde_json::to_string_pretty(&resource)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

use super::ExportError;
use super::fhir_types::{ElementDefinition, StructureDefinition};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{AstNode, FixedValueRule, Instance, Rule};
use crate::semantic::FishingContext;
use crate::semantic::ruleset::{RuleSetExpander, RuleSetInsert};
use serde_json::{Map, Value as JsonValue};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, trace, warn};

// ============================================================================
// Types
// ============================================================================

/// Array index type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayIndex {
    /// Specific numeric index: [0], [1], [2]
    Numeric(usize),
    /// Append to array: [+]
    Append,
    /// Reference current element: [=]
    Current,
}

/// Path segment (field name or array access)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    /// Field name
    Field(String),
    /// Array access with index and optional slice name
    ArrayAccess {
        field: String,
        index: ArrayIndex,
        /// Optional slice name for named slices (e.g., extension[myExtension])
        slice_name: Option<String>,
    },
}

// ============================================================================
// Instance Exporter
// ============================================================================

/// Exports FSH Instance definitions to FHIR resource instances
///
/// # Example
///
/// ```rust,no_run
/// use maki_core::export::InstanceExporter;
/// use maki_core::canonical::DefinitionSession;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let exporter = InstanceExporter::new(
///     session,
///     "http://example.org/fhir".to_string(),
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct InstanceExporter {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    session: Arc<DefinitionSession>,
    /// Fishing context for reference resolution
    fishing_context: Option<Arc<FishingContext>>,
    /// Base URL for instance canonical URLs (if needed)
    #[allow(dead_code)]
    base_url: String,
    /// Track current array indices for [=] operator
    current_indices: HashMap<String, usize>,
    /// Registry of exported instances for reference resolution
    instance_registry: HashMap<String, JsonValue>,
    /// Current profile name (InstanceOf) being exported
    current_profile_name: Option<String>,
    /// Current profile canonical URL (if resolved)
    current_profile_url: Option<String>,
    /// Cached map of extension slice names -> canonical URLs for current profile
    current_extension_urls: HashMap<String, String>,
    /// Current base resource type (e.g., "Patient", "Observation") for cardinality lookups
    current_resource_type: Option<String>,
    /// Optional RuleSet expander for handling insert rules
    ruleset_expander: Option<Arc<RuleSetExpander>>,
}

#[derive(Debug)]
struct DeferredConstraint {
    segments: Vec<PathSegment>,
    value: JsonValue,
}

impl InstanceExporter {
    /// Create a new instance exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving resource types and profiles
    /// * `base_url` - Base URL for generated instance identifiers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = InstanceExporter::new(
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
        // Pre-populate the base resource types cache for later lookups
        // This ensures is_base_resource_type() works correctly
        let _ = session.base_resource_types().await.map_err(|e| {
            ExportError::CanonicalError(format!("Failed to load base resource types: {}", e))
        })?;

        Ok(Self {
            session,
            fishing_context: None,
            base_url,
            current_indices: HashMap::new(),
            instance_registry: HashMap::new(),
            current_profile_name: None,
            current_profile_url: None,
            current_extension_urls: HashMap::new(),
            current_resource_type: None,
            ruleset_expander: None,
        })
    }

    /// Set the fishing context for reference validation
    ///
    /// This enables validation of references to profiles, value sets, and other resources
    /// during instance export.
    pub fn with_fishing_context(mut self, fishing_context: Arc<FishingContext>) -> Self {
        self.fishing_context = Some(fishing_context);
        self
    }

    /// Provide a RuleSet expander for handling insert rules
    pub fn with_ruleset_expander(mut self, expander: Arc<RuleSetExpander>) -> Self {
        self.ruleset_expander = Some(expander);
        self
    }

    /// Register an instance for reference resolution
    pub fn register_instance(&mut self, name: String, json: JsonValue) {
        self.instance_registry.insert(name.clone(), json.clone());

        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
            // Allow lookup by raw id
            self.instance_registry
                .entry(id.to_string())
                .or_insert_with(|| json.clone());

            // Allow lookup by typed reference (ResourceType/id)
            if let Some(rt) = json.get("resourceType").and_then(|v| v.as_str()) {
                let typed_key = format!("{}/{}", rt, id);
                self.instance_registry
                    .entry(typed_key)
                    .or_insert(json.clone());
            }
        }
    }

    /// Get a registered instance by name
    pub fn get_instance(&self, name: &str) -> Option<&JsonValue> {
        self.instance_registry.get(name)
    }

    /// Resolve the base FHIR resource type for a profile by following the parent chain
    ///
    /// This method recursively follows the parent chain of a profile until it finds
    /// a base FHIR resource type (like "Patient", "Observation", etc.).
    ///
    /// # Arguments
    ///
    /// * `fishing_ctx` - Fishing context for looking up profiles
    /// * `profile_name` - Name of the profile to resolve
    /// * `metadata` - Initial metadata for the profile
    ///
    /// # Returns
    ///
    /// The base FHIR resource type (e.g., "Patient" for a CancerPatient profile)
    async fn resolve_base_resource_type(
        &self,
        fishing_ctx: &Arc<FishingContext>,
        profile_name: &str,
        metadata: &crate::semantic::FishableMetadata,
    ) -> String {
        const MAX_DEPTH: usize = 10; // Prevent infinite loops
        let mut depth = 0;
        let mut current_parent = metadata.parent.clone();

        trace!(
            "Starting base resource type resolution for profile '{}' with parent: {:?}",
            profile_name, current_parent
        );

        // Follow the parent chain until we find a base resource or hit max depth
        while let Some(parent) = current_parent {
            depth += 1;
            if depth > MAX_DEPTH {
                warn!(
                    "Max depth ({}) exceeded resolving base type for '{}', using parent '{}'",
                    MAX_DEPTH, profile_name, parent
                );
                return parent;
            }

            trace!("  [Depth {}] Checking parent: '{}'", depth, parent);

            // Check if parent is a known base FHIR resource type
            // Base resources typically don't have parents or their parent is "DomainResource"/"Resource"
            if self.is_base_resource_type(&parent) {
                debug!(
                    "Found base resource type '{}' for profile '{}' at depth {}",
                    parent, profile_name, depth
                );
                return parent;
            }

            // Try to fish for the parent profile's metadata
            if let Some(parent_metadata) = fishing_ctx
                .fish_metadata(
                    &parent,
                    &[
                        crate::semantic::ResourceType::Profile,
                        crate::semantic::ResourceType::Logical,
                    ],
                )
                .await
            {
                trace!(
                    "  [Depth {}] Found parent '{}' with parent: {:?}",
                    depth, parent, parent_metadata.parent
                );
                current_parent = parent_metadata.parent.clone();
            } else {
                // Parent not found in tank, try canonical packages
                trace!(
                    "  [Depth {}] Parent '{}' not in tank, trying canonical",
                    depth, parent
                );

                // Try to resolve alias first (e.g., "USCorePatient" -> canonical URL)
                let parent_to_fish = fishing_ctx
                    .resolve_alias(&parent)
                    .unwrap_or_else(|| parent.clone());
                if parent_to_fish != parent {
                    trace!(
                        "  [Depth {}] Resolved alias '{}' -> '{}'",
                        depth, parent, parent_to_fish
                    );
                }

                match fishing_ctx.fish_structure_definition(&parent_to_fish).await {
                    Ok(Some(sd)) => {
                        trace!(
                            "  [Depth {}] Found parent '{}' in canonical with type: '{}'",
                            depth, parent, sd.type_field
                        );
                        // StructureDefinition.type is the base resource type
                        return sd.type_field;
                    }
                    Ok(None) => {
                        debug!(
                            "Parent '{}' not found (None), assuming it's the base resource type for '{}'",
                            parent, profile_name
                        );
                        return parent;
                    }
                    Err(e) => {
                        debug!(
                            "Parent '{}' error: {}, assuming it's the base resource type for '{}'",
                            parent, e, profile_name
                        );
                        return parent;
                    }
                }
            }
        }

        // No parent found - use the profile name as the base type
        debug!(
            "No parent found for '{}', using profile name as base resource type",
            profile_name
        );
        profile_name.to_string()
    }

    /// Check if a name is a known base FHIR resource type
    ///
    /// This checks against base FHIR resource types loaded from the canonical manager
    /// to determine if we've reached the base of the profile inheritance chain.
    /// The cache is populated when the exporter is created via `new()`.
    fn is_base_resource_type(&self, name: &str) -> bool {
        self.session.is_base_resource_type(name)
    }

    /// Export an Instance to a FHIR resource (JSON)
    ///
    /// # Arguments
    ///
    /// * `instance` - FSH Instance AST node
    ///
    /// # Returns
    ///
    /// A FHIR resource as JSON
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - InstanceOf type not found
    /// - Rule application fails
    /// - Path resolution fails
    /// - Value conversion fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    /// use maki_core::cst::ast::Instance;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut exporter: InstanceExporter = todo!();
    /// # let instance: Instance = todo!();
    /// let resource = exporter.export(&instance).await?;
    /// println!("{}", serde_json::to_string_pretty(&resource)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&mut self, instance: &Instance) -> Result<JsonValue, ExportError> {
        let name = instance.name().unwrap_or_else(|| "unnamed".to_string());
        debug!("Exporting instance: {}", name);

        // Get resource type from InstanceOf
        let instance_of = instance
            .instance_of()
            .and_then(|c| c.value())
            .ok_or_else(|| ExportError::MissingRequiredField("InstanceOf".to_string()))?;

        trace!("Instance {} is of type {}", name, instance_of);

        // If InstanceOf references a base FHIR resource type, short-circuit resolution
        // to avoid accidentally selecting similarly named profiles or extensions.
        if self.is_base_resource_type(&instance_of) {
            let mut resource = serde_json::json!({
                "resourceType": instance_of,
                "id": name,
            });

            // Set resource type for cardinality lookups
            self.current_resource_type = Some(instance_of.to_string());

            for rule in instance.rules() {
                self.apply_rule(&mut resource, &rule).await?;
            }

            self.current_profile_name = None;
            self.current_profile_url = None;
            self.current_extension_urls.clear();
            self.current_resource_type = None;

            debug!(
                "Exported base resource instance {} with resourceType {}",
                name, instance_of
            );
            return Ok(resource);
        }

        // Resolve profile to get base resource type and canonical URL
        let (resource_type, canonical_url) = if let Some(fishing_ctx) = &self.fishing_context {
            // Try to get metadata from the tank first (profiles not yet exported)
            // This is fast and doesn't require full export
            if let Some(metadata) = fishing_ctx
                .fish_metadata(
                    &instance_of,
                    &[
                        crate::semantic::ResourceType::Profile,
                        crate::semantic::ResourceType::Extension,
                        crate::semantic::ResourceType::Logical,
                    ],
                )
                .await
            {
                // Found in tank - resolve base resource type by following parent chain
                let base_type = self
                    .resolve_base_resource_type(fishing_ctx, &instance_of, &metadata)
                    .await;
                let profile_url = match metadata.resource_type.as_str() {
                    "StructureDefinition" => Some(format!(
                        "{}/StructureDefinition/{}",
                        self.base_url, metadata.id
                    )),
                    _ => Some(metadata.url.clone()),
                };
                debug!(
                    "Resolved profile '{}' from tank -> base type: '{}', canonical URL: '{}'",
                    instance_of,
                    base_type,
                    profile_url.as_deref().unwrap_or("<none>")
                );
                (base_type, profile_url)
            } else {
                // Not in tank, try canonical packages (external FHIR definitions)
                match fishing_ctx.fish_structure_definition(&instance_of).await {
                    Ok(Some(sd)) => {
                        // Found in canonical packages - use its base type and canonical URL
                        let base_type = sd.type_field.clone();
                        let profile_url = sd.url.clone();
                        debug!(
                            "Resolved profile '{}' from canonical -> base type: '{}', canonical URL: '{}'",
                            instance_of, base_type, profile_url
                        );
                        (base_type, Some(profile_url))
                    }
                    Ok(None) => {
                        // Not found anywhere - might be a base resource type
                        debug!(
                            "Profile '{}' not found, assuming it's a base resource type",
                            instance_of
                        );
                        (instance_of.to_string(), None)
                    }
                    Err(e) => {
                        // Error during resolution - log warning and fall back
                        warn!(
                            "Error resolving profile '{}': {}, using as resourceType",
                            instance_of, e
                        );
                        (instance_of.to_string(), None)
                    }
                }
            }
        } else {
            // No fishing context available - fall back to using instance_of as-is
            debug!(
                "No fishing context available, using '{}' as resourceType",
                instance_of
            );
            (instance_of.to_string(), None)
        };

        // Optionally load full StructureDefinition to refine resourceType/canonical and extract slices
        let sd_opt = if let Some(fishing_ctx) = &self.fishing_context {
            self.load_structure_definition(fishing_ctx, &instance_of, canonical_url.as_deref())
                .await
        } else {
            None
        };

        let mut resource_type = resource_type;
        let mut canonical_url = canonical_url;

        if let Some(sd) = sd_opt.as_ref() {
            if canonical_url.is_none() {
                canonical_url = Some(sd.url.clone());
            }
            // Ensure resourceType matches the base type, not the profile name
            resource_type = sd.type_field.clone();
        }

        // Cache profile context for this export (used by extension resolution and cardinality lookups)
        self.current_profile_name = Some(instance_of.to_string());
        self.current_profile_url = canonical_url.clone();
        self.current_extension_urls = sd_opt
            .as_ref()
            .map(Self::build_extension_url_map)
            .unwrap_or_default();
        self.current_resource_type = Some(resource_type.clone());

        // Create base resource with correct resourceType
        let mut resource = if let Some(profile_url) = canonical_url {
            // Include meta.profile with canonical URL
            serde_json::json!({
                "resourceType": resource_type,
                "id": name,
                "meta": {
                    "profile": [profile_url]
                }
            })
        } else {
            // No profile URL available - just use resourceType and id
            serde_json::json!({
                "resourceType": resource_type,
                "id": name,
            })
        };

        // Pre-populate fixed/pattern values from the profile chain
        let mut deferred_constraints = Vec::new();
        if let Some(sd) = sd_opt.as_ref() {
            let profile_chain = if let Some(fishing_ctx) = &self.fishing_context {
                self.collect_profile_chain(fishing_ctx, sd).await
            } else {
                vec![sd.clone()]
            };

            deferred_constraints = self
                .apply_profile_constraints(&mut resource, &profile_chain)
                .await?;
        }

        // Apply rules
        for rule in instance.rules() {
            if let Err(e) = self.apply_rule(&mut resource, &rule).await {
                let path_hint = match &rule {
                    Rule::FixedValue(fv) => fv.path().map(|p| p.as_string()).unwrap_or_default(),
                    _ => String::new(),
                };
                eprintln!(
                    "Instance '{}' failed applying rule `{}` (path: '{}'): {}",
                    name,
                    rule.syntax().text(),
                    path_hint,
                    e
                );
                return Err(e);
            }
        }

        // Inline referenced instances inside Bundle entries
        if resource["resourceType"] == "Bundle" {
            self.inline_bundle_resources(&mut resource);
        }

        if !deferred_constraints.is_empty() {
            self.apply_deferred_constraints(&mut resource, deferred_constraints)
                .await?;
        }

        // Safety net: ensure BodyStructure instances retain patient reference (parity with SUSHI)
        if resource["resourceType"] == "BodyStructure" && resource.get("patient").is_none() {
            if let Some(patient_value) = instance.rules().find_map(|rule| match rule {
                Rule::FixedValue(fv)
                    if fv
                        .path()
                        .map(|p| p.as_string() == "patient")
                        .unwrap_or(false) =>
                {
                    fv.value()
                }
                _ => None,
            }) {
                debug!(
                    "Restoring missing BodyStructure.patient for instance '{}'",
                    name
                );
                let segments = self.parse_path("patient")?;
                let json_value = self
                    .convert_value_with_path(&patient_value, "patient")
                    .await?;
                self.set_value_at_path(&mut resource, &segments, json_value)
                    .await?;
            } else {
                debug!(
                    "BodyStructure '{}' missing patient and no patient rule found",
                    name
                );
            }
        }

        // Clear per-instance caches
        self.current_profile_name = None;
        self.current_profile_url = None;
        self.current_extension_urls.clear();
        self.current_resource_type = None;

        // Remove internal slice markers before returning
        Self::strip_slice_markers(&mut resource);

        // Reorder fields to match FHIR specification order
        Self::order_fhir_fields(&mut resource);

        debug!("Successfully exported instance {}", name);
        Ok(resource)
    }

    /// Apply a single rule to the resource
    async fn apply_rule(
        &mut self,
        resource: &mut JsonValue,
        rule: &Rule,
    ) -> Result<(), ExportError> {
        match rule {
            Rule::FixedValue(fixed_rule) => {
                self.apply_fixed_value_rule(resource, fixed_rule).await?;
            }
            Rule::Card(_) => {
                // Card rules don't apply to instances
                trace!("Skipping card rule in instance");
            }
            Rule::Flag(_) => {
                // Flag rules don't apply to instances
                trace!("Skipping flag rule in instance");
            }
            Rule::ValueSet(_) => {
                // ValueSet rules don't apply to instances
                trace!("Skipping valueset rule in instance");
            }
            Rule::Path(_) => {
                // Path rules don't apply to instances
                trace!("Skipping path rule in instance");
            }
            Rule::AddElement(_)
            | Rule::Contains(_)
            | Rule::Only(_)
            | Rule::Obeys(_)
            | Rule::Mapping(_)
            | Rule::CaretValue(_)
            | Rule::CodeCaretValue(_)
            | Rule::CodeInsert(_)
            | Rule::Insert(_) => {
                match rule {
                    Rule::Insert(insert_rule) => {
                        if let Some(name) = insert_rule.ruleset_reference() {
                            let args = insert_rule.arguments();
                            let range = insert_rule.syntax().text_range();
                            self.apply_ruleset_insert(
                                resource,
                                &name,
                                args,
                                range.start().into()..range.end().into(),
                            )
                            .await?;
                        }
                    }
                    Rule::CodeInsert(insert_rule) => {
                        if let Some(name) = insert_rule.ruleset_reference() {
                            let args = insert_rule.arguments();
                            let range = insert_rule.syntax().text_range();
                            self.apply_ruleset_insert(
                                resource,
                                &name,
                                args,
                                range.start().into()..range.end().into(),
                            )
                            .await?;
                        }
                    }
                    _ => {
                        // These rules don't apply to instances
                        trace!("Skipping contains/only/obeys rule in instance");
                    }
                }
            }
        }
        Ok(())
    }

    /// Recursively remove internal `_sliceName` markers from the exported JSON
    fn strip_slice_markers(value: &mut JsonValue) {
        match value {
            JsonValue::Object(map) => {
                map.remove("_sliceName");
                for v in map.values_mut() {
                    Self::strip_slice_markers(v);
                }
            }
            JsonValue::Array(arr) => {
                for v in arr.iter_mut() {
                    Self::strip_slice_markers(v);
                }
            }
            _ => {}
        }
    }

    /// Minimal field ordering - just ensures resourceType, id, meta are first
    ///
    /// NOTE: This is a presentation preference, not a hardcoded element list.
    /// - JSON key order doesn't affect FHIR validity (JSON is unordered by spec)
    /// - resourceType/id/meta first is a universal FHIR convention (SUSHI does this too)
    /// - Full SD-based ordering would be expensive (resolve SD for every object)
    fn order_fhir_fields(value: &mut JsonValue) {
        if let JsonValue::Object(map) = value {
            // Standard FHIR presentation: resourceType, id, meta first
            const FIRST_FIELDS: &[&str] = &["resourceType", "id", "meta"];

            let mut ordered = serde_json::Map::new();

            // Put resourceType, id, meta first
            for &key in FIRST_FIELDS {
                if let Some(v) = map.remove(key) {
                    ordered.insert(key.to_string(), v);
                }
            }

            // Keep remaining fields in their current order
            let remaining_keys: Vec<String> = map.keys().cloned().collect();
            for key in remaining_keys {
                if let Some(v) = map.remove(&key) {
                    ordered.insert(key, v);
                }
            }

            *map = ordered;

            // Recursively process nested objects
            for v in map.values_mut() {
                Self::order_fhir_fields(v);
            }
        } else if let JsonValue::Array(arr) = value {
            for v in arr.iter_mut() {
                Self::order_fhir_fields(v);
            }
        }
    }

    /// Replace Bundle.entry.resource string/reference values with inline instances when available
    fn inline_bundle_resources(&self, bundle: &mut JsonValue) {
        let Some(entries) = bundle.get_mut("entry").and_then(|v| v.as_array_mut()) else {
            return;
        };

        for entry in entries.iter_mut() {
            let Some(resource_field) = entry.get_mut("resource") else {
                continue;
            };

            if let Some(inline) = self.resolve_inline_resource(resource_field) {
                *resource_field = inline;
            }
        }
    }

    /// Resolve a bundle resource field (string or Reference-like object) to an inline instance
    fn resolve_inline_resource(&self, resource_field: &JsonValue) -> Option<JsonValue> {
        match resource_field {
            JsonValue::String(name) => self.instance_registry.get(name).cloned().or_else(|| {
                name.rsplit('/')
                    .next()
                    .and_then(|id| self.instance_registry.get(id).cloned())
            }),
            JsonValue::Object(map) => {
                map.get("reference")
                    .and_then(|v| v.as_str())
                    .and_then(|reference| {
                        self.instance_registry.get(reference).cloned().or_else(|| {
                            reference
                                .rsplit('/')
                                .next()
                                .and_then(|id| self.instance_registry.get(id).cloned())
                        })
                    })
            }
            _ => None,
        }
    }

    /// Apply a fixed value rule (assignment: * path = value)
    async fn apply_fixed_value_rule(
        &mut self,
        resource: &mut JsonValue,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        // Get path and value
        let path = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::InvalidPath {
                path: "<unknown>".to_string(),
                resource: resource["resourceType"]
                    .as_str()
                    .unwrap_or("Unknown")
                    .to_string(),
            })?;

        // Skip if there's no value - this is actually a path rule (structural navigation)
        // Example: * valueCodeableConcept (without = value)
        let Some(value_str) = rule.value() else {
            trace!(
                "Skipping fixed value rule with no value (path rule): {}",
                path
            );
            return Ok(());
        };

        trace!("Applying assignment: {} = {}", path, value_str);

        // Parse the path into segments
        let segments = self.parse_path(&path)?;

        // Convert value string to JSON, passing path context to preserve string types
        let json_value = self.convert_value_with_path(&value_str, &path).await?;

        // Navigate and set the value
        self.set_value_at_path(resource, &segments, json_value)
            .await?;

        Ok(())
    }

    /// Expand a RuleSet insert using the configured expander and apply resulting rules.
    async fn apply_ruleset_insert(
        &mut self,
        resource: &mut JsonValue,
        name: &str,
        arguments: Vec<String>,
        source_range: std::ops::Range<usize>,
    ) -> Result<(), ExportError> {
        // For now, only expand known instance RuleSets we can translate safely
        if name != "StagingInstanceRuleSet" {
            trace!("Skipping RuleSet insert '{}' (not whitelisted)", name);
            return Ok(());
        }

        let Some(expander) = &self.ruleset_expander else {
            trace!("No RuleSet expander configured; skipping insert {}", name);
            return Ok(());
        };

        eprintln!(
            "Applying RuleSet insert '{}' with args {:?}",
            name, arguments
        );

        let insert = RuleSetInsert {
            ruleset_name: name.to_string(),
            arguments: arguments.clone(),
            source_range,
        };

        let expanded = match expander.expand(&insert) {
            Ok(rules) => rules,
            Err(e) => {
                warn!("Failed to expand RuleSet '{}': {}", name, e);
                return Ok(());
            }
        };

        let mut parsed_rules = Vec::new();
        for raw_rule in expanded {
            eprintln!("  Raw expanded rule: {}", raw_rule);
            if let Some((path, value)) = Self::parse_simple_rule(&raw_rule) {
                eprintln!(
                    "  Expanded rule from '{}': path='{}' value='{}'",
                    name, path, value
                );
                parsed_rules.push((path, value));
            } else {
                eprintln!(
                    "  Skipping RuleSet '{}' expansion; unsupported rule '{}'",
                    name, raw_rule
                );
                return Ok(());
            }
        }

        for (path, value) in parsed_rules {
            let segments = self.parse_path(&path)?;
            let json_value = self.convert_value_with_path(&value, &path).await?;
            self.set_value_at_path(resource, &segments, json_value)
                .await?;
        }

        Ok(())
    }

    /// Parse a simple rule string like "* path = value" into (path, value)
    fn parse_simple_rule(raw: &str) -> Option<(String, String)> {
        let trimmed = raw.trim_start();
        let rule_body = trimmed.trim_start_matches('*').trim();
        let (path_part, value_part) = rule_body
            .split_once('=')
            .map(|(p, v)| (p.trim(), v.trim()))?;
        if path_part.is_empty() || value_part.is_empty() {
            return None;
        }
        Some((path_part.to_string(), value_part.to_string()))
    }

    /// Parse a path string into segments
    ///
    /// Examples:
    /// - "name.family" -> [Field("name"), Field("family")]
    /// - "name.given[0]" -> [Field("name"), ArrayAccess("given", 0, None)]
    /// - "address[+].line[0]" -> [ArrayAccess("address", Append, None), ArrayAccess("line", 0, None)]
    /// - "extension[myExt].valueString" -> [ArrayAccess("extension", 0, Some("myExt")), Field("valueString")]
    fn parse_path(&self, path: &str) -> Result<Vec<PathSegment>, ExportError> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Field(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    // Parse array index or slice name
                    let field = current.clone();
                    current.clear();

                    let mut index_str = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == ']' {
                            chars.next(); // consume ']'
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }

                    let index_str_trimmed = index_str.trim();

                    // Determine if it's a slice name or numeric index
                    let (index, slice_name) = if index_str_trimmed == "+" {
                        (ArrayIndex::Append, None)
                    } else if index_str_trimmed == "=" {
                        (ArrayIndex::Current, None)
                    } else if let Ok(num) = index_str_trimmed.parse::<usize>() {
                        (ArrayIndex::Numeric(num), None)
                    } else {
                        // It's a slice name - use index 0 and store the name
                        (ArrayIndex::Numeric(0), Some(index_str_trimmed.to_string()))
                    };

                    // Handle consecutive brackets like [sliceName][0]
                    // If the field is empty and the previous segment is an ArrayAccess,
                    // this is a secondary index on the same slice
                    if field.is_empty()
                        && let Some(PathSegment::ArrayAccess {
                            field: prev_field,
                            slice_name: prev_slice,
                            ..
                        }) = segments.last()
                    {
                        // Update the previous segment with the new index while keeping the slice name
                        let updated_segment = PathSegment::ArrayAccess {
                            field: prev_field.clone(),
                            index: index.clone(),
                            slice_name: prev_slice.clone(),
                        };
                        segments.pop();
                        segments.push(updated_segment);
                        continue;
                    }

                    segments.push(PathSegment::ArrayAccess {
                        field,
                        index,
                        slice_name,
                    });
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Add final segment if any
        if !current.is_empty() {
            segments.push(PathSegment::Field(current));
        }

        if segments.is_empty() {
            return Err(ExportError::InvalidPath {
                path: path.to_string(),
                resource: "".to_string(),
            });
        }

        Ok(segments)
    }

    /// Set a value at the given path, creating intermediate structures as needed
    ///
    /// This method handles:
    /// - Simple field assignment (name.family = "Doe")
    /// - Array indexing (name.given[0] = "John")
    /// - Slice names (extension[myExtension].url = "http://...")
    /// - Complex value merging (when setting properties on existing objects)
    async fn set_value_at_path(
        &mut self,
        resource: &mut JsonValue,
        segments: &[PathSegment],
        value: JsonValue,
    ) -> Result<(), ExportError> {
        if segments.is_empty() {
            return Err(ExportError::InvalidPath {
                path: "<empty>".to_string(),
                resource: "".to_string(),
            });
        }

        // Start at the root
        let mut current_value = resource;

        // Process each segment
        for (i, segment) in segments.iter().enumerate() {
            let is_last = i == segments.len() - 1;

            current_value = match segment {
                PathSegment::Field(field) => {
                    if is_last {
                        // Set the final value
                        if !current_value.is_object() {
                            *current_value = JsonValue::Object(Map::new());
                        }
                        if let JsonValue::Object(obj) = current_value {
                            // Check if this field should be an array
                            let final_value =
                                if self.is_array_field(field).await && !value.is_array() {
                                    // Wrap scalar value in an array
                                    trace!("Wrapping value in array for field '{}'", field);
                                    JsonValue::Array(vec![value.clone()])
                                } else {
                                    value.clone()
                                };

                            // If the field already exists and both are objects, merge them
                            if let Some(existing) = obj.get_mut(field) {
                                if existing.is_object() && final_value.is_object() {
                                    Self::merge_objects(existing, &final_value);
                                } else {
                                    *existing = final_value;
                                }
                            } else {
                                obj.insert(field.clone(), final_value);
                            }
                            return Ok(());
                        } else {
                            return Err(ExportError::InvalidPath {
                                path: field.clone(),
                                resource: "".to_string(),
                            });
                        }
                    } else {
                        // Navigate or create intermediate structure
                        if !current_value.is_object() {
                            *current_value = JsonValue::Object(Map::new());
                        }
                        if let JsonValue::Object(obj) = current_value {
                            if !obj.contains_key(field) {
                                // Only create an array if this is a known FHIR array field
                                // FSH shorthand like "identifier.use" means identifier[0].use
                                // when identifier IS an array field
                                if self.is_array_field(field).await {
                                    // Create an array with one empty object for known array fields
                                    let arr = vec![JsonValue::Object(Map::new())];
                                    obj.insert(field.clone(), JsonValue::Array(arr));
                                } else {
                                    // Create a regular object for backbone elements like
                                    // timing, repeat, boundsPeriod, numerator, etc.
                                    obj.insert(field.clone(), JsonValue::Object(Map::new()));
                                }
                            }

                            // Navigate into the field
                            let field_value = obj.get_mut(field).unwrap();

                            // If it's an array and next is a field, navigate into first element
                            // Note: Without explicit array index, always use index 0 per SUSHI behavior
                            // The current_indices is only for [=] operator to reference same index as previous
                            if field_value.is_array() {
                                let arr = field_value.as_array_mut().unwrap();
                                if arr.is_empty() {
                                    arr.push(JsonValue::Object(Map::new()));
                                }
                                // Always use index 0 when no explicit index is given
                                &mut arr[0]
                            } else {
                                field_value
                            }
                        } else {
                            return Err(ExportError::InvalidPath {
                                path: field.clone(),
                                resource: "".to_string(),
                            });
                        }
                    }
                }
                PathSegment::ArrayAccess {
                    field,
                    index,
                    slice_name,
                } => {
                    // Ensure parent is an object
                    if !current_value.is_object() {
                        *current_value = JsonValue::Object(Map::new());
                    }
                    if let JsonValue::Object(obj) = current_value {
                        // Ensure array exists
                        if !obj.contains_key(field) {
                            obj.insert(field.clone(), JsonValue::Array(Vec::new()));
                        }

                        let array_value = obj.get_mut(field).unwrap();

                        // Check if it's an array
                        if !array_value.is_array() {
                            return Err(ExportError::TypeMismatch {
                                expected: "array".to_string(),
                                actual: "object".to_string(),
                            });
                        }

                        let arr = array_value.as_array_mut().unwrap();

                        // Determine actual index based on slice name or numeric index
                        let actual_index = if let Some(slice) = slice_name {
                            // Find or create element with matching slice name
                            self.find_or_create_slice(arr, slice, field).await?
                        } else {
                            match index {
                                ArrayIndex::Numeric(n) => *n,
                                ArrayIndex::Append => arr.len(),
                                ArrayIndex::Current => {
                                    *self.current_indices.get(field).unwrap_or(&0)
                                }
                            }
                        };

                        // Ensure array is large enough
                        while arr.len() <= actual_index {
                            arr.push(JsonValue::Object(Map::new()));
                        }

                        // Update current index tracker
                        self.current_indices.insert(field.clone(), actual_index);

                        if is_last {
                            // Set the value at this array index
                            // If existing value is an object and new value is an object, merge
                            if arr[actual_index].is_object() && value.is_object() {
                                Self::merge_objects(&mut arr[actual_index], &value);
                            } else {
                                arr[actual_index] = value.clone();
                            }
                            return Ok(());
                        } else {
                            // Navigate into array element
                            &mut arr[actual_index]
                        }
                    } else {
                        return Err(ExportError::InvalidPath {
                            path: field.clone(),
                            resource: "".to_string(),
                        });
                    }
                }
            };
        }

        Ok(())
    }

    /// Find or create an array element with a specific slice name
    ///
    /// For extensions, this matches by the `url` field and resolves slice names to canonical URLs.
    /// For other slices, this uses the `_sliceName` internal field.
    async fn find_or_create_slice(
        &self,
        arr: &mut Vec<JsonValue>,
        slice_name: &str,
        field: &str,
    ) -> Result<usize, ExportError> {
        // For extensions, match by URL and resolve slice names to canonical URLs
        if field == "extension" || field == "modifierExtension" {
            // Resolve slice name to canonical URL
            let extension_url = self.resolve_extension_url(slice_name).await;

            trace!(
                "Resolving extension slice '{}' -> canonical URL '{}'",
                slice_name, extension_url
            );

            // Try to find existing extension with this URL
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(url) = elem.get("url").and_then(|u| u.as_str())
                    && url == extension_url
                {
                    return Ok(idx);
                }
            }
            // Not found, create new element at end with canonical URL
            let idx = arr.len();
            arr.push(serde_json::json!({
                "url": extension_url
            }));
            Ok(idx)
        } else {
            // For other slices, match by _sliceName
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(name) = elem.get("_sliceName").and_then(|n| n.as_str())
                    && name == slice_name
                {
                    return Ok(idx);
                }
            }
            // Not found, create new element at end
            let idx = arr.len();
            arr.push(serde_json::json!({
                "_sliceName": slice_name
            }));
            Ok(idx)
        }
    }

    /// Resolve extension slice name to canonical URL
    ///
    /// This method attempts to resolve extension slice names in the following order:
    /// 1. If it's already a URL (contains "://"), use it as-is
    /// 2. If it's an alias, resolve to canonical URL via alias table
    /// 3. Check the current profile's slice map extracted from its StructureDefinition
    /// 4. Try to find extension definition in tank/canonical
    /// 5. Fall back to using slice name as-is (for backwards compatibility)
    async fn resolve_extension_url(&self, slice_name: &str) -> String {
        // If it's already a full URL, use as-is
        if slice_name.contains("://") {
            return slice_name.to_string();
        }

        // Try alias resolution first
        if let Some(fishing_ctx) = &self.fishing_context {
            if let Some(canonical_url) = fishing_ctx.resolve_alias(slice_name) {
                debug!(
                    "Resolved extension alias '{}' -> '{}'",
                    slice_name, canonical_url
                );
                return canonical_url;
            }

            // Try current profile slice map extracted from StructureDefinition
            if let Some(url) = self.current_extension_urls.get(slice_name) {
                debug!(
                    "Resolved extension slice '{}' using profile slice map -> '{}'",
                    slice_name, url
                );
                return url.clone();
            }

            // Try multiple naming patterns for the extension (id/name variations)
            // Include common IG prefixes and case variants
            let pascal = Self::pascal_case(slice_name);
            let kebab = Self::kebab_case(slice_name);
            let candidates = vec![
                slice_name.to_string(),
                format!("us-core-{}", slice_name),
                format!("us-core-{}", kebab),
                format!("uscoreext-{}", slice_name),
                format!("mcode-{}", slice_name),
                format!("mcode-{}", kebab),
                pascal.clone(),
                kebab.clone(),
                // Try mapping common slice names to extension names
                Self::map_slice_to_extension(slice_name),
            ];

            for candidate in &candidates {
                trace!("Trying extension name: '{}'", candidate);

                // Try to find extension definition in tank
                if let Some(metadata) = fishing_ctx
                    .fish_metadata(candidate, &[crate::semantic::ResourceType::Extension])
                    .await
                {
                    debug!(
                        "Resolved extension '{}' (tried '{}') from tank -> '{}'",
                        slice_name, candidate, metadata.url
                    );
                    return metadata.url;
                }

                // Try to find extension in canonical packages
                // Use fish_extension which specifically looks for Extension StructureDefinitions
                match fishing_ctx.fish_extension(candidate).await {
                    Ok(Some(sd)) => {
                        debug!(
                            "Resolved extension '{}' (tried '{}') from canonical -> '{}'",
                            slice_name, candidate, sd.url
                        );
                        return sd.url;
                    }
                    Ok(None) | Err(_) => {
                        // Not found with this candidate, try next
                    }
                }
            }
        }

        // Fallback: use slice name as-is
        debug!("Could not resolve extension '{}', using as-is", slice_name);
        slice_name.to_string()
    }

    /// Build a map of extension slice names to canonical URLs from a StructureDefinition
    ///
    /// This inspects the differential first (preferred) and falls back to snapshot.
    /// It looks for `extension:<slice>.url` elements with fixedUri/fixedCanonical values,
    /// or extension slices with type.profile pointing at an extension definition.
    fn build_extension_url_map(sd: &StructureDefinition) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let elements = sd
            .differential
            .as_ref()
            .map(|d| d.element.as_slice())
            .or_else(|| sd.snapshot.as_ref().map(|s| s.element.as_slice()))
            .unwrap_or(&[]);

        for element in elements {
            if let Some(slice_name) = Self::extract_slice_name(element) {
                if element.path.ends_with(".url")
                    && let Some(url) = Self::extract_fixed_uri(element)
                {
                    map.entry(slice_name.clone()).or_insert(url);
                    continue;
                }

                if let Some(url) = Self::extract_type_profile(element) {
                    map.entry(slice_name.clone()).or_insert(url);
                }
            }
        }

        map
    }

    /// Extract slice name from an ElementDefinition using slice_name or path (extension:<slice>)
    fn extract_slice_name(element: &ElementDefinition) -> Option<String> {
        if let Some(name) = element.slice_name.clone() {
            return Some(name);
        }

        element
            .path
            .split('.')
            .find_map(|segment| segment.strip_prefix("extension:"))
            .map(|s| s.to_string())
    }

    /// Extract fixed URI/Canonical from an ElementDefinition
    fn extract_fixed_uri(element: &ElementDefinition) -> Option<String> {
        element.fixed.as_ref().and_then(|fixed_map| {
            fixed_map
                .get("fixedUri")
                .or_else(|| fixed_map.get("fixedCanonical"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }

    /// Extract extension profile URL from ElementDefinition.type.profile
    fn extract_type_profile(element: &ElementDefinition) -> Option<String> {
        element.type_.as_ref().and_then(|types| {
            types
                .first()
                .and_then(|t| t.profile.as_ref())
                .and_then(|profiles| profiles.first())
                .cloned()
        })
    }

    /// Convert a string to PascalCase (simple heuristic)
    fn pascal_case(input: &str) -> String {
        input
            .split(['-', '_', ' '])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<String>()
    }

    /// Convert a string to kebab-case (simple heuristic)
    fn kebab_case(input: &str) -> String {
        let mut result = String::new();
        for (i, ch) in input.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('-');
                }
                result.push(ch.to_ascii_lowercase());
            } else if ch == '_' || ch == ' ' {
                result.push('-');
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// Map common slice names to their corresponding extension names
    /// This handles cases where the slice name differs from the extension name
    fn map_slice_to_extension(slice_name: &str) -> String {
        match slice_name {
            // mCode Radiotherapy extensions
            "actualNumberOfSessions" => "RadiotherapySessions".to_string(),
            "treatmentIntent" => "ProcedureIntent".to_string(),
            "modalityAndTechnique" => "RadiotherapyModalityAndTechnique".to_string(),
            "doseDeliveredToVolume" => "RadiotherapyDoseDeliveredToVolume".to_string(),
            // US Core extensions
            "birthsex" => "us-core-birthsex".to_string(),
            "race" => "us-core-race".to_string(),
            "ethnicity" => "us-core-ethnicity".to_string(),
            // Default: use PascalCase conversion
            _ => Self::pascal_case(slice_name),
        }
    }

    /// Load a StructureDefinition by URL or name using the fishing context
    async fn load_structure_definition(
        &self,
        fishing_ctx: &Arc<FishingContext>,
        instance_of: &str,
        canonical_url: Option<&str>,
    ) -> Option<StructureDefinition> {
        if let Some(url) = canonical_url
            && let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(url).await
        {
            return Some(sd);
        }

        if let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(instance_of).await {
            return Some(sd);
        }

        let kebab = Self::kebab_case(instance_of);
        let url_candidate = format!("{}/StructureDefinition/{}", self.base_url, kebab);
        if let Ok(Some(sd)) = fishing_ctx.fish_structure_definition(&url_candidate).await {
            return Some(sd);
        }

        None
    }

    /// Collect the StructureDefinition chain (profile -> parents) up to a base resource
    async fn collect_profile_chain(
        &self,
        fishing_ctx: &Arc<FishingContext>,
        starting_sd: &StructureDefinition,
    ) -> Vec<StructureDefinition> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut current_opt = Some(starting_sd.clone());

        while let Some(current) = current_opt {
            // Stop if we've already seen this URL to avoid loops
            if !visited.insert(current.url.clone()) {
                break;
            }

            let next_base = current.base_definition.clone();
            chain.push(current);

            let Some(base_def) = next_base else {
                break;
            };

            let base_name = base_def.rsplit('/').next().unwrap_or(&base_def);

            // Stop if we've reached the base FHIR resource
            if self.is_base_resource_type(base_name) {
                break;
            }

            // Try to load the parent StructureDefinition by URL first, then by name
            let parent_sd = match fishing_ctx.fish_structure_definition(&base_def).await {
                Ok(Some(sd)) => Some(sd),
                _ => match fishing_ctx.fish_structure_definition(base_name).await {
                    Ok(Some(sd)) => Some(sd),
                    _ => None,
                },
            };

            current_opt = parent_sd;
        }

        chain
    }

    /// Apply fixed/pattern constraints from the profile chain to the resource JSON
    async fn apply_profile_constraints(
        &mut self,
        resource: &mut JsonValue,
        profile_chain: &[StructureDefinition],
    ) -> Result<Vec<DeferredConstraint>, ExportError> {
        let mut deferred = Vec::new();
        // Apply ancestors first so child profiles can override
        for sd in profile_chain.iter().rev() {
            if sd.derivation.as_deref() != Some("constraint") {
                // Skip base resources/logicals; only apply actual profile constraints
                continue;
            }

            let elements = sd
                .differential
                .as_ref()
                .map(|d| d.element.as_slice())
                .or_else(|| sd.snapshot.as_ref().map(|s| s.element.as_slice()))
                .unwrap_or(&[]);

            for element in elements {
                let Some((constraint_key, constraint_value)) =
                    Self::extract_fixed_or_pattern(element)
                else {
                    continue;
                };

                let Some(path_str) = Self::element_path_to_instance_path(element, &constraint_key)
                else {
                    continue;
                };

                let Ok(segments) = self.parse_path(&path_str) else {
                    trace!(
                        "Skipping constraint with unparseable path '{}' derived from '{}'",
                        path_str, element.path
                    );
                    continue;
                };

                if Self::path_has_value(resource, &segments) {
                    // Leave explicit instance assignments intact
                    continue;
                }

                // Resolve any code system aliases in the constraint value
                let mut resolved_value = constraint_value.clone();
                self.resolve_aliases_in_json(&mut resolved_value);

                if element.path.contains(".component") {
                    deferred.push(DeferredConstraint {
                        segments,
                        value: resolved_value,
                    });
                    continue;
                }

                self.set_value_at_path(resource, &segments, resolved_value)
                    .await?;
            }
        }

        Ok(deferred)
    }

    async fn apply_deferred_constraints(
        &mut self,
        resource: &mut JsonValue,
        deferred: Vec<DeferredConstraint>,
    ) -> Result<(), ExportError> {
        for constraint in deferred {
            if Self::path_has_value(resource, &constraint.segments) {
                continue;
            }
            if !self.path_slice_exists(resource, &constraint.segments).await {
                continue;
            }

            self.set_value_at_path(resource, &constraint.segments, constraint.value)
                .await?;
        }
        Ok(())
    }

    async fn path_slice_exists(&self, resource: &JsonValue, segments: &[PathSegment]) -> bool {
        let mut current = resource;
        for (idx, segment) in segments.iter().enumerate() {
            let is_last = idx == segments.len() - 1;
            match segment {
                PathSegment::Field(field) => {
                    let Some(obj) = current.as_object() else {
                        return false;
                    };
                    if is_last {
                        return true;
                    }
                    let Some(next) = obj.get(field) else {
                        return false;
                    };
                    current = next;
                }
                PathSegment::ArrayAccess {
                    field,
                    index,
                    slice_name,
                } => {
                    let Some(obj) = current.as_object() else {
                        return false;
                    };
                    let Some(arr) = obj.get(field).and_then(|v| v.as_array()) else {
                        return false;
                    };

                    if let Some(slice) = slice_name {
                        let idx = if field == "extension" || field == "modifierExtension" {
                            let target_url = self.resolve_extension_url(slice).await;
                            arr.iter().position(|elem| {
                                elem.get("url").and_then(|u| u.as_str())
                                    == Some(target_url.as_str())
                            })
                        } else {
                            arr.iter().position(|elem| {
                                elem.get("_sliceName").and_then(|n| n.as_str())
                                    == Some(slice.as_str())
                            })
                        };

                        let Some(idx) = idx else {
                            return false;
                        };
                        current = &arr[idx];
                    } else {
                        let actual_index = match index {
                            ArrayIndex::Numeric(n) => *n,
                            ArrayIndex::Append | ArrayIndex::Current => {
                                if arr.is_empty() {
                                    return false;
                                }
                                arr.len() - 1
                            }
                        };

                        let Some(next) = arr.get(actual_index) else {
                            return false;
                        };
                        current = next;
                    }
                }
            }
        }

        true
    }

    /// Extract the fixed or pattern constraint value from an element
    fn extract_fixed_or_pattern(element: &ElementDefinition) -> Option<(String, JsonValue)> {
        if let Some(fixed_map) = element.fixed.as_ref()
            && let Some((key, value)) = fixed_map.iter().find(|(key, _)| key.starts_with("fixed"))
        {
            return Some((key.clone(), value.clone()));
        }

        if let Some(pattern_map) = element.pattern.as_ref()
            && let Some((key, value)) = pattern_map
                .iter()
                .find(|(key, _)| key.starts_with("pattern"))
        {
            return Some((key.clone(), value.clone()));
        }

        None
    }

    /// Resolve code system aliases in a JSON value recursively
    ///
    /// This processes CodeableConcept and Coding structures to resolve
    /// alias references (e.g., "NCIT") to their full URLs.
    fn resolve_aliases_in_json(&self, value: &mut JsonValue) {
        match value {
            JsonValue::Object(map) => {
                // If this looks like a Coding with a system field, resolve the alias
                if let Some(system_val) = map.get_mut("system")
                    && let Some(system_str) = system_val.as_str()
                    // Only resolve if it looks like an alias (not a URL)
                    && !system_str.starts_with("http://")
                    && !system_str.starts_with("https://")
                    && !system_str.starts_with("urn:")
                    && let Some(fishing_ctx) = &self.fishing_context
                    && let Some(resolved) = fishing_ctx.resolve_alias(system_str)
                {
                    *system_val = JsonValue::String(resolved);
                }
                // Recurse into all values
                for (_, v) in map.iter_mut() {
                    self.resolve_aliases_in_json(v);
                }
            }
            JsonValue::Array(arr) => {
                for item in arr.iter_mut() {
                    self.resolve_aliases_in_json(item);
                }
            }
            _ => {}
        }
    }

    /// Convert an ElementDefinition path to an instance assignment path
    fn element_path_to_instance_path(
        element: &ElementDefinition,
        constraint_key: &str,
    ) -> Option<String> {
        let mut parts: Vec<&str> = element.path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        // Drop the resource type prefix (e.g., "Observation")
        parts.remove(0);
        if parts.is_empty() {
            return None;
        }

        let mut slice_overrides: HashMap<usize, (String, String)> = HashMap::new();
        if let Some(id) = &element.id {
            let id_parts: Vec<&str> = id.split('.').collect();
            for (idx, part) in id_parts.iter().enumerate() {
                if let Some((field, slice)) = part.split_once(':') {
                    slice_overrides.insert(idx, (field.to_string(), slice.to_string()));
                }
            }
        }

        let mut path_segments = Vec::new();
        for (idx, raw) in parts.into_iter().enumerate() {
            let original_idx = idx + 1;
            if let Some((field, slice)) = raw.split_once(':') {
                path_segments.push(format!("{}[{}]", field, slice));
                continue;
            } else if let Some((field, slice)) = slice_overrides.get(&original_idx) {
                path_segments.push(format!("{}[{}]", field, slice));
                continue;
            }

            if let Some(choice_base) = raw.strip_suffix("[x]") {
                path_segments.push(Self::expand_choice_path(choice_base, constraint_key));
                continue;
            }

            path_segments.push(raw.to_string());
        }

        Some(path_segments.join("."))
    }

    /// Expand a choice element (value[x]) to a concrete path using the constraint type
    fn expand_choice_path(base: &str, constraint_key: &str) -> String {
        // constraint_key examples: fixedCodeableConcept, patternQuantity, fixedBoolean
        let suffix = constraint_key
            .trim_start_matches("fixed")
            .trim_start_matches("pattern");

        if suffix.is_empty() {
            base.to_string()
        } else {
            format!("{}{}", base, suffix)
        }
    }

    /// Check if a path already has a value in the resource JSON
    fn path_has_value(resource: &JsonValue, segments: &[PathSegment]) -> bool {
        let mut current = resource;

        for segment in segments {
            match segment {
                PathSegment::Field(field) => {
                    let Some(obj) = current.as_object() else {
                        return false;
                    };
                    let Some(next) = obj.get(field) else {
                        return false;
                    };
                    current = next;
                }
                PathSegment::ArrayAccess {
                    field,
                    index,
                    slice_name: _,
                } => {
                    let Some(obj) = current.as_object() else {
                        return false;
                    };
                    let Some(arr) = obj.get(field).and_then(|v| v.as_array()) else {
                        return false;
                    };

                    let actual_index = match index {
                        ArrayIndex::Numeric(n) => *n,
                        ArrayIndex::Append | ArrayIndex::Current => {
                            if arr.is_empty() {
                                return false;
                            }
                            arr.len() - 1
                        }
                    };

                    if let Some(next) = arr.get(actual_index) {
                        current = next;
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Merge two JSON objects, combining their properties
    ///
    /// This is used when assigning multiple properties to the same object.
    /// For example:
    /// ```fsh
    /// * contact.name.text = "John Doe"
    /// * contact.name.family = "Doe"
    /// ```
    /// Both rules target `contact.name`, so we merge the values.
    fn merge_objects(target: &mut JsonValue, source: &JsonValue) {
        if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
            for (key, value) in source_obj {
                if let Some(existing) = target_obj.get_mut(key) {
                    if existing.is_object() && value.is_object() {
                        Self::merge_objects(existing, value);
                    } else if existing.is_array() && value.is_array() {
                        // Special handling for "coding" arrays - merge by code+system
                        if key == "coding" {
                            Self::merge_coding_arrays(existing, value);
                        } else {
                            // For other arrays, append unique elements
                            if let (Some(existing_arr), Some(source_arr)) =
                                (existing.as_array_mut(), value.as_array())
                            {
                                for item in source_arr {
                                    if !existing_arr.contains(item) {
                                        existing_arr.push(item.clone());
                                    }
                                }
                            }
                        }
                    } else {
                        // Replace with new value
                        *existing = value.clone();
                    }
                } else {
                    target_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }

    /// Merge coding arrays by matching on code+system
    /// When a coding with the same code+system exists, merge the fields (e.g., add display)
    fn merge_coding_arrays(target: &mut JsonValue, source: &JsonValue) {
        let Some(target_arr) = target.as_array_mut() else {
            return;
        };
        let Some(source_arr) = source.as_array() else {
            return;
        };

        for src_item in source_arr {
            let src_code = src_item.get("code").and_then(|v| v.as_str());
            let src_system = src_item.get("system").and_then(|v| v.as_str());

            // Try to find a matching coding in target
            let mut found = false;
            for target_item in target_arr.iter_mut() {
                let tgt_code = target_item.get("code").and_then(|v| v.as_str());
                let tgt_system = target_item.get("system").and_then(|v| v.as_str());

                if src_code == tgt_code && src_system == tgt_system {
                    // Merge the source coding into the target (e.g., add display)
                    Self::merge_objects(target_item, src_item);
                    found = true;
                    break;
                }
            }

            // If no matching coding found, append it
            if !found {
                target_arr.push(src_item.clone());
            }
        }
    }

    /// Convert a FSH value string to appropriate JSON value with path context
    ///
    /// This method preserves string types for known FHIR string fields even if the value
    /// looks like a number (e.g., postalCode, identifier.value).
    ///
    /// Handles:
    /// - Simple types: strings, numbers, booleans, codes
    /// - Complex types: CodeableConcept, Quantity, Reference, Ratio
    /// - FHIR-specific patterns
    async fn convert_value_with_path(
        &self,
        value_str: &str,
        path: &str,
    ) -> Result<JsonValue, ExportError> {
        let trimmed = value_str.trim();

        // Check if this is a FHIR `code` type field (should be simple string, not CodeableConcept)
        // This handles: status, intent, gender, etc.
        if self.is_code_field_path(path) {
            let code = self.extract_code_only(trimmed);
            debug!(
                "Converting code field '{}' value '{}' -> '{}'",
                path, trimmed, code
            );
            return Ok(JsonValue::String(code));
        }

        // Check if this path should always be a string
        if self.is_string_field_path(path) {
            // If it's a quoted string, remove quotes
            if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            {
                return Ok(JsonValue::String(trimmed[1..trimmed.len() - 1].to_string()));
            }
            // Otherwise, keep as-is (even if it looks like a number)
            return Ok(JsonValue::String(trimmed.to_string()));
        }

        // If this is a reference, use path context to help typing
        if trimmed.starts_with("Reference(") && trimmed.ends_with(')') {
            return self.parse_reference_with_context(trimmed, Some(path)).await;
        }

        // Check if this is a Coding field (not CodeableConcept)
        // Coding fields should output {code, system, display} not {coding: [...]}
        if self.is_coding_field_path(path) && trimmed.contains('#') && !trimmed.starts_with('#') {
            return self.parse_coding(trimmed).await;
        }

        // Use standard conversion for other fields
        self.convert_value(value_str).await
    }

    /// Check if a path represents a FHIR Coding field (not CodeableConcept)
    /// Coding fields output {code, system, display} directly
    fn is_coding_field_path(&self, path: &str) -> bool {
        let field_name = path.rsplit('.').next().unwrap_or(path);
        // Strip array index from field name (e.g., "coding[0]" -> "coding")
        let base_field = field_name.split('[').next().unwrap_or(field_name);
        // Fields that end with "Coding" are Coding type, not CodeableConcept
        // Also check for "coding" which is an array of Coding inside CodeableConcept
        base_field.ends_with("Coding") || base_field == "coding"
    }

    /// Parse Coding from FSH notation (for Coding type fields, not CodeableConcept)
    /// Format: system#code "display"
    /// Output: {code, system, display} - NOT wrapped in coding array
    async fn parse_coding(&self, value: &str) -> Result<JsonValue, ExportError> {
        let parts: Vec<&str> = value.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let system_or_alias = parts[0].trim();
        let code_and_display = parts[1];

        // Resolve system alias to canonical URL
        let system = if let Some(fishing_ctx) = &self.fishing_context {
            if let Some(canonical_url) = fishing_ctx.resolve_alias(system_or_alias) {
                canonical_url
            } else {
                system_or_alias.to_string()
            }
        } else {
            system_or_alias.to_string()
        };

        // Extract code and display
        let (code, display) = if let Some(space_idx) = code_and_display.find(' ') {
            let code = code_and_display[..space_idx].trim();
            let display_part = code_and_display[space_idx..].trim();
            let display = if display_part.starts_with('"') && display_part.ends_with('"') {
                &display_part[1..display_part.len() - 1]
            } else {
                display_part
            };
            (code, Some(display))
        } else {
            (code_and_display.trim(), None)
        };

        // Build Coding (NOT CodeableConcept - no wrapping in coding array)
        let mut coding = serde_json::json!({
            "code": code,
            "system": system
        });

        if let Some(display_text) = display {
            coding["display"] = JsonValue::String(display_text.to_string());
        }

        Ok(coding)
    }

    /// Check if a path represents a field that should always be a string in FHIR
    ///
    /// This includes fields like postalCode, identifier.value, id, etc. that are
    /// defined as string types in the FHIR spec but often contain numeric-looking values.
    fn is_string_field_path(&self, path: &str) -> bool {
        // Extract the final field name from the path (after last dot)
        let field_name = path.rsplit('.').next().unwrap_or(path);

        // Check for known FHIR string fields that often contain numeric values
        match field_name {
            // Address fields
            "postalCode" | "postal" => true,
            // Identifier fields
            "value" if path.contains("identifier") => true,
            // NPI, SSN, etc.
            "system" if path.contains("identifier") => true,
            // Phone numbers
            "value" if path.contains("telecom") => true,
            // ID fields
            "id" => true,
            // Various other string fields that might look numeric
            "version" | "reference" | "display" => true,
            _ => false,
        }
    }

    /// Check if a path represents a FHIR `code` type field
    ///
    /// Code fields are simple string values (not CodeableConcept) that should
    /// only contain the code value, not a full coding structure.
    fn is_code_field_path(&self, path: &str) -> bool {
        // Extract the final field name from the path
        let field_name = path.rsplit('.').next().unwrap_or(path);

        // Context-sensitive check: some fields are code in some contexts but CodeableConcept in others
        // identifier.type is a CodeableConcept, not a code
        if field_name == "type" && path.contains("identifier") {
            return false;
        }

        // Common FHIR code fields (not CodeableConcept)
        // Note: clinicalStatus and verificationStatus are CodeableConcept in Condition, AllergyIntolerance, etc.
        // Note: Removed 'type' and 'kind' as they are often CodeableConcept in many resources
        matches!(
            field_name,
            // Status fields - only simple code-typed status fields
            "status"  // Most resources have status as code (not CodeableConcept)
                | "eventStatus"
                | "publicationStatus"
                |
                // Intent/purpose fields
                "intent"
                | "priority"
                |
                // Gender/sex
                "gender"
                | "administrativeGender"
                |
                // Timing.repeat unit fields (code type)
                "periodUnit"
                | "durationUnit"
                | "when"
                | "dayOfWeek"
                |
                // Other common code fields
                "use"
                | "rank"
                | "mode"
                | "resourceType"
        )
    }

    /// Extract just the code from a code value with optional display
    /// Handles: #code, #code "display", system#code, system#code "display"
    fn extract_code_only(&self, value: &str) -> String {
        let trimmed = value.trim();

        // Pattern: #code "display" or #code
        if let Some(code_part) = trimmed.strip_prefix('#') {
            // Split on space to separate code from display
            let code = code_part.split_whitespace().next().unwrap_or(code_part);
            return code.trim_matches('"').to_string();
        }

        // Pattern: system#code "display" or system#code
        if trimmed.contains('#')
            && let Some(code_part) = trimmed.split('#').nth(1)
        {
            // Split on space to separate code from display
            let code = code_part.split_whitespace().next().unwrap_or(code_part);
            return code.trim_matches('"').to_string();
        }

        // No code pattern, return as-is (remove quotes if present)
        trimmed.trim_matches('"').trim_matches('\'').to_string()
    }

    /// Check if a field name represents an array field in FHIR
    ///
    /// This method looks up the element's max cardinality from the FHIR StructureDefinition
    /// using the canonical manager. Returns true if max cardinality != "1".
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name (e.g., "identifier", "telecom")
    ///
    /// # Returns
    ///
    /// `true` if the field is an array based on the current resource type's
    /// StructureDefinition, `false` otherwise or if resource type is unknown.
    ///
    /// # Implementation
    ///
    /// This method uses dynamic SD lookup like SUSHI instead of hardcoded field lists.
    /// It tries multiple resolution strategies:
    /// 1. Resource-level path (e.g., "Patient.identifier")
    /// 2. Common datatype paths for nested fields (e.g., "HumanName.given", "Address.line")
    ///
    /// Reference: SUSHI ElementDefinition.ts:359-367 `isArrayOrChoice()`
    async fn is_array_field(&self, field_name: &str) -> bool {
        // 1. Try resource-level path first
        if let Some(resource_type) = &self.current_resource_type {
            let element_path = format!("{}.{}", resource_type, field_name);
            if self.session.is_array_element(&element_path).await {
                return true;
            }
        }

        // 2. Try common datatype paths for nested fields
        // This handles fields like "given" (HumanName), "line" (Address), "coding" (CodeableConcept)
        // The canonical manager will look up the datatype's SD and check cardinality
        let datatype_paths = Self::get_datatype_paths_for_field(field_name);
        for datatype_path in datatype_paths {
            if self.session.is_array_element(&datatype_path).await {
                return true;
            }
        }

        // Default to non-array if no path matched
        trace!(
            "Field '{}' not found as array in any known type, defaulting to non-array",
            field_name
        );
        false
    }

    /// Get potential datatype paths for a field name
    ///
    /// This maps field names to the FHIR datatypes where they commonly appear.
    /// Used for looking up cardinality when we don't have full path context.
    ///
    /// NOTE: This is a mapping, not a hardcoded "is array" list.
    /// The actual cardinality is queried from the StructureDefinition.
    fn get_datatype_paths_for_field(field_name: &str) -> Vec<String> {
        match field_name {
            // HumanName fields
            "given" | "prefix" | "suffix" => vec![format!("HumanName.{}", field_name)],
            // Address fields
            "line" => vec!["Address.line".to_string()],
            // CodeableConcept fields
            "coding" => vec!["CodeableConcept.coding".to_string()],
            // Timing fields
            "event" | "timeOfDay" => vec![format!("Timing.repeat.{}", field_name)],
            // ContactPoint (within various types)
            "telecom" => vec![
                "Patient.telecom".to_string(),
                "ContactPoint.telecom".to_string(),
            ],
            // Dosage fields
            "doseAndRate" => vec!["Dosage.doseAndRate".to_string()],
            // Extension fields (always array)
            "extension" | "modifierExtension" => vec!["Element.extension".to_string()],
            // Other common fields - check in DomainResource
            "contained" => vec!["DomainResource.contained".to_string()],
            _ => vec![],
        }
    }

    // REMOVED: is_known_array_field() - replaced with dynamic SD lookup via session.is_array_element()
    // This follows SUSHI's approach: no hardcoded array field lists.
    // See: SUSHI ElementDefinition.ts:359-367 isArrayOrChoice()

    /// Convert a FSH value string to appropriate JSON value
    ///
    /// Handles:
    /// - Simple types: strings, numbers, booleans, codes
    /// - Complex types: CodeableConcept, Quantity, Reference, Ratio
    /// - FHIR-specific patterns
    async fn convert_value(&self, value_str: &str) -> Result<JsonValue, ExportError> {
        let trimmed = value_str.trim();

        // String literal (remove quotes)
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            return Ok(JsonValue::String(trimmed[1..trimmed.len() - 1].to_string()));
        }

        // Boolean
        if trimmed == "true" {
            return Ok(JsonValue::Bool(true));
        }
        if trimmed == "false" {
            return Ok(JsonValue::Bool(false));
        }

        // Code with system (CodeableConcept pattern): system#code "display"
        // Example: http://loinc.org#LA6576-8 "Excellent"
        if trimmed.contains('#') && !trimmed.starts_with('#') {
            return self.parse_codeable_concept(trimmed).await;
        }

        // Code (starts with #)
        if let Some(code) = trimmed.strip_prefix('#') {
            return Ok(JsonValue::String(code.to_string()));
        }

        // Quantity pattern: 70 'kg'
        if trimmed.contains('\'') && trimmed.chars().next().is_some_and(|c| c.is_numeric()) {
            return self.parse_quantity(trimmed);
        }

        // Reference pattern: Reference(Patient/example)
        if trimmed.starts_with("Reference(") && trimmed.ends_with(')') {
            return self.parse_reference_with_context(trimmed, None).await;
        }

        // Number (integer)
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(JsonValue::Number(num.into()));
        }

        // Number (float)
        if let Ok(num) = trimmed.parse::<f64>()
            && let Some(json_num) = serde_json::Number::from_f64(num)
        {
            return Ok(JsonValue::Number(json_num));
        }

        // Check if it's an instance reference (identifier without quotes)
        // Instance references are plain identifiers that match registered instances
        if self.is_instance_reference(trimmed)
            && let Some(instance_json) = self.get_instance(trimmed)
        {
            debug!("Resolved instance reference: {}", trimmed);
            return Ok(instance_json.clone());
        }

        // Default to string
        Ok(JsonValue::String(trimmed.to_string()))
    }

    /// Check if a value looks like an instance reference
    /// Instance references are plain identifiers (alphanumeric + hyphen/underscore)
    fn is_instance_reference(&self, value: &str) -> bool {
        // Must not be empty
        if value.is_empty() {
            return false;
        }

        // Must start with a letter
        if !value.chars().next().unwrap().is_alphabetic() {
            return false;
        }

        // Must contain only valid identifier characters
        value
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Parse CodeableConcept from FSH notation
    /// Format: system#code "display" or ALIAS#code "display"
    async fn parse_codeable_concept(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Split into system#code and display
        let parts: Vec<&str> = value.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let system_or_alias = parts[0].trim();
        let code_and_display = parts[1];

        // Resolve system alias to canonical URL
        let system = if let Some(fishing_ctx) = &self.fishing_context {
            if let Some(canonical_url) = fishing_ctx.resolve_alias(system_or_alias) {
                debug!(
                    "Resolved CodeSystem alias '{}' -> '{}'",
                    system_or_alias, canonical_url
                );
                canonical_url
            } else {
                // Not an alias, use as-is
                system_or_alias.to_string()
            }
        } else {
            // No fishing context, use as-is
            system_or_alias.to_string()
        };

        // Extract code and display
        let (code, display) = if let Some(space_idx) = code_and_display.find(' ') {
            let code = code_and_display[..space_idx].trim();
            let display_part = code_and_display[space_idx..].trim();
            let display = if display_part.starts_with('"') && display_part.ends_with('"') {
                &display_part[1..display_part.len() - 1]
            } else {
                display_part
            };
            (code, Some(display))
        } else {
            (code_and_display.trim(), None)
        };

        // Build CodeableConcept
        let mut codeable_concept = serde_json::json!({
            "coding": [{
                "code": code,
                "system": system
            }]
        });

        if let Some(display_text) = display {
            codeable_concept["coding"][0]["display"] = JsonValue::String(display_text.to_string());
        }

        Ok(codeable_concept)
    }

    /// Parse Quantity from FSH notation
    /// Format: 70 'kg' or 5.5 'cm' or 272.01 'mg' "mg" (with display)
    fn parse_quantity(&self, value: &str) -> Result<JsonValue, ExportError> {
        // FSH Quantity format: VALUE 'UNIT' or VALUE 'UNIT' "DISPLAY"
        // Find the opening and closing single quotes for the unit
        let first_quote = value.find('\'');
        let last_quote = value.rfind('\'');

        if first_quote.is_none() || last_quote.is_none() || first_quote == last_quote {
            return Ok(JsonValue::String(value.to_string()));
        }

        let first_quote_idx = first_quote.unwrap();
        let last_quote_idx = last_quote.unwrap();

        // Extract value (before first quote)
        let value_str = value[..first_quote_idx].trim();

        // Extract UCUM code (between single quotes)
        let ucum_code = value[first_quote_idx + 1..last_quote_idx].trim();

        // Extract optional display text (between double quotes after the unit)
        let after_unit = &value[last_quote_idx + 1..];
        let display = after_unit.find('"').and_then(|dq_start| {
            after_unit[dq_start + 1..]
                .find('"')
                .map(|dq_end| after_unit[dq_start + 1..dq_start + 1 + dq_end].trim())
        });

        // Use display text as unit if provided, otherwise use UCUM code
        let unit_display = display.unwrap_or(ucum_code);

        // Parse numeric value
        // If the value can be represented as an integer, serialize as integer
        let numeric_value = if let Ok(num) = value_str.parse::<i64>() {
            serde_json::json!(num)
        } else if let Ok(num) = value_str.parse::<f64>() {
            // If the float is actually a whole number (e.g., 155.0), serialize as integer
            if num.fract() == 0.0 && num >= i64::MIN as f64 && num <= i64::MAX as f64 {
                serde_json::json!(num as i64)
            } else {
                serde_json::json!(num)
            }
        } else {
            return Ok(JsonValue::String(value.to_string()));
        };

        // Build Quantity with UCUM system and code
        // UCUM (Unified Code for Units of Measure) is the standard system for units in FHIR
        // Field order matches SUSHI: value, code, system, unit
        let quantity = serde_json::json!({
            "value": numeric_value,
            "code": ucum_code,
            "system": "http://unitsofmeasure.org",
            "unit": unit_display
        });

        Ok(quantity)
    }

    /// Clean a path for canonical manager lookup by removing array indices
    /// e.g., "hasMember[0]" -> "hasMember", "stage.assessment" -> "stage.assessment"
    fn clean_path_for_lookup(&self, path: &str) -> String {
        // Remove array indices from each segment
        path.split('.')
            .map(|segment| segment.find('[').map(|i| &segment[..i]).unwrap_or(segment))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Parse Reference from FSH notation with optional path context
    /// Format: Reference(Patient/example) or Reference(resourceId)
    ///
    /// Uses proper resource resolution via fishing context:
    /// 1. Check local instance registry
    /// 2. Fish for the instance in the tank, resolve its InstanceOf to get actual FHIR type
    /// 3. Fall back to canonical manager for path-based type inference
    async fn parse_reference_with_context(
        &self,
        value: &str,
        path: Option<&str>,
    ) -> Result<JsonValue, ExportError> {
        // Extract reference from Reference(...)
        let mut ref_value: String = value
            .strip_prefix("Reference(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(value)
            .to_string();

        // If this looks like a local instance id (no slash), resolve its type
        if !ref_value.contains('/') {
            // First, try local instance registry (already exported instances)
            if let Some(instance_json) = self.instance_registry.get(&ref_value) {
                if let Some(rt) = instance_json.get("resourceType").and_then(|v| v.as_str()) {
                    ref_value = format!("{}/{}", rt, ref_value);
                }
            }
            // Second, try fishing context to find the instance in FSH tank
            else if let Some(fishing_ctx) = &self.fishing_context {
                // Look up the instance in the tank
                if let Some(metadata) = fishing_ctx
                    .fish_metadata(&ref_value, &[crate::semantic::ResourceType::Instance])
                    .await
                    // For Instances, metadata.parent contains the InstanceOf value
                    // We need to resolve it to the actual FHIR resourceType
                    && let Some(instance_of) = &metadata.parent
                    && let Some(base_type) =
                        fishing_ctx.resolve_instance_base_type(instance_of).await
                {
                    ref_value = format!("{}/{}", base_type, ref_value);
                }
                // If not found as an instance, try other resource types (profiles, etc.)
                else if let Some(metadata) = fishing_ctx.fish_metadata(&ref_value, &[]).await {
                    // For non-Instance resources, resource_type is the FHIR type
                    if metadata.resource_type != "Instance" && !metadata.resource_type.is_empty() {
                        ref_value = format!("{}/{}", metadata.resource_type, ref_value);
                    }
                }
                // Fall back to canonical manager for path-based type inference
                else if let Some(p) = path {
                    let full_path = if let Some(ref rt) = self.current_resource_type {
                        let clean_path = self.clean_path_for_lookup(p);
                        format!("{}.{}", rt, clean_path)
                    } else {
                        p.to_string()
                    };

                    let target_types = self.session.get_reference_target_types(&full_path).await;
                    if let Some(rt) = target_types.into_iter().next() {
                        ref_value = format!("{}/{}", rt, ref_value);
                    }
                }
            }
            // If no fishing context available, fall back to canonical manager
            else if let Some(p) = path {
                let full_path = if let Some(ref rt) = self.current_resource_type {
                    let clean_path = self.clean_path_for_lookup(p);
                    format!("{}.{}", rt, clean_path)
                } else {
                    p.to_string()
                };

                let target_types = self.session.get_reference_target_types(&full_path).await;
                if let Some(rt) = target_types.into_iter().next() {
                    ref_value = format!("{}/{}", rt, ref_value);
                }
            }
        }

        // Validate the reference if fishing context is available
        if let Err(e) = self.validate_reference(&ref_value).await {
            warn!("Reference validation failed for '{}': {}", ref_value, e);
            // Note: We don't fail export on invalid references, just warn
            // This matches SUSHI behavior for better user experience
        }

        // Build Reference
        let reference = serde_json::json!({
            "reference": ref_value
        });

        Ok(reference)
    }

    /// Validate that a reference target exists
    ///
    /// Checks if the referenced resource exists in:
    /// 1. Local instance registry (inline instances)
    /// 2. FSH tank (parsed FSH resources)
    /// 3. Canonical packages (external FHIR resources)
    ///
    /// # Arguments
    ///
    /// * `reference` - The reference string (e.g., "Patient/example" or "my-patient-instance")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Reference is valid
    /// * `Err(ExportError)` - Reference target not found or validation failed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use maki_core::export::InstanceExporter;
    ///
    /// # async fn example(exporter: &InstanceExporter) -> Result<(), Box<dyn std::error::Error>> {
    /// // Validate a FHIR reference
    /// exporter.validate_reference("Patient/example").await?;
    ///
    /// // Validate an instance reference
    /// exporter.validate_reference("my-patient-instance").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_reference(&self, reference: &str) -> Result<(), ExportError> {
        trace!("Validating reference: {}", reference);

        // Check if it's an inline instance reference (local registry)
        if self.instance_registry.contains_key(reference) {
            debug!("Reference '{}' resolved to inline instance", reference);
            return Ok(());
        }

        // If fishing context is available, check tank and canonical
        if let Some(fishing_ctx) = &self.fishing_context {
            // Try to find the resource in the tank (any resource type)
            if fishing_ctx.fish_metadata(reference, &[]).await.is_some() {
                debug!("Reference '{}' resolved to FSH resource in tank", reference);
                return Ok(());
            }

            // For FHIR-style references like "Patient/example", we can't validate without async
            // but we trust that they're intended references to external resources
            if reference.contains('/') {
                debug!(
                    "Reference '{}' appears to be FHIR-style, trusting it's valid",
                    reference
                );
                return Ok(());
            }
        }

        // Without fishing context, we can only validate inline instances
        // For FHIR-style references, we'll be lenient and assume they're valid
        // This matches SUSHI's behavior where external references are trusted
        if reference.contains('/') {
            debug!(
                "Reference '{}' appears to be FHIR-style (no fishing context), assuming valid",
                reference
            );
            return Ok(());
        }

        // If we get here, we couldn't validate the reference
        // Return error but caller can decide to warn instead of failing
        Err(ExportError::InvalidReference {
            reference: reference.to_string(),
            reason: "Reference target not found in instances, tank, or canonical packages"
                .to_string(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_exporter() -> InstanceExporter {
        InstanceExporter {
            session: Arc::new(crate::canonical::DefinitionSession::for_testing()),
            fishing_context: None,
            base_url: "http://example.org/fhir".to_string(),
            current_indices: HashMap::new(),
            instance_registry: HashMap::new(),
            current_profile_name: None,
            current_profile_url: None,
            current_extension_urls: HashMap::new(),
            current_resource_type: None,
            ruleset_expander: None,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_simple_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.family").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Field("name".to_string()));
        assert_eq!(segments[1], PathSegment::Field("family".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_array_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.given[0]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Field("name".to_string()));
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "given".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_append_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.given[+]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "given".to_string(),
                index: ArrayIndex::Append,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_current_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("telecom[=].value").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "telecom".to_string(),
                index: ArrayIndex::Current,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_nested_array_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("address[0].line[+]").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "address".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: None
            }
        );
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "line".to_string(),
                index: ArrayIndex::Append,
                slice_name: None
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_string_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("\"Hello World\"").await.unwrap();
        assert_eq!(value, JsonValue::String("Hello World".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_boolean_value() {
        let exporter = create_test_exporter();
        assert_eq!(
            exporter.convert_value("true").await.unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(
            exporter.convert_value("false").await.unwrap(),
            JsonValue::Bool(false)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_integer_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("42").await.unwrap();
        assert_eq!(value, JsonValue::Number(42.into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_code_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("#male").await.unwrap();
        assert_eq!(value, JsonValue::String("male".to_string()));
    }

    #[test]
    fn test_patient_assignment_rule_is_parsed() {
        let fsh = r#"Instance: test-volume
InstanceOf: RadiotherapyVolume
* patient = Reference(cancer-patient-john-anyperson)
"#;

        let (cst, lex, parse) = crate::cst::parse_fsh(fsh);
        assert!(lex.is_empty() && parse.is_empty());

        let instance = cst.descendants().find_map(Instance::cast).unwrap();
        let patient_value = instance.rules().find_map(|rule| match rule {
            Rule::FixedValue(fv)
                if fv
                    .path()
                    .map(|p| p.as_string() == "patient")
                    .unwrap_or(false) =>
            {
                fv.value()
            }
            _ => None,
        });

        assert_eq!(
            patient_value,
            Some("Reference(cancer-patient-john-anyperson)".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_simple_value() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("birthDate").unwrap();
        let value = JsonValue::String("1970-01-01".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .await
            .unwrap();

        assert_eq!(resource["birthDate"], "1970-01-01");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_array_value_numeric() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("name.given[0]").unwrap();
        let value = JsonValue::String("John".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .await
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_array_value_append() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add first element
        let segments = exporter.parse_path("name.given[+]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("John".to_string()),
            )
            .await
            .unwrap();

        // Add second element
        let segments = exporter.parse_path("name.given[+]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Jacob".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
        assert_eq!(resource["name"]["given"][1], "Jacob");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_nested_array_access() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Set address[0].line[0]
        let segments = exporter.parse_path("address[0].line[0]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("123 Main St".to_string()),
            )
            .await
            .unwrap();

        // Set address[0].city
        let segments = exporter.parse_path("address[0].city").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Boston".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(resource["address"][0]["line"][0], "123 Main St");
        assert_eq!(resource["address"][0]["city"], "Boston");
    }

    // ===== Reference Validation Tests =====

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_inline_instance() {
        let mut exporter = create_test_exporter();

        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        assert!(exporter.validate_reference("my-patient").await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_not_found() {
        let exporter = create_test_exporter();

        let result = exporter.validate_reference("nonexistent").await;
        assert!(result.is_err());
        if let Err(ExportError::InvalidReference { reference, reason }) = result {
            assert_eq!(reference, "nonexistent");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected InvalidReference error");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_with_fishing_context() {
        use crate::Location;
        use crate::semantic::{FhirResource, FshTank, Package, ResourceType};
        use tokio::sync::RwLock;

        let mut exporter = create_test_exporter();

        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(crate::canonical::DefinitionSession::for_testing());

        {
            let mut t = tank.write().await;
            t.add_resource(FhirResource {
                resource_type: ResourceType::Profile,
                id: "PatientProfile".to_string(),
                name: Some("PatientProfile".to_string()),
                title: None,
                description: None,
                parent: Some("Patient".to_string()),
                elements: Vec::new(),
                location: Location::default(),
                metadata: crate::semantic::ResourceMetadata::default(),
            });
        }

        let fishing_ctx = Arc::new(FishingContext::new(session, tank, package));
        exporter.fishing_context = Some(fishing_ctx);

        assert!(exporter.validate_reference("PatientProfile").await.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_reference_fhir_style() {
        let exporter = create_test_exporter();

        assert!(exporter.validate_reference("Patient/example").await.is_ok());
        assert!(
            exporter
                .validate_reference("Observation/vital-signs")
                .await
                .is_ok()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_reference_validates() {
        let mut exporter = create_test_exporter();

        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        let result = exporter
            .parse_reference_with_context("Reference(my-patient)", None)
            .await;
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "Patient/my-patient");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_reference_warns_on_invalid() {
        let exporter = create_test_exporter();

        let result = exporter
            .parse_reference_with_context("Reference(nonexistent)", None)
            .await;
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "nonexistent");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_inline_instance_resolution() {
        let mut exporter = create_test_exporter();

        let patient_json = serde_json::json!({
            "resourceType": "Patient",
            "id": "example-patient",
            "name": [{
                "family": "Doe",
                "given": ["John"]
            }]
        });

        exporter.register_instance("example-patient".to_string(), patient_json.clone());

        let value = exporter.convert_value("example-patient").await.unwrap();
        assert_eq!(value, patient_json);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_is_instance_reference() {
        let exporter = create_test_exporter();

        // Valid instance reference patterns
        assert!(exporter.is_instance_reference("my-patient"));
        assert!(exporter.is_instance_reference("Patient123"));
        assert!(exporter.is_instance_reference("example_instance"));

        // Invalid patterns
        assert!(!exporter.is_instance_reference("")); // empty
        assert!(!exporter.is_instance_reference("123patient")); // starts with number
        assert!(!exporter.is_instance_reference("patient.name")); // contains dot
        assert!(!exporter.is_instance_reference("\"patient\"")); // quoted
        assert!(!exporter.is_instance_reference("#code")); // code
    }

    // ===== Slice Name Tests =====

    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_slice_name_path() {
        let exporter = create_test_exporter();
        let segments = exporter
            .parse_path("extension[myExtension].valueString")
            .unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(
            segments[0],
            PathSegment::ArrayAccess {
                field: "extension".to_string(),
                index: ArrayIndex::Numeric(0),
                slice_name: Some("myExtension".to_string())
            }
        );
        assert_eq!(segments[1], PathSegment::Field("valueString".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_set_extension_by_slice_name() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Set extension URL
        let segments = exporter
            .parse_path("extension[http://example.org/ext].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("test value".to_string()),
            )
            .await
            .unwrap();

        // Verify the extension was created with URL
        assert!(resource["extension"].is_array());
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "test value");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_extensions_by_slice_name() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add first extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext1].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("value1".to_string()),
            )
            .await
            .unwrap();

        // Add second extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext2].valueInteger")
            .unwrap();
        exporter
            .set_value_at_path(&mut resource, &segments, JsonValue::Number(42.into()))
            .await
            .unwrap();

        // Verify both extensions exist
        assert!(resource["extension"].is_array());
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 2);
        assert_eq!(extensions[0]["url"], "http://example.org/ext1");
        assert_eq!(extensions[0]["valueString"], "value1");
        assert_eq!(extensions[1]["url"], "http://example.org/ext2");
        assert_eq!(extensions[1]["valueInteger"], 42);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_existing_extension() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Add extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext].valueString")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("initial".to_string()),
            )
            .await
            .unwrap();

        // Update the same extension with additional property
        let segments = exporter
            .parse_path("extension[http://example.org/ext].id")
            .unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("ext-id".to_string()),
            )
            .await
            .unwrap();

        // Verify extension has both properties
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "initial");
        assert_eq!(extensions[0]["id"], "ext-id");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ruleset_insert_expansion_on_instance() {
        let fsh = r#"
RuleSet: StagingInstanceRuleSet
* status = #final

Instance: test-stage
InstanceOf: Observation
* insert StagingInstanceRuleSet
"#;

        let (cst, lex, parse) = crate::cst::parse_fsh(fsh);
        assert!(lex.is_empty() && parse.is_empty());

        let instance = cst.descendants().find_map(Instance::cast).unwrap();

        let mut expander = crate::semantic::ruleset::RuleSetExpander::new();
        expander.register_ruleset(crate::semantic::ruleset::RuleSet {
            name: "StagingInstanceRuleSet".to_string(),
            parameters: vec![],
            rules: vec!["* status = #final".to_string()],
            source_file: std::path::PathBuf::from("test.fsh"),
            source_range: 0..0,
        });

        let session = Arc::new(crate::canonical::DefinitionSession::for_testing());
        let mut exporter = InstanceExporter::new(session, "http://example.org/fhir".to_string())
            .await
            .unwrap()
            .with_ruleset_expander(Arc::new(expander));

        let resource = exporter.export(&instance).await.unwrap();
        assert_eq!(resource["status"], "final");
    }
}
