//! FSH parsing functionality using tree-sitter

use std::sync::Arc;
use tree_sitter::{Language, Node, Parser as TreeSitterParser, Tree, TreeCursor};
use crate::{FshLintError, Result};
use crate::cache::{ContentHash, ParseResultCache};

/// Parser trait for FSH content
pub trait Parser {
    /// Parse FSH content and return a ParseResult
    fn parse(&mut self, content: &str, old_tree: Option<&Tree>) -> Result<ParseResult>;
    
    /// Parse FSH content incrementally using previous parse tree
    fn parse_incremental(&mut self, content: &str, old_tree: &Tree) -> Result<ParseResult>;
    
    /// Set the language for the parser
    fn set_language(&mut self, language: Language) -> Result<()>;
}

/// Result of parsing FSH content
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The parsed syntax tree
    pub tree: Tree,
    /// Parse errors encountered during parsing
    pub errors: Vec<ParseError>,
    /// Whether the parse was successful (no errors)
    pub is_valid: bool,
    /// Source content that was parsed
    pub source: String,
}

/// Parse error information
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Line number (0-based)
    pub line: usize,
    /// Column number (0-based)
    pub column: usize,
    /// Byte offset in source
    pub offset: usize,
    /// Length of the error span
    pub length: usize,
}

/// Tree-sitter based FSH parser implementation
pub struct FshParser {
    parser: TreeSitterParser,
    language: Language,
}

/// Cached FSH parser that uses content hash-based caching
pub struct CachedFshParser {
    parser: FshParser,
    cache: ParseResultCache,
}

/// Parser configuration for caching behavior
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Enable caching of parse results
    pub enable_cache: bool,
    /// Maximum number of cached parse results
    pub cache_capacity: usize,
}

impl FshParser {
    /// Create a new FSH parser
    pub fn new() -> Result<Self> {
        let language = tree_sitter_fsh::language();
        let mut parser = TreeSitterParser::new();
        
        parser.set_language(language)
            .map_err(|e| FshLintError::parser_error(format!("Failed to set FSH language: {}", e)))?;
        
        Ok(Self { parser, language })
    }
    
    /// Get the FSH language
    pub fn language(&self) -> Language {
        self.language.clone()
    }
    
    /// Extract parse errors from the syntax tree
    fn extract_errors(&self, tree: &Tree, source: &str) -> Vec<ParseError> {
        let mut errors = Vec::new();
        let mut cursor = tree.walk();
        
        self.collect_errors_recursive(&mut cursor, source, &mut errors);
        
        errors
    }
    
    /// Recursively collect parse errors from the tree
    fn collect_errors_recursive(&self, cursor: &mut TreeCursor, source: &str, errors: &mut Vec<ParseError>) {
        let node = cursor.node();
        
        // Check if this node represents an error
        if node.is_error() || node.is_missing() {
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let start_point = node.start_position();
            
            let message = if node.is_missing() {
                format!("Missing {}", node.kind())
            } else {
                format!("Syntax error: unexpected {}", node.kind())
            };
            
            errors.push(ParseError {
                message,
                line: start_point.row,
                column: start_point.column,
                offset: start_byte,
                length: end_byte.saturating_sub(start_byte),
            });
        }
        
        // Recursively check children
        if cursor.goto_first_child() {
            loop {
                self.collect_errors_recursive(cursor, source, errors);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }
}

impl Default for FshParser {
    fn default() -> Self {
        Self::new().expect("Failed to create default FSH parser")
    }
}

impl CachedFshParser {
    /// Create a new cached FSH parser with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(ParserConfig::default())
    }
    
    /// Create a new cached FSH parser with specified configuration
    pub fn with_config(config: ParserConfig) -> Result<Self> {
        let parser = FshParser::new()?;
        let cache = if config.enable_cache {
            ParseResultCache::with_capacity(config.cache_capacity)
        } else {
            ParseResultCache::with_capacity(0) // Disabled cache
        };
        
        Ok(Self { parser, cache })
    }
    
    /// Parse content with caching support
    pub fn parse_with_cache(&mut self, content: &str) -> Result<Arc<ParseResult>> {
        let content_hash = ContentHash::from_content(content);
        
        // Try to get from cache first
        if let Some(cached_result) = self.cache.get(&content_hash) {
            return Ok(cached_result);
        }
        
        // Parse and cache the result
        let parse_result = self.parser.parse(content, None)?;
        let arc_result = Arc::new(parse_result);
        
        // Store in cache (clone the Arc, not the ParseResult)
        self.cache.insert_arc(content_hash, arc_result.clone());
        
        Ok(arc_result)
    }
    
    /// Parse content incrementally with caching support
    pub fn parse_incremental_with_cache(&mut self, content: &str, old_tree: &Tree) -> Result<Arc<ParseResult>> {
        let content_hash = ContentHash::from_content(content);
        
        // For incremental parsing, we still check cache but don't rely on old_tree from cache
        if let Some(cached_result) = self.cache.get(&content_hash) {
            return Ok(cached_result);
        }
        
        // Parse incrementally and cache the result
        let parse_result = self.parser.parse_incremental(content, old_tree)?;
        let arc_result = Arc::new(parse_result);
        
        // Store in cache (clone the Arc, not the ParseResult)
        self.cache.insert_arc(content_hash, arc_result.clone());
        
        Ok(arc_result)
    }
    
    /// Invalidate cache entry for specific content
    pub fn invalidate(&self, content: &str) {
        let content_hash = ContentHash::from_content(content);
        self.cache.remove(&content_hash);
    }
    
    /// Clear all cached parse results
    pub fn clear_cache(&self) {
        self.cache.invalidate_all();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }
    
    /// Get the underlying parser
    pub fn parser(&mut self) -> &mut FshParser {
        &mut self.parser
    }
    
    /// Get the FSH language
    pub fn language(&self) -> Language {
        self.parser.language()
    }
}

impl Default for CachedFshParser {
    fn default() -> Self {
        Self::new().expect("Failed to create default cached FSH parser")
    }
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_capacity: 1000,
        }
    }
}

impl Parser for CachedFshParser {
    fn parse(&mut self, content: &str, old_tree: Option<&Tree>) -> Result<ParseResult> {
        match old_tree {
            Some(tree) => {
                let arc_result = self.parse_incremental_with_cache(content, tree)?;
                Ok((*arc_result).clone())
            }
            None => {
                let arc_result = self.parse_with_cache(content)?;
                Ok((*arc_result).clone())
            }
        }
    }
    
    fn parse_incremental(&mut self, content: &str, old_tree: &Tree) -> Result<ParseResult> {
        let arc_result = self.parse_incremental_with_cache(content, old_tree)?;
        Ok((*arc_result).clone())
    }
    
    fn set_language(&mut self, language: Language) -> Result<()> {
        // Clear cache when language changes as cached results are no longer valid
        self.clear_cache();
        self.parser.set_language(language)
    }
}

impl Parser for FshParser {
    fn parse(&mut self, content: &str, old_tree: Option<&Tree>) -> Result<ParseResult> {
        let tree = self.parser.parse(content, old_tree)
            .ok_or_else(|| FshLintError::parser_error("Failed to parse FSH content".to_string()))?;
        
        let errors = self.extract_errors(&tree, content);
        let is_valid = errors.is_empty();
        
        Ok(ParseResult {
            tree,
            errors,
            is_valid,
            source: content.to_string(),
        })
    }
    
    fn parse_incremental(&mut self, content: &str, old_tree: &Tree) -> Result<ParseResult> {
        self.parse(content, Some(old_tree))
    }
    
    fn set_language(&mut self, language: Language) -> Result<()> {
        self.parser.set_language(language)
            .map_err(|e| FshLintError::parser_error(format!("Failed to set language: {}", e)))?;
        self.language = language;
        Ok(())
    }
}

impl ParseResult {
    /// Get the root node of the syntax tree
    pub fn root_node(&self) -> Node {
        self.tree.root_node()
    }
    
    /// Check if the parse result has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// Get all parse errors
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
    
    /// Get the source content
    pub fn source(&self) -> &str {
        &self.source
    }
    
    /// Get the syntax tree
    pub fn tree(&self) -> &Tree {
        &self.tree
    }
    
    /// Create a tree cursor for traversing the syntax tree
    pub fn walk(&self) -> TreeCursor {
        self.tree.walk()
    }
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: String, line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            message,
            line,
            column,
            offset,
            length,
        }
    }
    
    /// Get the error message
    pub fn message(&self) -> &str {
        &self.message
    }
    
    /// Get the line number (0-based)
    pub fn line(&self) -> usize {
        self.line
    }
    
    /// Get the column number (0-based)
    pub fn column(&self) -> usize {
        self.column
    }
    
    /// Get the byte offset
    pub fn offset(&self) -> usize {
        self.offset
    }
    
    /// Get the error span length
    pub fn length(&self) -> usize {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parser_creation() {
        let parser = FshParser::new();
        assert!(parser.is_ok());
    }
    
    #[test]
    fn test_parse_empty_content() {
        let mut parser = FshParser::new().unwrap();
        let result = parser.parse("", None);
        assert!(result.is_ok());
        
        let parse_result = result.unwrap();
        assert_eq!(parse_result.source(), "");
    }
    
    #[test]
    fn test_parse_simple_fsh() {
        let mut parser = FshParser::new().unwrap();
        let fsh_content = r#"
Profile: MyPatient
Parent: Patient
* name 1..1
"#;
        
        let result = parser.parse(fsh_content, None);
        assert!(result.is_ok());
        
        let parse_result = result.unwrap();
        assert_eq!(parse_result.source(), fsh_content);
        assert!(!parse_result.root_node().is_error());
    }
    
    #[test]
    fn test_incremental_parsing() {
        let mut parser = FshParser::new().unwrap();
        let initial_content = "Profile: MyPatient\nParent: Patient";
        
        let initial_result = parser.parse(initial_content, None).unwrap();
        
        let updated_content = "Profile: MyPatient\nParent: Patient\n* name 1..1";
        let incremental_result = parser.parse_incremental(updated_content, &initial_result.tree);
        
        assert!(incremental_result.is_ok());
        assert_eq!(incremental_result.unwrap().source(), updated_content);
    }
    
    #[test]
    fn test_cached_parser_creation() {
        let parser = CachedFshParser::new();
        assert!(parser.is_ok());
        
        let config = ParserConfig {
            enable_cache: true,
            cache_capacity: 500,
        };
        let parser_with_config = CachedFshParser::with_config(config);
        assert!(parser_with_config.is_ok());
    }
    
    #[test]
    fn test_cached_parser_caching() {
        let mut parser = CachedFshParser::new().unwrap();
        let content = "Profile: MyPatient\nParent: Patient\n* name 1..1";
        
        // First parse - should cache the result
        let result1 = parser.parse_with_cache(content).unwrap();
        let stats_after_first = parser.cache_stats();
        assert_eq!(stats_after_first.size, 1);
        
        // Second parse - should return cached result
        let result2 = parser.parse_with_cache(content).unwrap();
        let stats_after_second = parser.cache_stats();
        assert_eq!(stats_after_second.size, 1);
        
        // Results should be the same (Arc pointing to same data)
        assert!(Arc::ptr_eq(&result1, &result2));
        assert_eq!(result1.source(), content);
        assert_eq!(result2.source(), content);
    }
    
    #[test]
    fn test_cached_parser_invalidation() {
        let mut parser = CachedFshParser::new().unwrap();
        let content = "Profile: MyPatient\nParent: Patient";
        
        // Parse and cache
        let _result = parser.parse_with_cache(content).unwrap();
        assert_eq!(parser.cache_stats().size, 1);
        
        // Invalidate specific content
        parser.invalidate(content);
        assert_eq!(parser.cache_stats().size, 0);
        
        // Parse again and cache
        let _result = parser.parse_with_cache(content).unwrap();
        assert_eq!(parser.cache_stats().size, 1);
        
        // Clear all cache
        parser.clear_cache();
        assert_eq!(parser.cache_stats().size, 0);
    }
    
    #[test]
    fn test_cached_parser_trait_implementation() {
        let mut parser = CachedFshParser::new().unwrap();
        let content = "Profile: MyPatient\nParent: Patient";
        
        // Test Parser trait methods
        let result = parser.parse(content, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().source(), content);
        
        // Test incremental parsing through trait
        let initial_result = parser.parse(content, None).unwrap();
        let updated_content = "Profile: MyPatient\nParent: Patient\n* name 1..1";
        let incremental_result = parser.parse_incremental(updated_content, &initial_result.tree);
        assert!(incremental_result.is_ok());
        assert_eq!(incremental_result.unwrap().source(), updated_content);
    }
    
    #[test]
    fn test_cached_parser_language_change_clears_cache() {
        let mut parser = CachedFshParser::new().unwrap();
        let content = "Profile: MyPatient\nParent: Patient";
        
        // Parse and cache
        let _result = parser.parse_with_cache(content).unwrap();
        assert_eq!(parser.cache_stats().size, 1);
        
        // Change language (should clear cache)
        let language = parser.language();
        let result = parser.set_language(language);
        assert!(result.is_ok());
        assert_eq!(parser.cache_stats().size, 0);
    }
    
    #[test]
    fn test_parser_config() {
        let config = ParserConfig::default();
        assert!(config.enable_cache);
        assert_eq!(config.cache_capacity, 1000);
        
        let custom_config = ParserConfig {
            enable_cache: false,
            cache_capacity: 100,
        };
        assert!(!custom_config.enable_cache);
        assert_eq!(custom_config.cache_capacity, 100);
    }
}