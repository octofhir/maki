# Task 41: LSP Diagnostics Publisher

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 1-2 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Task 40 (LSP Server Foundation), Tasks 30-36 (Lint rules)

## Overview

Implement the diagnostics publishing feature for the LSP server, converting MAKI's internal diagnostic representation to LSP format and publishing them to the editor client. This includes running lint rules on file changes, handling related information, supporting severity levels, and providing code actions for quick fixes.

**Part of LSP Phase**: Week 17-18 focus on LSP features, with diagnostics being the most visible feature for users.

## Context

Diagnostics are the primary way users see errors and warnings in their code:

- **Real-time feedback**: Show errors as user types (with debouncing)
- **Rich information**: Severity, message, source location, related information
- **Quick fixes**: Attach code actions for automatic fixes
- **Multi-file validation**: Show diagnostics across dependent files

The diagnostics publisher bridges MAKI's diagnostic system with the LSP protocol, enabling real-time error reporting in all LSP-compatible editors.

## Goals

1. **Convert internal diagnostics to LSP format** - Map Diagnostic → lsp_types::Diagnostic
2. **Run lint rules on document changes** - Execute all enabled rules
3. **Publish diagnostics to client** - Send via textDocument/publishDiagnostics
4. **Support related information** - Show parent constraints and cross-references
5. **Handle severity levels** - Error, Warning, Info, Hint
6. **Attach code actions** - Include quick fixes from autof engine
7. **Debounce diagnostics** - Avoid excessive validation while typing

## Technical Specification

### Diagnostic Conversion

```rust
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic,
    DiagnosticSeverity,
    DiagnosticRelatedInformation,
    Location,
    NumberOrString,
};

/// Convert MAKI diagnostic to LSP diagnostic
pub fn to_lsp_diagnostic(
    diagnostic: &Diagnostic,
    uri: &Url,
) -> LspDiagnostic {
    LspDiagnostic {
        range: text_range_to_lsp_range(&diagnostic.range),
        severity: Some(severity_to_lsp(diagnostic.severity)),
        code: diagnostic.code.as_ref().map(|c| {
            NumberOrString::String(c.to_string())
        }),
        code_description: None,
        source: Some("maki".to_string()),
        message: diagnostic.message.clone(),
        related_information: diagnostic.related.as_ref().map(|related| {
            related.iter()
                .map(|r| related_info_to_lsp(r, uri))
                .collect()
        }),
        tags: diagnostic_tags(&diagnostic),
        data: None,
    }
}

/// Convert severity to LSP severity
fn severity_to_lsp(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Info => DiagnosticSeverity::INFORMATION,
        Severity::Hint => DiagnosticSeverity::HINT,
    }
}

/// Convert TextRange to LSP Range
fn text_range_to_lsp_range(range: &TextRange) -> Range {
    let start = offset_to_position(range.start());
    let end = offset_to_position(range.end());

    Range { start, end }
}

/// Convert byte offset to LSP Position
fn offset_to_position(offset: usize, content: &str) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (i, ch) in content.chars().enumerate() {
        if i >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position { line: line as u32, character: character as u32 }
}

/// Convert related information to LSP
fn related_info_to_lsp(
    related: &RelatedInformation,
    base_uri: &Url,
) -> DiagnosticRelatedInformation {
    let location = Location {
        uri: related.uri.clone().unwrap_or_else(|| base_uri.clone()),
        range: text_range_to_lsp_range(&related.range),
    };

    DiagnosticRelatedInformation {
        location,
        message: related.message.clone(),
    }
}

/// Generate diagnostic tags
fn diagnostic_tags(diagnostic: &Diagnostic) -> Option<Vec<DiagnosticTag>> {
    let mut tags = Vec::new();

    if diagnostic.is_deprecated() {
        tags.push(DiagnosticTag::DEPRECATED);
    }

    if diagnostic.is_unnecessary() {
        tags.push(DiagnosticTag::UNNECESSARY);
    }

    if tags.is_empty() {
        None
    } else {
        Some(tags)
    }
}
```

### Diagnostics Publisher Implementation

```rust
impl MakiLanguageServer {
    /// Publish diagnostics for a document
    pub async fn publish_diagnostics(&self, uri: Url) {
        // Get document
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            None => return,
        };

        let content = &doc.content;
        let mut diagnostics = Vec::new();

        // 1. Add parse errors
        for error in &doc.parse_errors {
            let lsp_diag = parse_error_to_lsp(error, content);
            diagnostics.push(lsp_diag);
        }

        // 2. Run lint rules
        let workspace = self.workspace.read().await;
        let lint_diagnostics = self.run_lint_rules(&doc, &workspace, content).await;
        diagnostics.extend(lint_diagnostics);

        // 3. Send to client
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, Some(doc.version))
            .await;
    }

    /// Run all lint rules on a document
    async fn run_lint_rules(
        &self,
        doc: &Document,
        workspace: &Workspace,
        content: &str,
    ) -> Vec<LspDiagnostic> {
        let mut diagnostics = Vec::new();

        // Get enabled rules
        let config = self.load_config().await;
        let rules = self.get_enabled_rules(&config);

        // Run each rule
        for rule in rules {
            let rule_diagnostics = rule.check(&doc.cst, workspace);

            for diag in rule_diagnostics {
                let lsp_diag = to_lsp_diagnostic(&diag, &doc.uri, content);
                diagnostics.push(lsp_diag);
            }
        }

        diagnostics
    }

    /// Load lint configuration
    async fn load_config(&self) -> LintConfig {
        // Try to load from workspace root
        if let Some(root) = &self.workspace.read().await.root {
            if let Ok(config) = LintConfig::load_from_dir(root) {
                return config;
            }
        }

        // Fall back to defaults
        LintConfig::default()
    }

    /// Get enabled rules from configuration
    fn get_enabled_rules(&self, config: &LintConfig) -> Vec<Box<dyn Rule>> {
        config.enabled_rules()
            .into_iter()
            .filter_map(|name| self.rule_registry.get(name))
            .collect()
    }
}

/// Convert parse error to LSP diagnostic
fn parse_error_to_lsp(error: &ParseError, content: &str) -> LspDiagnostic {
    LspDiagnostic {
        range: text_range_to_lsp_range(&error.range, content),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("parse-error".to_string())),
        source: Some("maki-parser".to_string()),
        message: error.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Enhanced version with content parameter
fn to_lsp_diagnostic(
    diagnostic: &Diagnostic,
    uri: &Url,
    content: &str,
) -> LspDiagnostic {
    LspDiagnostic {
        range: text_range_to_lsp_range(&diagnostic.range, content),
        severity: Some(severity_to_lsp(diagnostic.severity)),
        code: diagnostic.code.as_ref().map(|c| {
            NumberOrString::String(c.to_string())
        }),
        code_description: None,
        source: Some("maki".to_string()),
        message: diagnostic.message.clone(),
        related_information: diagnostic.related.as_ref().map(|related| {
            related.iter()
                .map(|r| related_info_to_lsp(r, uri, content))
                .collect()
        }),
        tags: diagnostic_tags(&diagnostic),
        data: None,
    }
}
```

### Debounced Diagnostics

```rust
use std::time::Duration;
use tokio::time::sleep;

/// Debounce diagnostics to avoid excessive validation
pub struct DiagnosticsDebouncer {
    pending: Arc<DashMap<Url, tokio::task::JoinHandle<()>>>,
    delay: Duration,
}

impl DiagnosticsDebouncer {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
            delay: Duration::from_millis(delay_ms),
        }
    }

    /// Schedule diagnostics with debouncing
    pub fn schedule<F>(&self, uri: Url, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Cancel previous task for this URI
        if let Some((_, handle)) = self.pending.remove(&uri) {
            handle.abort();
        }

        // Schedule new task
        let delay = self.delay;
        let pending = self.pending.clone();
        let uri_clone = uri.clone();

        let handle = tokio::spawn(async move {
            sleep(delay).await;
            callback();
            pending.remove(&uri_clone);
        });

        self.pending.insert(uri, handle);
    }
}

// Usage in MakiLanguageServer:
impl MakiLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            workspace: Arc::new(RwLock::new(Workspace::new())),
            debouncer: DiagnosticsDebouncer::new(500), // 500ms delay
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        // ... apply changes ...

        // Schedule debounced diagnostics
        let server = self.clone();
        let uri_clone = uri.clone();
        self.debouncer.schedule(uri, move || {
            tokio::spawn(async move {
                server.publish_diagnostics(uri_clone).await;
            });
        });
    }
}
```

### Related Information Example

```rust
/// Create diagnostic with related information
pub fn create_duplicate_diagnostic(
    duplicate: &Entity,
    original: &Entity,
) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        range: duplicate.name_range(),
        message: format!(
            "Duplicate entity name '{}' found",
            duplicate.name()
        ),
        code: Some("duplicate-entity-name".to_string()),
        related: Some(vec![
            RelatedInformation {
                uri: Some(original.uri.clone()),
                range: original.name_range(),
                message: format!(
                    "Original definition of '{}' is here",
                    original.name()
                ),
            }
        ]),
        ..Default::default()
    }
}

// Converted to LSP:
// Main diagnostic:
//   error: Duplicate entity name 'MyProfile' found
//     --> file:///project/profiles/MyProfile2.fsh:3:10
//
// Related information:
//   note: Original definition of 'MyProfile' is here
//     --> file:///project/profiles/MyProfile.fsh:1:10
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/diagnostics.rs` (new file)

**Supporting Files**:
- `crates/maki-lsp/src/server.rs` - Integration with MakiLanguageServer
- `crates/maki-lsp/src/convert.rs` - Conversion utilities
- `crates/maki-lsp/src/debounce.rs` - Debouncing logic

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        assert_eq!(
            severity_to_lsp(Severity::Error),
            DiagnosticSeverity::ERROR
        );
        assert_eq!(
            severity_to_lsp(Severity::Warning),
            DiagnosticSeverity::WARNING
        );
    }

    #[test]
    fn test_range_conversion() {
        let content = "Line 1\nLine 2\nLine 3";
        let range = TextRange::new(7.into(), 13.into()); // "Line 2"

        let lsp_range = text_range_to_lsp_range(&range, content);

        assert_eq!(lsp_range.start.line, 1);
        assert_eq!(lsp_range.start.character, 0);
        assert_eq!(lsp_range.end.line, 1);
        assert_eq!(lsp_range.end.character, 6);
    }

    #[test]
    fn test_diagnostic_with_related_info() {
        let diag = Diagnostic {
            severity: Severity::Error,
            range: TextRange::new(0.into(), 10.into()),
            message: "Duplicate name".to_string(),
            related: Some(vec![
                RelatedInformation {
                    uri: Some(Url::parse("file:///other.fsh").unwrap()),
                    range: TextRange::new(5.into(), 15.into()),
                    message: "Original here".to_string(),
                }
            ]),
            ..Default::default()
        };

        let uri = Url::parse("file:///test.fsh").unwrap();
        let content = "Profile: MyProfile";

        let lsp_diag = to_lsp_diagnostic(&diag, &uri, content);

        assert!(lsp_diag.related_information.is_some());
        assert_eq!(lsp_diag.related_information.unwrap().len(), 1);
    }

    #[test]
    fn test_diagnostic_tags() {
        let diag = Diagnostic {
            deprecated: true,
            ..Default::default()
        };

        let tags = diagnostic_tags(&diag).unwrap();

        assert!(tags.contains(&DiagnosticTag::DEPRECATED));
    }
}
```

### Integration Tests

```typescript
// VS Code extension test
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Diagnostics Tests', () => {
  test('Parse error shows as diagnostic', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile MyProfile\n',  // Missing colon
      language: 'fsh',
    });

    await vscode.window.showTextDocument(doc);

    // Wait for diagnostics
    await new Promise(resolve => setTimeout(resolve, 1000));

    const diagnostics = vscode.languages.getDiagnostics(doc.uri);

    assert.strictEqual(diagnostics.length, 1);
    assert.strictEqual(diagnostics[0].severity, vscode.DiagnosticSeverity.Error);
    assert.ok(diagnostics[0].message.includes('Expected :'));
  });

  test('Lint rule shows as diagnostic', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: MyProfile\n',  // Missing Parent
      language: 'fsh',
    });

    await vscode.window.showTextDocument(doc);
    await new Promise(resolve => setTimeout(resolve, 1000));

    const diagnostics = vscode.languages.getDiagnostics(doc.uri);

    const parentDiag = diagnostics.find(d => d.message.includes('Parent'));
    assert.ok(parentDiag);
  });

  test('Diagnostics clear on fix', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: MyProfile\n',
      language: 'fsh',
    });

    await vscode.window.showTextDocument(doc);
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Initially has diagnostics
    let diagnostics = vscode.languages.getDiagnostics(doc.uri);
    assert.ok(diagnostics.length > 0);

    // Fix the issue
    const edit = new vscode.WorkspaceEdit();
    edit.insert(doc.uri, doc.positionAt(doc.getText().length), 'Parent: Patient\n');
    await vscode.workspace.applyEdit(edit);

    await new Promise(resolve => setTimeout(resolve, 1000));

    // Diagnostics should be cleared
    diagnostics = vscode.languages.getDiagnostics(doc.uri);
    const parentDiag = diagnostics.find(d => d.message.includes('Parent'));
    assert.ok(!parentDiag);
  });
});
```

## Performance Considerations

- **Debouncing**: Wait 500ms after last keystroke before running diagnostics
- **Incremental validation**: Only re-validate changed files and dependents
- **Parallel rule execution**: Run independent rules in parallel
- **Cache semantic models**: Reuse semantic analysis when possible
- **Limit diagnostics**: Cap at 100 diagnostics per file to avoid UI lag

**Performance Targets:**
- Publish diagnostics: <100ms for 1000-line file
- Debounce delay: 500ms (configurable)
- Memory overhead: <10MB for diagnostic storage

## Dependencies

### Required Components
- **LSP Server Foundation** (Task 40): For server infrastructure
- **Lint Rules** (Tasks 30-36): For generating diagnostics
- **Diagnostic System** (Task 06): For internal diagnostic representation

## Acceptance Criteria

- [ ] Parse errors are converted to LSP diagnostics
- [ ] Lint rules run on file open
- [ ] Lint rules run on file change (debounced)
- [ ] Diagnostics are published to client
- [ ] Severity levels map correctly (Error, Warning, Info, Hint)
- [ ] Related information shows cross-references
- [ ] Diagnostic codes are included
- [ ] Source is set to "maki"
- [ ] Debouncing prevents excessive validation
- [ ] Multi-file diagnostics work (dependent files)
- [ ] Diagnostics clear when file is closed
- [ ] Configuration affects which rules run
- [ ] Unit tests cover all conversion functions
- [ ] Integration tests verify diagnostics in VS Code
- [ ] Performance targets are met

## Edge Cases

1. **Empty files**: Should not produce diagnostics
2. **Very large files**: Limit diagnostics to avoid UI lag
3. **Rapid typing**: Debouncing prevents thrashing
4. **Invalid ranges**: Handle gracefully, skip diagnostic
5. **Missing related files**: Show diagnostic without related info
6. **Concurrent changes**: Handle multiple files changing at once

## Future Enhancements

1. **Diagnostic categories**: Group related diagnostics
2. **Diagnostic hints**: Show "Did you mean?" suggestions
3. **Custom severities**: Allow rule-specific severity overrides
4. **Diagnostic persistence**: Cache diagnostics across sessions
5. **Pull diagnostics**: Support LSP 3.17 pull model

## Related Tasks

- **Task 40: LSP Server Foundation** - Provides server infrastructure
- **Task 47: Code Actions** - Uses diagnostics to provide quick fixes
- **Tasks 30-36: Lint Rules** - Generate diagnostics

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium (conversion logic, debouncing)
**Priority**: High (most visible LSP feature)
**Updated**: 2025-11-03
