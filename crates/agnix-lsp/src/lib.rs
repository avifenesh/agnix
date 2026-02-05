//! # agnix-lsp
//!
//! Language Server Protocol implementation for agnix.
//!
//! Provides real-time validation of agent configuration files in editors
//! that support LSP (VS Code, Neovim, Helix, etc.).
//!
//! ## Features
//!
//! - Real-time diagnostics on file open, change, and save
//! - Quick-fix code actions for automatic repairs
//! - Hover documentation for configuration fields
//! - Supports all agnix validation rules
//! - Maps agnix diagnostics to LSP diagnostics
//!
//! ## Usage
//!
//! Run the LSP server:
//!
//! ```bash
//! agnix-lsp
//! ```
//!
//! The server communicates over stdin/stdout using the LSP protocol.

rust_i18n::i18n!("../../locales", fallback = "en");

mod backend;
mod code_actions;
mod diagnostic_mapper;
mod hover_provider;
mod position;
mod vscode_config;

pub use backend::Backend;
pub use vscode_config::{VsCodeConfig, VsCodeRules, VsCodeSpecs, VsCodeVersions};

use tower_lsp::{LspService, Server};

/// Start the LSP server.
///
/// This function sets up stdin/stdout communication and runs the server
/// until shutdown is requested.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a fatal error.
pub async fn start_server() -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
