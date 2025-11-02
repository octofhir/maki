//! File structure generator for FHIR IG Publisher
//!
//! This module handles creating the correct output directory structure
//! for the IG Publisher, following SUSHI conventions.
//!
//! ## Directory Structure
//!
//! ```text
//! fsh-generated/
//! ├── resources/           # All exported FHIR resources (JSON)
//! │   ├── StructureDefinition-*.json
//! │   ├── ValueSet-*.json
//! │   ├── CodeSystem-*.json
//! │   ├── ImplementationGuide-*.json
//! │   └── [other resource instances]
//! ├── includes/            # Generated include files
//! │   └── menu.xml
//! ├── data/               # Machine-readable indexes
//! │   └── fsh-index.json
//! └── fsh-index.txt       # Human-readable index
//! ```
//!
//! **Reference**: <https://fshschool.org/docs/sushi/project/>

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default output directory name (SUSHI convention)
pub const FSH_GENERATED_DIR: &str = "fsh-generated";

/// Resources subdirectory
pub const RESOURCES_DIR: &str = "resources";

/// Includes subdirectory (for menu.xml, etc.)
pub const INCLUDES_DIR: &str = "includes";

/// Data subdirectory (for machine-readable indexes)
pub const DATA_DIR: &str = "data";

/// File structure generator for IG output
///
/// Creates and manages the fsh-generated/ directory structure
/// following IG Publisher conventions.
pub struct FileStructureGenerator {
    /// Output directory (the fsh-generated directory path)
    output_dir: PathBuf,

    /// Whether to clear existing output before writing
    clean_output: bool,
}

impl FileStructureGenerator {
    /// Create a new file structure generator
    ///
    /// # Arguments
    ///
    /// * `output_dir` - The fsh-generated directory path (not the project root)
    /// * `clean_output` - Whether to clear existing fsh-generated/ directory
    pub fn new(output_dir: impl Into<PathBuf>, clean_output: bool) -> Self {
        Self {
            output_dir: output_dir.into(),
            clean_output,
        }
    }

    /// Get the fsh-generated directory path
    pub fn fsh_generated_dir(&self) -> PathBuf {
        // output_dir is already the fsh-generated directory (set by CLI)
        self.output_dir.clone()
    }

    /// Get the resources directory path
    pub fn resources_dir(&self) -> PathBuf {
        self.output_dir.join(RESOURCES_DIR)
    }

    /// Get the includes directory path
    pub fn includes_dir(&self) -> PathBuf {
        self.output_dir.join(INCLUDES_DIR)
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> PathBuf {
        self.output_dir.join(DATA_DIR)
    }

    /// Initialize the directory structure
    ///
    /// Creates all necessary directories. If `clean_output` is true,
    /// removes existing fsh-generated/ directory first.
    pub fn initialize(&self) -> Result<(), FileStructureError> {
        let fsh_gen = self.fsh_generated_dir();

        // Clean existing directory if requested
        if self.clean_output && fsh_gen.exists() {
            fs::remove_dir_all(&fsh_gen)
                .map_err(|e| FileStructureError::RemoveDirectory(fsh_gen.clone(), e))?;
        }

        // Create all required directories
        self.create_directory(&self.resources_dir())?;
        self.create_directory(&self.includes_dir())?;
        self.create_directory(&self.data_dir())?;

        Ok(())
    }

    /// Create a directory, including all parent directories
    fn create_directory(&self, path: &Path) -> Result<(), FileStructureError> {
        fs::create_dir_all(path)
            .map_err(|e| FileStructureError::CreateDirectory(path.to_path_buf(), e))
    }

    /// Write a FHIR resource to the resources directory
    ///
    /// # Arguments
    ///
    /// * `filename` - Resource filename (e.g., "StructureDefinition-patient.json")
    /// * `content` - Serializable resource content
    pub fn write_resource<T: Serialize>(
        &self,
        filename: &str,
        content: &T,
    ) -> Result<(), FileStructureError> {
        let path = self.resources_dir().join(filename);
        self.write_json(&path, content)
    }

    /// Write the FSH index file (human-readable)
    ///
    /// # Arguments
    ///
    /// * `index` - FSH index content
    pub fn write_fsh_index_txt(&self, index: &str) -> Result<(), FileStructureError> {
        let path = self.fsh_generated_dir().join("fsh-index.txt");
        fs::write(&path, index).map_err(|e| FileStructureError::WriteFile(path, e))
    }

    /// Write the FSH index file (machine-readable JSON)
    ///
    /// # Arguments
    ///
    /// * `index` - FSH index entries
    pub fn write_fsh_index_json(&self, index: &[FshIndexEntry]) -> Result<(), FileStructureError> {
        let path = self.data_dir().join("fsh-index.json");
        self.write_json(&path, index)
    }

    /// Write menu.xml to the includes directory
    ///
    /// # Arguments
    ///
    /// * `menu_xml` - XML content for the menu
    pub fn write_menu_xml(&self, menu_xml: &str) -> Result<(), FileStructureError> {
        let path = self.includes_dir().join("menu.xml");
        fs::write(&path, menu_xml).map_err(|e| FileStructureError::WriteFile(path, e))
    }

    /// Write package.json to the root output directory
    ///
    /// # Arguments
    ///
    /// * `package_json` - Package JSON structure from SUSHI config
    pub fn write_package_json(
        &self,
        package_json: &super::PackageJson,
    ) -> Result<(), FileStructureError> {
        let path = self.output_dir.join("package.json");
        self.write_json(&path, package_json)
    }

    /// Write JSON content to a file with pretty formatting
    fn write_json<T: Serialize + ?Sized>(
        &self,
        path: &Path,
        content: &T,
    ) -> Result<(), FileStructureError> {
        let json = serde_json::to_string_pretty(content)
            .map_err(|e| FileStructureError::SerializeJson(path.to_path_buf(), e))?;

        fs::write(path, json).map_err(|e| FileStructureError::WriteFile(path.to_path_buf(), e))
    }

    /// Get the relative path for a resource file
    ///
    /// Returns the path relative to the project root, in the format
    /// expected by IG Publisher (e.g., "fsh-generated/resources/StructureDefinition-patient.json")
    pub fn resource_relative_path(&self, filename: &str) -> String {
        format!("{}/{}/{}", FSH_GENERATED_DIR, RESOURCES_DIR, filename)
    }
}

/// FSH index entry for tracking FSH to FHIR mappings
///
/// This is written to fsh-generated/data/fsh-index.json for tooling
/// and fsh-generated/fsh-index.txt for human consumption.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FshIndexEntry {
    /// Output filename (e.g., "StructureDefinition-patient.json")
    pub output_file: String,

    /// FSH name (e.g., "PatientProfile")
    pub fsh_name: String,

    /// FSH type (e.g., "Profile", "Extension", "ValueSet")
    pub fsh_type: String,

    /// Source FSH file path
    pub fsh_file: String,

    /// Starting line number in FSH file
    pub start_line: usize,

    /// Ending line number in FSH file
    pub end_line: usize,
}

impl FshIndexEntry {
    /// Format as a table row for fsh-index.txt
    pub fn to_table_row(&self) -> Vec<String> {
        vec![
            self.output_file.clone(),
            self.fsh_name.clone(),
            self.fsh_type.clone(),
            self.fsh_file.clone(),
            format!("{} - {}", self.start_line, self.end_line),
        ]
    }
}

/// Generate a text table from index entries
pub fn format_fsh_index_table(entries: &[FshIndexEntry]) -> String {
    let mut rows = vec![vec![
        "Output File".to_string(),
        "Name".to_string(),
        "Type".to_string(),
        "FSH File".to_string(),
        "Lines".to_string(),
    ]];

    for entry in entries {
        rows.push(entry.to_table_row());
    }

    // Calculate column widths
    let col_widths: Vec<usize> = (0..5)
        .map(|col| rows.iter().map(|row| row[col].len()).max().unwrap_or(0))
        .collect();

    // Format rows
    let formatted_rows: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let formatted_row = row
                .iter()
                .enumerate()
                .map(|(j, cell)| format!("{:<width$}", cell, width = col_widths[j]))
                .collect::<Vec<_>>()
                .join("  ");

            // Add separator line after header
            if i == 0 {
                let separator = col_widths
                    .iter()
                    .map(|&w| "-".repeat(w))
                    .collect::<Vec<_>>()
                    .join("  ");
                format!("{}\n{}", formatted_row, separator)
            } else {
                formatted_row
            }
        })
        .collect();

    formatted_rows.join("\n")
}

/// Errors that can occur during file structure operations
#[derive(Debug, Error)]
pub enum FileStructureError {
    #[error("Failed to create directory {0}: {1}")]
    CreateDirectory(PathBuf, std::io::Error),

    #[error("Failed to remove directory {0}: {1}")]
    RemoveDirectory(PathBuf, std::io::Error),

    #[error("Failed to write file {0}: {1}")]
    WriteFile(PathBuf, std::io::Error),

    #[error("Failed to serialize JSON for {0}: {1}")]
    SerializeJson(PathBuf, serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_directory_paths() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);

        assert_eq!(generator.fsh_generated_dir(), fsh_gen_path);
        assert_eq!(generator.resources_dir(), fsh_gen_path.join("resources"));
        assert_eq!(generator.includes_dir(), fsh_gen_path.join("includes"));
        assert_eq!(generator.data_dir(), fsh_gen_path.join("data"));
    }

    #[test]
    fn test_initialize_creates_directories() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);

        generator.initialize().unwrap();

        assert!(generator.fsh_generated_dir().exists());
        assert!(generator.resources_dir().exists());
        assert!(generator.includes_dir().exists());
        assert!(generator.data_dir().exists());
    }

    #[test]
    fn test_initialize_with_clean() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, true);

        // Create directories and a test file
        generator.initialize().unwrap();
        let test_file = generator.resources_dir().join("test.json");
        fs::write(&test_file, "test").unwrap();
        assert!(test_file.exists());

        // Re-initialize with clean should remove the file
        generator.initialize().unwrap();
        assert!(!test_file.exists());
        assert!(generator.resources_dir().exists());
    }

    #[test]
    fn test_write_resource() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);
        generator.initialize().unwrap();

        #[derive(Serialize)]
        struct TestResource {
            id: String,
            name: String,
        }

        let resource = TestResource {
            id: "test".to_string(),
            name: "Test Resource".to_string(),
        };

        generator
            .write_resource("StructureDefinition-test.json", &resource)
            .unwrap();

        let path = generator
            .resources_dir()
            .join("StructureDefinition-test.json");
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("\"id\": \"test\""));
        assert!(content.contains("\"name\": \"Test Resource\""));
    }

    #[test]
    fn test_write_fsh_index_txt() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);
        generator.initialize().unwrap();

        let index =
            "Output File  Name  Type  FSH File  Lines\ntest.json    Test  Profile  test.fsh  1-10";
        generator.write_fsh_index_txt(index).unwrap();

        let path = generator.fsh_generated_dir().join("fsh-index.txt");
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, index);
    }

    #[test]
    fn test_write_fsh_index_json() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);
        generator.initialize().unwrap();

        let entries = vec![FshIndexEntry {
            output_file: "StructureDefinition-test.json".to_string(),
            fsh_name: "TestProfile".to_string(),
            fsh_type: "Profile".to_string(),
            fsh_file: "input/fsh/test.fsh".to_string(),
            start_line: 1,
            end_line: 10,
        }];

        generator.write_fsh_index_json(&entries).unwrap();

        let path = generator.data_dir().join("fsh-index.json");
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("\"outputFile\": \"StructureDefinition-test.json\""));
    }

    #[test]
    fn test_write_menu_xml() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);
        generator.initialize().unwrap();

        let menu_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<menu>
  <item name="Home" url="index.html"/>
</menu>"#;

        generator.write_menu_xml(menu_xml).unwrap();

        let path = generator.includes_dir().join("menu.xml");
        assert!(path.exists());

        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, menu_xml);
    }

    #[test]
    fn test_resource_relative_path() {
        let temp = TempDir::new().unwrap();
        let fsh_gen_path = temp.path().join("fsh-generated");
        let generator = FileStructureGenerator::new(&fsh_gen_path, false);

        let path = generator.resource_relative_path("StructureDefinition-test.json");
        assert_eq!(
            path,
            "fsh-generated/resources/StructureDefinition-test.json"
        );
    }

    #[test]
    fn test_fsh_index_entry_to_table_row() {
        let entry = FshIndexEntry {
            output_file: "SD-test.json".to_string(),
            fsh_name: "TestProfile".to_string(),
            fsh_type: "Profile".to_string(),
            fsh_file: "test.fsh".to_string(),
            start_line: 5,
            end_line: 15,
        };

        let row = entry.to_table_row();
        assert_eq!(row.len(), 5);
        assert_eq!(row[0], "SD-test.json");
        assert_eq!(row[4], "5 - 15");
    }

    #[test]
    fn test_format_fsh_index_table() {
        let entries = vec![
            FshIndexEntry {
                output_file: "SD-patient.json".to_string(),
                fsh_name: "Patient".to_string(),
                fsh_type: "Profile".to_string(),
                fsh_file: "patient.fsh".to_string(),
                start_line: 1,
                end_line: 20,
            },
            FshIndexEntry {
                output_file: "VS-condition-codes.json".to_string(),
                fsh_name: "ConditionCodes".to_string(),
                fsh_type: "ValueSet".to_string(),
                fsh_file: "valuesets.fsh".to_string(),
                start_line: 50,
                end_line: 75,
            },
        ];

        let table = format_fsh_index_table(&entries);

        // Check header is present
        assert!(table.contains("Output File"));
        assert!(table.contains("Name"));
        assert!(table.contains("Type"));

        // Check data rows
        assert!(table.contains("SD-patient.json"));
        assert!(table.contains("Patient"));
        assert!(table.contains("Profile"));
        assert!(table.contains("1 - 20"));

        assert!(table.contains("VS-condition-codes.json"));
        assert!(table.contains("ConditionCodes"));
        assert!(table.contains("ValueSet"));
        assert!(table.contains("50 - 75"));

        // Check separator line exists
        assert!(table.contains("---"));
    }
}
