//! File discovery and management for FSH linting
//!
//! This module provides functionality for discovering FSH files based on
//! glob patterns, respecting ignore files, and watching for file changes.

use crate::config::UnifiedConfig;
use crate::{MakiError, Result};
use glob::{Pattern, glob};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Helper to extract file patterns from configuration
fn get_include_patterns(config: &UnifiedConfig) -> Vec<String> {
    config
        .files
        .as_ref()
        .and_then(|f| f.include.as_ref())
        .map(|v| v.clone())
        .unwrap_or_else(|| vec!["**/*.fsh".to_string()])
}

fn get_exclude_patterns(config: &UnifiedConfig) -> Vec<String> {
    config
        .files
        .as_ref()
        .and_then(|f| f.exclude.as_ref())
        .map(|v| v.clone())
        .unwrap_or_default()
}

fn get_ignore_files(config: &UnifiedConfig) -> Vec<String> {
    config
        .files
        .as_ref()
        .and_then(|f| f.ignore_files.as_ref())
        .map(|v| v.clone())
        .unwrap_or_else(|| vec![".fshlintignore".to_string()])
}

use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Trait for file discovery functionality
pub trait FileDiscovery {
    /// Discover FSH files based on configuration patterns
    fn discover_files(&self, config: &UnifiedConfig) -> Result<Vec<PathBuf>>;

    /// Check if a file should be included based on configuration
    fn should_include(&self, path: &Path, config: &UnifiedConfig) -> bool;

    /// Create a file watcher for LSP integration
    fn watch_for_changes(&self) -> Result<FileWatcher>;
}

/// Default implementation of file discovery
#[derive(Debug, Clone)]
pub struct DefaultFileDiscovery {
    /// Root directory for file discovery
    pub root_dir: PathBuf,
}

impl DefaultFileDiscovery {
    /// Create a new file discovery instance
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    /// Load ignore patterns from .gitignore and custom ignore files
    fn load_ignore_patterns(&self, config: &UnifiedConfig) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();

        // Load .gitignore if it exists
        let gitignore_path = self.root_dir.join(".gitignore");
        if gitignore_path.exists() {
            match std::fs::read_to_string(&gitignore_path) {
                Ok(content) => {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            // Convert gitignore-style patterns to glob patterns
                            let glob_pattern = if line.ends_with('/') {
                                // Directory pattern: "temp/" becomes "temp/**"
                                format!("{line}**")
                            } else if !line.contains('*') && !line.contains('?') {
                                // Plain filename: "file.txt" becomes "**/file.txt"
                                if line.contains('/') {
                                    line.to_string()
                                } else {
                                    format!("**/{line}")
                                }
                            } else {
                                // Already a glob pattern
                                line.to_string()
                            };

                            if let Ok(pattern) = Pattern::new(&glob_pattern) {
                                patterns.push(pattern);
                            }
                        }
                    }
                    debug!("Loaded {} patterns from .gitignore", patterns.len());
                }
                Err(e) => warn!("Failed to read .gitignore: {}", e),
            }
        }

        // Load custom ignore files specified in config
        for ignore_file in &get_ignore_files(config) {
            let ignore_path = self.root_dir.join(ignore_file);
            if ignore_path.exists() {
                match std::fs::read_to_string(&ignore_path) {
                    Ok(content) => {
                        for line in content.lines() {
                            let line = line.trim();
                            if !line.is_empty() && !line.starts_with('#') {
                                // Convert gitignore-style patterns to glob patterns
                                let glob_pattern = if line.ends_with('/') {
                                    // Directory pattern: "temp/" becomes "temp/**"
                                    format!("{line}**")
                                } else if !line.contains('*') && !line.contains('?') {
                                    // Plain filename: "file.txt" becomes "**/file.txt"
                                    if line.contains('/') {
                                        line.to_string()
                                    } else {
                                        format!("**/{line}")
                                    }
                                } else {
                                    // Already a glob pattern
                                    line.to_string()
                                };

                                if let Ok(pattern) = Pattern::new(&glob_pattern) {
                                    patterns.push(pattern);
                                }
                            }
                        }
                        debug!("Loaded patterns from {}", ignore_file);
                    }
                    Err(e) => warn!("Failed to read ignore file {}: {}", ignore_file, e),
                }
            }
        }

        Ok(patterns)
    }

    /// Check if a path matches any ignore pattern
    fn is_ignored(&self, path: &Path, ignore_patterns: &[Pattern]) -> bool {
        // Convert to relative path for pattern matching
        let relative_path = if let Ok(rel) = path.strip_prefix(&self.root_dir) {
            rel
        } else {
            path
        };
        let path_str = relative_path.to_string_lossy();

        for pattern in ignore_patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }

        false
    }

    /// Discover files using glob patterns
    fn discover_with_globs(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = HashSet::new();

        for pattern in patterns {
            let full_pattern = if pattern.starts_with('/') {
                pattern.clone()
            } else {
                format!("{}/{}", self.root_dir.display(), pattern)
            };

            match glob(&full_pattern) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(path) => {
                                if path.is_file() {
                                    files.insert(path);
                                }
                            }
                            Err(e) => warn!("Glob entry error: {}", e),
                        }
                    }
                }
                Err(e) => {
                    return Err(MakiError::ConfigError {
                        message: format!("Invalid glob pattern '{pattern}': {e}"),
                    });
                }
            }
        }

        Ok(files.into_iter().collect())
    }

    /// Discover files by walking directory tree
    fn discover_by_walking(&self, extensions: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.root_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file()
                && let Some(ext) = path.extension()
            {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if extensions
                    .iter()
                    .any(|allowed_ext| allowed_ext.to_lowercase() == ext_str)
                {
                    files.push(path.to_path_buf());
                }
            }
        }

        Ok(files)
    }
}

impl FileDiscovery for DefaultFileDiscovery {
    fn discover_files(&self, config: &UnifiedConfig) -> Result<Vec<PathBuf>> {
        info!("Discovering FSH files in {}", self.root_dir.display());

        let ignore_patterns = self.load_ignore_patterns(config)?;

        // Discover files using include patterns
        let mut discovered_files = if get_include_patterns(config).is_empty() {
            // Default: discover .fsh files
            self.discover_by_walking(&["fsh".to_string()])?
        } else {
            self.discover_with_globs(&get_include_patterns(config))?
        };

        // Filter out files matching exclude patterns
        let exclude_list = get_exclude_patterns(config);
        if !exclude_list.is_empty() {
            let exclude_patterns: Result<Vec<Pattern>> = exclude_list
                .iter()
                .map(|p| {
                    Pattern::new(p).map_err(|e| MakiError::ConfigError {
                        message: format!("Invalid exclude pattern '{p}': {e}"),
                    })
                })
                .collect();

            let exclude_patterns = exclude_patterns?;

            discovered_files.retain(|path| !self.is_ignored(path, &exclude_patterns));
        }

        // Filter out files matching ignore patterns
        discovered_files.retain(|path| !self.is_ignored(path, &ignore_patterns));

        // Ensure all paths are absolute - convert to canonical paths for consistency
        let mut absolute_files = Vec::new();
        for file in discovered_files {
            // If the path is already absolute, use it as-is
            // Otherwise, join it with the root directory
            let absolute_path = if file.is_absolute() {
                file
            } else if file.starts_with(&self.root_dir) {
                // Path already includes root_dir (from glob expansion), just ensure it's absolute
                if let Ok(canonical) = std::fs::canonicalize(&file) {
                    canonical
                } else {
                    // If canonicalize fails, try joining with current dir
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(&file)
                }
            } else {
                // Path is relative to root_dir, join them
                let joined = self.root_dir.join(&file);
                if let Ok(canonical) = std::fs::canonicalize(&joined) {
                    canonical
                } else {
                    joined
                }
            };
            absolute_files.push(absolute_path);
        }

        info!("Discovered {} FSH files", absolute_files.len());
        debug!("Files: {:?}", absolute_files);

        Ok(absolute_files)
    }

    fn should_include(&self, path: &Path, config: &UnifiedConfig) -> bool {
        // Check if file matches include patterns
        let matches_include = if get_include_patterns(config).is_empty() {
            // Default: include .fsh files (case-insensitive)
            path.extension()
                .map(|ext| ext.to_string_lossy().to_lowercase() == "fsh")
                .unwrap_or(false)
        } else {
            get_include_patterns(config).iter().any(|pattern| {
                Pattern::new(pattern)
                    .map(|p| p.matches(&path.to_string_lossy()))
                    .unwrap_or(false)
            })
        };

        if !matches_include {
            return false;
        }

        // Check if file matches exclude patterns
        let matches_exclude = get_exclude_patterns(config).iter().any(|pattern| {
            Pattern::new(pattern)
                .map(|p| p.matches(&path.to_string_lossy()))
                .unwrap_or(false)
        });

        if matches_exclude {
            return false;
        }

        // Check ignore patterns
        if let Ok(ignore_patterns) = self.load_ignore_patterns(config)
            && self.is_ignored(path, &ignore_patterns)
        {
            return false;
        }

        true
    }

    fn watch_for_changes(&self) -> Result<FileWatcher> {
        FileWatcher::new(&self.root_dir)
    }
}

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

/// File watcher for LSP integration
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: tokio_mpsc::UnboundedReceiver<FileChangeEvent>,
    debounce_duration: Duration,
    last_events: std::collections::HashMap<PathBuf, Instant>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(root_dir: &Path) -> Result<Self> {
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        let debounce_duration = Duration::from_millis(100);

        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| match res {
                Ok(event) => {
                    let change_kind = match event.kind {
                        EventKind::Create(_) => FileChangeKind::Created,
                        EventKind::Modify(_) => FileChangeKind::Modified,
                        EventKind::Remove(_) => FileChangeKind::Deleted,
                        _ => FileChangeKind::Modified,
                    };

                    for path in event.paths {
                        let change_event = FileChangeEvent {
                            path,
                            kind: change_kind.clone(),
                            timestamp: Instant::now(),
                        };

                        if let Err(e) = tx.send(change_event) {
                            warn!("Failed to send file change event: {}", e);
                        }
                    }
                }
                Err(e) => warn!("File watcher error: {}", e),
            })
            .map_err(|e| MakiError::IoError {
                path: root_dir.to_path_buf(),
                source: std::io::Error::other(format!("Failed to create file watcher: {e}")),
            })?;

        watcher
            .watch(root_dir, RecursiveMode::Recursive)
            .map_err(|e| MakiError::IoError {
                path: root_dir.to_path_buf(),
                source: std::io::Error::other(format!("Failed to watch directory: {e}")),
            })?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            debounce_duration,
            last_events: std::collections::HashMap::new(),
        })
    }

    /// Receive the next file change event with debouncing
    pub async fn next_event(&mut self) -> Option<FileChangeEvent> {
        loop {
            match self.receiver.recv().await {
                Some(event) => {
                    let now = Instant::now();

                    // Check if we should debounce this event
                    if let Some(last_time) = self.last_events.get(&event.path)
                        && now.duration_since(*last_time) < self.debounce_duration
                    {
                        // Skip this event due to debouncing
                        continue;
                    }

                    // Update the last event time for this path
                    self.last_events.insert(event.path.clone(), now);

                    // Clean up old entries to prevent memory leaks
                    self.last_events.retain(|_, &mut last_time| {
                        now.duration_since(last_time) < Duration::from_secs(60)
                    });

                    return Some(event);
                }
                None => return None,
            }
        }
    }

    /// Set debounce duration for file change events
    pub fn set_debounce_duration(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }

    /// Get current debounce duration
    pub fn debounce_duration(&self) -> Duration {
        self.debounce_duration
    }

    /// Receive multiple file change events in a batch with debouncing
    /// This is useful for processing multiple rapid changes efficiently
    pub async fn next_events_batch(&mut self, max_batch_size: usize) -> Vec<FileChangeEvent> {
        let mut events = Vec::new();

        // Get the first event (blocking)
        if let Some(first_event) = self.next_event().await {
            events.push(first_event);
        } else {
            return events;
        }

        // Try to get additional events without blocking (up to max_batch_size)
        while events.len() < max_batch_size {
            match tokio::time::timeout(Duration::from_millis(10), self.next_event()).await {
                Ok(Some(event)) => events.push(event),
                _ => break, // Timeout or no more events
            }
        }

        events
    }

    /// Check if there are pending events without consuming them
    pub fn has_pending_events(&self) -> bool {
        !self.receiver.is_empty()
    }

    /// Filter file change events by file extension
    pub fn filter_by_extension(
        events: Vec<FileChangeEvent>,
        extensions: &[&str],
    ) -> Vec<FileChangeEvent> {
        events
            .into_iter()
            .filter(|event| {
                if let Some(ext) = event.path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    extensions
                        .iter()
                        .any(|&allowed_ext| allowed_ext.to_lowercase() == ext_str)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Filter file change events to only include FSH files
    pub fn filter_fsh_files(events: Vec<FileChangeEvent>) -> Vec<FileChangeEvent> {
        Self::filter_by_extension(events, &["fsh"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config() -> crate::config::UnifiedConfig {
        crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["**/*.fsh".to_string()]),
                exclude: Some(vec!["**/target/**".to_string()]),
                ignore_files: Some(vec![".fshlintignore".to_string()]),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_discover_files_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::write(root.join("test1.fsh"), "Profile: Test1").unwrap();
        fs::write(root.join("test2.fsh"), "Profile: Test2").unwrap();
        fs::write(root.join("readme.md"), "# README").unwrap();

        let discovery = DefaultFileDiscovery::new(root);
        let config = create_test_config();

        let files = discovery.discover_files(&config).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&root.join("test1.fsh")));
        assert!(files.contains(&root.join("test2.fsh")));
    }

    #[test]
    fn test_should_include() {
        let temp_dir = TempDir::new().unwrap();
        let discovery = DefaultFileDiscovery::new(temp_dir.path());
        let config = create_test_config();

        assert!(discovery.should_include(&PathBuf::from("test.fsh"), &config));
        assert!(!discovery.should_include(&PathBuf::from("test.md"), &config));
        assert!(!discovery.should_include(&PathBuf::from("target/test.fsh"), &config));
    }

    #[test]
    fn test_gitignore_support() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create .gitignore
        fs::write(root.join(".gitignore"), "*.tmp\ntarget/\n").unwrap();

        // Create test files
        fs::write(root.join("test.fsh"), "Profile: Test").unwrap();
        fs::write(root.join("temp.tmp"), "temporary").unwrap();

        let discovery = DefaultFileDiscovery::new(root);
        let config = create_test_config();

        let files = discovery.discover_files(&config).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&root.join("test.fsh")));
    }

    #[tokio::test]
    async fn test_file_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let watcher = FileWatcher::new(temp_dir.path());

        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_file_watcher_debouncing() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(temp_dir.path()).unwrap();

        // Set a short debounce duration for testing
        watcher.set_debounce_duration(Duration::from_millis(50));
        assert_eq!(watcher.debounce_duration(), Duration::from_millis(50));

        // Test that watcher has no pending events initially
        assert!(!watcher.has_pending_events());
    }

    #[tokio::test]
    async fn test_file_watcher_batch_events() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = FileWatcher::new(temp_dir.path()).unwrap();

        // Test batch event collection (should return empty if no events)
        let events =
            tokio::time::timeout(Duration::from_millis(100), watcher.next_events_batch(5)).await;

        // Should timeout since no file changes occurred
        assert!(events.is_err() || events.unwrap().is_empty());
    }

    #[test]
    fn test_file_change_event_filtering() {
        let events = vec![
            FileChangeEvent {
                path: PathBuf::from("test.fsh"),
                kind: FileChangeKind::Modified,
                timestamp: Instant::now(),
            },
            FileChangeEvent {
                path: PathBuf::from("test.md"),
                kind: FileChangeKind::Modified,
                timestamp: Instant::now(),
            },
            FileChangeEvent {
                path: PathBuf::from("another.fsh"),
                kind: FileChangeKind::Created,
                timestamp: Instant::now(),
            },
        ];

        // Test filtering by extension
        let fsh_events = FileWatcher::filter_by_extension(events.clone(), &["fsh"]);
        assert_eq!(fsh_events.len(), 2);
        assert!(
            fsh_events
                .iter()
                .all(|e| e.path.extension().unwrap() == "fsh")
        );

        // Test FSH-specific filtering
        let fsh_only = FileWatcher::filter_fsh_files(events);
        assert_eq!(fsh_only.len(), 2);
        assert!(
            fsh_only
                .iter()
                .all(|e| e.path.extension().unwrap() == "fsh")
        );
    }

    #[test]
    fn test_glob_pattern_matching() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create nested directory structure
        fs::create_dir_all(root.join("src/profiles")).unwrap();
        fs::create_dir_all(root.join("src/extensions")).unwrap();
        fs::create_dir_all(root.join("test/fixtures")).unwrap();
        fs::create_dir_all(root.join("build/output")).unwrap();

        // Create test files
        fs::write(root.join("src/profiles/patient.fsh"), "Profile: Patient").unwrap();
        fs::write(
            root.join("src/extensions/address.fsh"),
            "Extension: Address",
        )
        .unwrap();
        fs::write(root.join("test/fixtures/sample.fsh"), "Profile: Sample").unwrap();
        fs::write(
            root.join("build/output/generated.fsh"),
            "Profile: Generated",
        )
        .unwrap();
        fs::write(root.join("readme.md"), "# README").unwrap();

        let discovery = DefaultFileDiscovery::new(root);

        // Test specific glob patterns

        // Test matching all FSH files
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["**/*.fsh".to_string()]),
                exclude: Some(vec![]),
                ignore_files: None,
            }),
            ..Default::default()
        };
        let files = discovery.discover_files(&config).unwrap();
        assert_eq!(files.len(), 4);

        // Test matching only src directory
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["src/**/*.fsh".to_string()]),
                exclude: None,
                ignore_files: None,
            }),
            ..Default::default()
        };
        let files = discovery.discover_files(&config).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|f| f.starts_with(root.join("src"))));

        // Test excluding build directory
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["**/*.fsh".to_string()]),
                exclude: Some(vec!["build/**".to_string()]),
                ignore_files: None,
            }),
            ..Default::default()
        };
        let files = discovery.discover_files(&config).unwrap();
        assert_eq!(files.len(), 3);
        assert!(!files.iter().any(|f| f.starts_with(root.join("build"))));

        // Test multiple include patterns
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec![
                    "src/**/*.fsh".to_string(),
                    "test/**/*.fsh".to_string(),
                ]),
                exclude: Some(vec![]),
                ignore_files: None,
            }),
            ..Default::default()
        };
        let files = discovery.discover_files(&config).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_ignore_file_handling() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("temp")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();

        // Create test files
        fs::write(root.join("src/test.fsh"), "Profile: Test").unwrap();
        fs::write(root.join("temp/cache.fsh"), "Profile: Cache").unwrap();
        fs::write(root.join("node_modules/dep.fsh"), "Profile: Dependency").unwrap();
        fs::write(root.join("important.fsh"), "Profile: Important").unwrap();

        // Create .gitignore
        fs::write(root.join(".gitignore"), "temp/\nnode_modules/\n*.log\n").unwrap();

        // Create custom ignore file
        fs::write(root.join(".fshlintignore"), "*.backup\nold/\n").unwrap();

        let discovery = DefaultFileDiscovery::new(root);
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["**/*.fsh".to_string()]),
                exclude: Some(vec![]),
                ignore_files: Some(vec![".fshlintignore".to_string()]),
            }),
            ..Default::default()
        };

        let files = discovery.discover_files(&config).unwrap();

        // Should exclude files matching .gitignore patterns
        assert!(!files.iter().any(|f| f.starts_with(root.join("temp"))));
        assert!(
            !files
                .iter()
                .any(|f| f.starts_with(root.join("node_modules")))
        );

        // Should include files not matching ignore patterns
        assert!(files.iter().any(|f| f.file_name().unwrap() == "test.fsh"));
        assert!(
            files
                .iter()
                .any(|f| f.file_name().unwrap() == "important.fsh")
        );

        // Test should_include method
        assert!(discovery.should_include(&PathBuf::from("src/test.fsh"), &config));
        assert!(!discovery.should_include(&PathBuf::from("temp/cache.fsh"), &config));
        assert!(!discovery.should_include(&PathBuf::from("node_modules/dep.fsh"), &config));
    }

    #[test]
    fn test_complex_file_structure() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create complex nested structure
        let dirs = [
            "project/src/main/fsh/profiles",
            "project/src/main/fsh/extensions",
            "project/src/test/fsh/examples",
            "project/target/generated",
            "project/docs/examples",
            "another-project/src/fsh",
        ];

        for dir in &dirs {
            fs::create_dir_all(root.join(dir)).unwrap();
        }

        // Create files in various locations
        let files = [
            (
                "project/src/main/fsh/profiles/patient.fsh",
                "Profile: Patient",
            ),
            (
                "project/src/main/fsh/extensions/address.fsh",
                "Extension: Address",
            ),
            (
                "project/src/test/fsh/examples/example.fsh",
                "Instance: Example",
            ),
            ("project/target/generated/output.fsh", "Generated content"),
            ("project/docs/examples/sample.fsh", "Documentation sample"),
            ("another-project/src/fsh/other.fsh", "Profile: Other"),
            ("project/README.md", "# Project README"),
        ];

        for (file_path, content) in &files {
            fs::write(root.join(file_path), content).unwrap();
        }

        let discovery = DefaultFileDiscovery::new(root);

        // Test discovering all FSH files
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["**/*.fsh".to_string()]),
                exclude: Some(vec!["**/target/**".to_string()]),
                ignore_files: None,
            }),
            ..Default::default()
        };
        let discovered = discovery.discover_files(&config).unwrap();

        assert_eq!(discovered.len(), 5); // Excludes target/generated/output.fsh
        assert!(
            !discovered
                .iter()
                .any(|f| f.starts_with(root.join("project/target")))
        );

        // Test project-specific discovery
        let config = crate::config::UnifiedConfig {
            files: Some(crate::config::FilesConfiguration {
                include: Some(vec!["project/**/*.fsh".to_string()]),
                exclude: Some(vec!["**/target/**".to_string(), "**/docs/**".to_string()]),
                ignore_files: None,
            }),
            ..Default::default()
        };
        let project_files = discovery.discover_files(&config).unwrap();

        assert_eq!(project_files.len(), 3);
        assert!(
            project_files
                .iter()
                .all(|f| f.starts_with(root.join("project")))
        );
        assert!(!project_files.iter().any(|f| {
            let path_str = f.to_string_lossy();
            path_str.contains("target") || path_str.contains("docs")
        }));
    }

    #[test]
    fn test_file_discovery_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Test empty directory
        let discovery = DefaultFileDiscovery::new(root);
        let config = crate::config::UnifiedConfig::default();
        let files = discovery.discover_files(&config).unwrap();
        assert!(files.is_empty());

        // Test directory with no FSH files
        fs::write(root.join("readme.txt"), "README").unwrap();
        fs::write(root.join("config.json"), "{}").unwrap();
        let files = discovery.discover_files(&config).unwrap();
        assert!(files.is_empty());

        // Test files without extensions
        fs::write(root.join("Makefile"), "all:").unwrap();
        let files = discovery.discover_files(&config).unwrap();
        assert!(files.is_empty());

        // Test case sensitivity - use empty include patterns to trigger walking mode
        let mut config_empty = config.clone();
        config_empty
            .files
            .get_or_insert_with(Default::default)
            .include = Some(vec![]); // This will trigger discover_by_walking
        fs::write(root.join("test1.FSH"), "Profile: Test1").unwrap();
        fs::write(root.join("test2.Fsh"), "Profile: Test2").unwrap();

        let files = discovery.discover_files(&config_empty).unwrap();
        // Should find both files (case-insensitive extension matching in walking mode)
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_file_watcher_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let discovery = DefaultFileDiscovery::new(root);

        // Test creating file watcher
        let watcher_result = discovery.watch_for_changes();
        assert!(watcher_result.is_ok());

        let mut watcher = watcher_result.unwrap();

        // Test watcher configuration
        assert_eq!(watcher.debounce_duration(), Duration::from_millis(100));

        watcher.set_debounce_duration(Duration::from_millis(200));
        assert_eq!(watcher.debounce_duration(), Duration::from_millis(200));

        // Test that watcher starts with no pending events
        assert!(!watcher.has_pending_events());
    }
}
