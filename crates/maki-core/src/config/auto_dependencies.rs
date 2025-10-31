//! Automatic dependency injection for FHIR Implementation Guides.
//!
//! This module provides automatic injection of standard FHIR packages based on
//! the FHIR version, matching SUSHI's behavior for FHIR IG builds.
//!
//! ## Standard Dependencies
//!
//! SUSHI automatically includes these packages for all IGs:
//! - `hl7.fhir.uv.tools.r{X}`: IG Publisher tooling resources
//! - `hl7.terminology.r{X}`: Standard terminologies (SNOMED, LOINC, RxNorm, etc.)
//! - `hl7.fhir.uv.extensions.r{X}`: Common FHIR extensions
//!
//! These are essential for building IGs and should not need to be explicitly
//! declared in the configuration.

use crate::config::DependencyVersion;

/// Package identifier and version for an auto-injected dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDependency {
    pub package_id: String,
    pub version: String,
}

impl AutoDependency {
    pub fn new(package_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            package_id: package_id.into(),
            version: version.into(),
        }
    }
}

/// Get automatic dependencies for a FHIR version
///
/// Returns the standard packages that SUSHI automatically includes for all IGs.
/// These packages provide tooling, terminology, and common extensions.
///
/// # Arguments
///
/// * `fhir_version` - The FHIR version string (e.g., "4.0.1", "5.0.0")
///
/// # Returns
///
/// A vector of auto-dependencies (package ID and version) that should be loaded.
///
/// # Examples
///
/// ```
/// use maki_core::config::auto_dependencies::get_auto_dependencies;
///
/// let deps = get_auto_dependencies("4.0.1");
/// assert_eq!(deps.len(), 3);
/// ```
pub fn get_auto_dependencies(fhir_version: &str) -> Vec<AutoDependency> {
    // Determine FHIR release family
    let (release_id, tools_version, terminology_version, extensions_version) = if fhir_version
        .starts_with("4.0")
    {
        // R4
        (
            "r4",
            "0.2.0",      // tools
            "6.1.0",      // terminology (latest stable)
            "5.1.0",      // extensions
        )
    } else if fhir_version.starts_with("4.3") {
        // R4B
        (
            "r4b",
            "0.1.0",      // tools
            "6.1.0",      // terminology
            "5.1.0",      // extensions
        )
    } else if fhir_version.starts_with("5.0") {
        // R5
        (
            "r5",
            "0.3.0",      // tools
            "6.1.0",      // terminology
            "5.1.0",      // extensions
        )
    } else if fhir_version.starts_with("6.") {
        // R6 (ballot)
        (
            "r6",
            "0.1.0",      // tools (may not exist yet)
            "6.1.0",      // terminology
            "5.1.0",      // extensions (may not exist yet)
        )
    } else {
        // Default to R4 for unknown versions
        tracing::warn!(
            "Unknown FHIR version '{}', defaulting to R4 auto-dependencies",
            fhir_version
        );
        (
            "r4",
            "0.2.0",
            "6.1.0",
            "5.1.0",
        )
    };

    vec![
        AutoDependency::new(
            format!("hl7.fhir.uv.tools.{}", release_id),
            tools_version,
        ),
        AutoDependency::new(
            format!("hl7.terminology.{}", release_id),
            terminology_version,
        ),
        AutoDependency::new(
            format!("hl7.fhir.uv.extensions.{}", release_id),
            extensions_version,
        ),
    ]
}

/// Parse a dependency specification, handling NPM aliases
///
/// NPM aliases allow package renaming using the format: `alias@npm:actual-package`.
/// This is used when an IG needs multiple versions of the same package.
///
/// # Arguments
///
/// * `package_id` - The package identifier (may contain NPM alias)
/// * `spec` - The dependency version specification
///
/// # Returns
///
/// A tuple of (actual_package_name, version)
///
/// # Examples
///
/// ```
/// use maki_core::config::{DependencyVersion, auto_dependencies::parse_dependency_spec};
///
/// // Normal dependency
/// let spec = DependencyVersion::Simple("6.1.0".to_string());
/// let (pkg, ver) = parse_dependency_spec("hl7.fhir.us.core", &spec).unwrap();
/// assert_eq!(pkg, "hl7.fhir.us.core");
/// assert_eq!(ver, "6.1.0");
///
/// // NPM alias
/// let spec = DependencyVersion::Simple("3.1.0".to_string());
/// let (pkg, ver) = parse_dependency_spec("us-core-3@npm:hl7.fhir.us.core", &spec).unwrap();
/// assert_eq!(pkg, "hl7.fhir.us.core");
/// assert_eq!(ver, "3.1.0");
/// ```
pub fn parse_dependency_spec(
    package_id: &str,
    spec: &DependencyVersion,
) -> Result<(String, String), String> {
    // Handle NPM alias: "alias@npm:actual-package"
    let actual_package = if let Some(npm_pos) = package_id.find("@npm:") {
        &package_id[npm_pos + 5..]
    } else {
        package_id
    };

    // Extract version from spec
    let version = match spec {
        DependencyVersion::Simple(v) => v.clone(),
        DependencyVersion::Complex { version, .. } => version.clone(),
    };

    Ok((actual_package.to_string(), version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_dependencies_r4() {
        let deps = get_auto_dependencies("4.0.1");
        assert_eq!(deps.len(), 3);

        // Check tools package
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.fhir.uv.tools.r4"));

        // Check terminology package
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.terminology.r4"));

        // Check extensions package
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.fhir.uv.extensions.r4"));
    }

    #[test]
    fn test_auto_dependencies_r4b() {
        let deps = get_auto_dependencies("4.3.0");
        assert_eq!(deps.len(), 3);
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.fhir.uv.tools.r4b"));
    }

    #[test]
    fn test_auto_dependencies_r5() {
        let deps = get_auto_dependencies("5.0.0");
        assert_eq!(deps.len(), 3);
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.fhir.uv.tools.r5"));
    }

    #[test]
    fn test_auto_dependencies_r6() {
        let deps = get_auto_dependencies("6.0.0-ballot");
        assert_eq!(deps.len(), 3);
        assert!(deps
            .iter()
            .any(|d| d.package_id == "hl7.fhir.uv.tools.r6"));
    }

    #[test]
    fn test_parse_normal_dependency() {
        let spec = DependencyVersion::Simple("6.1.0".to_string());
        let (pkg, ver) = parse_dependency_spec("hl7.fhir.us.core", &spec).unwrap();
        assert_eq!(pkg, "hl7.fhir.us.core");
        assert_eq!(ver, "6.1.0");
    }

    #[test]
    fn test_parse_npm_alias() {
        let spec = DependencyVersion::Simple("3.1.0".to_string());
        let (pkg, ver) = parse_dependency_spec("us-core-3@npm:hl7.fhir.us.core", &spec).unwrap();
        assert_eq!(pkg, "hl7.fhir.us.core");
        assert_eq!(ver, "3.1.0");
    }

    #[test]
    fn test_parse_complex_dependency() {
        let spec = DependencyVersion::Complex {
            version: "1.0.0".to_string(),
            uri: Some("http://example.org".to_string()),
            reason: Some("Required for profiles".to_string()),
            extension: None,
        };
        let (pkg, ver) = parse_dependency_spec("my.custom.ig", &spec).unwrap();
        assert_eq!(pkg, "my.custom.ig");
        assert_eq!(ver, "1.0.0");
    }

    #[test]
    fn test_parse_complex_dependency_with_alias() {
        let spec = DependencyVersion::Complex {
            version: "2.0.0".to_string(),
            uri: Some("http://example.org".to_string()),
            reason: None,
            extension: None,
        };
        let (pkg, ver) = parse_dependency_spec("alias@npm:real.package", &spec).unwrap();
        assert_eq!(pkg, "real.package");
        assert_eq!(ver, "2.0.0");
    }
}
