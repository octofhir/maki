# Task 47: LSP Code Actions

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 2 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Task 40 (LSP Foundation), Task 37 (Autofix Engine), Task 41 (Diagnostics)

## Overview

Implement the code actions feature for the LSP server, providing quick fixes for diagnostics, refactoring actions, and source actions. This integrates with the autofix engine to apply fixes from lint rules and provides common refactorings.

## Goals

1. **Quick fixes from diagnostics** - Apply autofixes from lint rules
2. **Add missing metadata** - Id, Title, Description
3. **Organize rules** - Sort by type or alphabetically
4. **Extract RuleSet** - Create reusable rule groups
5. **Inline RuleSet** - Expand RuleSet insertions
6. **Format document** - Trigger formatter

## Technical Specification

```rust
use tower_lsp::lsp_types::{CodeActionParams, CodeAction, CodeActionKind, Command};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let doc = self.documents.get(&uri)?;
        let mut actions = Vec::new();

        // Quick fixes from diagnostics
        for diagnostic in &params.context.diagnostics {
            if let Some(fix_actions) = self.create_fix_actions(&diagnostic, &uri, &doc) {
                actions.extend(fix_actions);
            }
        }

        // Refactoring actions
        actions.extend(self.create_refactor_actions(&uri, range, &doc));

        // Source actions
        actions.extend(self.create_source_actions(&uri, &doc));

        Ok(Some(actions))
    }
}

impl MakiLanguageServer {
    fn create_fix_actions(
        &self,
        diagnostic: &Diagnostic,
        uri: &Url,
        doc: &Document,
    ) -> Option<Vec<CodeActionOrCommand>> {
        // Get autofixes from diagnostic
        let fixes = self.get_fixes_for_diagnostic(diagnostic)?;

        Some(fixes.into_iter().map(|fix| {
            CodeActionOrCommand::CodeAction(CodeAction {
                title: fix.description.clone(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(hashmap! {
                        uri.clone() => vec![TextEdit {
                            range: text_range_to_lsp_range(&fix.range),
                            new_text: fix.replacement,
                        }]
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }).collect())
    }

    fn create_refactor_actions(
        &self,
        uri: &Url,
        range: Range,
        doc: &Document,
    ) -> Vec<CodeActionOrCommand> {
        vec![
            // Extract RuleSet
            CodeAction {
                title: "Extract to RuleSet".to_string(),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                ..Default::default()
            },
            // Inline RuleSet
            CodeAction {
                title: "Inline RuleSet".to_string(),
                kind: Some(CodeActionKind::REFACTOR_INLINE),
                ..Default::default()
            },
            // Organize rules
            CodeAction {
                title: "Organize rules".to_string(),
                kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
                ..Default::default()
            },
        ].into_iter()
            .map(CodeActionOrCommand::CodeAction)
            .collect()
    }

    fn create_source_actions(
        &self,
        uri: &Url,
        doc: &Document,
    ) -> Vec<CodeActionOrCommand> {
        vec![
            CodeAction {
                title: "Format document".to_string(),
                kind: Some(CodeActionKind::SOURCE),
                command: Some(Command {
                    title: "Format".to_string(),
                    command: "editor.action.formatDocument".to_string(),
                    arguments: None,
                }),
                ..Default::default()
            },
        ].into_iter()
            .map(CodeActionOrCommand::CodeAction)
            .collect()
    }
}
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/code_actions.rs`

## Acceptance Criteria

- [ ] Quick fixes from diagnostics work
- [ ] Add missing metadata action works
- [ ] Organize rules action works
- [ ] Extract RuleSet works (basic)
- [ ] Format document action works
- [ ] Performance <100ms
- [ ] Unit tests cover all action types
- [ ] Integration tests verify VS Code actions

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: High
**Updated**: 2025-11-03
