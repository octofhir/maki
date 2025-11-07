# Task 44: LSP Go to Definition

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 1-2 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Task 40 (LSP Server Foundation), Task 13 (Semantic Analyzer)

## Overview

Implement the go-to-definition feature for the LSP server, allowing users to navigate to the definition of FSH entities, Parent profiles, RuleSets, and FHIR elements. This includes cross-file navigation, symbol resolution, and support for external packages.

## Goals

1. **Jump to Profile/Extension/ValueSet definitions**
2. **Jump to Parent profile definitions**
3. **Jump to RuleSet definitions**
4. **Jump to Alias definitions**
5. **Cross-file navigation** - Resolve definitions across workspace
6. **External package navigation** - Navigate to definitions in dependencies

## Technical Specification

```rust
use tower_lsp::lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc = self.documents.get(&uri)?;
        let offset = position_to_offset(&doc.content, position);

        // Find symbol at cursor
        let symbol_name = find_symbol_at_offset(&doc.cst, offset)?;

        // Resolve definition in workspace
        let workspace = self.workspace.read().await;
        let location = self.resolve_definition(symbol_name, &workspace)?;

        Ok(Some(GotoDefinitionResponse::Scalar(location)))
    }
}

impl MakiLanguageServer {
    fn resolve_definition(
        &self,
        symbol: &str,
        workspace: &Workspace,
    ) -> Option<Location> {
        // Try to find in workspace symbols
        if let Some(entity) = workspace.symbol_table.get(symbol) {
            return Some(Location {
                uri: entity.uri.clone(),
                range: text_range_to_lsp_range(&entity.name_range()),
            });
        }

        // Try to find in FHIR definitions (external packages)
        if let Some(fhir_def) = workspace.fhir_defs.get_definition(symbol) {
            return Some(fhir_def_to_location(&fhir_def));
        }

        None
    }
}
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/goto_definition.rs`

## Acceptance Criteria

- [ ] Jump to entity definitions works
- [ ] Jump to Parent works (cross-file)
- [ ] Jump to RuleSet works
- [ ] Jump to Alias works
- [ ] External package definitions open correctly
- [ ] Performance <50ms
- [ ] Unit tests cover all symbol types
- [ ] Integration tests verify VS Code navigation

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: High
**Updated**: 2025-11-03
