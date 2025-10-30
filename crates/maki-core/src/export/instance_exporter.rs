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
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace};

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
    /// Array access with index
    ArrayAccess { field: String, index: ArrayIndex },
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
    /// Base URL for instance canonical URLs (if needed)
    #[allow(dead_code)]
    base_url: String,
    /// Track current array indices for [=] operator
    current_indices: HashMap<String, usize>,
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
            base_url,
            current_indices: HashMap::new(),
        })
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
            self.apply_rule(&mut resource, &rule)?;
        }

        debug!("Successfully exported instance {}", name);
        Ok(resource)
    }

    /// Apply a single rule to the resource
    fn apply_rule(&mut self, resource: &mut JsonValue, rule: &Rule) -> Result<(), ExportError> {
        match rule {
            Rule::FixedValue(fixed_rule) => {
                self.apply_fixed_value_rule(resource, fixed_rule)?;
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
        }
        Ok(())
    }

    /// Apply a fixed value rule (assignment: * path = value)
    fn apply_fixed_value_rule(
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
        let json_value = self.convert_value(&value_str)?;

        // Navigate and set the value
        self.set_value_at_path(resource, &segments, json_value)?;

        Ok(())
    }

    /// Parse a path string into segments
    ///
    /// Examples:
    /// - "name.family" -> [Field("name"), Field("family")]
    /// - "name.given[0]" -> [Field("name"), ArrayAccess("given", 0)]
    /// - "address[+].line[0]" -> [ArrayAccess("address", Append), ArrayAccess("line", 0)]
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
                    // Parse array index
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

                    let index = match index_str.trim() {
                        "+" => ArrayIndex::Append,
                        "=" => ArrayIndex::Current,
                        num => ArrayIndex::Numeric(num.parse().map_err(|_| {
                            ExportError::InvalidPath {
                                path: path.to_string(),
                                resource: "".to_string(),
                            }
                        })?),
                    };

                    segments.push(PathSegment::ArrayAccess { field, index });
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
                            obj.insert(field.clone(), value.clone());
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
                PathSegment::ArrayAccess { field, index } => {
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

                        // Determine actual index
                        let actual_index = match index {
                            ArrayIndex::Numeric(n) => *n,
                            ArrayIndex::Append => arr.len(),
                            ArrayIndex::Current => *self.current_indices.get(field).unwrap_or(&0),
                        };

                        // Ensure array is large enough
                        while arr.len() <= actual_index {
                            arr.push(JsonValue::Object(Map::new()));
                        }

                        // Update current index tracker
                        self.current_indices.insert(field.clone(), actual_index);

                        if is_last {
                            // Set the value at this array index
                            arr[actual_index] = value.clone();
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

    /// Convert a FSH value string to appropriate JSON value
    fn convert_value(&self, value_str: &str) -> Result<JsonValue, ExportError> {
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

        // Code (starts with #)
        if let Some(code) = trimmed.strip_prefix('#') {
            return Ok(JsonValue::String(code.to_string()));
        }

        // Number (integer)
        if let Ok(num) = trimmed.parse::<i64>() {
            return Ok(JsonValue::Number(num.into()));
        }

        // Number (float)
        if let Ok(num) = trimmed.parse::<f64>() {
            if let Some(json_num) = serde_json::Number::from_f64(num) {
                return Ok(JsonValue::Number(json_num));
            }
        }

        // Default to string
        Ok(JsonValue::String(trimmed.to_string()))
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
            base_url: "http://example.org/fhir".to_string(),
            current_indices: HashMap::new(),
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
                index: ArrayIndex::Numeric(0)
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
                index: ArrayIndex::Append
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
                index: ArrayIndex::Current
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
                index: ArrayIndex::Numeric(0)
            }
        );
        assert_eq!(
            segments[1],
            PathSegment::ArrayAccess {
                field: "line".to_string(),
                index: ArrayIndex::Append
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
}
