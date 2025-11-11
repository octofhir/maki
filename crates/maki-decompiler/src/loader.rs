//! File loading and parsing for FHIR resources
//!
//! This module handles discovery and loading of FHIR resources from JSON and XML files.
//! It supports single files, directories (recursive), and FHIR Bundles.

use std::path::{Path, PathBuf};
use std::fs;
use crate::models::*;
use crate::lake::ResourceLake;
use crate::error::{Result, Error};

/// File loader for FHIR resources
///
/// Discovers and loads FHIR resources from JSON/XML files into a ResourceLake.
/// Handles errors gracefully and tracks loading statistics.
pub struct FileLoader {
    /// Files successfully loaded
    loaded_count: usize,

    /// Files with parse errors
    error_count: usize,

    /// Detailed error messages
    errors: Vec<LoadError>,
}

/// Details about a file loading error
#[derive(Debug, Clone)]
pub struct LoadError {
    pub file_path: PathBuf,
    pub error_message: String,
}

/// Statistics about a loading operation
#[derive(Debug, Clone)]
pub struct LoadStats {
    pub loaded: usize,
    pub errors: usize,
    pub error_details: Vec<LoadError>,
}

impl FileLoader {
    /// Create a new FileLoader
    pub fn new() -> Self {
        Self {
            loaded_count: 0,
            error_count: 0,
            errors: Vec::new(),
        }
    }

    /// Load FHIR resources from path (file or directory) into ResourceLake
    ///
    /// # Arguments
    ///
    /// * `path` - File or directory path to load from
    /// * `lake` - ResourceLake to add resources to
    ///
    /// # Returns
    ///
    /// LoadStats with counts of loaded files and errors
    pub fn load_into_lake(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<LoadStats> {
        if path.is_file() {
            self.load_file(path, lake)?;
        } else if path.is_dir() {
            self.load_directory(path, lake)?;
        } else {
            return Err(Error::InvalidPath(path.to_path_buf()));
        }

        Ok(LoadStats {
            loaded: self.loaded_count,
            errors: self.error_count,
            error_details: self.errors.clone(),
        })
    }

    /// Load a single file
    fn load_file(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<()> {
        let ext = path.extension().and_then(|s| s.to_str());

        match ext {
            Some("json") => self.load_json_file(path, lake),
            Some("xml") => self.load_xml_file(path, lake),
            _ => {
                log::warn!("Skipping unsupported file type: {}", path.display());
                Ok(())
            }
        }
    }

    /// Load all files from directory recursively
    fn load_directory(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                // Skip package.json and other non-FHIR files
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    if name == "package.json" || name.starts_with('.') {
                        log::debug!("Skipping {}", name);
                        continue;
                    }
                }

                // Load file, but continue on error
                if let Err(e) = self.load_file(&entry_path, lake) {
                    log::warn!("Failed to load {}: {}", entry_path.display(), e);
                    self.error_count += 1;
                    self.errors.push(LoadError {
                        file_path: entry_path.clone(),
                        error_message: e.to_string(),
                    });
                }
            } else if entry_path.is_dir() {
                // Recursive
                self.load_directory(&entry_path, lake)?;
            }
        }

        Ok(())
    }

    /// Load JSON file
    fn load_json_file(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<()> {
        log::debug!("Loading JSON file: {}", path.display());

        let content = fs::read_to_string(path)?;
        let resource: FhirResource = serde_json::from_str(&content)
            .map_err(|e| Error::ParseError {
                file: path.to_path_buf(),
                message: e.to_string(),
            })?;

        self.add_resource_to_lake(resource, lake)?;
        self.loaded_count += 1;

        Ok(())
    }

    /// Load XML file
    fn load_xml_file(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<()> {
        log::debug!("Loading XML file: {}", path.display());

        let content = fs::read_to_string(path)?;
        let resource: FhirResource = quick_xml::de::from_str(&content)
            .map_err(|e| Error::ParseError {
                file: path.to_path_buf(),
                message: e.to_string(),
            })?;

        self.add_resource_to_lake(resource, lake)?;
        self.loaded_count += 1;

        Ok(())
    }

    /// Add resource to lake (handles different resource types)
    fn add_resource_to_lake(&self, resource: FhirResource, lake: &mut ResourceLake) -> Result<()> {
        match resource {
            FhirResource::StructureDefinition(sd) => {
                log::info!("Loaded StructureDefinition: {}", sd.name);
                lake.add_structure_definition(sd)?;
            }
            FhirResource::ValueSet(vs) => {
                log::info!("Loaded ValueSet: {}", vs.name);
                lake.add_value_set(vs)?;
            }
            FhirResource::CodeSystem(cs) => {
                log::info!("Loaded CodeSystem: {}", cs.name);
                lake.add_code_system(cs)?;
            }
            FhirResource::Other => {
                log::debug!("Skipping non-definitional resource");
                // For now, skip other resource types
                // In the future, we could add instances to the lake
            }
        }

        Ok(())
    }

    /// Get current loading statistics
    pub fn stats(&self) -> LoadStats {
        LoadStats {
            loaded: self.loaded_count,
            errors: self.error_count,
            error_details: self.errors.clone(),
        }
    }
}

impl Default for FileLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, DefinitionSession};
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    // Helper to create a test session
    // Note: This requires async context and proper initialization
    // For now, tests will use a simplified approach
    async fn create_test_session() -> DefinitionSession {
        let options = CanonicalOptions {
            quick_init: true,
            ..Default::default()
        };
        let facade = Arc::new(CanonicalFacade::new(options).await.unwrap());
        facade.session(vec![]).await.unwrap()
    }

    fn create_test_structure_definition_json() -> &'static str {
        r#"{
            "resourceType": "StructureDefinition",
            "url": "http://example.org/StructureDefinition/TestProfile",
            "name": "TestProfile",
            "status": "active"
        }"#
    }

    #[tokio::test]
    async fn test_load_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-profile.json");

        // Write test JSON
        let mut file = File::create(&file_path).unwrap();
        file.write_all(create_test_structure_definition_json().as_bytes()).unwrap();

        // Create loader and lake
        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        // Load file
        let stats = loader.load_into_lake(&file_path, &mut lake).unwrap();

        assert_eq!(stats.loaded, 1);
        assert_eq!(stats.errors, 0);

        // Verify resource was added
        let sd = lake.get_structure_definition("http://example.org/StructureDefinition/TestProfile");
        assert!(sd.is_some());
        assert_eq!(sd.unwrap().name, "TestProfile");
    }

    #[tokio::test]
    async fn test_load_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple JSON files
        for i in 1..=3 {
            let file_path = temp_dir.path().join(format!("profile{}.json", i));
            let json = format!(r#"{{
                "resourceType": "StructureDefinition",
                "url": "http://example.org/StructureDefinition/Profile{}",
                "name": "Profile{}",
                "status": "active"
            }}"#, i, i);

            let mut file = File::create(&file_path).unwrap();
            file.write_all(json.as_bytes()).unwrap();
        }

        // Create loader and lake
        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        // Load directory
        let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

        assert_eq!(stats.loaded, 3);
        assert_eq!(stats.errors, 0);

        // Verify resources were added
        for i in 1..=3 {
            let url = format!("http://example.org/StructureDefinition/Profile{}", i);
            assert!(lake.get_structure_definition(&url).is_some());
        }
    }

    #[tokio::test]
    async fn test_skip_non_fhir_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create package.json (should be skipped)
        let package_json_path = temp_dir.path().join("package.json");
        let mut file = File::create(&package_json_path).unwrap();
        file.write_all(b"{\"name\": \"test-package\"}").unwrap();

        // Create hidden file (should be skipped)
        let hidden_path = temp_dir.path().join(".hidden.json");
        let mut file = File::create(&hidden_path).unwrap();
        file.write_all(b"{}").unwrap();

        // Create valid FHIR file
        let fhir_path = temp_dir.path().join("profile.json");
        let mut file = File::create(&fhir_path).unwrap();
        file.write_all(create_test_structure_definition_json().as_bytes()).unwrap();

        // Create loader and lake
        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        // Load directory
        let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

        // Should only load the valid FHIR file
        assert_eq!(stats.loaded, 1);
    }

    #[tokio::test]
    async fn test_handle_parse_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.json");

        // Write invalid JSON
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"{invalid json}").unwrap();

        // Create loader and lake
        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        // Load directory (should handle error gracefully)
        let stats = loader.load_into_lake(temp_dir.path(), &mut lake).unwrap();

        assert_eq!(stats.loaded, 0);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.error_details.len(), 1);
        assert!(stats.error_details[0].error_message.contains("Parse"));
    }

    #[tokio::test]
    async fn test_value_set_loading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("valueset.json");

        let json = r#"{
            "resourceType": "ValueSet",
            "url": "http://example.org/ValueSet/TestVS",
            "name": "TestVS",
            "status": "active"
        }"#;

        let mut file = File::create(&file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        let stats = loader.load_into_lake(&file_path, &mut lake).unwrap();

        assert_eq!(stats.loaded, 1);
        assert!(lake.get_value_set("http://example.org/ValueSet/TestVS").is_some());
    }

    #[tokio::test]
    async fn test_code_system_loading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("codesystem.json");

        let json = r#"{
            "resourceType": "CodeSystem",
            "url": "http://example.org/CodeSystem/TestCS",
            "name": "TestCS",
            "status": "active",
            "content": "complete"
        }"#;

        let mut file = File::create(&file_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        let stats = loader.load_into_lake(&file_path, &mut lake).unwrap();

        assert_eq!(stats.loaded, 1);
        assert!(lake.get_code_system("http://example.org/CodeSystem/TestCS").is_some());
    }
}
