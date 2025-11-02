//! FSH Rule Application Module
//!
//! This module implements the application of FSH rules to FHIR StructureDefinitions.
//! It handles structural rules (cardinality, flags, types, slices) and value rules
//! (assignments, bindings).

pub mod structural;
pub mod value;

pub use structural::*;
pub use value::*;
