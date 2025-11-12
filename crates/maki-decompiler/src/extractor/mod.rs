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

pub mod cardinality;
pub mod flags;
pub mod binding;
pub mod caret;
pub mod type_constraint;
pub mod assignment;
pub mod contains;
pub mod obeys;

use crate::{
    processor::ProcessableElementDefinition,
    exportable::ExportableRule,
    Result,
};

// Re-exports
pub use cardinality::*;
pub use flags::*;
pub use binding::*;
pub use caret::*;
pub use type_constraint::*;
pub use assignment::*;
pub use contains::*;
pub use obeys::*;

/// Trait for rule extractors
pub trait RuleExtractor {
    /// Extract rules from an ElementDefinition
    ///
    /// Returns a vector of rules that were extracted from this element.
    /// Marks properties as processed to avoid duplicate rule generation.
    fn extract(
        &self,
        elem: &mut ProcessableElementDefinition,
    ) -> Result<Vec<Box<dyn ExportableRule>>>;
}
