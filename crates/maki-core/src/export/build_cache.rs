//! Build cache for incremental compilation
//!
//! Provides caching of parsed CSTs and file hashes to enable faster rebuilds
//! by only re-processing files that have changed since the last build.

use crate::export::run_blocking_io;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Build cache for incremental compilation
///
/// Stores file hashes and metadata to detect changes between builds.
/// The cache is persisted to disk in the output directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCache {
    /// File hashes (path -> hash)
    file_hashes: HashMap<PathBuf, u64>,

    /// File modification times (path -> timestamp)
    file_mtimes: HashMap<PathBuf, SystemTime>,

    /// Build timestamp
    last_build: SystemTime,

    /// Cache version for compatibility checking
    version: u32,
}

const CACHE_VERSION: u32 = 1;
const CACHE_FILENAME: &str = ".maki-cache.json";

impl BuildCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            file_hashes: HashMap::new(),
            file_mtimes: HashMap::new(),
            last_build: SystemTime::now(),
            version: CACHE_VERSION,
        }
    }

    /// Load cache from disk
    pub fn load(output_dir: &Path) -> io::Result<Self> {
        let cache_path = output_dir.join(CACHE_FILENAME);

        if !cache_path.exists() {
            debug!("No cache file found at {:?}", cache_path);
            return Ok(Self::new());
        }

        let content = run_blocking_io(|| fs::read_to_string(&cache_path))?;
        let cache: BuildCache = serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse cache: {}", e),
            )
        })?;

        // Check version compatibility
        if cache.version != CACHE_VERSION {
            warn!(
                "Cache version mismatch (found: {}, expected: {}), ignoring cache",
                cache.version, CACHE_VERSION
            );
            return Ok(Self::new());
        }

        debug!("Loaded cache with {} entries", cache.file_hashes.len());
        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, output_dir: &Path) -> io::Result<()> {
        let cache_path = output_dir.join(CACHE_FILENAME);

        let content = serde_json::to_string_pretty(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize cache: {}", e),
            )
        })?;

        run_blocking_io(|| fs::write(&cache_path, &content))?;
        debug!("Saved cache to {:?}", cache_path);
        Ok(())
    }

    /// Check if a file has changed since last build
    pub fn is_file_changed(&self, path: &Path) -> io::Result<bool> {
        // If file not in cache, it's new (changed)
        if !self.file_hashes.contains_key(path) {
            return Ok(true);
        }

        // Quick check: modification time
        let metadata = run_blocking_io(|| fs::metadata(path))?;
        let current_mtime = metadata.modified()?;

        if let Some(&cached_mtime) = self.file_mtimes.get(path)
            && current_mtime <= cached_mtime
        {
            // File hasn't been modified since last build
            return Ok(false);
        }

        // Slower check: content hash
        let current_hash = Self::hash_file(path)?;
        let cached_hash = self.file_hashes.get(path);

        Ok(cached_hash != Some(&current_hash))
    }

    /// Update cache entry for a file
    pub fn update_file(&mut self, path: &Path) -> io::Result<()> {
        let hash = Self::hash_file(path)?;
        let metadata = run_blocking_io(|| fs::metadata(path))?;
        let mtime = metadata.modified()?;

        self.file_hashes.insert(path.to_path_buf(), hash);
        self.file_mtimes.insert(path.to_path_buf(), mtime);

        Ok(())
    }

    /// Mark build as complete
    pub fn mark_build_complete(&mut self) {
        self.last_build = SystemTime::now();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_files: self.file_hashes.len(),
            last_build: self.last_build,
        }
    }

    /// Hash a file's contents
    fn hash_file(path: &Path) -> io::Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = run_blocking_io(|| fs::read_to_string(path))?;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(hasher.finish())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.file_hashes.clear();
        self.file_mtimes.clear();
    }

    /// Remove entries for files that no longer exist
    pub fn prune_deleted_files(&mut self) {
        let deleted: Vec<PathBuf> = self
            .file_hashes
            .keys()
            .filter(|path| !path.exists())
            .cloned()
            .collect();

        for path in deleted {
            self.file_hashes.remove(&path);
            self.file_mtimes.remove(&path);
        }
    }
}

impl Default for BuildCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total files in cache
    pub total_files: usize,
    /// Last build timestamp
    pub last_build: SystemTime,
}

/// Result of incremental build analysis
#[derive(Debug, Clone)]
pub struct IncrementalBuildInfo {
    /// Files that changed since last build
    pub changed_files: Vec<PathBuf>,
    /// Files that are unchanged
    pub unchanged_files: Vec<PathBuf>,
    /// New files not in cache
    pub new_files: Vec<PathBuf>,
    /// Files in cache but no longer exist
    pub deleted_files: Vec<PathBuf>,
}

impl IncrementalBuildInfo {
    /// Create analysis from file list and cache
    pub fn analyze(files: &[PathBuf], cache: &BuildCache) -> io::Result<Self> {
        let mut changed_files = Vec::new();
        let mut unchanged_files = Vec::new();
        let mut new_files = Vec::new();

        for file in files {
            if !cache.file_hashes.contains_key(file) {
                new_files.push(file.clone());
            } else if cache.is_file_changed(file)? {
                changed_files.push(file.clone());
            } else {
                unchanged_files.push(file.clone());
            }
        }

        // Find deleted files
        let file_set: std::collections::HashSet<_> = files.iter().collect();
        let deleted_files: Vec<PathBuf> = cache
            .file_hashes
            .keys()
            .filter(|path| !file_set.contains(path))
            .cloned()
            .collect();

        Ok(Self {
            changed_files,
            unchanged_files,
            new_files,
            deleted_files,
        })
    }

    /// Check if any files need processing
    pub fn needs_rebuild(&self) -> bool {
        !self.changed_files.is_empty() || !self.new_files.is_empty()
    }

    /// Total number of files to process
    pub fn files_to_process(&self) -> usize {
        self.changed_files.len() + self.new_files.len()
    }

    /// Log summary of changes
    pub fn log_summary(&self) {
        if !self.needs_rebuild() {
            info!("  No changes detected, using cached build");
            return;
        }

        info!("  Incremental build analysis:");
        if !self.new_files.is_empty() {
            info!("    {} new files", self.new_files.len());
        }
        if !self.changed_files.is_empty() {
            info!("    {} changed files", self.changed_files.len());
        }
        if !self.unchanged_files.is_empty() {
            info!(
                "    {} unchanged files (using cache)",
                self.unchanged_files.len()
            );
        }
        if !self.deleted_files.is_empty() {
            info!("    {} deleted files", self.deleted_files.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_cache() {
        let cache = BuildCache::new();
        assert_eq!(cache.file_hashes.len(), 0);
        assert_eq!(cache.version, CACHE_VERSION);
    }

    #[test]
    fn test_file_hashing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");

        fs::write(&file_path, "Profile: TestProfile").unwrap();

        let hash1 = BuildCache::hash_file(&file_path).unwrap();
        let hash2 = BuildCache::hash_file(&file_path).unwrap();

        // Same content should produce same hash
        assert_eq!(hash1, hash2);

        // Different content should produce different hash
        fs::write(&file_path, "Profile: DifferentProfile").unwrap();
        let hash3 = BuildCache::hash_file(&file_path).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");

        fs::write(&file_path, "Profile: TestProfile").unwrap();

        let mut cache = BuildCache::new();

        // New file should be detected as changed
        assert!(cache.is_file_changed(&file_path).unwrap());

        // Update cache
        cache.update_file(&file_path).unwrap();

        // Now it shouldn't be changed
        assert!(!cache.is_file_changed(&file_path).unwrap());

        // Modify file
        fs::write(&file_path, "Profile: ModifiedProfile").unwrap();

        // Should detect change
        assert!(cache.is_file_changed(&file_path).unwrap());
    }

    #[test]
    fn test_cache_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.fsh");
        let output_dir = temp_dir.path();

        fs::write(&file_path, "Profile: TestProfile").unwrap();

        let mut cache = BuildCache::new();
        cache.update_file(&file_path).unwrap();

        // Save cache
        cache.save(output_dir).unwrap();

        // Load cache
        let loaded_cache = BuildCache::load(output_dir).unwrap();

        assert_eq!(cache.file_hashes.len(), loaded_cache.file_hashes.len());
        assert!(!loaded_cache.is_file_changed(&file_path).unwrap());
    }

    #[test]
    fn test_incremental_analysis() {
        let temp_dir = TempDir::new().unwrap();

        let file1 = temp_dir.path().join("file1.fsh");
        let file2 = temp_dir.path().join("file2.fsh");
        let file3 = temp_dir.path().join("file3.fsh");

        fs::write(&file1, "Profile: File1").unwrap();
        fs::write(&file2, "Profile: File2").unwrap();

        let mut cache = BuildCache::new();
        cache.update_file(&file1).unwrap();
        cache.update_file(&file2).unwrap();

        // Modify file2, add file3
        fs::write(&file2, "Profile: File2Modified").unwrap();
        fs::write(&file3, "Profile: File3").unwrap();

        let files = vec![file1.clone(), file2.clone(), file3.clone()];
        let info = IncrementalBuildInfo::analyze(&files, &cache).unwrap();

        assert_eq!(info.unchanged_files.len(), 1); // file1
        assert_eq!(info.changed_files.len(), 1); // file2
        assert_eq!(info.new_files.len(), 1); // file3
        assert!(info.needs_rebuild());
        assert_eq!(info.files_to_process(), 2); // file2 + file3
    }
}
