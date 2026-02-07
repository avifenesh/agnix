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
        panic!(
            "Expected valid JSON output for {}, got: {}",
            path.display(),
            stdout
        )
    })
}

fn assert_has_rule(json: &serde_json::Value, rule: &str) {
    let diagnostics = json["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics missing in JSON output"));
    let found = diagnostics.iter().any(|d| d["rule"].as_str() == Some(rule));
    assert!(found, "Expected {} in diagnostics", rule);
}

fn count_rule(json: &serde_json::Value, rule: &str) -> usize {
    json["diagnostics"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|d| d["rule"].as_str() == Some(rule))
                .count()
        })
        .unwrap_or(0)
}

macro_rules! make_cli_test {
    ($name:ident, $path:expr, [$($rule:expr),+ $(,)?]) => {
        #[test]
        fn $name() {
            let path = workspace_root().join("tests/fixtures").join($path);
            let json = run_json(&path);
            $(assert_has_rule(&json, $rule);)+
        }
    };
}

make_cli_test!(
    test_cli_reports_xml_fixtures,
    "xml",
    ["XML-001", "XML-002", "XML-003"]
);
// REF-001 still fires on generic markdown (tests @import syntax)
make_cli_test!(test_cli_reports_ref_001_fixtures, "refs", ["REF-001"]);

// REF-002 only fires on agent config files, so we need a CLAUDE.md with broken links
#[test]
fn test_cli_reports_ref_002_fixtures() {
    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(
        &claude_path,
        "# Project\n\nSee [guide](missing-guide.md) for more details.\n",
    )
    .unwrap();

    let json = run_json(temp.path());
    assert_has_rule(&json, "REF-002");
}
make_cli_test!(test_cli_reports_mcp_fixtures, "mcp", ["MCP-001", "MCP-006"]);
make_cli_test!(
    test_cli_reports_agm_fixtures,
    "agents_md/no-headers",
    ["AGM-002"]
);
make_cli_test!(
    test_cli_reports_xp_fixtures,
    "cross_platform/hard-coded",
    ["XP-003"]
);

make_cli_test!(
    test_cli_reports_codex_invalid_fixtures,
    "codex-invalid",
    ["CDX-001", "CDX-002", "CDX-003"]
);

#[test]
fn test_cli_codex_invalid_fixture_counts() {
    let path = workspace_root().join("tests/fixtures/codex-invalid");
    let json = run_json(&path);
    // The codex-invalid fixture has exactly one of each CDX rule:
    // CDX-001 from approvalMode = "yolo", CDX-002 from fullAutoErrorMode = "crash",
    // CDX-003 from AGENTS.override.md
    assert_eq!(
        count_rule(&json, "CDX-001"),
        1,
        "Expected exactly 1 CDX-001"
    );
    assert_eq!(
        count_rule(&json, "CDX-002"),
        1,
        "Expected exactly 1 CDX-002"
    );
    assert_eq!(
        count_rule(&json, "CDX-003"),
        1,
        "Expected exactly 1 CDX-003"
    );
}

#[test]
fn test_cli_reports_pe_fixtures() {
    let source = workspace_root().join("tests/fixtures/prompt/pe-001-critical-in-middle.md");
    let content = std::fs::read_to_string(&source)
        .unwrap_or_else(|_| panic!("Failed to read {}", source.display()));

    let temp = tempfile::TempDir::new().unwrap();
    let claude_path = temp.path().join("CLAUDE.md");
    std::fs::write(&claude_path, content).unwrap();

    let json = run_json(temp.path());
    assert_has_rule(&json, "PE-001");
}
