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
    let mut param_stack: Vec<bool> = Vec::new();
    let mut last_non_trivia_token: Option<FshSyntaxKind> = None;

    while i < len {
        let ch = match next_char(input, i) {
            Some((c, size)) => (c, size),
            None => break,
        };

        let (current, size) = ch;
        let start = i;

        // Parameter context handling (RuleSet/insert arguments)
        if param_stack.last().copied().unwrap_or(false) {
            if current == '['
                && let Some((next_bracket, _)) = next_char(input, i + size)
                && next_bracket == '['
            {
                let (end, param_error) = lex_bracketed_param(input, start);
                if let Some(err) = param_error {
                    errors.push(err);
                }
                tokens.push(CstToken::new(
                    FshSyntaxKind::BracketedParamToken,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
                continue;
            }

            if current != ',' && current != ')' && !current.is_whitespace() {
                let (end, param_error) = lex_plain_param(input, start);
                if let Some(err) = param_error {
                    errors.push(err);
                }
                tokens.push(CstToken::new(
                    FshSyntaxKind::PlainParamToken,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
                continue;
            }
        }

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
                    // Check if this is :// or // in URL (protocol separator or path) instead of a comment
                    // If last token was Colon or Slash, treat // as part of URL, not a comment
                    let is_url_separator = matches!(
                        last_non_trivia_token,
                        Some(FshSyntaxKind::Colon) | Some(FshSyntaxKind::Slash)
                    );

                    if next == '/' && !is_url_separator {
                        // Line comment (but not if preceded by colon, e.g., http://)
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
                                if c == '*'
                                    && let Some((peek, peek_size)) = next_char(input, end + step)
                                    && peek == '/'
                                {
                                    end += step + peek_size;
                                    terminated = true;
                                    break;
                                }
                                end += step;
                            } else {
                                break;
                            }
                        }
                        if !terminated {
                            // SUSHI tolerates stray '/*' used as a line comment (common in mCODE).
                            // Fall back to a line-style comment instead of swallowing the rest of the file.
                            if let Some(rel_nl) = input[start..].find('\n') {
                                end = start + rel_nl;
                            } else {
                                end = len;
                            }
                            tokens.push(CstToken::new(
                                FshSyntaxKind::CommentLine,
                                &input[start..end],
                                span(start, end),
                            ));
                            i = end;
                            continue;
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
                // Not a comment, check for regex literal (e.g., /pattern/)
                // But skip regex parsing if part of URL (after colon or slash)
                let is_url_separator = matches!(
                    last_non_trivia_token,
                    Some(FshSyntaxKind::Colon) | Some(FshSyntaxKind::Slash)
                );

                if !is_url_separator {
                    // Try regex literal
                    if let Some((end, regex_error)) = lex_regex_literal(input, start) {
                        if let Some(err) = regex_error {
                            errors.push(err);
                        }
                        tokens.push(CstToken::new(
                            FshSyntaxKind::Regex,
                            &input[start..end],
                            span(start, end),
                        ));
                        i = end;
                        continue;
                    }
                }

                // Fallback to regular slash token (for URLs or division)
                tokens.push(CstToken::new(
                    FshSyntaxKind::Slash,
                    "/",
                    span(start, i + size),
                ));
                last_non_trivia_token = Some(FshSyntaxKind::Slash);
                i += size;
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
                last_non_trivia_token = Some(FshSyntaxKind::Colon);
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
                let (end, code_error) = lex_code_literal(input, start);
                if let Some(err) = code_error {
                    errors.push(err);
                }
                tokens.push(CstToken::new(
                    FshSyntaxKind::Code,
                    &input[start..end],
                    span(start, end),
                ));
                i = end;
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
                // Either modifier flag '?!' or standalone '?'
                if let Some((next, next_size)) = next_char(input, i + size) && next == '!' {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::ModifierFlag,
                        "?!",
                        span(start, i + size + next_size),
                    ));
                    i += size + next_size;
                } else {
                    tokens.push(CstToken::new(
                        FshSyntaxKind::Question,
                        "?",
                        span(start, i + size),
                    ));
                    i += size;
                }
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
                let is_param_context = should_enter_param_context(&tokens);
                tokens.push(CstToken::new(
                    FshSyntaxKind::LParen,
                    "(",
                    span(start, i + size),
                ));
                param_stack.push(is_param_context);
                i += size;
            }
            ')' => {
                tokens.push(CstToken::new(
                    FshSyntaxKind::RParen,
                    ")",
                    span(start, i + size),
                ));
                param_stack.pop();
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
                if let Some((kind, end, literal_error)) = lex_special_literal(input, start) {
                    if let Some(err) = literal_error {
                        errors.push(err);
                    }
                    tokens.push(CstToken::new(kind, &input[start..end], span(start, end)));
                    // Update last_non_trivia_token for special literals (dates, times, etc.)
                    if !kind.is_trivia() {
                        last_non_trivia_token = Some(kind);
                    }
                    i = end;
                } else {
                    let (kind, end) = lex_word(input, start);
                    tokens.push(CstToken::new(kind, &input[start..end], span(start, end)));
                    // BUG FIX: Update last_non_trivia_token for identifiers and keywords
                    // This ensures inline comments after identifiers are recognized correctly
                    // e.g., "Parent: Observation  // comment" should parse the // as a comment
                    last_non_trivia_token = Some(kind);
                    i = end;
                }
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
        "contentreference" | "contentReference" => FshSyntaxKind::ContentreferenceKw,

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

        // Date/Time types
        "dateTime" => FshSyntaxKind::DateTime,
        "time" => FshSyntaxKind::Time,

        // Otherwise, it's an identifier
        _ => FshSyntaxKind::Ident,
    };

    (kind, end)
}

/// Lex a number
fn lex_special_literal(
    input: &str,
    start: usize,
) -> Option<(FshSyntaxKind, usize, Option<LexerError>)> {
    const KEYWORDS: [(&str, FshSyntaxKind); 3] = [
        ("Reference", FshSyntaxKind::Reference),
        ("Canonical", FshSyntaxKind::Canonical),
        ("CodeableReference", FshSyntaxKind::CodeableReference),
    ];

    for (keyword, kind) in KEYWORDS {
        if input[start..].starts_with(keyword) {
            let mut idx = start + keyword.len();
            let len = input.len();

            // Consume optional whitespace between keyword and '('
            while idx < len {
                if let Some((ch, size)) = next_char(input, idx) {
                    if ch.is_whitespace() {
                        idx += size;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Must have opening parenthesis immediately after keyword/whitespace
            if idx < len
                && let Some(('(', _)) = next_char(input, idx)
            {
                let (end, error) = consume_parenthesized(input, idx, keyword);
                return Some((kind, end, error));
            }

            // Keyword without parentheses is not treated as literal
            return None;
        }
    }

    None
}

fn consume_parenthesized(
    input: &str,
    open_index: usize,
    keyword: &str,
) -> (usize, Option<LexerError>) {
    let len = input.len();
    let (first_char, open_size) = match next_char(input, open_index) {
        Some(pair) => pair,
        None => {
            return (
                len,
                Some(LexerError::new(
                    format!("Unterminated {keyword} literal"),
                    span(open_index, len),
                )),
            );
        }
    };

    debug_assert_eq!(first_char, '(');

    let mut i = open_index + open_size;
    let mut depth = 1usize;

    while i < len {
        let (ch, size) = match next_char(input, i) {
            Some(pair) => pair,
            None => break,
        };

        match ch {
            '\\' => {
                // Skip escaped character
                i += size;
                if i < len
                    && let Some((_, escape_size)) = next_char(input, i)
                {
                    i += escape_size;
                }
                continue;
            }
            '(' => {
                depth += 1;
            }
            ')' => {
                depth -= 1;
                i += size;
                if depth == 0 {
                    return (i, None);
                }
                continue;
            }
            _ => {}
        }

        i += size;
    }

    let span_end = len.min(i);
    (
        span_end,
        Some(LexerError::new(
            format!("Unterminated {keyword} literal"),
            span(open_index, span_end),
        )),
    )
}

fn lex_code_literal(input: &str, start: usize) -> (usize, Option<LexerError>) {
    let len = input.len();
    let mut i = start + 1; // skip '#'

    if i >= len {
        return (
            i,
            Some(LexerError::new("Expected code after '#'", span(start, len))),
        );
    }

    if let Some((ch, _)) = next_char(input, i)
        && ch == '"'
    {
        let (_, end, error) = lex_string(input, i);
        return (end, error);
    }

    let mut consumed = false;
    while i < len {
        let (ch, size) = match next_char(input, i) {
            Some(pair) => pair,
            None => break,
        };

        if ch.is_whitespace() || matches!(ch, ',' | ')' | '(' | '[' | ']' | '{' | '}' | ';') {
            break;
        }

        consumed = true;
        i += size;
    }

    if !consumed {
        return (
            i,
            Some(LexerError::new("Expected code after '#'", span(start, i))),
        );
    }

    (i, None)
}

fn lex_regex_literal(input: &str, start: usize) -> Option<(usize, Option<LexerError>)> {
    let len = input.len();
    let mut i = start + 1; // Skip initial '/'
    let mut is_escaped = false;

    while i < len {
        let (ch, size) = next_char(input, i)?;

        if ch == '\n' || ch == '\r' {
            return Some((
                i,
                Some(LexerError::new(
                    "Unterminated regex literal",
                    span(start, i),
                )),
            ));
        }

        if ch == '/' && !is_escaped {
            return Some((i + size, None));
        }

        is_escaped = !is_escaped && ch == '\\';
        i += size;
    }

    Some((
        len,
        Some(LexerError::new(
            "Unterminated regex literal",
            span(start, len),
        )),
    ))
}

fn lex_bracketed_param(input: &str, start: usize) -> (usize, Option<LexerError>) {
    let len = input.len();
    let mut i = start + 2; // Skip opening [[

    while i < len {
        let (ch, size) = match next_char(input, i) {
            Some(pair) => pair,
            None => break,
        };

        if ch == ']'
            && let Some((next, next_size)) = next_char(input, i + size)
            && next == ']'
        {
            return (i + size + next_size, None);
        }

        i += size;
    }

    (
        len,
        Some(LexerError::new(
            "Unterminated bracketed parameter",
            span(start, len),
        )),
    )
}

fn lex_plain_param(input: &str, start: usize) -> (usize, Option<LexerError>) {
    let len = input.len();
    let mut i = start;
    let mut consumed = false;

    while i < len {
        let (ch, size) = match next_char(input, i) {
            Some(pair) => pair,
            None => break,
        };

        match ch {
            '\\' => {
                consumed = true;
                i += size;
                if i < len
                    && let Some((_, escape_size)) = next_char(input, i)
                {
                    i += escape_size;
                }
            }
            ',' | ')' => break,
            '\n' | '\r' => {
                return (
                    i,
                    Some(LexerError::new(
                        "Unterminated parameter (line break)",
                        span(start, i),
                    )),
                );
            }
            _ => {
                consumed = true;
                i += size;
            }
        }
    }

    if !consumed {
        (
            i,
            Some(LexerError::new("Expected parameter value", span(start, i))),
        )
    } else {
        (i, None)
    }
}

fn lex_number(input: &str, start: usize) -> (FshSyntaxKind, usize) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let mut has_dot = false;

    // First, collect initial digits
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

    // Check for DateTime pattern: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS...
    // Must be exactly 4 digits at start to be a year
    if !has_dot && (i - start) == 4 && i < len && bytes[i] == b'-' {
        // Could be a datetime like 2024-01-06
        let checkpoint = i;
        i += 1; // skip '-'

        // Parse month (1-2 digits)
        let month_start = i;
        while i < len && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }

        if (i - month_start) >= 1 && (i - month_start) <= 2 {
            // Valid month digits
            if i < len && bytes[i] == b'-' {
                i += 1; // skip second '-'

                // Parse day (1-2 digits)
                let day_start = i;
                while i < len && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }

                if (i - day_start) >= 1 && (i - day_start) <= 2 {
                    // Valid day digits - we have at least YYYY-MM-DD
                    // Check for optional time component
                    if i < len && bytes[i] == b'T' {
                        i += 1; // skip 'T'

                        // Try to parse time (HH:MM:SS with optional fractional seconds and timezone)
                        let time_start = i;
                        if let Some(time_end) = lex_time_component(input, time_start) {
                            i = time_end;
                        } else {
                            // Invalid time after 'T', revert to just YYYY-MM-DD
                            i = day_start + (i - day_start);
                        }
                    }

                    return (FshSyntaxKind::DateTime, i);
                } else if (i - day_start) == 0 && i == len || !((bytes[i] as char).is_ascii_digit())
                {
                    // No day digits but we have YYYY-MM, which is also valid
                    return (FshSyntaxKind::DateTime, checkpoint + 1 + (i - month_start));
                }
            } else if i == len || !(bytes[i] as char).is_ascii_digit() {
                // No second dash, but we have YYYY-MM, which is valid
                return (FshSyntaxKind::DateTime, i);
            }
        }

        // Not a valid datetime, revert to integer
        i = checkpoint;
    }

    let kind = if has_dot {
        FshSyntaxKind::Decimal
    } else {
        FshSyntaxKind::Integer
    };

    (kind, i)
}

/// Helper to lex time component: HH:MM:SS with optional .ffffff and timezone
fn lex_time_component(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = start;

    // Parse HH
    let hour_start = i;
    while i < len && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if (i - hour_start) != 2 {
        return None;
    }

    if i >= len || bytes[i] != b':' {
        return None;
    }
    i += 1; // skip ':'

    // Parse MM
    let min_start = i;
    while i < len && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if (i - min_start) != 2 {
        return None;
    }

    if i >= len || bytes[i] != b':' {
        return None;
    }
    i += 1; // skip ':'

    // Parse SS
    let sec_start = i;
    while i < len && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if (i - sec_start) != 2 {
        return None;
    }

    // Optional fractional seconds
    if i < len && bytes[i] == b'.' {
        i += 1;
        while i < len && (bytes[i] as char).is_ascii_digit() {
            i += 1;
        }
    }

    // Optional timezone: Z, +HH:MM, or -HH:MM
    if i < len {
        match bytes[i] as char {
            'Z' => {
                i += 1;
            }
            '+' | '-' => {
                i += 1;
                // Parse timezone hours
                let tz_hour_start = i;
                while i < len && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                if (i - tz_hour_start) == 2 && i < len && bytes[i] == b':' {
                    i += 1;
                    // Parse timezone minutes
                    let tz_min_start = i;
                    while i < len && (bytes[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                    if (i - tz_min_start) != 2 {
                        // Invalid timezone format, but we got the time part
                        return Some(tz_hour_start - 1);
                    }
                }
            }
            _ => {}
        }
    }

    Some(i)
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
#[allow(dead_code)]
fn lex_regular_token(input: &str, start: usize) -> (FshSyntaxKind, usize) {
    // Fallback for anything else - treat as identifier
    let (_, end) = read_word(input, start);
    (FshSyntaxKind::Ident, end)
}

fn should_enter_param_context(tokens: &[CstToken]) -> bool {
    let mut seen_colon = false;
    let mut inspected = 0usize;

    for token in tokens.iter().rev() {
        let kind = token.kind;
        if kind.is_trivia() {
            continue;
        }

        inspected += 1;

        match kind {
            FshSyntaxKind::InsertKw => return true,
            FshSyntaxKind::RulesetKw => {
                if seen_colon {
                    return true;
                }
            }
            FshSyntaxKind::Colon => {
                seen_colon = true;
            }
            FshSyntaxKind::LParen | FshSyntaxKind::RParen | FshSyntaxKind::Newline => break,
            FshSyntaxKind::Asterisk => break,
            _ => {}
        }

        if inspected >= 8 {
            break;
        }
    }

    false
}

/// Read a word (sequence of alphanumeric/underscore chars)
fn read_word(input: &str, start: usize) -> (String, usize) {
    fn is_word_char(ch: char) -> bool {
        matches!(ch, '_' | '-' | '/' | '.')
            || ch.is_ascii_alphanumeric()
            || (!ch.is_ascii() && ch.is_alphanumeric())
    }

    let mut end = start;
    for (offset, ch) in input[start..].char_indices() {
        if is_word_char(ch) {
            end = start + offset + ch.len_utf8();
        } else {
            break;
        }
    }

    (input[start..end].to_string(), end)
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

    #[test]
    fn test_content_reference_keyword_case() {
        let (tokens, errors) = lex_with_trivia("contentReference");
        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 2); // keyword + EOF
        assert_eq!(tokens[0].kind, FshSyntaxKind::ContentreferenceKw);
    }

    #[test]
    fn test_code_literal_tokenization() {
        let (tokens, errors) = lex_with_trivia("#test-code");
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::Code);
        assert_eq!(tokens[0].text, "#test-code");
    }

    #[test]
    fn test_reference_literal_tokenization() {
        let input = "Reference(Patient or Observation)";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::Reference);
        assert_eq!(tokens[0].text, input);
    }

    #[test]
    fn test_regex_literal_tokenization() {
        let input = "/^[A-Z]{2,4}$/";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::Regex);
        assert_eq!(tokens[0].text, input);
    }

    #[test]
    fn test_ruleset_parameter_tokens() {
        let input = "RuleSet: MyRule([[First Param]], plain\\,value, second param)";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());

        let bracket = tokens
            .iter()
            .find(|t| t.kind == FshSyntaxKind::BracketedParamToken)
            .expect("bracketed parameter token");
        assert_eq!(bracket.text, "[[First Param]]");

        let plain_params: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind == FshSyntaxKind::PlainParamToken)
            .collect();
        assert_eq!(plain_params.len(), 2);
        assert_eq!(plain_params[0].text, "plain\\,value");
        assert_eq!(plain_params[1].text, "second param");
    }

    #[test]
    fn test_insert_parameter_tokens() {
        let input = "* insert Some.Rule([[test]], another)";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());

        let bracket = tokens
            .iter()
            .find(|t| t.kind == FshSyntaxKind::BracketedParamToken)
            .expect("bracketed parameter token");
        assert_eq!(bracket.text, "[[test]]");

        let plain = tokens
            .iter()
            .find(|t| t.kind == FshSyntaxKind::PlainParamToken)
            .expect("plain parameter token");
        assert_eq!(plain.text, "another");
    }

    #[test]
    fn test_canonical_literal_with_version() {
        let input = "Canonical(MyProfile|1.0.0 or OtherProfile|2.0)";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::Canonical);
        assert_eq!(tokens[0].text, input);
    }

    #[test]
    fn test_codeable_reference_literal_tokenization() {
        let input = "CodeableReference(Observation or Procedure)";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::CodeableReference);
        assert_eq!(tokens[0].text, input);
    }

    #[test]
    fn test_code_with_display_spaces() {
        let input = "#collection \"Display With Spaces\"";
        let (tokens, errors) = lex_with_trivia(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].kind, FshSyntaxKind::Code);
        assert_eq!(tokens[0].text, "#collection");
        assert_eq!(tokens[2].kind, FshSyntaxKind::String);
        assert_eq!(tokens[2].text, "\"Display With Spaces\"");
    }
}
