# Task 45: LSP Find References

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 1-2 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Task 40 (LSP Server Foundation), Task 13 (Semantic Analyzer)

## Overview

Implement the find-references feature for the LSP server, allowing users to find all usages of FSH entities, RuleSets, and aliases across the workspace. This includes cross-file reference search and efficient indexing.

## Goals

1. **Find all uses of Profiles/Extensions/ValueSets**
2. **Find all RuleSet insertions**
3. **Find all ValueSet bindings**
4. **Find all Alias uses**
5. **Cross-file search** - Search entire workspace efficiently
6. **Include declaration** - Optionally include definition location

## Technical Specification

```rust
use tower_lsp::lsp_types::{ReferenceParams, Location};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let doc = self.documents.get(&uri)?;
        let offset = position_to_offset(&doc.content, position);

        // Find symbol at cursor
        let symbol_name = find_symbol_at_offset(&doc.cst, offset)?;

        // Find all references in workspace
        let workspace = self.workspace.read().await;
        let mut locations = self.find_all_references(symbol_name, &workspace);

        // Include declaration if requested
        if include_declaration {
            if let Some(def_loc) = self.resolve_definition(symbol_name, &workspace) {
                locations.insert(0, def_loc);
            }
        }

        Ok(Some(locations))
    }
}

impl MakiLanguageServer {
    fn find_all_references(
        &self,
        symbol: &str,
        workspace: &Workspace,
    ) -> Vec<Location> {
        let mut locations = Vec::new();

        // Search all files in workspace
        for (path, doc) in &workspace.files {
            let references = find_references_in_file(&doc.cst, symbol);

            for range in references {
                locations.push(Location {
                    uri: Url::from_file_path(path).unwrap(),
                    range: text_range_to_lsp_range(&range),
                });
            }
        }

        locations
    }
}

fn find_references_in_file(root: &SyntaxNode, symbol: &str) -> Vec<TextRange> {
    let mut references = Vec::new();

    for node in root.descendants() {
        if is_reference_to_symbol(&node, symbol) {
            references.push(node.text_range());
        }
    }

    references
}
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/references.rs`

## Acceptance Criteria

- [ ] Find references to entities works
- [ ] Find references cross-file
- [ ] Find RuleSet insertions
- [ ] Find ValueSet bindings
- [ ] Include declaration option works
- [ ] Performance <200ms for 100 files
- [ ] Unit tests cover all reference types
- [ ] Integration tests verify VS Code display

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium
**Priority**: Medium
**Updated**: 2025-11-03
