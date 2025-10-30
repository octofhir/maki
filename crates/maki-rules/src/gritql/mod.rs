//! GritQL integration for FSH linting - CST-based implementation
//!
//! This module provides full GritQL pattern matching support for FSH files
//! using our Rowan-based CST (Concrete Syntax Tree).
//!
//! # Architecture
//!
//! - **CST Adapter** (`cst_adapter`): Wraps Rowan nodes to implement GritQL's AstNode trait
//! - **CST Tree** (`cst_tree`): Implements GritQL's Ast trait over our CST
//! - **Language** (`cst_language`): Tells GritQL how to work with FSH syntax
//! - **Executor**: Compiles and executes GritQL patterns against FSH code
//! - **Built-ins**: FSH-specific functions for use in GritQL patterns
//!
//! # Example
//!
//! ```ignore
//! use maki_rules::gritql::{FshGritTree, GritQLCompiler};
//!
//! // Parse FSH to queryable tree
//! let tree = FshGritTree::parse("Profile: MyPatient\nParent: Patient");
//!
//! // Compile a pattern
//! let compiler = GritQLCompiler::new()?;
//! let pattern = compiler.compile_pattern(
//!     "Profile: $name where $name == `MyPatient`",
//!     "test-rule"
//! )?;
//!
//! // Execute and get matches
//! let matches = pattern.execute("Profile: MyPatient", "test.fsh")?;
//! ```

// CST-based implementation
pub mod cst_adapter;
pub mod cst_language;
pub mod cst_tree;

// QueryContext - bridges our CST with grit-pattern-matcher
pub mod query_context;

// GritQL parser and compiler
pub mod compiler;
pub mod parser;

// Executor and built-ins
pub mod builtins;
pub mod executor;

// Rule loading and registry
pub mod loader;
pub mod registry;

// Re-export main types
pub use builtins::register_fsh_builtins;
pub use cst_adapter::{FshGritCursor, FshGritNode};
pub use cst_language::FshTargetLanguage;
pub use cst_tree::FshGritTree;
pub use executor::{CompiledGritQLPattern, GritQLCompiler, GritQLMatch, MatchRange};
pub use loader::GritQLRuleLoader;
pub use registry::GritQLRegistry;
