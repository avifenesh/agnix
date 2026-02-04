//! agnix-lsp binary entry point.
//!
//! Starts the Language Server Protocol server for agnix validation.

#[tokio::main]
async fn main() {
    if let Err(e) = agnix_lsp::start_server().await {
        eprintln!("LSP server error: {e}");
        std::process::exit(1);
    }
}
