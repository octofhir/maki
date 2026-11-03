//! File change data shared by caches and optional filesystem watchers.

use std::path::PathBuf;
use std::time::Instant;

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path of the changed file
    pub path: PathBuf,
    /// Type of change
    pub kind: FileChangeKind,
    /// Timestamp of the event
    pub timestamp: Instant,
}

/// Type of file change
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
}
