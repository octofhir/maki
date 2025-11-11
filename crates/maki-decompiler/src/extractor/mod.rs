//! Rule extractors convert ElementDefinition properties into FSH rules
//!
//! This module provides extractors for different rule types:
//! - CardinalityExtractor: min..max constraints
//! - FlagExtractor: MS, ?!, SU, D, N, TU flags
//! - BindingExtractor: ValueSet bindings
//! - CaretValueExtractor: Metadata (^short, ^definition, etc.)

pub mod cardinality;
pub mod flags;
pub mod binding;
pub mod caret;

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
