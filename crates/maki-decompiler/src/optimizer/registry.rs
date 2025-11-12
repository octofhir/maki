//! Optimizer registry and execution engine
//!
//! Manages a collection of optimizers and executes them in dependency order.

use super::{Optimizer, OptimizationStats};
use crate::{
    exportable::Exportable,
    lake::ResourceLake,
    Error, Result,
};
use log::{debug, info};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use std::collections::HashMap;

/// Registry of optimizers with dependency-aware execution
///
/// The registry manages a collection of optimizers and executes them in the
/// correct order based on their declared dependencies (run_before/run_after).
pub struct OptimizerRegistry {
    optimizers: Vec<Box<dyn Optimizer>>,
}

impl OptimizerRegistry {
    /// Create a new empty optimizer registry
    pub fn new() -> Self {
        Self {
            optimizers: Vec::new(),
        }
    }

    /// Add an optimizer to the registry
    pub fn add(&mut self, optimizer: Box<dyn Optimizer>) {
        debug!("Registering optimizer: {}", optimizer.name());
        self.optimizers.push(optimizer);
    }

    /// Get the number of registered optimizers
    pub fn len(&self) -> usize {
        self.optimizers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.optimizers.is_empty()
    }

    /// Optimize an exportable object using all registered optimizers
    ///
    /// Optimizers are executed in dependency order determined by topological sort.
    /// Returns combined statistics from all optimizers.
    pub fn optimize_all(
        &self,
        exportable: &mut dyn Exportable,
        lake: &ResourceLake,
    ) -> Result<OptimizationStats> {
        if self.optimizers.is_empty() {
            debug!("No optimizers registered, skipping optimization");
            return Ok(OptimizationStats::new());
        }

        info!("Starting optimization with {} optimizers", self.optimizers.len());

        // Get optimizers in dependency order
        let ordered = self.topological_sort()?;

        // Apply each optimizer in order
        let mut total_stats = OptimizationStats::new();

        for optimizer in ordered {
            debug!("Running optimizer: {}", optimizer.name());

            let stats = optimizer.optimize(exportable, lake)?;

            if stats.has_changes() {
                info!(
                    "Optimizer '{}' made changes: {}",
                    optimizer.name(),
                    stats
                );
            } else {
                debug!("Optimizer '{}' made no changes", optimizer.name());
            }

            total_stats.merge(&stats);
        }

        info!("Optimization complete: {}", total_stats);

        Ok(total_stats)
    }

    /// Sort optimizers by their dependencies using topological sort
    ///
    /// Returns optimizers in an order that respects all run_before/run_after constraints.
    fn topological_sort(&self) -> Result<Vec<&dyn Optimizer>> {
        // Build a name-to-index mapping
        let mut name_to_idx: HashMap<&str, usize> = HashMap::new();
        for (idx, optimizer) in self.optimizers.iter().enumerate() {
            name_to_idx.insert(optimizer.name(), idx);
        }

        // Build dependency graph
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        let mut nodes: HashMap<usize, NodeIndex> = HashMap::new();

        // Add all optimizers as nodes
        for idx in 0..self.optimizers.len() {
            nodes.insert(idx, graph.add_node(idx));
        }

        // Add edges based on dependencies
        for (idx, optimizer) in self.optimizers.iter().enumerate() {
            let current_node = nodes[&idx];

            // run_before: this optimizer must run before the specified ones
            // So add edge: this -> specified
            for before_name in optimizer.run_before() {
                if let Some(&before_idx) = name_to_idx.get(before_name) {
                    let before_node = nodes[&before_idx];
                    graph.add_edge(current_node, before_node, ());
                    debug!(
                        "Dependency: {} must run before {}",
                        optimizer.name(),
                        before_name
                    );
                }
            }

            // run_after: this optimizer must run after the specified ones
            // So add edge: specified -> this
            for after_name in optimizer.run_after() {
                if let Some(&after_idx) = name_to_idx.get(after_name) {
                    let after_node = nodes[&after_idx];
                    graph.add_edge(after_node, current_node, ());
                    debug!(
                        "Dependency: {} must run after {}",
                        optimizer.name(),
                        after_name
                    );
                }
            }
        }

        // Perform topological sort
        let sorted_nodes = toposort(&graph, None).map_err(|cycle| {
            Error::Processing(format!(
                "Circular dependency detected in optimizer graph at node {:?}",
                cycle.node_id()
            ))
        })?;

        // Convert node indices back to optimizer references
        let mut result = Vec::new();
        for node in sorted_nodes {
            let optimizer_idx = graph[node];
            result.push(self.optimizers[optimizer_idx].as_ref());
        }

        debug!(
            "Optimizer execution order: {:?}",
            result.iter().map(|o| o.name()).collect::<Vec<_>>()
        );

        Ok(result)
    }
}

impl Default for OptimizerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exportable::ExportableProfile;
    use maki_core::canonical::{CanonicalFacade, CanonicalOptions, FhirRelease};
    use std::sync::Arc;

    // Mock optimizer for testing
    struct MockOptimizer {
        name: String,
        run_before: Vec<String>,
        run_after: Vec<String>,
        execution_order: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl MockOptimizer {
        fn new(name: &str, execution_order: Arc<std::sync::Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                run_before: vec![],
                run_after: vec![],
                execution_order,
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
            // Record execution order
            self.execution_order.lock().unwrap().push(self.name.clone());
            Ok(OptimizationStats::new())
        }
    }

    async fn create_test_lake() -> ResourceLake {
        let options = CanonicalOptions {
            quick_init: true,
            auto_install_core: false,
            ..Default::default()
        };
        let facade = CanonicalFacade::new(options).await.unwrap();
        let session = facade.session(vec![FhirRelease::R4]).await.unwrap();
        ResourceLake::new(Arc::new(session))
    }

    #[test]
    fn test_empty_registry() {
        let registry = OptimizerRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_add_optimizer() {
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));

        registry.add(Box::new(MockOptimizer::new("test", execution_order)));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[tokio::test]
    async fn test_optimize_empty_registry() {
        let registry = OptimizerRegistry::new();
        let lake = create_test_lake().await;
        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());

        let stats = registry.optimize_all(&mut profile, &lake).unwrap();

        assert!(!stats.has_changes());
    }

    #[tokio::test]
    async fn test_optimize_single_optimizer() {
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let lake = create_test_lake().await;

        registry.add(Box::new(MockOptimizer::new("opt1", execution_order.clone())));

        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());
        registry.optimize_all(&mut profile, &lake).unwrap();

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["opt1"]);
    }

    #[tokio::test]
    async fn test_topological_sort_simple_chain() {
        // opt1 -> opt2 -> opt3 (must run in this order)
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let lake = create_test_lake().await;

        registry.add(Box::new(
            MockOptimizer::new("opt1", execution_order.clone()).with_run_before(vec!["opt2"]),
        ));
        registry.add(Box::new(
            MockOptimizer::new("opt2", execution_order.clone()).with_run_before(vec!["opt3"]),
        ));
        registry.add(Box::new(MockOptimizer::new("opt3", execution_order.clone())));

        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());
        registry.optimize_all(&mut profile, &lake).unwrap();

        let order = execution_order.lock().unwrap();
        assert_eq!(*order, vec!["opt1", "opt2", "opt3"]);
    }

    #[tokio::test]
    async fn test_topological_sort_run_after() {
        // opt3 must run after opt1 and opt2
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let lake = create_test_lake().await;

        registry.add(Box::new(MockOptimizer::new("opt1", execution_order.clone())));
        registry.add(Box::new(MockOptimizer::new("opt2", execution_order.clone())));
        registry.add(Box::new(
            MockOptimizer::new("opt3", execution_order.clone())
                .with_run_after(vec!["opt1", "opt2"]),
        ));

        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());
        registry.optimize_all(&mut profile, &lake).unwrap();

        let order = execution_order.lock().unwrap();
        // opt1 and opt2 must come before opt3
        let opt3_pos = order.iter().position(|x| x == "opt3").unwrap();
        let opt1_pos = order.iter().position(|x| x == "opt1").unwrap();
        let opt2_pos = order.iter().position(|x| x == "opt2").unwrap();

        assert!(opt1_pos < opt3_pos);
        assert!(opt2_pos < opt3_pos);
    }

    #[tokio::test]
    async fn test_topological_sort_complex() {
        // Complex dependency graph:
        // opt1 -> opt2 -> opt4
        //      -> opt3 -> opt4
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let lake = create_test_lake().await;

        registry.add(Box::new(
            MockOptimizer::new("opt1", execution_order.clone())
                .with_run_before(vec!["opt2", "opt3"]),
        ));
        registry.add(Box::new(
            MockOptimizer::new("opt2", execution_order.clone()).with_run_before(vec!["opt4"]),
        ));
        registry.add(Box::new(
            MockOptimizer::new("opt3", execution_order.clone()).with_run_before(vec!["opt4"]),
        ));
        registry.add(Box::new(MockOptimizer::new("opt4", execution_order.clone())));

        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());
        registry.optimize_all(&mut profile, &lake).unwrap();

        let order = execution_order.lock().unwrap();

        // Verify order constraints
        let opt1_pos = order.iter().position(|x| x == "opt1").unwrap();
        let opt2_pos = order.iter().position(|x| x == "opt2").unwrap();
        let opt3_pos = order.iter().position(|x| x == "opt3").unwrap();
        let opt4_pos = order.iter().position(|x| x == "opt4").unwrap();

        assert!(opt1_pos < opt2_pos);
        assert!(opt1_pos < opt3_pos);
        assert!(opt2_pos < opt4_pos);
        assert!(opt3_pos < opt4_pos);
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        // opt1 -> opt2 -> opt1 (circular)
        let mut registry = OptimizerRegistry::new();
        let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let lake = create_test_lake().await;

        registry.add(Box::new(
            MockOptimizer::new("opt1", execution_order.clone()).with_run_before(vec!["opt2"]),
        ));
        registry.add(Box::new(
            MockOptimizer::new("opt2", execution_order.clone()).with_run_before(vec!["opt1"]),
        ));

        let mut profile = ExportableProfile::new("TestProfile".to_string(), "Patient".to_string());
        let result = registry.optimize_all(&mut profile, &lake);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Circular dependency"));
    }
}
