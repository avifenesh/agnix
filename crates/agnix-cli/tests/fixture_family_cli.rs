use assert_cmd::Command;

fn agnix() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agnix");
    cmd.current_dir(workspace_root());
    cmd
}

fn workspace_root() -> &'static std::path::Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") || content.contains("[workspace.") {
                    return ancestor.to_path_buf();
                }
            }
        }
        panic!(
            "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
            manifest_dir.display()
        );
    })
    .as_path()
}

fn run_json(path: &std::path::Path) -> serde_json::Value {
    let output = agnix()
        .arg(path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!("Expected valid JSON output for {}, got: {}", path.display(), stdout)
    })
}

fn assert_has_rule(json: &serde_json::Value, rule: &str) {
    let diagnostics = json["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics missing in JSON output"));
    let found = diagnostics
        .iter()
        .any(|d| d["rule"].as_str() == Some(rule));
    assert!(found, "Expected {} in diagnostics", rule);
}

#[test]
fn test_cli_reports_xml_fixtures() {
    let path = workspace_root().join("tests/fixtures/xml");
    let json = run_json(&path);
    assert_has_rule(&json, "XML-001");
    assert_has_rule(&json, "XML-002");
    assert_has_rule(&json, "XML-003");
}

#[test]
fn test_cli_reports_ref_fixtures() {
    let path = workspace_root().join("tests/fixtures/refs");
    let json = run_json(&path);
    assert_has_rule(&json, "REF-001");
    assert_has_rule(&json, "REF-002");
}

#[test]
fn test_cli_reports_mcp_fixtures() {
    let path = workspace_root().join("tests/fixtures/mcp");
    let json = run_json(&path);
    assert_has_rule(&json, "MCP-001");
    assert_has_rule(&json, "MCP-006");
}

#[test]
fn test_cli_reports_agm_fixtures() {
    let path = workspace_root().join("tests/fixtures/agents_md/no-headers");
    let json = run_json(&path);
    assert_has_rule(&json, "AGM-002");
}

#[test]
fn test_cli_reports_xp_fixtures() {
    let path = workspace_root().join("tests/fixtures/cross_platform/hard-coded");
    let json = run_json(&path);
    assert_has_rule(&json, "XP-003");
}

#[test]
fn test_cli_reports_pe_fixtures() {
    let source = workspace_root().join("tests/fixtures/prompt/pe-001-critical-in-middle.md");
    let content = std::fs::read_to_string(&source)
        .unwrap_or_else(|_| panic!("Failed to read {}", source.display()));

    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(&claude_path, content).unwrap();

    let json = run_json(&temp.path().to_path_buf());
    assert_has_rule(&json, "PE-001");
}