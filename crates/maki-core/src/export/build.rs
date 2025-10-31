//! Build Orchestrator
//!
//! Coordinates all exporters to generate a complete FHIR IG package.
//! Implements SUSHI-compatible build pipeline with progress reporting.

use crate::config::SushiConfiguration;
use crate::cst::ast::{CodeSystem, Extension, Instance, Profile, ValueSet};
use crate::cst::FshSyntaxNode;
use crate::export::*;
use std::sync::Arc;
use crate::semantic::SemanticModel;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Parsed FSH resources ready for export
#[derive(Debug, Default)]
struct ParsedResources {
    profiles: Vec<Profile>,
    extensions: Vec<Extension>,
    valuesets: Vec<ValueSet>,
    codesystems: Vec<CodeSystem>,
    instances: Vec<Instance>,
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
    pub config: SushiConfiguration,

    /// FSH index entries for generated resources
    pub fsh_index: Vec<FshIndexEntry>,
}

/// Build orchestrator
///
/// Coordinates all exporters to generate a complete FHIR IG package.
/// Follows SUSHI's build pipeline:
/// 1. Parse FSH files
/// 2. Build semantic model
/// 3. Export profiles/extensions/logicals (with RuleSets)
/// 4. Export instances/value sets/code systems
/// 5. Generate ImplementationGuide resource
/// 6. Write package.json
/// 7. Load predefined resources
/// 8. Write FSH index
pub struct BuildOrchestrator {
    options: BuildOptions,
    config: SushiConfiguration,
}

impl BuildOrchestrator {
    /// Create a new build orchestrator
    pub fn new(config: SushiConfiguration, options: BuildOptions) -> Self {
        Self { config, options }
    }

    /// Run the complete build pipeline
    pub async fn build(&self) -> std::result::Result<BuildResult, BuildError> {
        info!("Starting MAKI build...");

        // Create canonical session for FHIR package resolution
        use crate::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
        let options = CanonicalOptions {
            auto_install_core: true,
            quick_init: true,
            ..Default::default()
        };
        let facade = CanonicalFacade::new(options).await
            .map_err(|e| BuildError::ExportError(format!("Failed to create CanonicalFacade: {}", e)))?;
        let session = Arc::new(facade.session([FhirRelease::R4]).await
            .map_err(|e| BuildError::ExportError(format!("Failed to create DefinitionSession: {}", e)))?);
        info!("Created FHIR package resolution session");

        info!("Input directory: {:?}", self.options.input_dir);
        info!("Output directory: {:?}", self.options.output_dir);

        // Initialize file structure
        let file_structure = FileStructureGenerator::new(
            &self.options.output_dir,
            self.options.clean_output,
        );
        file_structure.initialize()?;

        // Initialize stats
        let mut stats = BuildStats::default();
        let mut fsh_index = Vec::new();

        // Step 1: Discover FSH files
        info!("Discovering FSH files...");
        let fsh_files = self.discover_fsh_files()?;
        if fsh_files.is_empty() {
            return Err(BuildError::NoFshFiles);
        }
        info!("Found {} FSH files", fsh_files.len());

        // Step 2: Parse FSH files
        info!("Parsing FSH files...");
        let parsed_files = self.parse_fsh_files(&fsh_files)?;
        debug!("Parsed {} FSH files", parsed_files.len());

        // Step 3: Extract resources from parsed files
        info!("Extracting FSH resources...");
        let resources = self.extract_resources(&parsed_files)?;
        debug!("Extracted {} resources total",
            resources.profiles.len() + resources.extensions.len() +
            resources.valuesets.len() + resources.codesystems.len() + resources.instances.len());

        // Step 4: Export profiles and extensions
        info!("Exporting profiles and extensions...");
        self.export_profiles_and_extensions(
            session.clone(),
            &resources,
            &file_structure,
            &mut stats,
            &mut fsh_index,
        ).await?;

        // Step 5: Export instances
        info!("Exporting instances...");
        self.export_instances(session.clone(), &resources, &file_structure, &mut stats, &mut fsh_index).await?;

        // Step 6: Export value sets and code systems
        info!("Exporting value sets and code systems...");
        self.export_vocabularies(
            session.clone(),
            &resources,
            &file_structure,
            &mut stats,
            &mut fsh_index,
        ).await?;

        // Step 7: Generate ImplementationGuide resource
        info!("Generating ImplementationGuide resource...");
        self.generate_implementation_guide(&file_structure)?;

        // Step 8: Write package.json
        info!("Writing package.json...");
        self.write_package_json(&file_structure)?;

        // Step 9: Load predefined resources
        info!("Loading predefined resources...");
        self.load_predefined_resources(&file_structure, &stats)?;

        // Step 10: Write FSH index
        info!("Writing FSH index...");
        self.write_fsh_index(&file_structure, &fsh_index)?;

        info!("Build completed successfully!");
        info!(
            "Generated {} resources: {} profiles, {} extensions, {} valuesets, {} codesystems, {} instances",
            stats.total_resources(),
            stats.profiles,
            stats.extensions,
            stats.value_sets,
            stats.code_systems,
            stats.instances
        );
        if stats.errors > 0 || stats.warnings > 0 {
            warn!("Build had {} errors and {} warnings", stats.errors, stats.warnings);
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

    /// Extract FSH resources from parsed files
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

            // Extract profiles
            for profile_node in root.children().filter_map(Profile::cast) {
                profiles.push(profile_node);
            }

            // Extract extensions
            for extension_node in root.children().filter_map(Extension::cast) {
                extensions.push(extension_node);
            }

            // Extract valuesets
            for valueset_node in root.children().filter_map(ValueSet::cast) {
                valuesets.push(valueset_node);
            }

            // Extract code systems
            for codesystem_node in root.children().filter_map(CodeSystem::cast) {
                codesystems.push(codesystem_node);
            }

            // Extract instances
            for instance_node in root.children().filter_map(Instance::cast) {
                instances.push(instance_node);
            }
        }

        debug!("Extracted {} profiles, {} extensions, {} valuesets, {} codesystems, {} instances",
            profiles.len(), extensions.len(), valuesets.len(), codesystems.len(), instances.len());

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
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::{ExtensionExporter, ProfileExporter};

        // Create exporters
        let mut profile_exporter = ProfileExporter::new(session.clone(), self.config.canonical.clone())
            .await
            .map_err(|e| BuildError::ExportError(format!("Failed to create ProfileExporter: {}", e)))?;

        // Configure snapshot generation
        profile_exporter.set_generate_snapshots(self.options.generate_snapshots);

        let extension_exporter = ExtensionExporter::new(session.clone(), self.config.canonical.clone())
            .await
            .map_err(|e| BuildError::ExportError(format!("Failed to create ExtensionExporter: {}", e)))?;

        // Track failed exports for reporting
        let mut failed_profiles: Vec<(String, String)> = Vec::new();

        // Export profiles
        for profile in &resources.profiles {
            let profile_name = profile.name().unwrap_or_else(|| "Unknown".to_string());
            debug!("Exporting profile: {}", profile_name);

            match profile_exporter.export(profile).await {
                Ok(structure_def) => {
                    // Use Id field for filename if present, otherwise fall back to name
                    let profile_id = profile.id()
                        .and_then(|id_clause| id_clause.value())
                        .unwrap_or_else(|| profile_name.clone());

                    // Write to file
                    let filename = format!("StructureDefinition-{}.json", profile_id);
                    file_structure
                        .write_resource(&filename, &structure_def)
                        .map_err(|e| BuildError::ExportError(format!("Failed to write profile {}: {}", profile_name, e)))?;

                    stats.profiles += 1;
                    fsh_index.push(FshIndexEntry {
                        output_file: filename,
                        fsh_name: profile_name.clone(),
                        fsh_type: "Profile".to_string(),
                        fsh_file: "".to_string(),
                        start_line: 0,
                        end_line: 0,
                    });

                    debug!("Successfully exported profile: {}", profile_name);
                }
                Err(e) => {
                    // Log detailed error with full error chain
                    let error_msg = format!("{}", e); // Display error
                    warn!("Failed to export profile '{}': {}", profile_name, error_msg);
                    failed_profiles.push((profile_name.clone(), error_msg));
                    stats.errors += 1;
                }
            }
        }

        // Export extensions
        for extension in &resources.extensions {
            let extension_name = extension.name().unwrap_or_else(|| "Unknown".to_string());
            // Use Id field for filename if present, otherwise fall back to name
            let extension_id = extension.id()
                .and_then(|id_clause| id_clause.value())
                .unwrap_or_else(|| extension_name.clone());
            debug!("Exporting extension: {} (id: {})", extension_name, extension_id);

            match extension_exporter.export(extension).await {
                Ok(structure_def) => {
                    // Write to file using Id field
                    let filename = format!("StructureDefinition-{}.json", extension_id);
                    file_structure
                        .write_resource(&filename, &structure_def)
                        .map_err(|e| BuildError::ExportError(format!("Failed to write extension {}: {}", extension_name, e)))?;

                    stats.extensions += 1;
                    fsh_index.push(FshIndexEntry {
                        fsh_name: extension_name.clone(),
                        fsh_type: "StructureDefinition".to_string(),
                        output_file: filename,
                    fsh_file: "".to_string(), start_line: 0, end_line: 0, });

                    debug!("Successfully exported extension: {}", extension_name);
                }
                Err(e) => {
                    warn!("Failed to export extension {}: {}", extension_name, e);
                    stats.errors += 1;
                }
            }
        }

        // Log summary of failed profiles
        if !failed_profiles.is_empty() {
            warn!("Profile export summary: {} failed out of {} total", failed_profiles.len(), resources.profiles.len());
            warn!("Failed profiles:");
            for (name, error) in &failed_profiles {
                warn!("  - {}: {}", name, error);
            }
        } else {
            info!("All {} profiles exported successfully", resources.profiles.len());
        }

        Ok(())
    }

    /// Export instances
    async fn export_instances(
        &self,
        session: Arc<crate::canonical::DefinitionSession>,
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::InstanceExporter;

        // Create instance exporter
        let mut instance_exporter = InstanceExporter::new(session, self.config.canonical.clone())
            .await
            .map_err(|e| BuildError::ExportError(format!("Failed to create InstanceExporter: {}", e)))?;

        // Export instances
        for instance in &resources.instances {
            let instance_name = instance.name().unwrap_or_else(|| "Unknown".to_string());
            let instance_type = instance.instance_of().map(|iof| iof.value().unwrap_or_else(|| "Resource".to_string())).unwrap_or_else(|| "Resource".to_string());
            debug!("Exporting instance: {} ({})", instance_name, instance_type);

            match instance_exporter.export(instance).await {
                Ok(resource_json) => {
                    // Use Id field for filename if present, otherwise fall back to name (SUSHI compatible)
                    let instance_id = instance.id()
                        .and_then(|id_clause| id_clause.value())
                        .unwrap_or_else(|| instance_name.clone());

                    // Write to file
                    let filename = format!("{}-{}.json", instance_type, instance_id);
                    file_structure
                        .write_resource(&filename, &resource_json)
                        .map_err(|e| BuildError::ExportError(format!("Failed to write instance {}: {}", instance_name, e)))?;

                    stats.instances += 1;
                    fsh_index.push(FshIndexEntry {
                        fsh_name: instance_name.clone(),
                        fsh_type: instance_type,
                        output_file: filename,
                    fsh_file: "".to_string(), start_line: 0, end_line: 0, });

                    debug!("Successfully exported instance: {}", instance_name);
                }
                Err(e) => {
                    warn!("Failed to export instance {}: {}", instance_name, e);
                    stats.errors += 1;
                }
            }
        }

        Ok(())
    }

    /// Export value sets and code systems
    async fn export_vocabularies(
        &self,
        session: Arc<crate::canonical::DefinitionSession>,
        resources: &ParsedResources,
        file_structure: &FileStructureGenerator,
        stats: &mut BuildStats,
        fsh_index: &mut Vec<FshIndexEntry>,
    ) -> std::result::Result<(), BuildError> {
        use crate::export::{CodeSystemExporter, ValueSetExporter};

        // Create exporters
        let valueset_exporter = ValueSetExporter::new(session.clone(), self.config.canonical.clone())
            .await
            .map_err(|e| BuildError::ExportError(format!("Failed to create ValueSetExporter: {}", e)))?;

        let codesystem_exporter = CodeSystemExporter::new(session, self.config.canonical.clone())
            .await
            .map_err(|e| BuildError::ExportError(format!("Failed to create CodeSystemExporter: {}", e)))?;

        // Export valuesets
        for valueset in &resources.valuesets {
            let name = valueset.name().unwrap_or_else(|| "Unknown".to_string());
            debug!("Exporting ValueSet: {}", name);

            match valueset_exporter.export(valueset).await {
                Ok(resource_json) => {
                    // Use Id field for filename if present, otherwise fall back to name
                    let vs_id = valueset.id()
                        .and_then(|id_clause| id_clause.value())
                        .unwrap_or_else(|| name.clone());

                    let filename = format!("ValueSet-{}.json", vs_id);
                    file_structure
                        .write_resource(&filename, &resource_json)
                        .map_err(|e| BuildError::ExportError(format!("Failed to write ValueSet {}: {}", name, e)))?;

                    stats.value_sets += 1;
                    fsh_index.push(FshIndexEntry {
                        fsh_name: name.clone(),
                        fsh_type: "ValueSet".to_string(),
                        output_file: filename,
                    fsh_file: "".to_string(), start_line: 0, end_line: 0, });

                    debug!("Successfully exported ValueSet: {}", name);
                }
                Err(e) => {
                    warn!("Failed to export ValueSet {}: {}", name, e);
                    stats.errors += 1;
                }
            }
        }

        // Export codesystems
        for codesystem in &resources.codesystems {
            let name = codesystem.name().unwrap_or_else(|| "Unknown".to_string());
            debug!("Exporting CodeSystem: {}", name);

            match codesystem_exporter.export(codesystem).await {
                Ok(resource_json) => {
                    // Use Id field for filename if present, otherwise fall back to name
                    let cs_id = codesystem.id()
                        .and_then(|id_clause| id_clause.value())
                        .unwrap_or_else(|| name.clone());

                    let filename = format!("CodeSystem-{}.json", cs_id);
                    file_structure
                        .write_resource(&filename, &resource_json)
                        .map_err(|e| BuildError::ExportError(format!("Failed to write CodeSystem {}: {}", name, e)))?;

                    stats.code_systems += 1;
                    fsh_index.push(FshIndexEntry {
                        fsh_name: name.clone(),
                        fsh_type: "CodeSystem".to_string(),
                        output_file: filename,
                    fsh_file: "".to_string(), start_line: 0, end_line: 0, });

                    debug!("Successfully exported CodeSystem: {}", name);
                }
                Err(e) => {
                    warn!("Failed to export CodeSystem {}: {}", name, e);
                    stats.errors += 1;
                }
            }
        }

        Ok(())
    }

    /// Generate ImplementationGuide resource
    fn generate_implementation_guide(
        &self,
        file_structure: &FileStructureGenerator,
    ) -> std::result::Result<(), BuildError> {
        let ig_generator = ImplementationGuideGenerator::new(self.config.clone());
        let ig = ig_generator.generate();

        // Write ImplementationGuide resource
        let id = self.config.id.as_deref().unwrap_or("ig");
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
        let package_json = PackageJson::from_sushi_config(&self.config);

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
            .map_err(|e| BuildError::ExportError(format!("Failed to write FSH index JSON: {}", e)))?;

        debug!("Generated FSH index ({} entries)", fsh_index.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> SushiConfiguration {
        SushiConfiguration {
            id: "test.ig".to_string(),
            canonical: "http://example.org/fhir/test".to_string(),
            fsh_name: "TestIG".to_string(),
            title: Some("Test Implementation Guide".to_string()),
            status: "draft".to_string(),
            version: "1.0.0".to_string(),
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

        assert_eq!(orchestrator.config.id, "test.ig");
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
        assert!(files
            .iter()
            .all(|f| f.extension().and_then(|s| s.to_str()) == Some("fsh")));
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
        let ig_path = temp_dir
            .path()
            .join("resources")
            .join(format!("ImplementationGuide-{}.json", config.id));
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
