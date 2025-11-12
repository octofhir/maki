//! Statistics tracking for GoFSH operations
//!
//! This module provides statistics structures for tracking progress and results
//! during the FHIR-to-FSH conversion process.

use crate::optimizer::OptimizationStats;
use std::fmt;
use std::time::Duration;

/// Statistics for resource processing phase
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Number of profiles processed
    pub profiles_processed: usize,
    /// Number of extensions processed
    pub extensions_processed: usize,
    /// Number of value sets processed
    pub value_sets_processed: usize,
    /// Number of code systems processed
    pub code_systems_processed: usize,
    /// Number of instances processed
    pub instances_processed: usize,
    /// Number of rules extracted
    pub rules_extracted: usize,
    /// Number of processing errors
    pub errors: usize,
    /// Optimization statistics
    pub optimization_stats: OptimizationStats,
}

impl ProcessingStats {
    /// Create new empty processing statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total number of resources processed
    pub fn total_processed(&self) -> usize {
        self.profiles_processed
            + self.extensions_processed
            + self.value_sets_processed
            + self.code_systems_processed
            + self.instances_processed
    }

    /// Check if any resources were processed
    pub fn has_processed(&self) -> bool {
        self.total_processed() > 0
    }
}

impl fmt::Display for ProcessingStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} resources processed", self.total_processed())?;
        if self.rules_extracted > 0 {
            write!(f, ", {} rules extracted", self.rules_extracted)?;
        }
        if self.optimization_stats.has_changes() {
            write!(f, ", optimized ({})", self.optimization_stats)?;
        }
        if self.errors > 0 {
            write!(f, ", {} errors", self.errors)?;
        }
        Ok(())
    }
}

/// Statistics for file writing phase
#[derive(Debug, Clone, Default)]
pub struct WriteStats {
    /// Number of FSH files written
    pub files_written: usize,
    /// Total bytes written
    pub bytes_written: usize,
    /// Number of write errors
    pub errors: usize,
}

impl WriteStats {
    /// Create new empty write statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any files were written
    pub fn has_written(&self) -> bool {
        self.files_written > 0
    }

    /// Get human-readable size
    pub fn human_size(&self) -> String {
        let bytes = self.bytes_written as f64;
        if bytes < 1024.0 {
            format!("{} B", bytes)
        } else if bytes < 1024.0 * 1024.0 {
            format!("{:.2} KB", bytes / 1024.0)
        } else {
            format!("{:.2} MB", bytes / (1024.0 * 1024.0))
        }
    }
}

impl fmt::Display for WriteStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} files written ({})",
            self.files_written,
            self.human_size()
        )?;
        if self.errors > 0 {
            write!(f, ", {} errors", self.errors)?;
        }
        Ok(())
    }
}

/// Summary of entire GoFSH operation
#[derive(Debug, Clone)]
pub struct GoFshSummary {
    /// Loading statistics
    pub load_stats: crate::LoadStats,
    /// Processing statistics
    pub processing_stats: ProcessingStats,
    /// Writing statistics
    pub write_stats: WriteStats,
    /// Total duration
    pub duration: Duration,
    /// Whether config files were generated
    pub config_generated: bool,
}

impl GoFshSummary {
    /// Create new summary
    pub fn new(
        load_stats: crate::LoadStats,
        processing_stats: ProcessingStats,
        write_stats: WriteStats,
        duration: Duration,
        config_generated: bool,
    ) -> Self {
        Self {
            load_stats,
            processing_stats,
            write_stats,
            duration,
            config_generated,
        }
    }

    /// Get total number of errors across all phases
    pub fn total_errors(&self) -> usize {
        self.load_stats.errors + self.processing_stats.errors + self.write_stats.errors
    }

    /// Check if the operation was successful
    pub fn is_success(&self) -> bool {
        self.total_errors() == 0 && self.write_stats.has_written()
    }
}

impl fmt::Display for GoFshSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "GoFSH Conversion Summary")?;
        writeln!(f, "  Loaded: {} resources", self.load_stats.loaded)?;
        writeln!(f, "  Processed: {}", self.processing_stats)?;
        writeln!(f, "  Written: {}", self.write_stats)?;
        writeln!(f, "  Duration: {:.2}s", self.duration.as_secs_f64())?;
        if self.config_generated {
            writeln!(f, "  Config: Generated")?;
        }
        if self.total_errors() > 0 {
            writeln!(f, "  Total errors: {}", self.total_errors())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_stats_total() {
        let mut stats = ProcessingStats::new();
        assert_eq!(stats.total_processed(), 0);

        stats.profiles_processed = 5;
        stats.extensions_processed = 3;
        stats.value_sets_processed = 2;
        assert_eq!(stats.total_processed(), 10);
    }

    #[test]
    fn test_processing_stats_display() {
        let mut stats = ProcessingStats::new();
        stats.profiles_processed = 5;
        stats.rules_extracted = 42;

        let display = format!("{}", stats);
        assert!(display.contains("5 resources"));
        assert!(display.contains("42 rules"));
    }

    #[test]
    fn test_write_stats_human_size() {
        let mut stats = WriteStats::new();

        stats.bytes_written = 512;
        assert_eq!(stats.human_size(), "512 B");

        stats.bytes_written = 2048;
        assert_eq!(stats.human_size(), "2.00 KB");

        stats.bytes_written = 2_097_152; // 2 MB
        assert_eq!(stats.human_size(), "2.00 MB");
    }

    #[test]
    fn test_write_stats_display() {
        let mut stats = WriteStats::new();
        stats.files_written = 10;
        stats.bytes_written = 15360; // 15 KB

        let display = format!("{}", stats);
        assert!(display.contains("10 files"));
        assert!(display.contains("15.00 KB"));
    }

    #[test]
    fn test_gofsh_summary() {
        use crate::LoadStats;

        let load_stats = LoadStats {
            loaded: 20,
            errors: 0,
            error_details: vec![],
        };

        let mut processing_stats = ProcessingStats::new();
        processing_stats.profiles_processed = 10;
        processing_stats.rules_extracted = 100;

        let mut write_stats = WriteStats::new();
        write_stats.files_written = 10;
        write_stats.bytes_written = 50000;

        let summary = GoFshSummary::new(
            load_stats,
            processing_stats,
            write_stats,
            Duration::from_secs(5),
            true,
        );

        assert_eq!(summary.total_errors(), 0);
        assert!(summary.is_success());

        let display = format!("{}", summary);
        assert!(display.contains("20 resources"));
        assert!(display.contains("10 files"));
        assert!(display.contains("Config: Generated"));
    }
}
