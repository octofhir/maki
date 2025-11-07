# Task 42: LSP Hover Provider

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 2 days
**Status**: 📝 Planned
**Priority**: Medium
**Dependencies**: Task 40 (LSP Server Foundation), Task 13 (Semantic Analyzer)

## Overview

Implement the hover provider for the LSP server, showing contextual documentation and type information when users hover over FSH entities, FHIR paths, and keywords. This includes entity definitions, FHIR element documentation, cardinality/type information, binding details, and markdown-formatted help text.

**Part of LSP Phase**: Week 17-18 focus on LSP features, with hover being a key IntelliSense feature.

## Context

Hover documentation helps users understand code without leaving the editor:

- **Entity hover**: Show definition, parent, description for Profiles/Extensions/ValueSets
- **FHIR path hover**: Show element documentation, cardinality, type
- **Binding hover**: Show ValueSet details, strength, description
- **Keyword hover**: Show FSH keyword documentation
- **Alias hover**: Show alias resolution

The hover provider uses the semantic model and FHIR definitions to provide rich, contextual information.

## Goals

1. **Implement hover for FSH entities** - Show definition, parent, metadata
2. **Implement hover for FHIR paths** - Show element documentation
3. **Implement hover for bindings** - Show ValueSet information
4. **Implement hover for keywords** - Show FSH syntax help
5. **Format with markdown** - Rich formatting for readability
6. **Performance optimization** - Fast hover response (<50ms)

## Technical Specification

### Hover Provider Implementation

```rust
use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams, MarkedString, MarkupContent, MarkupKind};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get document
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Convert position to offset
        let offset = position_to_offset(&doc.content, position);

        // Find node at cursor
        let node = find_node_at_offset(&doc.cst, offset)?;

        // Determine hover content based on node type
        let workspace = self.workspace.read().await;
        let hover_content = self.generate_hover(&node, &doc, &workspace)?;

        Ok(hover_content)
    }
}

impl MakiLanguageServer {
    /// Generate hover content for a node
    fn generate_hover(
        &self,
        node: &SyntaxNode,
        doc: &Document,
        workspace: &Workspace,
    ) -> Option<Hover> {
        match node.kind() {
            SyntaxKind::ENTITY_NAME => self.hover_entity(node, workspace),
            SyntaxKind::FHIR_PATH => self.hover_fhir_path(node, workspace),
            SyntaxKind::BINDING => self.hover_binding(node, workspace),
            SyntaxKind::KEYWORD => self.hover_keyword(node),
            SyntaxKind::ALIAS_REFERENCE => self.hover_alias(node, workspace),
            _ => None,
        }
    }

    /// Hover over FSH entity (Profile, Extension, ValueSet, etc.)
    fn hover_entity(&self, node: &SyntaxNode, workspace: &Workspace) -> Option<Hover> {
        let name = node.text().to_string();

        // Find entity in workspace
        let entity = workspace.symbol_table.get(&name)?;

        let mut content = String::new();

        // Entity header
        writeln!(&mut content, "### {} `{}`", entity.kind(), name).unwrap();
        writeln!(&mut content).unwrap();

        // Parent
        if let Some(parent) = entity.parent() {
            writeln!(&mut content, "**Parent:** `{}`", parent).unwrap();
        }

        // Id
        if let Some(id) = entity.id() {
            writeln!(&mut content, "**Id:** `{}`", id).unwrap();
        }

        // Title
        if let Some(title) = entity.title() {
            writeln!(&mut content, "**Title:** {}", title).unwrap();
        }

        // Description
        if let Some(desc) = entity.description() {
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "{}", desc).unwrap();
        }

        // Location
        writeln!(&mut content).unwrap();
        writeln!(&mut content, "---").unwrap();
        writeln!(&mut content, "*Defined in {}*", entity.file_path()).unwrap();

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(text_range_to_lsp_range(&node.text_range())),
        })
    }

    /// Hover over FHIR path
    fn hover_fhir_path(&self, node: &SyntaxNode, workspace: &Workspace) -> Option<Hover> {
        let path = node.text().to_string();

        // Get parent entity
        let entity = find_parent_entity(node)?;

        // Resolve path in FHIR definitions
        let base_type = entity.parent().unwrap_or("Resource");
        let element = workspace.fhir_defs.resolve_path(base_type, &path)?;

        let mut content = String::new();

        // Element header
        writeln!(&mut content, "### `{}`", path).unwrap();
        writeln!(&mut content).unwrap();

        // Type
        if !element.types.is_empty() {
            let types_str = element.types
                .iter()
                .map(|t| format!("`{}`", t))
                .collect::<Vec<_>>()
                .join(" | ");
            writeln!(&mut content, "**Type:** {}", types_str).unwrap();
        }

        // Cardinality
        writeln!(&mut content, "**Cardinality:** `{}..{}`",
            element.min, element.max).unwrap();

        // Description
        if let Some(desc) = &element.description {
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "{}", desc).unwrap();
        }

        // Binding
        if let Some(binding) = &element.binding {
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "**Binding:** `{}` ({})",
                binding.value_set, binding.strength).unwrap();
        }

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(text_range_to_lsp_range(&node.text_range())),
        })
    }

    /// Hover over binding
    fn hover_binding(&self, node: &SyntaxNode, workspace: &Workspace) -> Option<Hover> {
        let binding_rule = ast::BindingRule::cast(node.clone())?;

        let valueset_name = binding_rule.valueset()?.value();
        let strength = binding_rule.strength()?.value();

        // Find ValueSet
        let valueset = workspace.symbol_table.get(valueset_name)?;

        let mut content = String::new();

        // Header
        writeln!(&mut content, "### ValueSet `{}`", valueset_name).unwrap();
        writeln!(&mut content).unwrap();

        // Binding strength
        writeln!(&mut content, "**Strength:** `{}`", strength).unwrap();

        // Description
        if let Some(desc) = valueset.description() {
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "{}", desc).unwrap();
        }

        // URL
        if let Some(url) = valueset.url() {
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "**URL:** `{}`", url).unwrap();
        }

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(text_range_to_lsp_range(&node.text_range())),
        })
    }

    /// Hover over keyword
    fn hover_keyword(&self, node: &SyntaxNode) -> Option<Hover> {
        let keyword = node.text().to_string();

        let content = match keyword.as_str() {
            "Profile" => format!(
                "### Profile\n\n\
                Defines a constraint on a FHIR resource.\n\n\
                **Syntax:**\n```fsh\n\
                Profile: ProfileName\n\
                Parent: BaseResource\n\
                Id: profile-id\n\
                * element 1..1 MS\n\
                ```"
            ),
            "Extension" => format!(
                "### Extension\n\n\
                Defines a FHIR extension.\n\n\
                **Syntax:**\n```fsh\n\
                Extension: ExtensionName\n\
                Id: extension-id\n\
                * value[x] only string\n\
                * ^context[+].type = #element\n\
                * ^context[=].expression = \"Patient\"\n\
                ```"
            ),
            "ValueSet" => format!(
                "### ValueSet\n\n\
                Defines a set of codes for use in FHIR resources.\n\n\
                **Syntax:**\n```fsh\n\
                ValueSet: ValueSetName\n\
                Id: valueset-id\n\
                * include codes from system http://...\n\
                ```"
            ),
            "Parent" => "**Parent:** Specifies the base resource or profile to constrain".to_string(),
            "Id" => "**Id:** Unique identifier for this resource (kebab-case)".to_string(),
            "Title" => "**Title:** Human-readable title".to_string(),
            "Description" => "**Description:** Detailed description of the resource".to_string(),
            "MS" => "**Must Support (MS):** This element must be supported by implementations".to_string(),
            "SU" => "**Summary (SU):** This element is part of the summary view".to_string(),
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(text_range_to_lsp_range(&node.text_range())),
        })
    }

    /// Hover over alias reference
    fn hover_alias(&self, node: &SyntaxNode, workspace: &Workspace) -> Option<Hover> {
        let alias_name = node.text().to_string();

        // Find alias in workspace
        let alias = workspace.symbol_table.get_alias(&alias_name)?;

        let content = format!(
            "### Alias `{}`\n\n\
            **Resolves to:** `{}`",
            alias_name,
            alias.value()
        );

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: content,
            }),
            range: Some(text_range_to_lsp_range(&node.text_range())),
        })
    }
}

/// Find node at byte offset
fn find_node_at_offset(root: &SyntaxNode, offset: usize) -> Option<SyntaxNode> {
    let token = root.token_at_offset(TextSize::from(offset as u32))
        .right_biased()?;

    Some(token.parent()?)
}

/// Find parent entity for a node
fn find_parent_entity(node: &SyntaxNode) -> Option<Entity> {
    let mut current = Some(node.clone());

    while let Some(node) = current {
        if matches!(
            node.kind(),
            SyntaxKind::PROFILE | SyntaxKind::EXTENSION | SyntaxKind::VALUE_SET
        ) {
            return Some(Entity::from_node(&node));
        }

        current = node.parent();
    }

    None
}
```

### Example Hover Content

**Profile Hover:**

```markdown
### Profile `MyPatientProfile`

**Parent:** `Patient`
**Id:** `my-patient-profile`
**Title:** Custom Patient Profile

This profile adds constraints for patient demographics in our system.

---
*Defined in input/fsh/profiles/MyPatient.fsh*
```

**FHIR Path Hover:**

```markdown
### `name.given`

**Type:** `string`
**Cardinality:** `0..*`

Given names (not always 'first'). Includes middle names. This repeating element order: Given Names appear in the correct order for presenting the name.
```

**Binding Hover:**

```markdown
### ValueSet `AdministrativeGenderVS`

**Strength:** `required`

Codes representing administrative gender (male, female, other, unknown)

**URL:** `http://hl7.org/fhir/ValueSet/administrative-gender`
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/hover.rs` (new file)

**Supporting Files**:
- `crates/maki-lsp/src/server.rs` - Integration with MakiLanguageServer
- `crates/maki-lsp/src/markdown.rs` - Markdown formatting utilities
- `crates/maki-core/src/fhir_defs.rs` - FHIR element lookup

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_entity() {
        let source = r#"
            Profile: MyProfile
            Parent: Patient
            Description: "A test profile"
        "#;

        let hover = get_hover_at_position(source, /* position of "MyProfile" */);

        assert!(hover.is_some());
        let content = hover.unwrap().contents;
        assert!(content.to_string().contains("Parent: `Patient`"));
        assert!(content.to_string().contains("A test profile"));
    }

    #[test]
    fn test_hover_fhir_path() {
        let source = r#"
            Profile: MyProfile
            Parent: Patient
            * name.given 1..*
        "#;

        let hover = get_hover_at_position(source, /* position of "name.given" */);

        assert!(hover.is_some());
        let content = hover.unwrap().contents;
        assert!(content.to_string().contains("Type: `string`"));
        assert!(content.to_string().contains("Cardinality:"));
    }

    #[test]
    fn test_hover_keyword() {
        let source = "Profile: Test";

        let hover = get_hover_at_position(source, /* position of "Profile" */);

        assert!(hover.is_some());
        let content = hover.unwrap().contents;
        assert!(content.to_string().contains("Defines a constraint"));
    }

    #[test]
    fn test_hover_alias() {
        let source = r#"
            Alias: $SCT = http://snomed.info/sct

            ValueSet: TestVS
            * include codes from system $SCT
        "#;

        let hover = get_hover_at_position(source, /* position of "$SCT" in include */);

        assert!(hover.is_some());
        let content = hover.unwrap().contents;
        assert!(content.to_string().contains("http://snomed.info/sct"));
    }
}
```

### Integration Tests

```typescript
// VS Code extension test
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Hover Tests', () => {
  test('Hover over profile name shows documentation', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: MyProfile\nParent: Patient\nDescription: "Test"',
      language: 'fsh',
    });

    const editor = await vscode.window.showTextDocument(doc);

    // Hover over "MyProfile"
    const position = new vscode.Position(0, 10);
    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
      'vscode.executeHoverProvider',
      doc.uri,
      position
    );

    assert.ok(hovers);
    assert.ok(hovers.length > 0);

    const hoverContent = hovers[0].contents[0] as vscode.MarkdownString;
    assert.ok(hoverContent.value.includes('Parent'));
    assert.ok(hoverContent.value.includes('Patient'));
  });

  test('Hover over FHIR path shows type info', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: Test\nParent: Patient\n* name.given 1..*',
      language: 'fsh',
    });

    await vscode.window.showTextDocument(doc);

    // Hover over "name.given"
    const position = new vscode.Position(2, 4);
    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
      'vscode.executeHoverProvider',
      doc.uri,
      position
    );

    const hoverContent = hovers[0].contents[0] as vscode.MarkdownString;
    assert.ok(hoverContent.value.includes('Type'));
    assert.ok(hoverContent.value.includes('string'));
  });
});
```

## Performance Considerations

- **Caching**: Cache FHIR element lookups
- **Lazy loading**: Only load documentation when needed
- **Debouncing**: Avoid excessive hover requests
- **Limit content**: Cap hover text at reasonable length

**Performance Targets:**
- Hover response: <50ms
- FHIR path lookup: <10ms
- Memory overhead: <5MB for cached documentation

## Dependencies

### Required Components
- **LSP Server Foundation** (Task 40): For server infrastructure
- **Semantic Analyzer** (Task 13): For symbol resolution
- **FHIR Definitions** (Task 29): For element documentation

## Acceptance Criteria

- [ ] Hover over entity names shows definition
- [ ] Hover over FHIR paths shows element documentation
- [ ] Hover over bindings shows ValueSet information
- [ ] Hover over keywords shows FSH syntax help
- [ ] Hover over aliases shows resolution
- [ ] Content is formatted as markdown
- [ ] Range highlighting works correctly
- [ ] Performance targets are met
- [ ] Unit tests cover all hover types
- [ ] Integration tests verify hover in VS Code

## Edge Cases

1. **Unresolved references**: Show partial information
2. **Cross-file references**: Resolve from workspace
3. **FHIR paths with slicing**: Show slice documentation
4. **Choice types**: Show all possible types
5. **Deeply nested paths**: Handle long path resolution

## Future Enhancements

1. **Code examples**: Show usage examples in hover
2. **Links**: Add clickable links to FHIR spec
3. **Signature help**: Show parameter info for functions
4. **Related items**: Show related profiles/extensions
5. **Custom documentation**: Allow user-defined hover content

## Related Tasks

- **Task 40: LSP Server Foundation** - Provides server infrastructure
- **Task 13: Semantic Analyzer** - Provides symbol resolution
- **Task 29: FHIR Definitions** - Provides element documentation
- **Task 44: Go to Definition** - Similar navigation feature

---

**Status**: Ready for implementation
**Estimated Complexity**: Medium (documentation formatting, FHIR integration)
**Priority**: Medium (improves developer experience)
**Updated**: 2025-11-03
