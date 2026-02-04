//! Build script for agnix-rules.
//!
//! Generates Rust code from rules.json at compile time.
//! Supports both local crate builds (crates.io) and workspace builds (development).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Find the workspace root by searching for Cargo.toml with [workspace]
fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|path| {
            path.join("Cargo.toml")
                .exists()
                .then(|| fs::read_to_string(path.join("Cargo.toml")).ok())
                .flatten()
                .is_some_and(|content| {
                    content.contains("[workspace]") || content.contains("[workspace.")
                })
        })
        .map(|p| p.to_path_buf())
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = Path::new(&manifest_dir);

    // Try crate-local rules.json first (for crates.io builds)
    // Then fall back to workspace knowledge-base/rules.json (for development)
    let crate_rules = manifest_path.join("rules.json");
    let workspace_rules = find_workspace_root(manifest_path)
        .map(|root| root.join("knowledge-base/rules.json"));

    // Watch crate-local path for changes (always, in case file is added later)
    println!("cargo:rerun-if-changed={}", crate_rules.display());

    let rules_path = if crate_rules.exists() {
        crate_rules
    } else if let Some(ws_rules) = workspace_rules {
        if ws_rules.exists() {
            // Also watch workspace rules for development builds
            println!("cargo:rerun-if-changed={}", ws_rules.display());
            ws_rules
        } else {
            panic!(
                "Could not find rules.json at {} or {}",
                manifest_path.join("rules.json").display(),
                ws_rules.display()
            );
        }
    } else {
        panic!(
            "Could not find rules.json at {} (no workspace root found)",
            manifest_path.join("rules.json").display()
        );
    };

    let rules_json = fs::read_to_string(&rules_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read rules.json at {}: {}",
            rules_path.display(),
            e
        )
    });

    // Parse to validate JSON structure
    let rules: serde_json::Value = serde_json::from_str(&rules_json).unwrap_or_else(|e| {
        panic!(
            "Failed to parse rules.json at {}: {}",
            rules_path.display(),
            e
        )
    });

    // Extract just the rules array and generate Rust code
    let rules_array = rules["rules"]
        .as_array()
        .expect("rules.json must have a 'rules' array");

    let mut generated_code = String::new();
    generated_code.push_str("// Auto-generated from rules.json by build.rs\n");
    generated_code.push_str("// Do not edit manually!\n\n");
    generated_code.push_str("/// Rule data as (id, name) tuples.\n");
    generated_code.push_str("/// \n");
    generated_code.push_str("/// This is the complete list of validation rules from knowledge-base/rules.json.\n");
    generated_code.push_str("pub const RULES_DATA: &[(&str, &str)] = &[\n");

    for rule in rules_array {
        let id = rule["id"].as_str().expect("rule must have id");
        let name = rule["name"].as_str().expect("rule must have name");
        // Escape special characters for Rust string literal (defense-in-depth)
        let escape_str = |s: &str| {
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
        };
        let escaped_id = escape_str(id);
        let escaped_name = escape_str(name);
        generated_code.push_str(&format!(
            "    (\"{}\", \"{}\"),\n",
            escaped_id, escaped_name
        ));
    }

    generated_code.push_str("];\n");

    // Write to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("rules_data.rs");
    fs::write(&dest_path, generated_code).expect("Failed to write generated rules");
}
