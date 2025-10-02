//! Caching infrastructure for parse results and rule compilation

use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use dashmap::DashMap;
use crate::parser::ParseResult;
use crate::discovery::{FileChangeEvent, FileChangeKind};

/// Trait for cache implementations
pub trait Cache<K, V> {
    /// Get a value from the cache
    fn get(&self, key: &K) -> Option<V>;
    
    /// Insert a value into the cache
    fn insert(&self, key: K, value: V);
    
    /// Remove a value from the cache
    fn remove(&self, key: &K) -> Option<V>;
    
    /// Clear all entries from the cache
    fn clear(&self);
    
    /// Get the current size of the cache
    fn len(&self) -> usize;
    
    /// Check if the cache is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Content hash for cache keys
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentHash {
    hash: u64,
    size: usize,
}

impl ContentHash {
    /// Create a new content hash from string content
    pub fn from_content(content: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        
        Self {
            hash: hasher.finish(),
            size: content.len(),
        }
    }
    
    /// Get the hash value
    pub fn hash(&self) -> u64 {
        self.hash
    }
    
    /// Get the content size
    pub fn size(&self) -> usize {
        self.size
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    access_time: u64,
}

impl<V> CacheEntry<V> {
    fn new(value: V) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            value,
            access_time: now,
        }
    }
    
    fn touch(&mut self) {
        self.access_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// LRU cache implementation for parse results
pub struct LruCache<K, V> {
    entries: DashMap<K, CacheEntry<V>>,
    max_size: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Create a new LRU cache with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: DashMap::new(),
            max_size,
        }
    }
    
    /// Evict least recently used entries if cache is over capacity
    fn evict_if_needed(&self) {
        while self.entries.len() > self.max_size {
            // Find the entry with the oldest access time
            let oldest_key = {
                let mut oldest_time = u64::MAX;
                let mut oldest_key = None;
                
                for entry in self.entries.iter() {
                    if entry.value().access_time < oldest_time {
                        oldest_time = entry.value().access_time;
                        oldest_key = Some(entry.key().clone());
                    }
                }
                
                oldest_key
            };
            
            // Remove the oldest entry
            if let Some(key) = oldest_key {
                self.entries.remove(&key);
            } else {
                break; // No entries to remove
            }
        }
    }
}

impl<K, V> Cache<K, V> for LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        self.entries.get_mut(key).map(|mut entry| {
            entry.touch();
            entry.value.clone()
        })
    }
    
    fn insert(&self, key: K, value: V) {
        // First insert the new entry
        self.entries.insert(key, CacheEntry::new(value));
        // Then evict if needed
        self.evict_if_needed();
    }
    
    fn remove(&self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|(_, entry)| entry.value)
    }
    
    fn clear(&self) {
        self.entries.clear();
    }
    
    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Parse result cache with content hash-based keying
pub struct ParseResultCache {
    cache: LruCache<ContentHash, Arc<ParseResult>>,
}

impl ParseResultCache {
    /// Create a new parse result cache with default size (1000 entries)
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }
    
    /// Create a new parse result cache with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity),
        }
    }
    
    /// Get a cached parse result by content hash
    pub fn get(&self, content_hash: &ContentHash) -> Option<Arc<ParseResult>> {
        self.cache.get(content_hash)
    }
    
    /// Cache a parse result with its content hash
    pub fn insert(&self, content_hash: ContentHash, parse_result: ParseResult) {
        self.cache.insert(content_hash, Arc::new(parse_result));
    }
    
    /// Cache a parse result Arc with its content hash
    pub fn insert_arc(&self, content_hash: ContentHash, parse_result: Arc<ParseResult>) {
        self.cache.insert(content_hash, parse_result);
    }
    
    /// Remove a cached parse result
    pub fn remove(&self, content_hash: &ContentHash) -> Option<Arc<ParseResult>> {
        self.cache.remove(content_hash)
    }
    
    /// Invalidate cache entries (remove all entries)
    pub fn invalidate_all(&self) {
        self.cache.clear();
    }
    
    /// Invalidate cache entries based on file change events
    pub fn invalidate_on_file_change(&self, event: &FileChangeEvent) {
        match event.kind {
            FileChangeKind::Modified | FileChangeKind::Created => {
                // For modified or created files, we need to invalidate any cached
                // parse results that might be for this file. Since we cache by content hash,
                // we can't directly map file paths to cache entries, so we clear all cache.
                // This is a conservative approach that ensures correctness.
                self.invalidate_all();
            }
            FileChangeKind::Deleted => {
                // For deleted files, we also clear all cache to be safe
                self.invalidate_all();
            }
            FileChangeKind::Renamed => {
                // For renamed files, clear all cache
                self.invalidate_all();
            }
        }
    }
    
    /// Invalidate cache entries for files in a specific directory
    pub fn invalidate_directory(&self, _dir_path: &Path) {
        // Since we cache by content hash rather than file path,
        // we conservatively clear all cache when a directory changes
        self.invalidate_all();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.cache.max_size,
        }
    }
}

impl Default for ParseResultCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}

impl CacheStats {
    /// Get the cache utilization as a percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.size as f64 / self.capacity as f64) * 100.0
        }
    }
}

/// Cache manager that coordinates cache invalidation with file watching
pub struct CacheManager {
    parse_cache: ParseResultCache,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            parse_cache: ParseResultCache::new(),
        }
    }
    
    /// Create a new cache manager with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            parse_cache: ParseResultCache::with_capacity(capacity),
        }
    }
    
    /// Get the parse result cache
    pub fn parse_cache(&self) -> &ParseResultCache {
        &self.parse_cache
    }
    
    /// Handle file change events and invalidate cache as needed
    pub fn handle_file_change(&self, event: &FileChangeEvent) {
        // Only invalidate for FSH files
        if let Some(extension) = event.path.extension() {
            if extension == "fsh" {
                self.parse_cache.invalidate_on_file_change(event);
            }
        }
    }
    
    /// Handle multiple file change events
    pub fn handle_file_changes(&self, events: &[FileChangeEvent]) {
        for event in events {
            self.handle_file_change(event);
        }
    }
    
    /// Get comprehensive cache statistics
    pub fn stats(&self) -> CacheManagerStats {
        CacheManagerStats {
            parse_cache: self.parse_cache.stats(),
        }
    }
    
    /// Clear all caches
    pub fn clear_all(&self) {
        self.parse_cache.invalidate_all();
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive cache statistics
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    pub parse_cache: CacheStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{FshParser, Parser};
    
    #[test]
    fn test_content_hash_creation() {
        let content1 = "Profile: MyPatient\nParent: Patient";
        let content2 = "Profile: MyPatient\nParent: Patient";
        let content3 = "Profile: MyPatient\nParent: DomainResource";
        
        let hash1 = ContentHash::from_content(content1);
        let hash2 = ContentHash::from_content(content2);
        let hash3 = ContentHash::from_content(content3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.size(), content1.len());
    }
    
    #[test]
    fn test_lru_cache_basic_operations() {
        let cache: LruCache<String, i32> = LruCache::new(3);
        
        // Test insertion and retrieval
        cache.insert("key1".to_string(), 1);
        cache.insert("key2".to_string(), 2);
        cache.insert("key3".to_string(), 3);
        
        assert_eq!(cache.get(&"key1".to_string()), Some(1));
        assert_eq!(cache.get(&"key2".to_string()), Some(2));
        assert_eq!(cache.get(&"key3".to_string()), Some(3));
        assert_eq!(cache.len(), 3);
    }
    
    #[test]
    fn test_lru_cache_eviction() {
        let cache: LruCache<String, i32> = LruCache::new(2);
        
        // Fill cache to capacity
        cache.insert("key1".to_string(), 1);
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamps
        cache.insert("key2".to_string(), 2);
        assert_eq!(cache.len(), 2);
        
        // Access key1 to make it more recently used
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamps
        let val1 = cache.get(&"key1".to_string());
        assert_eq!(val1, Some(1));
        
        // Insert new item, should evict key2 (least recently used)
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamps
        cache.insert("key3".to_string(), 3);
        
        // After insertion, cache should have key1 and key3, but not key2
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"key1".to_string()), Some(1));
        assert_eq!(cache.get(&"key3".to_string()), Some(3));
        assert_eq!(cache.get(&"key2".to_string()), None);
    }
    
    #[test]
    fn test_parse_result_cache() {
        let cache = ParseResultCache::with_capacity(2);
        let mut parser = FshParser::new().unwrap();
        
        let content1 = "Profile: MyPatient\nParent: Patient";
        let content2 = "Profile: MyObservation\nParent: Observation";
        
        let hash1 = ContentHash::from_content(content1);
        let hash2 = ContentHash::from_content(content2);
        
        let result1 = parser.parse(content1, None).unwrap();
        let result2 = parser.parse(content2, None).unwrap();
        
        // Cache the results
        cache.insert(hash1.clone(), result1);
        cache.insert(hash2.clone(), result2);
        
        // Retrieve from cache
        let cached1 = cache.get(&hash1);
        let cached2 = cache.get(&hash2);
        
        assert!(cached1.is_some());
        assert!(cached2.is_some());
        assert_eq!(cached1.unwrap().source(), content1);
        assert_eq!(cached2.unwrap().source(), content2);
    }
    
    #[test]
    fn test_cache_stats() {
        let cache = ParseResultCache::with_capacity(10);
        let stats = cache.stats();
        
        assert_eq!(stats.size, 0);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.utilization(), 0.0);
        
        // Add some entries
        let hash = ContentHash::from_content("test content");
        let mut parser = FshParser::new().unwrap();
        let result = parser.parse("test content", None).unwrap();
        cache.insert(hash, result);
        
        let stats = cache.stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.utilization(), 10.0);
    }
    
    #[test]
    fn test_cache_invalidation_on_file_change() {
        use std::time::Instant;
        
        let cache = ParseResultCache::with_capacity(10);
        let mut parser = FshParser::new().unwrap();
        
        // Add some entries to cache
        let hash1 = ContentHash::from_content("content1");
        let hash2 = ContentHash::from_content("content2");
        let result1 = parser.parse("content1", None).unwrap();
        let result2 = parser.parse("content2", None).unwrap();
        
        cache.insert(hash1.clone(), result1);
        cache.insert(hash2.clone(), result2);
        assert_eq!(cache.stats().size, 2);
        
        // Create file change event
        let change_event = FileChangeEvent {
            path: std::path::PathBuf::from("test.fsh"),
            kind: FileChangeKind::Modified,
            timestamp: Instant::now(),
        };
        
        // Invalidate on file change
        cache.invalidate_on_file_change(&change_event);
        assert_eq!(cache.stats().size, 0);
    }
    
    #[test]
    fn test_cache_manager() {
        use std::time::Instant;
        
        let manager = CacheManager::with_capacity(5);
        let mut parser = FshParser::new().unwrap();
        
        // Add entry to parse cache
        let hash = ContentHash::from_content("test content");
        let result = parser.parse("test content", None).unwrap();
        manager.parse_cache().insert(hash, result);
        
        let stats = manager.stats();
        assert_eq!(stats.parse_cache.size, 1);
        
        // Test file change handling
        let fsh_change = FileChangeEvent {
            path: std::path::PathBuf::from("test.fsh"),
            kind: FileChangeKind::Modified,
            timestamp: Instant::now(),
        };
        
        let non_fsh_change = FileChangeEvent {
            path: std::path::PathBuf::from("test.txt"),
            kind: FileChangeKind::Modified,
            timestamp: Instant::now(),
        };
        
        // FSH file change should invalidate cache
        manager.handle_file_change(&fsh_change);
        assert_eq!(manager.stats().parse_cache.size, 0);
        
        // Add entry back
        let hash = ContentHash::from_content("test content");
        let result = parser.parse("test content", None).unwrap();
        manager.parse_cache().insert(hash, result);
        assert_eq!(manager.stats().parse_cache.size, 1);
        
        // Non-FSH file change should not invalidate cache
        manager.handle_file_change(&non_fsh_change);
        assert_eq!(manager.stats().parse_cache.size, 1);
        
        // Test clearing all caches
        manager.clear_all();
        assert_eq!(manager.stats().parse_cache.size, 0);
    }
    
    #[test]
    fn test_cache_manager_multiple_events() {
        use std::time::Instant;
        
        let manager = CacheManager::new();
        let mut parser = FshParser::new().unwrap();
        
        // Add entry to cache
        let hash = ContentHash::from_content("test content");
        let result = parser.parse("test content", None).unwrap();
        manager.parse_cache().insert(hash, result);
        assert_eq!(manager.stats().parse_cache.size, 1);
        
        // Create multiple file change events
        let events = vec![
            FileChangeEvent {
                path: std::path::PathBuf::from("file1.fsh"),
                kind: FileChangeKind::Modified,
                timestamp: Instant::now(),
            },
            FileChangeEvent {
                path: std::path::PathBuf::from("file2.txt"),
                kind: FileChangeKind::Created,
                timestamp: Instant::now(),
            },
            FileChangeEvent {
                path: std::path::PathBuf::from("file3.fsh"),
                kind: FileChangeKind::Deleted,
                timestamp: Instant::now(),
            },
        ];
        
        // Handle multiple events
        manager.handle_file_changes(&events);
        
        // Cache should be invalidated due to FSH file changes
        assert_eq!(manager.stats().parse_cache.size, 0);
    }
}