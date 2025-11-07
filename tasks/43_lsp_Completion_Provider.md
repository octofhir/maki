# Task 43: LSP Completion Provider

**Phase**: 4 (Language Server - Weeks 17-18)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Task 40 (LSP Server Foundation), Task 13 (Semantic Analyzer)

## Overview

Implement the completion provider for the LSP server, offering IntelliSense suggestions for FSH entities, keywords, FHIR paths, types, and snippets. This includes context-aware completions, trigger characters, fuzzy matching, and snippet templates for common patterns.

**Part of LSP Phase**: Week 17-18 focus on LSP features, with completion being critical for developer productivity.

## Context

Code completion dramatically improves FSH authoring speed:

- **Entity completions**: Suggest Profile, Extension, ValueSet names
- **Keyword completions**: Suggest FSH keywords (Parent, Id, Title, etc.)
- **FHIR path completions**: Suggest valid paths based on parent type
- **Type completions**: Suggest FHIR types (string, CodeableConcept, etc.)
- **Binding completions**: Suggest ValueSet/CodeSystem names
- **Snippet completions**: Insert templates for common patterns

The completion provider uses the semantic model, FHIR definitions, and workspace symbols to provide accurate, context-aware suggestions.

## Goals

1. **Entity name completions** - Profile, Extension, ValueSet, Instance names
2. **Keyword completions** - FSH keywords with documentation
3. **FHIR path completions** - Valid paths based on context
4. **FHIR type completions** - Data types for constraints
5. **Binding completions** - ValueSet/CodeSystem for bindings
6. **RuleSet completions** - RuleSet names for inserts
7. **Snippet completions** - Templates for common patterns
8. **Performance optimization** - Fast completion (<100ms)

## Technical Specification

### Completion Provider Implementation

```rust
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    CompletionTextEdit, InsertTextFormat, TextEdit,
};

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Get document
        let doc = match self.documents.get(&uri) {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Convert position to offset
        let offset = position_to_offset(&doc.content, position);

        // Find context at cursor
        let context = find_completion_context(&doc.cst, offset)?;

        // Generate completions based on context
        let workspace = self.workspace.read().await;
        let completions = self.generate_completions(&context, &doc, &workspace)?;

        Ok(Some(CompletionResponse::Array(completions)))
    }
}

/// Completion context determines what to suggest
#[derive(Debug, Clone)]
pub enum CompletionContext {
    /// After "Profile:", "Extension:", etc.
    EntityName { kind: EntityKind },

    /// After keyword like "Parent:", "Id:"
    KeywordValue { keyword: String },

    /// Inside rule path (e.g., "* name.|")
    FhirPath { prefix: String, parent_type: String },

    /// After "only" keyword
    TypeConstraint,

    /// After "from" keyword
    Binding,

    /// After "insert" keyword
    RuleSetInsert,

    /// Top-level, suggest keywords
    TopLevel,

    /// Inside entity, suggest keywords
    EntityBody { entity_kind: EntityKind },
}

impl MakiLanguageServer {
    /// Generate completions based on context
    fn generate_completions(
        &self,
        context: &CompletionContext,
        doc: &Document,
        workspace: &Workspace,
    ) -> Vec<CompletionItem> {
        match context {
            CompletionContext::EntityName { kind } => {
                self.complete_entity_name(*kind, workspace)
            }
            CompletionContext::KeywordValue { keyword } => {
                self.complete_keyword_value(keyword, workspace)
            }
            CompletionContext::FhirPath { prefix, parent_type } => {
                self.complete_fhir_path(prefix, parent_type, workspace)
            }
            CompletionContext::TypeConstraint => {
                self.complete_type(workspace)
            }
            CompletionContext::Binding => {
                self.complete_binding(workspace)
            }
            CompletionContext::RuleSetInsert => {
                self.complete_ruleset(workspace)
            }
            CompletionContext::TopLevel => {
                self.complete_top_level()
            }
            CompletionContext::EntityBody { entity_kind } => {
                self.complete_entity_body(*entity_kind)
            }
        }
    }

    /// Complete entity names (Profile, Extension, ValueSet)
    fn complete_entity_name(
        &self,
        kind: EntityKind,
        workspace: &Workspace,
    ) -> Vec<CompletionItem> {
        workspace.symbol_table
            .entities_of_kind(kind)
            .into_iter()
            .map(|entity| CompletionItem {
                label: entity.name().to_string(),
                kind: Some(completion_item_kind(kind)),
                detail: Some(format!("{} ({})", entity.title().unwrap_or(""), entity.file_path())),
                documentation: entity.description().map(|desc| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: desc.to_string(),
                    })
                }),
                ..Default::default()
            })
            .collect()
    }

    /// Complete keyword values (e.g., Parent: <here>)
    fn complete_keyword_value(
        &self,
        keyword: &str,
        workspace: &Workspace,
    ) -> Vec<CompletionItem> {
        match keyword {
            "Parent" => {
                // Suggest FHIR resources and profiles
                let mut items = Vec::new();

                // FHIR base resources
                for resource in workspace.fhir_defs.resources() {
                    items.push(CompletionItem {
                        label: resource.name.clone(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some("FHIR Resource".to_string()),
                        documentation: resource.description.as_ref().map(|desc| {
                            Documentation::String(desc.clone())
                        }),
                        ..Default::default()
                    });
                }

                // Profiles from workspace
                for profile in workspace.symbol_table.profiles() {
                    items.push(CompletionItem {
                        label: profile.name().to_string(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some(format!("Profile ({})", profile.file_path())),
                        ..Default::default()
                    });
                }

                items
            }
            "Id" => {
                // No completions for Id (user-defined)
                vec![]
            }
            _ => vec![],
        }
    }

    /// Complete FHIR paths
    fn complete_fhir_path(
        &self,
        prefix: &str,
        parent_type: &str,
        workspace: &Workspace,
    ) -> Vec<CompletionItem> {
        let elements = workspace.fhir_defs.elements_for_type(parent_type);

        elements
            .into_iter()
            .filter(|elem| elem.path.starts_with(prefix))
            .map(|elem| CompletionItem {
                label: elem.path.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!(
                    "{} ({}..{})",
                    elem.types.join(" | "),
                    elem.min,
                    elem.max
                )),
                documentation: elem.description.as_ref().map(|desc| {
                    Documentation::String(desc.clone())
                }),
                insert_text: Some(elem.path.clone()),
                ..Default::default()
            })
            .collect()
    }

    /// Complete FHIR types
    fn complete_type(&self, workspace: &Workspace) -> Vec<CompletionItem> {
        workspace.fhir_defs.types()
            .into_iter()
            .map(|ty| CompletionItem {
                label: ty.name.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("FHIR Type".to_string()),
                documentation: ty.description.as_ref().map(|desc| {
                    Documentation::String(desc.clone())
                }),
                ..Default::default()
            })
            .collect()
    }

    /// Complete bindings (ValueSets/CodeSystems)
    fn complete_binding(&self, workspace: &Workspace) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // ValueSets from workspace
        for vs in workspace.symbol_table.valuesets() {
            items.push(CompletionItem {
                label: vs.name().to_string(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some(format!("ValueSet ({})", vs.file_path())),
                documentation: vs.description().map(|desc| {
                    Documentation::String(desc.to_string())
                }),
                ..Default::default()
            });
        }

        // CodeSystems from workspace
        for cs in workspace.symbol_table.codesystems() {
            items.push(CompletionItem {
                label: cs.name().to_string(),
                kind: Some(CompletionItemKind::ENUM),
                detail: Some(format!("CodeSystem ({})", cs.file_path())),
                ..Default::default()
            });
        }

        items
    }

    /// Complete RuleSet names
    fn complete_ruleset(&self, workspace: &Workspace) -> Vec<CompletionItem> {
        workspace.symbol_table.rulesets()
            .into_iter()
            .map(|rs| CompletionItem {
                label: rs.name().to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some(format!("RuleSet ({})", rs.file_path())),
                ..Default::default()
            })
            .collect()
    }

    /// Complete top-level keywords
    fn complete_top_level(&self) -> Vec<CompletionItem> {
        vec![
            snippet_completion(
                "Profile",
                "Profile: ${1:ProfileName}\nParent: ${2:Patient}\nId: ${3:profile-id}\nTitle: \"${4:Profile Title}\"\nDescription: \"${5:Description}\"\n$0",
                "Create a new Profile"
            ),
            snippet_completion(
                "Extension",
                "Extension: ${1:ExtensionName}\nId: ${2:extension-id}\nTitle: \"${3:Extension Title}\"\nDescription: \"${4:Description}\"\n* value[x] only ${5:string}\n* ^context[+].type = #element\n* ^context[=].expression = \"${6:Patient}\"\n$0",
                "Create a new Extension"
            ),
            snippet_completion(
                "ValueSet",
                "ValueSet: ${1:ValueSetName}\nId: ${2:valueset-id}\nTitle: \"${3:ValueSet Title}\"\nDescription: \"${4:Description}\"\n* include codes from system ${5:http://example.org}\n$0",
                "Create a new ValueSet"
            ),
            snippet_completion(
                "CodeSystem",
                "CodeSystem: ${1:CodeSystemName}\nId: ${2:codesystem-id}\nTitle: \"${3:CodeSystem Title}\"\nDescription: \"${4:Description}\"\n* #${5:code1} \"${6:Display 1}\"\n* #${7:code2} \"${8:Display 2}\"\n$0",
                "Create a new CodeSystem"
            ),
            snippet_completion(
                "Instance",
                "Instance: ${1:InstanceName}\nInstanceOf: ${2:Profile}\nUsage: #${3:example}\nTitle: \"${4:Instance Title}\"\nDescription: \"${5:Description}\"\n$0",
                "Create a new Instance"
            ),
            snippet_completion(
                "Alias",
                "Alias: ${1:\\$ALIAS} = ${2:http://example.org}\n$0",
                "Create a new Alias"
            ),
            snippet_completion(
                "RuleSet",
                "RuleSet: ${1:RuleSetName}\n* ${2:element} ${3:1..1}\n$0",
                "Create a new RuleSet"
            ),
        ]
    }

    /// Complete entity body keywords
    fn complete_entity_body(&self, entity_kind: EntityKind) -> Vec<CompletionItem> {
        let common = vec![
            keyword_completion("Id", "Unique identifier (kebab-case)"),
            keyword_completion("Title", "Human-readable title"),
            keyword_completion("Description", "Detailed description"),
        ];

        let specific = match entity_kind {
            EntityKind::Profile | EntityKind::Extension => vec![
                keyword_completion("Parent", "Base resource or profile"),
                keyword_completion("*", "Rule prefix"),
            ],
            EntityKind::ValueSet => vec![
                keyword_completion("*", "Include/exclude rules"),
            ],
            EntityKind::Instance => vec![
                keyword_completion("InstanceOf", "Profile to conform to"),
                keyword_completion("Usage", "Usage context (#example, #definition)"),
                keyword_completion("*", "Assignment rules"),
            ],
            _ => vec![],
        };

        [common, specific].concat()
    }
}

/// Create snippet completion item
fn snippet_completion(
    label: &str,
    snippet: &str,
    documentation: &str,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(documentation.to_string()),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        documentation: Some(Documentation::String(documentation.to_string())),
        ..Default::default()
    }
}

/// Create keyword completion item
fn keyword_completion(label: &str, documentation: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(documentation.to_string()),
        insert_text: Some(format!("{}: ", label)),
        ..Default::default()
    }
}

/// Map entity kind to completion item kind
fn completion_item_kind(kind: EntityKind) -> CompletionItemKind {
    match kind {
        EntityKind::Profile => CompletionItemKind::CLASS,
        EntityKind::Extension => CompletionItemKind::INTERFACE,
        EntityKind::ValueSet => CompletionItemKind::ENUM,
        EntityKind::CodeSystem => CompletionItemKind::ENUM,
        EntityKind::Instance => CompletionItemKind::VALUE,
        EntityKind::RuleSet => CompletionItemKind::SNIPPET,
        _ => CompletionItemKind::TEXT,
    }
}
```

### Completion Context Detection

```rust
/// Find completion context at cursor position
fn find_completion_context(
    root: &SyntaxNode,
    offset: usize,
) -> Option<CompletionContext> {
    let token = root.token_at_offset(TextSize::from(offset as u32))
        .right_biased()?;

    let node = token.parent()?;

    // Check for entity name context (after "Profile:", etc.)
    if let Some(entity_header) = node.parent()
        .and_then(|p| ast::EntityHeader::cast(p))
    {
        if token.text().ends_with(':') {
            return Some(CompletionContext::EntityName {
                kind: entity_header.kind(),
            });
        }
    }

    // Check for keyword value context
    if let Some(keyword_rule) = node.parent()
        .and_then(|p| ast::KeywordRule::cast(p))
    {
        if let Some(keyword) = keyword_rule.keyword() {
            return Some(CompletionContext::KeywordValue {
                keyword: keyword.text().to_string(),
            });
        }
    }

    // Check for FHIR path context
    if let Some(path_rule) = node.parent()
        .and_then(|p| ast::PathRule::cast(p))
    {
        if let Some(path) = path_rule.path() {
            let prefix = path.text().to_string();
            let parent_entity = find_parent_entity(&node)?;
            let parent_type = parent_entity.parent().unwrap_or("Resource");

            return Some(CompletionContext::FhirPath {
                prefix,
                parent_type: parent_type.to_string(),
            });
        }
    }

    // Check for type constraint context (after "only")
    if token.text() == "only" {
        return Some(CompletionContext::TypeConstraint);
    }

    // Check for binding context (after "from")
    if token.text() == "from" {
        return Some(CompletionContext::Binding);
    }

    // Check for RuleSet insert context
    if token.text() == "insert" {
        return Some(CompletionContext::RuleSetInsert);
    }

    // Check if inside entity body
    if let Some(entity) = find_parent_entity(&node) {
        return Some(CompletionContext::EntityBody {
            entity_kind: entity.kind(),
        });
    }

    // Default to top-level
    Some(CompletionContext::TopLevel)
}
```

## Trigger Characters

Configure trigger characters for automatic completion:

```rust
// In server capabilities
completion_provider: Some(CompletionOptions {
    trigger_characters: Some(vec![
        ":".to_string(),  // After "Profile:", "Parent:", etc.
        ".".to_string(),  // FHIR paths (name.given)
        "*".to_string(),  // Rule prefix
        " ".to_string(),  // After keywords
    ]),
    resolve_provider: Some(false),
    ..Default::default()
}),
```

## Implementation Location

**Primary File**: `crates/maki-lsp/src/completion.rs` (new file)

**Supporting Files**:
- `crates/maki-lsp/src/server.rs` - Integration with MakiLanguageServer
- `crates/maki-lsp/src/context.rs` - Context detection utilities
- `crates/maki-lsp/src/snippets.rs` - Snippet templates

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_entity_name() {
        let source = "Profile: ";
        let completions = get_completions(source, source.len());

        assert!(!completions.is_empty());
        // Should suggest existing profiles from workspace
    }

    #[test]
    fn test_complete_parent() {
        let source = "Profile: Test\nParent: ";
        let completions = get_completions(source, source.len());

        // Should suggest FHIR resources
        assert!(completions.iter().any(|c| c.label == "Patient"));
        assert!(completions.iter().any(|c| c.label == "Observation"));
    }

    #[test]
    fn test_complete_fhir_path() {
        let source = "Profile: Test\nParent: Patient\n* name.";
        let completions = get_completions(source, source.len());

        // Should suggest Patient.name elements
        assert!(completions.iter().any(|c| c.label == "name.given"));
        assert!(completions.iter().any(|c| c.label == "name.family"));
    }

    #[test]
    fn test_complete_type() {
        let source = "Profile: Test\nParent: Patient\n* identifier only ";
        let completions = get_completions(source, source.len());

        // Should suggest FHIR types
        assert!(completions.iter().any(|c| c.label == "string"));
        assert!(completions.iter().any(|c| c.label == "Identifier"));
    }

    #[test]
    fn test_complete_binding() {
        let source = "Profile: Test\nParent: Patient\n* gender from ";
        let completions = get_completions(source, source.len());

        // Should suggest ValueSets
        assert!(!completions.is_empty());
        assert!(completions.iter().all(|c| {
            c.kind == Some(CompletionItemKind::ENUM)
        }));
    }

    #[test]
    fn test_snippet_completion() {
        let source = "";
        let completions = get_completions(source, 0);

        // Should include snippet for Profile
        let profile_snippet = completions.iter()
            .find(|c| c.label == "Profile");

        assert!(profile_snippet.is_some());
        assert_eq!(
            profile_snippet.unwrap().insert_text_format,
            Some(InsertTextFormat::SNIPPET)
        );
    }
}
```

### Integration Tests

```typescript
// VS Code extension test
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Completion Tests', () => {
  test('Complete Parent keyword', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: Test\nParent: ',
      language: 'fsh',
    });

    const editor = await vscode.window.showTextDocument(doc);
    const position = doc.positionAt(doc.getText().length);

    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      doc.uri,
      position
    );

    assert.ok(completions);
    assert.ok(completions.items.length > 0);

    const patient = completions.items.find(c => c.label === 'Patient');
    assert.ok(patient);
  });

  test('Complete FHIR path', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: 'Profile: Test\nParent: Patient\n* name.',
      language: 'fsh',
    });

    await vscode.window.showTextDocument(doc);
    const position = doc.positionAt(doc.getText().length);

    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      doc.uri,
      position
    );

    const given = completions.items.find(c => c.label === 'name.given');
    assert.ok(given);
  });

  test('Snippet completion inserts template', async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: '',
      language: 'fsh',
    });

    const editor = await vscode.window.showTextDocument(doc);
    const position = new vscode.Position(0, 0);

    const completions = await vscode.commands.executeCommand<vscode.CompletionList>(
      'vscode.executeCompletionItemProvider',
      doc.uri,
      position
    );

    const profile = completions.items.find(c => c.label === 'Profile');
    assert.ok(profile);
    assert.strictEqual(profile.kind, vscode.CompletionItemKind.Snippet);
  });
});
```

## Performance Considerations

- **Caching**: Cache FHIR element lookups
- **Filtering**: Filter completions on server side
- **Fuzzy matching**: Implement fuzzy matching for better UX
- **Limit results**: Cap at 100 completions to avoid UI lag

**Performance Targets:**
- Completion response: <100ms
- FHIR path lookup: <20ms
- Filtering: <10ms for 1000 items

## Dependencies

### Required Components
- **LSP Server Foundation** (Task 40): For server infrastructure
- **Semantic Analyzer** (Task 13): For symbol resolution
- **FHIR Definitions** (Task 29): For element/type lookup

## Acceptance Criteria

- [ ] Entity name completions work
- [ ] Keyword completions work
- [ ] FHIR path completions work
- [ ] Type completions work
- [ ] Binding completions work
- [ ] RuleSet completions work
- [ ] Snippet completions insert templates
- [ ] Trigger characters activate completion
- [ ] Documentation shows for items
- [ ] Performance targets are met
- [ ] Unit tests cover all completion types
- [ ] Integration tests verify completion in VS Code

## Edge Cases

1. **Empty prefix**: Show all available completions
2. **Invalid context**: Return empty completions
3. **Cross-file references**: Resolve from workspace
4. **Deeply nested paths**: Handle long path completion
5. **Choice types**: Show all type options

## Future Enhancements

1. **Fuzzy matching**: Better search for completions
2. **Commit characters**: Auto-complete on certain characters
3. **Completion resolve**: Lazy-load documentation
4. **Context-aware sorting**: Rank by relevance
5. **Custom snippets**: User-defined templates

## Related Tasks

- **Task 40: LSP Server Foundation** - Provides server infrastructure
- **Task 13: Semantic Analyzer** - Provides symbol resolution
- **Task 29: FHIR Definitions** - Provides element/type lookup
- **Task 42: Hover Provider** - Similar documentation feature

---

**Status**: Ready for implementation
**Estimated Complexity**: High (context detection, multiple completion types)
**Priority**: High (critical for developer productivity)
**Updated**: 2025-11-03
