//! Configuration system for fsh-lint-rs
//!
//! This module provides a comprehensive configuration management system with:
//! - JSON/JSONC configuration file support
//! - Auto-discovery by traversing up directories
//! - Configuration extension/inheritance (`extends` field)
//! - Strong typing with serde and JSON Schema generation via schemars
//!
//! ## Configuration Files
//!
//! The linter supports two file formats:
//! - `fsh-lint.json` - Standard JSON
//! - `fsh-lint.jsonc` - JSON with comments and trailing commas (preferred)
//!
//! ## Configuration Discovery
//!
//! When no explicit config path is provided, the linter will search for
//! configuration files starting from the current directory and moving up
//! the directory tree until a config is found or the filesystem root is reached.
//!
//! ## Configuration Inheritance
//!
//! Configurations can extend other configurations using the `extends` field:
//!
//! ```jsonc
//! {
//!   "extends": ["../../base.json"],
//!   "linter": {
//!     "rules": {
//!       "correctness": {
//!         "duplicate-definition": "error"
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! ## Example Configuration
//!
//! ```jsonc
//! {
//!   "$schema": "https://octofhir.github.io/fsh-lint-rs/schema/v1.json",
//!   "root": true,
//!   "linter": {
//!     "enabled": true,
//!     "rules": {
//!       "recommended": true,
//!       "correctness": {
//!         "duplicate-definition": "error"
//!       },
//!       "style": {
//!         "naming-convention": "warn"
//!       }
//!     },
//!     "ruleDirectories": ["./custom-rules"]
//!   },
//!   "formatter": {
//!     "enabled": true,
//!     "indentSize": 2,
//!     "lineWidth": 100
//!   },
//!   "files": {
//!     "include": ["**/*.fsh"],
//!     "exclude": ["**/node_modules/**", "**/*.generated.fsh"]
//!   }
//! }
//! ```

mod fsh_lint_config;
mod loader;
mod merge;

// Re-export main types
pub use fsh_lint_config::{
    FilesConfiguration, FormatterConfiguration, FshLintConfiguration, LinterConfiguration,
    RuleConfig, RuleSeverity, RulesConfiguration,
};
pub use loader::ConfigLoader;

// Re-export Result type
pub use loader::Result;
