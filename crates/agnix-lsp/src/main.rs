#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("agnix-lsp {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Err(e) = agnix_lsp::start_server().await {
        eprintln!("LSP server error: {e}");
        std::process::exit(1);
    }
}
