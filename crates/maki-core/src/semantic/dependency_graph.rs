//! Dependency graph construction and topological sorting
//!
//! This module builds a dependency graph from FSH definitions and provides
//! topological sorting to determine the correct processing order. It also
//! detects circular dependencies and computes parallel processing batches.
//!
//! # Overview
//!
//! FSH definitions often depend on other definitions:
//! - Profiles depend on their parent
//! - ValueSets are referenced by bindings
//! - Extensions are referenced by profiles
//! - Instances depend on profiles
//!
//! To process definitions correctly, we must process dependencies before dependents.
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::semantic::dependency_graph::{DependencyGraph, DependencyType};
//! use std::ops::Range;
//!
//! let mut graph = DependencyGraph::new();
//!
//! // Add nodes
//! graph.add_node("Patient".to_string());
//! graph.add_node("MyPatientProfile".to_string());
//!
//! // Add edge: MyPatientProfile depends on Patient
//! graph.add_edge(
//!     "MyPatientProfile",
//!     "Patient",
//!     DependencyType::Parent,
//!     0..10
//! );
//!
//! // Get topological sort
//! let sorted = graph.topological_sort().unwrap();
//! assert_eq!(sorted, vec!["Patient", "MyPatientProfile"]);
//! ```

use crate::semantic::{FhirResource, ResourceType, SemanticModel};
use petgraph::Direction;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace, warn};

/// Type of dependency between FSH definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyType {
    /// Profile parent dependency (Profile: Parent Observation)
    Parent,

    /// ValueSet binding dependency (* code from MyValueSet)
    ValueSetBinding,

    /// Extension reference (* extension contains MyExtension)
    ExtensionReference,

    /// Type reference (* value[x] only MyType)
    TypeReference,

    /// Profile/invariant constraint reference (obeys MyInvariant)
    ProfileReference,

    /// Instance-of dependency (Instance: * of MyProfile)
    InstanceOf,

    /// CodeSystem reference (from MyCodeSystem)
    CodeSystemReference,
}

impl DependencyType {
    /// Get a display name for this dependency type
    pub fn display_name(&self) -> &'static str {
        match self {
            DependencyType::Parent => "Parent",
            DependencyType::ValueSetBinding => "ValueSet Binding",
            DependencyType::ExtensionReference => "Extension Reference",
            DependencyType::TypeReference => "Type Reference",
            DependencyType::ProfileReference => "Profile Reference",
            DependencyType::InstanceOf => "Instance Of",
            DependencyType::CodeSystemReference => "CodeSystem Reference",
        }
    }
}

/// Dependency edge with metadata
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source definition name
    pub from: String,
    /// Target definition name
    pub to: String,
    /// Type of dependency
    pub dep_type: DependencyType,
    /// Source code location where dependency is declared
    pub source_location: Range<usize>,
}

/// Errors that can occur during dependency graph operations
#[derive(Debug, Error)]
pub enum DependencyError {
    /// Circular dependency detected
    #[error("Circular dependency detected: {}", format_cycle(.cycle))]
    CircularDependency { cycle: Vec<String> },

    /// Missing dependency
    #[error("Missing dependency: {name} referenced by {referrer} at {location:?} but not defined")]
    MissingDependency {
        name: String,
        referrer: String,
        location: Range<usize>,
    },

    /// Graph construction failed
    #[error("Dependency graph construction failed: {0}")]
    GraphConstructionFailed(String),

    /// Topological sort failed
    #[error("Topological sort failed: {0}")]
    TopologicalSortFailed(String),

    /// Node not found in graph
    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Format cycle for error messages
fn format_cycle(cycle: &[String]) -> String {
    cycle.join(" → ")
}

/// Dependency graph using petgraph
///
/// Provides efficient graph operations for dependency analysis:
/// - Topological sorting
/// - Cycle detection
/// - Path finding
/// - Batch computation for parallel processing
pub struct DependencyGraph {
    /// Directed graph (node = definition name, edge = dependency)
    graph: DiGraph<String, DependencyEdge>,
    /// Map from definition name to graph node index
    node_map: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a node (definition) to the graph
    ///
    /// Returns the node index. If the node already exists, returns existing index.
    pub fn add_node(&mut self, name: String) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&name) {
            return idx;
        }

        let idx = self.graph.add_node(name.clone());
        self.node_map.insert(name, idx);
        idx
    }

    /// Add a dependency edge
    ///
    /// Creates nodes if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `from` - Source definition (dependent)
    /// * `to` - Target definition (dependency)
    /// * `dep_type` - Type of dependency
    /// * `location` - Source code location
    pub fn add_edge(
        &mut self,
        from: &str,
        to: &str,
        dep_type: DependencyType,
        location: Range<usize>,
    ) {
        let from_idx = self.add_node(from.to_string());
        let to_idx = self.add_node(to.to_string());

        let edge = DependencyEdge {
            from: from.to_string(),
            to: to.to_string(),
            dep_type,
            source_location: location,
        };

        self.graph.add_edge(from_idx, to_idx, edge);
    }

    /// Get all direct dependencies of a node
    ///
    /// Returns the names of definitions that this definition depends on.
    pub fn get_dependencies(&self, name: &str) -> Vec<&str> {
        if let Some(&idx) = self.node_map.get(name) {
            self.graph
                .edges_directed(idx, Direction::Outgoing)
                .map(|edge| self.graph[edge.target()].as_str())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all direct dependents of a node (reverse dependencies)
    ///
    /// Returns the names of definitions that depend on this definition.
    pub fn get_dependents(&self, name: &str) -> Vec<&str> {
        if let Some(&idx) = self.node_map.get(name) {
            self.graph
                .edges_directed(idx, Direction::Incoming)
                .map(|edge| self.graph[edge.source()].as_str())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Perform topological sort
    ///
    /// Returns definitions in dependency order (dependencies before dependents).
    ///
    /// # Errors
    ///
    /// Returns `CircularDependency` error if the graph contains cycles.
    pub fn topological_sort(&self) -> Result<Vec<String>, DependencyError> {
        trace!(
            "Performing topological sort on {} nodes",
            self.graph.node_count()
        );

        // Check for cycles first
        if is_cyclic_directed(&self.graph) {
            let cycles = self.find_cycles();
            if let Some(cycle) = cycles.first() {
                return Err(DependencyError::CircularDependency {
                    cycle: cycle.clone(),
                });
            }
        }

        match toposort(&self.graph, None) {
            Ok(sorted) => {
                let mut result: Vec<String> = sorted
                    .into_iter()
                    .map(|idx| self.graph[idx].clone())
                    .collect();
                // Reverse because toposort returns in reverse order
                result.reverse();
                debug!("Topological sort produced {} nodes", result.len());
                Ok(result)
            }
            Err(cycle) => {
                // Shouldn't happen since we checked above, but handle it
                let node_name = self.graph[cycle.node_id()].clone();
                Err(DependencyError::TopologicalSortFailed(format!(
                    "Cycle detected at node: {}",
                    node_name
                )))
            }
        }
    }

    /// Find all cycles in the graph
    ///
    /// Returns a list of cycles, where each cycle is a list of node names.
    pub fn find_cycles(&self) -> Vec<Vec<String>> {
        use petgraph::algo::tarjan_scc;

        let sccs = tarjan_scc(&self.graph);

        // Filter to only cycles (SCCs with more than 1 node or self-loops)
        sccs.into_iter()
            .filter(|scc| {
                scc.len() > 1 || (scc.len() == 1 && self.graph.contains_edge(scc[0], scc[0]))
            })
            .map(|scc| {
                let mut cycle: Vec<String> =
                    scc.iter().map(|&idx| self.graph[idx].clone()).collect();
                // Add first node at end to show the cycle
                if !cycle.is_empty() {
                    cycle.push(cycle[0].clone());
                }
                cycle
            })
            .collect()
    }

    /// Get strongly connected components
    ///
    /// Returns groups of nodes that are mutually reachable.
    pub fn strongly_connected_components(&self) -> Vec<Vec<String>> {
        use petgraph::algo::tarjan_scc;

        tarjan_scc(&self.graph)
            .into_iter()
            .map(|scc| scc.iter().map(|&idx| self.graph[idx].clone()).collect())
            .collect()
    }

    /// Check if there's a path from `from` to `to`
    ///
    /// Uses BFS to determine reachability.
    pub fn has_path(&self, from: &str, to: &str) -> bool {
        let from_idx = match self.node_map.get(from) {
            Some(&idx) => idx,
            None => return false,
        };

        let to_idx = match self.node_map.get(to) {
            Some(&idx) => idx,
            None => return false,
        };

        use petgraph::algo::has_path_connecting;
        has_path_connecting(&self.graph, from_idx, to_idx, None)
    }

    /// Get processing batches (definitions that can be processed in parallel)
    ///
    /// Returns batches of definitions where:
    /// - All definitions in a batch have no dependencies on each other
    /// - Each batch only depends on previous batches
    /// - Definitions within a batch can be processed in parallel
    ///
    /// # Example
    ///
    /// ```text
    /// A, B (no dependencies) -> Batch 0
    /// C depends on A, D depends on B -> Batch 1
    /// E depends on C -> Batch 2
    /// ```
    pub fn get_processing_batches(&self) -> Vec<Vec<String>> {
        let mut batches: Vec<Vec<String>> = Vec::new();
        let mut levels: HashMap<NodeIndex, usize> = HashMap::new();

        // Compute level for each node (max distance from any root)
        let mut queue: VecDeque<NodeIndex> = VecDeque::new();

        // Find all roots (nodes with no dependencies)
        for node in self.graph.node_indices() {
            if self.graph.edges_directed(node, Direction::Outgoing).count() == 0 {
                levels.insert(node, 0);
                queue.push_back(node);
            }
        }

        // BFS to assign levels
        while let Some(node) = queue.pop_front() {
            let _current_level = levels[&node];

            // Process all dependents
            for edge in self.graph.edges_directed(node, Direction::Incoming) {
                let dependent = edge.source();

                // Check if all dependencies of dependent have been processed
                let max_dep_level = self
                    .graph
                    .edges_directed(dependent, Direction::Outgoing)
                    .filter_map(|e| levels.get(&e.target()))
                    .max()
                    .copied();

                if let Some(max_level) = max_dep_level {
                    let new_level = max_level + 1;

                    // Update level if necessary
                    if levels.get(&dependent).copied().unwrap_or(0) < new_level {
                        levels.insert(dependent, new_level);
                    }

                    // Check if all dependencies are resolved
                    let all_deps_resolved = self
                        .graph
                        .edges_directed(dependent, Direction::Outgoing)
                        .all(|e| levels.contains_key(&e.target()));

                    if all_deps_resolved && !queue.contains(&dependent) {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        // Group nodes by level
        let max_level = levels.values().max().copied().unwrap_or(0);
        batches.resize(max_level + 1, Vec::new());

        for (node, level) in levels {
            batches[level].push(self.graph[node].clone());
        }

        // Remove empty batches and return
        batches.into_iter().filter(|b| !b.is_empty()).collect()
    }

    /// Get number of nodes in graph
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get number of edges in graph
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Convert graph to DOT format for visualization
    ///
    /// Can be rendered with Graphviz:
    /// ```bash
    /// echo "$dot_output" | dot -Tpng > graph.png
    /// ```
    pub fn to_dot(&self) -> String {
        format!(
            "{:?}",
            Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
        )
    }

    /// Get all nodes in the graph
    pub fn all_nodes(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .map(|idx| self.graph[idx].clone())
            .collect()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependency analyzer
///
/// Analyzes FSH semantic models to extract dependencies and build the dependency graph.
pub struct DependencyAnalyzer {
    /// Semantic model to analyze
    model: Arc<SemanticModel>,
}

impl DependencyAnalyzer {
    /// Create a new dependency analyzer
    pub fn new(model: Arc<SemanticModel>) -> Self {
        Self { model }
    }

    /// Build dependency graph from semantic model
    ///
    /// Analyzes all resources in the semantic model and extracts dependencies.
    pub fn build_graph(&self) -> Result<DependencyGraph, DependencyError> {
        debug!(
            "Building dependency graph from {} resources",
            self.model.resources.len()
        );

        let mut graph = DependencyGraph::new();

        // Add all resources as nodes first
        for resource in &self.model.resources {
            graph.add_node(resource.id.clone());
        }

        // Analyze each resource for dependencies
        for resource in &self.model.resources {
            let edges = self.analyze_resource(resource);

            for (target, dep_type, location) in edges {
                trace!(
                    "Adding edge: {} -[{}]-> {}",
                    resource.id,
                    dep_type.display_name(),
                    target
                );
                graph.add_edge(&resource.id, &target, dep_type, location);
            }
        }

        debug!(
            "Built dependency graph: {} nodes, {} edges",
            graph.node_count(),
            graph.edge_count()
        );

        Ok(graph)
    }

    /// Analyze a single resource for dependencies
    fn analyze_resource(
        &self,
        resource: &FhirResource,
    ) -> Vec<(String, DependencyType, Range<usize>)> {
        let mut dependencies = Vec::new();

        // Parent dependency
        if let Some(parent) = &resource.parent {
            let location =
                resource.location.offset..resource.location.offset + resource.location.length;
            dependencies.push((parent.clone(), DependencyType::Parent, location));
        }

        // Instance-of dependency
        if resource.resource_type == ResourceType::Instance
            && let Some(parent) = &resource.parent
        {
            let location =
                resource.location.offset..resource.location.offset + resource.location.length;
            dependencies.push((parent.clone(), DependencyType::InstanceOf, location));
        }

        // Analyze elements for additional dependencies
        for element in &resource.elements {
            // Type references
            if let Some(type_info) = &element.type_info {
                let location =
                    element.location.offset..element.location.offset + element.location.length;

                // Type reference
                if !Self::is_primitive_type(&type_info.type_name) {
                    dependencies.push((
                        type_info.type_name.clone(),
                        DependencyType::TypeReference,
                        location.clone(),
                    ));
                }

                // Profile reference
                if let Some(profile) = &type_info.profile {
                    dependencies.push((
                        profile.clone(),
                        DependencyType::ProfileReference,
                        location.clone(),
                    ));
                }

                // Target type references (for Reference types)
                for target in &type_info.target_types {
                    dependencies.push((
                        target.clone(),
                        DependencyType::TypeReference,
                        location.clone(),
                    ));
                }
            }

            // Constraint dependencies
            for constraint in &element.constraints {
                use crate::semantic::ConstraintType;

                let location = constraint.location.offset
                    ..constraint.location.offset + constraint.location.length;

                match constraint.constraint_type {
                    ConstraintType::Binding => {
                        dependencies.push((
                            constraint.value.clone(),
                            DependencyType::ValueSetBinding,
                            location,
                        ));
                    }
                    ConstraintType::Obeys => {
                        dependencies.push((
                            constraint.value.clone(),
                            DependencyType::ProfileReference,
                            location,
                        ));
                    }
                    ConstraintType::Contains => {
                        dependencies.push((
                            constraint.value.clone(),
                            DependencyType::ExtensionReference,
                            location,
                        ));
                    }
                    _ => {}
                }
            }
        }

        dependencies
    }

    /// Check if a type name is a primitive FHIR type
    fn is_primitive_type(type_name: &str) -> bool {
        matches!(
            type_name,
            "boolean"
                | "integer"
                | "string"
                | "decimal"
                | "uri"
                | "url"
                | "canonical"
                | "base64Binary"
                | "instant"
                | "date"
                | "dateTime"
                | "time"
                | "code"
                | "oid"
                | "id"
                | "markdown"
                | "unsignedInt"
                | "positiveInt"
                | "uuid"
        )
    }

    /// Validate the dependency graph
    ///
    /// Checks for missing dependencies and circular references.
    pub fn validate_graph(&self, graph: &DependencyGraph) -> Vec<DependencyError> {
        let mut errors = Vec::new();

        // Check for missing dependencies
        let defined_names: HashSet<String> =
            self.model.resources.iter().map(|r| r.id.clone()).collect();

        for resource in &self.model.resources {
            let dependencies = graph.get_dependencies(&resource.id);

            for dep in dependencies {
                if !defined_names.contains(dep) && !Self::is_builtin(dep) {
                    warn!("Missing dependency: {} referenced by {}", dep, resource.id);
                    let location = resource.location.offset
                        ..resource.location.offset + resource.location.length;
                    errors.push(DependencyError::MissingDependency {
                        name: dep.to_string(),
                        referrer: resource.id.clone(),
                        location,
                    });
                }
            }
        }

        // Check for cycles
        let cycles = graph.find_cycles();
        for cycle in cycles {
            errors.push(DependencyError::CircularDependency { cycle });
        }

        errors
    }

    /// Check if a name is a built-in FHIR resource
    fn is_builtin(name: &str) -> bool {
        // Common FHIR base resources and types
        matches!(
            name,
            "Patient"
                | "Observation"
                | "Practitioner"
                | "Organization"
                | "Condition"
                | "Procedure"
                | "Medication"
                | "MedicationRequest"
                | "AllergyIntolerance"
                | "DiagnosticReport"
                | "Encounter"
                | "CarePlan"
                | "CareTeam"
                | "Device"
                | "Goal"
                | "Immunization"
                | "Specimen"
                | "HumanName"
                | "Address"
                | "ContactPoint"
                | "Identifier"
                | "CodeableConcept"
                | "Coding"
                | "Reference"
                | "Period"
                | "Quantity"
                | "Range"
                | "Ratio"
                | "Annotation"
                | "Attachment"
                | "BackboneElement"
                | "Extension"
                | "Resource"
                | "DomainResource"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();
        let idx1 = graph.add_node("A".to_string());
        let idx2 = graph.add_node("B".to_string());
        let idx3 = graph.add_node("A".to_string()); // Duplicate

        assert_ne!(idx1, idx2);
        assert_eq!(idx1, idx3); // Should return existing
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = DependencyGraph::new();
        graph.add_edge("B", "A", DependencyType::Parent, 0..10);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        let deps = graph.get_dependencies("B");
        assert_eq!(deps, vec!["A"]);
    }

    #[test]
    fn test_simple_topological_sort() {
        let mut graph = DependencyGraph::new();

        // B depends on A, C depends on B
        // So processing order should be: A, B, C
        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "B", DependencyType::Parent, 10..20);

        let sorted = graph.topological_sort().unwrap();

        // A should come before B, B before C
        let a_pos = sorted.iter().position(|s| s == "A").unwrap();
        let b_pos = sorted.iter().position(|s| s == "B").unwrap();
        let c_pos = sorted.iter().position(|s| s == "C").unwrap();

        assert!(a_pos < b_pos, "A at {}, B at {}", a_pos, b_pos);
        assert!(b_pos < c_pos, "B at {}, C at {}", b_pos, c_pos);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();

        // A → B → C → A (cycle)
        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "B", DependencyType::Parent, 10..20);
        graph.add_edge("A", "C", DependencyType::Parent, 20..30);

        let result = graph.topological_sort();
        assert!(result.is_err());

        let cycles = graph.find_cycles();
        assert_eq!(cycles.len(), 1);
        assert!(cycles[0].contains(&"A".to_string()));
        assert!(cycles[0].contains(&"B".to_string()));
        assert!(cycles[0].contains(&"C".to_string()));
    }

    #[test]
    fn test_processing_batches() {
        let mut graph = DependencyGraph::new();

        // Level 0: A, D (no dependencies)
        // Level 1: B (depends on A), E (depends on D)
        // Level 2: C (depends on B)

        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "B", DependencyType::Parent, 10..20);
        graph.add_edge("E", "D", DependencyType::Parent, 20..30);

        let batches = graph.get_processing_batches();

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 2); // A, D
        assert_eq!(batches[1].len(), 2); // B, E
        assert_eq!(batches[2].len(), 1); // C
    }

    #[test]
    fn test_has_path() {
        let mut graph = DependencyGraph::new();

        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "B", DependencyType::Parent, 10..20);

        assert!(graph.has_path("C", "A"));
        assert!(graph.has_path("C", "B"));
        assert!(graph.has_path("B", "A"));
        assert!(!graph.has_path("A", "C"));
        assert!(!graph.has_path("A", "B"));
    }

    #[test]
    fn test_diamond_dependency() {
        let mut graph = DependencyGraph::new();

        //     A
        //    / \
        //   B   C
        //    \ /
        //     D

        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "A", DependencyType::Parent, 10..20);
        graph.add_edge("D", "B", DependencyType::Parent, 20..30);
        graph.add_edge("D", "C", DependencyType::Parent, 30..40);

        let sorted = graph.topological_sort().unwrap();

        // A must come before B and C
        // B and C must come before D
        let a_pos = sorted.iter().position(|s| s == "A").unwrap();
        let b_pos = sorted.iter().position(|s| s == "B").unwrap();
        let c_pos = sorted.iter().position(|s| s == "C").unwrap();
        let d_pos = sorted.iter().position(|s| s == "D").unwrap();

        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();

        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "A", DependencyType::Parent, 10..20);

        let dependents = graph.get_dependents("A");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&"B"));
        assert!(dependents.contains(&"C"));
    }

    #[test]
    fn test_strongly_connected_components() {
        let mut graph = DependencyGraph::new();

        // Create two separate SCCs
        // SCC 1: A → B → C → A
        graph.add_edge("B", "A", DependencyType::Parent, 0..10);
        graph.add_edge("C", "B", DependencyType::Parent, 10..20);
        graph.add_edge("A", "C", DependencyType::Parent, 20..30);

        // SCC 2: D (alone)
        graph.add_node("D".to_string());

        let sccs = graph.strongly_connected_components();
        assert_eq!(sccs.len(), 2);
    }
}
