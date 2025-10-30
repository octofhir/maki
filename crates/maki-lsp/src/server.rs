//! LSP server implementation for FHIR Shorthand
//!
//! This module provides the Language Server Protocol implementation
//! for FHIR Shorthand, enabling IDE features.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// MAKI Language Server
///
/// Implements the Language Server Protocol for FHIR Shorthand files.
pub struct MakiLanguageServer {
    client: Client,
}

impl MakiLanguageServer {
    /// Create a new MAKI language server
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for MakiLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "maki-lsp".to_string(),
                version: Some(crate::VERSION.to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // Additional capabilities will be added in future tasks
                ..Default::default()
            },
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
}
