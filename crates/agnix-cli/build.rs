//! Build script for agnix-cli.
//!
//! Embeds rules.json at compile time for SARIF rule generation.
//! This ensures the CLI always has the rules available without
//! needing to read the file at runtime.

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
    // Find workspace root dynamically (resilient to directory structure changes)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = find_workspace_root(Path::new(&manifest_dir))
        .expect("Could not find workspace root containing [workspace] in Cargo.toml");

    let rules_path = workspace_root.join("knowledge-base/rules.json");

    // Tell Cargo to re-run this build script if rules.json changes
    println!("cargo:rerun-if-changed={}", rules_path.display());

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
    generated_code.push_str("// Auto-generated from knowledge-base/rules.json by build.rs\n");
    generated_code.push_str("// Do not edit manually!\n\n");
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
    let dest_path = Path::new(&out_dir).join("sarif_rules.rs");
    fs::write(&dest_path, generated_code).expect("Failed to write generated rules");
}
