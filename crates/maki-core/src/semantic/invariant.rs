//! Invariant (constraint) support for FHIR profiles.
//!
//! This module provides structures and validation for FHIR invariants, which define
//! custom validation rules using FHIRPath expressions. Invariants can be attached to
//! profiles and elements to enforce business rules beyond what the base FHIR specification provides.
//!
//! # FSH Example
//!
//! ```fsh
//! Invariant: inv-1
//! Description: "Name must be present if active"
//! Severity: #error
//! Expression: "active.not() or name.exists()"
//! XPath: "not(f:active/@value='true') or exists(f:name)"
//!
//! Profile: PatientProfile
//! Parent: Patient
//! * obeys inv-1
//! * name obeys inv-2
//! ```
//!
//! # Usage
//!
//! ```rust
//! use maki_core::semantic::invariant::{Invariant, ConstraintSeverity, InvariantRegistry};
//!
//! // Create an invariant
//! let inv = Invariant::new(
//!     "inv-1".to_string(),
//!     ConstraintSeverity::Error,
//!     "active.not() or name.exists()".to_string()
//! ).with_description("Name must be present if active".to_string());
//!
//! // Register invariants
//! let mut registry = InvariantRegistry::new();
//! registry.register(inv)?;
//!
//! // Retrieve invariants
//! let inv = registry.get("inv-1").unwrap();
//! assert_eq!(inv.severity, ConstraintSeverity::Error);
//! # Ok::<(), maki_core::semantic::invariant::InvariantError>(())
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur when working with invariants.
#[derive(Debug, Error, Clone)]
pub enum InvariantError {
    /// Duplicate invariant name
    #[error("Duplicate invariant name: {0}")]
    DuplicateName(String),

    /// Invariant not found
    #[error("Invariant not found: {0}")]
    NotFound(String),

    /// Invalid FHIRPath expression
    #[error("Invalid FHIRPath expression: {0}")]
    InvalidFhirPath(String),

    /// Empty invariant name
    #[error("Invariant name cannot be empty")]
    EmptyName,

    /// Empty FHIRPath expression
    #[error("FHIRPath expression cannot be empty")]
    EmptyExpression,

    /// Invalid severity value
    #[error("Invalid severity: {0}")]
    InvalidSeverity(String),
}

/// An invariant (constraint) that defines a validation rule using FHIRPath.
///
/// Invariants can be attached to profiles or specific elements to enforce
/// business rules that go beyond the base FHIR specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    /// Unique identifier for the invariant (e.g., "inv-1", "us-core-1")
    pub name: String,

    /// Human-readable description of the constraint
    pub description: Option<String>,

    /// Severity level (error or warning)
    pub severity: ConstraintSeverity,

    /// FHIRPath expression that defines the constraint
    pub expression: String,

    /// Optional XPath expression (legacy support)
    pub xpath: Option<String>,

    /// Optional human-readable explanation
    pub human: Option<String>,

    /// Source/key for the constraint (used in StructureDefinition)
    pub key: Option<String>,

    /// Requirements that led to this constraint
    pub requirements: Option<String>,
}

impl Invariant {
    /// Create a new invariant with required fields.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the invariant
    /// * `severity` - Error or warning severity
    /// * `expression` - FHIRPath expression
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::invariant::{Invariant, ConstraintSeverity};
    ///
    /// let inv = Invariant::new(
    ///     "inv-1".to_string(),
    ///     ConstraintSeverity::Error,
    ///     "name.exists()".to_string()
    /// );
    /// ```
    pub fn new(name: String, severity: ConstraintSeverity, expression: String) -> Self {
        Self {
            name,
            description: None,
            severity,
            expression,
            xpath: None,
            human: None,
            key: None,
            requirements: None,
        }
    }

    /// Set the description for this invariant.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the XPath expression for this invariant.
    pub fn with_xpath(mut self, xpath: String) -> Self {
        self.xpath = Some(xpath);
        self
    }

    /// Set the human-readable explanation for this invariant.
    pub fn with_human(mut self, human: String) -> Self {
        self.human = Some(human);
        self
    }

    /// Set the key for this invariant.
    pub fn with_key(mut self, key: String) -> Self {
        self.key = Some(key);
        self
    }

    /// Set the requirements for this invariant.
    pub fn with_requirements(mut self, requirements: String) -> Self {
        self.requirements = Some(requirements);
        self
    }

    /// Validate the invariant structure.
    ///
    /// Checks that:
    /// - Name is not empty
    /// - Expression is not empty
    /// - FHIRPath expression has basic syntax validity
    pub fn validate(&self) -> Result<(), InvariantError> {
        if self.name.trim().is_empty() {
            return Err(InvariantError::EmptyName);
        }

        if self.expression.trim().is_empty() {
            return Err(InvariantError::EmptyExpression);
        }

        // Basic FHIRPath validation
        validate_fhirpath_basic(&self.expression)?;

        Ok(())
    }

    /// Parse an invariant from FHIR JSON.
    ///
    /// # Example JSON
    ///
    /// ```json
    /// {
    ///   "key": "inv-1",
    ///   "severity": "error",
    ///   "human": "Name must be present",
    ///   "expression": "name.exists()",
    ///   "xpath": "exists(f:name)"
    /// }
    /// ```
    pub fn from_json(json: &serde_json::Value) -> Result<Self, InvariantError> {
        let name = json
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or(InvariantError::EmptyName)?
            .to_string();

        let severity_str = json
            .get("severity")
            .and_then(|v| v.as_str())
            .ok_or_else(|| InvariantError::InvalidSeverity("missing".to_string()))?;

        let severity = ConstraintSeverity::parse(severity_str)?;

        let expression = json
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or(InvariantError::EmptyExpression)?
            .to_string();

        let mut invariant = Invariant::new(name, severity, expression);

        if let Some(human) = json.get("human").and_then(|v| v.as_str()) {
            invariant = invariant.with_human(human.to_string());
        }

        if let Some(xpath) = json.get("xpath").and_then(|v| v.as_str()) {
            invariant = invariant.with_xpath(xpath.to_string());
        }

        if let Some(requirements) = json.get("requirements").and_then(|v| v.as_str()) {
            invariant = invariant.with_requirements(requirements.to_string());
        }

        Ok(invariant)
    }

    /// Export to FHIR JSON format.
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        map.insert(
            "key".to_string(),
            serde_json::Value::String(self.name.clone()),
        );
        map.insert(
            "severity".to_string(),
            serde_json::Value::String(self.severity.as_str().to_string()),
        );
        map.insert(
            "expression".to_string(),
            serde_json::Value::String(self.expression.clone()),
        );

        if let Some(ref human) = self.human {
            map.insert(
                "human".to_string(),
                serde_json::Value::String(human.clone()),
            );
        }

        if let Some(ref xpath) = self.xpath {
            map.insert(
                "xpath".to_string(),
                serde_json::Value::String(xpath.clone()),
            );
        }

        if let Some(ref requirements) = self.requirements {
            map.insert(
                "requirements".to_string(),
                serde_json::Value::String(requirements.clone()),
            );
        }

        serde_json::Value::Object(map)
    }
}

/// Severity level for a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintSeverity {
    /// Constraint violation is an error (validation fails)
    Error,

    /// Constraint violation is a warning (validation passes with warning)
    Warning,
}

impl ConstraintSeverity {
    /// Parse from FHIR severity string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::invariant::ConstraintSeverity;
    ///
    /// assert_eq!(
    ///     ConstraintSeverity::parse("error").unwrap(),
    ///     ConstraintSeverity::Error
    /// );
    /// assert_eq!(
    ///     ConstraintSeverity::parse("warning").unwrap(),
    ///     ConstraintSeverity::Warning
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, InvariantError> {
        match s.to_lowercase().as_str() {
            "error" => Ok(ConstraintSeverity::Error),
            "warning" => Ok(ConstraintSeverity::Warning),
            _ => Err(InvariantError::InvalidSeverity(s.to_string())),
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConstraintSeverity::Error => "error",
            ConstraintSeverity::Warning => "warning",
        }
    }
}

/// Registry for managing invariants.
///
/// Provides O(1) lookup by name using a HashMap, with Arc-based sharing
/// for efficient cloning.
#[derive(Debug, Clone)]
pub struct InvariantRegistry {
    invariants: HashMap<String, Arc<Invariant>>,
}

impl InvariantRegistry {
    /// Create a new empty invariant registry.
    pub fn new() -> Self {
        Self {
            invariants: HashMap::new(),
        }
    }

    /// Register a new invariant.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - An invariant with the same name already exists
    /// - The invariant fails validation
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::invariant::{Invariant, ConstraintSeverity, InvariantRegistry};
    ///
    /// let mut registry = InvariantRegistry::new();
    /// let inv = Invariant::new(
    ///     "inv-1".to_string(),
    ///     ConstraintSeverity::Error,
    ///     "name.exists()".to_string()
    /// );
    ///
    /// registry.register(inv)?;
    /// # Ok::<(), maki_core::semantic::invariant::InvariantError>(())
    /// ```
    pub fn register(&mut self, invariant: Invariant) -> Result<(), InvariantError> {
        // Validate before registering
        invariant.validate()?;

        // Check for duplicates
        if self.invariants.contains_key(&invariant.name) {
            return Err(InvariantError::DuplicateName(invariant.name.clone()));
        }

        self.invariants
            .insert(invariant.name.clone(), Arc::new(invariant));

        Ok(())
    }

    /// Get an invariant by name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::invariant::{Invariant, ConstraintSeverity, InvariantRegistry};
    ///
    /// let mut registry = InvariantRegistry::new();
    /// let inv = Invariant::new(
    ///     "inv-1".to_string(),
    ///     ConstraintSeverity::Error,
    ///     "name.exists()".to_string()
    /// );
    ///
    /// registry.register(inv)?;
    /// let retrieved = registry.get("inv-1").unwrap();
    /// assert_eq!(retrieved.name, "inv-1");
    /// # Ok::<(), maki_core::semantic::invariant::InvariantError>(())
    /// ```
    pub fn get(&self, name: &str) -> Option<&Invariant> {
        self.invariants.get(name).map(|arc| arc.as_ref())
    }

    /// Check if an invariant exists.
    pub fn contains(&self, name: &str) -> bool {
        self.invariants.contains_key(name)
    }

    /// Get all registered invariants.
    pub fn all(&self) -> Vec<&Invariant> {
        self.invariants.values().map(|arc| arc.as_ref()).collect()
    }

    /// Get the number of registered invariants.
    pub fn len(&self) -> usize {
        self.invariants.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.invariants.is_empty()
    }

    /// Remove an invariant by name.
    pub fn remove(&mut self, name: &str) -> Option<Arc<Invariant>> {
        self.invariants.remove(name)
    }

    /// Clear all invariants.
    pub fn clear(&mut self) {
        self.invariants.clear();
    }

    /// Validate a FHIRPath expression.
    ///
    /// Performs basic syntax validation on the FHIRPath expression.
    /// Full FHIRPath parsing and evaluation is complex and would require
    /// a dedicated FHIRPath library.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::invariant::InvariantRegistry;
    ///
    /// let registry = InvariantRegistry::new();
    ///
    /// // Valid expressions
    /// assert!(registry.validate_fhirpath("name.exists()").is_ok());
    /// assert!(registry.validate_fhirpath("active.not() or name.exists()").is_ok());
    ///
    /// // Invalid expressions
    /// assert!(registry.validate_fhirpath("").is_err());
    /// assert!(registry.validate_fhirpath("   ").is_err());
    /// ```
    pub fn validate_fhirpath(&self, expression: &str) -> Result<(), InvariantError> {
        validate_fhirpath_basic(expression)
    }
}

impl Default for InvariantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Perform basic FHIRPath validation.
///
/// This is a simplified validation that checks for:
/// - Non-empty expression
/// - Balanced parentheses
/// - No obvious syntax errors
///
/// For full FHIRPath validation, a proper FHIRPath parser would be needed.
fn validate_fhirpath_basic(expression: &str) -> Result<(), InvariantError> {
    let trimmed = expression.trim();

    // Check for empty expression
    if trimmed.is_empty() {
        return Err(InvariantError::InvalidFhirPath(
            "Expression cannot be empty".to_string(),
        ));
    }

    // Check for balanced parentheses
    let mut paren_count = 0;
    for ch in trimmed.chars() {
        match ch {
            '(' => paren_count += 1,
            ')' => {
                paren_count -= 1;
                if paren_count < 0 {
                    return Err(InvariantError::InvalidFhirPath(
                        "Unbalanced parentheses".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }

    if paren_count != 0 {
        return Err(InvariantError::InvalidFhirPath(
            "Unbalanced parentheses".to_string(),
        ));
    }

    // Check for common FHIRPath operators and functions
    // This is a very basic check - just ensuring it looks like FHIRPath
    let has_fhirpath_content = trimmed.contains('.')
        || trimmed.contains("exists")
        || trimmed.contains("empty")
        || trimmed.contains("all")
        || trimmed.contains("where")
        || trimmed.contains("select")
        || trimmed.contains("or")
        || trimmed.contains("and")
        || trimmed.contains("not");

    if !has_fhirpath_content && trimmed.len() > 50 {
        return Err(InvariantError::InvalidFhirPath(
            "Expression does not look like valid FHIRPath".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_new() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        assert_eq!(inv.name, "inv-1");
        assert_eq!(inv.severity, ConstraintSeverity::Error);
        assert_eq!(inv.expression, "name.exists()");
        assert!(inv.description.is_none());
        assert!(inv.xpath.is_none());
    }

    #[test]
    fn test_invariant_with_description() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        )
        .with_description("Name must exist".to_string());

        assert_eq!(inv.description, Some("Name must exist".to_string()));
    }

    #[test]
    fn test_invariant_with_xpath() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        )
        .with_xpath("exists(f:name)".to_string());

        assert_eq!(inv.xpath, Some("exists(f:name)".to_string()));
    }

    #[test]
    fn test_invariant_validate() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        assert!(inv.validate().is_ok());
    }

    #[test]
    fn test_invariant_validate_empty_name() {
        let inv = Invariant::new(
            "".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        assert!(matches!(inv.validate(), Err(InvariantError::EmptyName)));
    }

    #[test]
    fn test_invariant_validate_empty_expression() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "".to_string(),
        );

        assert!(matches!(
            inv.validate(),
            Err(InvariantError::EmptyExpression)
        ));
    }

    #[test]
    fn test_constraint_severity_parse() {
        assert_eq!(
            ConstraintSeverity::parse("error").unwrap(),
            ConstraintSeverity::Error
        );
        assert_eq!(
            ConstraintSeverity::parse("warning").unwrap(),
            ConstraintSeverity::Warning
        );
        assert_eq!(
            ConstraintSeverity::parse("ERROR").unwrap(),
            ConstraintSeverity::Error
        );
    }

    #[test]
    fn test_constraint_severity_parse_invalid() {
        assert!(ConstraintSeverity::parse("invalid").is_err());
    }

    #[test]
    fn test_constraint_severity_as_str() {
        assert_eq!(ConstraintSeverity::Error.as_str(), "error");
        assert_eq!(ConstraintSeverity::Warning.as_str(), "warning");
    }

    #[test]
    fn test_invariant_registry_new() {
        let registry = InvariantRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_invariant_registry_register() {
        let mut registry = InvariantRegistry::new();
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        assert!(registry.register(inv).is_ok());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("inv-1"));
    }

    #[test]
    fn test_invariant_registry_duplicate() {
        let mut registry = InvariantRegistry::new();
        let inv1 = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );
        let inv2 = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Warning,
            "active.exists()".to_string(),
        );

        assert!(registry.register(inv1).is_ok());
        let result = registry.register(inv2);
        assert!(matches!(result, Err(InvariantError::DuplicateName(_))));
    }

    #[test]
    fn test_invariant_registry_get() {
        let mut registry = InvariantRegistry::new();
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        registry.register(inv).unwrap();

        let retrieved = registry.get("inv-1").unwrap();
        assert_eq!(retrieved.name, "inv-1");
        assert_eq!(retrieved.severity, ConstraintSeverity::Error);
    }

    #[test]
    fn test_invariant_registry_get_missing() {
        let registry = InvariantRegistry::new();
        assert!(registry.get("inv-1").is_none());
    }

    #[test]
    fn test_invariant_registry_all() {
        let mut registry = InvariantRegistry::new();

        let inv1 = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );
        let inv2 = Invariant::new(
            "inv-2".to_string(),
            ConstraintSeverity::Warning,
            "active.exists()".to_string(),
        );

        registry.register(inv1).unwrap();
        registry.register(inv2).unwrap();

        let all = registry.all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_invariant_registry_remove() {
        let mut registry = InvariantRegistry::new();
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );

        registry.register(inv).unwrap();
        assert_eq!(registry.len(), 1);

        registry.remove("inv-1");
        assert_eq!(registry.len(), 0);
        assert!(!registry.contains("inv-1"));
    }

    #[test]
    fn test_invariant_registry_clear() {
        let mut registry = InvariantRegistry::new();

        let inv1 = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        );
        let inv2 = Invariant::new(
            "inv-2".to_string(),
            ConstraintSeverity::Warning,
            "active.exists()".to_string(),
        );

        registry.register(inv1).unwrap();
        registry.register(inv2).unwrap();
        assert_eq!(registry.len(), 2);

        registry.clear();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_validate_fhirpath_basic() {
        // Valid expressions
        assert!(validate_fhirpath_basic("name.exists()").is_ok());
        assert!(validate_fhirpath_basic("active.not() or name.exists()").is_ok());
        assert!(validate_fhirpath_basic("identifier.where(system = 'http://example.org')").is_ok());
        assert!(validate_fhirpath_basic("children.all(age > 0)").is_ok());

        // Invalid expressions
        assert!(validate_fhirpath_basic("").is_err());
        assert!(validate_fhirpath_basic("   ").is_err());
        assert!(validate_fhirpath_basic("(unbalanced").is_err());
        assert!(validate_fhirpath_basic("unbalanced)").is_err());
        assert!(validate_fhirpath_basic("((nested)").is_err());
    }

    #[test]
    fn test_invariant_from_json() {
        let json = serde_json::json!({
            "key": "inv-1",
            "severity": "error",
            "human": "Name must exist",
            "expression": "name.exists()",
            "xpath": "exists(f:name)"
        });

        let inv = Invariant::from_json(&json).unwrap();
        assert_eq!(inv.name, "inv-1");
        assert_eq!(inv.severity, ConstraintSeverity::Error);
        assert_eq!(inv.expression, "name.exists()");
        assert_eq!(inv.human, Some("Name must exist".to_string()));
        assert_eq!(inv.xpath, Some("exists(f:name)".to_string()));
    }

    #[test]
    fn test_invariant_to_json() {
        let inv = Invariant::new(
            "inv-1".to_string(),
            ConstraintSeverity::Error,
            "name.exists()".to_string(),
        )
        .with_human("Name must exist".to_string())
        .with_xpath("exists(f:name)".to_string());

        let json = inv.to_json();

        assert_eq!(json["key"], "inv-1");
        assert_eq!(json["severity"], "error");
        assert_eq!(json["expression"], "name.exists()");
        assert_eq!(json["human"], "Name must exist");
        assert_eq!(json["xpath"], "exists(f:name)");
    }

    #[test]
    fn test_invariant_registry_validate_fhirpath() {
        let registry = InvariantRegistry::new();

        assert!(registry.validate_fhirpath("name.exists()").is_ok());
        assert!(registry.validate_fhirpath("").is_err());
        assert!(registry.validate_fhirpath("(unbalanced").is_err());
    }
}
