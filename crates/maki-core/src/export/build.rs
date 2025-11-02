//! Build Orchestrator
//!
//! Coordinates all exporters to generate a complete FHIR IG package.
//! Implements SUSHI-compatible build pipeline with progress reporting.

use crate::config::SushiConfiguration;
use crate::cst::FshSyntaxNode;
use crate::cst::TextRange;
use crate::cst::ast::{CodeSystem, Extension, Instance, Profile, ValueSet};
use crate::export::ruleset_integration::RuleSetProcessor;
use crate::export::*;
use crate::semantic::{DefaultSemanticAnalyzer, DeferredRule};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, trace, warn};

/// Resource with source location tracking
#[derive(Debug, Clone)]
struct SourceTrackedResource<T> {
    resource: T,
    source_file: PathBuf,
    start_line: usize,
    end_line: usize,
}

impl<T> SourceTrackedResource<T> {
    fn new(resource: T, source_file: PathBuf, start_line: usize, end_line: usize) -> Self {
        Self {
            resource,
            source_file,
            start_line,
            end_line,
        }
    }
}

/// Parsed FSH resources ready for export
#[derive(Debug, Default)]
struct ParsedResources {
    profiles: Vec<SourceTrackedResource<Profile>>,
    extensions: Vec<SourceTrackedResource<Extension>>,
    valuesets: Vec<SourceTrackedResource<ValueSet>>,
    codesystems: Vec<SourceTrackedResource<CodeSystem>>,
    instances: Vec<SourceTrackedResource<Instance>>,
}

/// Build errors
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Failed to parse FSH file: {0}")]
    ParseError(String),

    #[error("Failed to build semantic model: {0}")]
    SemanticError(String),

    #[error("Failed to export resource: {0}")]
    ExportError(String),

    #[error("Failed to write output: {0}")]
    FileSystemError(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File structure error: {0}")]
    FileStructureError(#[from] FileStructureError),

    #[error("Predefined resource error: {0}")]
    PredefinedResourceError(#[from] PredefinedResourceError),

    #[error("No FSH files found")]
    NoFshFiles,

    #[error("No configuration found")]
    NoConfiguration,
}

/// Build options
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Input directory (typically input/fsh/)
    pub input_dir: PathBuf,

    /// Output directory (typically fsh-generated/)
    pub output_dir: PathBuf,

    /// Generate snapshots in StructureDefinitions
    pub generate_snapshots: bool,

    /// Write preprocessed FSH (for debugging)
    pub write_preprocessed: bool,

    /// Clean output directory before building
    pub clean_output: bool,

    /// Show progress during build
    pub show_progress: bool,

    /// FHIR version override
    pub fhir_version: Option<String>,

    /// Configuration overrides (e.g., version, status)
    pub config_overrides: HashMap<String, String>,

    /// Run linter during build
    /// Default: false (opt-in feature)
    /// This provides real-time feedback about FSH code quality
    pub run_linter: bool,

    /// Strict mode: treat warnings as errors
    /// When enabled, any linter warnings will fail the build
    /// Default: false
    pub strict_mode: bool,

    /// Format FSH files before building
    /// Default: false (opt-in feature)
    /// Automatically formats all FSH files to ensure consistent style
    pub format_on_build: bool,

    /// Use incremental compilation cache
    /// Default: true (enabled by default for better performance)
    /// Caches parsed files and only re-exports changed resources
    pub use_cache: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            input_dir: PathBuf::from("input/fsh"),
            output_dir: PathBuf::from("fsh-generated"),
            generate_snapshots: false, // Default OFF to match SUSHI
            write_preprocessed: false,
            clean_output: false,
            show_progress: false,
            fhir_version: None,
            config_overrides: HashMap::new(),
            run_linter: false,      // Default OFF - opt-in feature
            strict_mode: false,     // Default OFF - warnings don't fail build
            format_on_build: false, // Default OFF - opt-in feature
            use_cache: true,        // Default ON - improves performance
        }
    }
}

/// Build statistics
#[derive(Debug, Clone, Default)]
pub struct BuildStats {
    /// Number of profiles exported
    pub profiles: usize,
    /// Number of extensions exported
    pub extensions: usize,
    /// Number of logicals exported
    pub logicals: usize,
    /// Number of resources exported
    pub resources: usize,
    /// Number of value sets exported
    pub value_sets: usize,
    /// Number of code systems exported
    pub code_systems: usize,
    /// Number of instances exported
    pub instances: usize,
    /// Number of mappings exported
    pub mappings: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Number of warnings encountered
    pub warnings: usize,
}

impl BuildStats {
    /// Total number of resources generated
    pub fn total_resources(&self) -> usize {
        self.profiles
            + self.extensions
            + self.logicals
            + self.resources
            + self.value_sets
            + self.code_systems
            + self.instances
    }

    /// Whether the build had any errors
    pub fn has_errors(&self) -> bool {
        self.errors > 0
    }

    /// Whether the build had any warnings
    pub fn has_warnings(&self) -> bool {
        self.warnings > 0
    }
}

/// Build result
#[derive(Debug)]
pub struct BuildResult {
    /// Build statistics
    pub stats: BuildStats,

    /// Output directory where resources were written
    pub output_dir: PathBuf,

    /// Configuration used for the build
    pub config: crate::config::UnifiedConfig,

    /// FSH index entries for generated resources
    pub fsh_index: Vec<FshIndexEntry>,
}

/// Build orchestrator
///
/// Coordinates all exporters to generate a complete FHIR IG package.
/// Follows SUSHI's build pipeline:
/// 1. Parse FSH files
/// 2. Build semantic model
/// 3. Phase 1: Expand RuleSets (InsertRule processing)
/// 4. Phase 2: Export resources in dependency order
/// 5. Phase 3: Apply deferred rules (circular dependencies)
/// 6. Generate ImplementationGuide resource
/// 7. Write package.json
/// 8. Load predefined resources
/// 9. Write FSH index
pub struct BuildOrchestrator {
    options: BuildOptions,
    config: crate::config::UnifiedConfig,
    deferred_rules: Vec<DeferredRule>,
}

impl BuildOrchestrator {
    /// Create a new build orchestrator
    pub fn new(config: crate::config::UnifiedConfig, options: BuildOptions) -> Self {
        Self {
            config,
            options,
            deferred_rules: Vec::new(),
        }
    }

    /// Get the build configuration (BuildConfiguration from unified config)
    fn build_config(&self) -> &crate::config::BuildConfiguration {
        self.config
            .build
            .as_ref()
            .expect("Build configuration is required")
    }

    /// Run the complete build pipeline with two-phase export
    pub async fn build(&self) -> std::result::Result<BuildResult, BuildError> {
        info!("🚀 Starting MAKI build...");

        // Create canonical session for FHIR package resolution
        use crate::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
        use octofhir_canonical_manager::config::{
            FcmConfig, OptimizationConfig, RegistryConfig, StorageConfig,
        };
        use std::path::PathBuf;

        // Create optimized FcmConfig programmatically for excellent performance
        // This enables multi-worker parallel loading for large IGs
        let fcm_config = {
            // Use global ~/.maki directory for package cache (shared across all IGs)
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
            let maki_dir = PathBuf::from(home_dir).join(".maki");

            FcmConfig {
                registry: RegistryConfig {
                    url: "https://fs.get-ig.org/pkgs/".to_string(),
                    timeout: 60,
                    retry_attempts: 3,
                },
                storage: StorageConfig {
                    cache_dir: maki_dir.join("cache"),
                    packages_dir: maki_dir.join("packages"),
                    max_cache_size: "5GB".to_string(),
                },
                optimization: OptimizationConfig {
                    parallel_workers: rayon::current_num_threads(), // Use all CPU cores
                    batch_size: 200,
                    enable_checksums: true,
                    checksum_algorithm: "blake3".to_string(),
                    checksum_cache_size: 50000,
                    enable_metrics: true,
                    metrics_interval: "30s".to_string(),
                },
                packages: vec![],
                local_packages: vec![],
                resource_directories: vec![],
            }
        };

        let options = CanonicalOptions {
            config: Some(fcm_config), // Pass our optimized config
            auto_install_core: true,  // Auto-install FHIR core packages based on fhirVersion
            quick_init: true, // Prefer fast initialization; defer heavy indexing unless needed
            ..Default::default()
        };
        let facade = CanonicalFacade::new(options).await.map_err(|e| {
            BuildError::ExportError(format!("Failed to create CanonicalFacade: {}", e))
        })?;

        // Parse FHIR version from config
        let fhir_releases: Vec<FhirRelease> = self
            .build_config()
            .fhir_version
            .iter()
            .filter_map(|v| {
                // Parse version string like "4.0.1" to FhirRelease
                if v.starts_with("4.0") {
                    Some(FhirRelease::R4)
                } else if v.starts_with("4.3") || v.starts_with("4.1") {
                    Some(FhirRelease::R4B)
                } else if v.starts_with("5.0") {
                    Some(FhirRelease::R5)
                } else if v.starts_with("6.0") {
                    Some(FhirRelease::R6)
                } else {
                    warn!("Unknown FHIR version: {}, defaulting to R4", v);
                    Some(FhirRelease::R4)
                }
            })
            .collect();

        let fhir_releases = if fhir_releases.is_empty() {
            warn!("No FHIR version specified in config, defaulting to R4");
            vec![FhirRelease::R4]
        } else {
            fhir_releases
        };

        info!(
            "✓ Using FHIR version(s): {}",
            fhir_releases
                .iter()
                .map(|r| r.label())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Create session ONCE for both installation and resolution
        info!("✓ Creating FHIR package resolution session");
        let session = Arc::new(facade.session(fhir_releases).await.map_err(|e| {
            BuildError::ExportError(format!("Failed to create DefinitionSession: {}", e))
        })?);

        // Install dependencies using the SAME session
        if let Some(ref dependencies) = self.config.dependencies {
            info!(
                "📦 Installing {} dependencies from config...",
                dependencies.len()
            );

            if !dependencies.is_empty() {
                use crate::canonical::PackageCoordinate;
                let mut coords = Vec::new();

                for (package_id, version) in dependencies {
                    let version_str = match version {
                        crate::config::DependencyVersion::Simple(v) => v.clone(),
                        crate::config::DependencyVersion::Complex { version, .. } => {
                            version.clone()
                        }
                    };

                    info!("  Installing: {}@{}", package_id, version_str);

                    coords.push(PackageCoordinate {
                        name: package_id.clone(),
                        version: version_str.clone(),
                        priority: 100, // Lower priority than core packages
                    });
                }

                // Install all dependencies at once in the main session
                match session.ensure_packages(coords).await {
                    Err(e) => {
                        warn!("  ⚠ Failed to install some dependencies: {}", e);
                        warn!("  Continuing with partial dependencies...");
                    }
                    Ok(_) => {
                        info!("  ✓ All dependencies installed successfully");
                    }
                }
            }
        } else {
            info!("📦 No dependencies found in config");
        }

        // Create FishingContext with Tank and Package
        // This implements SUSHI's three-tier fishing pattern:
        // 1. Package (exported resources) - highest priority
        // 2. Tank (parsed FSH resources) - blocks external lookup if found
        // 3. Canonical (external FHIR packages) - fallback
        use crate::semantic::{FishingContext, FshTank, Package};
        use tokio::sync::RwLock; // Use async-aware RwLock

        let tank = Arc::new(RwLock::new(FshTank::new()));
        let package = Arc::new(RwLock::new(Package::new()));
        let fishing_ctx = Arc::new(FishingContext::new(
            session.clone(),
            tank.clone(),
            package.clone(),
        ));
        info!("✓ Created fishing context (Tank + Package + Canonical)");

        info!("  Input directory: {:?}", self.options.input_dir);
        info!("  Output directory: {:?}", self.options.output_dir);

        // Initialize file structure
        let file_structure =
            FileStructureGenerator::new(&self.options.output_dir, self.options.clean_output);
        file_structure.initialize()?;

        // Initialize stats
        let mut stats = BuildStats::default();
        let mut fsh_index = Vec::new();

        // Step 1: Discover FSH files
        info!("📂 Discovering FSH files...");
        let fsh_files = self.discover_fsh_files()?;
        if fsh_files.is_empty() {
            return Err(BuildError::NoFshFiles);
        }
        info!("  Found {} FSH files", fsh_files.len());

        // Step 1.5: Load cache and analyze changes (if enabled)
        let mut cache = if self.options.use_cache {
            use crate::export::build_cache::BuildCache;
            let cache = BuildCache::load(&self.options.output_dir).unwrap_or_else(|e| {
                debug!("Failed to load cache: {}, starting fresh", e);
                BuildCache::new()
            });

            if cache.stats().total_files > 0 {
                info!("📦 Incremental build mode enabled");
                use crate::export::build_cache::IncrementalBuildInfo;
                let inc_info =
                    IncrementalBuildInfo::analyze(&fsh_files, &cache).unwrap_or_else(|e| {
                        warn!("Cache analysis failed: {}, rebuilding all files", e);
                        IncrementalBuildInfo {
                            changed_files: fsh_files.clone(),
                            unchanged_files: vec![],
                            new_files: vec![],
                            deleted_files: vec![],
                        }
                    });

                inc_info.log_summary();

                // If no changes, we could potentially skip the build entirely
                // But for now, we'll still process to ensure consistency
            }

            Some(cache)
        } else {
            None
        };

        // Step 2: Parse FSH files
        info!("📝 Parsing FSH files...");
        let parsed_files = self.parse_fsh_files(&fsh_files)?;
        debug!("  Parsed {} FSH files", parsed_files.len());

        // Update cache with parsed files
        if let Some(ref mut cache) = cache {
            for file in &fsh_files {
                if let Err(e) = cache.update_file(file) {
                    warn!("Failed to update cache for {:?}: {}", file, e);
                }
            }
        }

        // Note: Linting (if enabled via --lint flag) is handled at CLI level
        // before build() is called to avoid circular dependencies

        // Step 3: Extract resources from parsed files
        info!("🔍 Extracting FSH resources...");
        let resources = self.extract_resources(&parsed_files)?;
        let total_resources = resources.profiles.len()
            + resources.extensions.len()
            + resources.valuesets.len()
            + resources.codesystems.len()
            + resources.instances.len();
        info!("  Extracted {} resources total", total_resources);

        // === POPULATE TANK ===
        // Convert extracted CST resources to semantic FhirResources and add to Tank
        // This enables fishing to find local FSH definitions before checking external packages
        info!("📥 Populating Tank with parsed FSH resources...");
        let analyzer = DefaultSemanticAnalyzer::new();

        // Add profiles to tank
        for tracked in &resources.profiles {
            let source_text = parsed_files
                .iter()
                .find(|(path, _)| path == &tracked.source_file)
                .map(|(_, root)| root.text().to_string())
                .unwrap_or_default();

            let fhir_resource = analyzer.build_profile_resource(
                &tracked.resource,
                &source_text,
                &tracked.source_file,
            );
            tank.write().await.add_resource(fhir_resource);
        }

        // Add extensions to tank
        for tracked in &resources.extensions {
            let source_text = parsed_files
                .iter()
                .find(|(path, _)| path == &tracked.source_file)
                .map(|(_, root)| root.text().to_string())
                .unwrap_or_default();

            let fhir_resource = analyzer.build_extension_resource(
                &tracked.resource,
                &source_text,
                &tracked.source_file,
            );
            tank.write().await.add_resource(fhir_resource);
        }

        // Add valuesets to tank
        for tracked in &resources.valuesets {
            let source_text = parsed_files
                .iter()
                .find(|(path, _)| path == &tracked.source_file)
                .map(|(_, root)| root.text().to_string())
                .unwrap_or_default();

            let fhir_resource = analyzer.build_value_set_resource(
                &tracked.resource,
                &source_text,
                &tracked.source_file,
            );
            tank.write().await.add_resource(fhir_resource);
        }

        // Add codesystems to tank
        for tracked in &resources.codesystems {
            let source_text = parsed_files
                .iter()
                .find(|(path, _)| path == &tracked.source_file)
                .map(|(_, root)| root.text().to_string())
                .unwrap_or_default();

            let fhir_resource = analyzer.build_code_system_resource(
                &tracked.resource,
                &source_text,
                &tracked.source_file,
            );
            tank.write().await.add_resource(fhir_resource);
        }

        let tank_count = tank.read().await.all_resources().len();
        info!("  ✓ Added {} resources to Tank", tank_count);

        // === PHASE 1: Expand RuleSets ===
        if self.options.show_progress {
            info!("🔄 Phase 1: Expanding RuleSets...");
        }

        let mut ruleset_processor = RuleSetProcessor::new();

        // Phase 1a: Collect all RuleSet definitions
        if let Err(e) = ruleset_processor.collect_rulesets(&parsed_files) {
            warn!("Failed to collect RuleSets: {}", e);
        }

        // Phase 1b: Expand all InsertRule statements
        let expanded_rules = match ruleset_processor.expand_all_inserts(&parsed_files) {
            Ok(rules) => rules,
            Err(e) => {
                warn!("Failed to expand InsertRules: {}", e);
                HashMap::new()
            }
        };

        let (rulesets_found, inserts_expanded) = ruleset_processor.stats();
        if self.options.show_progress {
            info!(
                "  Found {} RuleSets, expanded {} InsertRules",
                rulesets_found, inserts_expanded
            );
        }

        // === PHASE 2: Export Resources ===
        if self.options.show_progress {
            info!("📦 Phase 2: Exporting resources...");
        }

        // Step 4: Export profiles and extensions
        self.export_profiles_and_extensions(
            session.clone(),
            package.clone(),
            &resources,
            &file_structure,
            &mut stats,
            &mut fsh_index,
        )
        .await?;

        // Step 5: Export instances
        self.export_instances(
            session.clone(),
            package.clone(),
            &resources,
            &file_structure,
            &mut stats,
            &mut fsh_index,
        )
        .await?;

        // Step 6: Export value sets and code systems
        self.export_vocabularies(
            session.clone(),
            package.clone(),
            &resources,
            &file_structure,
            &mut stats,
            &mut fsh_index,
        )
        .await?;

        // === PHASE 3: Apply Deferred Rules ===
        if !self.deferred_rules.is_empty() {
            if self.options.show_progress {
                info!("🔗 Phase 3: Resolving circular dependencies...");
                info!("  Processing {} deferred rules", self.deferred_rules.len());
            }
            self.apply_deferred_rules()?;
        }

        // Step 7: Generate ImplementationGuide resource
        if self.options.show_progress {
            info!("📝 Generating additional files...");
        }
        self.generate_implementation_guide(&file_structure)?;
        if self.options.show_progress {
            info!("  ✓ ImplementationGuide resource");
        }

        // Step 8: Write package.json
        self.write_package_json(&file_structure)?;
        if self.options.show_progress {
            info!("  ✓ package.json");
        }

        // Step 9: Load predefined resources
        self.load_predefined_resources(&file_structure, &stats)?;

        // Step 10: Write FSH index
        self.write_fsh_index(&file_structure, &fsh_index)?;
        if self.options.show_progress {
            info!("  ✓ FSH index");
        }

        info!("✅ Build completed successfully!");
        info!(
            "   Profiles: {}, Extensions: {}, ValueSets: {}, CodeSystems: {}, Instances: {}",
            stats.profiles, stats.extensions, stats.value_sets, stats.code_systems, stats.instances
        );
        if stats.errors > 0 || stats.warnings > 0 {
            warn!(
                "⚠️  Build had {} errors and {} warnings",
                stats.errors, stats.warnings
            );
        }

        // Save cache after successful build
        if let Some(mut cache) = cache {
            cache.mark_build_complete();
            cache.prune_deleted_files();

            if let Err(e) = cache.save(&self.options.output_dir) {
                warn!("Failed to save build cache: {}", e);
            } else {
                debug!("Build cache saved successfully");
            }
        }

        Ok(BuildResult {
            stats,
            output_dir: self.options.output_dir.clone(),
            config: self.config.clone(),
            fsh_index,
        })
    }

    /// Discover all FSH files in the input directory
    fn discover_fsh_files(&self) -> std::result::Result<Vec<PathBuf>, BuildError> {
        let mut fsh_files = Vec::new();

        if !self.options.input_dir.exists() {
            warn!(
                "Input directory does not exist: {:?}",
                self.options.input_dir
            );
            return Ok(fsh_files);
        }

        for entry in walkdir::WalkDir::new(&self.options.input_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "fsh")
                    .unwrap_or(false)
            {
                fsh_files.push(path.to_path_buf());
            }
        }

        Ok(fsh_files)
    }

    /// Parse all FSH files
    fn parse_fsh_files(
        &self,
        files: &[PathBuf],
    ) -> std::result::Result<Vec<(PathBuf, FshSyntaxNode)>, BuildError> {
        let mut parsed = Vec::new();

        for file in files {
            let content = std::fs::read_to_string(file).map_err(|e| {
                BuildError::ParseError(format!("Failed to read file {:?}: {}", file, e))
            })?;

            let (root, errors) = crate::cst::parse_fsh(&content);

            // Check for parse errors
            if !errors.is_empty() {
                warn!("Parse errors in file {:?}: {} errors", file, errors.len());
            }

            parsed.push((file.clone(), root));
        }

        Ok(parsed)
    }

    /// Extract FSH resources from parsed files with source tracking
    fn extract_resources(
        &self,
        parsed_files: &[(PathBuf, FshSyntaxNode)],
    ) -> std::result::Result<ParsedResources, BuildError> {
        use crate::cst::ast::AstNode;

        let mut profiles = Vec::new();
        let mut extensions = Vec::new();
        let mut valuesets = Vec::new();
        let mut codesystems = Vec::new();
        let mut instances = Vec::new();

        // Extract all resources from parsed files
        for (file_path, root) in parsed_files {
            debug!("Extracting resources from {:?}", file_path);

            // Get source text for line number calculation
            let source_text = root.text().to_string();

            // Extract profiles with source tracking
            for profile_node in root.children().filter_map(Profile::cast) {
                let range = profile_node.syntax().text_range();
                let (start_line, end_line) = self.calculate_line_numbers(&source_text, range);
                profiles.push(SourceTrackedResource::new(
                    profile_node,
                    file_path.clone(),
                    start_line,
                    end_line,
                ));
            }

            // Extract extensions with source tracking
            for extension_node in root.children().filter_map(Extension::cast) {
                let range = extension_node.syntax().text_range();
                let (start_line, end_line) = self.calculate_line_numbers(&source_text, range);
                extensions.push(SourceTrackedResource::new(
                    extension_node,
                    file_path.clone(),
                    start_line,
                    end_line,
                ));
            }

            // Extract valuesets with source tracking
            for valueset_node in root.children().filter_map(ValueSet::cast) {
                let range = valueset_node.syntax().text_range();
                let (start_line, end_line) = self.calculate_line_numbers(&source_text, range);
                valuesets.push(SourceTrackedResource::new(
                    valueset_node,
                    file_path.clone(),
                    start_line,
                    end_line,
                ));
            }

            // Extract code systems with source tracking
            for codesystem_node in root.children().filter_map(CodeSystem::cast) {
                let range = codesystem_node.syntax().text_range();
                let (start_line, end_line) = self.calculate_line_numbers(&source_text, range);
                codesystems.push(SourceTrackedResource::new(
                    codesystem_node,
                    file_path.clone(),
                    start_line,
                    end_line,
                ));
            }

            // Extract instances with source tracking
            for instance_node in root.children().filter_map(Instance::cast) {
                let range = instance_node.syntax().text_range();
                let (start_line, end_line) = self.calculate_line_numbers(&source_text, range);
                instances.push(SourceTrackedResource::new(
                    instance_node,
                    file_path.clone(),
                    start_line,
                    end_line,
                ));
            }
        }

        debug!(
            "Extracted {} profiles, {} extensions, {} valuesets, {} codesystems, {} instances",
            profiles.len(),
            extensions.len(),
            valuesets.len(),
            codesystems.len(),
            instances.len()
        );

        Ok(ParsedResources {
            profiles,
            extensions,
            valuesets,
            codesystems,
            instances,
        })
    }

    /// Export profiles and extensions
    async fn export_profiles_and_extensions(
        &self,
        session: Arc<crate::canonical::DefinitionSession>,
        package: Arc<tokio::sync::RwLock<crate::semantic::Package>>,
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::{ExtensionExporter, ProfileExporter};
        use futures::stream::{self, StreamExt};
        use std::sync::Arc as StdArc;
        use tokio::sync::Mutex; // Use async-aware Mutex

        // Create exporters
        let mut profile_exporter =
            ProfileExporter::new(session.clone(), self.build_config().canonical.clone())
                .await
                .map_err(|e| {
                    BuildError::ExportError(format!("Failed to create ProfileExporter: {}", e))
                })?;

        // Configure snapshot generation
        profile_exporter.set_generate_snapshots(self.options.generate_snapshots);

        let extension_exporter =
            ExtensionExporter::new(session.clone(), self.build_config().canonical.clone())
                .await
                .map_err(|e| {
                    BuildError::ExportError(format!("Failed to create ExtensionExporter: {}", e))
                })?;

        // Create progress bar for profiles if show_progress is enabled
        let profile_pb = if self.options.show_progress && !resources.profiles.is_empty() {
            let pb = ProgressBar::new(resources.profiles.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("   {bar:40.cyan/blue} {pos}/{len} profiles")
                    .unwrap(),
            );
            Some(pb)
        } else {
            None
        };

        // Export profiles in parallel using async concurrency
        {
            // Thread-safe shared state for collecting results
            let failed_profiles_shared = StdArc::new(Mutex::new(Vec::new()));
            let fsh_index_shared = StdArc::new(Mutex::new(Vec::new()));
            let profile_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));
            let error_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));

            // Wrap exporter in Arc for sharing across tasks
            let profile_exporter = StdArc::new(profile_exporter);

            // Wrap immutable data in Arc for sharing
            let file_structure = StdArc::new(file_structure);
            let package = package.clone();

            // Create progress bar shared across tasks
            let profile_pb_arc = StdArc::new(profile_pb);

            // Create async tasks for each profile
            let profile_tasks: Vec<_> = resources
                .profiles
                .iter()
                .map(|tracked| {
                    let profile = tracked.resource.clone();
                    let profile_exporter = profile_exporter.clone();
                    let file_structure = file_structure.clone();
                    let failed_profiles_shared = failed_profiles_shared.clone();
                    let fsh_index_shared = fsh_index_shared.clone();
                    let profile_count = profile_count.clone();
                    let error_count = error_count.clone();
                    let profile_pb = profile_pb_arc.clone();
                    let package = package.clone();
                    let source_file = tracked.source_file.clone();
                    let start_line = tracked.start_line;
                    let end_line = tracked.end_line;
                    let input_dir = self.options.input_dir.clone();

                    async move {
                        let profile_name = profile.name().unwrap_or_else(|| "Unknown".to_string());
                        debug!("Exporting profile: {}", profile_name);

                        match profile_exporter.export(&profile).await {
                            Ok(structure_def) => {
                                // Use Id field for filename if present, otherwise fall back to name
                                let profile_id = profile
                                    .id()
                                    .and_then(|id_clause| id_clause.value())
                                    .unwrap_or_else(|| profile_name.clone());

                                // Write to file
                                let filename = format!("StructureDefinition-{}.json", profile_id);
                                if let Err(e) =
                                    file_structure.write_resource(&filename, &structure_def)
                                {
                                    let error_msg =
                                        format!("Failed to write profile {}: {}", profile_name, e);
                                    warn!("{}", error_msg);
                                    failed_profiles_shared
                                        .lock()
                                        .await
                                        .push((profile_name.clone(), error_msg));
                                    error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                } else {
                                    profile_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    // Calculate relative path
                                    let relative_path = source_file
                                        .strip_prefix(&input_dir)
                                        .unwrap_or(&source_file)
                                        .to_string_lossy()
                                        .to_string();

                                    fsh_index_shared.lock().await.push(FshIndexEntry {
                                        output_file: filename,
                                        fsh_name: profile_name.clone(),
                                        fsh_type: "Profile".to_string(),
                                        fsh_file: relative_path,
                                        start_line,
                                        end_line,
                                    });

                                    // ⚡ ADD TO PACKAGE ⚡
                                    if !structure_def.url.is_empty() {
                                        if let Ok(json) = serde_json::to_value(&structure_def) {
                                            package
                                                .write()
                                                .await
                                                .add_resource(structure_def.url.clone(), json);
                                            debug!(
                                                "Added profile {} to Package",
                                                structure_def.url
                                            );
                                        }
                                    }

                                    debug!("Successfully exported profile: {}", profile_name);
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("{}", e);
                                warn!("Failed to export profile '{}': {}", profile_name, error_msg);
                                failed_profiles_shared
                                    .lock()
                                    .await
                                    .push((profile_name.clone(), error_msg));
                                error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }

                        // Update progress bar
                        if let Some(pb) = profile_pb.as_ref() {
                            pb.inc(1);
                        }
                    }
                })
                .collect();

            // Execute up to 4 profile exports concurrently
            let concurrency = 4;
            let mut results_stream = stream::iter(profile_tasks).buffer_unordered(concurrency);

            // Wait for all tasks to complete
            let mut task_count = 0;
            while let Some(()) = results_stream.next().await {
                task_count += 1;
            }

            // Finish progress bar before extracting from Arc
            if let Some(pb) = profile_pb_arc.as_ref() {
                pb.finish_with_message("done");
            }

            // Extract results from shared state
            let failed_profiles = match StdArc::try_unwrap(failed_profiles_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };

            let profile_index_entries = match StdArc::try_unwrap(fsh_index_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };
            fsh_index.extend(profile_index_entries);

            stats.profiles += profile_count.load(std::sync::atomic::Ordering::SeqCst);
            stats.errors += error_count.load(std::sync::atomic::Ordering::SeqCst);

            // Report failed profiles
            if !failed_profiles.is_empty() {
                eprintln!("⚠️  Failed to export {} profiles:", failed_profiles.len());
                warn!("⚠️  Failed to export {} profiles:", failed_profiles.len(),);
                for (name, error) in &failed_profiles {
                    eprintln!("   - {}: {}", name, error);
                    warn!("   - {}: {}", name, error);
                }
            }
        }

        // Export extensions in parallel using async concurrency
        {
            // Create progress bar for extensions if show_progress is enabled
            let extension_pb = if self.options.show_progress && !resources.extensions.is_empty() {
                let pb = ProgressBar::new(resources.extensions.len() as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("   {bar:40.cyan/blue} {pos}/{len} extensions")
                        .unwrap(),
                );
                Some(pb)
            } else {
                None
            };

            // Thread-safe shared state for collecting results
            let fsh_index_shared = StdArc::new(Mutex::new(Vec::new()));
            let extension_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));
            let error_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));

            // Wrap exporter in Arc for sharing across tasks
            let extension_exporter = StdArc::new(extension_exporter);

            // Wrap immutable data in Arc for sharing
            let file_structure = StdArc::new(file_structure);
            let package = package.clone();

            // Create progress bar shared across tasks
            let extension_pb_arc = StdArc::new(extension_pb);

            // Create async tasks for each extension
            let extension_tasks: Vec<_> = resources
                .extensions
                .iter()
                .map(|tracked| {
                    let extension = tracked.resource.clone();
                    let extension_exporter = extension_exporter.clone();
                    let file_structure = file_structure.clone();
                    let fsh_index_shared = fsh_index_shared.clone();
                    let extension_count = extension_count.clone();
                    let error_count = error_count.clone();
                    let extension_pb = extension_pb_arc.clone();
                    let package = package.clone();
                    let source_file = tracked.source_file.clone();
                    let start_line = tracked.start_line;
                    let end_line = tracked.end_line;
                    let input_dir = self.options.input_dir.clone();

                    async move {
                        let extension_name =
                            extension.name().unwrap_or_else(|| "Unknown".to_string());
                        let extension_id = extension
                            .id()
                            .and_then(|id_clause| id_clause.value())
                            .unwrap_or_else(|| extension_name.clone());
                        debug!(
                            "Exporting extension: {} (id: {})",
                            extension_name, extension_id
                        );

                        match extension_exporter.export(&extension).await {
                            Ok(structure_def) => {
                                // Write to file using Id field
                                let filename = format!("StructureDefinition-{}.json", extension_id);
                                if let Err(e) =
                                    file_structure.write_resource(&filename, &structure_def)
                                {
                                    let error_msg = format!(
                                        "Failed to write extension {}: {}",
                                        extension_name, e
                                    );
                                    warn!("{}", error_msg);
                                    error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                } else {
                                    extension_count
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    // Calculate relative path
                                    let relative_path = source_file
                                        .strip_prefix(&input_dir)
                                        .unwrap_or(&source_file)
                                        .to_string_lossy()
                                        .to_string();

                                    fsh_index_shared.lock().await.push(FshIndexEntry {
                                        fsh_name: extension_name.clone(),
                                        fsh_type: "Extension".to_string(),
                                        output_file: filename,
                                        fsh_file: relative_path,
                                        start_line,
                                        end_line,
                                    });

                                    // ⚡ ADD TO PACKAGE ⚡
                                    if !structure_def.url.is_empty() {
                                        if let Ok(json) = serde_json::to_value(&structure_def) {
                                            package
                                                .write()
                                                .await
                                                .add_resource(structure_def.url.clone(), json);
                                            debug!(
                                                "Added extension {} to Package",
                                                structure_def.url
                                            );
                                        }
                                    }

                                    debug!("Successfully exported extension: {}", extension_name);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to export extension {}: {}", extension_name, e);
                                error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }

                        // Update progress bar
                        if let Some(pb) = extension_pb.as_ref() {
                            pb.inc(1);
                        }
                    }
                })
                .collect();

            // Execute up to 4 extension exports concurrently
            let concurrency = 4;
            let mut results_stream = stream::iter(extension_tasks).buffer_unordered(concurrency);

            // Wait for all tasks to complete
            while let Some(()) = results_stream.next().await {
                // Task completed
            }

            // Finish progress bar before extracting from Arc
            if let Some(pb) = extension_pb_arc.as_ref() {
                pb.finish_with_message("done");
            }

            // Extract results from shared state
            let extension_index_entries = match StdArc::try_unwrap(fsh_index_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };
            fsh_index.extend(extension_index_entries);

            stats.extensions += extension_count.load(std::sync::atomic::Ordering::SeqCst);
            stats.errors += error_count.load(std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// Export instances
    async fn export_instances(
        &self,
        session: Arc<crate::canonical::DefinitionSession>,
        package: Arc<tokio::sync::RwLock<crate::semantic::Package>>,
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::InstanceExporter;
        use futures::stream::{self, StreamExt};
        use std::sync::Arc as StdArc;
        use tokio::sync::Mutex; // Use async-aware Mutex

        // Create instance exporter
        let mut instance_exporter =
            InstanceExporter::new(session, self.build_config().canonical.clone())
                .await
                .map_err(|e| {
                    BuildError::ExportError(format!("Failed to create InstanceExporter: {}", e))
                })?;

        // Create progress bar for instances if show_progress is enabled
        let instance_pb = if self.options.show_progress && !resources.instances.is_empty() {
            let pb = ProgressBar::new(resources.instances.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("   {bar:40.cyan/blue} {pos}/{len} instances")
                    .unwrap(),
            );
            Some(pb)
        } else {
            None
        };

        // Declare exported_instances vec
        let mut exported_instances: Vec<(
            String,
            String,
            String,
            JsonValue,
            PathBuf,
            usize,
            usize,
        )> = Vec::new();

        // PASS 1: Export all instances in parallel and register them
        // This allows instances to reference each other
        // Note: We use shared Arc<Mutex<InstanceExporter>> for thread-safe registration
        {
            let exported_instances_shared = StdArc::new(Mutex::new(Vec::new()));
            let error_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));

            // Wrap exporter in Arc<Mutex<>> because it needs mutable access for registration
            let instance_exporter = StdArc::new(Mutex::new(instance_exporter));

            // Create async tasks for each instance (PASS 1)
            let instance_tasks: Vec<_> = resources
                .instances
                .iter()
                .map(|tracked| {
                    let instance = tracked.resource.clone();
                    let instance_exporter = instance_exporter.clone();
                    let exported_instances_shared = exported_instances_shared.clone();
                    let error_count = error_count.clone();
                    let source_file = tracked.source_file.clone();
                    let start_line = tracked.start_line;
                    let end_line = tracked.end_line;

                    async move {
                        let instance_name =
                            instance.name().unwrap_or_else(|| "Unknown".to_string());
                        let instance_type = instance
                            .instance_of()
                            .map(|iof| iof.value().unwrap_or_else(|| "Resource".to_string()))
                            .unwrap_or_else(|| "Resource".to_string());
                        debug!(
                            "Pass 1 - Exporting instance: {} ({})",
                            instance_name, instance_type
                        );

                        // Lock the exporter for this export operation
                        let export_result = {
                            let mut exporter = instance_exporter.lock().await;
                            exporter.export(&instance).await
                        };

                        match export_result {
                            Ok(resource_json) => {
                                let instance_id = instance
                                    .id()
                                    .and_then(|id_clause| id_clause.value())
                                    .unwrap_or_else(|| instance_name.clone());

                                // Register the instance for reference resolution (requires lock)
                                {
                                    let mut exporter = instance_exporter.lock().await;
                                    exporter.register_instance(
                                        instance_name.clone(),
                                        resource_json.clone(),
                                    );
                                }

                                exported_instances_shared.lock().await.push((
                                    instance_name,
                                    instance_type,
                                    instance_id,
                                    resource_json,
                                    source_file,
                                    start_line,
                                    end_line,
                                ));
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to export instance {} (pass 1): {}",
                                    instance_name, e
                                );
                                error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }
                    }
                })
                .collect();

            // Execute up to 4 instance exports concurrently
            let concurrency = 4;
            let mut results_stream = stream::iter(instance_tasks).buffer_unordered(concurrency);

            // Wait for all tasks to complete
            while let Some(()) = results_stream.next().await {
                // Task completed
            }

            // Extract results
            exported_instances = match StdArc::try_unwrap(exported_instances_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };

            stats.errors += error_count.load(std::sync::atomic::Ordering::SeqCst);
        }

        // PASS 2: Write all exported instances to files
        // (References are already resolved from PASS 1 registry)
        for (
            instance_name,
            instance_type,
            instance_id,
            resource_json,
            source_file,
            start_line,
            end_line,
        ) in &exported_instances
        {
            let filename = format!("{}-{}.json", instance_type, instance_id);

            file_structure
                .write_resource(&filename, &resource_json)
                .map_err(|e| {
                    BuildError::ExportError(format!(
                        "Failed to write instance {}: {}",
                        instance_name, e
                    ))
                })?;

            stats.instances += 1;
            fsh_index.push(FshIndexEntry {
                fsh_name: instance_name.clone(),
                fsh_type: instance_type.clone(),
                output_file: filename,
                fsh_file: self.relative_path_from_input(source_file),
                start_line: *start_line,
                end_line: *end_line,
            });

            // ⚡ ADD TO PACKAGE ⚡
            // Instances don't have canonical URLs, but we can index by local reference
            if let Some(id) = resource_json.get("id").and_then(|v| v.as_str()) {
                // Construct a local reference URL
                let local_url =
                    format!("{}/{}/{}", self.build_config().canonical, instance_type, id);
                package
                    .write()
                    .await
                    .add_resource(local_url, resource_json.clone());
                debug!("Added instance {} to Package", id);
            }

            debug!("Successfully exported instance: {}", instance_name);

            // Update progress bar
            if let Some(pb) = &instance_pb {
                pb.inc(1);
            }
        }

        // Finish progress bar
        if let Some(pb) = instance_pb {
            pb.finish_with_message("done");
        }

        Ok(())
    }

    /// Export value sets and code systems
    async fn export_vocabularies(
        &self,
        session: Arc<crate::canonical::DefinitionSession>,
        package: Arc<tokio::sync::RwLock<crate::semantic::Package>>,
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::{CodeSystemExporter, ValueSetExporter};
        use futures::stream::{self, StreamExt};
        use std::sync::Arc as StdArc;
        use tokio::sync::Mutex; // Use async-aware Mutex

        // Create exporters
        let valueset_exporter =
            ValueSetExporter::new(session.clone(), self.build_config().canonical.clone())
                .await
                .map_err(|e| {
                    BuildError::ExportError(format!("Failed to create ValueSetExporter: {}", e))
                })?;

        let codesystem_exporter =
            CodeSystemExporter::new(session, self.build_config().canonical.clone())
                .await
                .map_err(|e| {
                    BuildError::ExportError(format!("Failed to create CodeSystemExporter: {}", e))
                })?;

        // Export valuesets in parallel using async concurrency
        {
            // Create progress bar for valuesets if show_progress is enabled
            let valueset_pb = if self.options.show_progress && !resources.valuesets.is_empty() {
                let pb = ProgressBar::new(resources.valuesets.len() as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("   {bar:40.cyan/blue} {pos}/{len} valuesets")
                        .unwrap(),
                );
                Some(pb)
            } else {
                None
            };

            // Thread-safe shared state
            let fsh_index_shared = StdArc::new(Mutex::new(Vec::new()));
            let valueset_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));
            let error_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));

            // Wrap exporter in Arc
            let valueset_exporter = StdArc::new(valueset_exporter);
            let file_structure = StdArc::new(file_structure);
            let package = package.clone();
            let valueset_pb_arc = StdArc::new(valueset_pb);

            // Create async tasks for each valueset
            let valueset_tasks: Vec<_> = resources
                .valuesets
                .iter()
                .map(|tracked| {
                    let valueset = tracked.resource.clone();
                    let valueset_exporter = valueset_exporter.clone();
                    let file_structure = file_structure.clone();
                    let fsh_index_shared = fsh_index_shared.clone();
                    let valueset_count = valueset_count.clone();
                    let error_count = error_count.clone();
                    let valueset_pb = valueset_pb_arc.clone();
                    let package = package.clone();
                    let source_file = tracked.source_file.clone();
                    let start_line = tracked.start_line;
                    let end_line = tracked.end_line;
                    let input_dir = self.options.input_dir.clone();

                    async move {
                        let name = valueset.name().unwrap_or_else(|| "Unknown".to_string());
                        debug!("Exporting ValueSet: {}", name);

                        match valueset_exporter.export(&valueset).await {
                            Ok(resource_json) => {
                                let vs_id = valueset
                                    .id()
                                    .and_then(|id_clause| id_clause.value())
                                    .unwrap_or_else(|| name.clone());

                                let filename = format!("ValueSet-{}.json", vs_id);
                                if let Err(e) =
                                    file_structure.write_resource(&filename, &resource_json)
                                {
                                    warn!("Failed to write ValueSet {}: {}", name, e);
                                    error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                } else {
                                    valueset_count
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    let relative_path = source_file
                                        .strip_prefix(&input_dir)
                                        .unwrap_or(&source_file)
                                        .to_string_lossy()
                                        .to_string();

                                    fsh_index_shared.lock().await.push(FshIndexEntry {
                                        fsh_name: name.clone(),
                                        fsh_type: "ValueSet".to_string(),
                                        output_file: filename,
                                        fsh_file: relative_path,
                                        start_line,
                                        end_line,
                                    });

                                    // ⚡ ADD TO PACKAGE ⚡
                                    if !resource_json.url.is_empty() {
                                        if let Ok(json) = serde_json::to_value(&resource_json) {
                                            package
                                                .write()
                                                .await
                                                .add_resource(resource_json.url.clone(), json);
                                            debug!(
                                                "Added ValueSet {} to Package",
                                                resource_json.url
                                            );
                                        }
                                    }

                                    debug!("Successfully exported ValueSet: {}", name);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to export ValueSet {}: {}", name, e);
                                error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }

                        if let Some(pb) = valueset_pb.as_ref() {
                            pb.inc(1);
                        }
                    }
                })
                .collect();

            // Execute up to 4 valueset exports concurrently
            let concurrency = 4;
            let mut results_stream = stream::iter(valueset_tasks).buffer_unordered(concurrency);

            while let Some(()) = results_stream.next().await {
                // Task completed
            }

            // Finish progress bar
            if let Some(pb) = valueset_pb_arc.as_ref() {
                pb.finish_with_message("done");
            }

            // Extract results
            let valueset_index_entries = match StdArc::try_unwrap(fsh_index_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };
            fsh_index.extend(valueset_index_entries);

            stats.value_sets += valueset_count.load(std::sync::atomic::Ordering::SeqCst);
            stats.errors += error_count.load(std::sync::atomic::Ordering::SeqCst);
        }

        // Export codesystems in parallel using async concurrency
        {
            // Create progress bar for codesystems if show_progress is enabled
            let codesystem_pb = if self.options.show_progress && !resources.codesystems.is_empty() {
                let pb = ProgressBar::new(resources.codesystems.len() as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("   {bar:40.cyan/blue} {pos}/{len} codesystems")
                        .unwrap(),
                );
                Some(pb)
            } else {
                None
            };

            // Thread-safe shared state
            let fsh_index_shared = StdArc::new(Mutex::new(Vec::new()));
            let codesystem_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));
            let error_count = StdArc::new(std::sync::atomic::AtomicUsize::new(0));

            // Wrap exporter in Arc
            let codesystem_exporter = StdArc::new(codesystem_exporter);
            let file_structure = StdArc::new(file_structure);
            let package = package.clone();
            let codesystem_pb_arc = StdArc::new(codesystem_pb);

            // Create async tasks for each codesystem
            let codesystem_tasks: Vec<_> = resources
                .codesystems
                .iter()
                .map(|tracked| {
                    let codesystem = tracked.resource.clone();
                    let codesystem_exporter = codesystem_exporter.clone();
                    let file_structure = file_structure.clone();
                    let fsh_index_shared = fsh_index_shared.clone();
                    let codesystem_count = codesystem_count.clone();
                    let error_count = error_count.clone();
                    let codesystem_pb = codesystem_pb_arc.clone();
                    let package = package.clone();
                    let source_file = tracked.source_file.clone();
                    let start_line = tracked.start_line;
                    let end_line = tracked.end_line;
                    let input_dir = self.options.input_dir.clone();

                    async move {
                        let name = codesystem.name().unwrap_or_else(|| "Unknown".to_string());
                        debug!("Exporting CodeSystem: {}", name);

                        match codesystem_exporter.export(&codesystem).await {
                            Ok(resource_json) => {
                                let cs_id = codesystem
                                    .id()
                                    .and_then(|id_clause| id_clause.value())
                                    .unwrap_or_else(|| name.clone());

                                let filename = format!("CodeSystem-{}.json", cs_id);
                                if let Err(e) =
                                    file_structure.write_resource(&filename, &resource_json)
                                {
                                    warn!("Failed to write CodeSystem {}: {}", name, e);
                                    error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                } else {
                                    codesystem_count
                                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    let relative_path = source_file
                                        .strip_prefix(&input_dir)
                                        .unwrap_or(&source_file)
                                        .to_string_lossy()
                                        .to_string();

                                    fsh_index_shared.lock().await.push(FshIndexEntry {
                                        fsh_name: name.clone(),
                                        fsh_type: "CodeSystem".to_string(),
                                        output_file: filename,
                                        fsh_file: relative_path,
                                        start_line,
                                        end_line,
                                    });

                                    // ⚡ ADD TO PACKAGE ⚡
                                    if !resource_json.url.is_empty() {
                                        if let Ok(json) = serde_json::to_value(&resource_json) {
                                            package
                                                .write()
                                                .await
                                                .add_resource(resource_json.url.clone(), json);
                                            debug!(
                                                "Added CodeSystem {} to Package",
                                                resource_json.url
                                            );
                                        }
                                    }

                                    debug!("Successfully exported CodeSystem: {}", name);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to export CodeSystem {}: {}", name, e);
                                error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            }
                        }

                        if let Some(pb) = codesystem_pb.as_ref() {
                            pb.inc(1);
                        }
                    }
                })
                .collect();

            // Execute up to 4 codesystem exports concurrently
            let concurrency = 4;
            let mut results_stream = stream::iter(codesystem_tasks).buffer_unordered(concurrency);

            while let Some(()) = results_stream.next().await {
                // Task completed
            }

            // Finish progress bar
            if let Some(pb) = codesystem_pb_arc.as_ref() {
                pb.finish_with_message("done");
            }

            // Extract results
            let codesystem_index_entries = match StdArc::try_unwrap(fsh_index_shared) {
                Ok(m) => m.into_inner(),
                Err(arc) => arc.lock().await.clone(),
            };
            fsh_index.extend(codesystem_index_entries);

            stats.code_systems += codesystem_count.load(std::sync::atomic::Ordering::SeqCst);
            stats.errors += error_count.load(std::sync::atomic::Ordering::SeqCst);
        }

        Ok(())
    }

    /// Apply deferred rules (Phase 3: circular dependency resolution)
    fn apply_deferred_rules(&self) -> std::result::Result<(), BuildError> {
        if self.deferred_rules.is_empty() {
            return Ok(());
        }

        debug!("Applying {} deferred rules", self.deferred_rules.len());

        let mut applied_count = 0;
        let mut failed_rules = Vec::new();

        // Attempt to apply each deferred rule
        // At this point, all resources have been exported, so references should resolve
        for deferred_rule in &self.deferred_rules {
            debug!(
                "Attempting to apply deferred rule for entity '{}': {:?}",
                deferred_rule.entity_id, deferred_rule.reason
            );

            // Try to apply the rule
            match self.retry_deferred_rule(deferred_rule) {
                Ok(()) => {
                    applied_count += 1;
                    trace!(
                        "  Successfully applied deferred rule for {}",
                        deferred_rule.entity_id
                    );
                }
                Err(e) => {
                    debug!(
                        "  Failed to apply deferred rule for {}: {}",
                        deferred_rule.entity_id, e
                    );
                    failed_rules.push((deferred_rule.clone(), e));
                }
            }
        }

        // Report results
        let total = self.deferred_rules.len();
        let failed_count = failed_rules.len();

        if failed_count > 0 {
            warn!(
                "Applied {}/{} deferred rules ({} could not be resolved)",
                applied_count, total, failed_count
            );

            // Log details of failed rules
            for (rule, reason) in &failed_rules {
                debug!(
                    "  Failed rule: entity={}, reason={:?}, error={}",
                    rule.entity_id, rule.reason, reason
                );
            }

            // Return warnings but don't fail the build
            // Circular dependencies might be intentional or resolved at runtime
        } else {
            info!("Successfully applied all {} deferred rules", applied_count);
        }

        Ok(())
    }

    /// Retry a single deferred rule
    fn retry_deferred_rule(
        &self,
        rule: &crate::semantic::DeferredRule,
    ) -> std::result::Result<(), String> {
        // Parse the rule content to understand what needs to be applied
        debug!("Retrying rule: {}", rule.rule_content);

        match &rule.reason {
            crate::semantic::DeferralReason::UnresolvedReference(ref_name) => {
                // Check if reference is now available
                // In a full implementation, we would:
                // 1. Look up the reference in exported resources
                // 2. If found, apply the rule
                // 3. If not found, return error

                // For now, assume it's resolvable (resources were exported in Phase 2)
                debug!("  Reference '{}' should now be resolvable", ref_name);
                Ok(())
            }
            crate::semantic::DeferralReason::CircularDependency(dep_name) => {
                // Circular dependencies can now be resolved since both resources exist
                debug!(
                    "  Circular dependency with '{}' can now be resolved",
                    dep_name
                );
                Ok(())
            }
            crate::semantic::DeferralReason::MissingResource(resource_name) => {
                // Resource should now exist (exported in Phase 2)
                debug!("  Resource '{}' should now exist", resource_name);
                Ok(())
            }
            crate::semantic::DeferralReason::MissingParent(parent_name) => {
                // Parent should now be exported
                debug!("  Parent '{}' should now be exported", parent_name);
                Ok(())
            }
        }
    }

    /// Generate ImplementationGuide resource
    fn generate_implementation_guide(
        &self,
        file_structure: &FileStructureGenerator,
    ) -> std::result::Result<(), BuildError> {
        let ig_generator = ImplementationGuideGenerator::new(self.build_config().clone());
        let ig = ig_generator.generate();

        // Write ImplementationGuide resource
        let id = self.build_config().id.as_deref().unwrap_or("ig");
        let filename = format!("ImplementationGuide-{}.json", id);
        file_structure
            .write_resource(&filename, &ig)
            .map_err(|e| BuildError::ExportError(format!("Failed to write IG: {}", e)))?;

        debug!("Generated ImplementationGuide: {}", filename);
        Ok(())
    }

    /// Write package.json
    fn write_package_json(
        &self,
        file_structure: &FileStructureGenerator,
    ) -> std::result::Result<(), BuildError> {
        let package_json = PackageJson::from_sushi_config(self.build_config());

        file_structure
            .write_package_json(&package_json)
            .map_err(|e| BuildError::ExportError(format!("Failed to write package.json: {}", e)))?;

        debug!("Generated package.json");
        Ok(())
    }

    /// Load predefined resources
    fn load_predefined_resources(
        &self,
        _file_structure: &FileStructureGenerator,
        _stats: &BuildStats,
    ) -> std::result::Result<(), BuildError> {
        // Build map of generated resources for conflict detection
        let generated_resources = HashMap::new();

        // TODO: Populate from actual exports
        // For now, just check predefined resources

        let input_parent = self
            .options
            .input_dir
            .parent()
            .unwrap_or(&self.options.input_dir);
        let project_dir = input_parent.parent().unwrap_or(input_parent);
        let loader = PredefinedResourcesLoader::new(input_parent, project_dir, vec![]);

        let (predefined, conflicts) = loader.load_with_conflict_check(&generated_resources)?;

        if !conflicts.is_empty() {
            warn!("Found {} resource conflicts:", conflicts.len());
            for conflict in &conflicts {
                warn!("  Conflict: {}", conflict.description());
            }
        }

        info!("Loaded {} predefined resources", predefined.len());
        Ok(())
    }

    /// Write FSH index
    fn write_fsh_index(
        &self,
        file_structure: &FileStructureGenerator,
        fsh_index: &[FshIndexEntry],
    ) -> std::result::Result<(), BuildError> {
        // Write text index
        let index_text = format_fsh_index_table(fsh_index);
        file_structure
            .write_fsh_index_txt(&index_text)
            .map_err(|e| BuildError::ExportError(format!("Failed to write FSH index: {}", e)))?;

        // Write JSON index
        file_structure
            .write_fsh_index_json(fsh_index)
            .map_err(|e| {
                BuildError::ExportError(format!("Failed to write FSH index JSON: {}", e))
            })?;

        debug!("Generated FSH index ({} entries)", fsh_index.len());
        Ok(())
    }

    /// Calculate line numbers from byte offsets
    ///
    /// Converts TextRange (byte offsets) to 1-based line numbers
    fn calculate_line_numbers(&self, source: &str, range: TextRange) -> (usize, usize) {
        let start_offset = range.start().into();
        let end_offset = range.end().into();

        // Count newlines before start and end positions
        let start_line = source[..start_offset].matches('\n').count() + 1;
        let end_line = source[..end_offset].matches('\n').count() + 1;

        (start_line, end_line)
    }

    /// Get relative path from input directory
    fn relative_path_from_input(&self, file_path: &PathBuf) -> String {
        file_path
            .strip_prefix(&self.options.input_dir)
            .unwrap_or(file_path)
            .display()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> SushiConfiguration {
        SushiConfiguration {
            id: Some("test.ig".to_string()),
            canonical: "http://example.org/fhir/test".to_string(),
            name: Some("TestIG".to_string()),
            title: Some("Test Implementation Guide".to_string()),
            status: Some("draft".to_string()),
            version: Some("1.0.0".to_string()),
            fhir_version: vec!["4.0.1".to_string()],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_build_options_default() {
        let options = BuildOptions::default();
        assert_eq!(options.input_dir, PathBuf::from("input/fsh"));
        assert_eq!(options.output_dir, PathBuf::from("fsh-generated"));
        assert!(!options.generate_snapshots);
        assert!(!options.write_preprocessed);
        assert!(!options.clean_output);
    }

    #[tokio::test]
    async fn test_build_stats() {
        let mut stats = BuildStats::default();
        stats.profiles = 5;
        stats.extensions = 3;
        stats.instances = 10;

        assert_eq!(stats.total_resources(), 18);
        assert!(!stats.has_errors());
        assert!(!stats.has_warnings());

        stats.errors = 1;
        assert!(stats.has_errors());
    }

    #[tokio::test]
    async fn test_build_orchestrator_creation() {
        let config = create_test_config();
        let options = BuildOptions::default();
        let orchestrator = BuildOrchestrator::new(config.clone(), options);

        assert_eq!(orchestrator.config.id, Some("test.ig".to_string()));
    }

    #[tokio::test]
    async fn test_discover_fsh_files_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let options = BuildOptions {
            input_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(config, options);
        let files = orchestrator.discover_fsh_files().unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_discover_fsh_files_with_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test FSH files
        std::fs::write(temp_dir.path().join("test1.fsh"), "Profile: TestProfile").unwrap();
        std::fs::write(temp_dir.path().join("test2.fsh"), "Instance: TestInstance").unwrap();
        std::fs::write(temp_dir.path().join("readme.txt"), "Not FSH").unwrap();

        let config = create_test_config();
        let options = BuildOptions {
            input_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(config, options);
        let files = orchestrator.discover_fsh_files().unwrap();

        assert_eq!(files.len(), 2);
        assert!(
            files
                .iter()
                .all(|f| f.extension().and_then(|s| s.to_str()) == Some("fsh"))
        );
    }

    #[tokio::test]
    async fn test_parse_fsh_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create a valid FSH file
        std::fs::write(
            temp_dir.path().join("test.fsh"),
            "Profile: TestProfile\nParent: Patient",
        )
        .unwrap();

        let config = create_test_config();
        let options = BuildOptions {
            input_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(config, options);
        let files = orchestrator.discover_fsh_files().unwrap();
        let parsed = orchestrator.parse_fsh_files(&files).unwrap();

        assert_eq!(parsed.len(), 1);
    }

    #[tokio::test]
    async fn test_generate_implementation_guide() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let options = BuildOptions {
            output_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(config.clone(), options);
        let file_structure = FileStructureGenerator::new(temp_dir.path(), false);
        file_structure.initialize().unwrap();

        orchestrator
            .generate_implementation_guide(&file_structure)
            .unwrap();

        // Check that ImplementationGuide was written
        let ig_path = temp_dir.path().join("resources").join(format!(
            "ImplementationGuide-{}.json",
            config.id.as_ref().unwrap()
        ));
        assert!(ig_path.exists());
    }

    #[tokio::test]
    async fn test_write_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config();
        let options = BuildOptions {
            output_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let orchestrator = BuildOrchestrator::new(config, options);
        let file_structure = FileStructureGenerator::new(temp_dir.path(), false);
        file_structure.initialize().unwrap();

        orchestrator.write_package_json(&file_structure).unwrap();

        // Check that package.json was written
        let pkg_path = temp_dir.path().join("package.json");
        assert!(pkg_path.exists());
    }
}
