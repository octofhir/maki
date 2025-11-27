//! Canonical package integration built on top of `octofhir-canonical-manager`.
//!
//! This module exposes a high-level facade for managing FHIR package
//! dependencies and resolving canonical URLs within MAKI. It wraps the
//! canonical manager crate so the rest of `maki-core` can depend on a stable,
//! async-friendly API with caching, version awareness, and ergonomic errors.

pub mod codesystem;
pub mod extension;
pub mod fishable;
pub mod valueset;
pub mod version;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use octofhir_canonical_manager::{CanonicalManager, FcmError, config::FcmConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;
use tracing::{debug, error, info, warn};

/// Result type returned by canonical loader operations.
pub type CanonicalResult<T> = Result<T, CanonicalLoaderError>;

/// Timeout applied to canonical package installation in seconds.
/// Reduced to 120s (2 minutes) after increasing SQLite connection pool size.
/// With proper connection pooling, package installation should complete faster.
const CANONICAL_INSTALL_TIMEOUT_SECS: u64 = 120;

/// Errors produced by the canonical loader facade.
#[derive(Debug, Error)]
pub enum CanonicalLoaderError {
    #[error("canonical manager error: {0}")]
    CanonicalManager(#[from] FcmError),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("unsupported FHIR release ({release}) – enable allow_r6 to opt into ballot content")]
    UnsupportedRelease { release: String },

    #[error("package installation failed for {name}@{version}: {source}")]
    PackageInstall {
        name: String,
        version: String,
        #[source]
        source: FcmError,
    },

    #[error("package installation timed out after {timeout_secs}s for packages: {packages:?}")]
    PackageInstallTimeout {
        packages: Vec<PackageCoordinate>,
        timeout_secs: u64,
    },

    #[error("canonical resolution failed for {url}: {source}")]
    Resolution {
        url: String,
        #[source]
        source: FcmError,
    },
}

/// FHIR releases supported by MAKI canonical integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FhirRelease {
    R4,
    R4B,
    R5,
    R6,
}

impl FhirRelease {
    /// Human readable label.
    pub fn label(self) -> &'static str {
        match self {
            FhirRelease::R4 => "R4",
            FhirRelease::R4B => "R4B",
            FhirRelease::R5 => "R5",
            FhirRelease::R6 => "R6",
        }
    }

    /// Convert FHIR release to version string for canonical manager.
    pub fn to_version_string(self) -> &'static str {
        match self {
            FhirRelease::R4 => "4.0.1",
            FhirRelease::R4B => "4.3.0",
            FhirRelease::R5 => "5.0.0",
            FhirRelease::R6 => "6.0.0",
        }
    }
}

/// Default versions for FHIR core packages per release.
#[derive(Debug, Clone)]
pub struct CorePackageVersions {
    pub r4: String,
    pub r4b: String,
    pub r5: String,
    pub r6: Option<String>,
}

impl Default for CorePackageVersions {
    fn default() -> Self {
        Self {
            r4: "4.0.1".to_string(),
            r4b: "4.3.0".to_string(),
            r5: "5.0.0".to_string(),
            r6: Some("6.0.0-ballot".to_string()),
        }
    }
}

/// Coordinates describing a FHIR package that should be available to the loader.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageCoordinate {
    pub name: String,
    pub version: String,
    pub priority: u32,
}

impl PackageCoordinate {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            priority: 1,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Configuration options controlling canonical manager integration.
#[derive(Debug, Clone)]
pub struct CanonicalOptions {
    pub config: Option<FcmConfig>,
    pub config_path: Option<PathBuf>,
    pub default_release: FhirRelease,
    pub allow_r6: bool,
    pub auto_install_core: bool,
    pub quick_init: bool,
    pub preload_packages: Vec<PackageCoordinate>,
    pub core_versions: CorePackageVersions,
}

impl Default for CanonicalOptions {
    fn default() -> Self {
        Self {
            config: None,
            config_path: None,
            default_release: FhirRelease::R4,
            allow_r6: false,
            auto_install_core: true,
            quick_init: std::env::var("CI").is_ok(),
            preload_packages: Vec::new(),
            core_versions: CorePackageVersions::default(),
        }
    }
}

/// Represents a resolved FHIR resource cached by the canonical loader.
#[derive(Debug, Clone)]
pub struct DefinitionResource {
    pub canonical_url: String,
    pub resource_type: String,
    pub package_id: String,
    pub version: Option<String>,
    pub content: Arc<serde_json::Value>,
}

impl DefinitionResource {
    fn from_resolved(resolved: octofhir_canonical_manager::resolver::ResolvedResource) -> Self {
        let package_id = format!(
            "{}@{}",
            resolved.package_info.name, resolved.package_info.version
        );
        Self {
            canonical_url: resolved.canonical_url,
            resource_type: resolved.resource.resource_type,
            package_id,
            version: resolved.metadata.version,
            content: Arc::new(resolved.resource.content),
        }
    }
}

/// Facade that owns a single CanonicalManager instance and caches resolved resources.
pub struct CanonicalFacade {
    manager: Arc<CanonicalManager>,
    options: CanonicalOptions,
    global_cache: Arc<DashMap<String, Arc<DefinitionResource>>>,
}

impl CanonicalFacade {
    /// Create a new facade from options.
    pub async fn new(options: CanonicalOptions) -> CanonicalResult<Self> {
        let mut config = if let Some(cfg) = options.config.clone() {
            cfg
        } else if let Some(path) = &options.config_path {
            FcmConfig::from_file(path)
                .await
                .map_err(CanonicalLoaderError::from)?
        } else {
            FcmConfig::load()
                .await
                .map_err(CanonicalLoaderError::from)?
        };

        // Apply quick-init heuristic for tests/CI.
        if options.quick_init {
            config.optimization.parallel_workers = 1;
            config.optimization.enable_metrics = false;
            // config.optimization.use_mmap = false; // Field removed from OptimizationConfig
        }

        // Ensure storage directories exist.
        ensure_storage_dirs(&config).await?;

        // Apply any preload packages to the config for priority ordering.
        for pkg in &options.preload_packages {
            config.add_package(&pkg.name, &pkg.version, Some(pkg.priority));
        }

        eprintln!("[DEBUG MAKI] About to call CanonicalManager::new()");
        let manager = CanonicalManager::new(config).await?;
        eprintln!("[DEBUG MAKI] CanonicalManager::new() returned successfully");

        eprintln!("[DEBUG MAKI] Creating CanonicalFacade");
        Ok(Self {
            manager: Arc::new(manager),
            options,
            global_cache: Arc::new(DashMap::new()),
        })
    }

    /// Access the underlying canonical manager.
    pub fn manager(&self) -> &Arc<CanonicalManager> {
        &self.manager
    }

    /// List installed packages.
    pub async fn list_packages(&self) -> CanonicalResult<Vec<String>> {
        Ok(self.manager.list_packages().await?)
    }

    /// Create a new definition session for the provided releases.
    pub async fn session<I>(&self, releases: I) -> CanonicalResult<DefinitionSession>
    where
        I: IntoIterator<Item = FhirRelease>,
    {
        let mut unique: HashSet<FhirRelease> = releases.into_iter().collect();
        if unique.is_empty() {
            unique.insert(self.options.default_release);
        }

        if !self.options.allow_r6 && unique.contains(&FhirRelease::R6) {
            return Err(CanonicalLoaderError::UnsupportedRelease {
                release: "R6".to_string(),
            });
        }

        let session = DefinitionSession {
            facade: Arc::new(self.clone()),
            releases: unique.into_iter().collect(),
            local_cache: DashMap::new(),
            installed: DashSet::new(),
        };

        if self.options.auto_install_core {
            session.ensure_core_packages().await?;
        }

        Ok(session)
    }

    fn default_core_package(&self, release: FhirRelease) -> Option<PackageCoordinate> {
        match release {
            FhirRelease::R4 => Some(
                PackageCoordinate::new("hl7.fhir.r4.core", &self.options.core_versions.r4)
                    .with_priority(1),
            ),
            FhirRelease::R4B => Some(
                PackageCoordinate::new("hl7.fhir.r4b.core", &self.options.core_versions.r4b)
                    .with_priority(1),
            ),
            FhirRelease::R5 => Some(
                PackageCoordinate::new("hl7.fhir.r5.core", &self.options.core_versions.r5)
                    .with_priority(1),
            ),
            FhirRelease::R6 => self.options.core_versions.r6.as_ref().map(|version| {
                PackageCoordinate::new("hl7.fhir.r6.core", version).with_priority(1)
            }),
        }
    }
}

impl Clone for CanonicalFacade {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
            options: self.options.clone(),
            global_cache: self.global_cache.clone(),
        }
    }
}

/// Session-scoped view of canonical resources for a specific set of releases.
pub struct DefinitionSession {
    facade: Arc<CanonicalFacade>,
    releases: Vec<FhirRelease>,
    local_cache: DashMap<String, Arc<DefinitionResource>>,
    installed: DashSet<String>,
}

impl DefinitionSession {
    /// Ensure core packages for the configured releases are present.
    pub async fn ensure_core_packages(&self) -> CanonicalResult<()> {
        let mut coords = Vec::new();
        for release in &self.releases {
            if let Some(pkg) = self.facade.default_core_package(*release) {
                coords.push(pkg);
            } else {
                warn!("No default core package configured for {:?}", release);
            }
        }
        self.ensure_packages(coords).await
    }

    /// Ensure the provided packages are installed and ready for resolution.
    pub async fn ensure_packages<I>(&self, packages: I) -> CanonicalResult<()>
    where
        I: IntoIterator<Item = PackageCoordinate>,
    {
        let mut to_install = Vec::new();

        for pkg in packages.into_iter() {
            let key = format!("{}@{}", pkg.name, pkg.version);
            // only install once per session
            if self.installed.insert(key.clone()) {
                info!(
                    "Queuing FHIR package {}@{} (priority {}) for installation",
                    pkg.name, pkg.version, pkg.priority
                );
                to_install.push((pkg, key));
            } else {
                debug!("Package {key} already installed in session");
            }
        }

        if to_install.is_empty() {
            return Ok(());
        }

        info!("Installing {} canonical package(s)", to_install.len());
        let specs: Vec<octofhir_canonical_manager::config::PackageSpec> = to_install
            .iter()
            .map(|(pkg, _)| octofhir_canonical_manager::config::PackageSpec {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                priority: pkg.priority,
            })
            .collect();

        let packages_only: Vec<PackageCoordinate> =
            to_install.iter().map(|(pkg, _)| pkg.clone()).collect();
        let keys: Vec<String> = to_install.iter().map(|(_, key)| key.clone()).collect();

        info!(
            "Canonical install timeout set to {}s",
            CANONICAL_INSTALL_TIMEOUT_SECS
        );
        for (index, pkg) in packages_only.iter().enumerate() {
            info!(
                "  [{} / {}] {}@{}",
                index + 1,
                packages_only.len(),
                pkg.name,
                pkg.version
            );
        }

        // Use new parallel installation pipeline for maximum performance
        let install_future = self.facade.manager.install_packages_parallel(specs);

        match tokio::time::timeout(
            std::time::Duration::from_secs(CANONICAL_INSTALL_TIMEOUT_SECS),
            install_future,
        )
        .await
        {
            Ok(Ok(())) => {
                info!(
                    "Canonical packages installed successfully ({} total)",
                    packages_only.len()
                );
                Ok(())
            }
            Ok(Err(source)) => {
                for key in keys {
                    self.installed.remove(&key);
                }

                if let Some(pkg) = packages_only.first() {
                    error!(
                        "Failed to install canonical package {}@{}: {}",
                        pkg.name, pkg.version, source
                    );
                    Err(CanonicalLoaderError::PackageInstall {
                        name: pkg.name.clone(),
                        version: pkg.version.clone(),
                        source,
                    })
                } else {
                    Err(CanonicalLoaderError::from(source))
                }
            }
            Err(_) => {
                for key in keys {
                    self.installed.remove(&key);
                }

                error!(
                    "Package installation timed out after {}s: {:?}",
                    CANONICAL_INSTALL_TIMEOUT_SECS, packages_only
                );
                Err(CanonicalLoaderError::PackageInstallTimeout {
                    packages: packages_only,
                    timeout_secs: CANONICAL_INSTALL_TIMEOUT_SECS,
                })
            }
        }
    }

    /// Resolve a canonical URL into a cached definition.
    ///
    /// This method attempts FHIR version-aware resolution using the primary FHIR version
    /// configured for this session. If version-specific resolution fails, it falls back
    /// to version-agnostic resolution for backward compatibility.
    pub async fn resolve(&self, canonical_url: &str) -> CanonicalResult<Arc<DefinitionResource>> {
        // Check local and global caches first
        if let Some(existing) = self.local_cache.get(canonical_url) {
            return Ok(existing.clone());
        }
        if let Some(existing) = self.facade.global_cache.get(canonical_url) {
            let arc = existing.clone();
            self.local_cache
                .insert(canonical_url.to_string(), arc.clone());
            return Ok(arc);
        }

        // Try FHIR version-specific resolution using the primary (first) release
        let resolved = if let Some(release) = self.releases.first() {
            let fhir_version = release.to_version_string();
            debug!(
                "Resolving {} with FHIR version {}",
                canonical_url, fhir_version
            );

            // Try version-specific resolution first
            match self
                .facade
                .manager
                .resolve_with_fhir_version(canonical_url, fhir_version)
                .await
            {
                Ok(resolved) => {
                    debug!(
                        "Successfully resolved {} with FHIR version {}",
                        canonical_url, fhir_version
                    );
                    resolved
                }
                Err(version_err) => {
                    // Fall back to version-agnostic resolution
                    debug!(
                        "Version-specific resolution failed for {}, falling back to version-agnostic: {}",
                        canonical_url, version_err
                    );
                    self.facade
                        .manager
                        .resolve(canonical_url)
                        .await
                        .map_err(|source| CanonicalLoaderError::Resolution {
                            url: canonical_url.to_string(),
                            source,
                        })?
                }
            }
        } else {
            // No FHIR version specified, use version-agnostic resolution
            debug!(
                "No FHIR version configured for session, using version-agnostic resolution for {}",
                canonical_url
            );
            self.facade
                .manager
                .resolve(canonical_url)
                .await
                .map_err(|source| CanonicalLoaderError::Resolution {
                    url: canonical_url.to_string(),
                    source,
                })?
        };

        let resource = Arc::new(DefinitionResource::from_resolved(resolved));
        self.facade
            .global_cache
            .insert(canonical_url.to_string(), resource.clone());
        self.local_cache
            .insert(canonical_url.to_string(), resource.clone());
        Ok(resource)
    }

    /// Resolve a canonical URL and clone the JSON payload.
    pub async fn resolve_json(&self, canonical_url: &str) -> CanonicalResult<serde_json::Value> {
        let resource = self.resolve(canonical_url).await?;
        Ok((*resource.content).clone())
    }

    /// Resolve by resource type and name (fast direct SQL lookup)
    pub async fn resource_by_type_and_name(
        &self,
        resource_type: &str,
        name: &str,
    ) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        eprintln!(
            "[DEBUG] >>> resource_by_type_and_name: direct SQL lookup for {}:{}",
            resource_type, name
        );
        let search_start = std::time::Instant::now();

        let results = self
            .facade
            .manager
            .find_by_type_and_name(resource_type, name)
            .await?;

        eprintln!(
            "[DEBUG]     find_by_type_and_name() returned {} results after {:?}",
            results.len(),
            search_start.elapsed()
        );

        if results.is_empty() {
            return Ok(None);
        }

        // Take the first result (there should only be one with exact name match)
        let resource_index = &results[0];
        let canonical_url = resource_index.canonical_url.clone();

        // Load the actual resource content from CAS
        let resource = self
            .facade
            .manager
            .storage()
            .package_storage()
            .get_resource(resource_index)
            .await?;

        let resolved = Arc::new(DefinitionResource {
            canonical_url: canonical_url.clone(),
            resource_type: resource_index.resource_type.clone(),
            package_id: format!(
                "{}@{}",
                resource_index.package_name, resource_index.package_version
            ),
            version: resource_index.version.clone(),
            content: Arc::new(resource.content),
        });

        eprintln!(
            "[DEBUG]     Found and cached resource by name: {}",
            canonical_url
        );

        self.facade
            .global_cache
            .insert(canonical_url.clone(), resolved.clone());
        self.local_cache.insert(canonical_url, resolved.clone());

        Ok(Some(resolved))
    }

    /// Resolve by resource type and id using the canonical manager search engine.
    ///
    /// This method filters search results by the FHIR version configured for this session
    /// to ensure the returned resource matches the expected FHIR version.
    pub async fn resource_by_type_and_id(
        &self,
        resource_type: &str,
        id: &str,
    ) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        eprintln!(
            "[DEBUG] >>> resource_by_type_and_id: direct SQL lookup for {}:{}",
            resource_type, id
        );
        let search_start = std::time::Instant::now();

        // Use fast direct SQL lookup instead of slow text search
        let results = self
            .facade
            .manager
            .find_by_type_and_id(resource_type, id)
            .await?;

        eprintln!(
            "[DEBUG]     find_by_type_and_id() returned {} results after {:?}",
            results.len(),
            search_start.elapsed()
        );

        // Get the primary FHIR version for filtering
        let fhir_version_filter = self.releases.first().map(|r| r.to_version_string());

        for resource_index in results {
            // Filter by FHIR version if configured
            if let Some(expected_version) = &fhir_version_filter
                && &resource_index.fhir_version != expected_version
            {
                debug!(
                    "Skipping resource {} from FHIR version {}, expecting {}",
                    id, resource_index.fhir_version, expected_version
                );
                continue;
            }

            // The SQL query already filtered by ID, so we can use the first match
            let canonical_url = resource_index.canonical_url.clone();

            // Load the actual resource content from CAS
            let resource = self
                .facade
                .manager
                .storage()
                .package_storage()
                .get_resource(&resource_index)
                .await?;

            let resolved = Arc::new(DefinitionResource {
                canonical_url: canonical_url.clone(),
                resource_type: resource_index.resource_type.clone(),
                package_id: format!(
                    "{}@{}",
                    resource_index.package_name, resource_index.package_version
                ),
                version: resource_index.version.clone(),
                content: Arc::new(resource.content),
            });

            eprintln!("[DEBUG]     Found and cached resource: {}", canonical_url);

            self.facade
                .global_cache
                .insert(canonical_url.clone(), resolved.clone());
            self.local_cache.insert(canonical_url, resolved.clone());

            return Ok(Some(resolved));
        }

        eprintln!(
            "[DEBUG]     No matching resource found for {}:{}",
            resource_type, id
        );
        Ok(None)
    }

    /// Returns the releases associated with this session.
    pub fn releases(&self) -> &[FhirRelease] {
        &self.releases
    }

    /// Create a minimal test session for unit testing
    ///
    /// This creates a DefinitionSession with minimal configuration suitable
    /// for unit tests that don't need real FHIR packages.
    ///
    /// # Note
    /// This is a blocking wrapper around an async operation. It should only
    /// be used in tests where async context is not available.
    #[cfg(test)]
    pub fn for_testing() -> Self {
        fn build_session() -> DefinitionSession {
            use std::time::{SystemTime, UNIX_EPOCH};
            use tokio::runtime::Runtime;

            // Create a unique test directory to avoid SQLite database locking issues
            // when running tests in parallel, especially on Windows
            let thread_id = std::thread::current().id();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let unique_dir = format!("/tmp/maki-test-{:?}-{}", thread_id, timestamp);

            let test_config = octofhir_canonical_manager::FcmConfig::test_config(
                std::path::Path::new(&unique_dir),
            );

            let rt = Runtime::new().expect("Failed to create test runtime");
            let manager = std::sync::Arc::new(
                rt.block_on(octofhir_canonical_manager::CanonicalManager::new(
                    test_config,
                ))
                .expect("Failed to create test manager"),
            );

            DefinitionSession {
                facade: std::sync::Arc::new(CanonicalFacade {
                    manager,
                    options: CanonicalOptions::default(),
                    global_cache: std::sync::Arc::new(dashmap::DashMap::new()),
                }),
                releases: vec![FhirRelease::R4],
                local_cache: dashmap::DashMap::new(),
                installed: dashmap::DashSet::new(),
            }
        }

        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(build_session)
        } else {
            build_session()
        }
    }

    /// Resolve a StructureDefinition by canonical URL
    ///
    /// This is a convenience method that resolves a resource and deserializes it
    /// as a StructureDefinition.
    ///
    /// # Arguments
    ///
    /// * `canonical_url` - Canonical URL of the StructureDefinition
    ///
    /// # Returns
    ///
    /// The StructureDefinition if found, or None if not found or not a StructureDefinition
    pub async fn resolve_structure_definition(
        &self,
        canonical_url: &str,
    ) -> CanonicalResult<Option<crate::export::StructureDefinition>> {
        let resource = self.resolve(canonical_url).await?;

        // Check resource type
        if resource.resource_type != "StructureDefinition" {
            return Ok(None);
        }

        // Deserialize from JSON
        match serde_json::from_value((*resource.content).clone()) {
            Ok(sd) => Ok(Some(sd)),
            Err(e) => {
                warn!(
                    "Failed to deserialize StructureDefinition {}: {}",
                    canonical_url, e
                );
                Ok(None)
            }
        }
    }

    /// Set package priorities from dependencies list
    ///
    /// First dependency gets highest priority (100), decreasing by 10 for each subsequent.
    /// This ensures that when resolving resources, dependencies are preferred over core packages.
    pub async fn set_dependencies_priority(
        &self,
        deps: &[(String, String)],
    ) -> CanonicalResult<()> {
        for (i, (name, version)) in deps.iter().enumerate() {
            let priority = 100 - (i as i32 * 10);
            self.facade
                .manager
                .storage()
                .package_storage()
                .set_package_priority(name, version, priority)
                .await
                .map_err(CanonicalLoaderError::from)?;
            debug!(
                "Set package priority for {}@{}: {}",
                name, version, priority
            );
        }
        Ok(())
    }

    /// Find a StructureDefinition parent by key (id, name, or URL), excluding Extensions
    ///
    /// This method is optimized for resolving parent definitions during profile building.
    /// It uses case-insensitive matching and excludes Extension resources to avoid
    /// accidentally resolving an Extension when looking for a Profile/Resource parent.
    ///
    /// # Arguments
    ///
    /// * `key` - The identifier to search for (can be id, name, or canonical URL)
    ///
    /// # Returns
    ///
    /// The parent StructureDefinition if found, or None if not found
    pub async fn find_profile_parent(
        &self,
        key: &str,
    ) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        // Use the new find_resource_info with:
        // - types = Profile, Resource, Type, Logical (NOT Extension)
        // - exclude_extensions = true
        // - sort_by_priority = true (prefer dependencies over core packages)
        let types = ["Profile", "Resource", "Type", "Logical"];
        let result = self
            .facade
            .manager
            .storage()
            .package_storage()
            .find_resource_info(key, Some(&types), true, true)
            .await
            .map_err(CanonicalLoaderError::from)?;

        if let Some(resource_index) = result {
            let canonical_url = resource_index.canonical_url.clone();

            // Check caches first
            if let Some(existing) = self.local_cache.get(&canonical_url) {
                return Ok(Some(existing.clone()));
            }
            if let Some(existing) = self.facade.global_cache.get(&canonical_url) {
                let arc = existing.clone();
                self.local_cache.insert(canonical_url.clone(), arc.clone());
                return Ok(Some(arc));
            }

            // Load the actual resource content
            let resource = self
                .facade
                .manager
                .storage()
                .package_storage()
                .get_resource(&resource_index)
                .await
                .map_err(CanonicalLoaderError::from)?;

            let resolved = Arc::new(DefinitionResource {
                canonical_url: canonical_url.clone(),
                resource_type: resource_index.resource_type,
                package_id: format!(
                    "{}@{}",
                    resource_index.package_name, resource_index.package_version
                ),
                version: resource_index.version,
                content: Arc::new(resource.content),
            });

            self.facade
                .global_cache
                .insert(canonical_url.clone(), resolved.clone());
            self.local_cache.insert(canonical_url, resolved.clone());

            Ok(Some(resolved))
        } else {
            Ok(None)
        }
    }

    /// Find resource by key (id, name, or URL) with optional type filtering
    ///
    /// This provides access to the new case-insensitive, priority-aware search.
    ///
    /// # Arguments
    ///
    /// * `key` - The identifier to search for
    /// * `types` - Optional SD flavor types to filter by
    /// * `exclude_extensions` - If true, exclude Extension resources
    ///
    /// # Returns
    ///
    /// The matching resource if found
    pub async fn find_resource_by_key(
        &self,
        key: &str,
        types: Option<&[&str]>,
        exclude_extensions: bool,
    ) -> CanonicalResult<Option<Arc<DefinitionResource>>> {
        let result = self
            .facade
            .manager
            .storage()
            .package_storage()
            .find_resource_info(key, types, exclude_extensions, true)
            .await
            .map_err(CanonicalLoaderError::from)?;

        if let Some(resource_index) = result {
            let canonical_url = resource_index.canonical_url.clone();

            // Check caches first
            if let Some(existing) = self.local_cache.get(&canonical_url) {
                return Ok(Some(existing.clone()));
            }
            if let Some(existing) = self.facade.global_cache.get(&canonical_url) {
                let arc = existing.clone();
                self.local_cache.insert(canonical_url.clone(), arc.clone());
                return Ok(Some(arc));
            }

            // Load the actual resource content
            let resource = self
                .facade
                .manager
                .storage()
                .package_storage()
                .get_resource(&resource_index)
                .await
                .map_err(CanonicalLoaderError::from)?;

            let resolved = Arc::new(DefinitionResource {
                canonical_url: canonical_url.clone(),
                resource_type: resource_index.resource_type,
                package_id: format!(
                    "{}@{}",
                    resource_index.package_name, resource_index.package_version
                ),
                version: resource_index.version,
                content: Arc::new(resource.content),
            });

            self.facade
                .global_cache
                .insert(canonical_url.clone(), resolved.clone());
            self.local_cache.insert(canonical_url, resolved.clone());

            Ok(Some(resolved))
        } else {
            Ok(None)
        }
    }
}

async fn ensure_storage_dirs(config: &FcmConfig) -> CanonicalResult<()> {
    async fn ensure(path: &Path) -> Result<(), std::io::Error> {
        if !path.exists() {
            fs::create_dir_all(path).await?;
        }
        Ok(())
    }

    ensure(&config.storage.cache_dir).await?;
    ensure(&config.storage.packages_dir).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn build_test_package() -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::Builder;

        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = Builder::new(encoder);

        // package/package.json
        let package_json = serde_json::json!({
            "name": "example.test",
            "version": "0.1.0",
            "description": "Example package",
            "fhirVersions": ["4.0.1"],
            "dependencies": {}
        })
        .to_string();
        let mut header = tar::Header::new_gnu();
        header.set_size(package_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "package/package.json", package_json.as_bytes())
            .unwrap();

        // package/.index.json
        let index_json = serde_json::json!({
            "index-version": "1.0",
            "files": {
                "package/StructureDefinition-Example.json": {
                    "resourceType": "StructureDefinition",
                    "id": "Example",
                    "url": "http://example.org/fhir/StructureDefinition/Example",
                    "version": "0.1.0",
                    "kind": "resource"
                }
            }
        })
        .to_string();
        let mut header = tar::Header::new_gnu();
        header.set_size(index_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, "package/.index.json", index_json.as_bytes())
            .unwrap();

        // package resource
        let sd_json = serde_json::json!({
            "resourceType": "StructureDefinition",
            "id": "Example",
            "url": "http://example.org/fhir/StructureDefinition/Example",
            "version": "0.1.0",
            "name": "ExampleProfile",
            "status": "draft",
            "kind": "resource",
            "type": "Observation",
            "baseDefinition": "http://hl7.org/fhir/StructureDefinition/Observation",
            "derivation": "constraint"
        })
        .to_string();
        let mut header = tar::Header::new_gnu();
        header.set_size(sd_json.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(
            &mut header,
            "package/StructureDefinition-Example.json",
            sd_json.as_bytes(),
        )
        .unwrap();

        let encoder = tar.into_inner().unwrap();
        encoder.finish().unwrap()
    }

    async fn mock_server_with_package() -> MockServer {
        let server = MockServer::start().await;
        let tar_bytes = build_test_package();

        let metadata = serde_json::json!({
            "name": "example.test",
            "versions": {
                "0.1.0": {
                    "name": "example.test",
                    "version": "0.1.0",
                    "dist": { "tarball": format!("{}/example.test-0.1.0.tgz", server.uri()) },
                    "dependencies": {},
                    "fhirVersions": ["4.0.1"]
                }
            },
            "dist-tags": { "latest": "0.1.0" }
        });

        Mock::given(method("GET"))
            .and(path("/example.test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(metadata))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/example.test-0.1.0.tgz"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(tar_bytes))
            .mount(&server)
            .await;

        server
    }

    #[tokio::test]
    async fn session_can_install_and_resolve_resources() {
        let server = mock_server_with_package().await;
        let temp_dir = TempDir::new().unwrap();

        let mut config = FcmConfig::test_config(temp_dir.path());
        config.registry.url = format!("{}/", server.uri());
        config.add_package("example.test", "0.1.0", Some(1));

        let options = CanonicalOptions {
            config: Some(config),
            auto_install_core: false,
            preload_packages: vec![PackageCoordinate::new("example.test", "0.1.0")],
            ..Default::default()
        };

        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session([FhirRelease::R4]).await.unwrap();
        session
            .ensure_packages(vec![PackageCoordinate::new("example.test", "0.1.0")])
            .await
            .unwrap();

        let resource = session
            .resolve("http://example.org/fhir/StructureDefinition/Example")
            .await
            .unwrap();
        assert_eq!(resource.resource_type, "StructureDefinition");
        assert_eq!(resource.package_id, "example.test@0.1.0");

        let json = session
            .resolve_json("http://example.org/fhir/StructureDefinition/Example")
            .await
            .unwrap();
        assert_eq!(json["name"], "ExampleProfile");

        let lookup = session
            .resource_by_type_and_id("StructureDefinition", "Example")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lookup.package_id, "example.test@0.1.0");
    }
}
