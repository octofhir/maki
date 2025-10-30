//! Code system validation for FSH code references.
//!
//! This module provides parsing and validation of FSH code references (e.g., `#code`,
//! `system#code`, `$Alias#code`) against FHIR CodeSystem resources.
//!
//! # FSH Code Syntax
//!
//! ```fsh
//! // Short form (system inferred from context)
//! * code = #active
//!
//! // Full form with system URL
//! * code = http://hl7.org/fhir/observation-status#final
//!
//! // Using alias
//! Alias: $ObsStatus = http://hl7.org/fhir/observation-status
//! * code = $ObsStatus#final
//!
//! // With display string
//! * code = #final "Final"
//! ```
//!
//! # Example
//!
//! ```
//! use maki_core::canonical::codesystem::Code;
//!
//! // Parse simple code
//! let code = Code::from_fsh("#active").unwrap();
//! assert_eq!(code.code, "active");
//! assert!(code.system.is_none());
//!
//! // Parse code with system
//! let code = Code::from_fsh("http://hl7.org/fhir/observation-status#final").unwrap();
//! assert_eq!(code.code, "final");
//! assert_eq!(code.system.as_deref(), Some("http://hl7.org/fhir/observation-status"));
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// A code reference in FSH, potentially with system and display.
///
/// Represents FSH code syntax like:
/// - `#code` - code only
/// - `system#code` - code with system URL
/// - `$Alias#code` - code with alias (requires resolution)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code {
    /// The code system URL (if specified). None for context-inferred codes.
    pub system: Option<String>,
    /// The code value (required).
    pub code: String,
    /// The display string (optional).
    pub display: Option<String>,
}

impl Code {
    /// Parse a code from FSH format.
    ///
    /// Supports:
    /// - `#code` - Simple code without system
    /// - `system#code` - Code with system URL
    /// - `$alias#code` - Code with alias (alias not resolved here)
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::codesystem::Code;
    ///
    /// let code = Code::from_fsh("#active").unwrap();
    /// assert_eq!(code.code, "active");
    ///
    /// let code = Code::from_fsh("http://example.org#code123").unwrap();
    /// assert_eq!(code.system.as_deref(), Some("http://example.org"));
    /// assert_eq!(code.code, "code123");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `CodeError::InvalidFormat` if the input doesn't match expected patterns.
    pub fn from_fsh(s: &str) -> Result<Self, CodeError> {
        let s = s.trim();

        if s.is_empty() {
            return Err(CodeError::InvalidFormat("Empty code string".to_string()));
        }

        // Find the '#' separator between system and code
        if let Some(hash_pos) = s.rfind('#') {
            let (system_part, code_part) = s.split_at(hash_pos);
            let code = code_part[1..].trim().to_string(); // Skip the '#'

            if code.is_empty() {
                return Err(CodeError::InvalidFormat(
                    "Code cannot be empty after '#'".to_string(),
                ));
            }

            // System part may be empty (for #code), URL, or alias (starting with $)
            let system = if system_part.is_empty() {
                None
            } else {
                Some(system_part.to_string())
            };

            Ok(Self {
                system,
                code,
                display: None,
            })
        } else {
            // No '#' found - treat entire string as code
            Err(CodeError::InvalidFormat(format!(
                "Expected '#' separator in code reference: '{}'",
                s
            )))
        }
    }

    /// Create a code with all fields specified.
    pub fn new(system: Option<String>, code: String, display: Option<String>) -> Self {
        Self {
            system,
            code,
            display,
        }
    }

    /// Set the display string.
    pub fn with_display(mut self, display: String) -> Self {
        self.display = Some(display);
        self
    }

    /// Validate this code against a code system.
    ///
    /// Checks that the code exists in the system's concept list.
    ///
    /// # Errors
    ///
    /// Returns `CodeError::CodeNotFound` if the code doesn't exist in the system.
    /// Returns `CodeError::DisplayMismatch` if display is set and doesn't match.
    pub fn validate(&self, code_system: &CodeSystem) -> Result<(), CodeError> {
        // Find the concept
        let concept = code_system
            .concepts
            .iter()
            .find(|c| c.code == self.code)
            .ok_or_else(|| CodeError::CodeNotFound {
                code: self.code.clone(),
                system: code_system.url.clone(),
            })?;

        // Optionally validate display matches
        if let Some(ref display) = self.display
            && let Some(ref expected) = concept.display
            && display != expected
        {
            return Err(CodeError::DisplayMismatch {
                expected: expected.clone(),
                actual: display.clone(),
            });
        }

        Ok(())
    }
}

/// A FHIR CodeSystem resource (simplified representation).
///
/// Contains the URL, name, and list of concepts for validation purposes.
#[derive(Debug, Clone)]
pub struct CodeSystem {
    /// Canonical URL of the code system.
    pub url: String,
    /// Name of the code system.
    pub name: String,
    /// List of concepts (codes) defined in this system.
    pub concepts: Vec<Concept>,
}

impl CodeSystem {
    /// Create a new code system.
    pub fn new(url: String, name: String) -> Self {
        Self {
            url,
            name,
            concepts: Vec::new(),
        }
    }

    /// Add a concept to the code system.
    pub fn add_concept(&mut self, concept: Concept) {
        self.concepts.push(concept);
    }

    /// Check if a code exists in this system.
    pub fn contains_code(&self, code: &str) -> bool {
        self.concepts.iter().any(|c| c.code == code)
    }

    /// Get a concept by code.
    pub fn get_concept(&self, code: &str) -> Option<&Concept> {
        self.concepts.iter().find(|c| c.code == code)
    }

    /// Parse from a FHIR JSON CodeSystem resource.
    ///
    /// Extracts url, name, and concepts from the JSON structure.
    pub fn from_fhir_json(json: &serde_json::Value) -> Result<Self, CodeError> {
        let url = json["url"]
            .as_str()
            .ok_or_else(|| CodeError::InvalidSystemUrl("Missing 'url' field".to_string()))?
            .to_string();

        let name = json["name"].as_str().unwrap_or("Unknown").to_string();

        let mut code_system = Self::new(url, name);

        // Parse concepts from concept array
        if let Some(concepts_array) = json["concept"].as_array() {
            for concept_json in concepts_array {
                if let Some(code) = concept_json["code"].as_str() {
                    let display = concept_json["display"].as_str().map(String::from);
                    let definition = concept_json["definition"].as_str().map(String::from);

                    code_system.add_concept(Concept {
                        code: code.to_string(),
                        display,
                        definition,
                    });
                }
            }
        }

        Ok(code_system)
    }
}

/// A concept (code) within a CodeSystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// The code value.
    pub code: String,
    /// Display string (optional).
    pub display: Option<String>,
    /// Definition text (optional).
    pub definition: Option<String>,
}

impl Concept {
    /// Create a new concept with just a code.
    pub fn new(code: String) -> Self {
        Self {
            code,
            display: None,
            definition: None,
        }
    }

    /// Set the display string.
    pub fn with_display(mut self, display: String) -> Self {
        self.display = Some(display);
        self
    }

    /// Set the definition text.
    pub fn with_definition(mut self, definition: String) -> Self {
        self.definition = Some(definition);
        self
    }
}

/// Code system validator with caching.
///
/// Validates code references against FHIR CodeSystem resources, with
/// O(1) cached lookups after first load.
pub struct CodeSystemValidator {
    /// Cache of loaded code systems by URL.
    code_systems: HashMap<String, Arc<CodeSystem>>,
}

impl CodeSystemValidator {
    /// Create a new validator with an empty cache.
    pub fn new() -> Self {
        Self {
            code_systems: HashMap::new(),
        }
    }

    /// Load a code system into the cache.
    ///
    /// This allows pre-loading code systems before validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::codesystem::{CodeSystem, CodeSystemValidator, Concept};
    ///
    /// let mut validator = CodeSystemValidator::new();
    /// let mut cs = CodeSystem::new(
    ///     "http://example.org/codes".to_string(),
    ///     "Example".to_string()
    /// );
    /// cs.add_concept(Concept::new("code1".to_string()));
    /// validator.load_code_system(cs);
    /// ```
    pub fn load_code_system(&mut self, code_system: CodeSystem) {
        let url = code_system.url.clone();
        self.code_systems.insert(url, Arc::new(code_system));
    }

    /// Get a code system from the cache.
    ///
    /// Returns `None` if the system isn't loaded.
    pub fn get_code_system(&self, url: &str) -> Option<&Arc<CodeSystem>> {
        self.code_systems.get(url)
    }

    /// Validate a code reference.
    ///
    /// Checks that:
    /// 1. The code system exists (is loaded)
    /// 2. The code exists in the system
    /// 3. The display matches (if provided)
    ///
    /// # Errors
    ///
    /// - `CodeError::SystemNotFound` if the system isn't loaded
    /// - `CodeError::CodeNotFound` if the code doesn't exist
    /// - `CodeError::DisplayMismatch` if display doesn't match
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::canonical::codesystem::{Code, CodeSystem, CodeSystemValidator, Concept};
    ///
    /// let mut validator = CodeSystemValidator::new();
    /// let mut cs = CodeSystem::new(
    ///     "http://example.org/codes".to_string(),
    ///     "Example".to_string()
    /// );
    /// cs.add_concept(Concept::new("active".to_string()));
    /// validator.load_code_system(cs);
    ///
    /// let code = Code::new(
    ///     Some("http://example.org/codes".to_string()),
    ///     "active".to_string(),
    ///     None
    /// );
    ///
    /// assert!(validator.validate_code(&code).is_ok());
    /// ```
    pub fn validate_code(&self, code: &Code) -> Result<(), CodeError> {
        // If no system specified, we can't validate
        let system_url = code.system.as_ref().ok_or_else(|| {
            CodeError::InvalidFormat("Cannot validate code without system".to_string())
        })?;

        // Look up the code system
        let code_system = self
            .code_systems
            .get(system_url)
            .ok_or_else(|| CodeError::SystemNotFound(system_url.clone()))?;

        // Validate the code against the system
        code.validate(code_system)
    }

    /// Check if a code exists in a system (without full validation).
    ///
    /// Returns `false` if the system isn't loaded or the code doesn't exist.
    pub fn code_exists(&self, system: &str, code: &str) -> bool {
        self.code_systems
            .get(system)
            .map(|cs| cs.contains_code(code))
            .unwrap_or(false)
    }

    /// Get the number of cached code systems.
    pub fn cache_size(&self) -> usize {
        self.code_systems.len()
    }

    /// Clear the cache.
    pub fn clear_cache(&mut self) {
        self.code_systems.clear();
    }
}

impl Default for CodeSystemValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during code parsing and validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodeError {
    /// Invalid code format (missing '#', empty code, etc.).
    #[error("Invalid code format: {0}")]
    InvalidFormat(String),

    /// Code system not found (not loaded in validator).
    #[error("Code system not found: {0}")]
    SystemNotFound(String),

    /// Code not found in the specified system.
    #[error("Code '{code}' not found in system '{system}'")]
    CodeNotFound { code: String, system: String },

    /// Invalid system URL format.
    #[error("Invalid system URL: {0}")]
    InvalidSystemUrl(String),

    /// Display string doesn't match expected value.
    #[error("Display mismatch: expected '{expected}', got '{actual}'")]
    DisplayMismatch { expected: String, actual: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_code() {
        let code = Code::from_fsh("#active").unwrap();
        assert_eq!(code.code, "active");
        assert!(code.system.is_none());
        assert!(code.display.is_none());
    }

    #[test]
    fn test_parse_code_with_system() {
        let code = Code::from_fsh("http://hl7.org/fhir/observation-status#final").unwrap();
        assert_eq!(code.code, "final");
        assert_eq!(
            code.system.as_deref(),
            Some("http://hl7.org/fhir/observation-status")
        );
    }

    #[test]
    fn test_parse_code_with_alias() {
        let code = Code::from_fsh("$ObsStatus#final").unwrap();
        assert_eq!(code.code, "final");
        assert_eq!(code.system.as_deref(), Some("$ObsStatus"));
    }

    #[test]
    fn test_parse_invalid_no_hash() {
        let result = Code::from_fsh("just-a-code");
        assert!(result.is_err());
        assert!(matches!(result, Err(CodeError::InvalidFormat(_))));
    }

    #[test]
    fn test_parse_empty_code() {
        let result = Code::from_fsh("http://example.org#");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_string() {
        let result = Code::from_fsh("");
        assert!(result.is_err());
    }

    #[test]
    fn test_code_with_display() {
        let code = Code::from_fsh("#active")
            .unwrap()
            .with_display("Active".to_string());
        assert_eq!(code.display.as_deref(), Some("Active"));
    }

    #[test]
    fn test_code_system_new() {
        let cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        assert_eq!(cs.url, "http://example.org/codes");
        assert_eq!(cs.name, "Example");
        assert!(cs.concepts.is_empty());
    }

    #[test]
    fn test_code_system_add_concept() {
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("code1".to_string()));
        cs.add_concept(Concept::new("code2".to_string()).with_display("Code 2".to_string()));

        assert_eq!(cs.concepts.len(), 2);
        assert!(cs.contains_code("code1"));
        assert!(cs.contains_code("code2"));
        assert!(!cs.contains_code("code3"));
    }

    #[test]
    fn test_code_system_get_concept() {
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("active".to_string()).with_display("Active".to_string()));

        let concept = cs.get_concept("active").unwrap();
        assert_eq!(concept.code, "active");
        assert_eq!(concept.display.as_deref(), Some("Active"));

        assert!(cs.get_concept("inactive").is_none());
    }

    #[test]
    fn test_validate_code_exists() {
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("active".to_string()));

        let code = Code::new(
            Some("http://example.org/codes".to_string()),
            "active".to_string(),
            None,
        );

        assert!(code.validate(&cs).is_ok());
    }

    #[test]
    fn test_validate_code_not_found() {
        let cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );

        let code = Code::new(
            Some("http://example.org/codes".to_string()),
            "nonexistent".to_string(),
            None,
        );

        let result = code.validate(&cs);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodeError::CodeNotFound { .. })));
    }

    #[test]
    fn test_validate_display_match() {
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("active".to_string()).with_display("Active".to_string()));

        let code = Code::new(
            Some("http://example.org/codes".to_string()),
            "active".to_string(),
            Some("Active".to_string()),
        );

        assert!(code.validate(&cs).is_ok());
    }

    #[test]
    fn test_validate_display_mismatch() {
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("active".to_string()).with_display("Active".to_string()));

        let code = Code::new(
            Some("http://example.org/codes".to_string()),
            "active".to_string(),
            Some("Wrong Display".to_string()),
        );

        let result = code.validate(&cs);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodeError::DisplayMismatch { .. })));
    }

    #[test]
    fn test_validator_basic() {
        let mut validator = CodeSystemValidator::new();
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("code1".to_string()));
        validator.load_code_system(cs);

        let code = Code::new(
            Some("http://example.org/codes".to_string()),
            "code1".to_string(),
            None,
        );

        assert!(validator.validate_code(&code).is_ok());
        assert_eq!(validator.cache_size(), 1);
    }

    #[test]
    fn test_validator_system_not_found() {
        let validator = CodeSystemValidator::new();
        let code = Code::new(
            Some("http://unknown.org/codes".to_string()),
            "code1".to_string(),
            None,
        );

        let result = validator.validate_code(&code);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodeError::SystemNotFound(_))));
    }

    #[test]
    fn test_validator_code_exists() {
        let mut validator = CodeSystemValidator::new();
        let mut cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        cs.add_concept(Concept::new("active".to_string()));
        validator.load_code_system(cs);

        assert!(validator.code_exists("http://example.org/codes", "active"));
        assert!(!validator.code_exists("http://example.org/codes", "inactive"));
        assert!(!validator.code_exists("http://unknown.org/codes", "active"));
    }

    #[test]
    fn test_validator_clear_cache() {
        let mut validator = CodeSystemValidator::new();
        let cs = CodeSystem::new(
            "http://example.org/codes".to_string(),
            "Example".to_string(),
        );
        validator.load_code_system(cs);

        assert_eq!(validator.cache_size(), 1);
        validator.clear_cache();
        assert_eq!(validator.cache_size(), 0);
    }

    #[test]
    fn test_code_system_from_fhir_json() {
        let json = serde_json::json!({
            "resourceType": "CodeSystem",
            "url": "http://example.org/codes",
            "name": "ExampleCodeSystem",
            "concept": [
                {
                    "code": "active",
                    "display": "Active",
                    "definition": "The entity is active"
                },
                {
                    "code": "inactive",
                    "display": "Inactive"
                }
            ]
        });

        let cs = CodeSystem::from_fhir_json(&json).unwrap();
        assert_eq!(cs.url, "http://example.org/codes");
        assert_eq!(cs.name, "ExampleCodeSystem");
        assert_eq!(cs.concepts.len(), 2);
        assert!(cs.contains_code("active"));
        assert!(cs.contains_code("inactive"));

        let concept = cs.get_concept("active").unwrap();
        assert_eq!(concept.display.as_deref(), Some("Active"));
        assert_eq!(concept.definition.as_deref(), Some("The entity is active"));
    }

    #[test]
    fn test_code_system_from_fhir_json_missing_url() {
        let json = serde_json::json!({
            "resourceType": "CodeSystem",
            "name": "Example"
        });

        let result = CodeSystem::from_fhir_json(&json);
        assert!(result.is_err());
        assert!(matches!(result, Err(CodeError::InvalidSystemUrl(_))));
    }
}
