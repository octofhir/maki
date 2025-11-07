# Task 46: LSP Rename Symbol

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 2 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Task 40 (LSP Foundation), Task 45 (Find References)

## Overview

Implement the rename symbol feature for the LSP server, allowing users to rename FSH entities and automatically update all references across the workspace. This includes validation, conflict detection, and transactional workspace edits.

## Goals

1. **Rename Profiles/Extensions/ValueSets** - Update all references
2. **Rename RuleSets** - Update all insertions
3. **Rename Aliases** - Update all uses
4. **Validate new name** - Check for conflicts
5. **Preview changes** - Show all affected locations
6. **Transactional updates** - All-or-nothing application

## Technical Specification

```rust
use tower_lsp::lsp_types::{RenameParams, WorkspaceEdit, TextEdit};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let doc = self.documents.get(&uri)?;
        let offset = position_to_offset(&doc.content, position);

        // Find symbol at cursor
        let old_name = find_symbol_at_offset(&doc.cst, offset)?;

        // Validate new name
        let workspace = self.workspace.read().await;
        self.validate_rename(old_name, &new_name, &workspace)?;

        // Find all references
        let locations = self.find_all_references(old_name, &workspace);

        // Build workspace edit
        let mut changes = HashMap::new();
        for loc in locations {
            let edits = changes.entry(loc.uri).or_insert_with(Vec::new);
            edits.push(TextEdit {
                range: loc.range,
                new_text: new_name.clone(),
            });
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }
}

impl MakiLanguageServer {
    fn validate_rename(
        &self,
        old_name: &str,
        new_name: &str,
        workspace: &Workspace,
    ) -> Result<()> {
        // Check if new name already exists
        if workspace.symbol_table.contains(new_name) {
            return Err(anyhow!("Symbol '{}' already exists", new_name));
        }

        // Validate naming conventions
        if !is_valid_name(new_name) {
            return Err(anyhow!("Invalid name: {}", new_name));
        }

        Ok(())
    }
}
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/rename.rs`

## Acceptance Criteria

- [ ] Rename entities updates all references
- [ ] Validation prevents conflicts
- [ ] Preview shows all changes
- [ ] Transactional updates work
- [ ] Performance <300ms for 100 files
- [ ] Unit tests cover validation
- [ ] Integration tests verify VS Code rename

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: Medium
**Updated**: 2025-11-03
