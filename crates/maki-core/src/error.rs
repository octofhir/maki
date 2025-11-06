//! Error types and handling for FSH linting operations

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for FSH linting operations
#[derive(Debug, Error)]
pub enum MakiError {
    /// Parse errors from tree-sitter or FSH syntax issues
    #[error("Parse error: {message} at {location}")]
    ParseError {
        message: String,
        location: Box<crate::diagnostics::Location>,
    },

    /// Specific syntax error types for better error reporting
    #[error("Unclosed parameter bracket at line {line}, column {col}")]
    UnclosedParameterBracket { line: u32, col: u32 },

    #[error("Invalid grammar construct: {construct} at line {line}, column {col}")]
    InvalidGrammarConstruct {
        construct: String,
        line: u32,
        col: u32,
    },

    #[error("Malformed escape sequence: {sequence} at line {line}, column {col}")]
    MalformedEscapeSequence {
        sequence: String,
        line: u32,
        col: u32,
    },

    #[error("Recursive RuleSet insertion detected: {ruleset_chain}")]
    RecursiveRuleSetInsertion { ruleset_chain: String },

    #[error("Circular dependency in RuleSet: {dependency_chain}")]
    CircularRuleSetDependency { dependency_chain: String },

    /// Configuration loading or validation errors
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// Rule compilation or execution errors
    #[error("Rule error in '{rule_id}': {message}")]
    RuleError { rule_id: String, message: String },

    /// File system I/O errors
    #[error("IO error for path '{path}': {source}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Semantic analysis errors
    #[error("Semantic error: {message}")]
    SemanticError { message: String },

    /// Cache-related errors
    #[error("Cache error: {message}")]
    CacheError { message: String },

    /// Rule engine initialization errors
    #[error("Rule engine error: {message}")]
    RuleEngineError { message: String },

    /// Formatter errors
    #[error("Formatter error: {message}")]
    FormatterError { message: String },

    /// LSP server errors
    #[error("LSP error: {message}")]
    LspError { message: String },

    /// Execution engine errors
    #[error("Execution error: {message}")]
    ExecutionError { message: String },

    /// Autofix engine errors
    #[error("Autofix error: {message}")]
    AutofixError { message: String },

    /// Resource limit errors (memory, CPU, etc.)
    #[error("Resource limit exceeded for {resource}: current {current}, limit {limit}")]
    ResourceLimit {
        resource: String,
        current: String,
        limit: String,
    },

    /// Generic internal errors
    #[error("Internal error: {message}")]
    InternalError { message: String },
}

/// Error kind enumeration for categorizing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Parse,
    Config,
    Rule,
    Io,
    Semantic,
    Cache,
    RuleEngine,
    Formatter,
    Lsp,
    Execution,
    Autofix,
    ResourceLimit,
    Internal,
}

impl MakiError {
    /// Get the error kind for this error
    pub fn kind(&self) -> ErrorKind {
        match self {
            MakiError::ParseError { .. } => ErrorKind::Parse,
            MakiError::ConfigError { .. } => ErrorKind::Config,
            MakiError::RuleError { .. } => ErrorKind::Rule,
            MakiError::IoError { .. } => ErrorKind::Io,
            MakiError::SemanticError { .. } => ErrorKind::Semantic,
            MakiError::CacheError { .. } => ErrorKind::Cache,
            MakiError::RuleEngineError { .. } => ErrorKind::RuleEngine,
            MakiError::FormatterError { .. } => ErrorKind::Formatter,
            MakiError::LspError { .. } => ErrorKind::Lsp,
            MakiError::ExecutionError { .. } => ErrorKind::Execution,
            MakiError::AutofixError { .. } => ErrorKind::Autofix,
            MakiError::ResourceLimit { .. } => ErrorKind::ResourceLimit,
            MakiError::InternalError { .. } => ErrorKind::Internal,
            MakiError::UnclosedParameterBracket { .. } => ErrorKind::Parse,
            MakiError::InvalidGrammarConstruct { .. } => ErrorKind::Parse,
            MakiError::MalformedEscapeSequence { .. } => ErrorKind::Parse,
            MakiError::RecursiveRuleSetInsertion { .. } => ErrorKind::Parse,
            MakiError::CircularRuleSetDependency { .. } => ErrorKind::Parse,
        }
    }

    /// Check if this error is recoverable (can continue processing other files)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self.kind(),
            ErrorKind::Parse | ErrorKind::Rule | ErrorKind::Semantic
        )
    }

    /// Create a parse error
    pub fn parse_error(message: impl Into<String>, location: crate::diagnostics::Location) -> Self {
        Self::ParseError {
            message: message.into(),
            location: Box::new(location),
        }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// Create a rule error
    pub fn rule_error(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RuleError {
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }

    /// Create an IO error with path context
    pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::IoError {
            path: path.into(),
            source,
        }
    }

    /// Create a semantic error
    pub fn semantic_error(message: impl Into<String>) -> Self {
        Self::SemanticError {
            message: message.into(),
        }
    }

    /// Create an autofix error
    pub fn autofix_error(message: impl Into<String>) -> Self {
        Self::AutofixError {
            message: message.into(),
        }
    }

    /// Create a parser error with a simple message (for tree-sitter errors)
    pub fn parser_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            location: Box::new(crate::diagnostics::Location::default()),
        }
    }

    /// Create an unclosed parameter bracket error
    pub fn unclosed_parameter_bracket(line: u32, col: u32) -> Self {
        Self::UnclosedParameterBracket { line, col }
    }

    /// Create an invalid grammar construct error
    pub fn invalid_grammar_construct(construct: impl Into<String>, line: u32, col: u32) -> Self {
        Self::InvalidGrammarConstruct {
            construct: construct.into(),
            line,
            col,
        }
    }

    /// Create a malformed escape sequence error
    pub fn malformed_escape_sequence(sequence: impl Into<String>, line: u32, col: u32) -> Self {
        Self::MalformedEscapeSequence {
            sequence: sequence.into(),
            line,
            col,
        }
    }

    /// Create a recursive RuleSet insertion error
    pub fn recursive_ruleset_insertion(ruleset_chain: impl Into<String>) -> Self {
        Self::RecursiveRuleSetInsertion {
            ruleset_chain: ruleset_chain.into(),
        }
    }

    /// Create a circular RuleSet dependency error
    pub fn circular_ruleset_dependency(dependency_chain: impl Into<String>) -> Self {
        Self::CircularRuleSetDependency {
            dependency_chain: dependency_chain.into(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for MakiError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            path: PathBuf::new(),
            source: err,
        }
    }
}
