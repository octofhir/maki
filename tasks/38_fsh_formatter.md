# Task 38: FSH Formatter Implementation

**Phase**: 3 (Formatter - Week 13)
**Time Estimate**: 4-5 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Tasks 03-04 (CST with trivia preservation), Task 37 (Autofix engine integration)

## Overview

Implement a lossless FSH code formatter that standardizes code style while preserving all comments, blank lines, and semantic content. Uses the Rowan-based CST to ensure perfect source reconstruction with formatting applied.

**Part of Formatter Phase**: Week 13 focuses on core formatting logic, Week 14 adds CLI integration (Task 39).

## Context

FSH projects often have inconsistent formatting:
- Mixed spacing around colons and equals
- Inconsistent indentation
- Misaligned rules
- Varying blank line usage

A formatter improves readability and prevents formatting debates in code reviews.

## Goals

1. **Implement lossless formatting** - Preserve all trivia (comments, blank lines)
2. **Support configurable styles** - Indent size, line width, alignment
3. **Handle special cases** - Multiline strings, complex rules, mappings
4. **Enable selective formatting** - Format entire file or specific ranges
5. **Integrate with CST** - Use Rowan's green/red tree for formatting

## Technical Specification

### Implementation Status

✅ **ALREADY IMPLEMENTED** in `crates/maki-core/src/cst/formatter.rs`

This task documents existing formatter which uses Rowan CST for lossless formatting:

```rust
// Existing implementation pattern
use maki_core::cst::formatter::{Formatter, FormattingOptions};

let formatter = Formatter::new(options);
let formatted = formatter.format(&document)?;

// Preserves:
// - All comments (trivia)
// - Blank lines
// - Semantic content
// - Source is 100% reconstructable
```

### Formatting Options

```rust
/// Indent style configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    Spaces(usize),  // Number of spaces
    Tabs,
}

impl IndentStyle {
    pub fn to_string(&self, level: usize) -> String {
        match self {
            IndentStyle::Spaces(size) => " ".repeat(size * level),
            IndentStyle::Tabs => "\t".repeat(level),
        }
    }
}

/// Comprehensive formatting options (from actual implementation)
#[derive(Debug, Clone)]
pub struct FormattingOptions {
    /// Indent style (spaces or tabs)
    pub indent_style: IndentStyle,

    /// Maximum line width before wrapping
    pub line_width: usize,

    /// Align rule elements (paths, cardinalities, etc.)
    pub align_rules: bool,

    /// Group rules by type (metadata, constraints, flags)
    pub group_rules: bool,

    /// Sort rules within groups
    pub sort_rules: bool,

    /// Blank lines between rule groups
    pub blank_lines_between_groups: usize,

    /// Normalize spacing around operators (: and =)
    pub normalize_spacing: bool,

    /// Preserve user's blank lines
    pub preserve_blank_lines: bool,

    /// Maximum consecutive blank lines to keep
    pub max_blank_lines: usize,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            indent_style: IndentStyle::Spaces(2),
            line_width: 120,
            align_rules: true,
            group_rules: false,
            sort_rules: false,
            blank_lines_between_groups: 1,
            normalize_spacing: true,
            preserve_blank_lines: true,
            max_blank_lines: 2,
        }
    }
}
```

### Formatter Implementation

```rust
use rowan::ast::AstNode;
use crate::cst::{SyntaxNode, SyntaxToken, SyntaxKind};

/// Main formatter
pub struct Formatter {
    options: FormattingOptions,
}

impl Formatter {
    pub fn new(options: FormattingOptions) -> Self {
        Self { options }
    }

    /// Format entire FSH file
    pub fn format_file(&self, source: &str) -> Result<String> {
        // Parse to CST
        let parse = crate::parse(source);
        let root = parse.syntax();

        // Format the tree
        let formatted = self.format_node(&root, 0)?;

        Ok(formatted)
    }

    /// Format specific text range
    pub fn format_range(&self, source: &str, range: TextRange) -> Result<String> {
        let parse = crate::parse(source);
        let root = parse.syntax();

        // Find nodes in range
        let nodes_in_range = find_nodes_in_range(&root, range);

        // Format only those nodes
        let mut result = source.to_string();
        for node in nodes_in_range.iter().rev() {
            let formatted_node = self.format_node(node, 0)?;
            let node_range = node.text_range();
            result.replace_range(
                node_range.start().into()..node_range.end().into(),
                &formatted_node
            );
        }

        Ok(result)
    }

    /// Format a syntax node recursively
    fn format_node(&self, node: &SyntaxNode, indent_level: usize) -> Result<String> {
        match node.kind() {
            SyntaxKind::PROFILE => self.format_profile(node, indent_level),
            SyntaxKind::EXTENSION => self.format_extension(node, indent_level),
            SyntaxKind::VALUE_SET => self.format_valueset(node, indent_level),
            SyntaxKind::CODE_SYSTEM => self.format_codesystem(node, indent_level),
            SyntaxKind::INSTANCE => self.format_instance(node, indent_level),
            SyntaxKind::RULE => self.format_rule(node, indent_level),
            _ => Ok(node.text().to_string()),
        }
    }

    /// Format a Profile definition
    fn format_profile(&self, node: &SyntaxNode, indent_level: usize) -> Result<String> {
        let profile = ast::Profile::cast(node.clone())?;
        let mut output = String::new();

        // Format profile header
        writeln!(&mut output, "Profile: {}", profile.name()?.value())?;

        // Format metadata (in order: Parent, Id, Title, Description)
        if let Some(parent) = profile.parent() {
            writeln!(&mut output, "Parent: {}", parent.value())?;
        }

        if let Some(id) = profile.id() {
            writeln!(&mut output, "Id: {}", id.value())?;
        }

        if let Some(title) = profile.title() {
            writeln!(&mut output, "Title: \"{}\"", title.value())?;
        }

        if let Some(desc) = profile.description() {
            writeln!(&mut output, "Description: \"{}\"", desc.value())?;
        }

        // Blank line before rules
        writeln!(&mut output)?;

        // Format rules
        let rules = profile.rules();
        if self.options.align_rules {
            self.format_rules_aligned(&rules, indent_level, &mut output)?;
        } else {
            self.format_rules_simple(&rules, indent_level, &mut output)?;
        }

        Ok(output)
    }

    /// Format rules with alignment
    fn format_rules_aligned(
        &self,
        rules: &[Rule],
        indent_level: usize,
        output: &mut String,
    ) -> Result<()> {
        // Calculate max path length for alignment
        let max_path_len = rules.iter()
            .filter_map(|r| r.path())
            .map(|p| p.len())
            .max()
            .unwrap_or(0);

        for rule in rules {
            let indent = self.options.indent_style.to_string(indent_level);

            if let Some(path) = rule.path() {
                // Align cardinality/values
                let padding = " ".repeat(max_path_len - path.len());
                write!(output, "{}* {}{} ", indent, path, padding)?;

                // Add cardinality if present
                if let Some(card) = rule.cardinality() {
                    write!(output, "{} ", card)?;
                }

                // Add flags
                if rule.has_flag("MS") {
                    write!(output, "MS ")?;
                }
                if rule.has_flag("SU") {
                    write!(output, "SU ")?;
                }

                writeln!(output)?;
            }
        }

        Ok(())
    }

    /// Format rules without alignment
    fn format_rules_simple(
        &self,
        rules: &[Rule],
        indent_level: usize,
        output: &mut String,
    ) -> Result<()> {
        for rule in rules {
            let indent = self.options.indent_style.to_string(indent_level);
            writeln!(output, "{}* {}", indent, rule.text())?;
        }

        Ok(())
    }
}
```

### Special Cases

#### Multiline Strings

```rust
/// Preserve formatting inside multiline strings
fn format_multiline_string(&self, s: &str) -> String {
    // Don't modify content between triple quotes
    if s.starts_with("\"\"\"") && s.ends_with("\"\"\"") {
        return s.to_string();
    }

    // Ensure closing quote isn't lost (SUSHI issue #1569)
    if s.ends_with("\"") && !s.ends_with("\\\"") {
        return s.to_string();
    }

    s.to_string()
}
```

#### Mapping Comments (SUSHI issue #1577, #1576)

```rust
/// Preserve multi-line delimiters in mappings and invariants
fn format_mapping(&self, mapping: &Mapping) -> String {
    let mut output = String::new();

    writeln!(&mut output, "Mapping: {}", mapping.name()).unwrap();
    writeln!(&mut output, "Source: {}", mapping.source()).unwrap();
    writeln!(&mut output, "Target: \"{}\"", mapping.target()).unwrap();

    // Preserve multi-line comments as-is
    if let Some(comment) = mapping.comment() {
        if comment.contains('\n') {
            writeln!(&mut output, "* -> \"{}\" \"\"\"{}\"\"\"",
                mapping.path(), comment).unwrap();
        } else {
            writeln!(&mut output, "* -> \"{}\" \"{}\"",
                mapping.path(), comment).unwrap();
        }
    }

    output
}
```

#### Normalize Spacing (SUSHI issue #693)

```rust
/// Normalize spacing around : and = while accepting missing whitespace
fn normalize_spacing(&self, text: &str) -> String {
    // Add space after : if missing
    let text = regex::Regex::new(r":(\S)").unwrap()
        .replace_all(text, ": $1");

    // Add space around = if missing
    let text = regex::Regex::new(r"(\S)=(\S)").unwrap()
        .replace_all(&text, "$1 = $2");

    text.to_string()
}
```

## Implementation Location

**Primary File**: `crates/maki-core/src/cst/formatter.rs` (expand existing)

**Additional Files**:
- `crates/maki-core/src/cst/format_rules.rs` - Rule formatting logic
- `crates/maki-core/src/cst/format_options.rs` - Configuration

## Testing Requirements

### Unit Tests

```rust
#[test]
fn test_format_profile_basic() {
    let source = r#"
Profile:MyProfile
Parent:Patient
Id:  my-profile
Title:"My Profile"
* name 1..1  MS
* gender 1..1 MS
    "#;

    let formatted = format_source(source);

    assert_eq!(formatted, r#"Profile: MyProfile
Parent: Patient
Id: my-profile
Title: "My Profile"

* name   1..1 MS
* gender 1..1 MS
"#);
}

#[test]
fn test_preserve_comments() {
    let source = r#"
// This is a profile comment
Profile: MyProfile
Parent: Patient
* name MS  // Important field
    "#;

    let formatted = format_source(source);

    assert!(formatted.contains("// This is a profile comment"));
    assert!(formatted.contains("// Important field"));
}

#[test]
fn test_multiline_string_preserved() {
    let source = r#"
Profile: MyProfile
Parent: Patient
Description: """
This is a multi-line
description that should
preserve line breaks.
"""
    "#;

    let formatted = format_source(source);

    assert!(formatted.contains("This is a multi-line"));
    assert!(formatted.contains("description that should"));
    assert!(formatted.contains("preserve line breaks."));
}

#[test]
fn test_alignment() {
    let source = r#"
Profile: MyProfile
Parent: Patient
* name 1..1 MS
* birthDate 1..1 MS
* gender 1..1 MS
    "#;

    let options = FormattingOptions {
        align_rules: true,
        ..Default::default()
    };

    let formatted = Formatter::new(options).format_file(source).unwrap();

    // Check alignment
    assert!(formatted.contains("* name      1..1 MS"));
    assert!(formatted.contains("* birthDate 1..1 MS"));
    assert!(formatted.contains("* gender    1..1 MS"));
}
```

### Integration Tests

```bash
# Format entire project
maki format input/fsh/

# Verify formatting
maki format --check input/fsh/

# Test on real FSH files
for f in examples/*.fsh; do
    maki format "$f"
    # Verify parse still works
    maki lint "$f"
done
```

## Configuration

```toml
[format]
# Indent style
indent_style = "spaces"  # Options: spaces, tabs
indent_size = 2

# Line width
line_width = 120

# Rule formatting
align_rules = true
group_rules = false
sort_rules = false

# Spacing
normalize_spacing = true
blank_lines_between_groups = 1

# Comment preservation
preserve_blank_lines = true
max_blank_lines = 2
```

## CLI Integration

Implemented in Task 39, but formatter provides:

```rust
pub trait FormatProvider {
    fn format_file(&self, path: &Path) -> Result<String>;
    fn format_source(&self, source: &str) -> Result<String>;
    fn format_range(&self, source: &str, range: TextRange) -> Result<String>;
}
```

## Dependencies

### Required Components
- **CST with Trivia** (Task 03-04): Preserves all source information
- **Rowan Library**: Green/red tree pattern
- **Configuration System**: Load formatting options

## Acceptance Criteria

### Core Functionality
- [ ] Formats profiles, extensions, valuesets, codesystems, instances
- [ ] Preserves all comments (line and block)
- [ ] Preserves intentional blank lines (up to max_blank_lines)
- [ ] Normalizes spacing around `:` and `=`
- [ ] Aligns rules when align_rules = true
- [ ] Handles multiline strings without modification
- [ ] Preserves mapping multi-line comments (SUSHI #1577, #1576)
- [ ] Ensures triple-quote endings survive (SUSHI #1569)
- [ ] Accepts missing whitespace in input (SUSHI #693)
- [ ] Range formatting works for selected text
- [ ] Configuration file support
- [ ] Lossless: parse(format(parse(source))) == parse(source)
- [ ] Performance: <50ms per file
- [ ] Unit tests cover all node types
- [ ] Integration tests verify on real FSH files

### Token Optimization (Required)
- [ ] `FormatElement::Token` variant implemented with `&'static str`
- [ ] `FormatElement::Text` variant with `Box<str>` and `TextSize` position
- [ ] Builder API (`token()`, `text()`) with debug assertions
- [ ] Printer has optimized fast path for Token processing (bulk operations)
- [ ] Printer has slow path for Text processing (Unicode-aware)
- [ ] All FSH keywords use `token()`: Profile, ValueSet, Extension, etc.
- [ ] All FSH operators use `token()`: *, .., =, ->, only, from, etc.
- [ ] All FSH modifiers use `token()`: MS, SU, TU, N, D, ?!
- [ ] All dynamic content from CST uses `text()` with position
- [ ] Benchmark shows improvement >1% on real FSH files
- [ ] Documentation includes Token vs Text usage guidelines

## Performance Considerations

- **Single-pass formatting**: Format during tree traversal
- **Lazy allocation**: Only allocate strings when needed
- **Parallel processing**: Format multiple files in parallel (Task 39)

### Token Optimization Pattern (Required)

Based on proven optimizations from [Ruff](https://github.com/astral-sh/ruff/pull/7048) and [Biome](https://github.com/biomejs/biome/pull/7968) formatters, we will implement a `Token` variant for static content to achieve 2-3% performance improvement.

**Core Idea**: Distinguish between static ASCII-only keywords/operators (fast path) vs dynamic Unicode content from source (slow path).

#### FormatElement Design

```rust
pub enum FormatElement {
    /// Static compile-time text: keywords, operators, punctuation
    /// - Must be ASCII only (no Unicode)
    /// - Cannot contain \n, \r, \t
    /// - Examples: "Profile", "Parent", ":", "=", "*", "MS"
    Token(&'static str),

    /// Dynamic text from source: identifiers, strings, comments
    /// - Can contain Unicode
    /// - Can contain line breaks
    /// - Tracks source position for CST integration
    /// - Examples: profile names, path expressions, string literals
    Text {
        text: Box<str>,
        source_position: TextSize,
    },

    // ... other variants (Indent, HardLineBreak, etc.)
}
```

#### Builder API

```rust
/// Create token for static, ASCII-only text
pub fn token(text: &'static str) -> FormatElement {
    debug_assert!(text.is_ascii(), "Token must be ASCII: {text:?}");
    debug_assert!(
        !text.contains(['\n', '\r', '\t']),
        "Token cannot contain newlines/tabs: {text:?}"
    );
    FormatElement::Token(text)
}

/// Create text element from dynamic source content
pub fn text(text: &str, position: TextSize) -> FormatElement {
    FormatElement::Text {
        text: text.into(),
        source_position: position,
    }
}
```

#### Printer Fast Path

```rust
impl Printer {
    fn print_element(&mut self, element: &FormatElement) {
        match element {
            FormatElement::Token(token) => {
                // FAST PATH: bulk string operations
                self.buffer.push_str(token);
                self.line_width += token.len() as u32;
            }
            FormatElement::Text { text, .. } => {
                // SLOW PATH: Unicode-aware character processing
                for c in text.chars() {
                    let width = match c {
                        '\t' => self.tab_width,
                        '\n' => {
                            self.line_width = 0;
                            self.buffer.push('\n');
                            continue;
                        }
                        c => c.width().unwrap_or(0) as u32,
                    };
                    self.buffer.push(c);
                    self.line_width += width;
                }
            }
            // ... other cases
        }
    }
}
```

#### Usage Example

```rust
impl Format for Profile {
    fn fmt(&self, f: &mut Formatter) -> FormatResult<()> {
        write!(f, [
            token("Profile"),    // Static keyword
            token(":"),          // Static punctuation
            space(),
            text(&self.name, self.name_position),  // Dynamic from CST
            hard_line_break(),
            token("Parent"),
            token(":"),
            space(),
            text(&self.parent, self.parent_position),
        ])
    }
}
```

#### FSH Token Categories

**Use `token()` for**:
- Keywords: `Profile`, `ValueSet`, `Extension`, `Instance`, `Parent`, `Id`, `Title`, `Description`
- Operators: `*`, `..`, `=`, `->`, `only`, `from`, `contains`, `named`, `insert`
- Modifiers: `MS`, `SU`, `TU`, `N`, `D`, `?!`
- Punctuation: `:`, `,`, `{`, `}`, `[`, `]`, `(`, `)`, `|`

**Use `text()` for**:
- Profile/resource names from source
- Path expressions
- String literals
- Comments
- User-defined identifiers

#### Implementation Requirements

**Must implement**:
- ✅ Token optimization is required for all formatters
- ✅ All FSH keywords and operators must use `token()`
- ✅ All dynamic content from CST must use `text()` with position
- ✅ Builder API with debug assertions for safety
- ✅ Optimized printer with fast/slow path separation

#### Expected Impact

- **Ruff/Biome**: 2-3% overall improvement
- **FSH estimate**: 2-5% (high keyword density)
- **Fast path usage**: ~70-85% of text operations (keywords/operators)
- **Slow path usage**: ~15-30% (identifiers/strings/comments)

**Note**: Token optimization integrates seamlessly with Rowan CST (proven by Biome). See `TOKEN_OPTIMIZATION_ANALYSIS.md` for detailed analysis.

## Future Enhancements

1. **Smart line breaking**: Wrap long lines intelligently
2. **Custom formatting rules**: User-defined formatting plugins
3. **Format on save**: IDE integration via LSP
4. **Diff-aware formatting**: Only format changed lines

## Resources

- **Rowan Library**: https://github.com/rust-analyzer/rowan
- **Rust Formatter (rustfmt)**: https://github.com/rust-lang/rustfmt
- **Prettier**: https://prettier.io/ (inspiration for formatting philosophy)

## Related Tasks

- **Task 39: Format Command** - CLI integration for formatter
- **Task 37: Autofix Engine** - Formatting as safe autofix
- **Task 42: LSP Formatting** - Format on save via LSP

---

**Status**: Ready for implementation (extends existing formatter.rs)
**Estimated Complexity**: High (requires careful trivia handling)
**Priority**: Medium (improves code quality and consistency)
**Updated**: 2025-11-03
