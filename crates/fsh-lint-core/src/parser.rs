use std::ops::Range;
use std::sync::Arc;

use chumsky::span::Span;

use crate::Result;
use crate::ast::FSHDocument;
use crate::lexer::{LexSpan, lex};
use crate::parser_chumsky::parse_tokens;

/// Outcome of parsing FSH content.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Original source that was parsed.
    pub source: Arc<str>,
    /// Parsed document AST.
    pub document: Option<FSHDocument>,
    /// Combined lexer and parser errors.
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn is_valid(&self) -> bool {
        self.document.is_some() && self.errors.is_empty()
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }
}

pub trait Parser {
    fn parse(&mut self, content: &str) -> Result<ParseResult>;
}

#[derive(Default)]
pub struct FshParser;

impl FshParser {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Parser for FshParser {
    fn parse(&mut self, content: &str) -> Result<ParseResult> {
        let source: Arc<str> = Arc::from(content);

        let (tokens, lex_errors) = lex(&source);
        let eof = LexSpan::new((), source.len()..source.len());
        let (document, parse_errors) = parse_tokens(&tokens, eof);

        let mut errors = Vec::new();
        for error in lex_errors {
            errors.push(ParseError::from_lexer_error(&source, error));
        }
        for error in parse_errors {
            errors.push(ParseError::from_parser_error(&source, error));
        }

        Ok(ParseResult {
            source,
            document,
            errors,
        })
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
        let result = self.parse_with_cache(content)?;
        Ok((*result).clone())
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
    fn from_lexer_error(source: &str, error: crate::lexer::LexerError) -> Self {
        Self::from_span(
            source,
            error.message,
            error.span.start..error.span.end,
            ParseErrorKind::Lexer,
        )
    }

    fn from_parser_error(
        source: &str,
        error: chumsky::error::Rich<'_, crate::lexer::Token>,
    ) -> Self {
        let message = error.to_string();
        let span = error.span();
        Self::from_span(
            source,
            message,
            span.start..span.end,
            ParseErrorKind::Parser,
        )
    }

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
