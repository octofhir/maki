//! Integration tests for dependency graph construction and analysis

use maki_core::cst::parse_fsh;
use maki_core::semantic::dependency_graph::{
    DependencyAnalyzer, DependencyError, DependencyGraph, DependencyType,
};
use maki_core::semantic::{DefaultSemanticAnalyzer, SemanticAnalyzer};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_simple_profile_hierarchy() {
    let source = r#"
        Profile: GrandparentProfile
        Parent: Patient

        Profile: ParentProfile
        Parent: GrandparentProfile

        Profile: ChildProfile
        Parent: ParentProfile
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let sorted = graph.topological_sort().unwrap();

    // Find positions
    let gp_idx = sorted
        .iter()
        .position(|s| s == "GrandparentProfile")
        .unwrap();
    let p_idx = sorted.iter().position(|s| s == "ParentProfile").unwrap();
    let c_idx = sorted.iter().position(|s| s == "ChildProfile").unwrap();

    // Verify order: GrandparentProfile before ParentProfile before ChildProfile
    assert!(gp_idx < p_idx);
    assert!(p_idx < c_idx);
}

#[test]
fn test_circular_dependency_detection() {
    let source = r#"
        Profile: ProfileA
        Parent: ProfileB

        Profile: ProfileB
        Parent: ProfileA
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let result = graph.topological_sort();
    assert!(result.is_err());

    if let Err(DependencyError::CircularDependency { cycle }) = result {
        assert!(cycle.contains(&"ProfileA".to_string()));
        assert!(cycle.contains(&"ProfileB".to_string()));
    } else {
        panic!("Expected CircularDependency error");
    }
}

#[test]
fn test_multiple_independent_profiles() {
    let source = r#"
        Profile: Profile1
        Parent: Patient

        Profile: Profile2
        Parent: Observation

        Profile: Profile3
        Parent: Practitioner
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let sorted = graph.topological_sort().unwrap();

    // All three profiles should be in the sorted list
    assert!(sorted.contains(&"Profile1".to_string()));
    assert!(sorted.contains(&"Profile2".to_string()));
    assert!(sorted.contains(&"Profile3".to_string()));
}

#[test]
fn test_processing_batches_with_real_profiles() {
    let source = r#"
        Profile: BaseProfile1
        Parent: Patient

        Profile: BaseProfile2
        Parent: Observation

        Profile: DerivedProfile1
        Parent: BaseProfile1

        Profile: DerivedProfile2
        Parent: BaseProfile2

        Profile: ChildProfile
        Parent: DerivedProfile1
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let batches = graph.get_processing_batches();

    // Should have multiple levels
    assert!(batches.len() >= 3);

    // Helper to find which batch a profile is in
    let find_batch = |name: &str| {
        batches
            .iter()
            .position(|batch| batch.contains(&name.to_string()))
            .unwrap()
    };

    // Verify dependency ordering: parents come before children in batches
    let base1_batch = find_batch("BaseProfile1");
    let base2_batch = find_batch("BaseProfile2");
    let derived1_batch = find_batch("DerivedProfile1");
    let derived2_batch = find_batch("DerivedProfile2");
    let child_batch = find_batch("ChildProfile");

    // BaseProfiles should come before DerivedProfiles
    assert!(base1_batch < derived1_batch);
    assert!(base2_batch < derived2_batch);
    // DerivedProfile1 should come before ChildProfile
    assert!(derived1_batch < child_batch);
}

#[test]
fn test_diamond_dependency_pattern() {
    let source = r#"
        Profile: Root
        Parent: Patient

        Profile: Left
        Parent: Root

        Profile: Right
        Parent: Root

        Profile: Bottom
        Parent: Left
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let sorted = graph.topological_sort().unwrap();

    // Root must come before Left and Right
    // Left and Right must come before Bottom
    let root_idx = sorted.iter().position(|s| s == "Root").unwrap();
    let left_idx = sorted.iter().position(|s| s == "Left").unwrap();
    let right_idx = sorted.iter().position(|s| s == "Right").unwrap();
    let bottom_idx = sorted.iter().position(|s| s == "Bottom").unwrap();

    assert!(root_idx < left_idx);
    assert!(root_idx < right_idx);
    assert!(left_idx < bottom_idx);
}

#[test]
fn test_value_set_and_code_system() {
    let source = r#"
        ValueSet: MyValueSet
        Title: "My Value Set"

        CodeSystem: MyCodeSystem
        Title: "My Code System"

        Profile: MyProfile
        Parent: Patient
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    // At least 3 nodes (our definitions) - may have more if FHIR base types included
    assert!(graph.node_count() >= 3);

    let sorted = graph.topological_sort().unwrap();
    assert!(sorted.contains(&"MyValueSet".to_string()));
    assert!(sorted.contains(&"MyCodeSystem".to_string()));
    assert!(sorted.contains(&"MyProfile".to_string()));
}

#[test]
fn test_has_path_queries() {
    let source = r#"
        Profile: A
        Parent: Patient

        Profile: B
        Parent: A

        Profile: C
        Parent: B

        Profile: D
        Parent: Patient
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    // Test path queries
    assert!(graph.has_path("C", "A"));
    assert!(graph.has_path("C", "B"));
    assert!(graph.has_path("B", "A"));
    assert!(!graph.has_path("A", "C"));
    assert!(!graph.has_path("A", "D"));
    assert!(!graph.has_path("D", "A"));
}

#[test]
fn test_get_dependencies_and_dependents() {
    let source = r#"
        Profile: Parent1
        Parent: Patient

        Profile: Child1
        Parent: Parent1

        Profile: Child2
        Parent: Parent1
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    // Child1 depends on Parent1
    let deps = graph.get_dependencies("Child1");
    assert_eq!(deps.len(), 1);
    assert!(deps.contains(&"Parent1"));

    // Parent1 has two dependents
    let dependents = graph.get_dependents("Parent1");
    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"Child1"));
    assert!(dependents.contains(&"Child2"));
}

#[test]
fn test_empty_graph() {
    let graph = DependencyGraph::new();
    let sorted = graph.topological_sort().unwrap();
    assert!(sorted.is_empty());

    let batches = graph.get_processing_batches();
    assert!(batches.is_empty());
}

#[test]
fn test_single_node_graph() {
    let mut graph = DependencyGraph::new();
    graph.add_node("OnlyNode".to_string());

    let sorted = graph.topological_sort().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0], "OnlyNode");

    let batches = graph.get_processing_batches();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].len(), 1);
}

#[test]
fn test_to_dot_output() {
    let mut graph = DependencyGraph::new();
    graph.add_edge("B", "A", DependencyType::Parent, 0..10);
    graph.add_edge("C", "B", DependencyType::Parent, 10..20);

    let dot = graph.to_dot();
    assert!(dot.contains("digraph"));
    assert!(!dot.is_empty());
}

#[test]
fn test_strongly_connected_components() {
    let source = r#"
        Profile: A
        Parent: B

        Profile: B
        Parent: C

        Profile: C
        Parent: A

        Profile: D
        Parent: Patient
    "#;

    let (cst, _errors) = parse_fsh(source);
    let analyzer = DefaultSemanticAnalyzer::new();
    let model = analyzer
        .analyze(&cst, source, PathBuf::from("test.fsh"))
        .unwrap();

    let dep_analyzer = DependencyAnalyzer::new(Arc::new(model));
    let graph = dep_analyzer.build_graph().unwrap();

    let sccs = graph.strongly_connected_components();

    // Should have 2 SCCs: one with {A, B, C}, one with {D}
    assert!(sccs.len() >= 2);

    // Find the SCC with A, B, C
    let cycle_scc = sccs.iter().find(|scc| scc.len() == 3);
    assert!(cycle_scc.is_some());

    let cycle = cycle_scc.unwrap();
    assert!(cycle.contains(&"A".to_string()));
    assert!(cycle.contains(&"B".to_string()));
    assert!(cycle.contains(&"C".to_string()));
}
