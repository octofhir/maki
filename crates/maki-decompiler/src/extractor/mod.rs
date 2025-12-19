//! Rule extractors convert ElementDefinition properties into FSH rules
//!
//! This module provides extractors for different rule types:
//! - CardinalityExtractor: min..max constraints
//! - FlagExtractor: MS, ?!, SU, D, N, TU flags
//! - BindingExtractor: ValueSet bindings
//! - CaretValueExtractor: Metadata (^short, ^definition, etc.)
//! - TypeExtractor: Type constraints (only rules)
//! - AssignmentExtractor: fixed[x] and pattern[x] values
//! - ContainsExtractor: Slicing definitions
//! - ObeysExtractor: Invariants/constraints

pub mod assignment;
pub mod binding;
pub mod cardinality;
pub mod caret;
pub mod contains;
pub mod flags;
pub mod obeys;
pub mod type_constraint;

use crate::{Result, exportable::ExportableRule, processor::ProcessableElementDefinition};

// Re-exports
pub use assignment::*;
pub use binding::*;
pub use cardinality::*;
pub use caret::*;
pub use contains::*;
pub use flags::*;
pub use obeys::*;
pub use type_constraint::*;

/// Trait for rule extractors
pub trait RuleExtractor {
    /// Extract rules from an ElementDefinition
    ///
    /// Returns a vector of rules that were extracted from this element.
    /// Marks properties as processed to avoid duplicate rule generation.
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule + Send + Sync>>>;
}
