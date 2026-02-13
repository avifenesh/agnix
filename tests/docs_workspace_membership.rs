use std::fs;

#[test]
fn claude_and_agents_docs_are_byte_identical() {
    let root = env!("CARGO_MANIFEST_DIR");
    let claude = fs::read(format!("{root}/CLAUDE.md")).expect("Failed to read CLAUDE.md");
    let agents = fs::read(format!("{root}/AGENTS.md")).expect("Failed to read AGENTS.md");
    assert_eq!(claude, agents, "CLAUDE.md and AGENTS.md must stay identical");
}

#[test]
fn architecture_docs_include_workspace_wasm_crate() {
    let root = env!("CARGO_MANIFEST_DIR");
    let cargo = fs::read_to_string(format!("{root}/Cargo.toml")).expect("Failed to read Cargo.toml");
    assert!(
        cargo.contains("crates/agnix-wasm"),
        "Cargo.toml workspace members must include crates/agnix-wasm"
    );

    for path in ["README.md", "SPEC.md", "CLAUDE.md", "AGENTS.md"] {
        let content =
            fs::read_to_string(format!("{root}/{path}")).unwrap_or_else(|_| panic!("Failed to read {path}"));
        assert!(
            content.contains("agnix-wasm"),
            "{path} must mention agnix-wasm to match workspace membership"
        );
    }
}
