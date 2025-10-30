//! FHIR version support and resolution
//!
//! This module extends the base `FhirRelease` enum with version parsing,
//! wildcard resolution (latest, current, dev), and semver range support.
//!
//! # Overview
//!
//! FHIR has multiple releases (R4, R4B, R5, R6), each with specific version numbers.
//! This module provides utilities to:
//! - Parse version strings to `FhirRelease` (e.g., "4.0.1" -> R4)
//! - Resolve version wildcards (latest, current, dev)
//! - Handle semver ranges (e.g., "4.0.x")
//! - Map releases to core package names
//!
//! # Example
//!
//! ```rust
//! use maki_core::canonical::version::{FhirVersionExt, VersionSpecifier};
//! use maki_core::canonical::FhirRelease;
//!
//! // Parse version string
//! let release = FhirRelease::from_version_string("4.0.1").unwrap();
//! assert_eq!(release, FhirRelease::R4);
//!
//! // Get core package name
//! assert_eq!(release.core_package_name(), "hl7.fhir.r4.core");
//!
//! // Get canonical version
//! assert_eq!(release.canonical_version(), "4.0.1");
//! ```

use crate::canonical::FhirRelease;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during version operations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VersionError {
    /// Invalid FHIR version string
    #[error("Invalid FHIR version: {0}")]
    InvalidFhirVersion(String),

    /// Invalid version specifier
    #[error("Invalid version specifier: {0}")]
    InvalidSpecifier(String),

    /// Version not found for package
    #[error("Version not found: {package}#{version}")]
    VersionNotFound { package: String, version: String },

    /// No versions available for package
    #[error("No versions available for package: {0}")]
    NoVersionsAvailable(String),

    /// Version conflict detected
    #[error("Version conflict: {0}")]
    VersionConflict(String),

    /// Semver parsing error
    #[error("Semver parsing error: {0}")]
    SemverError(String),
}

impl From<semver::Error> for VersionError {
    fn from(err: semver::Error) -> Self {
        VersionError::SemverError(err.to_string())
    }
}

/// Extension trait for `FhirRelease` to add version-related methods
pub trait FhirVersionExt {
    /// Parse a FHIR version string to a release
    ///
    /// # Examples
    ///
    /// ```rust
    /// use maki_core::canonical::FhirRelease;
    /// use maki_core::canonical::version::FhirVersionExt;
    ///
    /// assert_eq!(FhirRelease::from_version_string("4.0.1").unwrap(), FhirRelease::R4);
    /// assert_eq!(FhirRelease::from_version_string("4.3.0").unwrap(), FhirRelease::R4B);
    /// assert_eq!(FhirRelease::from_version_string("5.0.0").unwrap(), FhirRelease::R5);
    /// ```
    fn from_version_string(s: &str) -> Result<Self, VersionError>
    where
        Self: Sized;

    /// Get the canonical version string for this release
    fn canonical_version(&self) -> &'static str;

    /// Get the core package name for this release
    fn core_package_name(&self) -> &'static str;

    /// Check if this release is compatible with another
    fn is_compatible_with(&self, other: &Self) -> bool;
}

impl FhirVersionExt for FhirRelease {
    fn from_version_string(s: &str) -> Result<Self, VersionError> {
        let normalized = s.trim().to_lowercase();

        // Handle release names (r4, r4b, r5, r6)
        if normalized == "r4" {
            return Ok(FhirRelease::R4);
        }
        if normalized == "r4b" {
            return Ok(FhirRelease::R4B);
        }
        if normalized == "r5" {
            return Ok(FhirRelease::R5);
        }
        if normalized == "r6" {
            return Ok(FhirRelease::R6);
        }

        // Handle version numbers
        if s.starts_with("4.0") {
            Ok(FhirRelease::R4)
        } else if s.starts_with("4.3") {
            Ok(FhirRelease::R4B)
        } else if s.starts_with("5.0") {
            Ok(FhirRelease::R5)
        } else if s.starts_with("6.0") {
            Ok(FhirRelease::R6)
        } else {
            Err(VersionError::InvalidFhirVersion(s.to_string()))
        }
    }

    fn canonical_version(&self) -> &'static str {
        match self {
            FhirRelease::R4 => "4.0.1",
            FhirRelease::R4B => "4.3.0",
            FhirRelease::R5 => "5.0.0",
            FhirRelease::R6 => "6.0.0",
        }
    }

    fn core_package_name(&self) -> &'static str {
        match self {
            FhirRelease::R4 => "hl7.fhir.r4.core",
            FhirRelease::R4B => "hl7.fhir.r4b.core",
            FhirRelease::R5 => "hl7.fhir.r5.core",
            FhirRelease::R6 => "hl7.fhir.r6.core",
        }
    }

    fn is_compatible_with(&self, other: &Self) -> bool {
        // For now, only exact match is compatible
        // In the future, we might allow R4 -> R4B compatibility
        self == other
    }
}

/// Version specifier supporting wildcards and ranges
///
/// Represents different ways to specify package versions:
/// - Exact versions (e.g., "4.0.1")
/// - Wildcards (latest, current, dev)
/// - Semver ranges (e.g., "4.0.x", ">=4.0.0")
#[derive(Debug, Clone, PartialEq)]
pub enum VersionSpecifier {
    /// Exact semantic version
    Exact(Version),
    /// Latest version (highest version number)
    Latest,
    /// Current version (latest stable release, no pre-release tags)
    Current,
    /// Development version (latest version including pre-releases)
    Dev,
    /// Semver range (e.g., "4.0.x", ">=4.0.0")
    Range(VersionReq),
}

impl VersionSpecifier {
    /// Resolve this specifier to a concrete version
    ///
    /// # Arguments
    ///
    /// * `available` - List of available versions to choose from
    ///
    /// # Returns
    ///
    /// The highest matching version, or None if no match found
    pub fn resolve(&self, available: &[Version]) -> Option<Version> {
        if available.is_empty() {
            return None;
        }

        match self {
            VersionSpecifier::Exact(v) => {
                // Return exact match if available
                available.iter().find(|av| *av == v).cloned()
            }
            VersionSpecifier::Latest => {
                // Return highest version
                available.iter().max().cloned()
            }
            VersionSpecifier::Current => {
                // Return highest stable version (no pre-release)
                available.iter().filter(|v| v.pre.is_empty()).max().cloned()
            }
            VersionSpecifier::Dev => {
                // Return highest version (including pre-releases)
                available.iter().max().cloned()
            }
            VersionSpecifier::Range(req) => {
                // Return highest version matching the requirement
                available.iter().filter(|v| req.matches(v)).max().cloned()
            }
        }
    }
}

impl FromStr for VersionSpecifier {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase();

        match normalized.as_str() {
            "latest" => Ok(VersionSpecifier::Latest),
            "current" => Ok(VersionSpecifier::Current),
            "dev" => Ok(VersionSpecifier::Dev),
            _ => {
                // Try to parse as exact version first
                if let Ok(version) = Version::parse(s) {
                    return Ok(VersionSpecifier::Exact(version));
                }

                // Try to parse as version requirement/range
                if let Ok(req) = VersionReq::parse(s) {
                    return Ok(VersionSpecifier::Range(req));
                }

                Err(VersionError::InvalidSpecifier(s.to_string()))
            }
        }
    }
}

/// Version resolver for managing package versions
///
/// Maintains a registry of available package versions and resolves
/// version specifiers to concrete versions.
///
/// # Example
///
/// ```rust
/// use maki_core::canonical::version::{VersionResolver, VersionSpecifier};
/// use semver::Version;
///
/// let mut resolver = VersionResolver::new();
/// resolver.register_package("hl7.fhir.r4.core", vec![
///     Version::parse("4.0.0").unwrap(),
///     Version::parse("4.0.1").unwrap(),
/// ]);
///
/// let latest = VersionSpecifier::Latest;
/// let resolved = resolver.resolve("hl7.fhir.r4.core", &latest).unwrap();
/// assert_eq!(resolved, Version::parse("4.0.1").unwrap());
/// ```
pub struct VersionResolver {
    /// Map of package name to available versions (sorted)
    package_versions: HashMap<String, Vec<Version>>,
}

impl VersionResolver {
    /// Create a new empty version resolver
    pub fn new() -> Self {
        Self {
            package_versions: HashMap::new(),
        }
    }

    /// Register available versions for a package
    ///
    /// Versions are automatically sorted in ascending order.
    pub fn register_package(&mut self, name: impl Into<String>, mut versions: Vec<Version>) {
        versions.sort();
        self.package_versions.insert(name.into(), versions);
    }

    /// Resolve a version specifier to a concrete version
    ///
    /// # Arguments
    ///
    /// * `package` - Package name
    /// * `specifier` - Version specifier to resolve
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Package is not registered
    /// - No versions match the specifier
    pub fn resolve(
        &self,
        package: &str,
        specifier: &VersionSpecifier,
    ) -> Result<Version, VersionError> {
        let versions = self
            .package_versions
            .get(package)
            .ok_or_else(|| VersionError::NoVersionsAvailable(package.to_string()))?;

        specifier
            .resolve(versions)
            .ok_or_else(|| VersionError::VersionNotFound {
                package: package.to_string(),
                version: format!("{:?}", specifier),
            })
    }

    /// Get the latest stable version for a package
    ///
    /// Returns the highest version without pre-release tags.
    pub fn latest_stable(&self, package: &str) -> Option<Version> {
        let versions = self.package_versions.get(package)?;
        VersionSpecifier::Current.resolve(versions)
    }

    /// Get the current version for a package
    ///
    /// Alias for `latest_stable()`. Returns the latest official release.
    pub fn current(&self, package: &str) -> Option<Version> {
        self.latest_stable(package)
    }

    /// Get the development version for a package
    ///
    /// Returns the highest version including pre-releases.
    pub fn dev(&self, package: &str) -> Option<Version> {
        let versions = self.package_versions.get(package)?;
        VersionSpecifier::Dev.resolve(versions)
    }

    /// Get all registered package names
    pub fn packages(&self) -> Vec<&str> {
        self.package_versions.keys().map(|s| s.as_str()).collect()
    }

    /// Get all versions for a package
    pub fn versions(&self, package: &str) -> Option<&[Version]> {
        self.package_versions.get(package).map(|v| v.as_slice())
    }
}

impl Default for VersionResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_version_parsing() {
        assert_eq!(
            FhirRelease::from_version_string("4.0.1").unwrap(),
            FhirRelease::R4
        );
        assert_eq!(
            FhirRelease::from_version_string("4.0.0").unwrap(),
            FhirRelease::R4
        );
        assert_eq!(
            FhirRelease::from_version_string("4.3.0").unwrap(),
            FhirRelease::R4B
        );
        assert_eq!(
            FhirRelease::from_version_string("5.0.0").unwrap(),
            FhirRelease::R5
        );
        assert_eq!(
            FhirRelease::from_version_string("6.0.0").unwrap(),
            FhirRelease::R6
        );
    }

    #[test]
    fn test_fhir_version_parsing_release_names() {
        assert_eq!(
            FhirRelease::from_version_string("R4").unwrap(),
            FhirRelease::R4
        );
        assert_eq!(
            FhirRelease::from_version_string("r4").unwrap(),
            FhirRelease::R4
        );
        assert_eq!(
            FhirRelease::from_version_string("R4B").unwrap(),
            FhirRelease::R4B
        );
        assert_eq!(
            FhirRelease::from_version_string("R5").unwrap(),
            FhirRelease::R5
        );
    }

    #[test]
    fn test_fhir_version_invalid() {
        assert!(FhirRelease::from_version_string("3.0.0").is_err());
        assert!(FhirRelease::from_version_string("invalid").is_err());
    }

    #[test]
    fn test_canonical_version() {
        assert_eq!(FhirRelease::R4.canonical_version(), "4.0.1");
        assert_eq!(FhirRelease::R4B.canonical_version(), "4.3.0");
        assert_eq!(FhirRelease::R5.canonical_version(), "5.0.0");
        assert_eq!(FhirRelease::R6.canonical_version(), "6.0.0");
    }

    #[test]
    fn test_core_package_name() {
        assert_eq!(FhirRelease::R4.core_package_name(), "hl7.fhir.r4.core");
        assert_eq!(FhirRelease::R4B.core_package_name(), "hl7.fhir.r4b.core");
        assert_eq!(FhirRelease::R5.core_package_name(), "hl7.fhir.r5.core");
        assert_eq!(FhirRelease::R6.core_package_name(), "hl7.fhir.r6.core");
    }

    #[test]
    fn test_version_compatibility() {
        assert!(FhirRelease::R4.is_compatible_with(&FhirRelease::R4));
        assert!(!FhirRelease::R4.is_compatible_with(&FhirRelease::R5));
        assert!(!FhirRelease::R4.is_compatible_with(&FhirRelease::R4B));
    }

    #[test]
    fn test_version_specifier_parsing() {
        let latest = VersionSpecifier::from_str("latest").unwrap();
        assert_eq!(latest, VersionSpecifier::Latest);

        let current = VersionSpecifier::from_str("current").unwrap();
        assert_eq!(current, VersionSpecifier::Current);

        let dev = VersionSpecifier::from_str("dev").unwrap();
        assert_eq!(dev, VersionSpecifier::Dev);

        let exact = VersionSpecifier::from_str("4.0.1").unwrap();
        assert!(matches!(exact, VersionSpecifier::Exact(_)));

        let range = VersionSpecifier::from_str("4.0.x").unwrap();
        assert!(matches!(range, VersionSpecifier::Range(_)));
    }

    #[test]
    fn test_version_specifier_invalid() {
        assert!(VersionSpecifier::from_str("not-a-version").is_err());
    }

    #[test]
    fn test_version_resolution_latest() {
        let available = vec![
            Version::parse("4.0.0").unwrap(),
            Version::parse("4.0.1").unwrap(),
            Version::parse("4.1.0").unwrap(),
        ];

        let resolved = VersionSpecifier::Latest.resolve(&available).unwrap();
        assert_eq!(resolved, Version::parse("4.1.0").unwrap());
    }

    #[test]
    fn test_version_resolution_current() {
        let available = vec![
            Version::parse("4.0.0").unwrap(),
            Version::parse("4.0.1").unwrap(),
            Version::parse("4.1.0-snapshot").unwrap(),
        ];

        let resolved = VersionSpecifier::Current.resolve(&available).unwrap();
        assert_eq!(resolved, Version::parse("4.0.1").unwrap());
    }

    #[test]
    fn test_version_resolution_dev() {
        let available = vec![
            Version::parse("4.0.0").unwrap(),
            Version::parse("4.0.1").unwrap(),
            Version::parse("4.1.0-snapshot").unwrap(),
        ];

        let resolved = VersionSpecifier::Dev.resolve(&available).unwrap();
        assert_eq!(resolved, Version::parse("4.1.0-snapshot").unwrap());
    }

    #[test]
    fn test_version_resolution_range() {
        let range = VersionSpecifier::from_str("4.0.x").unwrap();
        let available = vec![
            Version::parse("4.0.0").unwrap(),
            Version::parse("4.0.1").unwrap(),
            Version::parse("4.1.0").unwrap(),
        ];

        let resolved = range.resolve(&available).unwrap();
        assert_eq!(resolved, Version::parse("4.0.1").unwrap());
    }

    #[test]
    fn test_version_resolver() {
        let mut resolver = VersionResolver::new();

        resolver.register_package(
            "hl7.fhir.r4.core",
            vec![
                Version::parse("4.0.0").unwrap(),
                Version::parse("4.0.1").unwrap(),
            ],
        );

        // Test latest
        let latest = VersionSpecifier::Latest;
        let resolved = resolver.resolve("hl7.fhir.r4.core", &latest).unwrap();
        assert_eq!(resolved, Version::parse("4.0.1").unwrap());

        // Test exact
        let exact = VersionSpecifier::Exact(Version::parse("4.0.0").unwrap());
        let resolved = resolver.resolve("hl7.fhir.r4.core", &exact).unwrap();
        assert_eq!(resolved, Version::parse("4.0.0").unwrap());
    }

    #[test]
    fn test_version_resolver_not_found() {
        let resolver = VersionResolver::new();
        let result = resolver.resolve("nonexistent", &VersionSpecifier::Latest);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_resolver_helpers() {
        let mut resolver = VersionResolver::new();
        resolver.register_package(
            "test.package",
            vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.1.0").unwrap(),
                Version::parse("2.0.0-beta").unwrap(),
            ],
        );

        // latest_stable should exclude pre-releases
        assert_eq!(
            resolver.latest_stable("test.package").unwrap(),
            Version::parse("1.1.0").unwrap()
        );

        // current is alias for latest_stable
        assert_eq!(
            resolver.current("test.package").unwrap(),
            Version::parse("1.1.0").unwrap()
        );

        // dev includes pre-releases
        assert_eq!(
            resolver.dev("test.package").unwrap(),
            Version::parse("2.0.0-beta").unwrap()
        );
    }
}
