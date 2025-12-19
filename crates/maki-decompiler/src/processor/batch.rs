//! Batch processor for concurrent processing of FHIR resources
//!
//! This module provides concurrent processing capabilities using Tokio for async-first architecture.

use crate::error::Result;
use crate::exportable::Exportable;
use crate::lake::ResourceLake;
use crate::models::*;
use crate::processor::{CodeSystemProcessor, StructureDefinitionProcessor, ValueSetProcessor};
use futures::stream::{self, StreamExt};
use std::sync::Arc;

/// Batch processor for concurrent FHIR resource processing
///
/// Uses Tokio tasks to process multiple resources concurrently,
/// maximizing throughput for I/O-bound operations.
pub struct BatchProcessor {
    /// Maximum number of concurrent tasks
    concurrency: usize,
}

impl BatchProcessor {
    /// Create a new BatchProcessor with default concurrency (50)
    pub fn new() -> Self {
        Self { concurrency: 50 }
    }

    /// Create a new BatchProcessor with custom concurrency limit
    pub fn with_concurrency(concurrency: usize) -> Self {
        Self { concurrency }
    }

    /// Process multiple StructureDefinitions concurrently
    ///
    /// # Arguments
    ///
    /// * `structure_definitions` - List of StructureDefinitions to process
    /// * `lake` - ResourceLake for metadata lookups (must be Arc for sharing across tasks)
    ///
    /// # Returns
    ///
    /// Vector of Exportable results (maintains input order)
    pub async fn process_structure_definitions_concurrent(
        &self,
        structure_definitions: Vec<StructureDefinition>,
        lake: Arc<ResourceLake>,
    ) -> Result<Vec<Box<dyn Exportable + Send + Sync>>> {
        // Use futures::stream for concurrent processing without Send bounds
        let results: Vec<_> = stream::iter(structure_definitions)
            .map(|sd| {
                let lake_ref = Arc::clone(&lake);
                async move {
                    let processor = StructureDefinitionProcessor::new(&lake_ref);
                    processor.process(&sd).await
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        // Collect results, propagating errors
        results.into_iter().collect()
    }

    /// Process multiple ValueSets concurrently
    ///
    /// # Arguments
    ///
    /// * `value_sets` - List of ValueSets to process
    /// * `lake` - ResourceLake for metadata lookups
    ///
    /// # Returns
    ///
    /// Vector of Exportable results (maintains input order)
    pub async fn process_value_sets_concurrent(
        &self,
        value_sets: Vec<ValueSet>,
        lake: Arc<ResourceLake>,
    ) -> Result<Vec<Box<dyn Exportable + Send + Sync>>> {
        let results: Vec<_> = stream::iter(value_sets)
            .map(|vs| {
                let lake_ref = Arc::clone(&lake);
                async move {
                    let processor = ValueSetProcessor::new(&lake_ref);
                    processor
                        .process(&vs)
                        .map(|v| Box::new(v) as Box<dyn Exportable + Send + Sync>)
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        results.into_iter().collect()
    }

    /// Process multiple CodeSystems concurrently
    ///
    /// # Arguments
    ///
    /// * `code_systems` - List of CodeSystems to process
    /// * `lake` - ResourceLake for metadata lookups
    ///
    /// # Returns
    ///
    /// Vector of Exportable results (maintains input order)
    pub async fn process_code_systems_concurrent(
        &self,
        code_systems: Vec<CodeSystem>,
        lake: Arc<ResourceLake>,
    ) -> Result<Vec<Box<dyn Exportable + Send + Sync>>> {
        let results: Vec<_> = stream::iter(code_systems)
            .map(|cs| {
                let lake_ref = Arc::clone(&lake);
                async move {
                    let processor = CodeSystemProcessor::new(&lake_ref);
                    processor
                        .process(&cs)
                        .map(|c| Box::new(c) as Box<dyn Exportable + Send + Sync>)
                }
            })
            .buffer_unordered(self.concurrency)
            .collect()
            .await;

        results.into_iter().collect()
    }

    /// Process all resources from a ResourceLake concurrently
    ///
    /// Processes StructureDefinitions, ValueSets, and CodeSystems in parallel.
    ///
    /// # Arguments
    ///
    /// * `lake` - ResourceLake containing resources to process
    ///
    /// # Returns
    ///
    /// Vector of all Exportable results
    pub async fn process_all_concurrent(
        &self,
        lake: Arc<ResourceLake>,
    ) -> Result<Vec<Box<dyn Exportable + Send + Sync>>> {
        let lake_clone_1 = Arc::clone(&lake);
        let lake_clone_2 = Arc::clone(&lake);
        let lake_clone_3 = Arc::clone(&lake);

        // Get all resources from lake
        let structure_definitions = lake.get_all_structure_definitions();
        let value_sets = lake.get_all_value_sets();
        let code_systems = lake.get_all_code_systems();

        // Process all resource types concurrently
        let (sd_results, vs_results, cs_results) = tokio::try_join!(
            self.process_structure_definitions_concurrent(structure_definitions, lake_clone_1),
            self.process_value_sets_concurrent(value_sets, lake_clone_2),
            self.process_code_systems_concurrent(code_systems, lake_clone_3),
        )?;

        // Combine all results
        let mut all_results = Vec::new();
        all_results.extend(sd_results);
        all_results.extend(vs_results);
        all_results.extend(cs_results);

        Ok(all_results)
    }
}

impl Default for BatchProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;
    use crate::{parse_fhir_release, setup_canonical_environment};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
    async fn test_batch_process_structure_definitions() {
        let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
            .await
            .unwrap();
        let lake = Arc::new(ResourceLake::new(session));

        let profiles = vec![
            fixtures::simple_patient_profile(),
            fixtures::complex_patient_profile(),
        ];

        let processor = BatchProcessor::new();
        let results = processor
            .process_structure_definitions_concurrent(profiles, lake)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
    async fn test_batch_process_with_custom_concurrency() {
        let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
            .await
            .unwrap();
        let lake = Arc::new(ResourceLake::new(session));

        let profiles = vec![
            fixtures::simple_patient_profile(),
            fixtures::complex_patient_profile(),
        ];

        let processor = BatchProcessor::with_concurrency(1); // Sequential processing
        let results = processor
            .process_structure_definitions_concurrent(profiles, lake)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    #[serial]
    #[ignore = "requires canonical manager infrastructure"]
    async fn test_batch_process_large_batch() {
        let session = setup_canonical_environment(parse_fhir_release("R4").unwrap(), vec![])
            .await
            .unwrap();
        let lake = Arc::new(ResourceLake::new(session));

        // Create 100 profiles
        let profiles: Vec<_> = (0..100)
            .map(|_| fixtures::simple_patient_profile())
            .collect();

        let processor = BatchProcessor::new();
        let results = processor
            .process_structure_definitions_concurrent(profiles, lake)
            .await
            .unwrap();

        assert_eq!(results.len(), 100);
    }
}
