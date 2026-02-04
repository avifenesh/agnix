//! Tests to ensure agnix-rules/rules.json stays in sync with knowledge-base/rules.json.
//!
//! This is an integration test that runs in the workspace context, so it has access
//! to both files. For crates.io builds, only the crate-local rules.json is available.

use std::fs;
use std::path::Path;

fn find_workspace_root() -> Option<std::path::PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
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

#[test]
fn test_rules_json_parity() {
    let workspace_root = find_workspace_root();

    // Skip this test if we can't find the workspace root (e.g., crates.io build)
    let Some(root) = workspace_root else {
        eprintln!("Skipping parity test: workspace root not found");
        return;
    };

    let crate_rules_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("rules.json");
    let kb_rules_path = root.join("knowledge-base/rules.json");

    // Skip if either file doesn't exist
    if !crate_rules_path.exists() || !kb_rules_path.exists() {
        eprintln!("Skipping parity test: one or both rules.json files not found");
        return;
    }

    let crate_rules = fs::read_to_string(&crate_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", crate_rules_path.display(), e));
    let kb_rules = fs::read_to_string(&kb_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", kb_rules_path.display(), e));

    // Parse both as JSON to compare semantically (ignoring whitespace differences)
    let crate_json: serde_json::Value = serde_json::from_str(&crate_rules)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", crate_rules_path.display(), e));
    let kb_json: serde_json::Value = serde_json::from_str(&kb_rules)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", kb_rules_path.display(), e));

    assert_eq!(
        crate_json, kb_json,
        "rules.json files are out of sync!\n\
         crates/agnix-rules/rules.json and knowledge-base/rules.json must be identical.\n\
         Copy the updated file: cp knowledge-base/rules.json crates/agnix-rules/rules.json"
    );
}

#[test]
fn test_rules_count_matches_exported() {
    // Verify that RULES_DATA has the expected number of rules
    assert_eq!(
        agnix_rules::rule_count(),
        99,
        "Expected 99 rules in RULES_DATA"
    );
}
