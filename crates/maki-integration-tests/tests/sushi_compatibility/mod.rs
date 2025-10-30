//! SUSHI Compatibility Test Suite
//!
//! This module provides infrastructure for testing MAKI's compatibility
//! with SUSHI (the reference FSH compiler).
//!
//! # Purpose
//!
//! - Ensure MAKI produces identical or equivalent output to SUSHI
//! - Track compatibility percentage over time
//! - Identify regressions quickly
//! - Validate against real-world Implementation Guides
//!
//! # Usage
//!
//! ```bash
//! # Run all compatibility tests (requires SUSHI installed)
//! cargo test --test sushi_compatibility -- --ignored
//!
//! # Run without SUSHI (skips comparison)
//! cargo test --test sushi_compatibility
//! ```

pub mod comparator;
pub mod runner;

pub use comparator::{Difference, compare_json, format_differences};
pub use runner::{ComparisonResult, SushiCompatibilityHarness, TestCase};
