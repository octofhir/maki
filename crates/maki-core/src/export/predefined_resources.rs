//! Predefined Resources Handler
//!
//! This module handles loading and managing predefined FHIR resources
//! from the IG input directory, following SUSHI conventions.
//!
//! ## Standard Resource Directories
//!
//! The following directories are checked for predefined resources:
//! - `input/capabilities/` - CapabilityStatement resources
//! - `input/extensions/` - Extension definitions (non-FSH)
//! - `input/models/` - Logical models
//! - `input/operations/` - OperationDefinition resources
//! - `input/profiles/` - StructureDefinition profiles (non-FSH)
//! - `input/resources/` - General FHIR resources
//! - `input/vocabulary/` - ValueSet and CodeSystem resources
//! - `input/examples/` - Example instances
//!
//! ## Custom Directories
//!
//! Additional directories can be specified using the `path-resource`
//! parameter in sushi-config.yaml. Paths ending with `/*` will
//! recursively search subdirectories.
//!
//! ## Conflict Handling
//!
//! If a predefined resource has the same URL and resource type as a
//! FSH-generated resource, the FSH resource takes precedence and the
//! predefined resource is ignored with a warning.
//!
//! **SUSHI Reference**: `src/ig/predefinedResources.ts`

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Name for the virtual package containing predefined resources
pub const PREDEFINED_PACKAGE_NAME: &str = "sushi-local";

/// Version for the virtual package
pub const PREDEFINED_PACKAGE_VERSION: &str = "LOCAL";

/// Standard resource directory names in an IG
const STANDARD_RESOURCE_DIRS: &[&str] = &[
    "capabilities",
    "extensions",
    "models",
    "operations",
    "profiles",
    "resources",
    "vocabulary",
    "examples",
];

/// A loaded predefined FHIR resource
#[derive(Debug, Clone)]
pub struct PredefinedResource {
    /// Resource as JSON
    pub json: JsonValue,

    /// Resource type (e.g., "StructureDefinition", "ValueSet")
    pub resource_type: String,

    /// Resource ID
    pub id: Option<String>,

    /// Canonical URL
    pub url: Option<String>,

    /// Source file path
    pub file_path: PathBuf,

    /// Filename for output
    pub filename: String,
}

impl PredefinedResource {
    /// Create from a JSON file
    pub fn from_file(path: &Path) -> Result<Self, PredefinedResourceError> {
        let content = fs::read_to_string(path)
            .map_err(|e| PredefinedResourceError::ReadFile(path.to_path_buf(), e))?;

        let json: JsonValue = serde_json::from_str(&content)
            .map_err(|e| PredefinedResourceError::ParseJson(path.to_path_buf(), e))?;

        // Extract resource metadata
        let resource_type = json
            .get("resourceType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PredefinedResourceError::MissingField(path.to_path_buf(), "resourceType")
            })?
            .to_string();

        let id = json.get("id").and_then(|v| v.as_str()).map(String::from);

        let url = json.get("url").and_then(|v| v.as_str()).map(String::from);

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| PredefinedResourceError::InvalidFilename(path.to_path_buf()))?
            .to_string();

        Ok(Self {
            json,
            resource_type,
            id,
            url,
            file_path: path.to_path_buf(),
            filename,
        })
    }

    /// Get a unique key for conflict detection
    pub fn conflict_key(&self) -> String {
        if let Some(ref url) = self.url {
            format!("{}|{}", self.resource_type, url)
        } else if let Some(ref id) = self.id {
            format!("{}|{}", self.resource_type, id)
        } else {
            format!("{}|{}", self.resource_type, self.filename)
        }
    }
}

/// Predefined resources loader
pub struct PredefinedResourcesLoader {
    /// Base input directory (usually "input")
    input_dir: PathBuf,

    /// Project root directory
    project_dir: PathBuf,

    /// Custom resource paths from path-resource parameters
    custom_paths: Vec<String>,
}

impl PredefinedResourcesLoader {
    /// Create a new loader
    ///
    /// # Arguments
    ///
    /// * `input_dir` - Base input directory (e.g., "./input")
    /// * `project_dir` - Project root directory
    /// * `custom_paths` - Additional paths from path-resource parameters
    pub fn new(
        input_dir: impl Into<PathBuf>,
        project_dir: impl Into<PathBuf>,
        custom_paths: Vec<String>,
    ) -> Self {
        Self {
            input_dir: input_dir.into(),
            project_dir: project_dir.into(),
            custom_paths,
        }
    }

    /// Get all resource directory paths to search
    ///
    /// Returns paths to standard IG directories plus any custom paths
    /// configured via path-resource parameters.
    pub fn get_resource_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Add standard resource directories
        for dir_name in STANDARD_RESOURCE_DIRS {
            let path = self.input_dir.join(dir_name);
            if path.exists() && path.is_dir() {
                paths.push(path);
            }
        }

        // Add custom paths from path-resource parameters
        for custom_path in &self.custom_paths {
            let is_recursive = custom_path.ends_with("/*");
            let clean_path = custom_path.trim_end_matches("/*");

            let full_path = self.project_dir.join(clean_path);

            if full_path.exists() && full_path.is_dir() {
                paths.push(full_path.clone());

                // If path ends with /*, recursively add subdirectories
                if is_recursive {
                    self.add_recursive_subdirs(&full_path, &mut paths);
                }
            }
        }

        paths
    }

    /// Recursively add subdirectories
    fn add_recursive_subdirs(&self, base_path: &Path, paths: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata()
                    && metadata.is_dir()
                {
                    let dir_path = entry.path();
                    paths.push(dir_path.clone());
                    self.add_recursive_subdirs(&dir_path, paths);
                }
            }
        }
    }

    /// Load all predefined resources from discovered paths
    ///
    /// Searches all resource directories for JSON files and loads them
    /// as FHIR resources. Non-JSON files and invalid resources are skipped
    /// with warnings.
    pub fn load_all(&self) -> Result<Vec<PredefinedResource>, PredefinedResourceError> {
        let mut resources = Vec::new();
        let resource_paths = self.get_resource_paths();

        for dir_path in resource_paths {
            // Use WalkDir to recursively find all JSON files
            for entry in WalkDir::new(&dir_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                // Only process .json files
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    match PredefinedResource::from_file(path) {
                        Ok(resource) => {
                            resources.push(resource);
                        }
                        Err(e) => {
                            // Log warning but continue processing
                            eprintln!(
                                "Warning: Failed to load predefined resource from {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }

        Ok(resources)
    }

    /// Load resources and check for conflicts with generated resources
    ///
    /// # Arguments
    ///
    /// * `generated_resources` - Map of conflict keys to generated resource info
    ///
    /// # Returns
    ///
    /// Tuple of (non-conflicting resources, conflicting resources)
    pub fn load_with_conflict_check(
        &self,
        generated_resources: &HashMap<String, GeneratedResourceInfo>,
    ) -> Result<(Vec<PredefinedResource>, Vec<ConflictInfo>), PredefinedResourceError> {
        let all_resources = self.load_all()?;
        let mut non_conflicting = Vec::new();
        let mut conflicts = Vec::new();

        for resource in all_resources {
            let key = resource.conflict_key();

            if let Some(generated_info) = generated_resources.get(&key) {
                // Conflict detected
                conflicts.push(ConflictInfo {
                    resource,
                    generated_resource: generated_info.clone(),
                });
            } else {
                // No conflict
                non_conflicting.push(resource);
            }
        }

        Ok((non_conflicting, conflicts))
    }
}

/// Information about a generated resource for conflict detection
#[derive(Debug, Clone)]
pub struct GeneratedResourceInfo {
    /// Resource type
    pub resource_type: String,

    /// Resource URL (if applicable)
    pub url: Option<String>,

    /// Resource ID
    pub id: Option<String>,

    /// Output filename
    pub filename: String,
}

impl GeneratedResourceInfo {
    /// Create conflict key
    pub fn conflict_key(&self) -> String {
        if let Some(ref url) = self.url {
            format!("{}|{}", self.resource_type, url)
        } else if let Some(ref id) = self.id {
            format!("{}|{}", self.resource_type, id)
        } else {
            format!("{}|{}", self.resource_type, self.filename)
        }
    }
}

/// Information about a resource conflict
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// The predefined resource that conflicts
    pub resource: PredefinedResource,

    /// The generated resource it conflicts with
    pub generated_resource: GeneratedResourceInfo,
}

impl ConflictInfo {
    /// Get a human-readable conflict description
    pub fn description(&self) -> String {
        format!(
            "Predefined resource {} ({}) conflicts with FSH-generated resource {}",
            self.resource.filename,
            self.resource
                .url
                .as_ref()
                .or(self.resource.id.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown"),
            self.generated_resource.filename
        )
    }
}

/// Errors that can occur during predefined resource operations
#[derive(Debug, Error)]
pub enum PredefinedResourceError {
    #[error("Failed to read file {0}: {1}")]
    ReadFile(PathBuf, std::io::Error),

    #[error("Failed to parse JSON in {0}: {1}")]
    ParseJson(PathBuf, serde_json::Error),

    #[error("Missing required field '{1}' in {0}")]
    MissingField(PathBuf, &'static str),

    #[error("Invalid filename for {0}")]
    InvalidFilename(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_resource(dir: &Path, filename: &str, resource_type: &str, url: &str) {
        let resource = serde_json::json!({
            "resourceType": resource_type,
            "id": filename.trim_end_matches(".json"),
            "url": url,
            "status": "active"
        });

        fs::write(
            dir.join(filename),
            serde_json::to_string_pretty(&resource).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_get_resource_paths_standard_dirs() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");

        // Create some standard directories
        fs::create_dir_all(input_dir.join("profiles")).unwrap();
        fs::create_dir_all(input_dir.join("examples")).unwrap();
        fs::create_dir_all(input_dir.join("vocabulary")).unwrap();

        let loader = PredefinedResourcesLoader::new(&input_dir, temp.path(), vec![]);
        let paths = loader.get_resource_paths();

        assert_eq!(paths.len(), 3);
        assert!(paths.iter().any(|p| p.ends_with("profiles")));
        assert!(paths.iter().any(|p| p.ends_with("examples")));
        assert!(paths.iter().any(|p| p.ends_with("vocabulary")));
    }

    #[test]
    fn test_get_resource_paths_custom_paths() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");
        let custom_dir = temp.path().join("custom");

        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&custom_dir).unwrap();

        let loader =
            PredefinedResourcesLoader::new(&input_dir, temp.path(), vec!["custom".to_string()]);
        let paths = loader.get_resource_paths();

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("custom"));
    }

    #[test]
    fn test_get_resource_paths_recursive() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");
        let custom_dir = temp.path().join("custom");
        let subdir1 = custom_dir.join("sub1");
        let subdir2 = custom_dir.join("sub2");

        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&subdir1).unwrap();
        fs::create_dir_all(&subdir2).unwrap();

        let loader =
            PredefinedResourcesLoader::new(&input_dir, temp.path(), vec!["custom/*".to_string()]);
        let paths = loader.get_resource_paths();

        assert_eq!(paths.len(), 3); // custom + sub1 + sub2
        assert!(paths.iter().any(|p| p.ends_with("custom")));
        assert!(paths.iter().any(|p| p.ends_with("sub1")));
        assert!(paths.iter().any(|p| p.ends_with("sub2")));
    }

    #[test]
    fn test_load_predefined_resource() {
        let temp = TempDir::new().unwrap();

        create_test_resource(
            temp.path(),
            "Patient.json",
            "StructureDefinition",
            "http://example.org/fhir/StructureDefinition/Patient",
        );

        let resource = PredefinedResource::from_file(&temp.path().join("Patient.json")).unwrap();

        assert_eq!(resource.resource_type, "StructureDefinition");
        assert_eq!(resource.id, Some("Patient".to_string()));
        assert_eq!(
            resource.url,
            Some("http://example.org/fhir/StructureDefinition/Patient".to_string())
        );
        assert_eq!(resource.filename, "Patient.json");
    }

    #[test]
    fn test_load_all_resources() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");
        let profiles_dir = input_dir.join("profiles");
        let examples_dir = input_dir.join("examples");

        fs::create_dir_all(&profiles_dir).unwrap();
        fs::create_dir_all(&examples_dir).unwrap();

        create_test_resource(
            &profiles_dir,
            "Patient.json",
            "StructureDefinition",
            "http://example.org/fhir/StructureDefinition/Patient",
        );
        create_test_resource(
            &examples_dir,
            "patient-1.json",
            "Patient",
            "http://example.org/fhir/Patient/patient-1",
        );

        let loader = PredefinedResourcesLoader::new(&input_dir, temp.path(), vec![]);
        let resources = loader.load_all().unwrap();

        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_conflict_detection() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");
        let profiles_dir = input_dir.join("profiles");

        fs::create_dir_all(&profiles_dir).unwrap();

        create_test_resource(
            &profiles_dir,
            "Patient.json",
            "StructureDefinition",
            "http://example.org/fhir/StructureDefinition/Patient",
        );

        let loader = PredefinedResourcesLoader::new(&input_dir, temp.path(), vec![]);

        // Create a generated resource with the same URL
        let mut generated = HashMap::new();
        generated.insert(
            "StructureDefinition|http://example.org/fhir/StructureDefinition/Patient".to_string(),
            GeneratedResourceInfo {
                resource_type: "StructureDefinition".to_string(),
                url: Some("http://example.org/fhir/StructureDefinition/Patient".to_string()),
                id: Some("Patient".to_string()),
                filename: "StructureDefinition-Patient.json".to_string(),
            },
        );

        let (non_conflicting, conflicts) = loader.load_with_conflict_check(&generated).unwrap();

        assert_eq!(non_conflicting.len(), 0);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].description().contains("conflicts"));
    }

    #[test]
    fn test_no_conflicts() {
        let temp = TempDir::new().unwrap();
        let input_dir = temp.path().join("input");
        let profiles_dir = input_dir.join("profiles");

        fs::create_dir_all(&profiles_dir).unwrap();

        create_test_resource(
            &profiles_dir,
            "Patient.json",
            "StructureDefinition",
            "http://example.org/fhir/StructureDefinition/Patient",
        );

        let loader = PredefinedResourcesLoader::new(&input_dir, temp.path(), vec![]);

        // No generated resources
        let generated = HashMap::new();

        let (non_conflicting, conflicts) = loader.load_with_conflict_check(&generated).unwrap();

        assert_eq!(non_conflicting.len(), 1);
        assert_eq!(conflicts.len(), 0);
    }
}
