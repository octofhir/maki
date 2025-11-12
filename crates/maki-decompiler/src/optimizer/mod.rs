//! Optimizer framework for improving FSH output
//!
//! This module provides a plugin-based optimizer system that applies various
//! optimization passes to Exportable objects to improve FSH readability and
//! eliminate redundant rules.
//!
//! ## Architecture
//!
//! - **Optimizer Trait**: Base trait for all optimizers with dependency management
//! - **OptimizerRegistry**: Manages and executes optimizers in dependency order
//! - **OptimizationStats**: Tracks changes made during optimization
//!
//! ## Dependency Management
//!
//! Optimizers can declare dependencies using:
//! - `run_before()`: This optimizer must run before the specified optimizers
//! - `run_after()`: This optimizer must run after the specified optimizers
//!
//! The registry uses topological sorting to determine the correct execution order.

pub mod plugins;
pub mod registry;
pub mod stats;

pub use plugins::*;
pub use registry::*;
pub use stats::*;

use crate::{Result, exportable::Exportable, lake::ResourceLake};

/// Base trait for all optimizers
///
/// Optimizers transform Exportable objects to improve FSH output quality.
/// They can declare dependencies on other optimizers to ensure correct
/// execution order.
pub trait Optimizer: Send + Sync {
    /// Get the unique name of this optimizer
    fn name(&self) -> &str;

    /// Get the list of optimizer names that must run before this one
    ///
    /// Returns an empty vector by default (no dependencies).
    fn run_before(&self) -> Vec<&str> {
        vec![]
    }

    /// Get the list of optimizer names that must run after this one
    ///
    /// Returns an empty vector by default (no dependencies).
    fn run_after(&self) -> Vec<&str> {
        vec![]
    }

    /// Optimize an Exportable object
    ///
    /// This is the main entry point for the optimizer. It should modify
    /// the exportable in place and return statistics about the changes made.
    ///
    /// # Arguments
    ///
    /// * `exportable` - The exportable object to optimize (modified in place)
    /// * `lake` - Resource lake for looking up FHIR definitions
    ///
    /// # Returns
    ///
    /// Statistics about the optimization (rules removed, modified, etc.)
    fn optimize(
        &self,
        exportable: &mut dyn Exportable,
        lake: &ResourceLake,
    ) -> Result<OptimizationStats>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock optimizer for testing
    struct MockOptimizer {
        name: String,
        run_before: Vec<String>,
        run_after: Vec<String>,
    }

    impl MockOptimizer {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                run_before: vec![],
                run_after: vec![],
            }
        }

        fn with_run_before(mut self, names: Vec<&str>) -> Self {
            self.run_before = names.iter().map(|s| s.to_string()).collect();
            self
        }

        fn with_run_after(mut self, names: Vec<&str>) -> Self {
            self.run_after = names.iter().map(|s| s.to_string()).collect();
            self
        }
    }

    impl Optimizer for MockOptimizer {
        fn name(&self) -> &str {
            &self.name
        }

        fn run_before(&self) -> Vec<&str> {
            self.run_before.iter().map(|s| s.as_str()).collect()
        }

        fn run_after(&self) -> Vec<&str> {
            self.run_after.iter().map(|s| s.as_str()).collect()
        }

        fn optimize(
            &self,
            _exportable: &mut dyn Exportable,
            _lake: &ResourceLake,
        ) -> Result<OptimizationStats> {
            Ok(OptimizationStats::new())
        }
    }

    #[test]
    fn test_optimizer_default_dependencies() {
        let optimizer = MockOptimizer::new("test");
        assert_eq!(optimizer.name(), "test");
        assert!(optimizer.run_before().is_empty());
        assert!(optimizer.run_after().is_empty());
    }

    #[test]
    fn test_optimizer_with_dependencies() {
        let optimizer = MockOptimizer::new("test")
            .with_run_before(vec!["opt1", "opt2"])
            .with_run_after(vec!["opt3"]);

        assert_eq!(optimizer.run_before(), vec!["opt1", "opt2"]);
        assert_eq!(optimizer.run_after(), vec!["opt3"]);
    }
}
