//! File change data and optional project discovery/watch integrations.

mod events;
pub use events::{FileChangeEvent, FileChangeKind};

#[cfg(feature = "project")]
mod project;
#[cfg(feature = "project")]
pub use project::*;
