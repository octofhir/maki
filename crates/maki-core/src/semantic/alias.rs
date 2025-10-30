//! Alias Resolution System
//!
//! This module implements FSH alias resolution, mapping short alias names
//! (e.g., `$SCT`) to full URLs (e.g., `http://snomed.info/sct`).
//!
//! # FSH Alias Syntax
//!
//! ```fsh
//! Alias: $SCT = http://snomed.info/sct
//! Alias: $LOINC = http://loinc.org
//! Alias: $UCUM = http://unitsofmeasure.org
//!
//! Profile: MyProfile
//! Parent: Observation
//! * code from $SCT
//! * code = $LOINC#8480-6
//! ```
//!
//! # Usage
//!
//! ```rust
//! use maki_core::semantic::alias::{Alias, AliasTable};
//! use std::path::PathBuf;
//!
//! let mut table = AliasTable::new();
//!
//! // Add an alias
//! let alias = Alias {
//!     name: "$SCT".to_string(),
//!     url: "http://snomed.info/sct".to_string(),
//!     source_file: PathBuf::from("test.fsh"),
//!     source_span: 0..10,
//! };
//!
//! table.add_alias(alias).unwrap();
//!
//! // Resolve an alias
//! assert_eq!(table.resolve("$SCT"), Some("http://snomed.info/sct"));
//! ```

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use thiserror::Error;

/// Source code span (start byte offset..end byte offset)
pub type Span = Range<usize>;

/// An alias definition mapping a short name to a full URL
///
/// # Example
///
/// ```fsh
/// Alias: $SCT = http://snomed.info/sct
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    /// Alias name (must start with '$')
    pub name: String,

    /// Full URL this alias resolves to
    pub url: String,

    /// Source file where the alias was defined
    pub source_file: PathBuf,

    /// Byte span in the source file
    pub source_span: Span,
}

/// Errors that can occur during alias operations
#[derive(Debug, Error, Clone)]
pub enum AliasError {
    /// Duplicate alias definition
    #[error("Duplicate alias '{name}' defined in {file1:?} and {file2:?}")]
    DuplicateAlias {
        name: String,
        file1: PathBuf,
        file2: PathBuf,
    },

    /// Invalid alias name (must start with '$')
    #[error("Invalid alias name '{0}': alias names must start with '$'")]
    InvalidAliasName(String),

    /// Undefined alias reference
    #[error("Undefined alias '{0}'")]
    UndefinedAlias(String),

    /// Invalid URL
    #[error("Invalid URL '{0}'")]
    InvalidUrl(String),
}

/// Alias lookup table
///
/// Stores all aliases and provides O(1) resolution.
/// Aliases are globally scoped (SUSHI-compatible behavior).
///
/// # Example
///
/// ```rust
/// use maki_core::semantic::alias::{Alias, AliasTable};
/// use std::path::PathBuf;
///
/// let mut table = AliasTable::new();
///
/// let alias = Alias {
///     name: "$SCT".to_string(),
///     url: "http://snomed.info/sct".to_string(),
///     source_file: PathBuf::from("test.fsh"),
///     source_span: 0..10,
/// };
///
/// table.add_alias(alias).unwrap();
/// assert!(table.is_alias("$SCT"));
/// assert_eq!(table.resolve("$SCT"), Some("http://snomed.info/sct"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct AliasTable {
    /// Map from alias name to alias definition
    aliases: HashMap<String, Alias>,

    /// Index of aliases by source file (for file-level queries)
    by_file: HashMap<PathBuf, Vec<String>>,
}

impl AliasTable {
    /// Create a new empty alias table
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            by_file: HashMap::new(),
        }
    }

    /// Add an alias to the table
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The alias name doesn't start with '$'
    /// - The alias is already defined (stricter than SUSHI)
    /// - The URL is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::alias::{Alias, AliasTable};
    /// use std::path::PathBuf;
    ///
    /// let mut table = AliasTable::new();
    ///
    /// let alias = Alias {
    ///     name: "$SCT".to_string(),
    ///     url: "http://snomed.info/sct".to_string(),
    ///     source_file: PathBuf::from("test.fsh"),
    ///     source_span: 0..10,
    /// };
    ///
    /// assert!(table.add_alias(alias).is_ok());
    /// ```
    pub fn add_alias(&mut self, alias: Alias) -> Result<(), AliasError> {
        // Validate alias name starts with '$'
        if !alias.name.starts_with('$') {
            return Err(AliasError::InvalidAliasName(alias.name.clone()));
        }

        // Validate URL is not empty
        if alias.url.is_empty() {
            return Err(AliasError::InvalidUrl(alias.url.clone()));
        }

        // Check for duplicate definition
        if let Some(existing) = self.aliases.get(&alias.name) {
            return Err(AliasError::DuplicateAlias {
                name: alias.name.clone(),
                file1: existing.source_file.clone(),
                file2: alias.source_file.clone(),
            });
        }

        // Add to by_file index
        self.by_file
            .entry(alias.source_file.clone())
            .or_default()
            .push(alias.name.clone());

        // Add to main table
        self.aliases.insert(alias.name.clone(), alias);

        Ok(())
    }

    /// Resolve an alias to its URL
    ///
    /// Returns `None` if the alias is not defined.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::alias::{Alias, AliasTable};
    /// use std::path::PathBuf;
    ///
    /// let mut table = AliasTable::new();
    ///
    /// let alias = Alias {
    ///     name: "$SCT".to_string(),
    ///     url: "http://snomed.info/sct".to_string(),
    ///     source_file: PathBuf::from("test.fsh"),
    ///     source_span: 0..10,
    /// };
    ///
    /// table.add_alias(alias).unwrap();
    ///
    /// assert_eq!(table.resolve("$SCT"), Some("http://snomed.info/sct"));
    /// assert_eq!(table.resolve("$UNKNOWN"), None);
    /// ```
    pub fn resolve(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(|a| a.url.as_str())
    }

    /// Check if a name is a defined alias
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::alias::{Alias, AliasTable};
    /// use std::path::PathBuf;
    ///
    /// let mut table = AliasTable::new();
    ///
    /// let alias = Alias {
    ///     name: "$SCT".to_string(),
    ///     url: "http://snomed.info/sct".to_string(),
    ///     source_file: PathBuf::from("test.fsh"),
    ///     source_span: 0..10,
    /// };
    ///
    /// table.add_alias(alias).unwrap();
    ///
    /// assert!(table.is_alias("$SCT"));
    /// assert!(!table.is_alias("$UNKNOWN"));
    /// assert!(!table.is_alias("Patient"));
    /// ```
    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    /// Get the full alias definition
    ///
    /// Returns `None` if the alias is not defined.
    pub fn get_alias(&self, name: &str) -> Option<&Alias> {
        self.aliases.get(name)
    }

    /// Get all aliases defined in a specific file
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::alias::{Alias, AliasTable};
    /// use std::path::PathBuf;
    ///
    /// let mut table = AliasTable::new();
    /// let path = PathBuf::from("test.fsh");
    ///
    /// let alias1 = Alias {
    ///     name: "$SCT".to_string(),
    ///     url: "http://snomed.info/sct".to_string(),
    ///     source_file: path.clone(),
    ///     source_span: 0..10,
    /// };
    ///
    /// let alias2 = Alias {
    ///     name: "$LOINC".to_string(),
    ///     url: "http://loinc.org".to_string(),
    ///     source_file: path.clone(),
    ///     source_span: 11..20,
    /// };
    ///
    /// table.add_alias(alias1).unwrap();
    /// table.add_alias(alias2).unwrap();
    ///
    /// let file_aliases = table.get_file_aliases(&path);
    /// assert_eq!(file_aliases.len(), 2);
    /// ```
    pub fn get_file_aliases(&self, file: &std::path::Path) -> Vec<&Alias> {
        self.by_file
            .get(file)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.aliases.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all aliases
    pub fn get_all_aliases(&self) -> Vec<&Alias> {
        self.aliases.values().collect()
    }

    /// Get the number of aliases
    pub fn len(&self) -> usize {
        self.aliases.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.aliases.is_empty()
    }

    /// Resolve an alias or return the original name if not an alias
    ///
    /// This is useful for processing identifiers that may or may not be aliases.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maki_core::semantic::alias::{Alias, AliasTable};
    /// use std::path::PathBuf;
    ///
    /// let mut table = AliasTable::new();
    ///
    /// let alias = Alias {
    ///     name: "$SCT".to_string(),
    ///     url: "http://snomed.info/sct".to_string(),
    ///     source_file: PathBuf::from("test.fsh"),
    ///     source_span: 0..10,
    /// };
    ///
    /// table.add_alias(alias).unwrap();
    ///
    /// assert_eq!(table.resolve_or_original("$SCT"), "http://snomed.info/sct");
    /// assert_eq!(table.resolve_or_original("Patient"), "Patient");
    /// ```
    pub fn resolve_or_original<'a>(&'a self, name: &'a str) -> &'a str {
        self.resolve(name).unwrap_or(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_alias(name: &str, url: &str) -> Alias {
        Alias {
            name: name.to_string(),
            url: url.to_string(),
            source_file: PathBuf::from("test.fsh"),
            source_span: 0..10,
        }
    }

    #[test]
    fn test_add_and_resolve_alias() {
        let mut table = AliasTable::new();

        let alias = make_alias("$SCT", "http://snomed.info/sct");
        table.add_alias(alias).unwrap();

        assert_eq!(table.resolve("$SCT"), Some("http://snomed.info/sct"));
        assert!(table.is_alias("$SCT"));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_invalid_alias_name() {
        let mut table = AliasTable::new();

        // Missing '$' prefix
        let alias = make_alias("SCT", "http://snomed.info/sct");

        let result = table.add_alias(alias);
        assert!(result.is_err());
        assert!(matches!(result, Err(AliasError::InvalidAliasName(_))));
    }

    #[test]
    fn test_duplicate_alias() {
        let mut table = AliasTable::new();

        let alias1 = Alias {
            name: "$SCT".to_string(),
            url: "http://snomed.info/sct".to_string(),
            source_file: PathBuf::from("file1.fsh"),
            source_span: 0..10,
        };

        let alias2 = Alias {
            name: "$SCT".to_string(),
            url: "http://different.url".to_string(),
            source_file: PathBuf::from("file2.fsh"),
            source_span: 0..10,
        };

        table.add_alias(alias1).unwrap();
        let result = table.add_alias(alias2);

        assert!(result.is_err());
        assert!(matches!(result, Err(AliasError::DuplicateAlias { .. })));
    }

    #[test]
    fn test_undefined_alias() {
        let table = AliasTable::new();

        assert_eq!(table.resolve("$UNKNOWN"), None);
        assert!(!table.is_alias("$UNKNOWN"));
    }

    #[test]
    fn test_resolve_or_original() {
        let mut table = AliasTable::new();

        let alias = make_alias("$SCT", "http://snomed.info/sct");
        table.add_alias(alias).unwrap();

        assert_eq!(table.resolve_or_original("$SCT"), "http://snomed.info/sct");
        assert_eq!(table.resolve_or_original("Patient"), "Patient");
        assert_eq!(table.resolve_or_original("$UNKNOWN"), "$UNKNOWN");
    }

    #[test]
    fn test_get_file_aliases() {
        let mut table = AliasTable::new();
        let path = PathBuf::from("test.fsh");

        let alias1 = Alias {
            name: "$SCT".to_string(),
            url: "http://snomed.info/sct".to_string(),
            source_file: path.clone(),
            source_span: 0..10,
        };

        let alias2 = Alias {
            name: "$LOINC".to_string(),
            url: "http://loinc.org".to_string(),
            source_file: path.clone(),
            source_span: 11..20,
        };

        table.add_alias(alias1).unwrap();
        table.add_alias(alias2).unwrap();

        let file_aliases = table.get_file_aliases(&path);
        assert_eq!(file_aliases.len(), 2);
    }

    #[test]
    fn test_get_all_aliases() {
        let mut table = AliasTable::new();

        table
            .add_alias(make_alias("$SCT", "http://snomed.info/sct"))
            .unwrap();
        table
            .add_alias(make_alias("$LOINC", "http://loinc.org"))
            .unwrap();
        table
            .add_alias(make_alias("$UCUM", "http://unitsofmeasure.org"))
            .unwrap();

        let all_aliases = table.get_all_aliases();
        assert_eq!(all_aliases.len(), 3);
    }

    #[test]
    fn test_empty_url() {
        let mut table = AliasTable::new();

        let alias = make_alias("$TEST", "");

        let result = table.add_alias(alias);
        assert!(result.is_err());
        assert!(matches!(result, Err(AliasError::InvalidUrl(_))));
    }

    #[test]
    fn test_multiple_files() {
        let mut table = AliasTable::new();

        let alias1 = Alias {
            name: "$SCT".to_string(),
            url: "http://snomed.info/sct".to_string(),
            source_file: PathBuf::from("file1.fsh"),
            source_span: 0..10,
        };

        let alias2 = Alias {
            name: "$LOINC".to_string(),
            url: "http://loinc.org".to_string(),
            source_file: PathBuf::from("file2.fsh"),
            source_span: 0..10,
        };

        table.add_alias(alias1).unwrap();
        table.add_alias(alias2).unwrap();

        assert_eq!(table.get_file_aliases(&PathBuf::from("file1.fsh")).len(), 1);
        assert_eq!(table.get_file_aliases(&PathBuf::from("file2.fsh")).len(), 1);
        assert_eq!(table.len(), 2);
    }
}
