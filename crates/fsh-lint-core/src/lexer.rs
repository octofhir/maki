/**
 * FSH Lexer
 *
 * Tokenizes FSH source code into a stream of tokens with byte spans.
 * The lexer is intentionally conservative about what constitutes keywords
 * vs identifiers so that the parser can perform additional validation.
 */
use chumsky::span::{SimpleSpan, Span as _};
use serde::{Deserialize, Serialize};

/// Span type returned by the lexer
pub type LexSpan = SimpleSpan<usize>;

/// Result returned by the lexer
pub type LexResult = (Vec<(Token, LexSpan)>, Vec<LexerError>);

/// Lexing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub message: String,
    pub span: LexSpan,
}

impl LexerError {
    pub fn new(message: impl Into<String>, span: LexSpan) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

/// FSH tokens
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Token {
    // Keywords - Entity types
    Alias,
    Profile,
    Extension,
    Instance,
    ValueSet,
    CodeSystem,
    Invariant,
    RuleSet,
    Mapping,
    Logical,
    Resource,

    // Keywords - Metadata
    Parent,
    Id,
    Title,
    Description,
    Expression,
    XPath,
    Severity,
    InstanceOf,
    Usage,
    Source,
    Target,
    Context,
    Characteristics,

    // Keywords - Rules
    From,
    Include,
    Exclude,
    Codes,
    Where,
    System,
    ValueSetRef,
    Contains,
    Named,
    And,
    Or,
    Only,
    Obeys,
    Insert,
    Exactly,

    // Keywords - Flags
    MS,  // Must Support
    SU,  // Summary
    TU,  // Trial Use
    N,   // Normative
    D,   // Draft
    Mod, // Modifier (?!)

    // Keywords - Binding strength
    Required,
    Extensible,
    Preferred,
    Example,

    // Keywords - Boolean
    True,
    False,

    // Literals
    Ident(String),           // Identifier/name
    String(String),          // String literal "..."
    MultilineString(String), // Multiline string """..."""
    Number(String),          // Number literal (kept as string for precision)
    Code(String, String),    // Code: optional_system#code (system, code)
    DateTime(String),        // DateTime literal
    Time(String),            // Time literal

    // Symbols/Operators
    Star,     // *
    Plus,     // +
    Colon,    // :
    Equal,    // =
    Caret,    // ^
    Arrow,    // ->
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    Comma,    // ,
    Pipe,     // |
    Dot,      // .
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Alias => write!(f, "Alias"),
            Token::Profile => write!(f, "Profile"),
            Token::Extension => write!(f, "Extension"),
            Token::Instance => write!(f, "Instance"),
            Token::ValueSet => write!(f, "ValueSet"),
            Token::CodeSystem => write!(f, "CodeSystem"),
            Token::Invariant => write!(f, "Invariant"),
            Token::RuleSet => write!(f, "RuleSet"),
            Token::Mapping => write!(f, "Mapping"),
            Token::Logical => write!(f, "Logical"),
            Token::Resource => write!(f, "Resource"),
            Token::Parent => write!(f, "Parent"),
            Token::Id => write!(f, "Id"),
            Token::Title => write!(f, "Title"),
            Token::Description => write!(f, "Description"),
            Token::Expression => write!(f, "Expression"),
            Token::XPath => write!(f, "XPath"),
            Token::Severity => write!(f, "Severity"),
            Token::InstanceOf => write!(f, "InstanceOf"),
            Token::Usage => write!(f, "Usage"),
            Token::Source => write!(f, "Source"),
            Token::Target => write!(f, "Target"),
            Token::Context => write!(f, "Context"),
            Token::Characteristics => write!(f, "Characteristics"),
            Token::From => write!(f, "from"),
            Token::Include => write!(f, "include"),
            Token::Exclude => write!(f, "exclude"),
            Token::Codes => write!(f, "codes"),
            Token::Where => write!(f, "where"),
            Token::System => write!(f, "system"),
            Token::ValueSetRef => write!(f, "valueset"),
            Token::Contains => write!(f, "contains"),
            Token::Named => write!(f, "named"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::Only => write!(f, "only"),
            Token::Obeys => write!(f, "obeys"),
            Token::Insert => write!(f, "insert"),
            Token::Exactly => write!(f, "exactly"),
            Token::MS => write!(f, "MS"),
            Token::SU => write!(f, "SU"),
            Token::TU => write!(f, "TU"),
            Token::N => write!(f, "N"),
            Token::D => write!(f, "D"),
            Token::Mod => write!(f, "?!"),
            Token::Required => write!(f, "required"),
            Token::Extensible => write!(f, "extensible"),
            Token::Preferred => write!(f, "preferred"),
            Token::Example => write!(f, "example"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::MultilineString(s) => write!(f, "\"\"\"{}\"\"\"", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::Code(sys, code) => {
                if sys.is_empty() {
                    write!(f, "#{}", code)
                } else {
                    write!(f, "{}#{}", sys, code)
                }
            }
            Token::DateTime(dt) => write!(f, "{}", dt),
            Token::Time(t) => write!(f, "{}", t),
            Token::Star => write!(f, "*"),
            Token::Plus => write!(f, "+"),
            Token::Colon => write!(f, ":"),
            Token::Equal => write!(f, "="),
            Token::Caret => write!(f, "^"),
            Token::Arrow => write!(f, "->"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::Pipe => write!(f, "|"),
            Token::Dot => write!(f, "."),
        }
    }
}

/// Lex the provided input into tokens and spans.
pub fn lex(input: &str) -> LexResult {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        let ch = match next_char(input, i) {
            Some((c, size)) => {
                if c.is_whitespace() {
                    i += size;
                    continue;
                }
                (c, size)
            }
            None => break,
        };

        let (current, size) = ch;
        let start = i;
        match current {
            '/' => {
                if let Some((next, next_size)) = next_char(input, i + size) {
                    if next == '/' {
                        // Line comment
                        i += size + next_size;
                        while i < len {
                            if let Some((c, step)) = next_char(input, i) {
                                if c == '\n' {
                                    i += step;
                                    break;
                                }
                                i += step;
                            } else {
                                break;
                            }
                        }
                        continue;
                    } else if next == '*' {
                        // Block comment
                        i += size + next_size;
                        let mut terminated = false;
                        while i < len {
                            if let Some((c, step)) = next_char(input, i) {
                                if c == '*' {
                                    if let Some((peek, peek_size)) = next_char(input, i + step) {
                                        if peek == '/' {
                                            i += step + peek_size;
                                            terminated = true;
                                            break;
                                        }
                                    }
                                }
                                i += step;
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
                        continue;
                    }
                }
                // Not a comment, treat '/' as part of identifier (e.g., URLs)
                let (word, end) = read_word(input, start);
                if !word.is_empty() {
                    push_ident_like(&mut tokens, start, end, word);
                    i = end;
                } else {
                    i += size;
                }
            }
            '"' => {
                if input[start..].starts_with("\"\"\"") {
                    match read_multiline_string(input, start) {
                        Ok((value, end)) => {
                            tokens.push((Token::MultilineString(value), span(start, end)));
                            i = end;
                        }
                        Err(err) => {
                            errors.push(err);
                            i = len;
                        }
                    }
                } else {
                    match read_string(input, start) {
                        Ok((value, end)) => {
                            tokens.push((Token::String(value), span(start, end)));
                            i = end;
                        }
                        Err(err) => {
                            errors.push(err);
                            i = len;
                        }
                    }
                }
            }
            '*' => {
                tokens.push((Token::Star, span(start, start + size)));
                i += size;
            }
            '+' => {
                tokens.push((Token::Plus, span(start, start + size)));
                i += size;
            }
            ':' => {
                tokens.push((Token::Colon, span(start, start + size)));
                i += size;
            }
            '=' => {
                tokens.push((Token::Equal, span(start, start + size)));
                i += size;
            }
            '^' => {
                tokens.push((Token::Caret, span(start, start + size)));
                i += size;
            }
            '-' => {
                if let Some((next, next_size)) = next_char(input, i + size) {
                    if next == '>' {
                        tokens.push((Token::Arrow, span(start, start + size + next_size)));
                        i += size + next_size;
                        continue;
                    }
                }
                let (word, end) = read_word(input, start);
                if !word.is_empty() {
                    push_ident_like(&mut tokens, start, end, word);
                    i = end;
                } else {
                    i += size;
                }
            }
            '(' => {
                tokens.push((Token::LParen, span(start, start + size)));
                i += size;
            }
            ')' => {
                tokens.push((Token::RParen, span(start, start + size)));
                i += size;
            }
            '[' => {
                tokens.push((Token::LBracket, span(start, start + size)));
                i += size;
            }
            ']' => {
                tokens.push((Token::RBracket, span(start, start + size)));
                i += size;
            }
            ',' => {
                tokens.push((Token::Comma, span(start, start + size)));
                i += size;
            }
            '|' => {
                tokens.push((Token::Pipe, span(start, start + size)));
                i += size;
            }
            '.' => {
                // Always emit as Dot token
                // FSH uses .. for ranges, and . for path notation
                // Decimal numbers must start with a digit (0.5, not .5)
                tokens.push((Token::Dot, span(start, start + size)));
                i += size;
            }
            '?' => {
                if let Some((next, next_size)) = next_char(input, i + size) {
                    if next == '!' {
                        tokens.push((Token::Mod, span(start, start + size + next_size)));
                        i += size + next_size;
                        continue;
                    }
                }
                let (word, end) = read_word(input, start);
                if !word.is_empty() {
                    push_ident_like(&mut tokens, start, end, word);
                    i = end;
                } else {
                    errors.push(LexerError::new("Unexpected '?'", span(start, start + size)));
                    i += size;
                }
            }
            '#' => {
                let (code, end) = read_code_after_hash(input, start + size);
                tokens.push((Token::Code(String::new(), code), span(start, end)));
                i = end;
            }
            _ => {
                if current.is_ascii_digit() {
                    let (literal, end) = read_number(input, start);
                    tokens.push((literal, span(start, end)));
                    i = end;
                } else {
                    let (word, end) = read_word(input, start);
                    if !word.is_empty() {
                        push_ident_like(&mut tokens, start, end, word);
                        i = end;
                    } else {
                        errors.push(LexerError::new(
                            format!("Unexpected character '{}'", current),
                            span(start, start + size),
                        ));
                        i += size;
                    }
                }
            }
        }
    }

    (tokens, errors)
}

fn next_char(input: &str, start: usize) -> Option<(char, usize)> {
    input[start..].chars().next().map(|c| (c, c.len_utf8()))
}

fn span(start: usize, end: usize) -> LexSpan {
    SimpleSpan::new((), start..end)
}

fn read_string(input: &str, start: usize) -> Result<(String, usize), LexerError> {
    let mut value = String::new();
    let mut i = start + 1; // Skip opening quote
    let len = input.len();
    let mut escaped = false;

    while i < len {
        let (ch, size) = next_char(input, i).unwrap();
        i += size;

        if escaped {
            let decoded = match ch {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => '\u{0008}',
                'f' => '\u{000C}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            };
            value.push(decoded);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => {
                escaped = true;
            }
            '"' => {
                return Ok((value, i));
            }
            '\n' | '\r' => {
                return Err(LexerError::new(
                    "Unexpected newline in string literal",
                    span(start, i),
                ));
            }
            other => value.push(other),
        }
    }

    Err(LexerError::new(
        "Unterminated string literal",
        span(start, len),
    ))
}

fn read_multiline_string(input: &str, start: usize) -> Result<(String, usize), LexerError> {
    let mut i = start + 3; // Skip opening """
    let len = input.len();
    let mut value = String::new();

    while i < len {
        if input[i..].starts_with("\"\"\"") {
            i += 3;
            return Ok((value, i));
        }
        let (ch, size) = next_char(input, i).unwrap();
        i += size;
        value.push(ch);
    }

    Err(LexerError::new(
        "Unterminated multiline string literal",
        span(start, len),
    ))
}

fn read_code_after_hash(input: &str, mut index: usize) -> (String, usize) {
    let len = input.len();
    let mut code = String::new();

    while index < len {
        let (ch, size) = match next_char(input, index) {
            Some(pair) => pair,
            None => break,
        };

        if ch.is_whitespace() || matches!(ch, ',' | '|' | ')' | '(' | '[' | ']' | ':') {
            break;
        }

        code.push(ch);
        index += size;
    }

    (code, index)
}

fn read_number(input: &str, start: usize) -> (Token, usize) {
    let len = input.len();
    let mut i = start;
    let mut literal = String::new();
    let mut has_dot = false;

    while i < len {
        let (ch, size) = next_char(input, i).unwrap();
        if ch.is_ascii_digit() {
            literal.push(ch);
            i += size;
        } else if ch == '.' && !has_dot {
            // Check if next char is also a dot (for range operator ..)
            let next_is_dot = next_char(input, i + size)
                .map(|(c, _)| c == '.')
                .unwrap_or(false);

            if next_is_dot {
                // Don't consume the dot, it's part of the .. operator
                break;
            }

            has_dot = true;
            literal.push(ch);
            i += size;
        } else {
            break;
        }
    }

    (Token::Number(literal), i)
}

fn read_word(input: &str, start: usize) -> (String, usize) {
    let len = input.len();
    let mut i = start;
    let mut word = String::new();

    while i < len {
        let (ch, size) = next_char(input, i).unwrap();
        if ch.is_whitespace()
            || matches!(
                ch,
                '(' | ')' | '[' | ']' | ',' | '|' | '*' | '^' | '=' | '"'
            )
        {
            break;
        }

        if ch == ':' {
            let next = next_char(input, i + size).map(|(c, _)| c);
            if allow_colon_in_identifier(&word, next) {
                word.push(ch);
                i += size;
                continue;
            } else {
                break;
            }
        }

        if ch == '#' {
            word.push(ch);
            i += size;
            continue;
        }

        if ch == '?' || ch == '!' {
            break;
        }

        word.push(ch);
        i += size;
    }

    (word, i)
}

fn allow_colon_in_identifier(prefix: &str, next: Option<char>) -> bool {
    if let Some('/') = next {
        return true;
    }

    let lower = prefix.to_ascii_lowercase();
    lower.starts_with("http")
        || lower.starts_with("urn")
        || prefix.contains('/')
        || prefix.contains('.')
        || prefix.contains('#')
}

fn push_ident_like(tokens: &mut Vec<(Token, LexSpan)>, start: usize, end: usize, word: String) {
    if word.is_empty() {
        return;
    }

    let token = match classify_keyword(&word) {
        Some(keyword) => keyword,
        None => classify_literal(&word),
    };

    tokens.push((token, span(start, end)));
}

fn classify_keyword(word: &str) -> Option<Token> {
    match word {
        "Alias" => Some(Token::Alias),
        "Profile" => Some(Token::Profile),
        "Extension" => Some(Token::Extension),
        "Instance" => Some(Token::Instance),
        "ValueSet" => Some(Token::ValueSet),
        "CodeSystem" => Some(Token::CodeSystem),
        "Invariant" => Some(Token::Invariant),
        "RuleSet" => Some(Token::RuleSet),
        "Mapping" => Some(Token::Mapping),
        "Logical" => Some(Token::Logical),
        "Resource" => Some(Token::Resource),
        "Parent" => Some(Token::Parent),
        "Id" => Some(Token::Id),
        "Title" => Some(Token::Title),
        "Description" => Some(Token::Description),
        "Expression" => Some(Token::Expression),
        "XPath" => Some(Token::XPath),
        "Severity" => Some(Token::Severity),
        "InstanceOf" => Some(Token::InstanceOf),
        "Usage" => Some(Token::Usage),
        "Source" => Some(Token::Source),
        "Target" => Some(Token::Target),
        "Context" => Some(Token::Context),
        "Characteristics" => Some(Token::Characteristics),
        "from" => Some(Token::From),
        "include" => Some(Token::Include),
        "exclude" => Some(Token::Exclude),
        "codes" => Some(Token::Codes),
        "where" => Some(Token::Where),
        "system" => Some(Token::System),
        "valueset" => Some(Token::ValueSetRef),
        "contains" => Some(Token::Contains),
        "named" => Some(Token::Named),
        "and" => Some(Token::And),
        "or" => Some(Token::Or),
        "only" => Some(Token::Only),
        "obeys" => Some(Token::Obeys),
        "insert" => Some(Token::Insert),
        "exactly" => Some(Token::Exactly),
        "MS" => Some(Token::MS),
        "SU" => Some(Token::SU),
        "TU" => Some(Token::TU),
        "N" => Some(Token::N),
        "D" => Some(Token::D),
        "required" => Some(Token::Required),
        "extensible" => Some(Token::Extensible),
        "preferred" => Some(Token::Preferred),
        "example" => Some(Token::Example),
        "true" => Some(Token::True),
        "false" => Some(Token::False),
        _ => None,
    }
}

fn classify_literal(word: &str) -> Token {
    if let Some((system, code)) = split_code(word) {
        return Token::Code(system, code);
    }

    if is_time_literal(word) {
        return Token::Time(word.to_string());
    }

    if is_datetime_literal(word) {
        return Token::DateTime(word.to_string());
    }

    if is_number_literal(word) {
        return Token::Number(word.to_string());
    }

    Token::Ident(word.to_string())
}

fn split_code(word: &str) -> Option<(String, String)> {
    if let Some(idx) = word.find('#') {
        let system = &word[..idx];
        let code = &word[idx + 1..];
        if !code.is_empty() {
            return Some((system.to_string(), code.to_string()));
        }
    }
    None
}

fn is_number_literal(word: &str) -> bool {
    let mut chars = word.chars().peekable();
    if matches!(chars.peek(), Some('-')) {
        chars.next();
    }

    let mut seen_digit = false;
    let mut seen_dot = false;

    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            seen_digit = true;
            continue;
        }
        if ch == '.' && !seen_dot {
            seen_dot = true;
            continue;
        }
        return false;
    }

    seen_digit
}

fn is_time_literal(word: &str) -> bool {
    let parts: Vec<&str> = word.split(':').collect();
    if parts.len() == 3 {
        return parts
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()));
    }
    false
}

fn is_datetime_literal(word: &str) -> bool {
    let parts: Vec<&str> = word.split('T').collect();
    if parts.len() != 2 {
        return false;
    }

    let date = parts[0];
    let time = parts[1];

    date.split('-')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
        && (time.ends_with('Z') || time.contains('+') || time.contains('-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_simple_valueset() {
        let input = r#"
ValueSet: TestVS
Id: test-vs
Title: "Test ValueSet"
* include codes from system http://loinc.org
* exclude http://loinc.org#94563-4
"#;

        let (tokens, errors) = lex(input);
        assert!(
            errors.is_empty(),
            "Lexer should not emit errors: {:?}",
            errors
        );
        assert!(!tokens.is_empty(), "Should produce tokens");
        assert_eq!(tokens[0].0, Token::ValueSet);
        assert_eq!(tokens[1].0, Token::Colon);
        assert_eq!(tokens[2].0, Token::Ident("TestVS".to_string()));
    }

    #[test]
    fn lex_handles_strings() {
        let input = "Title: \"Simple\"";
        let (tokens, errors) = lex(input);
        assert!(errors.is_empty());
        assert_eq!(tokens[0].0, Token::Title);
        assert_eq!(tokens[1].0, Token::Colon);
        match &tokens[2].0 {
            Token::String(value) => assert_eq!(value, "Simple"),
            other => panic!("Expected string token, got {:?}", other),
        }
    }

    #[test]
    fn lex_codes() {
        let input = "* exclude http://loinc.org#94563-4";
        let (tokens, errors) = lex(input);
        assert!(errors.is_empty());
        assert!(tokens.iter().any(|(tok, _)| matches!(tok, Token::Code(system, code) if !system.is_empty() && !code.is_empty())));
    }
}
