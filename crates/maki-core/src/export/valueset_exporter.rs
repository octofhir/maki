//! ValueSet Exporter
//!
//! Exports FSH ValueSet definitions to FHIR ValueSet resources (JSON).
//!
//! # Overview
//!
//! ValueSets define allowed sets of codes from code systems. This module handles:
//! - Converting FSH ValueSet metadata to FHIR ValueSet resources
//! - Processing include/exclude rules
//! - Handling filters (is-a, regex, exists, etc.)
//! - Building compose.include and compose.exclude structures
//!
//! # Status
//!
//! **Phase 1 (Current)**: Basic metadata export with FixedValueRule support
//! - Exports id, url, title, description, status
//! - Creates skeleton ValueSet resources
//! - Foundation for component rules
//!
//! **Phase 2 (Future)**: Full component rule support
//! - Parse and export include/exclude components
//! - Handle filters (is-a, descendant-of, regex, exists)
//! - Support valueset references
//! - Note: Requires parser enhancements for ValueSet component syntax
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::ValueSetExporter;
//! use maki_core::cst::ast::ValueSet;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let session: Arc<maki_core::canonical::DefinitionSession> = todo!();
//! // Parse FSH valueset
//! let valueset: ValueSet = todo!();
//!
//! // Create exporter
//! let exporter = ValueSetExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//!     None,
//! ).await?;
//!
//! // Export to FHIR JSON
//! let resource = exporter.export(&valueset).await?;
//!
//! // Serialize
//! let json = serde_json::to_string_pretty(&resource)?;
//! println!("{}", json);
//! # Ok(())
//! # }
//! ```

use super::{
    ExportError, ValueSetCompose, ValueSetConcept, ValueSetConceptDesignation,
    ValueSetConceptProperty, ValueSetFilter, ValueSetInclude, ValueSetResource,
};
use crate::canonical::DefinitionSession;
use crate::cst::ast::{
    AstNode, Document, FixedValueRule, PathRule, Rule, ValueSet, VsComponent, VsConceptComponent,
    VsFilterComponent,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

// Type alias for complex HashMap used in multiple functions
type SystemComponentMap =
    HashMap<String, (Vec<ValueSetConcept>, Vec<ValueSetFilter>, Option<String>)>;

// Common FSH code system aliases mapped to canonical URLs (mirrors profile exporter)
const CODE_SYSTEM_ALIASES: &[(&str, &str)] = &[
    ("LNC", "http://loinc.org"),
    ("LOINC", "http://loinc.org"),
    ("SCT", "http://snomed.info/sct"),
    ("SNOMED", "http://snomed.info/sct"),
    ("NCIT", "http://ncicb.nci.nih.gov/xml/owl/EVS/Thesaurus.owl"),
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

fn resolve_code_system_alias(alias: &str) -> Option<&'static str> {
    CODE_SYSTEM_ALIASES
        .iter()
        .find_map(|(name, url)| (*name == alias).then(|| *url))
}

// Standard copyright notices for common code systems (SUSHI parity)
const CODE_SYSTEM_COPYRIGHTS: &[(&str, &str)] = &[
    (
        "http://snomed.info/sct",
        "This value set includes content from SNOMED CT, which is copyright © 2002+ International Health Terminology Standards Development Organisation (IHTSDO), and distributed by agreement between IHTSDO and HL7. Implementer use of SNOMED CT is not covered by this agreement",
    ),
    (
        "http://loinc.org",
        "This material contains content from LOINC (http://loinc.org). LOINC is copyright © 1995-2020, Regenstrief Institute, Inc. and the Logical Observation Identifiers Names and Codes (LOINC) Committee and is available at no cost under the license at http://loinc.org/license. LOINC® is a registered United States trademark of Regenstrief Institute, Inc",
    ),
    (
        "http://www.nlm.nih.gov/research/umls/rxnorm",
        "This material contains content from the National Library of Medicine's RxNorm (https://www.nlm.nih.gov/research/umls/rxnorm). RxNorm is a registered trademark of the National Library of Medicine",
    ),
    (
        "http://www.ama-assn.org/go/cpt",
        "Current Procedural Terminology (CPT) is copyright 2020 American Medical Association. All rights reserved",
    ),
];

/// Generate copyright text based on code systems used in a ValueSet compose
fn generate_copyright_from_compose(compose: &ValueSetCompose) -> Option<String> {
    let mut copyrights = Vec::new();
    let mut seen_systems = std::collections::HashSet::new();

    // Check include systems
    if let Some(includes) = &compose.include {
        for include in includes {
            if let Some(system) = &include.system {
                if seen_systems.insert(system.clone()) {
                    if let Some(copyright) = get_copyright_for_system(system) {
                        copyrights.push(copyright);
                    }
                }
            }
        }
    }

    // Check exclude systems
    if let Some(excludes) = &compose.exclude {
        for exclude in excludes {
            if let Some(system) = &exclude.system {
                if seen_systems.insert(system.clone()) {
                    if let Some(copyright) = get_copyright_for_system(system) {
                        copyrights.push(copyright);
                    }
                }
            }
        }
    }

    if copyrights.is_empty() {
        None
    } else {
        Some(copyrights.join("\n\n"))
    }
}

fn get_copyright_for_system(system: &str) -> Option<&'static str> {
    CODE_SYSTEM_COPYRIGHTS
        .iter()
        .find_map(|(url, copyright)| (system == *url).then(|| *copyright))
}

// ============================================================================
// Component Rule Types
// ============================================================================

/// Represents different types of ValueSet component rules
#[derive(Debug, Clone)]
enum ComponentRule {
    /// Include a specific concept: * SYSTEM#CODE "Display"
    IncludeConcept {
        system: String,
        concept: ValueSetConcept,
        version: Option<String>,
    },
    /// Exclude a specific concept: * exclude SYSTEM#CODE
    ExcludeConcept {
        system: String,
        concept: ValueSetConcept,
        version: Option<String>,
    },
    /// Include with filter: * SYSTEM where property op value
    IncludeFilter {
        system: String,
        filter: ValueSetFilter,
        version: Option<String>,
    },
    /// Exclude with filter: * exclude SYSTEM where property op value
    ExcludeFilter {
        system: String,
        filter: ValueSetFilter,
        version: Option<String>,
    },
    /// Include all codes from a system: * include codes from system X
    IncludeSystem {
        system: String,
        version: Option<String>,
    },
    /// Exclude all codes from a system: * exclude codes from system X
    ExcludeSystem {
        system: String,
        version: Option<String>,
    },
    /// Include from another ValueSet: * include codes from valueset "http://..."
    IncludeValueSet { value_set_url: String },
    /// Exclude from another ValueSet: * exclude codes from valueset "http://..."
    ExcludeValueSet { value_set_url: String },
}

// ============================================================================
// ValueSet Exporter
// ============================================================================

/// Exports FSH ValueSet definitions to FHIR ValueSet resources
///
/// # Phase 1: Metadata Export
///
/// Currently exports:
/// - Basic metadata (id, url, name, title, description)
/// - Status and date
/// - Publisher and copyright
///
/// # Future Enhancements
///
/// Phase 2 will add:
/// - Include/exclude component parsing
/// - Filter handling (is-a, regex, exists)
/// - ValueSet reference support
///
/// # Example
///
/// ```rust,no_run
/// use maki_core::export::ValueSetExporter;
/// use maki_core::canonical::DefinitionSession;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let session: Arc<DefinitionSession> = todo!();
/// let exporter = ValueSetExporter::new(
///     session,
///     "http://example.org/fhir".to_string(),
///     None,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct ValueSetExporter {
    /// Session for resolving FHIR definitions
    #[allow(dead_code)]
    session: Arc<DefinitionSession>,
    /// Base URL for valueset canonical URLs
    base_url: String,
    /// Version from config
    #[allow(dead_code)]
    version: Option<String>,
    /// Status from config (draft | active | retired | unknown)
    status: Option<String>,
}

impl ValueSetExporter {
    /// Create a new valueset exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving code systems and other valuesets
    /// * `base_url` - Base URL for generated valueset identifiers
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::ValueSetExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = ValueSetExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    ///     None,
    ///     Some("active".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
        version: Option<String>,
        status: Option<String>,
    ) -> Result<Self, ExportError> {
        Ok(Self {
            session,
            base_url,
            version,
            status,
        })
    }

    /// Export a ValueSet to a FHIR resource (JSON)
    ///
    /// # Arguments
    ///
    /// * `valueset` - FSH ValueSet AST node
    ///
    /// # Returns
    ///
    /// A FHIR ValueSet resource as JSON
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - ValueSet name is missing
    /// - Required metadata fields are invalid
    /// - Rule application fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::ValueSetExporter;
    /// use maki_core::cst::ast::ValueSet;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let exporter: ValueSetExporter = todo!();
    /// # let valueset: ValueSet = todo!();
    /// let resource = exporter.export(&valueset).await?;
    /// println!("{}", serde_json::to_string_pretty(&resource)?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export(&self, valueset: &ValueSet) -> Result<ValueSetResource, ExportError> {
        let name = valueset
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("ValueSet name".to_string()))?;

        debug!("Exporting ValueSet: {}", name);

        // Determine canonical id from Id clause when present, otherwise fall back to the FSH name
        let canonical_id = valueset
            .id()
            .and_then(|id_clause| id_clause.value())
            .unwrap_or_else(|| name.clone());

        // Generate canonical URL using the canonical id
        let url = format!("{}/ValueSet/{}", self.base_url, canonical_id);

        // Create base resource with status from config (defaults to "draft")
        let status = self.status.as_deref().unwrap_or("draft");
        let mut resource = ValueSetResource::new(url, name.clone(), status);
        // Set experimental to false by default (SUSHI parity)
        resource.experimental = Some(false);
        // Ensure id is always present for parity with SUSHI defaults
        resource.id = Some(canonical_id.clone());

        // Set id from Id clause if present
        if let Some(id_clause) = valueset.id()
            && let Some(id_value) = id_clause.value()
        {
            resource.id = Some(id_value);
        }

        // Set title from Title clause if present
        if let Some(title_clause) = valueset.title()
            && let Some(title_value) = title_clause.value()
        {
            resource.title = Some(title_value);
        }

        // Set description from Description clause if present
        if let Some(desc_clause) = valueset.description()
            && let Some(desc_value) = desc_clause.value()
        {
            resource.description = Some(desc_value);
        }

        // Do not set version unless explicitly provided in FSH (SUSHI parity)
        resource.version = None;

        // Build alias map from parent document
        let alias_map = self.build_alias_map(valueset);
        trace!("Built alias map with {} entries", alias_map.len());

        // Build map of ValueSet name -> id from the current document for URL resolution
        let valueset_id_map = self.build_valueset_id_map(valueset);

        // Initialize compose for component rules
        let mut compose = ValueSetCompose::new();

        // Group includes and excludes by system
        let mut system_includes: SystemComponentMap = HashMap::new();
        let mut system_excludes: SystemComponentMap = HashMap::new();
        let mut valueset_includes: Vec<String> = Vec::new();
        let mut valueset_excludes: Vec<String> = Vec::new();

        // Process rules
        for rule in valueset.rules() {
            match rule {
                Rule::FixedValue(fixed_rule) => {
                    self.apply_metadata_rule(&mut resource, &fixed_rule)?;
                }
                Rule::Card(_) => {
                    // Card rules don't apply to valuesets
                    trace!("Skipping card rule in valueset");
                }
                Rule::Flag(_) => {
                    // Flag rules don't apply to valuesets
                    trace!("Skipping flag rule in valueset");
                }
                Rule::ValueSet(_) => {
                    // ValueSet binding rules don't apply to valueset definitions
                    trace!("Skipping valueset rule in valueset definition");
                }
                Rule::Path(path_rule) => {
                    // Use structured component processing instead of manual parsing
                    self.process_path_rule_structured(
                        &path_rule,
                        &alias_map,
                        &name,
                        &valueset_id_map,
                        &mut system_includes,
                        &mut system_excludes,
                        &mut valueset_includes,
                        &mut valueset_excludes,
                    )?;
                }
                Rule::CaretValue(caret_rule) => {
                    // Only handle top-level caret rules (no element path)
                    if caret_rule.element_path().is_none() {
                        if let (Some(field), Some(value)) = (caret_rule.field(), caret_rule.value())
                        {
                            self.apply_metadata_property(&mut resource, &field, &value)?;
                        }
                    }
                }
                Rule::AddElement(_)
                | Rule::Contains(_)
                | Rule::Only(_)
                | Rule::Obeys(_)
                | Rule::Mapping(_)
                | Rule::CodeCaretValue(_)
                | Rule::Insert(_)
                | Rule::CodeInsert(_) => {
                    // These rules don't apply to valuesets
                    trace!("Skipping contains/only/obeys rule in valueset");
                }
            }
        }

        // Process top-level components (common syntax for ValueSet definitions)
        for vs_component in valueset.syntax().children().filter_map(VsComponent::cast) {
            self.process_top_level_component(
                &vs_component,
                &alias_map,
                &name,
                &valueset_id_map,
                &mut system_includes,
                &mut system_excludes,
                &mut valueset_includes,
                &mut valueset_excludes,
            )?;
        }

        // Build compose.include from grouped rules
        let mut has_content = false;

        // Add system-based includes (SUSHI parity: filters get separate includes, concepts are grouped)
        for (system, (concepts, filters, version)) in system_includes {
            let mut added_for_system = false;
            // Each filter gets its own include block (SUSHI behavior)
            for filter in filters {
                let mut include = ValueSetInclude::from_system(system.clone());
                if let Some(v) = &version {
                    include.version = Some(v.clone());
                }
                include.add_filter(filter);
                compose.add_include(include);
                has_content = true;
                added_for_system = true;
            }

            // Concepts are grouped together in one include block
            if !concepts.is_empty() {
                let mut include = ValueSetInclude::from_system(system.clone());
                if let Some(v) = version {
                    include.version = Some(v);
                }
                for concept in concepts {
                    include.add_concept(concept);
                }
                compose.add_include(include);
                has_content = true;
            } else if !added_for_system {
                // Bare system include (e.g., "include codes from system X")
                let mut include = ValueSetInclude::from_system(system);
                if let Some(v) = version {
                    include.version = Some(v);
                }
                compose.add_include(include);
                has_content = true;
            }
        }

        // Add ValueSet-based includes
        for value_set_url in valueset_includes {
            let include = ValueSetInclude::from_valueset(value_set_url);
            compose.add_include(include);
            has_content = true;
        }

        // Build compose.exclude from grouped rules (SUSHI parity: filters get separate excludes, concepts are grouped)
        for (system, (concepts, filters, version)) in system_excludes {
            let mut added_for_system = false;
            // Each filter gets its own exclude block (SUSHI behavior)
            for filter in filters {
                let mut exclude = ValueSetInclude::from_system(system.clone());
                if let Some(v) = &version {
                    exclude.version = Some(v.clone());
                }
                exclude.add_filter(filter);
                compose.add_exclude(exclude);
                has_content = true;
                added_for_system = true;
            }

            // Concepts are grouped together in one exclude block
            if !concepts.is_empty() {
                let mut exclude = ValueSetInclude::from_system(system.clone());
                if let Some(v) = version {
                    exclude.version = Some(v);
                }
                for concept in concepts {
                    exclude.add_concept(concept);
                }
                compose.add_exclude(exclude);
                has_content = true;
            } else if !added_for_system {
                // Bare system exclude (e.g., "exclude codes from system X")
                let mut exclude = ValueSetInclude::from_system(system);
                if let Some(v) = version {
                    exclude.version = Some(v);
                }
                compose.add_exclude(exclude);
                has_content = true;
            }
        }

        // Add ValueSet-based excludes
        for value_set_url in valueset_excludes {
            let exclude = ValueSetInclude::from_valueset(value_set_url);
            compose.add_exclude(exclude);
            has_content = true;
        }

        if has_content {
            // Validate exclude rules don't conflict with include rules
            self.validate_exclude_conflicts(&compose, &name)?;

            // Generate copyright notice based on code systems used (SUSHI parity)
            // Only set if not already set via FSH metadata rules
            if resource.copyright.is_none() {
                resource.copyright = generate_copyright_from_compose(&compose);
            }

            resource.compose = Some(compose);
        }

        debug!("Successfully exported ValueSet {}", name);
        Ok(resource)
    }

    /// Apply a metadata rule (^property = value)
    ///
    /// Handles common metadata properties:
    /// - ^url - Canonical URL
    /// - ^version - Business version
    /// - ^status - Publication status
    /// - ^date - Last changed date
    /// - ^publisher - Publisher name
    /// - ^description - Description
    /// - ^purpose - Purpose
    /// - ^copyright - Copyright notice
    fn apply_metadata_rule(
        &self,
        resource: &mut ValueSetResource,
        rule: &FixedValueRule,
    ) -> Result<(), ExportError> {
        // Get the path (property name)
        let path = rule
            .path()
            .map(|p| p.as_string())
            .ok_or_else(|| ExportError::InvalidPath {
                path: "<unknown>".to_string(),
                resource: "ValueSet".to_string(),
            })?;

        // Get the value
        let value_str = rule.value().ok_or_else(|| {
            ExportError::InvalidValue("Missing value in metadata rule".to_string())
        })?;

        trace!("Applying metadata: {} = {}", path, value_str);

        // Strip ^ prefix if present
        let property = path.strip_prefix('^').unwrap_or(&path);

        self.apply_metadata_property(resource, property, &value_str)
    }

    /// Apply metadata based on property/value strings
    fn apply_metadata_property(
        &self,
        resource: &mut ValueSetResource,
        property: &str,
        value_str: &str,
    ) -> Result<(), ExportError> {
        // Apply based on property name
        match property {
            "url" => {
                // Remove quotes from string value
                resource.url = self.extract_string_value(value_str);
            }
            "version" => {
                resource.version = Some(self.extract_string_value(value_str));
            }
            "status" => {
                // Extract code value (remove # prefix)
                let status = if let Some(code) = value_str.strip_prefix('#') {
                    code.to_string()
                } else {
                    value_str.to_string()
                };
                resource.status = status;
            }
            "date" => {
                resource.date = Some(self.extract_string_value(value_str));
            }
            "publisher" => {
                resource.publisher = Some(self.extract_string_value(value_str));
            }
            "description" => {
                resource.description = Some(self.extract_string_value(value_str));
            }
            "experimental" => {
                resource.experimental = Some(self.extract_bool_value(value_str)?);
            }
            "purpose" => {
                resource.purpose = Some(self.extract_string_value(value_str));
            }
            "copyright" => {
                resource.copyright = Some(self.extract_string_value(value_str));
            }
            "immutable" => {
                // Parse boolean
                let immutable = value_str.trim() == "true";
                resource.immutable = Some(immutable);
            }
            _ => {
                if property.starts_with("extension[") {
                    self.apply_extension_rule(resource, property, value_str)?;
                } else {
                    // Unknown property - log warning but don't fail
                    warn!("Unknown ValueSet metadata property: {}", property);
                }
            }
        }

        Ok(())
    }

    /// Extract string value from FSH value (removes quotes)
    fn extract_string_value(&self, value_str: &str) -> String {
        let trimmed = value_str.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Extract a boolean value from a FSH boolean literal
    fn extract_bool_value(&self, value_str: &str) -> Result<bool, ExportError> {
        let normalized = value_str.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ExportError::InvalidValue(format!(
                "Invalid boolean value '{}'",
                value_str
            ))),
        }
    }

    /// Convert a string to kebab-case
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

    /// Apply an extension caret rule (e.g., ^extension[FMM].valueInteger = 4)
    fn apply_extension_rule(
        &self,
        resource: &mut ValueSetResource,
        property: &str,
        value_str: &str,
    ) -> Result<(), ExportError> {
        // Expect format extension[Name].valueX
        let Some(rest) = property.strip_prefix("extension[") else {
            return Ok(());
        };

        let (ext_name, value_path) = if let Some((name, path)) = rest.split_once("].") {
            (name, path)
        } else {
            (rest.trim_end_matches(']'), "")
        };

        // Only handle FMM for now; other extensions can be added as needed
        let url = match ext_name {
            "FMM" => "http://hl7.org/fhir/StructureDefinition/structuredefinition-fmm",
            _ => return Ok(()),
        };

        let mut ext_obj = serde_json::Map::new();
        ext_obj.insert("url".to_string(), JsonValue::String(url.to_string()));

        match value_path {
            "valueInteger" => {
                let value = self
                    .extract_string_value(value_str)
                    .parse::<i64>()
                    .map_err(|_| ExportError::InvalidValue(value_str.to_string()))?;
                ext_obj.insert("valueInteger".to_string(), JsonValue::from(value));
            }
            "valueString" | "" => {
                let value = self.extract_string_value(value_str);
                ext_obj.insert("valueString".to_string(), JsonValue::String(value));
            }
            "valueBoolean" => {
                let value = self.extract_bool_value(value_str)?;
                ext_obj.insert("valueBoolean".to_string(), JsonValue::Bool(value));
            }
            _ => {
                // Unsupported extension value path; ignore
                return Ok(());
            }
        }

        resource
            .extension
            .get_or_insert_with(Vec::new)
            .push(JsonValue::Object(ext_obj));
        Ok(())
    }

    /// Build alias map from parent document
    ///
    /// Traverses up to the parent Document node and extracts all aliases
    /// into a HashMap for quick lookup during component rule parsing.
    fn build_alias_map(&self, valueset: &ValueSet) -> HashMap<String, String> {
        // Seed with built-in code system aliases so short system names resolve to canonical URLs
        let mut alias_map: HashMap<String, String> = CODE_SYSTEM_ALIASES
            .iter()
            .map(|(name, url)| (name.to_string(), url.to_string()))
            .collect();

        // Get parent document by traversing up the tree
        if let Some(parent) = valueset.syntax().parent()
            && let Some(document) = Document::cast(parent)
        {
            for alias in document.aliases() {
                if let Some(name) = alias.name()
                    && let Some(value) = alias.value()
                {
                    alias_map.insert(name, value);
                }
            }
        }

        trace!("Built alias map with {} entries", alias_map.len());
        alias_map
    }

    /// Build a map of ValueSet name -> id from the current document
    fn build_valueset_id_map(&self, valueset: &ValueSet) -> HashMap<String, String> {
        let mut map = HashMap::new();

        if let Some(parent) = valueset.syntax().parent()
            && let Some(document) = Document::cast(parent)
        {
            for vs in document.value_sets() {
                if let Some(vs_name) = vs.name() {
                    if let Some(id_clause) = vs.id()
                        && let Some(id_value) = id_clause.value()
                    {
                        map.insert(vs_name, id_value);
                    }
                }
            }
        }

        map
    }

    /// Process a PathRule using structured CST-based processing
    ///
    /// Uses VsComponent and VsFilterDefinition nodes instead of manual string parsing
    #[allow(clippy::too_many_arguments)]
    fn process_path_rule_structured(
        &self,
        path_rule: &PathRule,
        alias_map: &HashMap<String, String>,
        valueset_name: &str,
        valueset_id_map: &HashMap<String, String>,
        system_includes: &mut SystemComponentMap,
        system_excludes: &mut SystemComponentMap,
        valueset_includes: &mut Vec<String>,
        valueset_excludes: &mut Vec<String>,
    ) -> Result<(), ExportError> {
        let mut structured_applied = false;

        // Try to find VsComponent nodes in the path rule
        for vs_component in path_rule.syntax().children().filter_map(VsComponent::cast) {
            if let Some(concept_component) = vs_component.concept() {
                // Handle concept components using structured access
                if self.process_concept_component_structured(
                    &concept_component,
                    vs_component.is_exclude(),
                    alias_map,
                    system_includes,
                    system_excludes,
                )? {
                    structured_applied = true;
                }
            } else if let Some(filter_component) = vs_component.filter() {
                // Handle filter components using structured access
                if self.process_filter_component_structured(
                    &filter_component,
                    vs_component.is_exclude(),
                    alias_map,
                    system_includes,
                    system_excludes,
                )? {
                    structured_applied = true;
                }
            }
        }

        // Fallback to manual parsing if we didn't handle anything structurally
        if !structured_applied {
            let rule_text = path_rule.syntax().text().to_string();
            if let Some(component_rule) = self.parse_component_rule(
                &rule_text,
                Some(path_rule),
                alias_map,
                valueset_name,
                valueset_id_map,
            )? {
                self.apply_component_rule_to_maps(
                    component_rule,
                    system_includes,
                    system_excludes,
                    valueset_includes,
                    valueset_excludes,
                );
            }
        }

        Ok(())
    }

    /// Process a top-level VsComponent (common in ValueSet definitions)
    #[allow(clippy::too_many_arguments)]
    fn process_top_level_component(
        &self,
        vs_component: &VsComponent,
        alias_map: &HashMap<String, String>,
        valueset_name: &str,
        valueset_id_map: &HashMap<String, String>,
        system_includes: &mut SystemComponentMap,
        system_excludes: &mut SystemComponentMap,
        valueset_includes: &mut Vec<String>,
        valueset_excludes: &mut Vec<String>,
    ) -> Result<(), ExportError> {
        let mut structured_applied = false;

        if let Some(concept_component) = vs_component.concept() {
            if self.process_concept_component_structured(
                &concept_component,
                vs_component.is_exclude(),
                alias_map,
                system_includes,
                system_excludes,
            )? {
                structured_applied = true;
            }
        } else if let Some(filter_component) = vs_component.filter() {
            if self.process_filter_component_structured(
                &filter_component,
                vs_component.is_exclude(),
                alias_map,
                system_includes,
                system_excludes,
            )? {
                structured_applied = true;
            }
        }

        if !structured_applied {
            let rule_text = vs_component.syntax().text().to_string();
            if let Some(component_rule) = self.parse_component_rule(
                &rule_text,
                None,
                alias_map,
                valueset_name,
                valueset_id_map,
            )? {
                self.apply_component_rule_to_maps(
                    component_rule,
                    system_includes,
                    system_excludes,
                    valueset_includes,
                    valueset_excludes,
                );
            }
        }

        Ok(())
    }

    /// Apply a parsed component rule to the appropriate include/exclude collections
    fn apply_component_rule_to_maps(
        &self,
        component_rule: ComponentRule,
        system_includes: &mut SystemComponentMap,
        system_excludes: &mut SystemComponentMap,
        valueset_includes: &mut Vec<String>,
        valueset_excludes: &mut Vec<String>,
    ) {
        match component_rule {
            ComponentRule::IncludeConcept {
                system,
                concept,
                version,
            } => {
                let entry = system_includes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                entry.0.push(concept);
            }
            ComponentRule::ExcludeConcept {
                system,
                concept,
                version,
            } => {
                let entry = system_excludes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                entry.0.push(concept);
            }
            ComponentRule::IncludeFilter {
                system,
                filter,
                version,
            } => {
                let entry = system_includes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                entry.1.push(filter);
            }
            ComponentRule::ExcludeFilter {
                system,
                filter,
                version,
            } => {
                let entry = system_excludes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
                entry.1.push(filter);
            }
            ComponentRule::IncludeValueSet { value_set_url } => {
                valueset_includes.push(value_set_url);
            }
            ComponentRule::ExcludeValueSet { value_set_url } => {
                valueset_excludes.push(value_set_url);
            }
            ComponentRule::IncludeSystem { system, version } => {
                system_includes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
            }
            ComponentRule::ExcludeSystem { system, version } => {
                system_excludes
                    .entry(system)
                    .or_insert_with(|| (Vec::new(), Vec::new(), version));
            }
        }
    }

    /// Process concept component using structured access
    fn process_concept_component_structured(
        &self,
        concept_component: &VsConceptComponent,
        is_exclude: bool,
        alias_map: &HashMap<String, String>,
        system_includes: &mut SystemComponentMap,
        system_excludes: &mut SystemComponentMap,
    ) -> Result<bool, ExportError> {
        let mut applied = false;
        if let Some(code_ref) = concept_component.code() {
            let system_url = if let Some(system) = code_ref.system() {
                // Resolve alias to full URL
                alias_map.get(&system).cloned().unwrap_or(system)
            } else {
                return Ok(false); // Skip if no system
            };

            if let Some(code) = code_ref.code() {
                let display = concept_component.display();
                // Version handling simplified for now - can be enhanced later
                let version = None;

                let concept = if let Some(display_text) = display {
                    ValueSetConcept::with_display(&code, display_text)
                } else {
                    ValueSetConcept::new(&code)
                };

                if is_exclude {
                    let entry = system_excludes
                        .entry(system_url)
                        .or_insert_with(|| (Vec::new(), Vec::new(), version));
                    entry.0.push(concept);
                } else {
                    let entry = system_includes
                        .entry(system_url)
                        .or_insert_with(|| (Vec::new(), Vec::new(), version));
                    entry.0.push(concept);
                }
                applied = true;
            }
        }

        Ok(applied)
    }

    /// Process filter component using structured access
    fn process_filter_component_structured(
        &self,
        filter_component: &VsFilterComponent,
        is_exclude: bool,
        alias_map: &HashMap<String, String>,
        system_includes: &mut SystemComponentMap,
        system_excludes: &mut SystemComponentMap,
    ) -> Result<bool, ExportError> {
        let mut applied = false;
        if let Some(from_clause) = filter_component.from_clause() {
            for system in from_clause.systems() {
                let system_url = alias_map.get(&system).cloned().unwrap_or(system);
                // Version handling simplified for now - can be enhanced later
                let version = None;

                // Process all filters in this component
                for filter_def in filter_component.filters() {
                    if let (Some(property), Some(operator), Some(value)) = (
                        filter_def.property(),
                        filter_def.operator_string(),
                        filter_def.value_string(),
                    ) {
                        // Strip # prefix from code values (SUSHI parity)
                        let clean_value = self.clean_filter_value(&value);
                        let filter = ValueSetFilter::new(&property, &operator, clean_value);

                        if is_exclude {
                            let entry = system_excludes
                                .entry(system_url.clone())
                                .or_insert_with(|| (Vec::new(), Vec::new(), version.clone()));
                            entry.1.push(filter);
                        } else {
                            let entry = system_includes
                                .entry(system_url.clone())
                                .or_insert_with(|| (Vec::new(), Vec::new(), version.clone()));
                            entry.1.push(filter);
                        }
                        applied = true;
                    }
                }
            }
        }

        Ok(applied)
    }

    /// Parse a component rule (PathRule) into system includes/excludes
    ///
    /// Handles syntax like:
    /// - `* SPTY#AMN "Amniotic fluid"` - code with display
    /// - `* SPTY#BLD` - code without display
    /// - `* SNOMED_CT where concept is-a #123456` - filter rule
    /// - `* exclude LOINC#12345-6` - exclude rule
    /// - `* LOINC version "2.72"` - version-specific include
    ///
    /// Returns: ComponentRule enum indicating the type of rule parsed
    fn parse_component_rule(
        &self,
        rule_text: &str,
        path_rule: Option<&PathRule>,
        alias_map: &HashMap<String, String>,
        valueset_name: &str,
        valueset_id_map: &HashMap<String, String>,
    ) -> Result<Option<ComponentRule>, ExportError> {
        // Get the full text of the component rule
        let rule_text = rule_text.trim();

        trace!("Parsing component rule: '{}'", rule_text);

        // Check for exclude prefix
        let (is_exclude, remaining_text) = if rule_text.starts_with("exclude ") {
            (true, rule_text.strip_prefix("exclude ").unwrap().trim())
        } else {
            (false, rule_text)
        };

        // Check for "codes from valueset" syntax
        if remaining_text.contains("codes from valueset") {
            return self.parse_valueset_reference(
                remaining_text,
                is_exclude,
                alias_map,
                valueset_id_map,
            );
        }

        // Check for "codes from system" syntax
        if remaining_text.contains("codes from system") {
            if let Some(component_rule) =
                self.parse_system_reference(remaining_text, is_exclude, alias_map)?
            {
                return Ok(Some(component_rule));
            }
        }

        // Check for "where" clause (filter syntax)
        if remaining_text.contains(" where ") {
            return self.parse_filter_rule(remaining_text, alias_map, is_exclude, valueset_name);
        }

        // Check for version specification
        let (system_and_code, version) = if let Some(version_pos) = remaining_text.find(" version ")
        {
            let code_part = remaining_text[..version_pos].trim();
            let version_part = remaining_text[version_pos + 9..].trim(); // " version " is 9 chars
            let version = self.extract_string_value(version_part);
            (code_part, Some(version))
        } else {
            (remaining_text, None)
        };

        // Parse: CodeSystemPrefix#Code
        let (system_prefix, code) = if let Some(hash_pos) = system_and_code.find('#') {
            let system = system_and_code[..hash_pos].trim();
            let code = system_and_code[hash_pos + 1..].trim();
            (system, code)
        } else {
            warn!(
                "Invalid component rule format '{}' in ValueSet {}. Expected format: SYSTEM#CODE",
                system_and_code, valueset_name
            );
            return Ok(None);
        };

        // Resolve alias to full URL (document alias, built-ins, or fallback)
        let system_url = match alias_map.get(system_prefix) {
            Some(url) => url.clone(),
            None => resolve_code_system_alias(system_prefix)
                .map(|u| u.to_string())
                .unwrap_or_else(|| {
                    warn!(
                        "Unresolved alias '{}' in ValueSet {}. Using as-is.",
                        system_prefix, valueset_name
                    );
                    system_prefix.to_string()
                }),
        };

        // Try to get display text from following String token
        let display = path_rule.and_then(|pr| self.get_display_text(pr));

        // Create concept with enhanced parsing for properties and designations
        let mut concept = if let Some(display_text) = display {
            ValueSetConcept::with_display(code, display_text)
        } else {
            ValueSetConcept::new(code)
        };

        // Parse additional concept metadata from the rule text
        if let Some(path_rule) = path_rule {
            self.parse_concept_metadata(&mut concept, path_rule)?;
        }

        trace!(
            "Parsed component rule: system={}, code={}, display={:?}, exclude={}, version={:?}",
            system_url, code, concept.display, is_exclude, version
        );

        let component_rule = if is_exclude {
            ComponentRule::ExcludeConcept {
                system: system_url,
                concept,
                version,
            }
        } else {
            ComponentRule::IncludeConcept {
                system: system_url,
                concept,
                version,
            }
        };

        Ok(Some(component_rule))
    }

    /// Parse a filter rule: SYSTEM where property op value
    ///
    /// Examples:
    /// - `SNOMED_CT where concept is-a #123456`
    /// - `LOINC where STATUS = ACTIVE`
    /// - `ICD10 where concept regex "^A.*"`
    fn parse_filter_rule(
        &self,
        rule_text: &str,
        alias_map: &HashMap<String, String>,
        is_exclude: bool,
        valueset_name: &str,
    ) -> Result<Option<ComponentRule>, ExportError> {
        // Use more structured parsing approach
        let (system_part, filter_part) = if let Some(where_pos) = rule_text.find(" where ") {
            let system_part = rule_text[..where_pos].trim();
            let filter_part = rule_text[where_pos + 7..].trim(); // " where " is 7 chars
            (system_part, filter_part)
        } else {
            warn!(
                "Invalid filter rule format in ValueSet {}: {}",
                valueset_name, rule_text
            );
            return Ok(None);
        };

        // Parse version if present
        let (system_prefix, version) = if let Some(version_pos) = system_part.find(" version ") {
            let prefix = system_part[..version_pos].trim();
            let version_str = system_part[version_pos + 9..].trim(); // " version " is 9 chars
            let version = self.extract_string_value(version_str);
            (prefix, Some(version))
        } else {
            (system_part, None)
        };

        // Resolve system alias (document alias, built-ins, or fallback)
        let system_url = match alias_map.get(system_prefix) {
            Some(url) => url.clone(),
            None => resolve_code_system_alias(system_prefix)
                .map(|u| u.to_string())
                .unwrap_or_else(|| {
                    warn!(
                        "Unresolved alias '{}' in ValueSet {}. Using as-is.",
                        system_prefix, valueset_name
                    );
                    system_prefix.to_string()
                }),
        };

        // Parse filter expression: property op value
        let filter = self.parse_filter_expression(filter_part)?;

        trace!(
            "Parsed filter rule: system={}, filter={:?}, exclude={}, version={:?}",
            system_url, filter, is_exclude, version
        );

        let component_rule = if is_exclude {
            ComponentRule::ExcludeFilter {
                system: system_url,
                filter,
                version,
            }
        } else {
            ComponentRule::IncludeFilter {
                system: system_url,
                filter,
                version,
            }
        };

        Ok(Some(component_rule))
    }

    /// Parse a filter expression: property op value
    ///
    /// Examples:
    /// - `concept is-a #123456`
    /// - `STATUS = ACTIVE`
    /// - `concept regex "^A.*"`
    /// - `inactive exists true`
    /// - `concept generalizes #123456`
    /// - `STATUS in "ACTIVE,INACTIVE"`
    /// - `concept not-in "123456,789012"`
    fn parse_filter_expression(&self, filter_text: &str) -> Result<ValueSetFilter, ExportError> {
        let filter_text = filter_text.trim();

        // Validate filter expression syntax
        self.validate_filter_syntax(filter_text)?;

        // Enhanced patterns to match more complex expressions
        let patterns = [
            // Concept-based filters
            (r"concept\s+is-a\s+#?(\S+)", "concept", "is-a"),
            (
                r"concept\s+descend(?:e|a)nt-of\s+#?(\S+)",
                "concept",
                "descendant-of",
            ),
            (
                r"concept\s+descendsFrom\s+#?(\S+)",
                "concept",
                "descendant-of",
            ),
            (r"concept\s+is-not-a\s+#?(\S+)", "concept", "is-not-a"),
            (r"concept\s+generalizes\s+#?(\S+)", "concept", "generalizes"),
            (r"concept\s+regex\s+(.+)", "concept", "regex"),
            // Property-based filters with various operators
            (r"(\w+)\s+exists\s+(true|false)", "", "exists"),
            (r"(\w+)\s+=\s+(.+)", "", "="),
            (r"(\w+)\s+!=\s+(.+)", "", "!="),
            (r"(\w+)\s+in\s+(.+)", "", "in"),
            (r"(\w+)\s+not-in\s+(.+)", "", "not-in"),
            // Special SNOMED CT filters
            (r"(\w+)\s+child-of\s+#?(\S+)", "", "child-of"),
            (r"(\w+)\s+parent-of\s+#?(\S+)", "", "parent-of"),
            (r"(\w+)\s+ancestor-of\s+#?(\S+)", "", "ancestor-of"),
        ];

        for (pattern, default_property, op) in patterns {
            if let Ok(regex) = regex::Regex::new(pattern)
                && let Some(captures) = regex.captures(filter_text)
            {
                let property = if default_property.is_empty() {
                    captures.get(1).map(|m| m.as_str()).unwrap_or("concept")
                } else {
                    default_property
                };

                let value_index = if default_property.is_empty() { 2 } else { 1 };
                let value = captures
                    .get(value_index)
                    .map(|m| m.as_str().trim())
                    .unwrap_or("");

                // Clean up value (remove quotes, # prefix)
                let clean_value = self.clean_filter_value(value);

                // Validate the operator is supported
                let normalized_op = Self::normalize_filter_operator(op);
                self.validate_filter_operator(&normalized_op, property)?;

                return Ok(ValueSetFilter::new(property, normalized_op, clean_value));
            }
        }

        // Fallback: try to parse as "property op value" with whitespace
        // This is kept for backward compatibility but should be replaced with structured parsing
        let words: Vec<&str> = filter_text.split_whitespace().collect();
        if words.len() >= 3 {
            let property = words[0];
            let op = Self::normalize_filter_operator(words[1]);
            let value = words[2..].join(" ");
            let clean_value = self.clean_filter_value(&value);

            // Validate the operator
            self.validate_filter_operator(&op, property)?;

            return Ok(ValueSetFilter::new(property, op, clean_value));
        }

        Err(ExportError::InvalidValue(format!(
            "Unable to parse filter expression: {}",
            filter_text
        )))
    }

    /// Validate filter expression syntax
    fn validate_filter_syntax(&self, filter_text: &str) -> Result<(), ExportError> {
        if filter_text.is_empty() {
            return Err(ExportError::InvalidValue(
                "Empty filter expression".to_string(),
            ));
        }

        // Check for balanced quotes
        let quote_count = filter_text.chars().filter(|&c| c == '"').count();
        if quote_count % 2 != 0 {
            return Err(ExportError::InvalidValue(format!(
                "Unbalanced quotes in filter expression: {}",
                filter_text
            )));
        }

        // Check for basic structure (property operator value)
        let words: Vec<&str> = filter_text.split_whitespace().collect();
        if words.len() < 2 {
            return Err(ExportError::InvalidValue(format!(
                "Filter expression must have at least property and operator: {}",
                filter_text
            )));
        }

        Ok(())
    }

    /// Normalize filter operators to their canonical form (e.g., descendent-of -> descendant-of)
    fn normalize_filter_operator(op: &str) -> String {
        match op {
            "descendent-of" | "descendsFrom" => "descendant-of".to_string(),
            _ => op.to_string(),
        }
    }

    /// Validate that the filter operator is supported
    fn validate_filter_operator(&self, op: &str, property: &str) -> Result<(), ExportError> {
        let valid_operators = [
            "=",
            "!=",
            "is-a",
            "descendant-of",
            "is-not-a",
            "regex",
            "in",
            "not-in",
            "generalizes",
            "exists",
            "child-of",
            "parent-of",
            "ancestor-of",
        ];

        if !valid_operators.contains(&op) {
            return Err(ExportError::InvalidValue(format!(
                "Unsupported filter operator '{}' for property '{}'",
                op, property
            )));
        }

        // Validate operator-property combinations
        match (property, op) {
            ("concept", "exists") => {
                return Err(ExportError::InvalidValue(
                    "The 'exists' operator is not valid for the 'concept' property".to_string(),
                ));
            }
            (_, "is-a" | "descendant-of" | "is-not-a" | "generalizes") if property != "concept" => {
                warn!(
                    "Hierarchical operator '{}' used with property '{}' - may not be supported by all systems",
                    op, property
                );
            }
            _ => {} // Valid combination
        }

        Ok(())
    }

    /// Clean up filter value (remove quotes, # prefix, etc.)
    fn clean_filter_value(&self, value: &str) -> String {
        let value = value.trim();

        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            // Remove surrounding quotes
            value[1..value.len() - 1].to_string()
        } else if let Some(stripped) = value.strip_prefix('#') {
            // Remove # prefix for codes
            stripped.to_string()
        } else {
            value.to_string()
        }
    }

    /// Parse a ValueSet reference: codes from valueset "URL"
    fn parse_valueset_reference(
        &self,
        rule_text: &str,
        is_exclude: bool,
        alias_map: &HashMap<String, String>,
        valueset_id_map: &HashMap<String, String>,
    ) -> Result<Option<ComponentRule>, ExportError> {
        // Extract the ValueSet URL after "codes from valueset"
        if let Some(url_start) = rule_text.find("codes from valueset") {
            let url_part = rule_text[url_start + "codes from valueset".len()..].trim();
            let value_set_url =
                self.normalize_valueset_reference(url_part, alias_map, valueset_id_map);

            let component_rule = if is_exclude {
                ComponentRule::ExcludeValueSet { value_set_url }
            } else {
                ComponentRule::IncludeValueSet { value_set_url }
            };

            return Ok(Some(component_rule));
        }

        Ok(None)
    }

    /// Parse "codes from system" component (system-only include/exclude)
    fn parse_system_reference(
        &self,
        rule_text: &str,
        is_exclude: bool,
        alias_map: &HashMap<String, String>,
    ) -> Result<Option<ComponentRule>, ExportError> {
        if let Some(start) = rule_text.find("codes from system") {
            let mut system_part = rule_text[start + "codes from system".len()..].trim();
            let mut version: Option<String> = None;

            if let Some(version_pos) = system_part.find(" version ") {
                version = Some(
                    self.extract_string_value(&system_part[version_pos + " version ".len()..]),
                );
                system_part = system_part[..version_pos].trim();
            }

            let system_url = self.resolve_system_identifier(system_part, alias_map);

            let component_rule = if is_exclude {
                ComponentRule::ExcludeSystem {
                    system: system_url,
                    version,
                }
            } else {
                ComponentRule::IncludeSystem {
                    system: system_url,
                    version,
                }
            };

            return Ok(Some(component_rule));
        }

        Ok(None)
    }

    /// Resolve a system identifier using document aliases or built-in mappings
    fn resolve_system_identifier(
        &self,
        identifier: &str,
        alias_map: &HashMap<String, String>,
    ) -> String {
        let extracted = self.extract_string_value(identifier);

        alias_map
            .get(&extracted)
            .cloned()
            .or_else(|| resolve_code_system_alias(&extracted).map(|u| u.to_string()))
            .unwrap_or(extracted)
    }

    /// Normalize a ValueSet reference to a canonical URL
    fn normalize_valueset_reference(
        &self,
        value: &str,
        alias_map: &HashMap<String, String>,
        valueset_id_map: &HashMap<String, String>,
    ) -> String {
        let extracted = self.extract_string_value(value);
        let trimmed = extracted.trim();

        // Handle alias references like $MyAlias
        let alias_key = trimmed.strip_prefix('$').unwrap_or(trimmed);
        if let Some(alias_value) = alias_map.get(alias_key) {
            return alias_value.clone();
        }

        // Already canonical URL
        if trimmed.contains("://") {
            return trimmed.to_string();
        }

        // If we know the id for this ValueSet name, build canonical URL from it
        if let Some(id) = valueset_id_map.get(trimmed) {
            return format!("{}/ValueSet/{}", self.base_url, id);
        }

        // Fallback to kebab-casing the name
        let kebab = Self::kebab_case(trimmed);
        format!("{}/ValueSet/{}", self.base_url, kebab)
    }

    /// Validate that exclude rules don't conflict with include rules
    ///
    /// Checks for potential conflicts where the same system/concept is both included and excluded
    fn validate_exclude_conflicts(
        &self,
        compose: &ValueSetCompose,
        valueset_name: &str,
    ) -> Result<(), ExportError> {
        let empty_vec = Vec::new();
        let includes = compose.include.as_ref().unwrap_or(&empty_vec);
        let excludes = compose.exclude.as_ref().unwrap_or(&empty_vec);

        for exclude in excludes {
            for include in includes {
                // Check for same system conflicts
                if let (Some(exclude_system), Some(include_system)) =
                    (&exclude.system, &include.system)
                    && exclude_system == include_system
                {
                    // Check for specific concept conflicts
                    if let (Some(exclude_concepts), Some(include_concepts)) =
                        (&exclude.concept, &include.concept)
                    {
                        for exclude_concept in exclude_concepts {
                            for include_concept in include_concepts {
                                if exclude_concept.code == include_concept.code {
                                    warn!(
                                        "ValueSet {}: Concept {}#{} is both included and excluded",
                                        valueset_name, exclude_system, exclude_concept.code
                                    );
                                }
                            }
                        }
                    }

                    // Check for filter conflicts (more complex, just warn for now)
                    if exclude.filter.is_some() && include.filter.is_some() {
                        warn!(
                            "ValueSet {}: System {} has both include and exclude filters - potential conflicts",
                            valueset_name, exclude_system
                        );
                    }
                }

                // Check for ValueSet reference conflicts
                if let (Some(exclude_valuesets), Some(include_valuesets)) =
                    (&exclude.value_set, &include.value_set)
                {
                    for exclude_vs in exclude_valuesets {
                        for include_vs in include_valuesets {
                            if exclude_vs == include_vs {
                                warn!(
                                    "ValueSet {}: ValueSet {} is both included and excluded",
                                    valueset_name, exclude_vs
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse concept metadata like properties and designations
    ///
    /// Handles extended syntax like:
    /// - `* LOINC#12345-6 "Blood pressure" ^property[status] = #active`
    /// - `* SNOMED#123456 "Condition" ^designation[0].language = #en ^designation[0].value = "English term"`
    fn parse_concept_metadata(
        &self,
        concept: &mut ValueSetConcept,
        path_rule: &PathRule,
    ) -> Result<(), ExportError> {
        let rule_text = path_rule.syntax().text().to_string();

        // Look for property assignments: ^property[name] = value
        if rule_text.contains("^property[") {
            self.parse_concept_properties(concept, &rule_text)?;
        }

        // Look for designation assignments: ^designation[index].field = value
        if rule_text.contains("^designation[") {
            self.parse_concept_designations(concept, &rule_text)?;
        }

        Ok(())
    }

    /// Parse concept properties from rule text
    ///
    /// Examples:
    /// - `^property[status] = #active`
    /// - `^property[inactive] = false`
    /// - `^property[parent] = #123456`
    fn parse_concept_properties(
        &self,
        concept: &mut ValueSetConcept,
        rule_text: &str,
    ) -> Result<(), ExportError> {
        // Simple regex to find property assignments
        if let Ok(regex) = regex::Regex::new(r"\^property\[([^\]]+)\]\s*=\s*([^,\s]+)") {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(property_name), Some(property_value)) =
                    (captures.get(1), captures.get(2))
                {
                    let name = property_name.as_str();
                    let value = property_value.as_str();

                    // Determine property type and create appropriate property
                    let property = if let Some(stripped) = value.strip_prefix('#') {
                        // Code value
                        ValueSetConceptProperty::code(name, stripped)
                    } else if value == "true" || value == "false" {
                        // Boolean value
                        ValueSetConceptProperty::boolean(name, value == "true")
                    } else if let Ok(int_val) = value.parse::<i32>() {
                        // Integer value
                        ValueSetConceptProperty::integer(name, int_val)
                    } else if value.starts_with('"') && value.ends_with('"') {
                        // String value
                        ValueSetConceptProperty::string(name, &value[1..value.len() - 1])
                    } else {
                        // Default to string
                        ValueSetConceptProperty::string(name, value)
                    };

                    concept.add_property(property);
                }
            }
        }

        Ok(())
    }

    /// Parse concept designations from rule text
    ///
    /// Examples:
    /// - `^designation[0].language = #en`
    /// - `^designation[0].value = "English term"`
    /// - `^designation[1].language = #es ^designation[1].value = "Término español"`
    fn parse_concept_designations(
        &self,
        concept: &mut ValueSetConcept,
        rule_text: &str,
    ) -> Result<(), ExportError> {
        use std::collections::HashMap;

        // Collect designation assignments by index
        let mut designations: HashMap<usize, (Option<String>, Option<String>)> = HashMap::new();

        // Parse language assignments
        if let Ok(regex) = regex::Regex::new(r"\^designation\[(\d+)\]\.language\s*=\s*#?([^\s,]+)")
        {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(index_match), Some(lang_match)) = (captures.get(1), captures.get(2))
                    && let Ok(index) = index_match.as_str().parse::<usize>()
                {
                    let entry = designations.entry(index).or_insert((None, None));
                    entry.0 = Some(lang_match.as_str().to_string());
                }
            }
        }

        // Parse value assignments
        if let Ok(regex) = regex::Regex::new(r#"\^designation\[(\d+)\]\.value\s*=\s*"([^"]+)""#) {
            for captures in regex.captures_iter(rule_text) {
                if let (Some(index_match), Some(value_match)) = (captures.get(1), captures.get(2))
                    && let Ok(index) = index_match.as_str().parse::<usize>()
                {
                    let entry = designations.entry(index).or_insert((None, None));
                    entry.1 = Some(value_match.as_str().to_string());
                }
            }
        }

        // Create designation objects
        for (_index, (language, value)) in designations {
            if let Some(designation_value) = value {
                let mut designation = ValueSetConceptDesignation::new(designation_value);
                if let Some(lang) = language {
                    designation.language = Some(lang);
                }
                concept.add_designation(designation);
            }
        }

        Ok(())
    }

    /// Extract display text from a PathRule
    ///
    /// Looks for a String token following the path in the syntax tree.
    /// Example: `* SPTY#AMN "Amniotic fluid"` -> Some("Amniotic fluid")
    fn get_display_text(&self, path_rule: &PathRule) -> Option<String> {
        use crate::cst::FshSyntaxKind;

        // Look for a String token following the PathRule node
        // We iterate through all descendants with tokens to find a String
        for child in path_rule.syntax().descendants_with_tokens() {
            if let rowan::NodeOrToken::Token(token) = child
                && token.kind() == FshSyntaxKind::String
            {
                let text = token.text();
                // Remove surrounding quotes
                if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                    return Some(text[1..text.len() - 1].to_string());
                } else {
                    return Some(text.to_string());
                }
            }
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{ValueSetConcept, ValueSetFilter, ValueSetInclude};

    fn create_test_exporter() -> ValueSetExporter {
        ValueSetExporter {
            session: Arc::new(crate::canonical::DefinitionSession::for_testing()),
            base_url: "http://example.org/fhir".to_string(),
            version: None,
            status: None,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_exporter_creation() {
        let exporter = create_test_exporter();
        assert_eq!(exporter.base_url, "http://example.org/fhir");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_extract_string_value() {
        let exporter = create_test_exporter();

        assert_eq!(
            exporter.extract_string_value("\"Hello World\""),
            "Hello World"
        );
        assert_eq!(
            exporter.extract_string_value("'Hello World'"),
            "Hello World"
        );
        assert_eq!(exporter.extract_string_value("NoQuotes"), "NoQuotes");
        assert_eq!(exporter.extract_string_value("  \"Spaces\"  "), "Spaces");
    }

    #[test]
    fn test_valueset_resource_creation() {
        let vs = ValueSetResource::new(
            "http://example.org/fhir/ValueSet/test-vs",
            "TestVS",
            "draft",
        );

        assert_eq!(vs.resource_type, "ValueSet");
        assert_eq!(vs.url, "http://example.org/fhir/ValueSet/test-vs");
        assert_eq!(vs.name, "TestVS");
        assert_eq!(vs.status, "draft");
        assert!(vs.id.is_none());
        assert!(vs.compose.is_none());
    }

    #[test]
    fn test_valueset_compose_add_include() {
        let mut compose = ValueSetCompose::new();
        let include = ValueSetInclude::from_system("http://loinc.org");

        compose.add_include(include);

        assert!(compose.include.is_some());
        assert_eq!(compose.include.as_ref().unwrap().len(), 1);
        assert_eq!(
            compose.include.as_ref().unwrap()[0]
                .system
                .as_ref()
                .unwrap(),
            "http://loinc.org"
        );
    }

    #[test]
    fn test_valueset_include_with_concept() {
        let mut include = ValueSetInclude::from_system("http://loinc.org");
        include.add_concept(ValueSetConcept::with_display("12345-6", "Blood pressure"));

        assert!(include.concept.is_some());
        assert_eq!(include.concept.as_ref().unwrap().len(), 1);
        assert_eq!(include.concept.as_ref().unwrap()[0].code, "12345-6");
        assert_eq!(
            include.concept.as_ref().unwrap()[0]
                .display
                .as_ref()
                .unwrap(),
            "Blood pressure"
        );
    }

    #[test]
    fn test_valueset_filter_is_a() {
        let filter = ValueSetFilter::is_a("123456");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "is-a");
        assert_eq!(filter.value, "123456");
    }

    #[test]
    fn test_valueset_filter_regex() {
        let filter = ValueSetFilter::regex("^(A|B).*");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "regex");
        assert_eq!(filter.value, "^(A|B).*");
    }

    #[test]
    fn test_valueset_filter_descendent_of() {
        let filter = ValueSetFilter::descendent_of("12345");

        assert_eq!(filter.property, "concept");
        assert_eq!(filter.op, "descendant-of");
        assert_eq!(filter.value, "12345");
    }

    #[test]
    fn test_valueset_include_with_filter() {
        let mut include = ValueSetInclude::from_system("http://snomed.info/sct");
        include.add_filter(ValueSetFilter::is_a("123456"));

        assert!(include.filter.is_some());
        assert_eq!(include.filter.as_ref().unwrap().len(), 1);
        assert_eq!(include.filter.as_ref().unwrap()[0].op, "is-a");
    }

    #[test]
    fn test_valueset_compose_add_exclude() {
        let mut compose = ValueSetCompose::new();
        let exclude = ValueSetInclude::from_system("http://loinc.org");

        compose.add_exclude(exclude);

        assert!(compose.exclude.is_some());
        assert_eq!(compose.exclude.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_valueset_include_from_valueset() {
        let include = ValueSetInclude::from_valueset("http://example.org/vs/vital-signs");

        assert!(include.value_set.is_some());
        assert_eq!(
            include.value_set.as_ref().unwrap()[0],
            "http://example.org/vs/vital-signs"
        );
        assert!(include.system.is_none());
    }
}
