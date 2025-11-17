//! Canonical package integration for maki-decompiler
//!
//! This module provides helper functions for setting up canonical manager
//! integration, creating ResourceLake with DefinitionSession, and parsing
//! package dependencies from CLI arguments.

use crate::error::{Error, Result};
use crate::lake::ResourceLake;
use maki_core::canonical::{
    CanonicalFacade, CanonicalOptions, DefinitionSession, FhirRelease, PackageCoordinate,
};
use std::sync::Arc;

/// Initialize canonical manager and create DefinitionSession
///
/// This function sets up the canonical facade with the specified FHIR release
/// and ensures that the core packages are installed. Additional dependencies
/// can be provided via the `dependencies` parameter.
///
/// # Arguments
///
/// * `fhir_release` - FHIR release to use (R4, R4B, R5, R6)
/// * `dependencies` - Additional package dependencies to install
///
/// # Returns
///
/// Arc-wrapped DefinitionSession ready for use with ResourceLake
pub async fn setup_canonical_environment(
    fhir_release: FhirRelease,
    dependencies: Vec<PackageCoordinate>,
) -> Result<Arc<DefinitionSession>> {
    // 1. Initialize CanonicalFacade (creates ~/.maki/index/fhir.db)
    let mut options = CanonicalOptions {
        default_release: fhir_release,
        auto_install_core: true,
        preload_packages: dependencies,
        ..Default::default()
    };

    // In CI or test environments, use quick_init
    if std::env::var("CI").is_ok() || std::env::var("MAKI_QUICK_INIT").is_ok() {
        options.quick_init = true;
    }

    let facade = Arc::new(
        CanonicalFacade::new(options)
            .await
            .map_err(|e| Error::CanonicalError(e.to_string()))?,
    );

    // 2. Create DefinitionSession for FHIR release
    let session = facade
        .session(vec![fhir_release])
        .await
        .map_err(|e| Error::CanonicalError(e.to_string()))?;

    Ok(Arc::new(session))
}

/// Create ResourceLake with canonical manager integration
///
/// This is a convenience function that sets up the canonical environment
/// and creates a ResourceLake with the provided session.
///
/// # Arguments
///
/// * `fhir_release` - FHIR release to use
/// * `dependencies` - Package dependencies to install
///
/// # Returns
///
/// ResourceLake ready for loading resources
pub async fn create_lake_with_session(
    fhir_release: FhirRelease,
    dependencies: Vec<PackageCoordinate>,
) -> Result<ResourceLake> {
    let session = setup_canonical_environment(fhir_release, dependencies).await?;
    Ok(ResourceLake::new(session))
}

/// Parse FHIR release from string
///
/// Accepts both short forms (R4, R5) and version strings (4.0.1, 5.0.0).
///
/// # Arguments
///
/// * `version` - Version string to parse
///
/// # Returns
///
/// FhirRelease enum variant
pub fn parse_fhir_release(version: &str) -> Result<FhirRelease> {
    match version.to_uppercase().as_str() {
        "R4" | "4.0.1" => Ok(FhirRelease::R4),
        "R4B" | "4.3.0" => Ok(FhirRelease::R4B),
        "R5" | "5.0.0" => Ok(FhirRelease::R5),
        "R6" | "6.0.0" => Ok(FhirRelease::R6),
        _ => Err(Error::InvalidFhirVersion(version.to_string())),
    }
}

/// Parse package dependency from string
///
/// Expects format: `package-name@version` (e.g., `hl7.fhir.us.core@5.0.1`)
///
/// # Arguments
///
/// * `spec` - Package specification string
///
/// # Returns
///
/// PackageCoordinate with name and version
pub fn parse_package_spec(spec: &str) -> Result<PackageCoordinate> {
    let parts: Vec<&str> = spec.split('@').collect();

    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(Error::InvalidPackageSpec(spec.to_string()));
    }

    Ok(PackageCoordinate::new(parts[0], parts[1]))
}

/// Parse CLI arguments for FHIR release and dependencies
///
/// This function handles the complete CLI argument parsing for setting up
/// canonical manager integration.
///
/// # Arguments
///
/// * `fhir_version` - FHIR version string (e.g., "R4", "4.0.1")
/// * `dependency_specs` - Vector of dependency specs (e.g., ["hl7.fhir.us.core@5.0.1"])
///
/// # Returns
///
/// Tuple of (FhirRelease, Vec<PackageCoordinate>)
pub fn parse_cli_dependencies(
    fhir_version: &str,
    dependency_specs: &[String],
) -> Result<(FhirRelease, Vec<PackageCoordinate>)> {
    let release = parse_fhir_release(fhir_version)?;

    // Core package is automatically installed by CanonicalFacade with auto_install_core=true
    // We just need to parse user-provided dependencies
    let mut dependencies = Vec::new();

    for spec in dependency_specs {
        let pkg = parse_package_spec(spec)?;
        dependencies.push(pkg);
    }

    Ok((release, dependencies))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fhir_release() {
        assert_eq!(parse_fhir_release("R4").unwrap(), FhirRelease::R4);
        assert_eq!(parse_fhir_release("r4").unwrap(), FhirRelease::R4);
        assert_eq!(parse_fhir_release("4.0.1").unwrap(), FhirRelease::R4);

        assert_eq!(parse_fhir_release("R4B").unwrap(), FhirRelease::R4B);
        assert_eq!(parse_fhir_release("4.3.0").unwrap(), FhirRelease::R4B);

        assert_eq!(parse_fhir_release("R5").unwrap(), FhirRelease::R5);
        assert_eq!(parse_fhir_release("r5").unwrap(), FhirRelease::R5);
        assert_eq!(parse_fhir_release("5.0.0").unwrap(), FhirRelease::R5);

        assert_eq!(parse_fhir_release("R6").unwrap(), FhirRelease::R6);
        assert_eq!(parse_fhir_release("6.0.0").unwrap(), FhirRelease::R6);

        assert!(parse_fhir_release("invalid").is_err());
        assert!(parse_fhir_release("R7").is_err());
    }

    #[test]
    fn test_parse_package_spec() {
        let pkg = parse_package_spec("hl7.fhir.us.core@5.0.1").unwrap();
        assert_eq!(pkg.name, "hl7.fhir.us.core");
        assert_eq!(pkg.version, "5.0.1");

        let pkg2 = parse_package_spec("my.custom.package@1.2.3").unwrap();
        assert_eq!(pkg2.name, "my.custom.package");
        assert_eq!(pkg2.version, "1.2.3");

        assert!(parse_package_spec("invalid").is_err());
        assert!(parse_package_spec("invalid@").is_err());
        assert!(parse_package_spec("@1.0.0").is_err());
        assert!(parse_package_spec("pkg@ver@sion").is_err());
    }

    #[test]
    fn test_parse_cli_dependencies() {
        let (release, deps) = parse_cli_dependencies(
            "R4",
            &[
                "hl7.fhir.us.core@5.0.1".to_string(),
                "my.package@1.0.0".to_string(),
            ],
        )
        .unwrap();

        assert_eq!(release, FhirRelease::R4);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "hl7.fhir.us.core");
        assert_eq!(deps[0].version, "5.0.1");
        assert_eq!(deps[1].name, "my.package");
        assert_eq!(deps[1].version, "1.0.0");
    }

    #[test]
    fn test_parse_cli_dependencies_no_deps() {
        let (release, deps) = parse_cli_dependencies("R5", &[]).unwrap();

        assert_eq!(release, FhirRelease::R5);
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_parse_cli_dependencies_invalid_version() {
        let result = parse_cli_dependencies("R99", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cli_dependencies_invalid_spec() {
        let result = parse_cli_dependencies("R4", &["invalid-spec".to_string()]);
        assert!(result.is_err());
    }
}
