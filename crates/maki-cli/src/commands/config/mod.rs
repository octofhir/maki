//! Configuration management subcommands
//!
//! This module contains all config-related subcommands:
//! - init: Create new configuration files
//! - migrate: Migrate from SUSHI to MAKI config format
//! - validate: Validate configuration files
//! - show: Display current configuration

pub mod migrate;

// Re-export for easier access
pub use migrate::migrate_command;
