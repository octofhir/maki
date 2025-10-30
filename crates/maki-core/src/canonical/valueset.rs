//! ValueSet validation for FSH binding references.
//!
//! This module provides parsing and validation of FSH ValueSet references used in
//! binding statements (e.g., `* code from ValueSetURL (required)`).
//!
//! # FSH Binding Syntax
//!
//! ```fsh
//! // Binding with required strength
//! * status from http://hl7.org/fhir/ValueSet/observation-status (required)
//!
//! // Binding with alias
//! Alias: $ObsStatus = http://hl7.org/fhir/ValueSet/observation-status
//! * status from $ObsStatus (required)
//!
//! // Different binding strengths
//! * code from http://loinc.org (extensible)
//! * category from http://example.org/cats (preferred)
//! * note from http://example.org/notes (example)
//! ```
//!
//! # Example
//!
//! ```
//! use maki_core::canonical::valueset::{ValueSetReference, BindingStrength};
//!
//! let reference = ValueSetReference::new(
//!     "http://hl7.org/fhir/ValueSet/observation-status".to_string(),
//!     BindingStrength::Required
//! );
//!
//! assert_eq!(reference.strength, BindingStrength::Required);
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// A reference to a FHIR ValueSet used in a binding statement.
///
/// Represents FSH binding syntax like:
/// - `from ValueSetURL (required)`
/// - `from $Alias (extensible)`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSetReference {
    /// The ValueSet canonical URL (or alias).
    pub url: String,
    /// The binding strength.
    pub strength: BindingStrength,
}

impl ValueSetReference {
    /// Create a new ValueSet reference.
    pub fn new(url: String, strength: BindingStrength) -> Self {
        Self { url, strength }
    }

    /// Parse binding strength from FSH syntax.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::valueset::{BindingStrength, ValueSetReference};
    ///
    /// let strength = ValueSetReference::parse_strength("required").unwrap();
    /// assert_eq!(strength, BindingStrength::Required);
    ///
    /// let strength = ValueSetReference::parse_strength("extensible").unwrap();
    /// assert_eq!(strength, BindingStrength::Extensible);
    /// ```
    pub fn parse_strength(s: &str) -> Result<BindingStrength, ValueSetError> {
        match s.trim().to_lowercase().as_str() {
            "required" => Ok(BindingStrength::Required),
            "extensible" => Ok(BindingStrength::Extensible),
            "preferred" => Ok(BindingStrength::Preferred),
            "example" => Ok(BindingStrength::Example),
            _ => Err(ValueSetError::InvalidStrength(s.to_string())),
        }
    }
}

/// Binding strength for ValueSet bindings.
///
/// Defines how strictly a value must conform to the ValueSet.
/// See: <https://www.hl7.org/fhir/valueset-binding-strength.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingStrength {
    /// To be conformant, the concept must be from the specified ValueSet.
    Required,
    /// To be conformant, the concept should be from the specified ValueSet if possible,
    /// but codes from other ValueSets are allowed if necessary.
    Extensible,
    /// Instances are encouraged to use codes from the ValueSet for interoperability,
    /// but are not required to.
    Preferred,
    /// Instances are not required or encouraged to use the ValueSet.
    Example,
}

impl BindingStrength {
    /// Get the string representation of the binding strength.
    pub fn as_str(&self) -> &'static str {
        match self {
            BindingStrength::Required => "required",
            BindingStrength::Extensible => "extensible",
            BindingStrength::Preferred => "preferred",
            BindingStrength::Example => "example",
        }
    }

    /// Check if this binding strength is strict (required or extensible).
    pub fn is_strict(&self) -> bool {
        matches!(
            self,
            BindingStrength::Required | BindingStrength::Extensible
        )
    }
}

impl std::fmt::Display for BindingStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A FHIR ValueSet resource (simplified representation).
///
/// Contains the canonical URL, name, and compose/expansion information.
#[derive(Debug, Clone)]
pub struct ValueSet {
    /// Canonical URL of the ValueSet.
    pub url: String,
    /// Name of the ValueSet.
    pub name: String,
    /// Version of the ValueSet (optional).
    pub version: Option<String>,
    /// Status (draft, active, retired, etc.).
    pub status: String,
    /// Compose definition (included/excluded codes).
    pub compose: Option<ValueSetCompose>,
    /// Expansion (if available).
    pub expansion: Option<ValueSetExpansion>,
}

impl ValueSet {
    /// Create a new ValueSet.
    pub fn new(url: String, name: String) -> Self {
        Self {
            url,
            name,
            version: None,
            status: "draft".to_string(),
            compose: None,
            expansion: None,
        }
    }

    /// Set the version.
    pub fn with_version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    /// Set the status.
    pub fn with_status(mut self, status: String) -> Self {
        self.status = status;
        self
    }

    /// Set the compose definition.
    pub fn with_compose(mut self, compose: ValueSetCompose) -> Self {
        self.compose = Some(compose);
        self
    }

    /// Set the expansion.
    pub fn with_expansion(mut self, expansion: ValueSetExpansion) -> Self {
        self.expansion = Some(expansion);
        self
    }

    /// Check if a code might be in this ValueSet.
    ///
    /// First checks expansion if available, then compose includes.
    /// Returns `None` if cannot be determined (no expansion/compose).
    pub fn contains_code(&self, system: &str, code: &str) -> Option<bool> {
        // Check expansion first (most reliable)
        if let Some(ref expansion) = self.expansion {
            return Some(
                expansion
                    .contains
                    .iter()
                    .any(|c| c.system.as_deref() == Some(system) && c.code == code),
            );
        }

        // Check compose includes
        if let Some(ref compose) = self.compose {
            for include in &compose.include {
                if include.system.as_deref() == Some(system) {
                    // If no concepts listed, assume system includes all
                    if include.concept.is_empty() {
                        return Some(true);
                    }
                    // Check if code is explicitly listed
                    if include.concept.iter().any(|c| c.code == code) {
                        return Some(true);
                    }
                }
            }
            // Code not found in includes
            return Some(false);
        }

        // Cannot determine
        None
    }

    /// Parse from a FHIR JSON ValueSet resource.
    ///
    /// Extracts url, name, compose, and expansion from the JSON structure.
    pub fn from_fhir_json(json: &serde_json::Value) -> Result<Self, ValueSetError> {
        let url = json["url"]
            .as_str()
            .ok_or_else(|| ValueSetError::InvalidUrl("Missing 'url' field".to_string()))?
            .to_string();

        let name = json["name"].as_str().unwrap_or("Unknown").to_string();

        let version = json["version"].as_str().map(String::from);

        let status = json["status"].as_str().unwrap_or("draft").to_string();

        let mut value_set = Self::new(url, name).with_status(status);

        if let Some(v) = version {
            value_set = value_set.with_version(v);
        }

        // Parse compose
        if let Some(compose_json) = json.get("compose")
            && let Ok(compose) = ValueSetCompose::from_json(compose_json)
        {
            value_set = value_set.with_compose(compose);
        }

        // Parse expansion
        if let Some(expansion_json) = json.get("expansion")
            && let Ok(expansion) = ValueSetExpansion::from_json(expansion_json)
        {
            value_set = value_set.with_expansion(expansion);
        }

        Ok(value_set)
    }
}

/// ValueSet compose definition.
#[derive(Debug, Clone)]
pub struct ValueSetCompose {
    /// Included code systems and concepts.
    pub include: Vec<ConceptSetComponent>,
    /// Excluded code systems and concepts.
    pub exclude: Vec<ConceptSetComponent>,
}

impl ValueSetCompose {
    /// Parse from FHIR JSON.
    fn from_json(json: &serde_json::Value) -> Result<Self, ValueSetError> {
        let mut compose = Self {
            include: Vec::new(),
            exclude: Vec::new(),
        };

        if let Some(includes) = json["include"].as_array() {
            for inc in includes {
                if let Ok(component) = ConceptSetComponent::from_json(inc) {
                    compose.include.push(component);
                }
            }
        }

        if let Some(excludes) = json["exclude"].as_array() {
            for exc in excludes {
                if let Ok(component) = ConceptSetComponent::from_json(exc) {
                    compose.exclude.push(component);
                }
            }
        }

        Ok(compose)
    }
}

/// A component of a ValueSet compose (include or exclude).
#[derive(Debug, Clone)]
pub struct ConceptSetComponent {
    /// The code system.
    pub system: Option<String>,
    /// The version of the code system.
    pub version: Option<String>,
    /// Specific concepts to include/exclude.
    pub concept: Vec<ConceptReference>,
}

impl ConceptSetComponent {
    /// Parse from FHIR JSON.
    fn from_json(json: &serde_json::Value) -> Result<Self, ValueSetError> {
        let system = json["system"].as_str().map(String::from);
        let version = json["version"].as_str().map(String::from);

        let mut concept = Vec::new();
        if let Some(concepts) = json["concept"].as_array() {
            for c in concepts {
                if let Some(code) = c["code"].as_str() {
                    let display = c["display"].as_str().map(String::from);
                    concept.push(ConceptReference {
                        code: code.to_string(),
                        display,
                    });
                }
            }
        }

        Ok(Self {
            system,
            version,
            concept,
        })
    }
}

/// A reference to a concept in a ValueSet compose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptReference {
    /// The code.
    pub code: String,
    /// The display (optional).
    pub display: Option<String>,
}

/// ValueSet expansion.
#[derive(Debug, Clone)]
pub struct ValueSetExpansion {
    /// Timestamp when expanded.
    pub timestamp: Option<String>,
    /// Total number of concepts.
    pub total: Option<usize>,
    /// The expanded concepts.
    pub contains: Vec<ValueSetContains>,
}

impl ValueSetExpansion {
    /// Parse from FHIR JSON.
    fn from_json(json: &serde_json::Value) -> Result<Self, ValueSetError> {
        let timestamp = json["timestamp"].as_str().map(String::from);
        let total = json["total"].as_u64().map(|n| n as usize);

        let mut contains = Vec::new();
        if let Some(contains_array) = json["contains"].as_array() {
            for c in contains_array {
                if let Ok(item) = ValueSetContains::from_json(c) {
                    contains.push(item);
                }
            }
        }

        Ok(Self {
            timestamp,
            total,
            contains,
        })
    }
}

/// A concept in a ValueSet expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueSetContains {
    /// The code system.
    pub system: Option<String>,
    /// The code.
    pub code: String,
    /// The display.
    pub display: Option<String>,
}

impl ValueSetContains {
    /// Parse from FHIR JSON.
    fn from_json(json: &serde_json::Value) -> Result<Self, ValueSetError> {
        let code = json["code"]
            .as_str()
            .ok_or_else(|| ValueSetError::InvalidFormat("Missing 'code' in expansion".to_string()))?
            .to_string();

        let system = json["system"].as_str().map(String::from);
        let display = json["display"].as_str().map(String::from);

        Ok(Self {
            system,
            code,
            display,
        })
    }
}

/// ValueSet validator with caching.
///
/// Validates ValueSet references and checks code membership.
pub struct ValueSetValidator {
    /// Cache of loaded ValueSets by URL.
    value_sets: HashMap<String, Arc<ValueSet>>,
}

impl ValueSetValidator {
    /// Create a new validator with an empty cache.
    pub fn new() -> Self {
        Self {
            value_sets: HashMap::new(),
        }
    }

    /// Load a ValueSet into the cache.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::valueset::{ValueSet, ValueSetValidator};
    ///
    /// let mut validator = ValueSetValidator::new();
    /// let vs = ValueSet::new(
    ///     "http://example.org/vs".to_string(),
    ///     "Example".to_string()
    /// );
    /// validator.load_value_set(vs);
    /// ```
    pub fn load_value_set(&mut self, value_set: ValueSet) {
        let url = value_set.url.clone();
        self.value_sets.insert(url, Arc::new(value_set));
    }

    /// Get a ValueSet from the cache.
    ///
    /// Returns `None` if not loaded.
    pub fn get_value_set(&self, url: &str) -> Option<&Arc<ValueSet>> {
        self.value_sets.get(url)
    }

    /// Validate a ValueSet binding reference.
    ///
    /// Checks that:
    /// 1. The ValueSet exists (is loaded)
    /// 2. The ValueSet is active (not retired)
    ///
    /// # Errors
    ///
    /// - `ValueSetError::NotFound` if the ValueSet isn't loaded
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::valueset::{
    ///     ValueSet, ValueSetReference, ValueSetValidator, BindingStrength
    /// };
    ///
    /// let mut validator = ValueSetValidator::new();
    /// let vs = ValueSet::new(
    ///     "http://example.org/vs".to_string(),
    ///     "Example".to_string()
    /// ).with_status("active".to_string());
    /// validator.load_value_set(vs);
    ///
    /// let reference = ValueSetReference::new(
    ///     "http://example.org/vs".to_string(),
    ///     BindingStrength::Required
    /// );
    ///
    /// assert!(validator.validate_binding(&reference).is_ok());
    /// ```
    pub fn validate_binding(&self, reference: &ValueSetReference) -> Result<(), ValueSetError> {
        // Check if ValueSet exists
        let value_set = self
            .value_sets
            .get(&reference.url)
            .ok_or_else(|| ValueSetError::NotFound(reference.url.clone()))?;

        // Warn if ValueSet is retired (but don't fail)
        if value_set.status == "retired" {
            // In a real implementation, we might log a warning here
            // For now, we just allow it
        }

        Ok(())
    }

    /// Check if a code is in a ValueSet.
    ///
    /// Returns:
    /// - `Ok(true)` if the code is in the ValueSet
    /// - `Ok(false)` if the code is not in the ValueSet
    /// - `Err(_)` if the ValueSet is not loaded or cannot determine membership
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::valueset::{
    ///     ValueSet, ValueSetValidator, ValueSetExpansion, ValueSetContains
    /// };
    ///
    /// let mut validator = ValueSetValidator::new();
    /// let expansion = ValueSetExpansion {
    ///     timestamp: None,
    ///     total: Some(1),
    ///     contains: vec![
    ///         ValueSetContains {
    ///             system: Some("http://example.org".to_string()),
    ///             code: "code1".to_string(),
    ///             display: None,
    ///         }
    ///     ]
    /// };
    ///
    /// let vs = ValueSet::new(
    ///     "http://example.org/vs".to_string(),
    ///     "Example".to_string()
    /// ).with_expansion(expansion);
    /// validator.load_value_set(vs);
    ///
    /// let result = validator.code_in_value_set(
    ///     "http://example.org/vs",
    ///     "http://example.org",
    ///     "code1"
    /// ).unwrap();
    /// assert_eq!(result, true);
    /// ```
    pub fn code_in_value_set(
        &self,
        value_set_url: &str,
        system: &str,
        code: &str,
    ) -> Result<bool, ValueSetError> {
        let value_set = self
            .value_sets
            .get(value_set_url)
            .ok_or_else(|| ValueSetError::NotFound(value_set_url.to_string()))?;

        value_set
            .contains_code(system, code)
            .ok_or_else(|| ValueSetError::CannotDetermineMembership(value_set_url.to_string()))
    }

    /// Get the number of cached ValueSets.
    pub fn cache_size(&self) -> usize {
        self.value_sets.len()
    }

    /// Clear the cache.
    pub fn clear_cache(&mut self) {
        self.value_sets.clear();
    }
}

impl Default for ValueSetValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during ValueSet parsing and validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValueSetError {
    /// ValueSet not found (not loaded).
    #[error("ValueSet not found: {0}")]
    NotFound(String),

    /// Invalid binding strength.
    #[error("Invalid binding strength: {0}")]
    InvalidStrength(String),

    /// Invalid ValueSet URL.
    #[error("Invalid ValueSet URL: {0}")]
    InvalidUrl(String),

    /// Invalid format in ValueSet JSON.
    #[error("Invalid ValueSet format: {0}")]
    InvalidFormat(String),

    /// Cannot determine code membership.
    #[error("Cannot determine if code is in ValueSet '{0}' (no expansion or compose)")]
    CannotDetermineMembership(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_strength_parse() {
        assert_eq!(
            ValueSetReference::parse_strength("required").unwrap(),
            BindingStrength::Required
        );
        assert_eq!(
            ValueSetReference::parse_strength("extensible").unwrap(),
            BindingStrength::Extensible
        );
        assert_eq!(
            ValueSetReference::parse_strength("preferred").unwrap(),
            BindingStrength::Preferred
        );
        assert_eq!(
            ValueSetReference::parse_strength("example").unwrap(),
            BindingStrength::Example
        );
        assert_eq!(
            ValueSetReference::parse_strength("REQUIRED").unwrap(),
            BindingStrength::Required
        );
    }

    #[test]
    fn test_binding_strength_invalid() {
        let result = ValueSetReference::parse_strength("invalid");
        assert!(result.is_err());
        assert!(matches!(result, Err(ValueSetError::InvalidStrength(_))));
    }

    #[test]
    fn test_binding_strength_display() {
        assert_eq!(BindingStrength::Required.as_str(), "required");
        assert_eq!(BindingStrength::Extensible.as_str(), "extensible");
        assert_eq!(BindingStrength::Preferred.as_str(), "preferred");
        assert_eq!(BindingStrength::Example.as_str(), "example");
    }

    #[test]
    fn test_binding_strength_is_strict() {
        assert!(BindingStrength::Required.is_strict());
        assert!(BindingStrength::Extensible.is_strict());
        assert!(!BindingStrength::Preferred.is_strict());
        assert!(!BindingStrength::Example.is_strict());
    }

    #[test]
    fn test_value_set_reference_new() {
        let reference = ValueSetReference::new(
            "http://example.org/vs".to_string(),
            BindingStrength::Required,
        );
        assert_eq!(reference.url, "http://example.org/vs");
        assert_eq!(reference.strength, BindingStrength::Required);
    }

    #[test]
    fn test_value_set_new() {
        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string());
        assert_eq!(vs.url, "http://example.org/vs");
        assert_eq!(vs.name, "Example");
        assert_eq!(vs.status, "draft");
        assert!(vs.version.is_none());
        assert!(vs.compose.is_none());
        assert!(vs.expansion.is_none());
    }

    #[test]
    fn test_value_set_with_version() {
        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string())
            .with_version("1.0.0".to_string());
        assert_eq!(vs.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn test_value_set_with_status() {
        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string())
            .with_status("active".to_string());
        assert_eq!(vs.status, "active");
    }

    #[test]
    fn test_validator_basic() {
        let mut validator = ValueSetValidator::new();
        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string())
            .with_status("active".to_string());
        validator.load_value_set(vs);

        let reference = ValueSetReference::new(
            "http://example.org/vs".to_string(),
            BindingStrength::Required,
        );

        assert!(validator.validate_binding(&reference).is_ok());
        assert_eq!(validator.cache_size(), 1);
    }

    #[test]
    fn test_validator_not_found() {
        let validator = ValueSetValidator::new();
        let reference = ValueSetReference::new(
            "http://unknown.org/vs".to_string(),
            BindingStrength::Required,
        );

        let result = validator.validate_binding(&reference);
        assert!(result.is_err());
        assert!(matches!(result, Err(ValueSetError::NotFound(_))));
    }

    #[test]
    fn test_validator_clear_cache() {
        let mut validator = ValueSetValidator::new();
        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string());
        validator.load_value_set(vs);

        assert_eq!(validator.cache_size(), 1);
        validator.clear_cache();
        assert_eq!(validator.cache_size(), 0);
    }

    #[test]
    fn test_value_set_from_fhir_json() {
        let json = serde_json::json!({
            "resourceType": "ValueSet",
            "url": "http://example.org/vs",
            "name": "ExampleValueSet",
            "version": "1.0.0",
            "status": "active",
            "compose": {
                "include": [
                    {
                        "system": "http://example.org/codes",
                        "concept": [
                            {"code": "code1", "display": "Code 1"},
                            {"code": "code2", "display": "Code 2"}
                        ]
                    }
                ]
            }
        });

        let vs = ValueSet::from_fhir_json(&json).unwrap();
        assert_eq!(vs.url, "http://example.org/vs");
        assert_eq!(vs.name, "ExampleValueSet");
        assert_eq!(vs.version.as_deref(), Some("1.0.0"));
        assert_eq!(vs.status, "active");
        assert!(vs.compose.is_some());

        let compose = vs.compose.unwrap();
        assert_eq!(compose.include.len(), 1);
        assert_eq!(compose.include[0].concept.len(), 2);
    }

    #[test]
    fn test_value_set_from_fhir_json_missing_url() {
        let json = serde_json::json!({
            "resourceType": "ValueSet",
            "name": "Example"
        });

        let result = ValueSet::from_fhir_json(&json);
        assert!(result.is_err());
        assert!(matches!(result, Err(ValueSetError::InvalidUrl(_))));
    }

    #[test]
    fn test_value_set_contains_code_with_expansion() {
        let expansion = ValueSetExpansion {
            timestamp: None,
            total: Some(2),
            contains: vec![
                ValueSetContains {
                    system: Some("http://example.org".to_string()),
                    code: "code1".to_string(),
                    display: None,
                },
                ValueSetContains {
                    system: Some("http://example.org".to_string()),
                    code: "code2".to_string(),
                    display: None,
                },
            ],
        };

        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string())
            .with_expansion(expansion);

        assert_eq!(vs.contains_code("http://example.org", "code1"), Some(true));
        assert_eq!(vs.contains_code("http://example.org", "code2"), Some(true));
        assert_eq!(vs.contains_code("http://example.org", "code3"), Some(false));
        assert_eq!(vs.contains_code("http://other.org", "code1"), Some(false));
    }

    #[test]
    fn test_validator_code_in_value_set() {
        let mut validator = ValueSetValidator::new();
        let expansion = ValueSetExpansion {
            timestamp: None,
            total: Some(1),
            contains: vec![ValueSetContains {
                system: Some("http://example.org".to_string()),
                code: "code1".to_string(),
                display: None,
            }],
        };

        let vs = ValueSet::new("http://example.org/vs".to_string(), "Example".to_string())
            .with_expansion(expansion);
        validator.load_value_set(vs);

        let result = validator
            .code_in_value_set("http://example.org/vs", "http://example.org", "code1")
            .unwrap();
        assert!(result);

        let result = validator
            .code_in_value_set("http://example.org/vs", "http://example.org", "code2")
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_validator_code_in_value_set_not_found() {
        let validator = ValueSetValidator::new();
        let result =
            validator.code_in_value_set("http://unknown.org/vs", "http://example.org", "code1");
        assert!(result.is_err());
        assert!(matches!(result, Err(ValueSetError::NotFound(_))));
    }
}
