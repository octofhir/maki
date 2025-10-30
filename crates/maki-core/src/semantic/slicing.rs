//! Slicing support for FSH profiles.
//!
//! This module provides structures and validation for FHIR slicing, which allows
//! constraining repeating elements in profiles.
//!
//! # FSH Slicing Syntax
//!
//! ```fsh
//! // Basic slicing with contains
//! * identifier contains
//!     mrn 1..1 and
//!     ssn 0..1
//!
//! // Slice constraints
//! * identifier[mrn].system = "http://hospital.org/mrn"
//! * identifier[mrn].value 1..1
//!
//! * identifier[ssn].system = "http://hl7.org/fhir/sid/us-ssn"
//! * identifier[ssn].value 0..1
//!
//! // Slicing with discriminator
//! * extension contains myExt 0..* MS
//! * extension[myExt].url = "http://example.org/Extension/myExt"
//! ```
//!
//! # Example
//!
//! ```
//! use maki_core::semantic::slicing::{SliceDefinition, Discriminator, DiscriminatorType};
//!
//! let slice = SliceDefinition::new("mrn".to_string(), 1, "*".to_string());
//! let discriminator = Discriminator::new(DiscriminatorType::Value, "system".to_string());
//! let slice = slice.with_discriminator(discriminator);
//! ```

use thiserror::Error;

/// A slice definition in a FHIR profile.
///
/// Represents a named slice with cardinality and optional discriminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SliceDefinition {
    /// The name of the slice.
    pub name: String,
    /// Minimum cardinality.
    pub min: u32,
    /// Maximum cardinality ("*" for unbounded).
    pub max: String,
    /// Optional discriminator for this slice.
    pub discriminator: Option<Discriminator>,
    /// Short description of the slice.
    pub short: Option<String>,
    /// Flags (MS, SU, etc.).
    pub flags: Vec<String>,
}

impl SliceDefinition {
    /// Create a new slice definition.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::semantic::slicing::SliceDefinition;
    ///
    /// let slice = SliceDefinition::new("mrn".to_string(), 1, "1".to_string());
    /// assert_eq!(slice.name, "mrn");
    /// assert_eq!(slice.min, 1);
    /// assert_eq!(slice.max, "1");
    /// ```
    pub fn new(name: String, min: u32, max: String) -> Self {
        Self {
            name,
            min,
            max,
            discriminator: None,
            short: None,
            flags: Vec::new(),
        }
    }

    /// Set the discriminator.
    pub fn with_discriminator(mut self, discriminator: Discriminator) -> Self {
        self.discriminator = Some(discriminator);
        self
    }

    /// Set the short description.
    pub fn with_short(mut self, short: String) -> Self {
        self.short = Some(short);
        self
    }

    /// Add a flag (MS, SU, etc.).
    pub fn with_flag(mut self, flag: String) -> Self {
        self.flags.push(flag);
        self
    }

    /// Check if this slice has unbounded max cardinality.
    pub fn is_unbounded(&self) -> bool {
        self.max == "*"
    }

    /// Parse cardinality from FSH format (e.g., "1..1", "0..*").
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::semantic::slicing::SliceDefinition;
    ///
    /// let (min, max) = SliceDefinition::parse_cardinality("1..1").unwrap();
    /// assert_eq!(min, 1);
    /// assert_eq!(max, "1");
    ///
    /// let (min, max) = SliceDefinition::parse_cardinality("0..*").unwrap();
    /// assert_eq!(min, 0);
    /// assert_eq!(max, "*");
    /// ```
    pub fn parse_cardinality(s: &str) -> Result<(u32, String), SlicingError> {
        let parts: Vec<&str> = s.split("..").collect();
        if parts.len() != 2 {
            return Err(SlicingError::InvalidCardinality(s.to_string()));
        }

        let min = parts[0]
            .parse::<u32>()
            .map_err(|_| SlicingError::InvalidCardinality(s.to_string()))?;

        let max = if parts[1] == "*" {
            "*".to_string()
        } else {
            parts[1]
                .parse::<u32>()
                .map_err(|_| SlicingError::InvalidCardinality(s.to_string()))?
                .to_string()
        };

        Ok((min, max))
    }

    /// Validate the slice definition.
    ///
    /// Checks that:
    /// - Name is not empty
    /// - Cardinality is valid (min <= max)
    pub fn validate(&self) -> Result<(), SlicingError> {
        if self.name.is_empty() {
            return Err(SlicingError::InvalidSliceName(
                "Slice name cannot be empty".to_string(),
            ));
        }

        // Validate cardinality
        if !self.is_unbounded()
            && let Ok(max_num) = self.max.parse::<u32>()
            && self.min > max_num
        {
            return Err(SlicingError::InvalidCardinality(format!(
                "min ({}) cannot be greater than max ({})",
                self.min, self.max
            )));
        }

        Ok(())
    }
}

/// A discriminator defines how to distinguish between slices.
///
/// FHIR uses discriminators to determine which slice a particular element belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discriminator {
    /// The type of discriminator.
    pub discriminator_type: DiscriminatorType,
    /// The path to the discriminating element.
    pub path: String,
}

impl Discriminator {
    /// Create a new discriminator.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::semantic::slicing::{Discriminator, DiscriminatorType};
    ///
    /// let disc = Discriminator::new(DiscriminatorType::Value, "system".to_string());
    /// assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
    /// assert_eq!(disc.path, "system");
    /// ```
    pub fn new(discriminator_type: DiscriminatorType, path: String) -> Self {
        Self {
            discriminator_type,
            path,
        }
    }

    /// Parse discriminator type from string.
    pub fn parse_type(s: &str) -> Result<DiscriminatorType, SlicingError> {
        DiscriminatorType::parse(s)
    }
}

/// Types of discriminators for slicing.
///
/// See: <https://www.hl7.org/fhir/valueset-discriminator-type.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscriminatorType {
    /// The slices are differentiated by the value of the nominated element.
    Value,
    /// The slices have different values in the nominated element, as determined by pattern matching.
    Pattern,
    /// The slices are differentiated by type of the nominated element.
    Type,
    /// The slices are differentiated by conformance to a specified profile.
    Profile,
    /// The slices are differentiated by the presence or absence of the nominated element.
    Exists,
}

impl DiscriminatorType {
    /// Parse from FHIR discriminator type string.
    pub fn parse(s: &str) -> Result<Self, SlicingError> {
        match s.to_lowercase().as_str() {
            "value" => Ok(DiscriminatorType::Value),
            "pattern" => Ok(DiscriminatorType::Pattern),
            "type" => Ok(DiscriminatorType::Type),
            "profile" => Ok(DiscriminatorType::Profile),
            "exists" => Ok(DiscriminatorType::Exists),
            _ => Err(SlicingError::InvalidDiscriminatorType(s.to_string())),
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiscriminatorType::Value => "value",
            DiscriminatorType::Pattern => "pattern",
            DiscriminatorType::Type => "type",
            DiscriminatorType::Profile => "profile",
            DiscriminatorType::Exists => "exists",
        }
    }
}

impl std::fmt::Display for DiscriminatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Slicing configuration for an element.
///
/// Represents the slicing structure including all slices and discriminators.
#[derive(Debug, Clone)]
pub struct SlicingConfiguration {
    /// The element path being sliced (e.g., "Patient.identifier").
    pub element_path: String,
    /// Slicing rules (ordered, unordered).
    pub rules: SlicingRules,
    /// Whether slicing is closed (no additional slices allowed).
    pub closed: bool,
    /// The discriminators for this slicing.
    pub discriminators: Vec<Discriminator>,
    /// The individual slices.
    pub slices: Vec<SliceDefinition>,
}

impl SlicingConfiguration {
    /// Create a new slicing configuration.
    pub fn new(element_path: String) -> Self {
        Self {
            element_path,
            rules: SlicingRules::Open,
            closed: false,
            discriminators: Vec::new(),
            slices: Vec::new(),
        }
    }

    /// Set the slicing rules.
    pub fn with_rules(mut self, rules: SlicingRules) -> Self {
        self.rules = rules;
        self
    }

    /// Set whether slicing is closed.
    pub fn with_closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }

    /// Add a discriminator.
    pub fn add_discriminator(mut self, discriminator: Discriminator) -> Self {
        self.discriminators.push(discriminator);
        self
    }

    /// Add a slice.
    pub fn add_slice(mut self, slice: SliceDefinition) -> Self {
        self.slices.push(slice);
        self
    }

    /// Get a slice by name.
    pub fn get_slice(&self, name: &str) -> Option<&SliceDefinition> {
        self.slices.iter().find(|s| s.name == name)
    }

    /// Validate the slicing configuration.
    pub fn validate(&self) -> Result<(), SlicingError> {
        // Validate each slice
        for slice in &self.slices {
            slice.validate()?;
        }

        // Check for duplicate slice names
        let mut names = std::collections::HashSet::new();
        for slice in &self.slices {
            if !names.insert(&slice.name) {
                return Err(SlicingError::DuplicateSliceName(slice.name.clone()));
            }
        }

        Ok(())
    }
}

/// Slicing rules determining how slices are ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicingRules {
    /// Slices are ordered (closed).
    Closed,
    /// Slices are ordered (open).
    Open,
    /// Slices are unordered (openAtEnd).
    OpenAtEnd,
}

impl SlicingRules {
    /// Parse from FHIR slicing rules string.
    pub fn parse(s: &str) -> Result<Self, SlicingError> {
        match s.to_lowercase().as_str() {
            "closed" => Ok(SlicingRules::Closed),
            "open" => Ok(SlicingRules::Open),
            "openatend" => Ok(SlicingRules::OpenAtEnd),
            _ => Err(SlicingError::InvalidSlicingRules(s.to_string())),
        }
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            SlicingRules::Closed => "closed",
            SlicingRules::Open => "open",
            SlicingRules::OpenAtEnd => "openAtEnd",
        }
    }
}

/// Handler for slicing operations.
///
/// Provides utilities for creating and validating slices.
pub struct SlicingHandler;

impl SlicingHandler {
    /// Create a new slicing handler.
    pub fn new() -> Self {
        Self
    }

    /// Infer a discriminator for a slice based on common patterns.
    ///
    /// Algorithm 3 from MAKI_PLAN.md: Automatic discriminator inference.
    ///
    /// Common patterns:
    /// - `extension` slices: discriminate by `url` (value)
    /// - `identifier` slices: discriminate by `system` (value)
    /// - `coding` slices: discriminate by `system` (value)
    /// - `telecom` slices: discriminate by `system` (value)
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::semantic::slicing::{SlicingHandler, DiscriminatorType};
    ///
    /// let handler = SlicingHandler::new();
    /// let disc = handler.infer_discriminator("extension", "myExt").unwrap();
    /// assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
    /// assert_eq!(disc.path, "url");
    /// ```
    pub fn infer_discriminator(
        &self,
        element_name: &str,
        _slice_name: &str,
    ) -> Option<Discriminator> {
        // Common discriminator patterns
        match element_name {
            "extension" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "url".to_string(),
            )),
            "identifier" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "system".to_string(),
            )),
            "coding" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "system".to_string(),
            )),
            "telecom" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "system".to_string(),
            )),
            "name" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "use".to_string(),
            )),
            "address" => Some(Discriminator::new(
                DiscriminatorType::Value,
                "use".to_string(),
            )),
            _ => None,
        }
    }

    /// Validate a slicing configuration.
    pub fn validate_slicing(&self, config: &SlicingConfiguration) -> Result<(), SlicingError> {
        config.validate()
    }

    /// Create a slice path from an element path and slice name.
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_core::semantic::slicing::SlicingHandler;
    ///
    /// let handler = SlicingHandler::new();
    /// let path = handler.create_slice_path("Patient.identifier", "mrn");
    /// assert_eq!(path, "Patient.identifier:mrn");
    /// ```
    pub fn create_slice_path(&self, element_path: &str, slice_name: &str) -> String {
        format!("{}:{}", element_path, slice_name)
    }
}

impl Default for SlicingHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during slicing operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SlicingError {
    /// Invalid slice name.
    #[error("Invalid slice name: {0}")]
    InvalidSliceName(String),

    /// Invalid cardinality specification.
    #[error("Invalid cardinality: {0}")]
    InvalidCardinality(String),

    /// Invalid discriminator type.
    #[error("Invalid discriminator type: {0}")]
    InvalidDiscriminatorType(String),

    /// Invalid slicing rules.
    #[error("Invalid slicing rules: {0}")]
    InvalidSlicingRules(String),

    /// Duplicate slice name.
    #[error("Duplicate slice name: {0}")]
    DuplicateSliceName(String),

    /// Missing discriminator.
    #[error("Missing discriminator for slice '{0}'")]
    MissingDiscriminator(String),

    /// Invalid slice path.
    #[error("Invalid slice path: {0}")]
    InvalidSlicePath(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_definition_new() {
        let slice = SliceDefinition::new("mrn".to_string(), 1, "1".to_string());
        assert_eq!(slice.name, "mrn");
        assert_eq!(slice.min, 1);
        assert_eq!(slice.max, "1");
        assert!(slice.discriminator.is_none());
    }

    #[test]
    fn test_slice_definition_with_discriminator() {
        let disc = Discriminator::new(DiscriminatorType::Value, "system".to_string());
        let slice = SliceDefinition::new("mrn".to_string(), 1, "1".to_string())
            .with_discriminator(disc.clone());
        assert_eq!(slice.discriminator, Some(disc));
    }

    #[test]
    fn test_slice_definition_is_unbounded() {
        let slice = SliceDefinition::new("test".to_string(), 0, "*".to_string());
        assert!(slice.is_unbounded());

        let slice = SliceDefinition::new("test".to_string(), 0, "1".to_string());
        assert!(!slice.is_unbounded());
    }

    #[test]
    fn test_parse_cardinality() {
        let (min, max) = SliceDefinition::parse_cardinality("1..1").unwrap();
        assert_eq!(min, 1);
        assert_eq!(max, "1");

        let (min, max) = SliceDefinition::parse_cardinality("0..*").unwrap();
        assert_eq!(min, 0);
        assert_eq!(max, "*");

        let (min, max) = SliceDefinition::parse_cardinality("2..5").unwrap();
        assert_eq!(min, 2);
        assert_eq!(max, "5");
    }

    #[test]
    fn test_parse_cardinality_invalid() {
        assert!(SliceDefinition::parse_cardinality("invalid").is_err());
        assert!(SliceDefinition::parse_cardinality("1").is_err());
        assert!(SliceDefinition::parse_cardinality("1..x").is_err());
    }

    #[test]
    fn test_slice_definition_validate() {
        let slice = SliceDefinition::new("mrn".to_string(), 1, "1".to_string());
        assert!(slice.validate().is_ok());

        let slice = SliceDefinition::new("".to_string(), 1, "1".to_string());
        assert!(slice.validate().is_err());

        let slice = SliceDefinition::new("test".to_string(), 5, "2".to_string());
        assert!(slice.validate().is_err());
    }

    #[test]
    fn test_discriminator_new() {
        let disc = Discriminator::new(DiscriminatorType::Value, "system".to_string());
        assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
        assert_eq!(disc.path, "system");
    }

    #[test]
    fn test_discriminator_type_from_str() {
        assert_eq!(
            DiscriminatorType::parse("value").unwrap(),
            DiscriminatorType::Value
        );
        assert_eq!(
            DiscriminatorType::parse("pattern").unwrap(),
            DiscriminatorType::Pattern
        );
        assert_eq!(
            DiscriminatorType::parse("type").unwrap(),
            DiscriminatorType::Type
        );
        assert_eq!(
            DiscriminatorType::parse("profile").unwrap(),
            DiscriminatorType::Profile
        );
        assert_eq!(
            DiscriminatorType::parse("exists").unwrap(),
            DiscriminatorType::Exists
        );
    }

    #[test]
    fn test_discriminator_type_invalid() {
        assert!(DiscriminatorType::parse("invalid").is_err());
    }

    #[test]
    fn test_slicing_configuration() {
        let config = SlicingConfiguration::new("Patient.identifier".to_string())
            .add_discriminator(Discriminator::new(
                DiscriminatorType::Value,
                "system".to_string(),
            ))
            .add_slice(SliceDefinition::new("mrn".to_string(), 1, "1".to_string()))
            .add_slice(SliceDefinition::new("ssn".to_string(), 0, "1".to_string()));

        assert_eq!(config.element_path, "Patient.identifier");
        assert_eq!(config.discriminators.len(), 1);
        assert_eq!(config.slices.len(), 2);
    }

    #[test]
    fn test_slicing_configuration_get_slice() {
        let config = SlicingConfiguration::new("Patient.identifier".to_string())
            .add_slice(SliceDefinition::new("mrn".to_string(), 1, "1".to_string()));

        let slice = config.get_slice("mrn").unwrap();
        assert_eq!(slice.name, "mrn");

        assert!(config.get_slice("nonexistent").is_none());
    }

    #[test]
    fn test_slicing_configuration_validate() {
        let config = SlicingConfiguration::new("Patient.identifier".to_string())
            .add_slice(SliceDefinition::new("mrn".to_string(), 1, "1".to_string()))
            .add_slice(SliceDefinition::new("ssn".to_string(), 0, "1".to_string()));

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_slicing_configuration_duplicate_names() {
        let config = SlicingConfiguration::new("Patient.identifier".to_string())
            .add_slice(SliceDefinition::new("mrn".to_string(), 1, "1".to_string()))
            .add_slice(SliceDefinition::new("mrn".to_string(), 0, "1".to_string()));

        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(result, Err(SlicingError::DuplicateSliceName(_))));
    }

    #[test]
    fn test_slicing_rules_from_str() {
        assert_eq!(SlicingRules::parse("closed").unwrap(), SlicingRules::Closed);
        assert_eq!(SlicingRules::parse("open").unwrap(), SlicingRules::Open);
        assert_eq!(
            SlicingRules::parse("openatend").unwrap(),
            SlicingRules::OpenAtEnd
        );
    }

    #[test]
    fn test_slicing_handler_infer_discriminator() {
        let handler = SlicingHandler::new();

        let disc = handler.infer_discriminator("extension", "myExt").unwrap();
        assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
        assert_eq!(disc.path, "url");

        let disc = handler.infer_discriminator("identifier", "mrn").unwrap();
        assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
        assert_eq!(disc.path, "system");

        let disc = handler.infer_discriminator("coding", "myCode").unwrap();
        assert_eq!(disc.discriminator_type, DiscriminatorType::Value);
        assert_eq!(disc.path, "system");
    }

    #[test]
    fn test_slicing_handler_create_slice_path() {
        let handler = SlicingHandler::new();
        let path = handler.create_slice_path("Patient.identifier", "mrn");
        assert_eq!(path, "Patient.identifier:mrn");
    }

    #[test]
    fn test_slicing_handler_validate() {
        let handler = SlicingHandler::new();
        let config = SlicingConfiguration::new("Patient.identifier".to_string())
            .add_slice(SliceDefinition::new("mrn".to_string(), 1, "1".to_string()));

        assert!(handler.validate_slicing(&config).is_ok());
    }
}
