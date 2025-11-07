# Task 40: LSP Server Foundation

**Phase**: 4 (Language Server - Weeks 15-16)
**Time Estimate**: 2-3 days
**Status**: 📝 Planned
**Priority**: High
**Dependencies**: Tasks 01-39 (Core infrastructure, Parser, Formatter, Diagnostics)

## Overview

Implement the foundational Language Server Protocol (LSP) infrastructure for FSH editing. This includes setting up a tower-lsp server, document synchronization, incremental parsing, workspace management, and robust error recovery. The LSP server provides the backend for real-time IDE features like diagnostics, completion, and go-to-definition.

**Part of LSP Phase**: Weeks 15-16 focus on core LSP infrastructure, Weeks 17-18 add LSP features (Tasks 41-47).

## Context

Language Server Protocol enables rich IDE support for FSH:

- **Real-time diagnostics**: Show errors as user types
- **IntelliSense**: Code completion for FHIR elements and FSH entities
- **Navigation**: Go to definition, find references
- **Refactoring**: Rename symbols, extract rulesets
- **Formatting**: Format document on save

The LSP server acts as a backend service that editors (VS Code, Vim, Emacs, etc.) communicate with via JSON-RPC. A solid foundation is critical for all LSP features.

## Goals

1. **Set up tower-lsp server** - Implement LanguageServer trait
2. **Document synchronization** - Handle file open/change/save/close
3. **Incremental parsing** - Parse efficiently as user types
4. **Workspace management** - Load entire FSH project with symbol table
5. **Error recovery** - Robust parsing that handles incomplete code
6. **Server capabilities** - Advertise supported features to clients
7. **Performance optimization** - Fast response times (<50ms)

## Technical Specification

### Server Structure

```rust
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main LSP server for FSH
pub struct MakiLanguageServer {
    /// LSP client for sending notifications
    client: Client,

    /// Open documents (URI → Document)
    documents: DashMap<Url, Document>,

    /// Workspace state
    workspace: Arc<RwLock<Workspace>>,
}

impl MakiLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            workspace: Arc::new(RwLock::new(Workspace::new())),
        }
    }
}

/// Document state for an open FSH file
#[derive(Debug, Clone)]
pub struct Document {
    /// Document URI
    pub uri: Url,

    /// Current text content
    pub content: String,

    /// Parsed CST
    pub cst: SyntaxNode,

    /// Document version (increments on each change)
    pub version: i32,

    /// Parse errors
    pub parse_errors: Vec<ParseError>,

    /// Semantic model
    pub semantic: Option<SemanticModel>,
}

impl Document {
    pub fn new(uri: Url, content: String, version: i32) -> Self {
        let parse_result = crate::parse(&content);

        Self {
            uri,
            content: content.clone(),
            cst: parse_result.syntax(),
            version,
            parse_errors: parse_result.errors().to_vec(),
            semantic: None,
        }
    }

    /// Update document with new content
    pub fn update(&mut self, content: String, version: i32) {
        self.content = content.clone();
        self.version = version;

        // Re-parse
        let parse_result = crate::parse(&content);
        self.cst = parse_result.syntax();
        self.parse_errors = parse_result.errors().to_vec();

        // Invalidate semantic model
        self.semantic = None;
    }

    /// Get or build semantic model
    pub fn semantic(&mut self) -> &SemanticModel {
        if self.semantic.is_none() {
            self.semantic = Some(SemanticAnalyzer::analyze(&self.cst));
        }
        self.semantic.as_ref().unwrap()
    }
}

/// Workspace manages all files in the FSH project
#[derive(Debug)]
pub struct Workspace {
    /// Root directory of the project
    pub root: Option<PathBuf>,

    /// All files in the project (path → Document)
    pub files: HashMap<PathBuf, Document>,

    /// Global symbol table
    pub symbol_table: SymbolTable,

    /// FHIR definitions
    pub fhir_defs: Arc<FhirDefinitions>,

    /// File dependencies (for invalidation)
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            root: None,
            files: HashMap::new(),
            symbol_table: SymbolTable::new(),
            fhir_defs: Arc::new(FhirDefinitions::load_default()),
            dependencies: HashMap::new(),
        }
    }

    /// Initialize workspace from root path
    pub fn initialize(&mut self, root: PathBuf) {
        self.root = Some(root.clone());

        // Find all FSH files
        let fsh_files = find_fsh_files(&root);

        // Load each file
        for path in fsh_files {
            if let Ok(content) = fs::read_to_string(&path) {
                let uri = Url::from_file_path(&path).unwrap();
                let doc = Document::new(uri.clone(), content, 0);
                self.files.insert(path, doc);
            }
        }

        // Build symbol table
        self.rebuild_symbol_table();

        // Analyze dependencies
        self.rebuild_dependencies();
    }

    /// Rebuild symbol table from all files
    fn rebuild_symbol_table(&mut self) {
        self.symbol_table.clear();

        for doc in self.files.values() {
            // Extract symbols from each document
            let symbols = extract_symbols(&doc.cst);
            for symbol in symbols {
                self.symbol_table.insert(symbol);
            }
        }
    }

    /// Rebuild dependency graph
    fn rebuild_dependencies(&mut self) {
        self.dependencies.clear();

        for (path, doc) in &self.files {
            let deps = find_dependencies(&doc.cst, &self.symbol_table);
            self.dependencies.insert(path.clone(), deps);
        }
    }

    /// Get files that depend on the given file
    pub fn get_dependents(&self, path: &Path) -> Vec<PathBuf> {
        self.dependencies
            .iter()
            .filter_map(|(p, deps)| {
                if deps.contains(&path.to_path_buf()) {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}
```

### LanguageServer Implementation

```rust
#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Get workspace root
        let root = params.root_uri
            .and_then(|uri| uri.to_file_path().ok());

        // Initialize workspace
        if let Some(root) = root {
            let mut workspace = self.workspace.write().await;
            workspace.initialize(root);
        }

        // Advertise server capabilities
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(
                            SaveOptions {
                                include_text: Some(true),
                            }
                        )),
                        ..Default::default()
                    }
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "*".to_string(),
                    ]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "maki-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "MAKI LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        // Create document
        let doc = Document::new(uri.clone(), content, version);

        // Store document
        self.documents.insert(uri.clone(), doc.clone());

        // Update workspace
        if let Ok(path) = uri.to_file_path() {
            let mut workspace = self.workspace.write().await;
            workspace.files.insert(path, doc.clone());
            workspace.rebuild_symbol_table();
        }

        // Publish diagnostics
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // Get document
        if let Some(mut doc_ref) = self.documents.get_mut(&uri) {
            // Apply changes
            for change in params.content_changes {
                match change.range {
                    Some(range) => {
                        // Incremental change
                        apply_incremental_change(&mut doc_ref.content, range, &change.text);
                    }
                    None => {
                        // Full document update
                        doc_ref.content = change.text;
                    }
                }
            }

            // Re-parse with debouncing
            doc_ref.update(doc_ref.content.clone(), version);

            // Update workspace
            if let Ok(path) = uri.to_file_path() {
                let mut workspace = self.workspace.write().await;
                workspace.files.insert(path.clone(), doc_ref.clone());

                // Invalidate dependents
                let dependents = workspace.get_dependents(&path);
                for dep in dependents {
                    // Re-validate dependent files
                    if let Some(dep_uri) = Url::from_file_path(&dep).ok() {
                        self.publish_diagnostics(dep_uri).await;
                    }
                }
            }
        }

        // Publish diagnostics
        self.publish_diagnostics(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;

        // Trigger full validation on save
        self.publish_diagnostics(uri.clone()).await;

        // Rebuild workspace symbol table
        let mut workspace = self.workspace.write().await;
        workspace.rebuild_symbol_table();
        workspace.rebuild_dependencies();
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        // Remove document from memory
        self.documents.remove(&uri);

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}

impl MakiLanguageServer {
    /// Publish diagnostics for a document
    async fn publish_diagnostics(&self, uri: Url) {
        if let Some(doc) = self.documents.get(&uri) {
            let mut diagnostics = Vec::new();

            // Add parse errors
            for error in &doc.parse_errors {
                diagnostics.push(error.to_lsp_diagnostic());
            }

            // Run lint rules
            let workspace = self.workspace.read().await;
            let lint_diagnostics = run_lint_rules(&doc.cst, &workspace);
            diagnostics.extend(lint_diagnostics);

            // Send to client
            self.client
                .publish_diagnostics(uri.clone(), diagnostics, Some(doc.version))
                .await;
        }
    }
}

/// Apply incremental text change
fn apply_incremental_change(content: &mut String, range: Range, new_text: &str) {
    let start = position_to_offset(content, range.start);
    let end = position_to_offset(content, range.end);

    content.replace_range(start..end, new_text);
}

/// Convert LSP Position to byte offset
fn position_to_offset(content: &str, position: Position) -> usize {
    let mut offset = 0;
    let mut line = 0;

    for (i, ch) in content.chars().enumerate() {
        if line == position.line as usize {
            if offset == position.character as usize {
                return i;
            }
            offset += 1;
        }

        if ch == '\n' {
            line += 1;
            offset = 0;
        }
    }

    content.len()
}
```

### Main Binary

```rust
// crates/maki-lsp/src/main.rs

use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| MakiLanguageServer::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}
```

### CLI Integration

```rust
// In maki-cli/src/commands/lsp.rs

#[derive(Parser)]
pub struct LspCommand {
    /// Log level for LSP server
    #[arg(long, default_value = "info")]
    log_level: String,
}

impl LspCommand {
    pub fn execute(&self) -> Result<()> {
        // Set up logging
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(&self.log_level)
        ).init();

        // Run LSP server
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (service, socket) = LspService::new(|client| {
                MakiLanguageServer::new(client)
            });

            Server::new(stdin, stdout, socket).serve(service).await;
        });

        Ok(())
    }
}
```

## VS Code Extension Setup

**File**: `editors/vscode/package.json`

```json
{
  "name": "maki-fsh",
  "displayName": "MAKI FSH Language Support",
  "description": "Language support for FHIR Shorthand (FSH)",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": ["Programming Languages"],
  "activationEvents": ["onLanguage:fsh"],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [
      {
        "id": "fsh",
        "aliases": ["FSH", "FHIR Shorthand"],
        "extensions": [".fsh"],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "fsh",
        "scopeName": "source.fsh",
        "path": "./syntaxes/fsh.tmLanguage.json"
      }
    ],
    "configuration": {
      "title": "MAKI FSH",
      "properties": {
        "maki.lsp.path": {
          "type": "string",
          "default": "maki",
          "description": "Path to maki binary"
        },
        "maki.format.indentSize": {
          "type": "number",
          "default": 2,
          "description": "Number of spaces per indent"
        }
      }
    }
  },
  "dependencies": {
    "vscode-languageclient": "^8.1.0"
  }
}
```

**File**: `editors/vscode/src/extension.ts`

```typescript
import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  // Get maki binary path from settings
  const config = workspace.getConfiguration('maki');
  const makiPath = config.get<string>('lsp.path') || 'maki';

  // Server executable
  const serverExecutable: Executable = {
    command: makiPath,
    args: ['lsp'],
  };

  const serverOptions: ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable,
  };

  // Client options
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'fsh' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*.fsh'),
    },
  };

  // Create and start client
  client = new LanguageClient(
    'makiLsp',
    'MAKI LSP',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
```

## Implementation Location

**Primary Crate**: `crates/maki-lsp/` (new crate)

**Files**:
- `src/main.rs` - LSP server binary entry point
- `src/server.rs` - MakiLanguageServer implementation
- `src/document.rs` - Document management
- `src/workspace.rs` - Workspace management
- `src/capabilities.rs` - Server capabilities

**CLI Integration**: `crates/maki-cli/src/commands/lsp.rs`

**VS Code Extension**: `editors/vscode/` (new directory)

## Testing Requirements

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_parse_on_creation() {
        let uri = Url::parse("file:///test.fsh").unwrap();
        let content = "Profile: MyProfile\nParent: Patient";

        let doc = Document::new(uri, content.to_string(), 0);

        assert_eq!(doc.version, 0);
        assert!(doc.parse_errors.is_empty());
    }

    #[test]
    fn test_document_update() {
        let uri = Url::parse("file:///test.fsh").unwrap();
        let mut doc = Document::new(uri, "Profile: Test".to_string(), 0);

        doc.update("Profile: Updated".to_string(), 1);

        assert_eq!(doc.version, 1);
        assert_eq!(doc.content, "Profile: Updated");
    }

    #[test]
    fn test_position_to_offset() {
        let content = "Line 1\nLine 2\nLine 3";

        let offset = position_to_offset(content, Position { line: 1, character: 2 });
        assert_eq!(offset, 9); // "Line 1\nLi"
    }

    #[test]
    fn test_workspace_initialization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fsh_file = temp_dir.path().join("test.fsh");
        fs::write(&fsh_file, "Profile: Test").unwrap();

        let mut workspace = Workspace::new();
        workspace.initialize(temp_dir.path().to_path_buf());

        assert_eq!(workspace.files.len(), 1);
        assert!(workspace.files.contains_key(&fsh_file));
    }
}
```

### Integration Tests

```bash
# Start LSP server
maki lsp &
LSP_PID=$!

# Test with LSP test client
npm install -g @vscode/test-cli
vscode-test run-tests --test-workspace test-workspace

# Stop server
kill $LSP_PID
```

## Performance Targets

- **Initialization**: <1 second for 100 files
- **Document open**: <50ms
- **Document change**: <20ms (incremental parsing)
- **Diagnostics**: <100ms for 1000-line file
- **Memory**: <100MB for typical project

## Dependencies

### Crate Dependencies

```toml
[dependencies]
tower-lsp = "0.20"
tokio = { version = "1.35", features = ["full"] }
lsp-types = "0.95"
dashmap = "5.5"
url = "2.5"
env_logger = "0.11"
log = "0.4"

maki-core = { path = "../maki-core" }
maki-rules = { path = "../maki-rules" }
```

### Required Components
- **Parser** (Task 03-04): For parsing FSH
- **Semantic Analyzer** (Task 13): For symbol resolution
- **Diagnostics** (Task 06): For error reporting
- **Linter** (Tasks 30-36): For running lint rules

## Acceptance Criteria

- [ ] LSP server starts and responds to initialization
- [ ] Document synchronization (open/change/save/close) works
- [ ] Incremental parsing updates CST efficiently
- [ ] Workspace loads all FSH files on initialization
- [ ] Symbol table builds correctly from all files
- [ ] Dependency tracking invalidates dependent files
- [ ] Parse errors are published as diagnostics
- [ ] Lint rules run and publish diagnostics
- [ ] VS Code extension connects to LSP server
- [ ] Extension activates on .fsh files
- [ ] Server handles multiple documents concurrently
- [ ] Performance targets are met
- [ ] Unit tests cover document and workspace management
- [ ] Integration tests verify LSP protocol compliance

## Edge Cases

1. **Empty files**: Should parse without errors
2. **Very large files**: Handle files >10MB efficiently
3. **Rapid typing**: Debounce parsing to avoid thrashing
4. **Concurrent changes**: Handle multiple documents changing simultaneously
5. **Invalid UTF-8**: Gracefully handle encoding errors
6. **File deletion**: Remove from workspace, clear diagnostics
7. **Workspace reload**: Reinitialize workspace on configuration change

## Future Enhancements

1. **Incremental symbol table updates**: Only update changed symbols
2. **Caching**: Cache parse results and semantic models on disk
3. **Multi-workspace support**: Support multiple FSH projects
4. **Remote LSP**: Support LSP over HTTP for web editors
5. **Performance profiling**: Built-in performance diagnostics

## Related Tasks

- **Task 41: Diagnostics Publisher** - Publishes lint diagnostics
- **Task 42: Hover Provider** - Shows documentation on hover
- **Task 43: Completion Provider** - IntelliSense for FSH
- **Task 44: Go to Definition** - Navigate to definitions
- **Task 45: Find References** - Find all usages
- **Task 46: Rename Symbol** - Rename across files
- **Task 47: Code Actions** - Quick fixes and refactorings

---

**Status**: Ready for implementation
**Estimated Complexity**: High (LSP protocol, async I/O, state management)
**Priority**: High (enables IDE support)
**Updated**: 2025-11-03
