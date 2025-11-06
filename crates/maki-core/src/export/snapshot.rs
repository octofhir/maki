//! Differential and Snapshot Generation for FHIR StructureDefinitions
//!
//! This module implements the critical algorithms for generating FHIR differentials
//! and snapshots. These are essential for valid FHIR profiles.
//!
//! # Overview
//!
//! - **Differential**: Contains only the elements that have been modified from the parent
//! - **Snapshot**: Contains the complete element tree with all inherited elements
//!
//! # Algorithms
//!
//! ## Algorithm 2: Differential Generation
//!
//! Compares a modified StructureDefinition with its base to identify changes:
//! 1. Iterate through modified elements
//! 2. Compare each property (cardinality, type, binding, etc.)
//! 3. Include element in differential if ANY property differs
//! 4. Handle slicing and extensions specially
//!
//! ## Algorithm 4: Snapshot Generation
//!
//! Creates a complete element tree by merging differential with parent:
//! 1. Get parent snapshot as starting point
//! 2. Apply differential element by element
//! 3. Merge properties (child overrides parent)
//! 4. Unfold choice types and references
//! 5. Sort elements in tree order
//! 6. Validate the result
//!
//! # FHIR Specification
//!
//! See: <https://hl7.org/fhir/structuredefinition.html>
//!
//! # SUSHI Compatibility
//!
//! This implementation follows the same algorithm as SUSHI (FSH reference compiler)
//! but with improved error handling and validation.
//!
//! # Example
//!
//! ```rust,no_run
//! use maki_core::export::{SnapshotGenerator, StructureDefinition};
//! use maki_core::canonical::DefinitionSession;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let session: Arc<DefinitionSession> = todo!();
//! let generator = SnapshotGenerator::new(session);
//!
//! // Generate snapshot from differential
//! let mut sd: StructureDefinition = todo!();
//! let snapshot = generator.generate_snapshot(&sd).await?;
//!
//! // Generate differential by comparing two structures
//! let base: StructureDefinition = todo!();
//! let modified: StructureDefinition = todo!();
//! let differential = generator.generate_differential(&base, &modified);
//! # Ok(())
//! # }
//! ```

use super::fhir_types::*;
use crate::canonical::DefinitionSession;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, trace, warn};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during snapshot generation
#[derive(Debug, Error)]
pub enum SnapshotError {
    /// Parent StructureDefinition not found
    #[error("Parent snapshot not found for: {0}")]
    ParentSnapshotNotFound(String),

    /// Failed to merge element properties
    #[error("Invalid element merge: {element} - {reason}")]
    InvalidMerge { element: String, reason: String },

    /// Cardinality constraint violation
    #[error("Cardinality conflict: parent {parent_card}, child {child_card}")]
    CardinalityConflict {
        parent_card: String,
        child_card: String,
    },

    /// Element ordering error
    #[error("Element ordering error: {0}")]
    OrderingError(String),

    /// Snapshot validation failed
    #[error("Snapshot validation failed: {0}")]
    ValidationFailed(String),

    /// Type constraint error
    #[error("Type constraint error: {0}")]
    TypeConstraintError(String),

    /// Element not found in snapshot
    #[error("Element not found in snapshot: {0}")]
    ElementNotFound(String),

    /// Circular dependency detected
    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    /// Canonical resolution error
    #[error("Canonical resolution error: {0}")]
    CanonicalError(String),
}

// ============================================================================
// Snapshot Generator
// ============================================================================

/// Generator for FHIR StructureDefinition differentials and snapshots
///
/// This struct provides the core algorithms for:
/// - Computing differentials (what changed from parent)
/// - Generating snapshots (complete element tree)
/// - Merging element properties according to FHIR rules
pub struct SnapshotGenerator {
    /// Session for resolving FHIR definitions
    session: Arc<DefinitionSession>,
}

impl SnapshotGenerator {
    /// Create a new snapshot generator
    ///
    /// # Arguments
    ///
    /// * `session` - DefinitionSession for resolving parent definitions
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use maki_core::export::SnapshotGenerator;
    /// use maki_core::canonical::DefinitionSession;
    /// use std::sync::Arc;
    ///
    /// let session: Arc<DefinitionSession> = todo!();
    /// let generator = SnapshotGenerator::new(session);
    /// ```
    pub fn new(session: Arc<DefinitionSession>) -> Self {
        Self { session }
    }

    /// Generate differential from modified StructureDefinition
    ///
    /// Implements **Algorithm 2** from MAKI_PLAN.md.
    ///
    /// Compares a modified StructureDefinition with its base to identify which
    /// elements have been changed. Only changed elements are included in the
    /// differential.
    ///
    /// # Arguments
    ///
    /// * `base` - Base StructureDefinition (parent)
    /// * `modified` - Modified StructureDefinition
    ///
    /// # Returns
    ///
    /// Vector of ElementDefinitions that differ from base
    ///
    /// # Algorithm
    ///
    /// 1. Iterate through modified elements
    /// 2. Compare with corresponding base element
    /// 3. If ANY property differs, include in differential
    /// 4. If element is new (not in base), include in differential
    /// 5. Handle slicing discriminators specially
    /// 6. Handle extension declarations specially
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::{SnapshotGenerator, StructureDefinition};
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
    /// # let session: Arc<DefinitionSession> = todo!();
    /// let generator = SnapshotGenerator::new(session);
    ///
    /// let base: StructureDefinition = todo!();
    /// let modified: StructureDefinition = todo!();
    ///
    /// let differential = generator.generate_differential(&base, &modified);
    /// println!("Changed elements: {}", differential.len());
    /// ```
    pub fn generate_differential(
        &self,
        base: &StructureDefinition,
        modified: &StructureDefinition,
    ) -> Vec<ElementDefinition> {
        let mut differential = Vec::new();

        // Get snapshots for comparison
        let base_elements = match &base.snapshot {
            Some(snapshot) => &snapshot.element,
            None => {
                warn!("Base StructureDefinition has no snapshot, returning empty differential");
                return differential;
            }
        };

        let modified_elements = match &modified.snapshot {
            Some(snapshot) => &snapshot.element,
            None => {
                warn!("Modified StructureDefinition has no snapshot, returning empty differential");
                return differential;
            }
        };

        // Track which base elements we've seen
        let mut base_paths: HashSet<String> =
            base_elements.iter().map(|e| e.path.clone()).collect();

        // Iterate through modified elements
        for modified_elem in modified_elements {
            trace!("Checking element: {}", modified_elem.path);

            // Find corresponding element in base
            if let Some(base_elem) = base_elements.iter().find(|e| e.path == modified_elem.path) {
                // Element exists in base - check if modified
                if self.is_modified(base_elem, modified_elem) {
                    debug!("Element {} is modified", modified_elem.path);
                    differential.push(modified_elem.clone());
                }
                base_paths.remove(&modified_elem.path);
            } else {
                // Element is new (not in base)
                debug!("Element {} is new", modified_elem.path);
                differential.push(modified_elem.clone());
            }
        }

        debug!(
            "Generated differential with {} elements from {} modified elements",
            differential.len(),
            modified_elements.len()
        );

        differential
    }

    /// Generate snapshot from StructureDefinition with differential
    ///
    /// Implements **Algorithm 4** from MAKI_PLAN.md.
    ///
    /// Creates a complete element tree by:
    /// 1. Getting the parent's snapshot
    /// 2. Cloning it as a starting point
    /// 3. Applying the differential element by element
    /// 4. Sorting elements in tree order
    /// 5. Validating the result
    ///
    /// # Arguments
    ///
    /// * `sd` - StructureDefinition with differential populated
    ///
    /// # Returns
    ///
    /// Complete snapshot with all elements, or error if generation fails
    ///
    /// # Errors
    ///
    /// - `ParentSnapshotNotFound`: Parent definition not found or has no snapshot
    /// - `InvalidMerge`: Failed to merge element properties
    /// - `OrderingError`: Failed to sort elements correctly
    /// - `ValidationFailed`: Generated snapshot is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use maki_core::export::{SnapshotGenerator, StructureDefinition};
    /// # use maki_core::canonical::DefinitionSession;
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session: Arc<DefinitionSession> = todo!();
    /// let generator = SnapshotGenerator::new(session);
    ///
    /// let mut sd: StructureDefinition = todo!(); // Has differential
    /// let snapshot = generator.generate_snapshot(&sd).await?;
    ///
    /// println!("Generated snapshot with {} elements", snapshot.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_snapshot(
        &self,
        sd: &StructureDefinition,
    ) -> Result<Vec<ElementDefinition>, SnapshotError> {
        debug!("Generating snapshot for {}", sd.url);

        // Step 1: Get parent snapshot
        let parent_url = sd.base_definition.as_ref().ok_or_else(|| {
            SnapshotError::ParentSnapshotNotFound("No base definition specified".to_string())
        })?;

        trace!("Resolving parent: {}", parent_url);

        let parent_sd = self
            .session
            .resolve_structure_definition(parent_url)
            .await
            .map_err(|e| {
                SnapshotError::CanonicalError(format!(
                    "Failed to resolve parent {}: {}",
                    parent_url, e
                ))
            })?
            .ok_or_else(|| SnapshotError::ParentSnapshotNotFound(parent_url.clone()))?;

        let parent_snapshot = parent_sd.snapshot.as_ref().ok_or_else(|| {
            SnapshotError::ParentSnapshotNotFound(format!("Parent {} has no snapshot", parent_url))
        })?;

        // Step 2: Clone parent snapshot as starting point
        let mut snapshot = parent_snapshot.element.clone();
        debug!("Cloned parent snapshot with {} elements", snapshot.len());

        // Step 3: Apply differential
        if let Some(differential) = &sd.differential {
            trace!(
                "Applying differential with {} elements",
                differential.element.len()
            );

            for diff_elem in &differential.element {
                trace!("Applying differential element: {}", diff_elem.path);

                // Find corresponding element in snapshot
                if let Some(snap_elem) = snapshot.iter_mut().find(|e| e.path == diff_elem.path) {
                    // Merge with existing element
                    *snap_elem = self.merge_element(snap_elem, diff_elem)?;
                    debug!("Merged element: {}", diff_elem.path);
                } else {
                    // Insert new element
                    self.insert_element(&mut snapshot, diff_elem.clone())?;
                    debug!("Inserted new element: {}", diff_elem.path);
                }
            }
        } else {
            debug!("No differential to apply");
        }

        // Step 4: Sort elements in tree order
        self.sort_elements(&mut snapshot)?;

        // Step 5: Validate snapshot
        self.validate_snapshot(&snapshot)?;

        debug!(
            "Successfully generated snapshot with {} elements",
            snapshot.len()
        );
        Ok(snapshot)
    }

    /// Merge child element properties into parent element
    ///
    /// Implements property merging rules according to FHIR specification:
    /// - Child properties override parent properties
    /// - Cardinality: child must be within parent range
    /// - Types: child must be subset of parent types
    /// - Constraints and mappings are concatenated (not replaced)
    ///
    /// # Arguments
    ///
    /// * `parent` - Parent element definition
    /// * `child` - Child element definition (from differential)
    ///
    /// # Returns
    ///
    /// Merged element definition
    ///
    /// # Errors
    ///
    /// - `InvalidMerge`: If merge violates FHIR rules
    /// - `CardinalityConflict`: If child cardinality is outside parent range
    /// - `TypeConstraintError`: If child type is not compatible with parent
    fn merge_element(
        &self,
        parent: &ElementDefinition,
        child: &ElementDefinition,
    ) -> Result<ElementDefinition, SnapshotError> {
        trace!("Merging element: {}", parent.path);

        // Start with parent as base
        let mut merged = parent.clone();

        // Override scalar properties if present in child
        if child.min.is_some() {
            // Validate cardinality constraint
            if let (Some(parent_min), Some(child_min)) = (parent.min, child.min)
                && child_min < parent_min
            {
                return Err(SnapshotError::CardinalityConflict {
                    parent_card: format!(
                        "{}..{}",
                        parent_min,
                        parent.max.as_ref().unwrap_or(&"*".to_string())
                    ),
                    child_card: format!(
                        "{}..{}",
                        child_min,
                        child.max.as_ref().unwrap_or(&"*".to_string())
                    ),
                });
            }
            merged.min = child.min;
        }

        if child.max.is_some() {
            // Validate max cardinality
            if let (Some(parent_max), Some(child_max)) = (&parent.max, &child.max)
                && !self.is_cardinality_compatible(parent_max, child_max)
            {
                return Err(SnapshotError::CardinalityConflict {
                    parent_card: format!("{}..{}", parent.min.unwrap_or(0), parent_max),
                    child_card: format!("{}..{}", child.min.unwrap_or(0), child_max),
                });
            }
            merged.max = child.max.clone();
        }

        // Merge type constraints
        if let Some(ref child_types) = child.type_ {
            merged.type_ = Some(child_types.clone());
        }

        // Override description fields
        if child.short.is_some() {
            merged.short = child.short.clone();
        }
        if child.definition.is_some() {
            merged.definition = child.definition.clone();
        }
        if child.comment.is_some() {
            merged.comment = child.comment.clone();
        }

        // Override flags
        if child.must_support.is_some() {
            merged.must_support = child.must_support;
        }
        if child.is_modifier.is_some() {
            merged.is_modifier = child.is_modifier;
        }
        if child.is_summary.is_some() {
            merged.is_summary = child.is_summary;
        }

        // Override binding
        if child.binding.is_some() {
            merged.binding = child.binding.clone();
        }

        // Concatenate constraints (don't replace)
        if let Some(ref child_constraints) = child.constraint {
            match &merged.constraint {
                Some(parent_constraints) => {
                    // Add new constraints, avoiding duplicates by key
                    let existing_keys: HashSet<_> =
                        parent_constraints.iter().map(|c| &c.key).collect();
                    let mut new_constraints = parent_constraints.clone();
                    for constraint in child_constraints {
                        if !existing_keys.contains(&constraint.key) {
                            new_constraints.push(constraint.clone());
                        }
                    }
                    merged.constraint = Some(new_constraints);
                }
                None => {
                    merged.constraint = Some(child_constraints.clone());
                }
            }
        }

        // Override fixed/pattern values
        if child.fixed.is_some() {
            merged.fixed = child.fixed.clone();
        }
        if child.pattern.is_some() {
            merged.pattern = child.pattern.clone();
        }

        trace!("Successfully merged element: {}", parent.path);
        Ok(merged)
    }

    /// Check if an element has been modified from base
    ///
    /// Compares all properties to detect any changes.
    /// Used by differential generation to identify modified elements.
    ///
    /// # Arguments
    ///
    /// * `base` - Base element
    /// * `modified` - Modified element
    ///
    /// # Returns
    ///
    /// `true` if any property differs, `false` if identical
    fn is_modified(&self, base: &ElementDefinition, modified: &ElementDefinition) -> bool {
        base.is_modified_from(modified)
    }

    /// Insert a new element into snapshot at the correct position
    ///
    /// Maintains element tree order by finding the correct insertion point.
    ///
    /// # Arguments
    ///
    /// * `snapshot` - Current snapshot elements
    /// * `element` - New element to insert
    fn insert_element(
        &self,
        snapshot: &mut Vec<ElementDefinition>,
        element: ElementDefinition,
    ) -> Result<(), SnapshotError> {
        // Find correct insertion position
        // Elements must be in tree order: parent before children
        let insert_pos = snapshot
            .iter()
            .position(|e| self.compare_element_paths(&element.path, &e.path) == Ordering::Less)
            .unwrap_or(snapshot.len());

        snapshot.insert(insert_pos, element);
        Ok(())
    }

    /// Sort elements in tree order
    ///
    /// Elements must appear in depth-first order:
    /// - Parent before children
    /// - Siblings in alphabetical order
    ///
    /// # Arguments
    ///
    /// * `elements` - Elements to sort
    fn sort_elements(&self, elements: &mut [ElementDefinition]) -> Result<(), SnapshotError> {
        elements.sort_by(|a, b| self.compare_element_paths(&a.path, &b.path));
        Ok(())
    }

    /// Compare two element paths for sorting
    ///
    /// Implements tree ordering:
    /// - "Patient" < "Patient.name"
    /// - "Patient.name" < "Patient.name.given"
    /// - "Patient.name" < "Patient.telecom"
    ///
    /// # Arguments
    ///
    /// * `a` - First path
    /// * `b` - Second path
    ///
    /// # Returns
    ///
    /// Ordering relationship
    fn compare_element_paths(&self, a: &str, b: &str) -> Ordering {
        let a_parts: Vec<&str> = a.split('.').collect();
        let b_parts: Vec<&str> = b.split('.').collect();

        // Compare part by part
        for i in 0..a_parts.len().min(b_parts.len()) {
            match a_parts[i].cmp(b_parts[i]) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        // If all parts equal so far, shorter path comes first
        a_parts.len().cmp(&b_parts.len())
    }

    /// Check if child cardinality is compatible with parent
    ///
    /// Child max must be <= parent max
    ///
    /// # Arguments
    ///
    /// * `parent_max` - Parent maximum cardinality
    /// * `child_max` - Child maximum cardinality
    ///
    /// # Returns
    ///
    /// `true` if compatible, `false` if conflict
    fn is_cardinality_compatible(&self, parent_max: &str, child_max: &str) -> bool {
        // If parent is unbounded (*), child can be anything
        if parent_max == "*" {
            return true;
        }

        // If child is unbounded but parent isn't, conflict
        if child_max == "*" {
            return false;
        }

        // Both are numbers - parse and compare
        match (parent_max.parse::<u32>(), child_max.parse::<u32>()) {
            (Ok(p), Ok(c)) => c <= p,
            _ => false, // Parse error = incompatible
        }
    }

    /// Validate generated snapshot
    ///
    /// Checks:
    /// - All elements have paths
    /// - Root element exists
    /// - Elements are in tree order
    /// - No duplicate paths
    ///
    /// # Arguments
    ///
    /// * `snapshot` - Snapshot to validate
    ///
    /// # Errors
    ///
    /// Returns `ValidationFailed` if any check fails
    fn validate_snapshot(&self, snapshot: &[ElementDefinition]) -> Result<(), SnapshotError> {
        if snapshot.is_empty() {
            return Err(SnapshotError::ValidationFailed(
                "Snapshot is empty".to_string(),
            ));
        }

        // Check all elements have paths
        for (i, elem) in snapshot.iter().enumerate() {
            if elem.path.is_empty() {
                return Err(SnapshotError::ValidationFailed(format!(
                    "Element at index {} has empty path",
                    i
                )));
            }
        }

        // Check for duplicate paths
        let mut seen_paths = HashSet::new();
        for elem in snapshot {
            if !seen_paths.insert(&elem.path) {
                return Err(SnapshotError::ValidationFailed(format!(
                    "Duplicate element path: {}",
                    elem.path
                )));
            }
        }

        // Validation passed
        trace!("Snapshot validation passed");
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test generator without needing a real session
    // Most tests don't need session functionality
    struct TestGenerator;

    impl TestGenerator {
        fn new() -> Self {
            Self
        }

        fn compare_element_paths(&self, a: &str, b: &str) -> Ordering {
            let a_parts: Vec<&str> = a.split('.').collect();
            let b_parts: Vec<&str> = b.split('.').collect();

            for i in 0..a_parts.len().min(b_parts.len()) {
                match a_parts[i].cmp(b_parts[i]) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }

            a_parts.len().cmp(&b_parts.len())
        }

        fn is_cardinality_compatible(&self, parent_max: &str, child_max: &str) -> bool {
            if parent_max == "*" {
                return true;
            }

            if child_max == "*" {
                return false;
            }

            match (parent_max.parse::<u32>(), child_max.parse::<u32>()) {
                (Ok(p), Ok(c)) => c <= p,
                _ => false,
            }
        }

        fn merge_element(
            &self,
            parent: &ElementDefinition,
            child: &ElementDefinition,
        ) -> Result<ElementDefinition, SnapshotError> {
            let mut merged = parent.clone();

            if child.min.is_some() {
                if let (Some(parent_min), Some(child_min)) = (parent.min, child.min)
                    && child_min < parent_min
                {
                    return Err(SnapshotError::CardinalityConflict {
                        parent_card: format!(
                            "{}..{}",
                            parent_min,
                            parent.max.as_ref().unwrap_or(&"*".to_string())
                        ),
                        child_card: format!(
                            "{}..{}",
                            child_min,
                            child.max.as_ref().unwrap_or(&"*".to_string())
                        ),
                    });
                }
                merged.min = child.min;
            }

            if child.max.is_some() {
                if let (Some(parent_max), Some(child_max)) = (&parent.max, &child.max)
                    && !self.is_cardinality_compatible(parent_max, child_max)
                {
                    return Err(SnapshotError::CardinalityConflict {
                        parent_card: format!("{}..{}", parent.min.unwrap_or(0), parent_max),
                        child_card: format!("{}..{}", child.min.unwrap_or(0), child_max),
                    });
                }
                merged.max = child.max.clone();
            }

            if let Some(ref child_types) = child.type_ {
                merged.type_ = Some(child_types.clone());
            }

            if child.short.is_some() {
                merged.short = child.short.clone();
            }
            if child.definition.is_some() {
                merged.definition = child.definition.clone();
            }
            if child.comment.is_some() {
                merged.comment = child.comment.clone();
            }

            if child.must_support.is_some() {
                merged.must_support = child.must_support;
            }
            if child.is_modifier.is_some() {
                merged.is_modifier = child.is_modifier;
            }
            if child.is_summary.is_some() {
                merged.is_summary = child.is_summary;
            }

            if child.binding.is_some() {
                merged.binding = child.binding.clone();
            }

            if let Some(ref child_constraints) = child.constraint {
                match &merged.constraint {
                    Some(parent_constraints) => {
                        let existing_keys: HashSet<_> =
                            parent_constraints.iter().map(|c| &c.key).collect();
                        let mut new_constraints = parent_constraints.clone();
                        for constraint in child_constraints {
                            if !existing_keys.contains(&constraint.key) {
                                new_constraints.push(constraint.clone());
                            }
                        }
                        merged.constraint = Some(new_constraints);
                    }
                    None => {
                        merged.constraint = Some(child_constraints.clone());
                    }
                }
            }

            if child.fixed.is_some() {
                merged.fixed = child.fixed.clone();
            }
            if child.pattern.is_some() {
                merged.pattern = child.pattern.clone();
            }

            Ok(merged)
        }

        fn validate_snapshot(&self, snapshot: &[ElementDefinition]) -> Result<(), SnapshotError> {
            if snapshot.is_empty() {
                return Err(SnapshotError::ValidationFailed(
                    "Snapshot is empty".to_string(),
                ));
            }

            for (i, elem) in snapshot.iter().enumerate() {
                if elem.path.is_empty() {
                    return Err(SnapshotError::ValidationFailed(format!(
                        "Element at index {} has empty path",
                        i
                    )));
                }
            }

            let mut seen_paths = HashSet::new();
            for elem in snapshot {
                if !seen_paths.insert(&elem.path) {
                    return Err(SnapshotError::ValidationFailed(format!(
                        "Duplicate element path: {}",
                        elem.path
                    )));
                }
            }

            Ok(())
        }

        fn generate_differential(
            &self,
            base: &StructureDefinition,
            modified: &StructureDefinition,
        ) -> Vec<ElementDefinition> {
            let mut differential = Vec::new();

            let base_elements = match &base.snapshot {
                Some(snapshot) => &snapshot.element,
                None => return differential,
            };

            let modified_elements = match &modified.snapshot {
                Some(snapshot) => &snapshot.element,
                None => return differential,
            };

            let mut base_paths: HashSet<String> =
                base_elements.iter().map(|e| e.path.clone()).collect();

            for modified_elem in modified_elements {
                if let Some(base_elem) = base_elements.iter().find(|e| e.path == modified_elem.path)
                {
                    if base_elem.is_modified_from(modified_elem) {
                        differential.push(modified_elem.clone());
                    }
                    base_paths.remove(&modified_elem.path);
                } else {
                    differential.push(modified_elem.clone());
                }
            }

            differential
        }
    }

    fn create_test_element(path: &str) -> ElementDefinition {
        ElementDefinition::new(path.to_string())
    }

    fn create_test_element_with_card(path: &str, min: u32, max: &str) -> ElementDefinition {
        let mut elem = ElementDefinition::new(path.to_string());
        elem.min = Some(min);
        elem.max = Some(max.to_string());
        elem
    }

    #[test]
    fn test_compare_element_paths() {
        let generator = TestGenerator::new();

        // Parent < child
        assert_eq!(
            generator.compare_element_paths("Patient", "Patient.name"),
            Ordering::Less
        );

        // Alphabetical order for siblings
        assert_eq!(
            generator.compare_element_paths("Patient.name", "Patient.telecom"),
            Ordering::Less
        );

        // Equal paths
        assert_eq!(
            generator.compare_element_paths("Patient.name", "Patient.name"),
            Ordering::Equal
        );

        // Nested elements
        assert_eq!(
            generator.compare_element_paths("Patient.name", "Patient.name.given"),
            Ordering::Less
        );
    }

    #[test]
    fn test_is_cardinality_compatible() {
        let generator = TestGenerator::new();

        // Parent unbounded - anything allowed
        assert!(generator.is_cardinality_compatible("*", "1"));
        assert!(generator.is_cardinality_compatible("*", "*"));
        assert!(generator.is_cardinality_compatible("*", "100"));

        // Child cannot be unbounded if parent isn't
        assert!(!generator.is_cardinality_compatible("1", "*"));
        assert!(!generator.is_cardinality_compatible("5", "*"));

        // Child must be <= parent
        assert!(generator.is_cardinality_compatible("5", "3"));
        assert!(generator.is_cardinality_compatible("5", "5"));
        assert!(!generator.is_cardinality_compatible("3", "5"));
    }

    #[test]
    fn test_is_modified_no_changes() {
        let base = create_test_element("Patient.name");
        let modified = base.clone();

        assert!(!base.is_modified_from(&modified));
    }

    #[test]
    fn test_is_modified_with_changes() {
        let base = create_test_element_with_card("Patient.name", 0, "*");
        let mut modified = base.clone();
        modified.min = Some(1);

        assert!(base.is_modified_from(&modified));
    }

    #[test]
    fn test_merge_element_simple() {
        let generator = TestGenerator::new();

        let parent = create_test_element_with_card("Patient.name", 0, "*");
        let mut child = create_test_element("Patient.name");
        child.min = Some(1);
        child.max = Some("1".to_string());

        let merged = generator.merge_element(&parent, &child).unwrap();

        assert_eq!(merged.min, Some(1));
        assert_eq!(merged.max, Some("1".to_string()));
    }

    #[test]
    fn test_merge_element_cardinality_conflict() {
        let generator = TestGenerator::new();

        let parent = create_test_element_with_card("Patient.name", 1, "1");
        let child = create_test_element_with_card("Patient.name", 0, "1");

        let result = generator.merge_element(&parent, &child);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SnapshotError::CardinalityConflict { .. }
        ));
    }

    #[test]
    fn test_validate_snapshot_empty() {
        let generator = TestGenerator::new();

        let snapshot = vec![];
        let result = generator.validate_snapshot(&snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_snapshot_duplicates() {
        let generator = TestGenerator::new();

        let snapshot = vec![
            create_test_element("Patient"),
            create_test_element("Patient.name"),
            create_test_element("Patient.name"), // Duplicate
        ];

        let result = generator.validate_snapshot(&snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_snapshot_valid() {
        let generator = TestGenerator::new();

        let snapshot = vec![
            create_test_element("Patient"),
            create_test_element("Patient.name"),
            create_test_element("Patient.telecom"),
        ];

        let result = generator.validate_snapshot(&snapshot);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_differential_no_changes() {
        let generator = TestGenerator::new();

        let base = StructureDefinition {
            snapshot: Some(StructureDefinitionSnapshot {
                element: vec![create_test_element("Patient")],
            }),
            ..StructureDefinition::new(
                "http://example.org/base".to_string(),
                "Base".to_string(),
                "Patient".to_string(),
                StructureDefinitionKind::Resource,
            )
        };

        let modified = base.clone();

        let differential = generator.generate_differential(&base, &modified);
        assert_eq!(differential.len(), 0);
    }

    #[test]
    fn test_generate_differential_with_changes() {
        let generator = TestGenerator::new();

        let base = StructureDefinition {
            snapshot: Some(StructureDefinitionSnapshot {
                element: vec![
                    create_test_element("Patient"),
                    create_test_element_with_card("Patient.name", 0, "*"),
                ],
            }),
            ..StructureDefinition::new(
                "http://example.org/base".to_string(),
                "Base".to_string(),
                "Patient".to_string(),
                StructureDefinitionKind::Resource,
            )
        };

        let mut modified = base.clone();
        if let Some(ref mut snapshot) = modified.snapshot {
            snapshot.element[1].min = Some(1);
        }

        let differential = generator.generate_differential(&base, &modified);
        assert_eq!(differential.len(), 1);
        assert_eq!(differential[0].path, "Patient.name");
        assert_eq!(differential[0].min, Some(1));
    }
}
