//! CST-aware lexer that preserves all trivia (whitespace, comments)
//!
//! This lexer is specifically designed for CST construction. Unlike the original
//! lexer which skips whitespace and comments, this version preserves ALL source
//! information to enable lossless round-tripping.

use crate::cst::FshSyntaxKind;
use std::ops::Range;

/// Simple span representing a range in the source
pub type CstSpan = Range<usize>;

/// A lexer error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub message: String,
    pub span: CstSpan,
}

impl LexerError {
    pub fn new(message: impl Into<String>, span: CstSpan) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

/// A token with its syntax kind and span
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstToken {
    pub kind: FshSyntaxKind,
    pub text: String,
    pub span: CstSpan,
}

impl CstToken {
    pub fn new(kind: FshSyntaxKind, text: impl Into<String>, span: CstSpan) -> Self {
        Self {
            kind,
            text: text.into(),
            span,
        }
    }
}

/// Result returned by the CST lexer
pub type CstLexResult = (Vec<CstToken>, Vec<LexerError>);

/// Lex input preserving ALL trivia for CST construction
///
/// This is the key difference from the original lexer:
/// - Preserves whitespace as WHITESPACE tokens
/// - Preserves comments as COMMENT_LINE/COMMENT_BLOCK tokens
/// - Preserves newlines as NEWLINE tokens
///
/// This enables lossless round-tripping: parse(source).text() == source
pub fn lex_with_trivia(input: &str) -> CstLexResult {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        let ch = match next_char(input, i) {
            Some((c, size)) => (c, size),
            None => break,
        };

        let (current, size) = ch;
        let start = i;

        match current {
            // Newlines (separate from whitespace for formatting purposes)
            '\n' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Newline,
                    "\n",
                    span(start, i + size),
                ));
                i += size;
            }
            '\r' => {
                // Handle \r\n as single newline
                let mut end = i + size;
                if let Some(('\n', nl_size)) = next_char(input, end) {
                    end += nl_size;
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Newline,
                        &input[start..end],
                        span(start, end),
                    ));
                    i = end;
                } else {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Newline,
                        "\r",
                        span(start, end),
                    ));
                    i = end;
                }
            }

            // Whitespace (spaces, tabs) - PRESERVE IT!
            c if c.is_whitespace() && c != '\n' && c != '\r' => {
                let mut end = i + size;
                // Consume consecutive whitespace
                while end < len {
                    if let Some((next_ch, next_size)) = next_char(input, end) {
                        if next_ch.is_whitespace() && next_ch != '\n' && next_ch != '\r' {
                            end += next_size;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                tokens.push(CstToken::new(
                    FshSyntaxKind::Whitespace,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
            }

            // Comments - PRESERVE THEM!
            '/' => {
                if let Some((next, next_size)) = next_char(input, i + size) {
                    if next == '/' {
                        // Line comment
                        let mut end = i + size + next_size;
                        while end < len {
                            if let Some((c, step)) = next_char(input, end) {
                                if c == '\n' {
                                    // Don't include the newline in the comment
                                    break;
                                }
                                end += step;
                            } else {
                                break;
                            }
                        }
                        tokens.push(CstToken::new(
                            FshSyntaxKind::CommentLine,
                            &input[start..end],
                            span(start, end),
                        ));
                        i = end;
                        continue;
                    } else if next == '*' {
                        // Block comment
                        let mut end = i + size + next_size;
                        let mut terminated = false;
                        while end < len {
                            if let Some((c, step)) = next_char(input, end) {
                                if c == '*' {
                                    if let Some((peek, peek_size)) = next_char(input, end + step) {
                                        if peek == '/' {
                                            end += step + peek_size;
                                            terminated = true;
                                            break;
                                        }
                                    }
                                }
                                end += step;
                            } else {
                                break;
                            }
                        }
                        if !terminated {
                            errors.push(LexerError::new(
                                "Unterminated block comment",
                                span(start, len),
                            ));
                        }
                        tokens.push(CstToken::new(
                            FshSyntaxKind::CommentBlock,
                            &input[start..end],
                            span(start, end),
                        ));
                        i = end;
                        continue;
                    }
                }
                // Not a comment, fall through to handle as regular token
                let (token_kind, end) = lex_regular_token(input, start);
                tokens.push(CstToken::new(
                    token_kind,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
            }

            // Punctuation and operators
            '*' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Asterisk,
                    "*",
                    span(start, i + size),
                ));
                i += size;
            }
            ':' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Colon,
                    ":",
                    span(start, i + size),
                ));
                i += size;
            }
            '=' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Equals,
                    "=",
                    span(start, i + size),
                ));
                i += size;
            }
            '+' => {
                // Check for +=
                if let Some(('=', eq_size)) = next_char(input, i + size) {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::PlusEquals,
                        "+=",
                        span(start, i + size + eq_size),
                    ));
                    i += size + eq_size;
                } else {
                    // Just a plus (for soft indexing [+])
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Plus,
                        "+",
                        span(start, i + size),
                    ));
                    i += size;
                }
            }
            '^' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Caret,
                    "^",
                    span(start, i + size),
                ));
                i += size;
            }
            '.' => {
                // Check for range operator (..)
                if let Some(('.', dot_size)) = next_char(input, i + size) {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Range,
                        "..",
                        span(start, i + size + dot_size),
                    ));
                    i += size + dot_size;
                } else {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Dot,
                        ".",
                        span(start, i + size),
                    ));
                    i += size;
                }
            }
            '#' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Hash,
                    "#",
                    span(start, i + size),
                ));
                i += size;
            }
            '-' => {
                // Check for arrow ->
                if let Some(('>', arrow_size)) = next_char(input, i + size) {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Arrow,
                        "->",
                        span(start, i + size + arrow_size),
                    ));
                    i += size + arrow_size;
                } else {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Minus,
                        "-",
                        span(start, i + size),
                    ));
                    i += size;
                }
            }
            '>' => {
                tokens.push(CstToken::new(FshSyntaxKind::Gt, ">", span(start, i + size)));
                i += size;
            }
            '<' => {
                tokens.push(CstToken::new(FshSyntaxKind::Lt, "<", span(start, i + size)));
                i += size;
            }
            '?' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Question,
                    "?",
                    span(start, i + size),
                ));
                i += size;
            }
            '!' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Exclamation,
                    "!",
                    span(start, i + size),
                ));
                i += size;
            }
            '%' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Percent,
                    "%",
                    span(start, i + size),
                ));
                i += size;
            }
            '\'' => {
                // UCUM unit: 'mg', 'kg', etc.
                let (unit_kind, end, unit_error) = lex_unit(input, start);
                if let Some(err) = unit_error {
                    errors.push(err);
                }
                tokens.push(CstToken::new(
                    unit_kind,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
            }
            '\\' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Backslash,
                    "\\",
                    span(start, i + size),
                ));
                i += size;
            }
            '(' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::LParen,
                    "(",
                    span(start, i + size),
                ));
                i += size;
            }
            ')' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::RParen,
                    ")",
                    span(start, i + size),
                ));
                i += size;
            }
            '[' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::LBracket,
                    "[",
                    span(start, i + size),
                ));
                i += size;
            }
            ']' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::RBracket,
                    "]",
                    span(start, i + size),
                ));
                i += size;
            }
            '{' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::LBrace,
                    "{",
                    span(start, i + size),
                ));
                i += size;
            }
            '}' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::RBrace,
                    "}",
                    span(start, i + size),
                ));
                i += size;
            }
            ',' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::Comma,
                    ",",
                    span(start, i + size),
                ));
                i += size;
            }

            // String literals
            '"' => {
                let (string_kind, end, string_error) = lex_string(input, start);
                if let Some(err) = string_error {
                    errors.push(err);
                }
                tokens.push(CstToken::new(
                    string_kind,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
            }

            // Numbers, identifiers, keywords
            _ if current.is_ascii_digit() => {
                let (kind, end) = lex_number(input, start);
                tokens.push(CstToken::new(kind, &input[start..end], span(start, end)));
                i = end;
            }

            _ if current.is_alphabetic() || current == '_' => {
                let (kind, end) = lex_word(input, start);
                tokens.push(CstToken::new(kind, &input[start..end], span(start, end)));
                i = end;
            }

            // Unknown character - create error token
            _ => {
                errors.push(LexerError::new(
                    format!("Unexpected character: '{current}'"),
                    span(start, i + size),
                ));
                tokens.push(CstToken::new(
                    FshSyntaxKind::Error,
                    &input[start..i + size],
                    span(start, i + size),
                ));
                i += size;
            }
        }
    }

    // Add EOF token
    tokens.push(CstToken::new(FshSyntaxKind::Eof, "", span(len, len)));

    (tokens, errors)
}

/// Lex a word (identifier or keyword)
fn lex_word(input: &str, start: usize) -> (FshSyntaxKind, usize) {
    let (word, end) = read_word(input, start);

    let kind = match word.as_str() {
        // Definition keywords
        "Alias" => FshSyntaxKind::AliasKw,
        "Profile" => FshSyntaxKind::ProfileKw,
        "Extension" => FshSyntaxKind::ExtensionKw,
        "ValueSet" => FshSyntaxKind::ValuesetKw,
        "CodeSystem" => FshSyntaxKind::CodesystemKw,
        "Instance" => FshSyntaxKind::InstanceKw,
        "Invariant" => FshSyntaxKind::InvariantKw,
        "Mapping" => FshSyntaxKind::MappingKw,
        "Logical" => FshSyntaxKind::LogicalKw,
        "Resource" => FshSyntaxKind::ResourceKw,
        "RuleSet" => FshSyntaxKind::RulesetKw,

        // Metadata keywords
        "Parent" => FshSyntaxKind::ParentKw,
        "Id" => FshSyntaxKind::IdKw,
        "Title" => FshSyntaxKind::TitleKw,
        "Description" => FshSyntaxKind::DescriptionKw,
        "Expression" => FshSyntaxKind::ExpressionKw,
        "XPath" => FshSyntaxKind::XpathKw,
        "Severity" => FshSyntaxKind::SeverityKw,
        "InstanceOf" => FshSyntaxKind::InstanceofKw,
        "Usage" => FshSyntaxKind::UsageKw,
        "Source" => FshSyntaxKind::SourceKw,
        "Target" => FshSyntaxKind::TargetKw,
        "Context" => FshSyntaxKind::ContextKw,
        "Characteristics" => FshSyntaxKind::CharacteristicsKw,

        // Rule keywords
        "from" => FshSyntaxKind::FromKw,
        "only" => FshSyntaxKind::OnlyKw,
        "obeys" => FshSyntaxKind::ObeysKw,
        "contains" => FshSyntaxKind::ContainsKw,
        "named" => FshSyntaxKind::NamedKw,
        "and" => FshSyntaxKind::AndKw,
        "or" => FshSyntaxKind::OrKw,
        "insert" => FshSyntaxKind::InsertKw,
        "include" => FshSyntaxKind::IncludeKw,
        "exclude" => FshSyntaxKind::ExcludeKw,
        "codes" => FshSyntaxKind::CodesKw,
        "where" => FshSyntaxKind::WhereKw,
        "system" => FshSyntaxKind::SystemKw,
        "valueset" => FshSyntaxKind::ValuesetRefKw,
        "contentreference" => FshSyntaxKind::ContentreferenceKw,

        // Binding strength
        "required" => FshSyntaxKind::RequiredKw,
        "extensible" => FshSyntaxKind::ExtensibleKw,
        "preferred" => FshSyntaxKind::PreferredKw,
        "example" => FshSyntaxKind::ExampleKw,

        // Flags
        "MS" => FshSyntaxKind::MsFlag,
        "SU" => FshSyntaxKind::SuFlag,
        "TU" => FshSyntaxKind::TuFlag,
        "N" => FshSyntaxKind::NFlag,
        "D" => FshSyntaxKind::DFlag,
        "?!" => FshSyntaxKind::ModifierFlag,

        // Boolean
        "true" => FshSyntaxKind::True,
        "false" => FshSyntaxKind::False,

        // Otherwise, it's an identifier
        _ => FshSyntaxKind::Ident,
    };

    (kind, end)
}

/// Lex a number
fn lex_number(input: &str, start: usize) -> (FshSyntaxKind, usize) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let mut has_dot = false;

    while i < len {
        match bytes[i] as char {
            '0'..='9' => i += 1,
            '.' => {
                if has_dot {
                    break; // Second dot, not part of this number
                }
                // Lookahead: if next char is also '.', this is a range operator, not a decimal
                if i + 1 < len && bytes[i + 1] == b'.' {
                    break; // This is '..' range operator, stop here
                }
                has_dot = true;
                i += 1;
            }
            _ => break,
        }
    }

    let kind = if has_dot {
        FshSyntaxKind::Decimal
    } else {
        FshSyntaxKind::Integer
    };

    (kind, i)
}

/// Lex a string literal
fn lex_string(input: &str, start: usize) -> (FshSyntaxKind, usize, Option<LexerError>) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start + 1; // Skip opening quote

    // Check for triple-quoted string
    if i + 1 < len && bytes[i] == b'"' && bytes[i + 1] == b'"' {
        // Multiline string """..."""
        i += 2; // Skip the other two quotes
        while i < len {
            if i + 2 < len && bytes[i] == b'"' && bytes[i + 1] == b'"' && bytes[i + 2] == b'"' {
                return (FshSyntaxKind::String, i + 3, None);
            }
            i += 1;
        }
        return (
            FshSyntaxKind::String,
            len,
            Some(LexerError::new(
                "Unterminated multiline string",
                span(start, len),
            )),
        );
    }

    // Regular string "..."
    while i < len {
        match bytes[i] as char {
            '"' => return (FshSyntaxKind::String, i + 1, None),
            '\\' => {
                i += 1; // Skip escape char
                if i < len {
                    i += 1; // Skip escaped char
                }
            }
            _ => i += 1,
        }
    }

    (
        FshSyntaxKind::String,
        len,
        Some(LexerError::new("Unterminated string", span(start, len))),
    )
}

/// Lex a UCUM unit 'unit'
fn lex_unit(input: &str, start: usize) -> (FshSyntaxKind, usize, Option<LexerError>) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start + 1; // Skip opening single quote

    // Scan for closing single quote
    while i < len {
        match bytes[i] as char {
            '\'' => return (FshSyntaxKind::Unit, i + 1, None),
            '\\' => {
                // Handle escaped characters (e.g., '\'' inside unit)
                i += 1; // Skip escape char
                if i < len {
                    i += 1; // Skip escaped char
                }
            }
            '\n' | '\r' => {
                // Newline before closing quote - probably malformed
                return (
                    FshSyntaxKind::Unit,
                    i,
                    Some(LexerError::new(
                        "Unterminated unit (newline found)",
                        span(start, i),
                    )),
                );
            }
            _ => i += 1,
        }
    }

    // Reached end of input without closing quote
    (
        FshSyntaxKind::Unit,
        len,
        Some(LexerError::new("Unterminated unit", span(start, len))),
    )
}

/// Lex a regular token (non-trivia, non-special)
fn lex_regular_token(input: &str, start: usize) -> (FshSyntaxKind, usize) {
    // Fallback for anything else - treat as identifier
    let (_, end) = read_word(input, start);
    (FshSyntaxKind::Ident, end)
}

/// Read a word (sequence of alphanumeric/underscore chars)
fn read_word(input: &str, start: usize) -> (String, usize) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start;

    while i < len {
        let ch = bytes[i] as char;
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/' || ch == '.' {
            i += 1;
        } else {
            break;
        }
    }

    (input[start..i].to_string(), i)
}

/// Get next character and its UTF-8 size
fn next_char(input: &str, pos: usize) -> Option<(char, usize)> {
    input[pos..].chars().next().map(|c| (c, c.len_utf8()))
}

/// Create a span from start to end
fn span(start: usize, end: usize) -> CstSpan {
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserves_whitespace() {
        let input = "Profile:  MyPatient";
        let (tokens, _) = lex_with_trivia(input);

        // Should have: PROFILE_KW, COLON, WHITESPACE (2 spaces), IDENT, EOF
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, FshSyntaxKind::ProfileKw);
        assert_eq!(tokens[1].kind, FshSyntaxKind::Colon);
        assert_eq!(tokens[2].kind, FshSyntaxKind::Whitespace);
        assert_eq!(tokens[2].text, "  "); // Two spaces preserved!
        assert_eq!(tokens[3].kind, FshSyntaxKind::Ident);
    }

    #[test]
    fn test_preserves_comments() {
        let input = "Profile: MyPatient // comment";
        let (tokens, _) = lex_with_trivia(input);

        // Find comment token
        let comment = tokens.iter().find(|t| t.kind == FshSyntaxKind::CommentLine);
        assert!(comment.is_some());
        assert_eq!(comment.unwrap().text, "// comment");
    }

    #[test]
    fn test_preserves_newlines() {
        let input = "Profile: MyPatient\n\nParent: Patient";
        let (tokens, _) = lex_with_trivia(input);

        let newlines: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == FshSyntaxKind::Newline)
            .collect();
        assert_eq!(newlines.len(), 2); // Two newlines preserved
    }

    #[test]
    fn test_lossless_reconstruction() {
        let input = "Profile:  MyPatient // comment\n* name 1..1 MS";
        let (tokens, _) = lex_with_trivia(input);

        // Reconstruct source from tokens (excluding EOF)
        let reconstructed: String = tokens
            .iter()
            .filter(|t| t.kind != FshSyntaxKind::Eof)
            .map(|t| t.text.as_str())
            .collect();

        assert_eq!(reconstructed, input);
    }

    #[test]
    fn test_block_comment() {
        let input = "Profile: MyPatient /* block\ncomment */ Parent: Patient";
        let (tokens, _) = lex_with_trivia(input);

        let block = tokens
            .iter()
            .find(|t| t.kind == FshSyntaxKind::CommentBlock);
        assert!(block.is_some());
        assert!(block.unwrap().text.contains("block\ncomment"));
    }
}
