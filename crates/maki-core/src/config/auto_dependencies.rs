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
    let (release_id, tools_version, terminology_version, extensions_version) =
        if fhir_version.starts_with("4.0") {
            // R4
            (
                "r4", "0.2.0", // tools
                "6.1.0", // terminology (latest stable)
                "5.1.0", // extensions
            )
        } else if fhir_version.starts_with("4.3") {
            // R4B
            (
                "r4b", "0.1.0", // tools
                "6.1.0", // terminology
                "5.1.0", // extensions
            )
        } else if fhir_version.starts_with("5.0") {
            // R5
            (
                "r5", "0.3.0", // tools
                "6.1.0", // terminology
                "5.1.0", // extensions
            )
        } else if fhir_version.starts_with("6.") {
            // R6 (ballot)
            (
                "r6", "0.1.0", // tools (may not exist yet)
                "6.1.0", // terminology
                "5.1.0", // extensions (may not exist yet)
            )
        } else {
            // Default to R4 for unknown versions
            tracing::warn!(
                "Unknown FHIR version '{}', defaulting to R4 auto-dependencies",
                fhir_version
            );
            ("r4", "0.2.0", "6.1.0", "5.1.0")
        };

    vec![
        AutoDependency::new(format!("hl7.fhir.uv.tools.{}", release_id), tools_version),
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

/// Redirect legacy `hl7.fhir.extensions.r{X}:<source-version>` package
/// declarations to the SUSHI 3.19 official cross-version extension packages
/// (`hl7.fhir.uv.xver-r{X}.r{Y}`).
///
/// SUSHI 3.19 retired the `hl7.fhir.extensions.r4`/`.r4b`/`.r5` family; the
/// official cross-version extensions are now published under
/// `hl7.fhir.uv.xver-<source>.<target>` where `<source>` is the FHIR release
/// the extensions originate from and `<target>` is the IG's own release.
///
/// # Arguments
///
/// * `package_id` — declared dependency package id (e.g. `hl7.fhir.extensions.r5`).
/// * `pinned_fhir_version` — version pin from the dependency entry
///   (e.g. `4.0.1`); identifies the target release the extensions should
///   be back-ported to.
///
/// # Returns
///
/// `Some((new_pkg, new_version))` if the package matched a known legacy id,
/// otherwise `None`. The redirected version is `current` so `cargo`-style
/// pinning is left to the resolver.
///
/// # Examples
///
/// ```
/// use maki_core::config::auto_dependencies::redirect_legacy_extension_package;
///
/// // R5 extensions used inside an R4 IG → xver-r5.r4
/// let r = redirect_legacy_extension_package("hl7.fhir.extensions.r5", "4.0.1");
/// assert_eq!(r.unwrap().0, "hl7.fhir.uv.xver-r5.r4");
///
/// // R4 extensions used inside an R5 IG → xver-r4.r5
/// let r = redirect_legacy_extension_package("hl7.fhir.extensions.r4", "5.0.0");
/// assert_eq!(r.unwrap().0, "hl7.fhir.uv.xver-r4.r5");
/// ```
pub fn redirect_legacy_extension_package(
    package_id: &str,
    pinned_fhir_version: &str,
) -> Option<(String, String)> {
    let source_release = match package_id {
        "hl7.fhir.extensions.r4" => "r4",
        "hl7.fhir.extensions.r4b" => "r4b",
        "hl7.fhir.extensions.r5" => "r5",
        "hl7.fhir.extensions.r6" => "r6",
        _ => return None,
    };

    let target_release = if pinned_fhir_version.starts_with("4.0") {
        "r4"
    } else if pinned_fhir_version.starts_with("4.3") {
        "r4b"
    } else if pinned_fhir_version.starts_with("5.") {
        "r5"
    } else if pinned_fhir_version.starts_with("6.") {
        "r6"
    } else {
        // Unknown target — let the resolver fail rather than guess.
        return None;
    };

    if source_release == target_release {
        // No back-port needed; legacy package id was redundant.
        return None;
    }

    Some((
        format!("hl7.fhir.uv.xver-{}.{}", source_release, target_release),
        "current".to_string(),
    ))
}

/// URL-encode the literal `[x]` choice marker that appears in cross-version
/// extension URLs, matching SUSHI 3.14's normalisation rule.
///
/// FHIR canonical URLs that target choice elements (e.g.
/// `http://hl7.org/fhir/5.0/StructureDefinition/extension-Questionnaire.versionAlgorithm[x]`)
/// must percent-encode the `[` and `]` characters before being written
/// into the published IG, otherwise some validators reject the URL as
/// malformed. SUSHI rewrites `[x]` to `%5Bx%5D` (and only those exact
/// brackets — slice-name brackets remain literal).
///
/// The function is a no-op for URLs without a `[x]` marker.
///
/// # Examples
///
/// ```
/// use maki_core::config::auto_dependencies::encode_choice_marker_in_url;
///
/// let url = "http://hl7.org/fhir/StructureDefinition/Questionnaire.versionAlgorithm[x]";
/// assert_eq!(
///     encode_choice_marker_in_url(url),
///     "http://hl7.org/fhir/StructureDefinition/Questionnaire.versionAlgorithm%5Bx%5D"
/// );
///
/// let url = "http://hl7.org/fhir/StructureDefinition/Patient";
/// assert_eq!(encode_choice_marker_in_url(url), url);
/// ```
pub fn encode_choice_marker_in_url(url: &str) -> String {
    url.replace("[x]", "%5Bx%5D")
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
        assert!(deps.iter().any(|d| d.package_id == "hl7.fhir.uv.tools.r4"));

        // Check terminology package
        assert!(deps.iter().any(|d| d.package_id == "hl7.terminology.r4"));

        // Check extensions package
        assert!(
            deps.iter()
                .any(|d| d.package_id == "hl7.fhir.uv.extensions.r4")
        );
    }

    #[test]
    fn test_auto_dependencies_r4b() {
        let deps = get_auto_dependencies("4.3.0");
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.package_id == "hl7.fhir.uv.tools.r4b"));
    }

    #[test]
    fn test_auto_dependencies_r5() {
        let deps = get_auto_dependencies("5.0.0");
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.package_id == "hl7.fhir.uv.tools.r5"));
    }

    #[test]
    fn test_auto_dependencies_r6() {
        let deps = get_auto_dependencies("6.0.0-ballot");
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.package_id == "hl7.fhir.uv.tools.r6"));
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
