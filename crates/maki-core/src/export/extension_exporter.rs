//! Extension Exporter
//!
//! Exports FSH Extension definitions to FHIR StructureDefinition resources.
//! Extensions are special profiles with specific structural requirements.
//!
//! # FHIR Extension Structure
//!
//! Extensions are StructureDefinitions with:
//! - `kind`: "complex-type"
//! - `type`: "Extension"
//! - `baseDefinition`: "http://hl7.org/fhir/StructureDefinition/Extension"
//! - `derivation`: "constraint"
//! - `context`: Where the extension can be used
//! - Special element structure:
//!   - `Extension.url` (required, fixed to extension's canonical URL)
//!   - Either `Extension.value[x]` OR `Extension.extension` (mutually exclusive)
//!
//! # Simple vs Complex Extensions
//!
//! - **Simple Extension**: Has `value[x]` element with a single data type
//! - **Complex Extension**: Has `extension` element with nested extensions (no value[x])
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::ExtensionExporter;
//! use maki_core::cst::ast::Extension;
//! use maki_core::canonical::DefinitionSession;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let session: Arc<DefinitionSession> = todo!();
//! let exporter = ExtensionExporter::new(
//!     session,
//!     "http://example.org/fhir".to_string(),
//! ).await?;
//!
//! // Parse extension from FSH
//! let extension: Extension = todo!();
//!
//! // Export to StructureDefinition
//! let structure_def = exporter.export(&extension).await?;
//! # Ok(())
//! # }
//! ```

use super::ExportError;
use super::fhir_types::*;
use crate::canonical::DefinitionSession;
use crate::cst::ast::{CardRule, Extension, FixedValueRule, FlagRule, Rule, ValueSetRule};
use crate::semantic::path_resolver::PathResolver;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace, warn};

/// Extension exporter
///
/// Transforms FSH Extension AST nodes into FHIR StructureDefinition resources.
pub struct ExtensionExporter {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
    /// Path resolver for finding elements
    #[allow(dead_code)]
    path_resolver: Arc<PathResolver>,
    /// Base URL for generated extensions
    base_url: String,
}

impl ExtensionExporter {
    /// Create a new extension exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving base definitions
    /// * `base_url` - Base URL for generated extension canonical URLs
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use maki_core::export::ExtensionExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = ExtensionExporter::new(
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
        })
    }

    /// Export an Extension to StructureDefinition
    ///
    /// # Arguments
    ///
    /// * `extension` - FSH Extension AST node
    ///
    /// # Returns
    ///
    /// A FHIR StructureDefinition with proper extension structure
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Extension base not found
    /// - Required fields missing
    /// - Rule application fails
    pub async fn export(&self, extension: &Extension) -> Result<StructureDefinition, ExportError> {
        let extension_name = extension
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("extension name".to_string()))?;

        debug!("Exporting extension: {}", extension_name);

        // 1. Get base Extension StructureDefinition
        let mut structure_def = self.get_base_extension_definition().await?;

        // 2. Apply metadata
        self.apply_metadata(&mut structure_def, extension)?;

        // 3. Make a copy of the base snapshot for comparison
        let base_snapshot = structure_def.snapshot.clone();

        // 4. Set extension context
        self.set_extension_context(&mut structure_def, extension)?;

        // 5. Apply all rules to snapshot
        for rule in extension.rules() {
            if let Err(e) = self.apply_rule(&mut structure_def, &rule).await {
                warn!("Failed to apply rule: {}", e);
                // Continue with other rules instead of failing completely
            }
        }

        // 6. Ensure proper extension structure (url, value[x] or extension)
        self.ensure_extension_structure(&mut structure_def)?;

        // 7. Generate differential from changes
        if let Some(base_snap) = base_snapshot {
            structure_def.differential =
                Some(self.generate_differential(&base_snap, &structure_def));
        }

        // 8. Validate exported structure
        self.validate_extension_structure(&structure_def)?;

        debug!("Successfully exported extension: {}", extension_name);
        Ok(structure_def)
    }

    /// Get base Extension StructureDefinition
    async fn get_base_extension_definition(&self) -> Result<StructureDefinition, ExportError> {
        let canonical_url = "http://hl7.org/fhir/StructureDefinition/Extension";

        debug!("Resolving base Extension: {}", canonical_url);

        let resource = self.session.resolve(canonical_url).await.map_err(|e| {
            ExportError::ParentNotFound(format!("Extension base definition: {}", e))
        })?;

        // Parse JSON into StructureDefinition
        let structure_def: StructureDefinition =
            serde_json::from_value((*resource.content).clone()).map_err(|e| {
                ExportError::CanonicalError(format!(
                    "Failed to parse base Extension StructureDefinition: {}",
                    e
                ))
            })?;

        debug!("Resolved base Extension: {}", structure_def.url);
        Ok(structure_def)
    }

    /// Apply extension metadata
    fn apply_metadata(
        &self,
        structure_def: &mut StructureDefinition,
        extension: &Extension,
    ) -> Result<(), ExportError> {
        let extension_name = extension
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("name".to_string()))?;

        // Update metadata fields
        structure_def.name = extension_name.clone();
        structure_def.url = format!("{}/Extension/{}", self.base_url, extension_name);
        structure_def.kind = StructureDefinitionKind::ComplexType;
        structure_def.type_field = "Extension".to_string();
        structure_def.base_definition =
            Some("http://hl7.org/fhir/StructureDefinition/Extension".to_string());
        structure_def.derivation = Some("constraint".to_string());
        structure_def.is_abstract = false;

        // Optional metadata
        if let Some(id_clause) = extension.id()
            && let Some(id) = id_clause.value()
        {
            structure_def.id = Some(id);
        }

        if let Some(title_clause) = extension.title()
            && let Some(title) = title_clause.value()
        {
            structure_def.title = Some(title);
        }

        if let Some(desc_clause) = extension.description()
            && let Some(desc) = desc_clause.value()
        {
            structure_def.description = Some(desc);
        }

        Ok(())
    }

    /// Set extension context from caret rules
    ///
    /// Looks for rules like:
    /// - `^context[+].type = #element`
    /// - `^context[=].expression = "Patient"`
    fn set_extension_context(
        &self,
        structure_def: &mut StructureDefinition,
        _extension: &Extension,
    ) -> Result<(), ExportError> {
        // TODO: Parse caret rules for context
        // For now, set default context to Element (can be used anywhere)

        // Default context: can be used on any element
        let default_context = vec![StructureDefinitionContext::element("Element")];

        structure_def.context = Some(default_context);
        debug!("Setting default extension context to Element");

        Ok(())
    }

    /// Ensure proper extension structure
    ///
    /// Extensions must have:
    /// - Extension.url (fixed to canonical URL)
    /// - Either Extension.value[x] OR Extension.extension (but not both)
    fn ensure_extension_structure(
        &self,
        structure_def: &mut StructureDefinition,
    ) -> Result<(), ExportError> {
        // Check if we have a snapshot
        let snapshot = structure_def
            .snapshot
            .as_mut()
            .ok_or_else(|| ExportError::MissingRequiredField("snapshot".to_string()))?;

        // Find Extension.url element and fix it to this extension's URL
        if let Some(url_element) = snapshot
            .element
            .iter_mut()
            .find(|e| e.path == "Extension.url")
        {
            let mut fixed = HashMap::new();
            fixed.insert(
                "fixedUri".to_string(),
                JsonValue::String(structure_def.url.clone()),
            );
            url_element.fixed = Some(fixed);
            debug!("Fixed Extension.url to {}", structure_def.url);
        }

        // Check for value[x] constraints to determine if simple or complex extension
        let has_value_constraint = snapshot.element.iter().any(|e| {
            e.path.starts_with("Extension.value[x]") || e.path.starts_with("Extension.value")
        });

        let has_extension_constraint = snapshot
            .element
            .iter()
            .any(|e| e.path == "Extension.extension" && e.min.is_some());

        if has_value_constraint && has_extension_constraint {
            warn!(
                "Extension {} has both value[x] and extension constraints. \
                 Extensions must be either simple (value[x]) or complex (extension), not both.",
                structure_def.name
            );
        }

        if has_value_constraint {
            debug!(
                "Extension {} is a simple extension (has value[x])",
                structure_def.name
            );
        } else if has_extension_constraint {
            debug!(
                "Extension {} is a complex extension (has sub-extensions)",
                structure_def.name
            );
        }

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
                // PathRule is for type constraints
                Ok(())
            }
            Rule::Contains(_) | Rule::Only(_) | Rule::Obeys(_) => {
                // TODO: Implement these rule types for extensions
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
        let extension_name = structure_def.name.clone();

        // Find and update element
        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            warn!("Element not found in snapshot: {}", full_path);
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: extension_name,
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
        let extension_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: extension_name,
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
        let extension_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: extension_name,
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
        let extension_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: extension_name,
            }
        })?;

        // Parse value and determine type
        let mut pattern_map = HashMap::new();

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

        // For Extension, prepend "Extension."
        Ok(format!("{}.{}", structure_def.type_field, path))
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

    /// Validate extension structure
    fn validate_extension_structure(
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
        if structure_def.type_field != "Extension" {
            return Err(ExportError::InvalidType(format!(
                "Extension must have type 'Extension', got '{}'",
                structure_def.type_field
            )));
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_context_creation() {
        let ctx = StructureDefinitionContext::element("Patient");
        assert_eq!(ctx.type_, "element");
        assert_eq!(ctx.expression, "Patient");

        let ctx = StructureDefinitionContext::extension("http://example.org/Extension/myExt");
        assert_eq!(ctx.type_, "extension");
        assert_eq!(ctx.expression, "http://example.org/Extension/myExt");
    }

    // Integration tests would go here, similar to profile_exporter tests
    // They require a real DefinitionSession which is tested in integration tests
}
