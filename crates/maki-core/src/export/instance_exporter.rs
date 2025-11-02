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
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
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
use crate::canonical::DefinitionSession;
use crate::cst::ast::{FixedValueRule, Instance, Rule};
use crate::semantic::FishingContext;
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
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
/// # use maki_core::export::InstanceExporter;
/// # use maki_core::canonical::DefinitionSession;
/// # use std::sync::Arc;
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
    /// # use maki_core::export::InstanceExporter;
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
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
        Ok(Self {
            session,
            fishing_context: None,
            base_url,
            current_indices: HashMap::new(),
            instance_registry: HashMap::new(),
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

    /// Register an instance for reference resolution
    pub fn register_instance(&mut self, name: String, json: JsonValue) {
        self.instance_registry.insert(name, json);
    }

    /// Get a registered instance by name
    pub fn get_instance(&self, name: &str) -> Option<&JsonValue> {
        self.instance_registry.get(name)
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
    /// # use maki_core::export::InstanceExporter;
    /// # use maki_core::cst::ast::Instance;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let exporter: InstanceExporter = todo!();
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

        // Create base resource
        let mut resource = serde_json::json!({
            "resourceType": instance_of,
            "id": name,
        });

        // Apply rules
        for rule in instance.rules() {
            self.apply_rule(&mut resource, &rule).await?;
        }

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
            | Rule::Mapping(_) => {
                // These rules don't apply to instances
                trace!("Skipping contains/only/obeys rule in instance");
            }
        }
        Ok(())
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

        let value_str = rule.value().ok_or_else(|| {
            ExportError::InvalidValue("Missing value in fixed value rule".to_string())
        })?;

        trace!("Applying assignment: {} = {}", path, value_str);

        // Parse the path into segments
        let segments = self.parse_path(&path)?;

        // Convert value string to JSON
        let json_value = self.convert_value(&value_str).await?;

        // Navigate and set the value
        self.set_value_at_path(resource, &segments, json_value)?;

        Ok(())
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
    fn set_value_at_path(
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
                        if let JsonValue::Object(obj) = current_value {
                            // If the field already exists and both are objects, merge them
                            if let Some(existing) = obj.get_mut(field) {
                                if existing.is_object() && value.is_object() {
                                    self.merge_objects(existing, &value);
                                } else {
                                    *existing = value.clone();
                                }
                            } else {
                                obj.insert(field.clone(), value.clone());
                            }
                            return Ok(());
                        } else {
                            return Err(ExportError::InvalidPath {
                                path: field.clone(),
                                resource: "".to_string(),
                            });
                        }
                    } else {
                        // Navigate or create intermediate object
                        if let JsonValue::Object(obj) = current_value {
                            if !obj.contains_key(field) {
                                obj.insert(field.clone(), JsonValue::Object(Map::new()));
                            }
                            obj.get_mut(field).unwrap()
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
                            self.find_or_create_slice(arr, slice, field)?
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
                                self.merge_objects(&mut arr[actual_index], &value);
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
    /// For extensions, this matches by the `url` field.
    /// For other slices, this uses the `_sliceName` internal field.
    fn find_or_create_slice(
        &self,
        arr: &mut Vec<JsonValue>,
        slice_name: &str,
        field: &str,
    ) -> Result<usize, ExportError> {
        // For extensions, match by URL
        if field == "extension" || field == "modifierExtension" {
            // Try to find existing extension with this URL
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(url) = elem.get("url").and_then(|u| u.as_str()) {
                    if url == slice_name {
                        return Ok(idx);
                    }
                }
            }
            // Not found, create new element at end
            let idx = arr.len();
            arr.push(serde_json::json!({
                "url": slice_name,
                "_sliceName": slice_name
            }));
            Ok(idx)
        } else {
            // For other slices, match by _sliceName
            for (idx, elem) in arr.iter().enumerate() {
                if let Some(name) = elem.get("_sliceName").and_then(|n| n.as_str()) {
                    if name == slice_name {
                        return Ok(idx);
                    }
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

    /// Merge two JSON objects, combining their properties
    ///
    /// This is used when assigning multiple properties to the same object.
    /// For example:
    /// ```fsh
    /// * contact.name.text = "John Doe"
    /// * contact.name.family = "Doe"
    /// ```
    /// Both rules target `contact.name`, so we merge the values.
    fn merge_objects(&self, target: &mut JsonValue, source: &JsonValue) {
        if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
            for (key, value) in source_obj {
                if let Some(existing) = target_obj.get_mut(key) {
                    if existing.is_object() && value.is_object() {
                        self.merge_objects(existing, value);
                    } else if existing.is_array() && value.is_array() {
                        // Merge arrays by appending unique elements
                        if let (Some(existing_arr), Some(source_arr)) =
                            (existing.as_array_mut(), value.as_array())
                        {
                            for item in source_arr {
                                if !existing_arr.contains(item) {
                                    existing_arr.push(item.clone());
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
            return self.parse_codeable_concept(trimmed);
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
            return self.parse_reference(trimmed).await;
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
    /// Format: system#code "display"
    fn parse_codeable_concept(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Split into system#code and display
        let parts: Vec<&str> = value.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let system = parts[0].trim();
        let code_and_display = parts[1];

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
                "system": system,
                "code": code
            }]
        });

        if let Some(display_text) = display {
            codeable_concept["coding"][0]["display"] = JsonValue::String(display_text.to_string());
        }

        Ok(codeable_concept)
    }

    /// Parse Quantity from FSH notation
    /// Format: 70 'kg' or 5.5 'cm'
    fn parse_quantity(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Extract value and unit
        let parts: Vec<&str> = value.splitn(2, '\'').collect();
        if parts.len() < 2 {
            return Ok(JsonValue::String(value.to_string()));
        }

        let value_str = parts[0].trim();
        let unit_with_quote = parts[1];
        let unit = unit_with_quote.trim_end_matches('\'').trim();

        // Parse numeric value
        let numeric_value = if let Ok(num) = value_str.parse::<i64>() {
            serde_json::json!(num)
        } else if let Ok(num) = value_str.parse::<f64>() {
            serde_json::json!(num)
        } else {
            return Ok(JsonValue::String(value.to_string()));
        };

        // Build Quantity
        let mut quantity = serde_json::json!({
            "value": numeric_value,
            "unit": unit
        });

        // Add system and code for UCUM units
        if unit == "kg" || unit == "g" || unit == "cm" || unit == "m" || unit == "s" {
            quantity["system"] = JsonValue::String("http://unitsofmeasure.org".to_string());
            quantity["code"] = JsonValue::String(unit.to_string());
        }

        Ok(quantity)
    }

    /// Parse Reference from FSH notation
    /// Format: Reference(Patient/example) or Reference(resourceId)
    async fn parse_reference(&self, value: &str) -> Result<JsonValue, ExportError> {
        // Extract reference from Reference(...)
        let ref_value = value
            .strip_prefix("Reference(")
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(value);

        // Validate the reference if fishing context is available
        if let Err(e) = self.validate_reference(ref_value).await {
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
    /// # use maki_core::export::InstanceExporter;
    /// # fn example(exporter: &InstanceExporter) -> Result<(), Box<dyn std::error::Error>> {
    /// // Validate a FHIR reference
    /// exporter.validate_reference("Patient/example")?;
    ///
    /// // Validate an instance reference
    /// exporter.validate_reference("my-patient-instance")?;
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
        }
    }

    #[test]
    fn test_parse_simple_path() {
        let exporter = create_test_exporter();
        let segments = exporter.parse_path("name.family").unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], PathSegment::Field("name".to_string()));
        assert_eq!(segments[1], PathSegment::Field("family".to_string()));
    }

    #[test]
    fn test_parse_array_path() {
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

    #[test]
    fn test_parse_append_path() {
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

    #[test]
    fn test_parse_current_path() {
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

    #[test]
    fn test_parse_nested_array_path() {
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

    #[test]
    fn test_convert_string_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("\"Hello World\"").unwrap();
        assert_eq!(value, JsonValue::String("Hello World".to_string()));
    }

    #[test]
    fn test_convert_boolean_value() {
        let exporter = create_test_exporter();
        assert_eq!(
            exporter.convert_value("true").unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(
            exporter.convert_value("false").unwrap(),
            JsonValue::Bool(false)
        );
    }

    #[test]
    fn test_convert_integer_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("42").unwrap();
        assert_eq!(value, JsonValue::Number(42.into()));
    }

    #[test]
    fn test_convert_code_value() {
        let exporter = create_test_exporter();
        let value = exporter.convert_value("#male").unwrap();
        assert_eq!(value, JsonValue::String("male".to_string()));
    }

    #[test]
    fn test_set_simple_value() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("birthDate").unwrap();
        let value = JsonValue::String("1970-01-01".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .unwrap();

        assert_eq!(resource["birthDate"], "1970-01-01");
    }

    #[test]
    fn test_set_nested_value() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("name.family").unwrap();
        let value = JsonValue::String("Doe".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .unwrap();

        assert_eq!(resource["name"]["family"], "Doe");
    }

    #[test]
    fn test_set_array_value_numeric() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        let segments = exporter.parse_path("name.given[0]").unwrap();
        let value = JsonValue::String("John".to_string());

        exporter
            .set_value_at_path(&mut resource, &segments, value)
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
    }

    #[test]
    fn test_set_array_value_append() {
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
            .unwrap();

        // Add second element
        let segments = exporter.parse_path("name.given[+]").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Jacob".to_string()),
            )
            .unwrap();

        assert_eq!(resource["name"]["given"][0], "John");
        assert_eq!(resource["name"]["given"][1], "Jacob");
    }

    #[test]
    fn test_nested_array_access() {
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
            .unwrap();

        // Set address[0].city
        let segments = exporter.parse_path("address[0].city").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Boston".to_string()),
            )
            .unwrap();

        assert_eq!(resource["address"][0]["line"][0], "123 Main St");
        assert_eq!(resource["address"][0]["city"], "Boston");
    }

    // ===== Reference Validation Tests =====

    #[test]
    fn test_validate_reference_inline_instance() {
        let mut exporter = create_test_exporter();

        // Register an inline instance
        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        // Should validate successfully
        assert!(exporter.validate_reference("my-patient").is_ok());
    }

    #[test]
    fn test_validate_reference_not_found() {
        let exporter = create_test_exporter();

        // Should fail - no fishing context and not in registry
        let result = exporter.validate_reference("nonexistent");
        assert!(result.is_err());
        if let Err(ExportError::InvalidReference { reference, reason }) = result {
            assert_eq!(reference, "nonexistent");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected InvalidReference error");
        }
    }

    #[test]
    fn test_validate_reference_with_fishing_context() {
        use crate::Location;
        use crate::semantic::{FhirResource, FshTank, Package, ResourceType};
        use std::sync::RwLock;

        let mut exporter = create_test_exporter();

        // Create fishing context with a tank containing a resource
        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let session = Arc::new(crate::canonical::DefinitionSession::for_testing());

        // Add a resource to the tank
        {
            let mut t = tank.write().unwrap();
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

        // Should validate successfully - resource is in tank
        assert!(exporter.validate_reference("PatientProfile").is_ok());
    }

    #[test]
    fn test_validate_reference_fhir_style() {
        let exporter = create_test_exporter();

        // FHIR-style references (ResourceType/id) are assumed valid
        // even without fishing context
        assert!(exporter.validate_reference("Patient/example").is_ok());
        assert!(
            exporter
                .validate_reference("Observation/vital-signs")
                .is_ok()
        );
    }

    #[test]
    fn test_parse_reference_validates() {
        let mut exporter = create_test_exporter();

        // Register an instance
        exporter.register_instance(
            "my-patient".to_string(),
            serde_json::json!({
                "resourceType": "Patient",
                "id": "my-patient"
            }),
        );

        // Parse should succeed and validate
        let result = exporter.parse_reference("Reference(my-patient)");
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "my-patient");
    }

    #[test]
    fn test_parse_reference_warns_on_invalid() {
        let exporter = create_test_exporter();

        // Parse should succeed but log warning (doesn't fail export)
        let result = exporter.parse_reference("Reference(nonexistent)");
        assert!(result.is_ok());
        let reference = result.unwrap();
        assert_eq!(reference["reference"], "nonexistent");
        // Note: Warning is logged but doesn't fail the export
    }

    #[test]
    fn test_inline_instance_resolution() {
        let mut exporter = create_test_exporter();

        // Create a patient instance
        let patient_json = serde_json::json!({
            "resourceType": "Patient",
            "id": "example-patient",
            "name": [{
                "family": "Doe",
                "given": ["John"]
            }]
        });

        exporter.register_instance("example-patient".to_string(), patient_json.clone());

        // Reference should resolve to the inline instance
        let value = exporter.convert_value("example-patient").unwrap();
        assert_eq!(value, patient_json);
    }

    #[test]
    fn test_is_instance_reference() {
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

    #[test]
    fn test_parse_slice_name_path() {
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

    #[test]
    fn test_set_extension_by_slice_name() {
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
            .unwrap();

        // Verify the extension was created with URL
        assert!(resource["extension"].is_array());
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "test value");
    }

    #[test]
    fn test_merge_object_values() {
        let mut exporter = create_test_exporter();
        let mut resource = serde_json::json!({ "resourceType": "Patient" });

        // Set first property
        let segments = exporter.parse_path("contact.name.text").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("John Doe".to_string()),
            )
            .unwrap();

        // Set second property on same object
        let segments = exporter.parse_path("contact.name.family").unwrap();
        exporter
            .set_value_at_path(
                &mut resource,
                &segments,
                JsonValue::String("Doe".to_string()),
            )
            .unwrap();

        // Verify both properties exist
        assert_eq!(resource["contact"]["name"]["text"], "John Doe");
        assert_eq!(resource["contact"]["name"]["family"], "Doe");
    }

    #[test]
    fn test_multiple_extensions_by_slice_name() {
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
            .unwrap();

        // Add second extension
        let segments = exporter
            .parse_path("extension[http://example.org/ext2].valueInteger")
            .unwrap();
        exporter
            .set_value_at_path(&mut resource, &segments, JsonValue::Number(42.into()))
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

    #[test]
    fn test_update_existing_extension() {
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
            .unwrap();

        // Verify extension has both properties
        let extensions = resource["extension"].as_array().unwrap();
        assert_eq!(extensions.len(), 1);
        assert_eq!(extensions[0]["url"], "http://example.org/ext");
        assert_eq!(extensions[0]["valueString"], "initial");
        assert_eq!(extensions[0]["id"], "ext-id");
    }
}
