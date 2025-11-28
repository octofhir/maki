//! Extension Exporter
//!
//! Exports FSH Extension definitions to FHIR StructureDefinition resources.
//! Extensions are special profiles with specific structural requirements.
//!

#![allow(dead_code)]
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
//!     Some("1.0.0".to_string()),
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
use crate::cst::ast::{
    AstNode, CardRule, ContainsRule, Extension, FixedValueRule, FlagRule, OnlyRule, Rule,
    ValueSetRule,
};
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
    /// Version from config
    version: Option<String>,
}

impl ExtensionExporter {
    /// Create a new extension exporter
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving base definitions
    /// * `base_url` - Base URL for generated extension canonical URLs
    /// * `version` - Version from configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::ExtensionExporter;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let session: Arc<DefinitionSession> = todo!();
    /// let exporter = ExtensionExporter::new(
    ///     session,
    ///     "http://example.org/fhir".to_string(),
    ///     Some("1.0.0".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        session: Arc<DefinitionSession>,
        base_url: String,
        version: Option<String>,
    ) -> Result<Self, ExportError> {
        let path_resolver = Arc::new(PathResolver::new(session.clone()));

        Ok(Self {
            session,
            path_resolver,
            base_url,
            version,
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

        // 2. Apply metadata with proper extension settings
        self.apply_metadata(&mut structure_def, extension)?;

        // 3. Make a copy of the base snapshot for comparison
        let base_snapshot = structure_def.snapshot.clone();

        // 4. Generate and set extension context definitions
        self.generate_context_definitions(&mut structure_def, extension)
            .await?;

        // 5. Apply all rules to snapshot with enhanced processing
        for rule in extension.rules() {
            if let Err(e) = self.apply_rule(&mut structure_def, &rule).await {
                warn!("Failed to apply rule: {}", e);
                // Continue with other rules instead of failing completely
            }
        }

        // 6. Process extension value constraints and nested extensions
        self.process_extension_constraints(&mut structure_def, extension)
            .await?;

        // 7. Ensure proper extension structure (url, value[x] or extension)
        self.ensure_extension_structure(&mut structure_def)?;

        // 8. Generate differential from changes
        if let Some(base_snap) = base_snapshot {
            structure_def.differential =
                Some(self.generate_differential(&base_snap, &structure_def));
        }

        // 9. Validate exported structure
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

    /// Apply extension metadata with proper extension settings
    fn apply_metadata(
        &self,
        structure_def: &mut StructureDefinition,
        extension: &Extension,
    ) -> Result<(), ExportError> {
        let extension_name = extension
            .name()
            .ok_or_else(|| ExportError::MissingRequiredField("name".to_string()))?;

        // Prefer explicit Id for canonical/url; fall back to name if missing
        let canonical_id = extension
            .id()
            .and_then(|id| id.value())
            .unwrap_or_else(|| extension_name.clone());

        // Update metadata fields with proper extension configuration
        structure_def.name = extension_name.clone();
        structure_def.id = Some(canonical_id.clone());
        structure_def.url = format!("{}/StructureDefinition/{}", self.base_url, canonical_id);
        structure_def.kind = StructureDefinitionKind::ComplexType;
        structure_def.type_field = "Extension".to_string();
        structure_def.base_definition =
            Some("http://hl7.org/fhir/StructureDefinition/Extension".to_string());
        structure_def.derivation = Some("constraint".to_string());
        structure_def.is_abstract = false;

        // Set status to active for extensions (SUSHI parity)
        structure_def.status = "active".to_string();

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

        // Set version from config if available (SUSHI parity)
        structure_def.version = self.version.clone();

        // Set FHIR version from session
        if let Some(release) = self.session.releases().first() {
            structure_def.fhir_version = Some(release.to_version_string().to_string());
        }

        debug!("Applied extension metadata for: {}", extension_name);
        Ok(())
    }

    /// Generate and set extension context definitions
    ///
    /// Parses caret rules to determine where the extension can be used.
    /// Supports element, extension, and resource context types.
    /// Handles both single and multi-line context definitions.
    async fn generate_context_definitions(
        &self,
        structure_def: &mut StructureDefinition,
        extension: &Extension,
    ) -> Result<(), ExportError> {
        let mut contexts = Vec::new();
        let mut found_context_rules = false;

        // First pass: collect all context-related rules and group them
        let mut context_entries: std::collections::HashMap<
            String,
            (Option<String>, Option<String>),
        > = std::collections::HashMap::new();
        let mut current_index = 0;
        let mut last_plus_index = 0;

        debug!("Starting context rule processing for extension");

        for rule in extension.rules() {
            if let Rule::CaretValue(caret_rule) = rule {
                if let Some(path) = caret_rule.caret_path() {
                    let path_str = path.as_string();

                    debug!("Processing caret rule path: '{}'", path_str);

                    // Handle context type rules: ^context[0].type, ^context[+].type, etc.
                    if path_str.starts_with("context") && path_str.contains(".type") {
                        if let Some(value) = caret_rule.value() {
                            let context_type = value.trim_start_matches('#');
                            let raw_index = self.extract_context_index(&path_str);

                            // Handle special SUSHI indices
                            let index_key = match raw_index.as_str() {
                                "+" => {
                                    // [+] means "add new entry"
                                    let key = format!("idx_{}", current_index);
                                    last_plus_index = current_index;
                                    current_index += 1;
                                    key
                                }
                                "=" => {
                                    // [=] means "use the last [+] index"
                                    format!("idx_{}", last_plus_index)
                                }
                                _ => {
                                    // Numeric or default index
                                    if raw_index.parse::<usize>().is_ok() {
                                        format!("idx_{}", raw_index)
                                    } else {
                                        raw_index
                                    }
                                }
                            };

                            let entry = context_entries
                                .entry(index_key.clone())
                                .or_insert((None, None));
                            entry.0 = Some(context_type.to_string());
                            found_context_rules = true;

                            debug!(
                                "Found context type rule: {} = {} (index_key: {})",
                                path_str, context_type, index_key
                            );
                        }
                    }
                    // Handle context expression rules: ^context[0].expression, ^context[=].expression, etc.
                    else if path_str.starts_with("context") && path_str.contains(".expression") {
                        if let Some(value) = caret_rule.value() {
                            let expression = value.trim_matches('"');
                            let raw_index = self.extract_context_index(&path_str);

                            // Handle special SUSHI indices (same logic as above)
                            let index_key = match raw_index.as_str() {
                                "+" => {
                                    let key = format!("idx_{}", current_index);
                                    last_plus_index = current_index;
                                    current_index += 1;
                                    key
                                }
                                "=" => {
                                    format!("idx_{}", last_plus_index)
                                }
                                _ => {
                                    if raw_index.parse::<usize>().is_ok() {
                                        format!("idx_{}", raw_index)
                                    } else {
                                        raw_index
                                    }
                                }
                            };

                            let entry = context_entries
                                .entry(index_key.clone())
                                .or_insert((None, None));
                            entry.1 = Some(expression.to_string());
                            found_context_rules = true;

                            debug!(
                                "Found context expression rule: {} = {} (index_key: {})",
                                path_str, expression, index_key
                            );
                        }
                    }
                    // Handle direct context expression rules: ^context = "Patient"
                    else if path_str == "context"
                        && let Some(expression) = caret_rule.value()
                    {
                        let clean_expression = expression.trim_matches('"');

                        // Determine context type based on expression
                        let context = if clean_expression.starts_with("http://") {
                            StructureDefinitionContext::extension(clean_expression)
                        } else if clean_expression.contains('.') {
                            StructureDefinitionContext::element(clean_expression)
                        } else {
                            // Assume it's a resource type
                            StructureDefinitionContext::element(clean_expression)
                        };

                        contexts.push(context);
                        found_context_rules = true;
                    }
                }
            } else if let Rule::FixedValue(fixed_rule) = rule {
                // Handle fixed value rules that might be context rules (e.g., * ^context[+].type = #element)
                if let Some(path) = fixed_rule.path() {
                    let path_str = path.as_string();
                    debug!("Processing fixed value rule path: '{}'", path_str);

                    // Check if this is a context rule (starts with ^context)
                    if path_str.starts_with("^context") {
                        let clean_path = path_str.trim_start_matches('^');

                        // Handle context type rules: ^context[0].type, ^context[+].type, etc.
                        if clean_path.starts_with("context") && clean_path.contains(".type") {
                            if let Some(value) = fixed_rule.value() {
                                let context_type = value.trim_start_matches('#');
                                let raw_index = self.extract_context_index(clean_path);

                                // Handle special SUSHI indices
                                let index_key = match raw_index.as_str() {
                                    "+" => {
                                        // [+] means "add new entry"
                                        let key = format!("idx_{}", current_index);
                                        last_plus_index = current_index;
                                        current_index += 1;
                                        key
                                    }
                                    "=" => {
                                        // [=] means "use the last [+] index"
                                        format!("idx_{}", last_plus_index)
                                    }
                                    _ => {
                                        // Numeric or default index
                                        if raw_index.parse::<usize>().is_ok() {
                                            format!("idx_{}", raw_index)
                                        } else {
                                            raw_index
                                        }
                                    }
                                };

                                let entry = context_entries
                                    .entry(index_key.clone())
                                    .or_insert((None, None));
                                entry.0 = Some(context_type.to_string());
                                found_context_rules = true;

                                debug!(
                                    "Found context type rule via FixedValue: {} = {} (index_key: {})",
                                    path_str, context_type, index_key
                                );
                            }
                        }
                        // Handle context expression rules: ^context[0].expression, ^context[=].expression, etc.
                        else if clean_path.starts_with("context")
                            && clean_path.contains(".expression")
                        {
                            if let Some(value) = fixed_rule.value() {
                                let expression = value.trim_matches('"');
                                let raw_index = self.extract_context_index(clean_path);

                                // Handle special SUSHI indices (same logic as above)
                                let index_key = match raw_index.as_str() {
                                    "+" => {
                                        let key = format!("idx_{}", current_index);
                                        last_plus_index = current_index;
                                        current_index += 1;
                                        key
                                    }
                                    "=" => {
                                        format!("idx_{}", last_plus_index)
                                    }
                                    _ => {
                                        if raw_index.parse::<usize>().is_ok() {
                                            format!("idx_{}", raw_index)
                                        } else {
                                            raw_index
                                        }
                                    }
                                };

                                let entry = context_entries
                                    .entry(index_key.clone())
                                    .or_insert((None, None));
                                entry.1 = Some(expression.to_string());
                                found_context_rules = true;

                                debug!(
                                    "Found context expression rule via FixedValue: {} = {} (index_key: {})",
                                    path_str, expression, index_key
                                );
                            }
                        }
                        // Handle direct context expression rules: ^context = "Patient"
                        else if clean_path == "context"
                            && let Some(expression) = fixed_rule.value()
                        {
                            let clean_expression = expression.trim_matches('"');

                            // Determine context type based on expression
                            let context = if clean_expression.starts_with("http://") {
                                StructureDefinitionContext::extension(clean_expression)
                            } else if clean_expression.contains('.') {
                                StructureDefinitionContext::element(clean_expression)
                            } else {
                                // Assume it's a resource type
                                StructureDefinitionContext::element(clean_expression)
                            };

                            contexts.push(context);
                            found_context_rules = true;

                            debug!(
                                "Found direct context rule via FixedValue: {} = {}",
                                path_str, clean_expression
                            );
                        }
                    }
                }
            }
        }

        // Second pass: create contexts from collected entries
        for (index, (context_type_opt, expression_opt)) in context_entries {
            match (context_type_opt, expression_opt) {
                (Some(context_type), Some(expression)) => {
                    let context = match context_type.as_str() {
                        "element" => StructureDefinitionContext::element(&expression),
                        "extension" => StructureDefinitionContext::extension(&expression),
                        "fhirpath" => StructureDefinitionContext::fhirpath(&expression),
                        _ => {
                            warn!("Unknown context type: {}", context_type);
                            StructureDefinitionContext::element(&expression)
                        }
                    };
                    contexts.push(context);
                    debug!(
                        "Created context for index {}: type={}, expression={}",
                        index, context_type, expression
                    );
                }
                (Some(context_type), None) => {
                    warn!(
                        "Context type '{}' found for index '{}' but no expression",
                        context_type, index
                    );
                }
                (None, Some(expression)) => {
                    // Default to element type if only expression is provided
                    let context = StructureDefinitionContext::element(&expression);
                    contexts.push(context);
                    debug!(
                        "Created default element context for index {}: expression={}",
                        index, expression
                    );
                }
                (None, None) => {
                    // This shouldn't happen, but skip if it does
                    warn!("Empty context entry for index: {}", index);
                }
            }
        }

        // If no context rules found, set default context
        if !found_context_rules {
            contexts.push(StructureDefinitionContext::element("Element"));
            debug!("No context rules found, setting default context to Element");
        } else {
            debug!("Found {} context definitions", contexts.len());
        }

        // Validate context expressions
        for context in &contexts {
            self.validate_context_expression(&context.expression, &context.type_)?;
        }

        structure_def.context = Some(contexts);
        Ok(())
    }

    /// Extract context index from path string for grouping context rules
    ///
    /// Examples:
    /// - "context[0].type" -> "0"
    /// - "context[+].type" -> "+"  
    /// - "context[=].expression" -> "="
    /// - "context.type" -> "default"
    fn extract_context_index(&self, path: &str) -> String {
        if let Some(start) = path.find('[')
            && let Some(end) = path.find(']')
            && end > start
        {
            return path[start + 1..end].to_string();
        }

        // If no brackets found, use "default" as key
        "default".to_string()
    }

    /// Find the corresponding context expression for a context type rule (legacy method)
    fn find_context_expression(
        &self,
        extension: &Extension,
        type_path: &str,
    ) -> Result<String, ExportError> {
        // Extract the index from the type path (e.g., "context[0].type" -> "context[0].expression")
        let expression_path = type_path.replace(".type", ".expression");

        for rule in extension.rules() {
            if let Rule::CaretValue(caret_rule) = rule
                && let Some(path) = caret_rule.caret_path()
            {
                let path_str = path.as_string();
                if path_str == expression_path
                    && let Some(value) = caret_rule.value()
                {
                    return Ok(value.trim_matches('"').to_string());
                }
            }
        }

        // Default to Element if no expression found
        Ok("Element".to_string())
    }

    /// Validate context expression based on context type
    fn validate_context_expression(
        &self,
        expression: &str,
        context_type: &str,
    ) -> Result<(), ExportError> {
        match context_type {
            "element" => {
                // Element context should be a valid FHIR element path
                if expression.is_empty() {
                    return Err(ExportError::InvalidContextExpression(
                        "Element context expression cannot be empty".to_string(),
                    ));
                }
                // Additional validation could check if it's a valid FHIR path
            }
            "extension" => {
                // Extension context should be a canonical URL
                if !expression.starts_with("http://") && !expression.starts_with("https://") {
                    warn!(
                        "Extension context expression should be a canonical URL: {}",
                        expression
                    );
                }
            }
            "fhirpath" => {
                // FHIRPath context should be a valid FHIRPath expression
                if expression.is_empty() {
                    return Err(ExportError::InvalidContextExpression(
                        "FHIRPath context expression cannot be empty".to_string(),
                    ));
                }
                // Additional validation could parse the FHIRPath expression
            }
            _ => {
                warn!("Unknown context type for validation: {}", context_type);
            }
        }

        Ok(())
    }

    /// Process extension value constraints and nested extensions
    ///
    /// Creates ElementDefinition for extension.value[x] with type constraints
    /// and handles cardinality for extension root element.
    async fn process_extension_constraints(
        &self,
        structure_def: &mut StructureDefinition,
        extension: &Extension,
    ) -> Result<(), ExportError> {
        let mut has_value_constraints = false;
        let mut has_nested_extensions = false;
        let mut extension_cardinality: Option<(u32, String)> = None;

        // Analyze rules to determine extension type and constraints
        for rule in extension.rules() {
            match rule {
                Rule::Card(card_rule) => {
                    if let Some(path) = card_rule.path() {
                        let path_str = path.as_string();

                        // Root extension cardinality
                        if (path_str == "." || path_str.is_empty())
                            && let Some(cardinality_node) = card_rule.cardinality()
                            && let (Some(min), Some(max)) =
                                (cardinality_node.min(), cardinality_node.max())
                        {
                            extension_cardinality = Some((min, max));
                        }
                        // Value[x] constraints indicate simple extension
                        else if path_str.starts_with("value[x]") || path_str == "value" {
                            has_value_constraints = true;
                        }
                        // Extension constraints indicate complex extension
                        else if path_str.starts_with("extension") {
                            has_nested_extensions = true;
                        }
                    } else {
                        // No path means root element (.)
                        if let Some(cardinality_node) = card_rule.cardinality()
                            && let (Some(min), Some(max)) =
                                (cardinality_node.min(), cardinality_node.max())
                        {
                            extension_cardinality = Some((min, max));
                        }
                    }
                }
                Rule::Only(only_rule) => {
                    if let Some(path) = only_rule.path() {
                        let path_str = path.as_string();
                        if path_str.starts_with("value[x]") || path_str == "value" {
                            has_value_constraints = true;

                            // Create ElementDefinition for value[x] with type constraints
                            self.create_value_constraint_element(structure_def, &only_rule)
                                .await?;
                        }
                    }
                }
                Rule::Contains(contains_rule) => {
                    // Contains rules indicate nested extensions
                    has_nested_extensions = true;
                    debug!("Found ContainsRule, processing nested extensions");
                    debug!(
                        "ContainsRule syntax text: '{}'",
                        contains_rule.syntax().text()
                    );
                    debug!("ContainsRule items: {:?}", contains_rule.items());

                    // Try multiple approaches to extract extension names
                    let mut items = contains_rule.items();

                    // If the standard parser fails, try manual parsing
                    if items.is_empty() {
                        debug!("Standard parser returned empty items, trying manual parsing");
                        items = self.parse_contains_rule_manually(&contains_rule);
                    }

                    // If manual parsing also fails, try parsing from full document
                    if items.is_empty() {
                        debug!("Manual parsing failed, trying full document parsing");
                        items = self.extract_from_full_document(extension);
                    }

                    // If document parsing also fails, extract from other rules
                    if items.is_empty() {
                        debug!("Document parsing failed, extracting from other rules");
                        items = self.extract_nested_extension_names_from_rules(extension);
                    }

                    debug!("Final items to process: {:?}", items);

                    if !items.is_empty() {
                        self.process_nested_extensions_with_items(structure_def, &items)
                            .await?;
                    } else {
                        warn!("No nested extension names found in contains rule");
                    }
                }
                _ => {}
            }
        }

        // Apply root extension cardinality if specified
        if let Some((min, max)) = extension_cardinality
            && let Some(snapshot) = &mut structure_def.snapshot
            && let Some(root_element) = snapshot.element.iter_mut().find(|e| e.path == "Extension")
        {
            root_element.min = Some(min);
            root_element.max = Some(max);
        }

        // Validate extension type consistency
        if has_value_constraints && has_nested_extensions {
            warn!(
                "Extension {} has both value[x] and nested extension constraints. \
                 Extensions should be either simple (value[x]) or complex (nested extensions).",
                structure_def.name
            );
        }

        if has_value_constraints {
            debug!(
                "Extension {} is a simple extension with value[x] constraints",
                structure_def.name
            );
        } else if has_nested_extensions {
            debug!(
                "Extension {} is a complex extension with nested extensions",
                structure_def.name
            );
        }

        Ok(())
    }

    /// Create ElementDefinition for extension.value[x] with type constraints
    async fn create_value_constraint_element(
        &self,
        structure_def: &mut StructureDefinition,
        only_rule: &OnlyRule,
    ) -> Result<(), ExportError> {
        if let Some(snapshot) = &mut structure_def.snapshot {
            // Find or create Extension.value[x] element
            let value_element = snapshot
                .element
                .iter_mut()
                .find(|e| e.path == "Extension.value[x]")
                .ok_or_else(|| ExportError::ElementNotFound {
                    path: "Extension.value[x]".to_string(),
                    profile: structure_def.name.clone(),
                })?;

            // Apply type constraints from OnlyRule
            let types = only_rule.types();
            if !types.is_empty() {
                let mut element_types = Vec::new();

                for type_name in types {
                    let element_type = ElementDefinitionType::new(type_name);
                    element_types.push(element_type);
                }

                if !element_types.is_empty() {
                    value_element.type_ = Some(element_types);
                    debug!(
                        "Applied type constraints to Extension.value[x]: {:?}",
                        value_element
                            .type_
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|t| &t.code)
                            .collect::<Vec<_>>()
                    );
                }
            }
        }

        Ok(())
    }

    /// Process nested extensions from Contains rules
    ///
    /// Creates ElementDefinition entries for each nested extension
    /// and handles nested extension URL resolution and validation.
    async fn process_nested_extensions(
        &self,
        structure_def: &mut StructureDefinition,
        contains_rule: &ContainsRule,
    ) -> Result<(), ExportError> {
        if let Some(snapshot) = &mut structure_def.snapshot {
            // Get the base path for nested extensions
            let base_path = if let Some(path) = contains_rule.path() {
                format!("Extension.{}", path.as_string())
            } else {
                "Extension.extension".to_string()
            };

            // Process each contained extension name
            let items = if contains_rule.items().is_empty() {
                self.parse_contains_rule_manually(contains_rule)
            } else {
                contains_rule.items()
            };

            for name in items {
                if !name.is_empty() {
                    // Create ElementDefinition for the nested extension
                    let extension_path = format!("{}:{}", base_path, name);

                    let mut nested_element = ElementDefinition::new(extension_path.clone());

                    // Set default cardinality (will be overridden by specific rules)
                    nested_element.min = Some(0);
                    nested_element.max = Some("1".to_string());

                    // Set type to Extension
                    nested_element.type_ = Some(vec![ElementDefinitionType::new("Extension")]);

                    // Generate nested extension URL
                    let nested_url = format!("{}/StructureDefinition/{}", self.base_url, name);

                    // Set slicing discriminator for the nested extension
                    nested_element.short = Some(format!("Extension: {}", name));
                    nested_element.definition = Some(format!("Nested extension: {}", name));

                    // Add the nested extension element to snapshot
                    snapshot.element.push(nested_element);

                    // Create the nested extension's url element
                    let url_path = format!("{}.url", extension_path);
                    let mut url_element = ElementDefinition::new(url_path);
                    url_element.min = Some(1);
                    url_element.max = Some("1".to_string());
                    url_element.type_ = Some(vec![ElementDefinitionType::new("uri")]);

                    // Fix the URL value
                    let mut fixed_value = std::collections::HashMap::new();
                    fixed_value.insert(
                        "fixedUri".to_string(),
                        serde_json::Value::String(nested_url.clone()),
                    );
                    url_element.fixed = Some(fixed_value);

                    snapshot.element.push(url_element);

                    // Create the nested extension's value[x] element
                    let value_path = format!("{}.value[x]", extension_path);
                    let mut value_element = ElementDefinition::new(value_path);
                    value_element.min = Some(0);
                    value_element.max = Some("1".to_string());

                    // The specific type will be set by subsequent rules
                    snapshot.element.push(value_element);

                    // Validate the nested extension URL
                    self.validate_nested_extension_url(&nested_url, &name)?;

                    debug!(
                        "Created nested extension: {} with URL: {}",
                        name, nested_url
                    );
                }
            }

            // Ensure the base extension.extension element exists and has proper slicing
            if !snapshot.element.iter().any(|e| e.path == base_path) {
                let mut base_extension_element = ElementDefinition::new(base_path.clone());
                base_extension_element.min = Some(0);
                base_extension_element.max = Some("*".to_string());
                base_extension_element.type_ = Some(vec![ElementDefinitionType::new("Extension")]);

                // Add slicing information
                base_extension_element.short =
                    Some("Additional content defined by implementations".to_string());
                base_extension_element.definition = Some("May be used to represent additional information that is not part of the basic definition of the element.".to_string());

                snapshot.element.push(base_extension_element);
            }
        }

        Ok(())
    }

    /// Extract nested extension names from other rules in the extension
    /// This is used when the ContainsRule parser fails to extract the names
    fn extract_nested_extension_names_from_rules(&self, extension: &Extension) -> Vec<String> {
        let mut extension_names = std::collections::HashSet::new();

        // Look through all rules for patterns like "extension[subExt1].value[x]"
        for rule in extension.rules() {
            debug!("Checking rule for nested extension names: {:?}", rule);
            match rule {
                Rule::Only(only_rule) => {
                    if let Some(path) = only_rule.path() {
                        let path_str = path.as_string();
                        debug!("Checking OnlyRule path: '{}'", path_str);

                        // Look for patterns like "extension[subExt1].value[x]"
                        if let Some(ext_name) = self.extract_extension_name_from_path(&path_str) {
                            extension_names.insert(ext_name);
                        }
                    }
                }
                Rule::Card(card_rule) => {
                    if let Some(path) = card_rule.path() {
                        let path_str = path.as_string();
                        debug!("Checking CardRule path: '{}'", path_str);

                        // Look for patterns like "extension[subExt1]" or "extension[subExt1].value[x]"
                        if let Some(ext_name) = self.extract_extension_name_from_path(&path_str) {
                            extension_names.insert(ext_name);
                        }
                    }
                }
                Rule::Flag(flag_rule) => {
                    if let Some(path) = flag_rule.path() {
                        let path_str = path.as_string();
                        debug!("Checking FlagRule path: '{}'", path_str);

                        if let Some(ext_name) = self.extract_extension_name_from_path(&path_str) {
                            extension_names.insert(ext_name);
                        }
                    }
                }
                Rule::FixedValue(fixed_rule) => {
                    if let Some(path) = fixed_rule.path() {
                        let path_str = path.as_string();
                        debug!("Checking FixedValueRule path: '{}'", path_str);

                        if let Some(ext_name) = self.extract_extension_name_from_path(&path_str) {
                            extension_names.insert(ext_name);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut result: Vec<String> = extension_names.into_iter().collect();
        result.sort(); // Sort for consistent ordering
        debug!("Extracted nested extension names: {:?}", result);
        result
    }

    /// Extract extension names from the full document text
    fn extract_from_full_document(&self, extension: &Extension) -> Vec<String> {
        let mut items = Vec::new();

        // Get the full document text by traversing up to the root
        let mut current_node = extension.syntax().clone();
        while let Some(parent) = current_node.parent() {
            current_node = parent;
        }

        let full_text = current_node.text().to_string();
        debug!("Full document text for parsing: '{}'", full_text);

        // Find the contains block in the full text
        if let Some(contains_pos) = full_text.find("contains") {
            let after_contains = &full_text[contains_pos + "contains".len()..];

            // Find the end of the contains block (next rule starting with *)
            let lines: Vec<&str> = after_contains.lines().collect();
            let mut contains_block = String::new();

            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with('*') && !trimmed.starts_with("* extension contains") {
                    // This is the start of the next rule, stop here
                    break;
                }
                contains_block.push_str(line);
                contains_block.push('\n');
            }

            debug!("Extracted contains block: '{}'", contains_block);

            // Parse the contains block
            self.parse_contains_block_advanced(&contains_block, &mut items);
        }

        items
    }

    /// Extract extension name from a path like "extension[subExt1].value[x]"
    fn extract_extension_name_from_path(&self, path: &str) -> Option<String> {
        if path.starts_with("extension[")
            && path.contains(']')
            && let Some(start) = path.find('[')
            && let Some(end) = path.find(']')
            && end > start
        {
            let ext_name = &path[start + 1..end];
            if !ext_name.is_empty() && ext_name != "+" && ext_name != "=" {
                debug!(
                    "Found nested extension name from path '{}': '{}'",
                    path, ext_name
                );
                return Some(ext_name.to_string());
            }
        }
        None
    }

    /// Parse contains rule manually by examining the full text and child nodes
    fn parse_contains_rule_manually(&self, contains_rule: &ContainsRule) -> Vec<String> {
        let mut items = Vec::new();

        // Get the full text including child nodes
        let full_text = contains_rule.syntax().text().to_string();
        debug!("Full contains rule text: '{}'", full_text);

        // Try to get the parent node and look for the full rule text
        if let Some(parent) = contains_rule.syntax().parent() {
            let parent_text = parent.text().to_string();
            debug!("Parent node text: '{}'", parent_text);

            // Look for the contains pattern in the parent text
            if let Some(contains_start) = parent_text.find("contains") {
                let after_contains = &parent_text[contains_start + "contains".len()..];
                debug!("After contains in parent: '{}'", after_contains);

                // Extract extension names from the contains block
                self.extract_extension_names_from_text(after_contains, &mut items);
            }
        }

        // If we still don't have items, try a more sophisticated approach
        if items.is_empty() {
            debug!("No items found in parent, trying advanced parsing...");

            // Look for the pattern in the parent's full text more carefully
            if let Some(parent) = contains_rule.syntax().parent() {
                let parent_text = parent.text().to_string();

                // Find the contains rule and extract everything after it until the next rule
                if let Some(contains_pos) = parent_text.find("contains") {
                    let after_contains = &parent_text[contains_pos + "contains".len()..];

                    // Look for the next rule marker (starts with *)
                    let next_rule_pos = after_contains.find("\n*").unwrap_or(after_contains.len());
                    let contains_block = &after_contains[..next_rule_pos];

                    debug!("Contains block: '{}'", contains_block);

                    // Parse the contains block more carefully
                    self.parse_contains_block_advanced(contains_block, &mut items);
                }
            }
        }

        // If we still don't have items, try looking at sibling nodes
        if items.is_empty() {
            debug!("No items found with advanced parsing, checking siblings...");

            // Look at next siblings for the extension definitions
            let mut current = contains_rule.syntax().clone();
            while let Some(sibling) = current.next_sibling() {
                let sibling_text = sibling.text().to_string();
                println!("Sibling text: '{}'", sibling_text);

                // Look for extension names in sibling text
                self.extract_extension_names_from_text(&sibling_text, &mut items);

                current = sibling;

                // Stop if we find "and" or reach the end of the contains block
                if sibling_text.trim().is_empty() || sibling_text.contains("*") {
                    break;
                }
            }
        }

        println!("Final extracted items: {:?}", items);
        items
    }

    /// Parse contains block with advanced logic to handle multiline rules
    fn parse_contains_block_advanced(&self, contains_block: &str, items: &mut Vec<String>) {
        debug!("Parsing contains block: '{}'", contains_block);

        // Clean up the text - remove extra whitespace and newlines
        let cleaned = contains_block.replace(['\n', '\r'], " ");
        let cleaned = cleaned.trim();

        // Split by "and" to get individual extension definitions
        for part in cleaned.split("and") {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            debug!("Processing contains part: '{}'", part);

            // Extract the extension name (first word before any cardinality or flags)
            if let Some(first_word) = part.split_whitespace().next() {
                let first_word = first_word.trim();

                // Skip empty words, cardinality patterns, and flags
                if !first_word.is_empty()
                    && !first_word.contains("..")  // Skip cardinality like "0..1"
                    && first_word != "MS"          // Skip MustSupport flag
                    && !first_word.starts_with("//") // Skip comments
                    && first_word != "contains"
                // Skip the contains keyword itself
                {
                    items.push(first_word.to_string());
                    debug!(
                        "Extracted extension name from contains block: '{}'",
                        first_word
                    );
                }
            }
        }
    }

    /// Extract extension names from text containing extension definitions
    fn extract_extension_names_from_text(&self, text: &str, items: &mut Vec<String>) {
        debug!("Extracting extension names from text: '{}'", text);

        // Clean up the text - remove newlines and extra whitespace
        let cleaned_text = text.replace(['\n', '\r'], " ");
        let cleaned_text = cleaned_text.trim();

        // Split by "and" and extract extension names
        for part in cleaned_text.split("and") {
            let part = part.trim();
            if !part.is_empty() {
                debug!("Processing part: '{}'", part);

                // Extract the first word (extension name) before any cardinality or flags
                if let Some(first_word) = part.split_whitespace().next() {
                    let first_word = first_word.trim();
                    if !first_word.is_empty()
                        && !first_word.starts_with('*')
                        && first_word != "contains"
                        && !first_word.contains("..")  // Skip cardinality
                        && first_word != "MS"         // Skip flags
                        && !first_word.starts_with("//")
                    // Skip comments
                    {
                        items.push(first_word.to_string());
                        debug!("Extracted extension name: '{}'", first_word);
                    }
                }
            }
        }
    }

    /// Process nested extensions with a list of extension names
    async fn process_nested_extensions_with_items(
        &self,
        structure_def: &mut StructureDefinition,
        items: &[String],
    ) -> Result<(), ExportError> {
        if let Some(snapshot) = &mut structure_def.snapshot {
            let base_path = "Extension.extension".to_string();

            for name in items {
                if !name.is_empty() {
                    // Create ElementDefinition for the nested extension
                    let extension_path = format!("{}:{}", base_path, name);

                    let mut nested_element = ElementDefinition::new(extension_path.clone());

                    // Set default cardinality (will be overridden by specific rules)
                    nested_element.min = Some(0);
                    nested_element.max = Some("1".to_string());

                    // Set type to Extension
                    nested_element.type_ = Some(vec![ElementDefinitionType::new("Extension")]);

                    // Generate nested extension URL
                    let nested_url = format!("{}/StructureDefinition/{}", self.base_url, name);

                    // Set slicing discriminator for the nested extension
                    nested_element.short = Some(format!("Extension: {}", name));
                    nested_element.definition = Some(format!("Nested extension: {}", name));

                    // Add the nested extension element to snapshot
                    snapshot.element.push(nested_element);

                    // Create the nested extension's url element
                    let url_path = format!("{}.url", extension_path);
                    let mut url_element = ElementDefinition::new(url_path);
                    url_element.min = Some(1);
                    url_element.max = Some("1".to_string());
                    url_element.type_ = Some(vec![ElementDefinitionType::new("uri")]);

                    // Fix the URL value
                    let mut fixed_value = std::collections::HashMap::new();
                    fixed_value.insert(
                        "fixedUri".to_string(),
                        serde_json::Value::String(nested_url.clone()),
                    );
                    url_element.fixed = Some(fixed_value);

                    snapshot.element.push(url_element);

                    // Create the nested extension's value[x] element
                    let value_path = format!("{}.value[x]", extension_path);
                    let mut value_element = ElementDefinition::new(value_path);
                    value_element.min = Some(0);
                    value_element.max = Some("1".to_string());

                    // The specific type will be set by subsequent rules
                    snapshot.element.push(value_element);

                    // Validate the nested extension URL
                    self.validate_nested_extension_url(&nested_url, name)?;

                    debug!(
                        "Created nested extension: {} with URL: {}",
                        name, nested_url
                    );
                }
            }

            // Ensure the base extension.extension element exists and has proper slicing
            if !snapshot.element.iter().any(|e| e.path == base_path) {
                let mut base_extension_element = ElementDefinition::new(base_path.clone());
                base_extension_element.min = Some(0);
                base_extension_element.max = Some("*".to_string());
                base_extension_element.type_ = Some(vec![ElementDefinitionType::new("Extension")]);

                // Add slicing information
                base_extension_element.short =
                    Some("Additional content defined by implementations".to_string());
                base_extension_element.definition = Some("May be used to represent additional information that is not part of the basic definition of the element.".to_string());

                snapshot.element.push(base_extension_element);
            }
        }

        Ok(())
    }

    /// Validate nested extension URL resolution
    fn validate_nested_extension_url(&self, url: &str, name: &str) -> Result<(), ExportError> {
        if url.is_empty() {
            return Err(ExportError::InvalidValue(format!(
                "Nested extension URL cannot be empty for extension: {}",
                name
            )));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            warn!(
                "Nested extension URL should be a valid URI: {} for extension: {}",
                url, name
            );
        }

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
            Rule::AddElement(_)
            | Rule::Contains(_)
            | Rule::Only(_)
            | Rule::Obeys(_)
            | Rule::Mapping(_)
            | Rule::CaretValue(_)
            | Rule::CodeCaretValue(_)
            | Rule::Insert(_)
            | Rule::CodeInsert(_) => {
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

        let cardinality_node = rule
            .cardinality()
            .ok_or_else(|| ExportError::InvalidCardinality("missing".to_string()))?;

        trace!(
            "Applying cardinality rule: {} {}",
            path_str, cardinality_node
        );

        // Use structured cardinality access
        let min = cardinality_node
            .min()
            .ok_or_else(|| ExportError::InvalidCardinality("missing min".to_string()))?;
        let max = cardinality_node
            .max()
            .ok_or_else(|| ExportError::InvalidCardinality("missing max".to_string()))?;

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
        let extension_name = structure_def.name.clone();

        let element = structure_def.find_element_mut(&full_path).ok_or_else(|| {
            ExportError::ElementNotFound {
                path: full_path.clone(),
                profile: extension_name,
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
        if path.contains('.')
            && let Some(first_segment) = path.split('.').next()
            && first_segment == structure_def.type_field
        {
            return Ok(path.to_string());
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
