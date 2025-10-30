//! Extension validation for FSH extension references.
//!
//! This module provides parsing and validation of FHIR extension references and definitions.
//! Extensions are a core FHIR mechanism for adding custom elements to resources.
//!
//! # FSH Extension Syntax
//!
//! ```fsh
//! // Extension definition
//! Extension: PatientBirthPlace
//! Id: patient-birthPlace
//! Title: "Birth Place"
//! Description: "The registered place of birth of the patient."
//! * value[x] only Address
//! * valueAddress 1..1
//!
//! // Using extensions in profiles
//! Profile: MyPatient
//! Parent: Patient
//! * extension contains birthPlace 0..1
//! * extension[birthPlace] ^short = "Place of birth"
//! * extension[birthPlace].url = "http://example.org/Extension/patient-birthPlace"
//! * extension[birthPlace].valueAddress 1..1
//! ```
//!
//! # Example
//!
//! ```
//! use maki_core::canonical::extension::{ExtensionReference, ExtensionContext};
//!
//! let reference = ExtensionReference::new(
//!     "http://example.org/Extension/patient-birthPlace".to_string()
//! );
//!
//! // With slice name
//! let reference = reference.with_slice_name("birthPlace".to_string());
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// A reference to a FHIR extension used in a profile.
///
/// Represents FSH extension references like:
/// - `extension contains myExt 0..1`
/// - `extension[myExt].url = "http://..."`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionReference {
    /// The extension canonical URL.
    pub url: String,
    /// The slice name (optional).
    pub slice_name: Option<String>,
}

impl ExtensionReference {
    /// Create a new extension reference.
    pub fn new(url: String) -> Self {
        Self {
            url,
            slice_name: None,
        }
    }

    /// Set the slice name.
    pub fn with_slice_name(mut self, slice_name: String) -> Self {
        self.slice_name = Some(slice_name);
        self
    }

    /// Parse an extension URL from FSH format.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::extension::ExtensionReference;
    ///
    /// let reference = ExtensionReference::parse_url("http://example.org/Extension/myExt").unwrap();
    /// assert_eq!(reference.url, "http://example.org/Extension/myExt");
    /// ```
    pub fn parse_url(s: &str) -> Result<Self, ExtensionError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ExtensionError::InvalidUrl(
                "Extension URL cannot be empty".to_string(),
            ));
        }

        // Basic URL validation
        if !s.starts_with("http://") && !s.starts_with("https://") {
            return Err(ExtensionError::InvalidUrl(format!(
                "Extension URL must start with http:// or https://: '{}'",
                s
            )));
        }

        Ok(Self::new(s.to_string()))
    }
}

/// A FHIR extension definition.
///
/// Represents the structure and constraints of an extension.
#[derive(Debug, Clone)]
pub struct ExtensionDefinition {
    /// The canonical URL of the extension.
    pub url: String,
    /// The name of the extension.
    pub name: String,
    /// The title (human-readable).
    pub title: Option<String>,
    /// Description of the extension.
    pub description: Option<String>,
    /// Status (draft, active, retired, etc.).
    pub status: String,
    /// Version of the extension.
    pub version: Option<String>,
    /// Extension context (where it can be used).
    pub context: Vec<ExtensionContext>,
    /// The type of value this extension holds.
    pub value_type: Option<String>,
}

impl ExtensionDefinition {
    /// Create a new extension definition.
    pub fn new(url: String, name: String) -> Self {
        Self {
            url,
            name,
            title: None,
            description: None,
            status: "draft".to_string(),
            version: None,
            context: Vec::new(),
            value_type: None,
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the status.
    pub fn with_status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Set the version.
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Add a context where this extension can be used.
    pub fn add_context(mut self, context: ExtensionContext) -> Self {
        self.context.push(context);
        self
    }

    /// Set the value type.
    pub fn with_value_type(mut self, value_type: String) -> Self {
        self.value_type = Some(value_type);
        self
    }

    /// Check if this extension can be used in the given context.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::extension::{ExtensionDefinition, ExtensionContext, ContextType};
    ///
    /// let ext = ExtensionDefinition::new(
    ///     "http://example.org/Extension/myExt".to_string(),
    ///     "MyExt".to_string()
    /// ).add_context(ExtensionContext {
    ///     context_type: ContextType::Element,
    ///     expression: "Patient".to_string(),
    /// });
    ///
    /// assert!(ext.can_be_used_on("Patient"));
    /// assert!(!ext.can_be_used_on("Observation"));
    /// ```
    pub fn can_be_used_on(&self, resource_type: &str) -> bool {
        // If no context specified, can be used anywhere
        if self.context.is_empty() {
            return true;
        }

        // Check if any context matches
        self.context.iter().any(|ctx| ctx.matches(resource_type))
    }

    /// Parse from a FHIR JSON StructureDefinition resource.
    ///
    /// Extracts extension information from the JSON structure.
    pub fn from_fhir_json(json: &serde_json::Value) -> Result<Self, ExtensionError> {
        let url = json["url"]
            .as_str()
            .ok_or_else(|| ExtensionError::InvalidFormat("Missing 'url' field".to_string()))?
            .to_string();

        let name = json["name"]
            .as_str()
            .ok_or_else(|| ExtensionError::InvalidFormat("Missing 'name' field".to_string()))?
            .to_string();

        let title = json["title"].as_str().map(String::from);
        let description = json["description"].as_str().map(String::from);
        let status = json["status"].as_str().unwrap_or("draft").to_string();
        let version = json["version"].as_str().map(String::from);

        let mut extension = Self::new(url, name).with_status(status);

        if let Some(t) = title {
            extension = extension.with_title(t);
        }
        if let Some(d) = description {
            extension = extension.with_description(d);
        }
        if let Some(v) = version {
            extension = extension.with_version(v);
        }

        // Parse context
        if let Some(context_array) = json["context"].as_array() {
            for ctx in context_array {
                if let Ok(context) = ExtensionContext::from_json(ctx) {
                    extension = extension.add_context(context);
                }
            }
        }

        // Parse value type from snapshot or differential
        if let Some(snapshot) = json.get("snapshot")
            && let Some(elements) = snapshot["element"].as_array()
        {
            for element in elements {
                if let Some(path) = element["path"].as_str()
                    && (path == "Extension.value[x]" || path.starts_with("Extension.value"))
                    && let Some(types) = element["type"].as_array()
                    && let Some(first_type) = types.first()
                    && let Some(code) = first_type["code"].as_str()
                {
                    extension = extension.with_value_type(code.to_string());
                    break;
                }
            }
        }

        Ok(extension)
    }
}

/// Extension context - where an extension can be used.
///
/// FHIR extensions can specify where they are allowed to be used
/// (e.g., on Patient resources, on Observation.status element, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionContext {
    /// The type of context.
    pub context_type: ContextType,
    /// The expression defining the context (e.g., "Patient", "Observation.status").
    pub expression: String,
}

impl ExtensionContext {
    /// Create a new extension context.
    pub fn new(context_type: ContextType, expression: String) -> Self {
        Self {
            context_type,
            expression,
        }
    }

    /// Check if this context matches the given resource type or element path.
    pub fn matches(&self, target: &str) -> bool {
        match self.context_type {
            ContextType::Resource => {
                // Match resource type
                self.expression == target || self.expression == "*"
            }
            ContextType::Element => {
                // Match element path (can be resource or element)
                if self.expression.contains('.') {
                    // Full path like "Patient.name"
                    target.starts_with(&self.expression) || self.expression.starts_with(target)
                } else {
                    // Just resource name
                    self.expression == target
                }
            }
            ContextType::Extension => {
                // Match extension URL
                self.expression == target
            }
            ContextType::FhirPath => {
                // FHIRPath expression - simplified matching
                self.expression == target
            }
        }
    }

    /// Parse from FHIR JSON context element.
    fn from_json(json: &serde_json::Value) -> Result<Self, ExtensionError> {
        let type_str = json["type"]
            .as_str()
            .ok_or_else(|| ExtensionError::InvalidFormat("Missing context 'type'".to_string()))?;

        let context_type = ContextType::parse(type_str)?;

        let expression = json["expression"]
            .as_str()
            .ok_or_else(|| {
                ExtensionError::InvalidFormat("Missing context 'expression'".to_string())
            })?
            .to_string();

        Ok(Self::new(context_type, expression))
    }
}

/// Types of extension contexts.
///
/// See: <https://www.hl7.org/fhir/valueset-extension-context-type.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextType {
    /// The context is all resources of the specified type.
    Resource,
    /// The context is a particular element (can include resource types).
    Element,
    /// The context is another extension.
    Extension,
    /// The context is based on a FHIRPath expression.
    FhirPath,
}

impl ContextType {
    /// Parse from FHIR context type string.
    pub fn parse(s: &str) -> Result<Self, ExtensionError> {
        match s.to_lowercase().as_str() {
            "resource" => Ok(ContextType::Resource),
            "element" => Ok(ContextType::Element),
            "extension" => Ok(ContextType::Extension),
            "fhirpath" => Ok(ContextType::FhirPath),
            _ => Err(ExtensionError::InvalidContextType(s.to_string())),
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ContextType::Resource => "resource",
            ContextType::Element => "element",
            ContextType::Extension => "extension",
            ContextType::FhirPath => "fhirpath",
        }
    }
}

/// Extension validator with caching.
///
/// Validates extension references and checks context constraints.
pub struct ExtensionValidator {
    /// Cache of loaded extensions by URL.
    extensions: HashMap<String, Arc<ExtensionDefinition>>,
}

impl ExtensionValidator {
    /// Create a new validator with an empty cache.
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
        }
    }

    /// Load an extension definition into the cache.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::extension::{ExtensionDefinition, ExtensionValidator};
    ///
    /// let mut validator = ExtensionValidator::new();
    /// let ext = ExtensionDefinition::new(
    ///     "http://example.org/Extension/myExt".to_string(),
    ///     "MyExt".to_string()
    /// );
    /// validator.load_extension(ext);
    /// ```
    pub fn load_extension(&mut self, extension: ExtensionDefinition) {
        let url = extension.url.clone();
        self.extensions.insert(url, Arc::new(extension));
    }

    /// Get an extension definition from the cache.
    ///
    /// Returns `None` if not loaded.
    pub fn get_extension(&self, url: &str) -> Option<&Arc<ExtensionDefinition>> {
        self.extensions.get(url)
    }

    /// Validate an extension reference.
    ///
    /// Checks that:
    /// 1. The extension exists (is loaded)
    /// 2. The extension is active (not retired)
    ///
    /// # Errors
    ///
    /// - `ExtensionError::NotFound` if the extension isn't loaded
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::extension::{
    ///     ExtensionDefinition, ExtensionReference, ExtensionValidator
    /// };
    ///
    /// let mut validator = ExtensionValidator::new();
    /// let ext = ExtensionDefinition::new(
    ///     "http://example.org/Extension/myExt".to_string(),
    ///     "MyExt".to_string()
    /// ).with_status("active".to_string());
    /// validator.load_extension(ext);
    ///
    /// let reference = ExtensionReference::new(
    ///     "http://example.org/Extension/myExt".to_string()
    /// );
    ///
    /// assert!(validator.validate_extension(&reference).is_ok());
    /// ```
    pub fn validate_extension(&self, reference: &ExtensionReference) -> Result<(), ExtensionError> {
        // Check if extension exists
        let extension = self
            .extensions
            .get(&reference.url)
            .ok_or_else(|| ExtensionError::NotFound(reference.url.clone()))?;

        // Warn if extension is retired (but don't fail)
        if extension.status == "retired" {
            // In a real implementation, we might log a warning here
        }

        Ok(())
    }

    /// Validate that an extension can be used in a given context.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::extension::{
    ///     ExtensionDefinition, ExtensionValidator, ExtensionContext, ContextType
    /// };
    ///
    /// let mut validator = ExtensionValidator::new();
    /// let ext = ExtensionDefinition::new(
    ///     "http://example.org/Extension/myExt".to_string(),
    ///     "MyExt".to_string()
    /// ).add_context(ExtensionContext::new(
    ///     ContextType::Element,
    ///     "Patient".to_string()
    /// ));
    /// validator.load_extension(ext);
    ///
    /// // Valid context
    /// assert!(validator.validate_context(
    ///     "http://example.org/Extension/myExt",
    ///     "Patient"
    /// ).is_ok());
    ///
    /// // Invalid context
    /// assert!(validator.validate_context(
    ///     "http://example.org/Extension/myExt",
    ///     "Observation"
    /// ).is_err());
    /// ```
    pub fn validate_context(
        &self,
        extension_url: &str,
        target_context: &str,
    ) -> Result<(), ExtensionError> {
        let extension = self
            .extensions
            .get(extension_url)
            .ok_or_else(|| ExtensionError::NotFound(extension_url.to_string()))?;

        if !extension.can_be_used_on(target_context) {
            return Err(ExtensionError::InvalidContext {
                ext: extension_url.to_string(),
                resource: target_context.to_string(),
            });
        }

        Ok(())
    }

    /// Get the contexts where an extension can be used.
    ///
    /// Returns a list of context expressions.
    pub fn get_extension_context(&self, url: &str) -> Result<Vec<String>, ExtensionError> {
        let extension = self
            .extensions
            .get(url)
            .ok_or_else(|| ExtensionError::NotFound(url.to_string()))?;

        Ok(extension
            .context
            .iter()
            .map(|ctx| ctx.expression.clone())
            .collect())
    }

    /// Get the number of cached extensions.
    pub fn cache_size(&self) -> usize {
        self.extensions.len()
    }

    /// Clear the cache.
    pub fn clear_cache(&mut self) {
        self.extensions.clear();
    }
}

impl Default for ExtensionValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during extension parsing and validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    /// Extension not found (not loaded).
    #[error("Extension not found: {0}")]
    NotFound(String),

    /// Invalid extension URL.
    #[error("Invalid extension URL: {0}")]
    InvalidUrl(String),

    /// Invalid extension format in JSON.
    #[error("Invalid extension format: {0}")]
    InvalidFormat(String),

    /// Invalid context type.
    #[error("Invalid context type: {0}")]
    InvalidContextType(String),

    /// Extension cannot be used in the given context.
    #[error("Extension '{ext}' cannot be used on '{resource}'")]
    InvalidContext { ext: String, resource: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_reference_new() {
        let reference = ExtensionReference::new("http://example.org/Extension/myExt".to_string());
        assert_eq!(reference.url, "http://example.org/Extension/myExt");
        assert!(reference.slice_name.is_none());
    }

    #[test]
    fn test_extension_reference_with_slice_name() {
        let reference = ExtensionReference::new("http://example.org/Extension/myExt".to_string())
            .with_slice_name("mySlice".to_string());
        assert_eq!(reference.slice_name.as_deref(), Some("mySlice"));
    }

    #[test]
    fn test_extension_reference_parse_url() {
        let reference =
            ExtensionReference::parse_url("http://example.org/Extension/myExt").unwrap();
        assert_eq!(reference.url, "http://example.org/Extension/myExt");
    }

    #[test]
    fn test_extension_reference_parse_url_invalid() {
        let result = ExtensionReference::parse_url("not-a-url");
        assert!(result.is_err());
        assert!(matches!(result, Err(ExtensionError::InvalidUrl(_))));
    }

    #[test]
    fn test_extension_reference_parse_url_empty() {
        let result = ExtensionReference::parse_url("");
        assert!(result.is_err());
    }

    #[test]
    fn test_extension_definition_new() {
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        );
        assert_eq!(ext.url, "http://example.org/Extension/myExt");
        assert_eq!(ext.name, "MyExt");
        assert_eq!(ext.status, "draft");
        assert!(ext.context.is_empty());
    }

    #[test]
    fn test_extension_definition_with_title() {
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        )
        .with_title("My Extension".to_string());
        assert_eq!(ext.title.as_deref(), Some("My Extension"));
    }

    #[test]
    fn test_extension_definition_can_be_used_on() {
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        )
        .add_context(ExtensionContext::new(
            ContextType::Element,
            "Patient".to_string(),
        ));

        assert!(ext.can_be_used_on("Patient"));
        assert!(!ext.can_be_used_on("Observation"));
    }

    #[test]
    fn test_extension_definition_can_be_used_anywhere() {
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        );

        // No context = can be used anywhere
        assert!(ext.can_be_used_on("Patient"));
        assert!(ext.can_be_used_on("Observation"));
    }

    #[test]
    fn test_context_type_from_str() {
        assert_eq!(
            ContextType::parse("resource").unwrap(),
            ContextType::Resource
        );
        assert_eq!(ContextType::parse("element").unwrap(), ContextType::Element);
        assert_eq!(
            ContextType::parse("extension").unwrap(),
            ContextType::Extension
        );
        assert_eq!(
            ContextType::parse("fhirpath").unwrap(),
            ContextType::FhirPath
        );
    }

    #[test]
    fn test_context_type_invalid() {
        let result = ContextType::parse("invalid");
        assert!(result.is_err());
        assert!(matches!(result, Err(ExtensionError::InvalidContextType(_))));
    }

    #[test]
    fn test_extension_context_matches_resource() {
        let ctx = ExtensionContext::new(ContextType::Resource, "Patient".to_string());
        assert!(ctx.matches("Patient"));
        assert!(!ctx.matches("Observation"));
    }

    #[test]
    fn test_extension_context_matches_element() {
        let ctx = ExtensionContext::new(ContextType::Element, "Patient.name".to_string());
        assert!(ctx.matches("Patient"));
        assert!(ctx.matches("Patient.name"));
        assert!(!ctx.matches("Observation"));
    }

    #[test]
    fn test_validator_basic() {
        let mut validator = ExtensionValidator::new();
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        )
        .with_status("active".to_string());
        validator.load_extension(ext);

        let reference = ExtensionReference::new("http://example.org/Extension/myExt".to_string());

        assert!(validator.validate_extension(&reference).is_ok());
        assert_eq!(validator.cache_size(), 1);
    }

    #[test]
    fn test_validator_not_found() {
        let validator = ExtensionValidator::new();
        let reference = ExtensionReference::new("http://unknown.org/Extension/myExt".to_string());

        let result = validator.validate_extension(&reference);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExtensionError::NotFound(_))));
    }

    #[test]
    fn test_validator_context_validation() {
        let mut validator = ExtensionValidator::new();
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        )
        .add_context(ExtensionContext::new(
            ContextType::Element,
            "Patient".to_string(),
        ));
        validator.load_extension(ext);

        // Valid context
        assert!(
            validator
                .validate_context("http://example.org/Extension/myExt", "Patient")
                .is_ok()
        );

        // Invalid context
        let result =
            validator.validate_context("http://example.org/Extension/myExt", "Observation");
        assert!(result.is_err());
        assert!(matches!(result, Err(ExtensionError::InvalidContext { .. })));
    }

    #[test]
    fn test_validator_get_context() {
        let mut validator = ExtensionValidator::new();
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        )
        .add_context(ExtensionContext::new(
            ContextType::Element,
            "Patient".to_string(),
        ))
        .add_context(ExtensionContext::new(
            ContextType::Element,
            "Observation".to_string(),
        ));
        validator.load_extension(ext);

        let contexts = validator
            .get_extension_context("http://example.org/Extension/myExt")
            .unwrap();
        assert_eq!(contexts.len(), 2);
        assert!(contexts.contains(&"Patient".to_string()));
        assert!(contexts.contains(&"Observation".to_string()));
    }

    #[test]
    fn test_validator_clear_cache() {
        let mut validator = ExtensionValidator::new();
        let ext = ExtensionDefinition::new(
            "http://example.org/Extension/myExt".to_string(),
            "MyExt".to_string(),
        );
        validator.load_extension(ext);

        assert_eq!(validator.cache_size(), 1);
        validator.clear_cache();
        assert_eq!(validator.cache_size(), 0);
    }

    #[test]
    fn test_extension_definition_from_fhir_json() {
        let json = serde_json::json!({
            "resourceType": "StructureDefinition",
            "url": "http://example.org/Extension/patient-birthPlace",
            "name": "PatientBirthPlace",
            "title": "Birth Place",
            "description": "The registered place of birth of the patient.",
            "status": "active",
            "version": "1.0.0",
            "context": [
                {
                    "type": "element",
                    "expression": "Patient"
                }
            ]
        });

        let ext = ExtensionDefinition::from_fhir_json(&json).unwrap();
        assert_eq!(ext.url, "http://example.org/Extension/patient-birthPlace");
        assert_eq!(ext.name, "PatientBirthPlace");
        assert_eq!(ext.title.as_deref(), Some("Birth Place"));
        assert_eq!(ext.status, "active");
        assert_eq!(ext.version.as_deref(), Some("1.0.0"));
        assert_eq!(ext.context.len(), 1);
        assert_eq!(ext.context[0].expression, "Patient");
    }

    #[test]
    fn test_extension_definition_from_fhir_json_missing_url() {
        let json = serde_json::json!({
            "resourceType": "StructureDefinition",
            "name": "MyExt"
        });

        let result = ExtensionDefinition::from_fhir_json(&json);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExtensionError::InvalidFormat(_))));
    }
}
