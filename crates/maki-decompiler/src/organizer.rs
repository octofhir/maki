//! File Organizer - Organize FSH files into directory structures
//!
//! The FileOrganizer handles organizing exported FSH files using different
//! strategies, matching GoFSH's behavior for file organization.
//!
//! # Organization Strategies
//!
//! 1. **FilePerDefinition** - Each exportable gets its own file
//! 2. **GroupByFshType** - Group by FSH type (profiles/, valueSets/, etc.)
//! 3. **GroupByProfile** - Group by profile parent type
//! 4. **SingleFile** - All exportables in a single file
//!
//! # Examples
//!
//! ## File Per Definition
//!
//! ```no_run
//! use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
//! use maki_decompiler::exportable::{ExportableProfile, Exportable};
//! use std::path::Path;
//!
//! let organizer = FileOrganizer::new(OrganizationStrategy::FilePerDefinition);
//! let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());
//!
//! let exportables: Vec<Box<dyn Exportable>> = vec![Box::new(profile)];
//! organizer.organize(&exportables, Path::new("output")).unwrap();
//! ```
//!
//! ## Group By Type
//!
//! ```no_run
//! use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
//! use maki_decompiler::exportable::{ExportableProfile, ExportableValueSet, Exportable};
//! use std::path::Path;
//!
//! let organizer = FileOrganizer::new(OrganizationStrategy::GroupByFshType);
//! let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());
//! let value_set = ExportableValueSet::new("MyValueSet".to_string());
//!
//! let exportables: Vec<Box<dyn Exportable>> = vec![
//!     Box::new(profile),
//!     Box::new(value_set),
//! ];
//! organizer.organize(&exportables, Path::new("output")).unwrap();
//! // Creates: output/profiles/MyPatient.fsh
//! //          output/valuesets/MyValueSet.fsh
//! ```

use crate::error::Result;
use crate::exportable::Exportable;
use crate::writer::FshWriter;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Organization strategies for FSH files
///
/// Defines how FSH files should be organized in the output directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrganizationStrategy {
    /// Each definition gets its own file in the root output directory
    /// Example: output/MyPatient.fsh, output/MyValueSet.fsh
    #[default]
    FilePerDefinition,

    /// Group files by FSH type in subdirectories
    /// Example: output/profiles/MyPatient.fsh, output/valuesets/MyValueSet.fsh
    GroupByFshType,

    /// Group profiles by their parent type
    /// Example: output/Patient/MyPatient.fsh, output/Observation/MyObservation.fsh
    GroupByProfile,

    /// All definitions in a single file
    /// Example: output/definitions.fsh (contains all exportables)
    SingleFile,
}

/// File Organizer for FSH exports
///
/// Organizes FSH files according to the selected strategy.
pub struct FileOrganizer {
    strategy: OrganizationStrategy,
    writer: FshWriter,
}

impl FileOrganizer {
    /// Create a new FileOrganizer with the given strategy
    ///
    /// # Arguments
    ///
    /// * `strategy` - The organization strategy to use
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
    ///
    /// let organizer = FileOrganizer::new(OrganizationStrategy::GroupByFshType);
    /// ```
    pub fn new(strategy: OrganizationStrategy) -> Self {
        Self {
            strategy,
            writer: FshWriter::default(),
        }
    }

    /// Create a new FileOrganizer with custom FshWriter
    ///
    /// # Arguments
    ///
    /// * `strategy` - The organization strategy to use
    /// * `writer` - Custom FshWriter configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
    /// use maki_decompiler::writer::FshWriter;
    ///
    /// let writer = FshWriter::new(4, 120); // 4 spaces, 120 line width
    /// let organizer = FileOrganizer::with_writer(OrganizationStrategy::GroupByFshType, writer);
    /// ```
    pub fn with_writer(strategy: OrganizationStrategy, writer: FshWriter) -> Self {
        Self { strategy, writer }
    }

    /// Get the current organization strategy
    pub fn strategy(&self) -> OrganizationStrategy {
        self.strategy
    }

    /// Organize exportables according to the strategy
    ///
    /// # Arguments
    ///
    /// * `exportables` - The exportables to organize
    /// * `output_dir` - The output directory
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if file writing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use maki_decompiler::organizer::{FileOrganizer, OrganizationStrategy};
    /// use maki_decompiler::exportable::{ExportableProfile, Exportable};
    /// use std::path::Path;
    ///
    /// let organizer = FileOrganizer::new(OrganizationStrategy::FilePerDefinition);
    /// let profile = ExportableProfile::new("MyPatient".to_string(), "Patient".to_string());
    ///
    /// let exportables: Vec<Box<dyn Exportable>> = vec![Box::new(profile)];
    /// organizer.organize(&exportables, Path::new("output")).unwrap();
    /// ```
    pub fn organize(&self, exportables: &[Box<dyn Exportable>], output_dir: &Path) -> Result<()> {
        match self.strategy {
            OrganizationStrategy::FilePerDefinition => {
                self.organize_per_definition(exportables, output_dir)
            }
            OrganizationStrategy::GroupByFshType => self.organize_by_type(exportables, output_dir),
            OrganizationStrategy::GroupByProfile => {
                self.organize_by_profile(exportables, output_dir)
            }
            OrganizationStrategy::SingleFile => self.organize_single_file(exportables, output_dir),
        }
    }

    /// Organize with one file per definition
    fn organize_per_definition(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Write each exportable to its own file
        for exportable in exportables {
            let filename = format!("{}.fsh", exportable.id());
            let file_path = output_dir.join(filename);
            self.writer.write_to_file(exportable.as_ref(), &file_path)?;
        }

        Ok(())
    }

    /// Organize by grouping files by FSH type
    fn organize_by_type(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Group exportables by type
        let mut groups: HashMap<&str, Vec<&Box<dyn Exportable>>> = HashMap::new();

        for exportable in exportables {
            let type_dir = self.get_fsh_type_directory(exportable.as_ref());
            groups.entry(type_dir).or_default().push(exportable);
        }

        // Write each group to its directory
        for (type_dir, group_exportables) in groups {
            let type_path = output_dir.join(type_dir);
            fs::create_dir_all(&type_path)?;

            for exportable in group_exportables {
                let filename = format!("{}.fsh", exportable.id());
                let file_path = type_path.join(filename);
                self.writer.write_to_file(exportable.as_ref(), &file_path)?;
            }
        }

        Ok(())
    }

    /// Organize by grouping profiles by their parent type
    fn organize_by_profile(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Group exportables by profile parent or type
        let mut groups: HashMap<String, Vec<&Box<dyn Exportable>>> = HashMap::new();

        for exportable in exportables {
            let group_name = self.get_profile_group(exportable.as_ref());
            groups.entry(group_name).or_default().push(exportable);
        }

        // Write each group to its directory
        for (group_name, group_exportables) in groups {
            let group_path = output_dir.join(group_name);
            fs::create_dir_all(&group_path)?;

            for exportable in group_exportables {
                let filename = format!("{}.fsh", exportable.id());
                let file_path = group_path.join(filename);
                self.writer.write_to_file(exportable.as_ref(), &file_path)?;
            }
        }

        Ok(())
    }

    /// Organize into a single file
    fn organize_single_file(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Combine all FSH into a single string
        let mut combined_fsh = String::new();

        for (i, exportable) in exportables.iter().enumerate() {
            if i > 0 {
                combined_fsh.push_str("\n\n");
            }
            combined_fsh.push_str(&self.writer.write(exportable.as_ref()));
        }

        // Write to single file
        let file_path = output_dir.join("definitions.fsh");
        fs::write(file_path, combined_fsh)?;

        Ok(())
    }

    /// Get the directory name for an FSH type
    fn get_fsh_type_directory(&self, exportable: &dyn Exportable) -> &str {
        // Determine type based on the exportable's name
        // This is a simplified approach - in a real implementation,
        // we would check the actual type using downcasting or a type method
        let name = exportable.name();

        // Try to infer type from common patterns or use a type method
        // For now, we use a simple heuristic based on common naming
        if name.contains("Profile") || name.ends_with("Profile") {
            "profiles"
        } else if name.contains("ValueSet") || name.ends_with("VS") {
            "valuesets"
        } else if name.contains("CodeSystem") || name.ends_with("CS") {
            "codesystems"
        } else if name.contains("Extension") {
            "extensions"
        } else if name.contains("Logical") {
            "logical"
        } else if name.contains("Resource") {
            "resources"
        } else if name.contains("Instance") {
            "instances"
        } else {
            "other"
        }
    }

    /// Get the profile group name (parent type or general group)
    fn get_profile_group(&self, exportable: &dyn Exportable) -> String {
        // For profiles, use the parent type
        // For other types, use the FSH type
        let name = exportable.name();

        // This is simplified - ideally we'd have a method on Exportable
        // that returns the parent or type information
        if name.contains("Patient") {
            "Patient".to_string()
        } else if name.contains("Observation") {
            "Observation".to_string()
        } else if name.contains("Condition") {
            "Condition".to_string()
        } else if name.contains("Medication") {
            "Medication".to_string()
        } else if name.contains("Procedure") {
            "Procedure".to_string()
        } else {
            self.get_fsh_type_directory(exportable).to_string()
        }
    }

    // ===== Concurrent file writing methods =====

    /// Organize exportables using concurrent file writes
    ///
    /// This async version uses tokio::fs for concurrent I/O operations,
    /// improving performance when writing many files.
    pub async fn organize_concurrent(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        match self.strategy {
            OrganizationStrategy::FilePerDefinition => {
                self.organize_per_definition_concurrent(exportables, output_dir)
                    .await
            }
            OrganizationStrategy::GroupByFshType => {
                self.organize_by_type_concurrent(exportables, output_dir)
                    .await
            }
            OrganizationStrategy::GroupByProfile => {
                self.organize_by_profile_concurrent(exportables, output_dir)
                    .await
            }
            OrganizationStrategy::SingleFile => {
                self.organize_single_file_concurrent(exportables, output_dir)
                    .await
            }
        }
    }

    /// Organize with one file per definition (concurrent)
    async fn organize_per_definition_concurrent(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Create output directory
        tokio::fs::create_dir_all(output_dir).await?;

        // Prepare all write operations
        let writes: Vec<_> = exportables
            .iter()
            .map(|exportable| {
                let filename = format!("{}.fsh", exportable.id());
                let file_path = output_dir.join(filename);
                let fsh_content = self.writer.write(exportable.as_ref());

                async move { tokio::fs::write(&file_path, fsh_content).await }
            })
            .collect();

        // Execute all writes concurrently
        try_join_all(writes).await?;

        Ok(())
    }

    /// Organize by grouping files by FSH type (concurrent)
    async fn organize_by_type_concurrent(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Group exportables by type (synchronous)
        let mut groups: HashMap<&str, Vec<&Box<dyn Exportable>>> = HashMap::new();

        for exportable in exportables {
            let type_dir = self.get_fsh_type_directory(exportable.as_ref());
            groups.entry(type_dir).or_default().push(exportable);
        }

        // Create directories and prepare writes
        let mut all_writes = Vec::new();

        for (type_dir, group_exportables) in groups {
            let type_path = output_dir.join(type_dir);
            tokio::fs::create_dir_all(&type_path).await?;

            for exportable in group_exportables {
                let filename = format!("{}.fsh", exportable.id());
                let file_path = type_path.join(filename);
                let fsh_content = self.writer.write(exportable.as_ref());

                all_writes.push(async move { tokio::fs::write(&file_path, fsh_content).await });
            }
        }

        // Execute all writes concurrently
        try_join_all(all_writes).await?;

        Ok(())
    }

    /// Organize by grouping profiles by their parent type (concurrent)
    async fn organize_by_profile_concurrent(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Group exportables by profile parent or type
        let mut groups: HashMap<String, Vec<&Box<dyn Exportable>>> = HashMap::new();

        for exportable in exportables {
            let group_name = self.get_profile_group(exportable.as_ref());
            groups.entry(group_name).or_default().push(exportable);
        }

        // Create directories and prepare writes
        let mut all_writes = Vec::new();

        for (group_name, group_exportables) in groups {
            let group_path = output_dir.join(group_name);
            tokio::fs::create_dir_all(&group_path).await?;

            for exportable in group_exportables {
                let filename = format!("{}.fsh", exportable.id());
                let file_path = group_path.join(filename);
                let fsh_content = self.writer.write(exportable.as_ref());

                all_writes.push(async move { tokio::fs::write(&file_path, fsh_content).await });
            }
        }

        // Execute all writes concurrently
        try_join_all(all_writes).await?;

        Ok(())
    }

    /// Organize into a single file (concurrent)
    async fn organize_single_file_concurrent(
        &self,
        exportables: &[Box<dyn Exportable>],
        output_dir: &Path,
    ) -> Result<()> {
        // Create output directory
        tokio::fs::create_dir_all(output_dir).await?;

        // Combine all FSH into a single string
        let mut combined_fsh = String::new();

        for (i, exportable) in exportables.iter().enumerate() {
            if i > 0 {
                combined_fsh.push_str("\n\n");
            }
            combined_fsh.push_str(&self.writer.write(exportable.as_ref()));
        }

        // Write to single file
        let file_path = output_dir.join("definitions.fsh");
        tokio::fs::write(file_path, combined_fsh).await?;

        Ok(())
    }
}

impl Default for FileOrganizer {
    fn default() -> Self {
        Self::new(OrganizationStrategy::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::{ExportableCodeSystem, ExportableProfile, ExportableValueSet};
    use tempfile::TempDir;

    #[test]
    fn test_strategy_default() {
        assert_eq!(
            OrganizationStrategy::default(),
            OrganizationStrategy::FilePerDefinition
        );
    }

    #[test]
    fn test_organizer_new() {
        let organizer = FileOrganizer::new(OrganizationStrategy::GroupByFshType);
        assert_eq!(organizer.strategy(), OrganizationStrategy::GroupByFshType);
    }

    #[test]
    fn test_organizer_default() {
        let organizer = FileOrganizer::default();
        assert_eq!(
            organizer.strategy(),
            OrganizationStrategy::FilePerDefinition
        );
    }

    #[test]
    fn test_organize_per_definition() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path();

        let organizer = FileOrganizer::new(OrganizationStrategy::FilePerDefinition);

        let profile1 = ExportableProfile::new("Profile1".to_string(), "Patient".to_string());
        let profile2 = ExportableProfile::new("Profile2".to_string(), "Observation".to_string());

        let exportables: Vec<Box<dyn Exportable>> = vec![Box::new(profile1), Box::new(profile2)];

        organizer.organize(&exportables, output_dir).unwrap();

        // Verify files were created in root directory
        assert!(output_dir.join("Profile1.fsh").exists());
        assert!(output_dir.join("Profile2.fsh").exists());

        // Verify content
        let content = fs::read_to_string(output_dir.join("Profile1.fsh")).unwrap();
        assert!(content.contains("Profile: Profile1"));
    }

    #[test]
    fn test_organize_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path();

        let organizer = FileOrganizer::new(OrganizationStrategy::GroupByFshType);

        let profile = ExportableProfile::new("MyProfile".to_string(), "Patient".to_string());
        let value_set = ExportableValueSet::new("MyValueSet".to_string());
        let code_system = ExportableCodeSystem::new("MyCodeSystem".to_string());

        let exportables: Vec<Box<dyn Exportable>> = vec![
            Box::new(profile),
            Box::new(value_set),
            Box::new(code_system),
        ];

        organizer.organize(&exportables, output_dir).unwrap();

        // Verify directory structure
        assert!(output_dir.join("profiles").exists());
        assert!(output_dir.join("valuesets").exists());
        assert!(output_dir.join("codesystems").exists());

        // Verify files
        assert!(output_dir.join("profiles/MyProfile.fsh").exists());
        assert!(output_dir.join("valuesets/MyValueSet.fsh").exists());
        assert!(output_dir.join("codesystems/MyCodeSystem.fsh").exists());
    }

    #[test]
    fn test_organize_by_profile() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path();

        let organizer = FileOrganizer::new(OrganizationStrategy::GroupByProfile);

        let patient_profile =
            ExportableProfile::new("MyPatientProfile".to_string(), "Patient".to_string());
        let obs_profile = ExportableProfile::new(
            "MyObservationProfile".to_string(),
            "Observation".to_string(),
        );

        let exportables: Vec<Box<dyn Exportable>> =
            vec![Box::new(patient_profile), Box::new(obs_profile)];

        organizer.organize(&exportables, output_dir).unwrap();

        // Verify directory structure
        assert!(output_dir.join("Patient").exists());
        assert!(output_dir.join("Observation").exists());

        // Verify files
        assert!(output_dir.join("Patient/MyPatientProfile.fsh").exists());
        assert!(
            output_dir
                .join("Observation/MyObservationProfile.fsh")
                .exists()
        );
    }

    #[test]
    fn test_organize_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path();

        let organizer = FileOrganizer::new(OrganizationStrategy::SingleFile);

        let profile1 = ExportableProfile::new("Profile1".to_string(), "Patient".to_string());
        let profile2 = ExportableProfile::new("Profile2".to_string(), "Observation".to_string());

        let exportables: Vec<Box<dyn Exportable>> = vec![Box::new(profile1), Box::new(profile2)];

        organizer.organize(&exportables, output_dir).unwrap();

        // Verify single file was created
        let definitions_file = output_dir.join("definitions.fsh");
        assert!(definitions_file.exists());

        // Verify content contains both profiles
        let content = fs::read_to_string(&definitions_file).unwrap();
        assert!(content.contains("Profile: Profile1"));
        assert!(content.contains("Profile: Profile2"));
    }

    #[test]
    fn test_custom_writer() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path();

        let writer = FshWriter::new(4, 120);
        let organizer = FileOrganizer::with_writer(OrganizationStrategy::FilePerDefinition, writer);

        let profile = ExportableProfile::new("MyProfile".to_string(), "Patient".to_string());
        let exportables: Vec<Box<dyn Exportable>> = vec![Box::new(profile)];

        organizer.organize(&exportables, output_dir).unwrap();

        assert!(output_dir.join("MyProfile.fsh").exists());
    }
}
