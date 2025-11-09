//! Concrete Syntax Tree (CST) for FHIR Shorthand
//!
//! This module implements a lossless syntax tree using the Rowan library.
//! The CST preserves all source information including whitespace, comments,
//! and formatting, enabling:
//! - Accurate source-to-source transformations (formatter)
//! - Precise autofixes that preserve formatting
//! - Better error recovery and diagnostics
//!
//! ## Architecture
//!
//! The CST uses Rowan's green/red tree pattern:
//!
//! - **Green Tree**: Immutable, position-independent storage
//!   - Stores actual source text with trivia (whitespace, comments)
//!   - Deduplicates identical subtrees for memory efficiency
//!   - Cheap to clone (uses Arc internally)
//!
//! - **Red Tree**: Dynamically constructed view with parent pointers
//!   - Created on-demand for traversal
//!   - Provides typed AST-like API
//!   - Enables efficient parent/sibling navigation
//!
//! ## Trivia Handling
//!
//! Trivia is attached to tokens:
//! - **Leading trivia**: Everything before a token (whitespace, comments)
//! - **Trailing trivia**: Everything until the next line break
//!
//! This enables lossless representation: `parse(source).text() == source`
//!
//! ## Example
//!
//! ```rust,ignore
//! use maki_core::cst::{FshSyntaxNode, FshSyntaxKind};
//!
//! // Parse FSH to CST
//! let (cst, _lexer_errors, errors) = parse_fsh("Profile: MyPatient // comment\nParent: Patient");
//!
//! // Verify lossless property
//! assert_eq!(cst.text().to_string(), "Profile: MyPatient // comment\nParent: Patient");
//!
//! // Navigate CST
//! for token in cst.descendants_with_tokens() {
//!     if let Some(comment) = token.as_token() {
//!         if comment.kind() == FshSyntaxKind::CommentLine {
//!             println!("Found comment: {}", comment.text());
//!         }
//!     }
//! }
//! ```

mod builder;
mod language;
mod lexer;
mod nodes;
mod parser;
mod syntax_kind;

pub mod ast;
pub mod format_element;
pub mod formatter;
pub mod formatter_v2;
pub mod incremental;
pub mod printer;
pub mod round_trip;
pub mod trivia;

pub use builder::{CstBuilder, build_cst_from_tokens, parse_fsh_simple};
pub use formatter::{FormatOptions, format_document};
pub use incremental::{EditUtils, IncrementalUpdater, TextEdit, UpdateMetrics, UpdateResult};
pub use language::FshLanguage;
pub use lexer::{CstLexResult, CstToken, LexerError, lex_with_trivia};
pub use nodes::*;
pub use parser::{ParseError, ParseErrorKind, parse_fsh};
pub use round_trip::{DifferenceKind, RoundTripValidator, SemanticDifference, ValidationResult};
pub use syntax_kind::FshSyntaxKind;
pub use trivia::{TriviaCollector, TriviaFormatter, TriviaInfo, TriviaPreserver, TriviaToken};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod golden_tests;

#[cfg(test)]
mod ast_tests;

#[cfg(test)]
mod formatter_insta_tests;
#[cfg(test)]
mod formatter_snapshot_tests;
