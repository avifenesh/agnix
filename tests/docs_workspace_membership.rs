use std::fs;

#[test]
fn claude_and_agents_docs_are_byte_identical() {
    let root = env!("CARGO_MANIFEST_DIR");
    let claude = fs::read(format!("{root}/CLAUDE.md")).expect("Failed to read CLAUDE.md");
    let agents = fs::read(format!("{root}/AGENTS.md")).expect("Failed to read AGENTS.md");
    assert_eq!(
        claude, agents,
        "CLAUDE.md and AGENTS.md must stay identical"
    );
}

#[test]
fn architecture_docs_list_all_workspace_crates() {
    let root = env!("CARGO_MANIFEST_DIR");

    // Parse Cargo.toml to extract workspace members
    let cargo_content =
        fs::read_to_string(format!("{root}/Cargo.toml")).expect("Failed to read Cargo.toml");
    let cargo_toml: toml::Value =
        toml::from_str(&cargo_content).expect("Failed to parse Cargo.toml as TOML");

    let members = cargo_toml
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("Cargo.toml must have workspace.members array");

    // Extract crate names by stripping the "crates/" prefix
    let crate_names: Vec<&str> = members
        .iter()
        .filter_map(|m| m.as_str())
        .filter_map(|m| m.strip_prefix("crates/"))
        .collect();

    assert!(
        !crate_names.is_empty(),
        "No crate names extracted from workspace.members"
    );

    // Check each doc file mentions every workspace crate
    let doc_files = [
        "CLAUDE.md",
        "AGENTS.md",
        "README.md",
        "SPEC.md",
        "CONTRIBUTING.md",
    ];

    let mut mismatches: Vec<String> = Vec::new();

    for doc_file in &doc_files {
        let content = fs::read_to_string(format!("{root}/{doc_file}"))
            .unwrap_or_else(|_| panic!("Failed to read {doc_file}"));

        for crate_name in &crate_names {
            if !content.contains(crate_name) {
                mismatches.push(format!("  - {doc_file} is missing mention of `{crate_name}`"));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "Workspace crate graph is out of sync with documentation.\n\
         Workspace members: {crate_names:?}\n\
         Mismatches found:\n{}\n\n\
         Fix: add the missing crate names to the listed doc files \
         so they stay in sync with Cargo.toml workspace.members.",
        mismatches.join("\n")
    );
}
