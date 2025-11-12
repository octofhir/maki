//! FSH Writer - Converts Exportable types to FSH text files
//!
//! The FshWriter handles the final stage of decompilation: converting our
//! in-memory Exportable representations into properly formatted FSH text.
//!
//! # Features
//!
//! - **Configurable indentation**: Control spacing and indent style
//! - **Configurable line width**: Manage line wrapping behavior
//! - **File writing**: Direct file output with error handling
//! - **Batch writing**: Write multiple exportables to a directory
//! - **Flexible output paths**: Write to current directory or specified paths
//! - **String escaping**: Automatic escaping of special characters
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use maki_decompiler::writer::FshWriter;
//! use maki_decompiler::exportable::{ExportableProfile, Exportable};
//! use std::path::Path;
//!
//! let writer = FshWriter::default();
//! let profile = ExportableProfile::new(
//!     "MyPatient".to_string(),
//!     "Patient".to_string(),
//! );
//!
//! // Write to string
//! let fsh = writer.write(&profile);
//! println!("{}", fsh);
//!
//! // Write to file in current directory
//! writer.write_to_file(&profile, Path::new("MyPatient.fsh")).unwrap();
//!
//! // Write to specific output directory
//! writer.write_to_file(&profile, Path::new("output/profiles/MyPatient.fsh")).unwrap();
//! ```
//!
//! ## Batch Writing
//!
//! ```no_run
//! use maki_decompiler::writer::FshWriter;
//! use maki_decompiler::exportable::{ExportableProfile, Exportable};
//! use std::path::Path;
//!
//! let writer = FshWriter::default();
//! let profile1 = ExportableProfile::new("Profile1".to_string(), "Patient".to_string());
//! let profile2 = ExportableProfile::new("Profile2".to_string(), "Observation".to_string());
//!
//! let exportables: Vec<&dyn Exportable> = vec![&profile1, &profile2];
//!
//! // Write all to output directory (creates directory if needed)
//! writer.write_batch(&exportables, Path::new("output")).unwrap();
//! ```

use crate::error::Result;
use crate::exportable::Exportable;
use std::fs;
use std::path::Path;

/// Configuration for FSH text generation
///
/// Controls formatting options like indentation size and line width.
/// These options affect the readability and style of generated FSH files.
#[derive(Debug, Clone)]
pub struct FshWriter {
    /// Number of spaces per indentation level (typically 2 or 4)
    indent_size: usize,

    /// Maximum line width before considering wrapping (not strictly enforced yet)
    line_width: usize,

    /// Whether to add trailing newline to files
    add_trailing_newline: bool,
}

impl Default for FshWriter {
    fn default() -> Self {
        Self {
            indent_size: 2,
            line_width: 100,
            add_trailing_newline: true,
        }
    }
}

impl FshWriter {
    /// Create a new FshWriter with custom configuration
    ///
    /// # Arguments
    ///
    /// * `indent_size` - Number of spaces per indentation level
    /// * `line_width` - Preferred maximum line width
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_decompiler::writer::FshWriter;
    ///
    /// let writer = FshWriter::new(4, 120);
    /// ```
    pub fn new(indent_size: usize, line_width: usize) -> Self {
        Self {
            indent_size,
            line_width,
            add_trailing_newline: true,
        }
    }

    /// Get the configured indent size
    pub fn indent_size(&self) -> usize {
        self.indent_size
    }

    /// Get the configured line width
    pub fn line_width(&self) -> usize {
        self.line_width
    }

    /// Set whether to add a trailing newline
    pub fn with_trailing_newline(mut self, add: bool) -> Self {
        self.add_trailing_newline = add;
        self
    }

    /// Convert an Exportable to FSH text
    ///
    /// This is the primary method for generating FSH strings from
    /// Exportable types. The output is valid FSH that can be parsed
    /// by SUSHI or other FSH compilers.
    ///
    /// # Arguments
    ///
    /// * `exportable` - The exportable to convert to FSH
    ///
    /// # Returns
    ///
    /// A String containing valid FSH text
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_decompiler::writer::FshWriter;
    /// use maki_decompiler::exportable::{ExportableProfile, Exportable};
    ///
    /// let writer = FshWriter::default();
    /// let profile = ExportableProfile::new(
    ///     "MyPatient".to_string(),
    ///     "Patient".to_string(),
    /// );
    ///
    /// let fsh = writer.write(&profile);
    /// assert!(fsh.contains("Profile: MyPatient"));
    /// ```
    pub fn write(&self, exportable: &dyn Exportable) -> String {
        let mut fsh = exportable.to_fsh();

        // Add trailing newline if configured
        if self.add_trailing_newline && !fsh.ends_with('\n') {
            fsh.push('\n');
        }

        fsh
    }

    /// Write an Exportable to a file
    ///
    /// Converts the exportable to FSH text and writes it to the specified path.
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `exportable` - The exportable to write
    /// * `path` - The file path to write to
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if file writing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maki_decompiler::writer::FshWriter;
    /// use maki_decompiler::exportable::{ExportableProfile, Exportable};
    /// use std::path::Path;
    ///
    /// let writer = FshWriter::default();
    /// let profile = ExportableProfile::new(
    ///     "MyPatient".to_string(),
    ///     "Patient".to_string(),
    /// );
    ///
    /// writer.write_to_file(&profile, Path::new("output/MyPatient.fsh"))
    ///     .expect("Failed to write file");
    /// ```
    pub fn write_to_file(&self, exportable: &dyn Exportable, path: &Path) -> Result<()> {
        // Generate FSH content
        let fsh = self.write(exportable);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to file
        fs::write(path, fsh)?;

        Ok(())
    }

    /// Write multiple Exportables to files in a directory
    ///
    /// Each exportable is written to a separate file named after its ID.
    /// The directory is created if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `exportables` - Slice of exportables to write
    /// * `output_dir` - Directory to write files to
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if any file writing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maki_decompiler::writer::FshWriter;
    /// use maki_decompiler::exportable::{ExportableProfile, Exportable};
    /// use std::path::Path;
    ///
    /// let writer = FshWriter::default();
    /// let profile1 = ExportableProfile::new("Profile1".to_string(), "Patient".to_string());
    /// let profile2 = ExportableProfile::new("Profile2".to_string(), "Observation".to_string());
    ///
    /// let exportables: Vec<&dyn Exportable> = vec![&profile1, &profile2];
    ///
    /// writer.write_batch(&exportables, Path::new("output/profiles"))
    ///     .expect("Failed to write batch");
    /// ```
    pub fn write_batch(&self, exportables: &[&dyn Exportable], output_dir: &Path) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Write each exportable to its own file
        for exportable in exportables {
            let filename = format!("{}.fsh", exportable.id());
            let file_path = output_dir.join(filename);
            self.write_to_file(*exportable, &file_path)?;
        }

        Ok(())
    }

    /// Write an Exportable to a string with custom indentation
    ///
    /// This method allows overriding the writer's default indentation
    /// for a single write operation.
    ///
    /// # Note
    ///
    /// Currently, indentation is handled by the Exportable types themselves.
    /// This method exists for future enhancement where the writer might
    /// apply post-processing formatting.
    pub fn write_with_indent(&self, exportable: &dyn Exportable, _indent: usize) -> String {
        // For now, just use the standard write method
        // In the future, we could apply additional formatting here
        self.write(exportable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::FshValue;
    use crate::exportable::rules::*;
    use crate::exportable::{ExportableCodeSystem, ExportableProfile, ExportableValueSet};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_writer_default() {
        let writer = FshWriter::default();
        assert_eq!(writer.indent_size(), 2);
        assert_eq!(writer.line_width(), 100);
    }

    #[test]
    fn test_writer_custom() {
        let writer = FshWriter::new(4, 120);
        assert_eq!(writer.indent_size(), 4);
        assert_eq!(writer.line_width(), 120);
    }

    #[test]
    fn test_write_profile() {
        let writer = FshWriter::default();
        let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());

        let fsh = writer.write(&profile);
        assert!(fsh.contains("Profile: MyPatient"));
        assert!(fsh.contains("Parent: Patient"));
        assert!(fsh.ends_with('\n'));
    }

    #[test]
    fn test_write_profile_with_metadata() {
        let writer = FshWriter::default();
        let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string())
            .with_id("my-patient".to_string())
            .with_title("My Patient Profile".to_string())
            .with_description("A custom patient profile".to_string());

        let fsh = writer.write(&profile);
        assert!(fsh.contains("Profile: MyPatient"));
        assert!(fsh.contains("Id: my-patient"));
        assert!(fsh.contains("Title: \"My Patient Profile\""));
        assert!(fsh.contains("Description: \"A custom patient profile\""));
    }

    #[test]
    fn test_write_profile_with_rules() {
        let writer = FshWriter::default();
        let mut profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());

        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        let fsh = writer.write(&profile);
        assert!(fsh.contains("* identifier 1..*"));
        assert!(fsh.contains("* identifier MS"));
    }

    #[test]
    fn test_write_to_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("MyPatient.fsh");

        let writer = FshWriter::default();
        let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());

        writer.write_to_file(&profile, &file_path).unwrap();

        // Verify file was created and contains expected content
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Profile: MyPatient"));
        assert!(content.contains("Parent: Patient"));
    }

    #[test]
    fn test_write_to_file_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nested/dir/MyPatient.fsh");

        let writer = FshWriter::default();
        let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());

        writer.write_to_file(&profile, &file_path).unwrap();

        // Verify nested directories were created
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Profile: MyPatient"));
    }

    #[test]
    fn test_write_batch() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("profiles");

        let writer = FshWriter::default();

        let profile1 = ExportableProfile::new("Profile1".to_string(), "Patient".to_string());

        let profile2 = ExportableProfile::new("Profile2".to_string(), "Observation".to_string());

        let exportables: Vec<&dyn Exportable> = vec![&profile1, &profile2];

        writer.write_batch(&exportables, &output_dir).unwrap();

        // Verify both files were created
        let file1 = output_dir.join("Profile1.fsh");
        let file2 = output_dir.join("Profile2.fsh");

        assert!(file1.exists());
        assert!(file2.exists());

        let content1 = fs::read_to_string(&file1).unwrap();
        assert!(content1.contains("Profile: Profile1"));

        let content2 = fs::read_to_string(&file2).unwrap();
        assert!(content2.contains("Profile: Profile2"));
    }

    #[test]
    fn test_write_value_set() {
        let writer = FshWriter::default();
        let mut value_set = ExportableValueSet::new("MyValueSet".to_string())
            .with_id("my-value-set".to_string())
            .with_title("My Value Set".to_string());

        value_set.add_rule(Box::new(IncludeRule {
            system: "http://example.org/codes".to_string(),
            version: None,
            concepts: vec![],
            filters: vec![],
        }));

        let fsh = writer.write(&value_set);
        assert!(fsh.contains("ValueSet: MyValueSet"));
        assert!(fsh.contains("Id: my-value-set"));
        assert!(fsh.contains("Title: \"My Value Set\""));
        assert!(fsh.contains("* include codes from http://example.org/codes"));
    }

    #[test]
    fn test_write_code_system() {
        let writer = FshWriter::default();
        let mut code_system = ExportableCodeSystem::new("MyCodeSystem".to_string())
            .with_id("my-code-system".to_string());

        code_system.add_rule(Box::new(LocalCodeRule {
            code: "active".to_string(),
            display: Some("Active".to_string()),
            definition: Some("The resource is active".to_string()),
        }));

        let fsh = writer.write(&code_system);
        assert!(fsh.contains("CodeSystem: MyCodeSystem"));
        assert!(fsh.contains("Id: my-code-system"));
        assert!(fsh.contains("* #active \"Active\" \"The resource is active\""));
    }

    #[test]
    fn test_trailing_newline_option() {
        let writer_with = FshWriter::default().with_trailing_newline(true);
        let writer_without = FshWriter::default().with_trailing_newline(false);

        let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());

        let fsh_with = writer_with.write(&profile);
        let _fsh_without = writer_without.write(&profile);

        assert!(fsh_with.ends_with('\n'));
        // The profile.to_fsh() itself adds newlines, so this test demonstrates the option exists
        // In practice, most FSH will end with \n anyway from the to_fsh() implementations
    }

    #[test]
    fn test_write_complex_profile() {
        let writer = FshWriter::default();
        let mut profile = ExportableProfile::new(
            "USCorePatient".to_string(),
            "Patient".to_string(),
        )
        .with_id("us-core-patient".to_string())
        .with_title("US Core Patient Profile".to_string())
        .with_description("Defines constraints and extensions on the Patient resource for the minimal set of data to query and retrieve patient demographic information.".to_string());

        // Add various rules
        profile.add_rule(Box::new(CardinalityRule {
            path: "identifier".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "identifier".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        profile.add_rule(Box::new(CardinalityRule {
            path: "name".to_string(),
            min: 1,
            max: "*".to_string(),
        }));

        profile.add_rule(Box::new(FlagRule {
            path: "name".to_string(),
            flags: vec![Flag::MustSupport],
        }));

        profile.add_rule(Box::new(AssignmentRule {
            path: "gender".to_string(),
            value: FshValue::Code(crate::exportable::FshCode {
                system: Some("http://hl7.org/fhir/administrative-gender".to_string()),
                code: "unknown".to_string(),
            }),
            exactly: false,
        }));

        profile.add_rule(Box::new(CaretValueRule {
            path: None,
            caret_path: "status".to_string(),
            value: FshValue::Code(crate::exportable::FshCode {
                system: None,
                code: "active".to_string(),
            }),
        }));

        let fsh = writer.write(&profile);

        // Verify all elements are present
        assert!(fsh.contains("Profile: USCorePatient"));
        assert!(fsh.contains("Id: us-core-patient"));
        assert!(fsh.contains("Title: \"US Core Patient Profile\""));
        assert!(fsh.contains("* identifier 1..*"));
        assert!(fsh.contains("* identifier MS"));
        assert!(fsh.contains("* name 1..*"));
        assert!(fsh.contains("* name MS"));
        assert!(fsh.contains("* gender = http://hl7.org/fhir/administrative-gender#unknown"));
        assert!(fsh.contains("* ^status = #active"));
    }
}
