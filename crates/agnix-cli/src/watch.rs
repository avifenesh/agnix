//! Watch mode implementation for continuous validation

use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use rust_i18n::t;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

/// Run validation in watch mode, re-running on file changes
pub fn watch_and_validate<F>(path: &Path, mut validate_fn: F) -> anyhow::Result<()>
where
    F: FnMut() -> anyhow::Result<bool>,
{
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    // Initial validation
    println!("{}\n", t!("cli.watch_starting"));
    let _ = validate_fn();

    // Set up file watcher
    let (tx, rx) = channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
    debouncer.watcher().watch(path, RecursiveMode::Recursive)?;

    // Watch loop
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(events)) => {
                // Filter for relevant file changes
                let relevant = events.iter().any(|e| is_relevant_file(&e.path));
                if relevant {
                    clear_screen();
                    println!("{}\n", t!("cli.watch_changes_detected"));
                    let _ = validate_fn();
                }
            }
            Ok(Err(e)) => {
                eprintln!("{}", t!("cli.watch_error", error = format!("{:?}", e)));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue watching
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    println!("\n{}", t!("cli.watch_stopped"));
    Ok(())
}

fn is_relevant_file(path: &Path) -> bool {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Check for relevant filenames
    matches!(
        filename,
        "SKILL.md"
            | "CLAUDE.md"
            | "CLAUDE.local.md"
            | "AGENTS.md"
            | "AGENTS.local.md"
            | "AGENTS.override.md"
            | "settings.json"
            | "settings.local.json"
            | "plugin.json"
            | "copilot-instructions.md"
    ) || extension == "mcp"
        || filename.ends_with(".mcp.json")
        || filename.ends_with(".mdc")
        || filename.ends_with(".instructions.md")
        // Also watch for agent files
        || (extension == "md"
            && path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("agents"))
}

fn clear_screen() {
    // ANSI escape code to clear screen and move cursor to top
    print!("\x1B[2J\x1B[1;1H");
}
