//! File loading and parsing for FHIR resources
//!
//! This module handles discovery and loading of FHIR resources from JSON and XML files.
//! It supports single files, directories (recursive), and FHIR Bundles.

use crate::error::{Error, Result};
use crate::lake::ResourceLake;
use crate::models::*;
use futures::stream::{self, StreamExt};
use std::fs;
use std::path::{Path, PathBuf};

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
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str())
                    && (name == "package.json" || name.starts_with('.'))
                {
                    log::debug!("Skipping {}", name);
                    continue;
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
        let resource: FhirResource =
            serde_json::from_str(&content).map_err(|e| Error::ParseError {
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
        let resource: FhirResource =
            quick_xml::de::from_str(&content).map_err(|e| Error::ParseError {
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

    /// Load FHIR resources from path (file or directory) concurrently into ResourceLake
    ///
    /// This async version uses concurrent I/O for improved performance with multiple files.
    ///
    /// # Arguments
    ///
    /// * `path` - File or directory path to load from
    /// * `lake` - ResourceLake to add resources to
    /// * `concurrency` - Maximum concurrent file loads (default: 50)
    ///
    /// # Returns
    ///
    /// LoadStats with counts of loaded files and errors
    pub async fn load_into_lake_concurrent(
        &mut self,
        path: &Path,
        lake: &mut ResourceLake,
        concurrency: usize,
    ) -> Result<LoadStats> {
        if path.is_file() {
            self.load_file_async(path, lake).await?;
        } else if path.is_dir() {
            self.load_directory_concurrent(path, lake, concurrency)
                .await?;
        } else {
            return Err(Error::InvalidPath(path.to_path_buf()));
        }

        Ok(LoadStats {
            loaded: self.loaded_count,
            errors: self.error_count,
            error_details: self.errors.clone(),
        })
    }

    /// Load all files from directory concurrently
    async fn load_directory_concurrent(
        &mut self,
        path: &Path,
        lake: &mut ResourceLake,
        concurrency: usize,
    ) -> Result<()> {
        // Collect all file paths first (synchronously)
        let mut file_paths = Vec::new();
        Self::collect_file_paths(path, &mut file_paths)?;

        // Load files concurrently using futures::stream
        let results: Vec<_> = stream::iter(file_paths)
            .map(|file_path| async move {
                let content = tokio::fs::read_to_string(&file_path).await?;
                let ext = file_path.extension().and_then(|s| s.to_str());

                let resource: FhirResource = match ext {
                    Some("json") => {
                        serde_json::from_str(&content).map_err(|e| Error::ParseError {
                            file: file_path.clone(),
                            message: e.to_string(),
                        })?
                    }
                    Some("xml") => {
                        quick_xml::de::from_str(&content).map_err(|e| Error::ParseError {
                            file: file_path.clone(),
                            message: e.to_string(),
                        })?
                    }
                    _ => return Ok((None, None)), // Skip unsupported files
                };

                Ok::<_, Error>((Some(resource), Some(file_path)))
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        // Process results and add to lake
        for result in results {
            match result {
                Ok((Some(resource), Some(file_path))) => {
                    if let Err(e) = self.add_resource_to_lake(resource, lake) {
                        log::warn!("Failed to add resource from {}: {}", file_path.display(), e);
                        self.error_count += 1;
                        self.errors.push(LoadError {
                            file_path,
                            error_message: e.to_string(),
                        });
                    } else {
                        self.loaded_count += 1;
                    }
                }
                Ok(_) => {
                    // Skipped file (unsupported type)
                }
                Err(e) => {
                    log::warn!("Failed to load file: {}", e);
                    self.error_count += 1;
                    if let Error::ParseError { file, message } = e {
                        self.errors.push(LoadError {
                            file_path: file,
                            error_message: message,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Collect file paths recursively (helper for concurrent loading)
    fn collect_file_paths(path: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_file() {
                // Skip package.json and other non-FHIR files
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str())
                    && (name == "package.json" || name.starts_with('.'))
                {
                    continue;
                }

                // Only include JSON and XML files
                if let Some(ext) = entry_path.extension().and_then(|s| s.to_str())
                    && (ext == "json" || ext == "xml")
                {
                    paths.push(entry_path);
                }
            } else if entry_path.is_dir() {
                // Recursive
                Self::collect_file_paths(&entry_path, paths)?;
            }
        }

        Ok(())
    }

    /// Load a single file asynchronously
    async fn load_file_async(&mut self, path: &Path, lake: &mut ResourceLake) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let ext = path.extension().and_then(|s| s.to_str());

        let resource: FhirResource = match ext {
            Some("json") => {
                log::debug!("Loading JSON file: {}", path.display());
                serde_json::from_str(&content).map_err(|e| Error::ParseError {
                    file: path.to_path_buf(),
                    message: e.to_string(),
                })?
            }
            Some("xml") => {
                log::debug!("Loading XML file: {}", path.display());
                quick_xml::de::from_str(&content).map_err(|e| Error::ParseError {
                    file: path.to_path_buf(),
                    message: e.to_string(),
                })?
            }
            _ => {
                log::warn!("Skipping unsupported file type: {}", path.display());
                return Ok(());
            }
        };

        self.add_resource_to_lake(resource, lake)?;
        self.loaded_count += 1;

        Ok(())
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
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, DefinitionSession};
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::TempDir;

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
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
    async fn test_load_json_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test-profile.json");

        // Write test JSON
        let mut file = File::create(&file_path).unwrap();
        file.write_all(create_test_structure_definition_json().as_bytes())
            .unwrap();

        // Create loader and lake
        let session = Arc::new(create_test_session().await);
        let mut lake = ResourceLake::new(session);
        let mut loader = FileLoader::new();

        // Load file
        let stats = loader.load_into_lake(&file_path, &mut lake).unwrap();

        assert_eq!(stats.loaded, 1);
        assert_eq!(stats.errors, 0);

        // Verify resource was added
        let sd =
            lake.get_structure_definition("http://example.org/StructureDefinition/TestProfile");
        assert!(sd.is_some());
        assert_eq!(sd.unwrap().name, "TestProfile");
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
    async fn test_load_directory() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple JSON files
        for i in 1..=3 {
            let file_path = temp_dir.path().join(format!("profile{}.json", i));
            let json = format!(
                r#"{{
                "resourceType": "StructureDefinition",
                "url": "http://example.org/StructureDefinition/Profile{}",
                "name": "Profile{}",
                "status": "active"
            }}"#,
                i, i
            );

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
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
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
        file.write_all(create_test_structure_definition_json().as_bytes())
            .unwrap();

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
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
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
        // The error message should indicate a JSON/parse error
        let error_msg = &stats.error_details[0].error_message;
        assert!(
            error_msg.contains("Parse")
                || error_msg.contains("parse")
                || error_msg.contains("JSON")
                || error_msg.contains("json")
                || error_msg.contains("invalid"),
            "Expected parse/JSON error but got: {}",
            error_msg
        );
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
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
        assert!(
            lake.get_value_set("http://example.org/ValueSet/TestVS")
                .is_some()
        );
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
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
        assert!(
            lake.get_code_system("http://example.org/CodeSystem/TestCS")
                .is_some()
        );
    }
}
