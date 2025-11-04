use std::ops::Range;
use std::sync::Arc;

use crate::Result;
use crate::cst::FshSyntaxNode;

/// Outcome of parsing FSH content.
#[derive(Debug)]
pub struct ParseResult {
    /// Original source that was parsed.
    pub source: Arc<str>,
    /// Parsed CST root node.
    pub cst: FshSyntaxNode,
    /// Combined lexer and parser errors.
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    pub fn cst(&self) -> &FshSyntaxNode {
        &self.cst
    }
}

pub trait Parser {
    fn parse(&mut self, content: &str) -> Result<ParseResult>;
}

/// Async-first FSH parser - stateless and concurrent-safe
///
/// The parser is designed to be used in async contexts with `Arc` sharing.
/// It's stateless, so no locks are needed - just wrap in Arc<Mutex<>> if
/// needed for the trait's &mut self requirement.
#[derive(Default)]
pub struct FshParser;

impl FshParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse FSH content (stateless, thread-safe)
    ///
    /// Core parsing logic as pure function.
    /// Can be called safely from multiple threads/tasks.
    pub fn parse_content(content: &str) -> Result<ParseResult> {
        let source: Arc<str> = Arc::from(content);

        // Use CST parser exclusively
        let (cst, lex_errors, _parse_errors) = crate::cst::parse_fsh(&source);

        // Convert lexer errors to ParseError
        let mut errors = Vec::new();
        for error in lex_errors {
            errors.push(ParseError::from_span(
                &source,
                error.message,
                error.span.start..error.span.end,
                ParseErrorKind::Lexer,
            ));
        }

        Ok(ParseResult {
            source,
            cst,
            errors,
        })
    }

    /// Parse FSH content asynchronously
    ///
    /// Async wrapper for parse_content.
    /// Zero overhead, just API compatibility for async contexts.
    pub async fn parse_async(content: &str) -> Result<ParseResult> {
        Self::parse_content(content)
    }
}

impl Parser for FshParser {
    fn parse(&mut self, content: &str) -> Result<ParseResult> {
        Self::parse_content(content)
    }
}

pub struct CachedFshParser {
    parser: FshParser,
    cache: crate::cache::ParseResultCache,
    config: ParserConfig,
}

impl CachedFshParser {
    pub fn new() -> Result<Self> {
        Self::with_config(ParserConfig::default())
    }

    pub fn with_config(config: ParserConfig) -> Result<Self> {
        Ok(Self {
            parser: FshParser::new(),
            cache: if config.enable_cache {
                crate::cache::ParseResultCache::with_capacity(config.cache_capacity)
            } else {
                crate::cache::ParseResultCache::with_capacity(0)
            },
            config,
        })
    }

    pub fn parse_with_cache(&mut self, content: &str) -> Result<Arc<ParseResult>> {
        if !self.config.enable_cache {
            return Ok(Arc::new(self.parser.parse(content)?));
        }

        let hash = crate::cache::ContentHash::from_content(content);

        if let Some(result) = self.cache.get(&hash) {
            return Ok(result);
        }

        let result = Arc::new(self.parser.parse(content)?);
        self.cache.insert_arc(hash, Arc::clone(&result));
        Ok(result)
    }

    pub fn invalidate(&self, content: &str) {
        let hash = crate::cache::ContentHash::from_content(content);
        self.cache.remove(&hash);
    }

    pub fn clear_cache(&self) {
        self.cache.invalidate_all();
    }

    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }
}

impl Parser for CachedFshParser {
    fn parse(&mut self, content: &str) -> Result<ParseResult> {
        // CST cannot be cloned, parse directly
        FshParser::parse_content(content)
    }
}

#[derive(Debug, Clone)]
pub struct ParserConfig {
    pub enable_cache: bool,
    pub cache_capacity: usize,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            cache_capacity: 1000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub offset: usize,
    pub length: usize,
    pub span: Range<usize>,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    Lexer,
    Parser,
}

impl ParseError {
    // Old error conversion functions removed - using CST lexer errors directly

    fn from_span(source: &str, message: String, span: Range<usize>, kind: ParseErrorKind) -> Self {
        let (line, column) = offset_to_line_col(source, span.start);
        let length = span.end.saturating_sub(span.start);
        Self {
            message,
            line,
            column,
            offset: span.start,
            length,
            span,
            kind,
        }
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut column = 0usize;
    let mut current = 0usize;

    for ch in source.chars() {
        if current >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }

        current += ch.len_utf8();
    }

    (line, column)
}
