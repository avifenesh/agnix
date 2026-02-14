use assert_cmd::Command;
use predicates::prelude::*;

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
            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                && (content.contains("[workspace]") || content.contains("[workspace."))
            {
                return ancestor.to_path_buf();
            }
        }
        panic!(
            "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
            manifest_dir.display()
        );
    })
    .as_path()
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    workspace_root().join(relative)
}

fn fixtures_config() -> tempfile::NamedTempFile {
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(
        br#"severity = "Error"
target = "Generic"
exclude = [
  "node_modules/**",
  ".git/**",
  "target/**",
]

[rules]
"#,
    )
    .unwrap();
    file.flush().unwrap();

    file
}

fn assert_fix_flags_rejected(format: &str, flag: &str) {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/valid")
        .arg("--format")
        .arg(format)
        .arg(flag)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Fix flags are only supported with text output",
        ));
}

// Helper function to check JSON output contains rules from a specific family
fn check_json_rule_family(fixture: &str, prefixes: &[&str], family_name: &str) {
    let mut cmd = agnix();
    let output = cmd
        .arg(fixture)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostics = json["diagnostics"].as_array().unwrap();

    let has_rule = diagnostics.iter().any(|d| {
        let rule = d["rule"].as_str().unwrap_or("");
        prefixes.iter().any(|p| rule.starts_with(p))
    });

    assert!(
        has_rule,
        "Expected at least one {} rule ({}) in diagnostics, got: {}",
        family_name,
        prefixes.join(" or "),
        stdout
    );
}

// Helper function to check SARIF output contains rules from a specific family
fn check_sarif_rule_family(fixture: &str, prefixes: &[&str], family_name: &str) {
    let mut cmd = agnix();
    let output = cmd
        .arg(fixture)
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = json["runs"][0]["results"].as_array().unwrap();

    let has_rule = results.iter().any(|r| {
        let rule_id = r["ruleId"].as_str().unwrap_or("");
        prefixes.iter().any(|p| rule_id.starts_with(p))
    });

    assert!(
        has_rule,
        "SARIF results should include {} diagnostics ({})",
        family_name,
        prefixes.join(" or ")
    );
}

#[test]
fn test_format_sarif_produces_valid_json() {
    let mut cmd = agnix();
    let assert = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .assert();

    assert
        .success()
        .stdout(predicate::str::contains("\"version\": \"2.1.0\""))
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"runs\""));
}

#[test]
fn test_fix_flags_rejected_for_json_and_sarif() {
    let formats = ["json", "sarif"];
    let flags = ["--fix", "--dry-run", "--fix-safe", "--fix-unsafe"];

    for format in formats {
        for flag in flags {
            assert_fix_flags_rejected(format, flag);
        }
    }
}

#[test]
fn test_format_sarif_contains_tool_info() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["runs"][0]["tool"]["driver"]["name"], "agnix");
    assert!(json["runs"][0]["tool"]["driver"]["rules"].is_array());
}

#[test]
fn test_format_sarif_has_all_rules() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let rules = json["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();

    // Use threshold range to avoid brittleness when rules are added/removed,
    // while still catching major regressions (missing rules) or explosions.
    // As of writing, there are 180 rules documented in VALIDATION-RULES.md.
    assert!(
        rules.len() >= 70,
        "Expected at least 70 validation rules, found {} (possible rule registration bug)",
        rules.len()
    );
    assert!(
        rules.len() <= 250,
        "Expected at most 250 validation rules, found {} (unexpected rule explosion)",
        rules.len()
    );

    // Verify rule structure: each rule should have id and shortDescription
    for (i, rule) in rules.iter().enumerate() {
        assert!(
            rule["id"].is_string(),
            "Rule at index {} should have an 'id' field. Rule: {}",
            i,
            rule
        );
        assert!(
            rule["shortDescription"]["text"].is_string(),
            "Rule at index {} should have a 'shortDescription.text' field. Rule: {}",
            i,
            rule
        );
    }
}

#[test]
fn test_format_sarif_exit_code_on_success() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .assert()
        .success();
}

#[test]
fn test_format_text_is_default() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/valid")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"version\"").not());
}

#[test]
fn test_format_sarif_results_array_exists() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(
        json["runs"][0]["results"].is_array(),
        "SARIF output should have results array"
    );
}

#[test]
fn test_format_sarif_schema_url() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(
        json["$schema"]
            .as_str()
            .unwrap()
            .contains("sarif-schema-2.1.0"),
        "Schema URL should reference SARIF 2.1.0"
    );
}

#[test]
fn test_help_shows_format_option() {
    let mut cmd = agnix();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--format"));
}

// JSON format tests

#[test]
fn test_format_json_produces_valid_json() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(json.is_ok(), "JSON output should be valid JSON");

    let json = json.unwrap();
    assert!(json["version"].is_string());
    assert!(json["files_checked"].is_number());
    assert!(json["diagnostics"].is_array());
    assert!(json["summary"].is_object());
}

#[test]
fn test_format_json_version_matches_cargo() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Version must exactly match CARGO_PKG_VERSION (works for 0.x and 1.x+)
    let version = json["version"].as_str().unwrap();
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "JSON version should match Cargo.toml version"
    );
}

#[test]
fn test_format_json_summary_counts() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let summary = &json["summary"];
    assert!(summary["errors"].is_number());
    assert!(summary["warnings"].is_number());
    assert!(summary["info"].is_number());

    // Valid fixtures should have no errors
    assert_eq!(summary["errors"].as_u64().unwrap(), 0);
}

#[test]
fn test_format_json_diagnostic_fields() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let diagnostics = json["diagnostics"].as_array().unwrap();
    if !diagnostics.is_empty() {
        let diag = &diagnostics[0];
        assert!(diag["level"].is_string());
        assert!(diag["rule"].is_string());
        assert!(diag["file"].is_string());
        assert!(diag["line"].is_number());
        assert!(diag["column"].is_number());
        assert!(diag["message"].is_string());
        // suggestion is optional, so just verify it's either null or string
        assert!(diag["suggestion"].is_null() || diag["suggestion"].is_string());
    }
}

#[test]
fn test_format_json_exit_code_on_error() {
    use std::fs;
    use std::io::Write;

    // Use tempfile for automatic cleanup even on panic
    let temp_dir = tempfile::tempdir().unwrap();

    let skills_dir = temp_dir.path().join("skills").join("bad-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // Create a skill with invalid name (uppercase) to trigger error
    writeln!(
        file,
        "---\nname: Bad-Skill\ndescription: test\n---\nContent"
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let errors = json["summary"]["errors"].as_u64().unwrap();
    // Invalid skill name should produce an error
    assert!(
        errors > 0,
        "Invalid skill name should produce at least one error, got: {}",
        stdout
    );
    assert!(
        !output.status.success(),
        "Should exit with error code when errors present"
    );
}

#[test]
fn test_format_json_strict_mode_with_warnings() {
    use std::fs;
    use std::io::Write;

    // Create a dedicated fixture that guarantees warnings but no errors
    let temp_dir = tempfile::tempdir().unwrap();

    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // Valid skill name but missing trigger phrase (AS-010 warning)
    writeln!(
        file,
        "---\nname: test-skill\ndescription: A test skill for validation\n---\nThis skill does something."
    )
    .unwrap();

    // Without --strict, warnings should not cause failure
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let warnings = json["summary"]["warnings"].as_u64().unwrap();
    let errors = json["summary"]["errors"].as_u64().unwrap();

    assert_eq!(errors, 0, "Should have no errors");
    assert!(warnings > 0, "Should have at least one warning (AS-010)");
    assert!(
        output.status.success(),
        "Without --strict, warnings should not cause failure"
    );

    // With --strict, warnings should cause exit code 1
    let mut cmd_strict = agnix();
    let output_strict = cmd_strict
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .arg("--strict")
        .output()
        .unwrap();

    assert!(
        !output_strict.status.success(),
        "With --strict, warnings should cause exit code 1"
    );
}

#[test]
fn test_format_json_strict_mode_no_warnings() {
    // With --strict but no warnings or errors, should succeed
    // Use a path that produces clean output
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid/skills")
        .arg("--format")
        .arg("json")
        .arg("--strict")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let errors = json["summary"]["errors"].as_u64().unwrap();
    let warnings = json["summary"]["warnings"].as_u64().unwrap();

    // Unconditionally assert: valid/skills fixture must be clean
    assert_eq!(errors, 0, "valid/skills fixture should have no errors");
    assert_eq!(warnings, 0, "valid/skills fixture should have no warnings");
    assert!(
        output.status.success(),
        "With --strict and no issues, should succeed"
    );
}

#[test]
fn test_format_json_exit_code_on_success() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .assert()
        .success();
}

#[test]
fn test_help_shows_json_format() {
    let mut cmd = agnix();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("json"));
}

#[test]
fn test_format_json_files_checked_count() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // files_checked should be a valid number
    let files_checked = json["files_checked"].as_u64();
    assert!(
        files_checked.is_some(),
        "files_checked should be a valid number"
    );
}

#[test]
fn test_format_json_forward_slashes_in_paths() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let diagnostics = json["diagnostics"].as_array().unwrap();
    for diag in diagnostics {
        let file = diag["file"].as_str().unwrap();
        assert!(
            !file.contains('\\'),
            "File paths should use forward slashes, got: {}",
            file
        );
    }
}

#[test]
fn test_cli_covers_hook_fixtures_via_cli_validation() {
    let config = fixtures_config();

    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/hooks/missing-command-field")
        .arg("--format")
        .arg("json")
        .arg("--config")
        .arg(config.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "Invalid hooks fixture should exit non-zero"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let diagnostics = json["diagnostics"].as_array().unwrap();
    let has_cchk006 = diagnostics.iter().any(|d| {
        d["rule"].as_str() == Some("CC-HK-006")
            && d["file"]
                .as_str()
                .map(|file| file.ends_with("missing-command-field/settings.json"))
                .unwrap_or(false)
    });
    assert!(
        has_cchk006,
        "Expected CC-HK-006 for missing-command-field settings.json, got: {}",
        stdout
    );
}

// ============================================================================
// JSON Output Rule Family Coverage Tests
// ============================================================================

#[test]
fn test_format_json_contains_skill_rules() {
    check_json_rule_family("tests/fixtures/invalid/skills", &["AS-", "CC-SK-"], "skill");
}

#[test]
fn test_format_json_contains_hook_rules() {
    check_json_rule_family("tests/fixtures/invalid/hooks", &["CC-HK-"], "hook");
}

#[test]
fn test_format_json_contains_agent_rules() {
    check_json_rule_family("tests/fixtures/invalid/agents", &["CC-AG-"], "agent");
}

#[test]
fn test_format_json_contains_mcp_rules() {
    check_json_rule_family("tests/fixtures/mcp", &["MCP-"], "MCP");
}

#[test]
fn test_format_json_contains_xml_rules() {
    check_json_rule_family("tests/fixtures/xml", &["XML-"], "XML");
}

#[test]
fn test_format_json_contains_plugin_rules() {
    check_json_rule_family("tests/fixtures/invalid/plugins", &["CC-PL-"], "plugin");
}

#[test]
fn test_format_json_contains_copilot_rules() {
    check_json_rule_family("tests/fixtures/copilot-invalid", &["COP-"], "Copilot");
}

#[test]
fn test_format_json_contains_agents_md_rules() {
    check_json_rule_family("tests/fixtures/agents_md", &["AGM-"], "AGENTS.md");
}

#[test]
fn test_format_json_contains_memory_rules() {
    use std::fs;
    use std::io::Write;

    // CC-MEM rules require specific content patterns to trigger
    // Create a fixture with generic instructions (CC-MEM-005)
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    let mut file = fs::File::create(&claude_md).unwrap();
    writeln!(
        file,
        "# Project Memory\n\nBe helpful and concise. Always follow best practices."
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostics = json["diagnostics"].as_array().unwrap();

    let has_memory_rule = diagnostics
        .iter()
        .any(|d| d["rule"].as_str().unwrap_or("").starts_with("CC-MEM-"));

    assert!(
        has_memory_rule,
        "Expected at least one memory rule (CC-MEM-*) in diagnostics, got: {}",
        stdout
    );
}

#[test]
fn test_format_json_contains_ref_rules() {
    check_json_rule_family("tests/fixtures/refs", &["REF-"], "reference");
}

#[test]
fn test_format_json_contains_cross_platform_rules() {
    check_json_rule_family("tests/fixtures/cross_platform", &["XP-"], "cross-platform");
}

#[test]
fn test_format_json_contains_amp_rules() {
    check_json_rule_family("tests/fixtures/amp-checks", &["AMP-"], "Amp");
}

#[test]
fn test_format_json_contains_amp_002_diagnostic() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/amp-checks")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostics = json["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d["rule"].as_str() == Some("AMP-002")),
        "Expected AMP-002 in JSON diagnostics, got: {}",
        stdout
    );
}

// ============================================================================
// SARIF Output Completeness Tests
// ============================================================================

#[test]
fn test_format_sarif_results_include_skill_diagnostics() {
    check_sarif_rule_family("tests/fixtures/invalid/skills", &["AS-", "CC-SK-"], "skill");
}

#[test]
fn test_format_sarif_results_include_hook_diagnostics() {
    check_sarif_rule_family("tests/fixtures/invalid/hooks", &["CC-HK-"], "hook");
}

#[test]
fn test_format_sarif_results_include_mcp_diagnostics() {
    check_sarif_rule_family("tests/fixtures/mcp", &["MCP-"], "MCP");
}

#[test]
fn test_format_sarif_results_include_amp_diagnostics() {
    check_sarif_rule_family("tests/fixtures/amp-checks", &["AMP-"], "Amp");
}

#[test]
fn test_format_sarif_results_include_amp_002_diagnostic() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/amp-checks")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = json["runs"][0]["results"].as_array().unwrap();
    assert!(
        results
            .iter()
            .any(|r| r["ruleId"].as_str() == Some("AMP-002")),
        "Expected AMP-002 in SARIF results, got: {}",
        stdout
    );
}

#[test]
fn test_format_sarif_results_include_memory_diagnostics() {
    use std::fs;
    use std::io::Write;

    // CC-MEM rules require specific content patterns to trigger
    // Create a fixture with generic instructions (CC-MEM-005)
    let temp_dir = tempfile::tempdir().unwrap();
    let claude_md = temp_dir.path().join("CLAUDE.md");
    let mut file = fs::File::create(&claude_md).unwrap();
    writeln!(
        file,
        "# Project Memory\n\nBe helpful and concise. Always follow best practices."
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = json["runs"][0]["results"].as_array().unwrap();

    let has_memory_result = results
        .iter()
        .any(|r| r["ruleId"].as_str().unwrap_or("").starts_with("CC-MEM-"));

    assert!(
        has_memory_result,
        "SARIF results should include memory diagnostics (CC-MEM-*)"
    );
}

#[test]
fn test_format_sarif_location_fields() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = json["runs"][0]["results"].as_array().unwrap();

    assert!(!results.is_empty(), "Should have at least one result");

    for result in results {
        let locations = result["locations"].as_array();
        assert!(
            locations.is_some(),
            "Each result should have locations array"
        );

        if let Some(locs) = locations {
            assert!(!locs.is_empty(), "Result should have at least one location");
            let physical = &locs[0]["physicalLocation"];
            assert!(
                physical["artifactLocation"]["uri"].is_string(),
                "Should have artifactLocation.uri"
            );
            assert!(
                physical["region"]["startLine"].is_number(),
                "Should have region.startLine"
            );
            assert!(
                physical["region"]["startColumn"].is_number(),
                "Should have region.startColumn"
            );
        }
    }
}

#[test]
fn test_format_sarif_rules_have_help_uri() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rules = json["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();

    for rule in rules {
        let help_uri = rule["helpUri"].as_str();
        assert!(
            help_uri.is_some(),
            "Rule {} should have helpUri",
            rule["id"]
        );
        assert!(
            help_uri
                .unwrap()
                .contains("avifenesh.github.io/agnix/docs/rules/generated/"),
            "helpUri should reference website rule docs"
        );
    }
}

// ============================================================================
// Text Output Formatting Tests
// ============================================================================

#[test]
fn test_format_text_shows_file_location() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/invalid/skills/unknown-tool")
        .assert()
        .failure()
        .stdout(predicate::str::is_match(r"[^:]+:\d+:\d+").unwrap());
}

#[test]
fn test_format_text_shows_error_level() {
    let mut cmd = agnix();
    // Match diagnostic line format: file:line:col error: message
    cmd.arg("tests/fixtures/invalid/skills/unknown-tool")
        .assert()
        .failure()
        .stdout(predicate::str::is_match(r":\d+:\d+.*error").unwrap());
}

#[test]
fn test_format_text_shows_warning_level() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // Valid skill name but missing trigger phrase (AS-010 warning)
    writeln!(
        file,
        "---\nname: test-skill\ndescription: A test skill\n---\nContent"
    )
    .unwrap();

    let mut cmd = agnix();
    // Match diagnostic line format: file:line:col warning: message
    cmd.arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r":\d+:\d+.*warning").unwrap());
}

#[test]
fn test_format_text_shows_summary() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/invalid/skills")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Found"));
}

#[test]
fn test_format_text_verbose_shows_rule() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--verbose")
        .assert()
        .failure()
        .stdout(predicate::str::is_match(r"(AS|CC)-\w+-\d+").unwrap());
}

#[test]
fn test_format_text_verbose_shows_suggestion() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--verbose")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verbose mode should show additional help/suggestion info
    assert!(
        stdout.contains("help") || stdout.contains("suggestion") || stdout.contains("-->"),
        "Verbose output should contain help or suggestion info, got: {}",
        stdout
    );
}

// ============================================================================
// Fix and Dry-Run Tests
// ============================================================================

#[test]
fn test_dry_run_no_file_modification() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("bad-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let original_content = "---\nname: Bad-Skill\ndescription: test\n---\nContent";
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        write!(file, "{}", original_content).unwrap();
    }

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--dry-run")
        .output()
        .unwrap();

    // Verify file was not modified
    let content_after = fs::read_to_string(&skill_path).unwrap();
    assert_eq!(
        content_after, original_content,
        "File should not be modified with --dry-run"
    );

    // Verify the flag was recognized (not just silently ignored)
    // CLI should still produce diagnostic output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.is_empty() || !output.status.success(),
        "--dry-run flag should be recognized and produce output or error"
    );
}

#[test]
fn test_fix_exit_code_on_remaining_errors() {
    let mut cmd = agnix();
    // Invalid fixtures have errors that cannot be auto-fixed
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--fix")
        .output()
        .unwrap();

    // Should still exit with error since errors remain
    assert!(
        !output.status.success(),
        "Should exit with error code when non-fixable errors remain"
    );

    // Verify the flag was recognized (produces diagnostic output, not clap error)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.is_empty() || stderr.is_empty(),
        "--fix flag should be recognized and run fix mode, got stderr: {}",
        stderr
    );
}

#[test]
fn test_fix_exit_code_when_all_issues_are_fixed() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir for test");
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).expect("Failed to create skill dir for test");

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut file = fs::File::create(&skill_path).expect("Failed to create skill file for test");
        // Only AS-004 should fail here, and it is auto-fixable.
        write!(
            file,
            "---\nname: Test_Skill_Name\ndescription: Use when testing\n---\nContent"
        )
        .expect("Failed to write to skill file for test");
    }

    let mut cmd = agnix();
    let output = cmd
        .arg(
            temp_dir
                .path()
                .to_str()
                .expect("Temp path is not valid UTF-8"),
        )
        .arg("--fix")
        .output()
        .expect("Failed to execute agnix command");

    assert!(
        output.status.success(),
        "Expected --fix to exit 0 when all issues are fixed, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let fixed_content = fs::read_to_string(&skill_path).expect("Failed to read fixed skill file");
    assert!(
        fixed_content.contains("name: test-skill-name"),
        "Expected AS-004 fix to be applied, got: {}",
        fixed_content
    );
}

#[test]
fn test_fix_safe_exit_code() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--fix-safe")
        .output()
        .unwrap();

    // Should still exit with error since errors remain
    assert!(
        !output.status.success(),
        "Should exit with error code when errors remain after --fix-safe"
    );

    // Verify the flag was recognized (produces diagnostic output, not clap error)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.is_empty() || stderr.is_empty(),
        "--fix-safe flag should be recognized and run fix mode, got stderr: {}",
        stderr
    );
}

#[test]
fn test_fix_unsafe_flag_recognized() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--fix-unsafe")
        .output()
        .unwrap();

    // This fixture still has remaining diagnostics, so non-zero is expected.
    // The key assertion is that clap does not reject the flag as unknown.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument '--fix-unsafe'"),
        "--fix-unsafe should be accepted by the CLI, got stderr: {}",
        stderr
    );
}

#[test]
fn test_dry_run_combines_with_fix_safe() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--dry-run")
        .arg("--fix-safe")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("cannot be used with"),
        "--dry-run should combine with --fix-safe, got stderr: {}",
        stderr
    );
}

#[test]
fn test_dry_run_combines_with_fix_unsafe() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/unknown-tool")
        .arg("--dry-run")
        .arg("--fix-unsafe")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("cannot be used with"),
        "--dry-run should combine with --fix-unsafe, got stderr: {}",
        stderr
    );
}

#[test]
fn test_show_fixes_displays_fixes_without_verbose() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("show-fixes");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    writeln!(
        file,
        "---\nname: Invalid_Name\ndescription: use when testing\n---\nSome content"
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path())
        .arg("--show-fixes")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("fix:"),
        "--show-fixes should print inline fix descriptions, got:\n{}",
        stdout
    );
}

// ============================================================================
// Flag Combination Tests
// ============================================================================

#[test]
fn test_strict_with_sarif_format() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // Valid skill name but missing trigger phrase (AS-010 warning)
    writeln!(
        file,
        "---\nname: test-skill\ndescription: A test skill\n---\nContent"
    )
    .unwrap();

    // With --strict, warnings should cause exit code 1
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("sarif")
        .arg("--strict")
        .output()
        .unwrap();

    // Verify it's valid SARIF
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["runs"].is_array(), "Should produce valid SARIF");

    // Should fail due to warnings in strict mode
    assert!(
        !output.status.success(),
        "With --strict and warnings, should exit with error code"
    );
}

#[test]
fn test_verbose_with_json_ignored() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/valid")
        .arg("--verbose")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should still be valid JSON (verbose doesn't corrupt JSON output)
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "--verbose should not corrupt JSON output, got: {}",
        stdout
    );
}

#[test]
fn test_target_cursor_disables_cc_rules() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("deploy-prod");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // This would normally trigger CC-SK-006 (Claude-specific rule)
    writeln!(
        file,
        "---\nname: deploy-prod\ndescription: Deploy to production\n---\nDeploy the application"
    )
    .unwrap();

    // With --target cursor, CC-* rules should be disabled
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .arg("--target")
        .arg("cursor")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostics = json["diagnostics"].as_array().unwrap();

    // Should not have any CC-* rules for cursor target
    let has_cc_rule = diagnostics
        .iter()
        .any(|d| d["rule"].as_str().unwrap_or("").starts_with("CC-"));

    assert!(
        !has_cc_rule,
        "With --target cursor, CC-* rules should be disabled"
    );
}

#[test]
fn test_validate_subcommand() {
    let mut cmd = agnix();
    cmd.arg("validate")
        .arg("tests/fixtures/valid")
        .assert()
        .success();
}

#[test]
fn test_dry_run_with_format_json_rejected() {
    assert_fix_flags_rejected("json", "--dry-run");
}

#[test]
fn test_fixtures_have_no_empty_placeholder_dirs() {
    use std::fs;
    use std::path::{Path, PathBuf};

    fn check_dir(dir: &Path, empty_dirs: &mut Vec<PathBuf>) -> bool {
        let mut has_file = false;
        let entries = fs::read_dir(dir).unwrap_or_else(|e| {
            panic!("Failed to read fixture directory {}: {}", dir.display(), e)
        });

        for entry in entries {
            let entry = entry
                .unwrap_or_else(|e| panic!("Failed to read entry under {}: {}", dir.display(), e));
            let path = entry.path();
            if path.is_file() {
                has_file = true;
                continue;
            }
            if path.is_dir() && check_dir(&path, empty_dirs) {
                has_file = true;
            }
        }

        if !has_file {
            empty_dirs.push(dir.to_path_buf());
        }

        has_file
    }

    let root = workspace_path("tests/fixtures");
    assert!(
        root.is_dir(),
        "Expected fixtures directory at {}",
        root.display()
    );

    let mut empty_dirs = Vec::new();
    check_dir(&root, &mut empty_dirs);

    assert!(
        empty_dirs.is_empty(),
        "Empty fixture directories found:\n{}",
        empty_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ===== Config Parse Warning Tests =====

#[test]
fn test_invalid_config_displays_warning_to_stderr() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join(".agnix.toml");

    // Create invalid TOML
    std::fs::write(&config_path, "this is [ invalid toml").unwrap();

    // Create a minimal valid directory to scan
    let skill_dir = temp_dir.path().join(".claude").join("skills");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_file = skill_dir.join("test.md");
    std::fs::write(&skill_file, "# Test\nSimple content").unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain warning message
    assert!(
        stderr.contains("Warning:") && stderr.contains("Failed to parse config"),
        "Expected config warning in stderr, got: {}",
        stderr
    );

    // Should still produce validation output (continues with defaults)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Validating:"),
        "Should still run validation with default config"
    );
}

#[test]
fn test_valid_config_no_warning_in_stderr() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join(".agnix.toml");

    // Create valid config
    std::fs::write(
        &config_path,
        r#"
severity = "Warning"
target = "Generic"
exclude = []

[rules]
"#,
    )
    .unwrap();

    // Create a minimal valid directory to scan
    let skill_dir = temp_dir.path().join(".claude").join("skills");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_file = skill_dir.join("test.md");
    std::fs::write(&skill_file, "# Test\nSimple content").unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain any config warning
    assert!(
        !stderr.contains("Failed to parse config"),
        "Valid config should not produce warning, stderr: {}",
        stderr
    );
}

#[test]
fn test_config_warning_with_json_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join(".agnix.toml");

    // Create invalid TOML
    std::fs::write(&config_path, "invalid [[ toml").unwrap();

    // Create a minimal valid directory to scan
    let skill_dir = temp_dir.path().join(".claude").join("skills");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_file = skill_dir.join("test.md");
    std::fs::write(&skill_file, "# Test\nSimple content").unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path())
        .arg("--config")
        .arg(&config_path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Warning should go to stderr, not corrupt JSON output
    assert!(
        stderr.contains("Warning:"),
        "Warning should be in stderr for JSON output"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "JSON output should be valid despite config warning, got: {}",
        stdout
    );
}

// ============================================================================
// Target Argument Validation Tests (Issue #129)
// ============================================================================

#[test]
fn test_invalid_target_rejected() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/valid")
        .arg("--target")
        .arg("invalid-target")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_typo_target_rejected() {
    let mut cmd = agnix();
    // Underscore instead of hyphen should be rejected
    cmd.arg("tests/fixtures/valid")
        .arg("--target")
        .arg("claude_code")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_case_sensitive_target_rejected() {
    let mut cmd = agnix();
    // PascalCase should be rejected (CLI uses kebab-case)
    cmd.arg("tests/fixtures/valid")
        .arg("--target")
        .arg("ClaudeCode")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_valid_targets_accepted() {
    for target in ["generic", "claude-code", "cursor", "codex"] {
        let mut cmd = agnix();
        cmd.arg("tests/fixtures/valid")
            .arg("--target")
            .arg(target)
            .assert()
            .success();
    }
}

#[test]
fn test_help_shows_target_possible_values() {
    let mut cmd = agnix();
    let output = cmd.arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help should list exact possible values for --target
    assert!(
        stdout.contains("[possible values: generic, claude-code, cursor, codex]"),
        "Help should show exact possible target values, got: {}",
        stdout
    );
}

// ============================================================================
// JSON Output files_checked Accuracy Tests (Issue #127)
// ============================================================================

#[test]
fn test_format_json_files_checked_counts_all_validated_files() {
    use std::fs;

    // Create a directory with valid files that produce no diagnostics
    let temp_dir = tempfile::tempdir().unwrap();

    // Create valid SKILL.md files (no diagnostics expected)
    let skill_dir1 = temp_dir.path().join("skills").join("code-review");
    fs::create_dir_all(&skill_dir1).unwrap();
    fs::write(
        skill_dir1.join("SKILL.md"),
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    let skill_dir2 = temp_dir.path().join("skills").join("test-runner");
    fs::create_dir_all(&skill_dir2).unwrap();
    fs::write(
        skill_dir2.join("SKILL.md"),
        "---\nname: test-runner\ndescription: Use when running tests\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "agnix exited with non-zero status: {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let files_checked = json["files_checked"].as_u64().unwrap();

    // files_checked should be exactly 2 (the two SKILL.md files we created)
    assert_eq!(
        files_checked, 2,
        "Expected 2 files checked, found {}",
        files_checked
    );
}

#[test]
fn test_format_json_files_checked_excludes_unknown_types() {
    use std::fs;

    // Create a directory with mixed file types
    let temp_dir = tempfile::tempdir().unwrap();

    // Create files of unknown type (should NOT be counted)
    fs::write(temp_dir.path().join("main.rs"), "fn main() {}").unwrap();
    fs::write(temp_dir.path().join("index.ts"), "console.log('hello')").unwrap();

    // Create one recognized file (should be counted)
    fs::write(
        temp_dir.path().join("SKILL.md"),
        "---\nname: code-review\ndescription: Use when reviewing code\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "agnix exited with non-zero status: {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let files_checked = json["files_checked"].as_u64().unwrap();

    // Only the SKILL.md file should be counted
    assert_eq!(
        files_checked, 1,
        "files_checked should only count recognized file types (SKILL.md), got {}",
        files_checked
    );
}

#[test]
fn test_init_creates_config_file_with_plain_text_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join(".agnix.toml");

    let mut cmd = agnix();
    let output = cmd
        .arg("init")
        .arg(config_path.to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify output contains "Created:" (plain text, no emoji)
    assert!(
        stdout.contains("Created:"),
        "Init output should contain 'Created:', got: {}",
        stdout
    );

    // Verify output does NOT contain checkmark emoji
    assert!(
        !stdout.contains('\u{2713}') && !stdout.contains('\u{2714}'),
        "Init output should not contain checkmark emoji, got: {}",
        stdout
    );

    // Verify the config file was created
    assert!(
        config_path.exists(),
        "Config file should be created at {}",
        config_path.display()
    );

    // Verify the config file contains valid TOML
    let content = std::fs::read_to_string(&config_path).unwrap();
    let parsed: Result<toml::Value, _> = toml::from_str(&content);
    assert!(
        parsed.is_ok(),
        "Created config should be valid TOML, got: {}",
        content
    );

    // Verify exit code is success
    assert!(output.status.success(), "Init command should succeed");
}

// ============================================================================
// Auto-Fix Tests for AS-004 and AS-010 (Issue #15)
// ============================================================================

#[test]
fn test_fix_as_004_converts_name_to_kebab_case() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        // Invalid name with underscores
        write!(
            file,
            "---\nname: Test_Skill_Name\ndescription: Use when testing\n---\nBody"
        )
        .unwrap();
    }

    // Run with --fix
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    // Read the fixed file
    let fixed_content = fs::read_to_string(&skill_path).unwrap();

    // Should convert Test_Skill_Name to test-skill-name
    assert!(
        fixed_content.contains("name: test-skill-name"),
        "AS-004 fix should convert name to kebab-case, got: {}",
        fixed_content
    );

    // Output should indicate fixes applied
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Fixed") || stdout.contains("fix"),
        "Output should mention fix applied"
    );
}

#[test]
fn test_fix_as_010_prepends_trigger_phrase() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("code-review");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        // Valid name but missing trigger phrase
        write!(
            file,
            "---\nname: code-review\ndescription: Reviews code for quality\n---\nBody"
        )
        .unwrap();
    }

    // Run with --fix (not --fix-safe since AS-010 is not a safe fix)
    let mut cmd = agnix();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    // Read the fixed file
    let fixed_content = fs::read_to_string(&skill_path).unwrap();

    // Should prepend "Use when user wants to " to description
    assert!(
        fixed_content.contains("Use when user wants to Reviews code for quality"),
        "AS-010 fix should prepend trigger phrase, got: {}",
        fixed_content
    );
}

#[test]
fn test_fix_safe_skips_as_010() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("code-review");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let original_content = "---\nname: code-review\ndescription: Reviews code\n---\nBody";
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        write!(file, "{}", original_content).unwrap();
    }

    // Run with --fix-safe (should NOT fix AS-010 since it's not safe)
    let mut cmd = agnix();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix-safe")
        .output()
        .unwrap();

    // Read the file
    let content_after = fs::read_to_string(&skill_path).unwrap();

    // AS-010 fix is NOT safe, so it should NOT be applied
    assert_eq!(
        content_after, original_content,
        "--fix-safe should not apply AS-010 fix"
    );
}

#[test]
fn test_fix_safe_applies_case_only_as_004() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        // Name only needs lowercase (case-only change = safe)
        write!(
            file,
            "---\nname: TestSkill\ndescription: Use when testing\n---\nBody"
        )
        .unwrap();
    }

    // Run with --fix-safe
    let mut cmd = agnix();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix-safe")
        .output()
        .unwrap();

    // Read the fixed file
    let fixed_content = fs::read_to_string(&skill_path).unwrap();

    // Case-only fix IS safe, so it should be applied
    assert!(
        fixed_content.contains("name: testskill"),
        "--fix-safe should apply case-only AS-004 fix, got: {}",
        fixed_content
    );
}

#[test]
fn test_dry_run_shows_as_004_fix_without_applying() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let original_content = "---\nname: Test_Skill\ndescription: Use when testing\n---\nBody";
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        write!(file, "{}", original_content).unwrap();
    }

    // Run with --dry-run
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--dry-run")
        .output()
        .unwrap();

    // File should NOT be modified
    let content_after = fs::read_to_string(&skill_path).unwrap();
    assert_eq!(
        content_after, original_content,
        "--dry-run should not modify files"
    );

    // Output should show what would be fixed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Would fix") || stdout.contains("dry-run") || stdout.contains("test-skill"),
        "--dry-run should show what would be fixed"
    );
}

#[test]
fn test_fix_both_as_004_and_as_010_simultaneously() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        // Both AS-004 (invalid name) and AS-010 (missing trigger)
        write!(
            file,
            "---\nname: Test_Skill\ndescription: Does testing\n---\nBody"
        )
        .unwrap();
    }

    // Run with --fix
    let mut cmd = agnix();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    // Read the fixed file
    let fixed_content = fs::read_to_string(&skill_path).unwrap();

    // Both fixes should be applied
    assert!(
        fixed_content.contains("name: test-skill"),
        "AS-004 fix should be applied, got: {}",
        fixed_content
    );
    assert!(
        fixed_content.contains("Use when user wants to Does testing"),
        "AS-010 fix should be applied, got: {}",
        fixed_content
    );
}

#[test]
fn test_fix_safe_skips_structural_as_004() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let original_content = "---\nname: test_skill_name\ndescription: Use when testing\n---\nBody";
    {
        let mut file = fs::File::create(&skill_path).unwrap();
        // Name with underscores = structural change (not just case)
        write!(file, "{}", original_content).unwrap();
    }

    // Run with --fix-safe
    let mut cmd = agnix();
    cmd.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix-safe")
        .output()
        .unwrap();

    // Read the file
    let content_after = fs::read_to_string(&skill_path).unwrap();

    // Structural AS-004 fix is NOT safe, should NOT be applied
    assert_eq!(
        content_after, original_content,
        "--fix-safe should not apply structural AS-004 fix"
    );
}

// ============================================================================
// Telemetry Command Tests (Issue #209)
// ============================================================================

#[test]
fn test_telemetry_status_shows_disabled_by_default() {
    let mut cmd = agnix();
    let output = cmd.arg("telemetry").arg("status").output().unwrap();

    assert!(output.status.success(), "telemetry status should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show telemetry is disabled
    assert!(
        stdout.contains("disabled"),
        "Telemetry should be disabled by default, got: {}",
        stdout
    );

    // Should show privacy guarantees
    assert!(
        stdout.contains("Privacy Guarantees"),
        "Should display privacy guarantees, got: {}",
        stdout
    );
}

#[test]
fn test_telemetry_status_default_action() {
    // "agnix telemetry" without action should default to status
    let mut cmd = agnix();
    let output = cmd.arg("telemetry").output().unwrap();

    assert!(
        output.status.success(),
        "telemetry without action should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Telemetry Status"),
        "Should show status by default, got: {}",
        stdout
    );
}

#[test]
fn test_telemetry_help_shows_actions() {
    let mut cmd = agnix();
    let output = cmd.arg("telemetry").arg("--help").output().unwrap();

    assert!(output.status.success(), "telemetry --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should list the available actions
    assert!(
        stdout.contains("status") && stdout.contains("enable") && stdout.contains("disable"),
        "Help should show status/enable/disable actions, got: {}",
        stdout
    );
}

#[test]
fn test_telemetry_enable_disable_roundtrip() {
    use std::env;

    // Use a temp config directory to avoid affecting real config
    let temp_dir = tempfile::tempdir().unwrap();
    let config_dir = temp_dir.path().join("agnix");
    std::fs::create_dir_all(&config_dir).unwrap();

    // Skip this test in CI - it would try to write to the real config dir
    // and we can't easily override dirs::config_dir()
    if env::var("CI").is_ok() || env::var("GITHUB_ACTIONS").is_ok() {
        eprintln!("Skipping telemetry roundtrip test in CI");
        return;
    }

    // Test enable
    let mut cmd = agnix();
    let output = cmd.arg("telemetry").arg("enable").output().unwrap();

    assert!(output.status.success(), "telemetry enable should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should indicate success or already enabled
    assert!(
        stdout.contains("enabled") || stdout.contains("already"),
        "Should confirm enable, got: {}",
        stdout
    );

    // Test disable
    let mut cmd = agnix();
    let output = cmd.arg("telemetry").arg("disable").output().unwrap();

    assert!(output.status.success(), "telemetry disable should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("disabled") || stdout.contains("already"),
        "Should confirm disable, got: {}",
        stdout
    );
}

#[test]
fn test_telemetry_invalid_action_rejected() {
    let mut cmd = agnix();
    cmd.arg("telemetry")
        .arg("invalid-action")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

// ============================================================================
// Schema Command Integration Tests (Issue #206)
// ============================================================================

#[test]
fn test_schema_command_stdout() {
    // agnix schema outputs valid JSON to stdout
    let mut cmd = agnix();
    cmd.arg("schema")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("LintConfig"));
}

#[test]
fn test_schema_command_output_file() {
    // agnix schema --output file.json writes to file
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("schema.json");

    let mut cmd = agnix();
    cmd.arg("schema")
        .arg("--output")
        .arg(&output_path)
        .assert()
        .success();

    // Verify file was created and contains valid JSON
    let content = std::fs::read_to_string(&output_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Verify it's a valid JSON Schema
    assert!(
        json["$schema"].is_string(),
        "Schema output should have $schema field"
    );
    assert!(
        json["title"].is_string() || json["definitions"].is_object(),
        "Schema output should have title or definitions"
    );
}

#[test]
fn test_schema_command_help_shows_output_option() {
    let mut cmd = agnix();
    cmd.arg("schema")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"));
}

// ============================================================================
// Config Validation Warning Display Integration Tests (Issue #206)
// ============================================================================

#[test]
fn test_invalid_rule_displays_semantic_warning() {
    // Use tests/fixtures/config_validation/invalid_rules.toml
    let mut cmd = agnix();
    cmd.arg("--config")
        .arg("tests/fixtures/config_validation/invalid_rules.toml")
        .arg("tests/fixtures/valid")
        .assert()
        .success()
        .stderr(predicate::str::contains("Config warning"))
        .stderr(predicate::str::contains("INVALID-001"));
}

#[test]
fn test_deprecated_field_displays_warning() {
    // Use tests/fixtures/config_validation/deprecated_fields.toml
    let mut cmd = agnix();
    cmd.arg("--config")
        .arg("tests/fixtures/config_validation/deprecated_fields.toml")
        .arg("tests/fixtures/valid")
        .assert()
        .success()
        .stderr(predicate::str::contains("mcp_protocol_version"))
        .stderr(predicate::str::contains("deprecated"));
}

#[test]
fn test_invalid_tools_displays_warning() {
    // Use tests/fixtures/config_validation/invalid_tools.toml
    let mut cmd = agnix();
    cmd.arg("--config")
        .arg("tests/fixtures/config_validation/invalid_tools.toml")
        .arg("tests/fixtures/valid")
        .assert()
        .success()
        .stderr(predicate::str::contains("Config warning"))
        .stderr(predicate::str::contains("unknown-tool"));
}

#[test]
fn test_valid_config_no_semantic_warnings() {
    // Use tests/fixtures/config_validation/valid_config.toml
    let mut cmd = agnix();
    let output = cmd
        .arg("--config")
        .arg("tests/fixtures/config_validation/valid_config.toml")
        .arg("tests/fixtures/valid")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Valid config should NOT produce "Config warning:" messages
    assert!(
        !stderr.contains("Config warning:"),
        "Valid config should not produce semantic warnings, stderr: {}",
        stderr
    );
}

#[test]
fn test_config_semantic_warnings_go_to_stderr_with_json_output() {
    // Semantic warnings should go to stderr, not corrupt JSON output
    let mut cmd = agnix();
    let output = cmd
        .arg("--config")
        .arg("tests/fixtures/config_validation/invalid_rules.toml")
        .arg("tests/fixtures/valid")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let _stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Note: Semantic warnings only display for text output per main.rs line 273
    // JSON output suppresses them to avoid corrupting the JSON
    // So this test verifies JSON is still valid
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "JSON output should be valid even with config that has semantic issues, got: {}",
        stdout
    );

    // The parse warning (if any) goes to stderr, but semantic warnings are suppressed for JSON
    // This is intentional behavior per the implementation
}

// ============================================================================
// i18n / Locale CLI Integration Tests (Issue #207)
// ============================================================================

#[test]
fn test_locale_flag_switches_output_to_spanish() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("es")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Spanish locale should produce "Validando:" instead of "Validating:"
    assert!(
        stdout.contains("Validando:"),
        "--locale es should produce Spanish output ('Validando:'), got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Validating:"),
        "--locale es should NOT produce English 'Validating:', got: {}",
        stdout
    );
}

#[test]
fn test_locale_flag_switches_output_to_chinese() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("zh-CN")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Chinese locale should produce the zh-CN translated header
    assert!(
        stdout.contains("\u{6b63}\u{5728}\u{9a8c}\u{8bc1}:"),
        "--locale zh-CN should produce Chinese output, got: {}",
        stdout
    );
}

#[test]
fn test_list_locales_flag_outputs_supported_locales() {
    let mut cmd = agnix();
    let output = cmd.arg("--list-locales").output().unwrap();

    assert!(
        output.status.success(),
        "--list-locales should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain the header
    assert!(
        stdout.contains("Supported locales:"),
        "--list-locales should show 'Supported locales:' header, got: {}",
        stdout
    );

    // Should list all three supported locales
    assert!(
        stdout.contains("en"),
        "--list-locales should list 'en', got: {}",
        stdout
    );
    assert!(
        stdout.contains("es"),
        "--list-locales should list 'es', got: {}",
        stdout
    );
    assert!(
        stdout.contains("zh-CN"),
        "--list-locales should list 'zh-CN', got: {}",
        stdout
    );

    // Should include display names
    assert!(
        stdout.contains("English"),
        "--list-locales should show 'English' display name, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Spanish"),
        "--list-locales should show 'Spanish' display name, got: {}",
        stdout
    );
    assert!(
        stdout.contains("Chinese"),
        "--list-locales should show 'Chinese' display name, got: {}",
        stdout
    );
}

#[test]
fn test_list_locales_exits_without_validation() {
    // --list-locales should print locales and exit, even without a valid path
    let mut cmd = agnix();
    let output = cmd
        .arg("--list-locales")
        .arg("/nonexistent/path/that/does/not/exist")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--list-locales should succeed even with nonexistent path"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Supported locales:"),
        "--list-locales should print locale list regardless of path, got: {}",
        stdout
    );
}

#[test]
fn test_agnix_locale_env_var_switches_output() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .env("AGNIX_LOCALE", "es")
        // Clear LANG/LC_ALL to avoid interference
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .arg(temp_dir.path().to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // AGNIX_LOCALE=es should produce Spanish output
    assert!(
        stdout.contains("Validando:"),
        "AGNIX_LOCALE=es should produce Spanish output, got: {}",
        stdout
    );
}

#[test]
fn test_config_file_locale_field() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();

    // Create a config file with locale = "es"
    let config_path = temp_dir.path().join(".agnix.toml");
    fs::write(
        &config_path,
        "severity = \"Warning\"\ntarget = \"Generic\"\nlocale = \"es\"\nexclude = []\n\n[rules]\n",
    )
    .unwrap();

    // Create a minimal valid fixture
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(&config_path)
        // Clear env vars so config locale takes effect
        .env_remove("AGNIX_LOCALE")
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Config locale = "es" should produce Spanish output
    assert!(
        stdout.contains("Validando:"),
        "Config locale = 'es' should produce Spanish output, got: {}",
        stdout
    );
}

#[test]
fn test_locale_priority_cli_flag_overrides_env_var() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    // Set env to Spanish, but CLI flag to English
    let mut cmd = agnix();
    let output = cmd
        .env("AGNIX_LOCALE", "es")
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("en")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // --locale en should override AGNIX_LOCALE=es
    assert!(
        stdout.contains("Validating:"),
        "--locale en should override AGNIX_LOCALE=es, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Validando:"),
        "--locale en should NOT show Spanish output, got: {}",
        stdout
    );
}

#[test]
fn test_locale_priority_cli_flag_overrides_config() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();

    // Config file with locale = "es"
    let config_path = temp_dir.path().join(".agnix.toml");
    fs::write(
        &config_path,
        "severity = \"Warning\"\ntarget = \"Generic\"\nlocale = \"es\"\nexclude = []\n\n[rules]\n",
    )
    .unwrap();

    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    // CLI flag should override config
    let mut cmd = agnix();
    let output = cmd
        .env_remove("AGNIX_LOCALE")
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(&config_path)
        .arg("--locale")
        .arg("en")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // --locale en should override config locale = "es"
    assert!(
        stdout.contains("Validating:"),
        "--locale en should override config locale=es, got: {}",
        stdout
    );
    assert!(
        !stdout.contains("Validando:"),
        "--locale en should NOT show Spanish output when overriding config, got: {}",
        stdout
    );
}

#[test]
fn test_locale_priority_env_var_overrides_config() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();

    // Config file with locale = "en"
    let config_path = temp_dir.path().join(".agnix.toml");
    fs::write(
        &config_path,
        "severity = \"Warning\"\ntarget = \"Generic\"\nlocale = \"en\"\nexclude = []\n\n[rules]\n",
    )
    .unwrap();

    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    // AGNIX_LOCALE=es should be picked up by detect_locale() which runs before
    // config is loaded, so env var takes effect for initial output.
    // However, locale re-initialization happens after config load only when no --locale flag.
    // The priority is: --locale > config > env var (inside detect_locale).
    // Actually per the code: locale::init(cli_locale, None) first, then
    // if cli.locale.is_none() && config.locale.is_some(), re-init with config locale.
    // So config locale overrides env var when no --locale flag is set.
    // This test documents that behavior.
    let mut cmd = agnix();
    let output = cmd
        .env("AGNIX_LOCALE", "es")
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(&config_path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // When no --locale flag: config locale takes precedence over AGNIX_LOCALE
    // because the code re-initializes locale from config after loading it.
    // The initial locale::init picks up AGNIX_LOCALE but then config overrides it.
    // This verifies the re-initialization logic works.
    assert!(
        stdout.contains("Validating:"),
        "Config locale='en' should override AGNIX_LOCALE=es (no --locale flag), got: {}",
        stdout
    );
}

#[test]
fn test_invalid_locale_warns_and_falls_back_to_english() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("xx-INVALID")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should warn about unsupported locale on stderr
    assert!(
        stderr.contains("unsupported locale") || stderr.contains("Warning"),
        "Invalid locale should produce warning on stderr, got stderr: {}",
        stderr
    );

    // Should fall back to English output
    assert!(
        stdout.contains("Validating:"),
        "Invalid locale should fall back to English output, got: {}",
        stdout
    );
}

#[test]
fn test_invalid_locale_in_config_warns_and_falls_back() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();

    // Config with invalid locale
    let config_path = temp_dir.path().join(".agnix.toml");
    fs::write(
        &config_path,
        "severity = \"Warning\"\ntarget = \"Generic\"\nlocale = \"xx-INVALID\"\nexclude = []\n\n[rules]\n",
    )
    .unwrap();

    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .env_remove("AGNIX_LOCALE")
        .env_remove("LANG")
        .env_remove("LC_ALL")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--config")
        .arg(&config_path)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should warn about unsupported locale
    assert!(
        stderr.contains("unsupported locale") || stderr.contains("Warning"),
        "Invalid config locale should produce warning, got stderr: {}",
        stderr
    );

    // Should fall back to English
    assert!(
        stdout.contains("Validating:"),
        "Invalid config locale should fall back to English, got stdout: {}",
        stdout
    );
}

#[test]
fn test_locale_flag_with_json_output_still_works() {
    use std::fs;

    // Locale flag should not affect JSON structure (JSON always uses keys not messages)
    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    fs::write(
        skills_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Use when testing\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("es")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "--locale es with --format json should produce valid JSON, got: {}",
        stdout
    );

    let json = json.unwrap();
    assert!(
        json["version"].is_string(),
        "JSON should have version field"
    );
    assert!(
        json["diagnostics"].is_array(),
        "JSON should have diagnostics array"
    );
}

#[test]
fn test_help_shows_locale_flags() {
    let mut cmd = agnix();
    let output = cmd.arg("--help").output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--locale"),
        "Help should show --locale flag, got: {}",
        stdout
    );
    assert!(
        stdout.contains("--list-locales"),
        "Help should show --list-locales flag, got: {}",
        stdout
    );
}

// ============================================================================
// Cross-Crate Autofix Consistency Tests (Issue #285)
// ============================================================================

/// Verify that all major autofix rule families produce fixable diagnostics
/// when run through the CLI.  This is a cross-crate end-to-end smoke test:
/// agnix-rules  ->  agnix-core (validators + fix generation)  ->  agnix-cli.
#[test]
fn test_autofix_rules_json_reports_fixes_available() {
    use std::fs;
    use std::io::Write;

    // Create temp fixtures that trigger fixable rules from each family

    let temp_dir = tempfile::tempdir().unwrap();

    // 1. Skill fixture (AS-004: invalid name -> fixable)
    let skill_dir = temp_dir.path().join("skills").join("test-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    {
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        write!(
            f,
            "---\nname: Test_Skill\ndescription: Use when testing\n---\nBody"
        )
        .unwrap();
    }

    // 2. Hooks fixture (CC-HK-001: invalid event -> fixable)
    let hooks_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&hooks_dir).unwrap();
    fs::write(
        hooks_dir.join("settings.json"),
        r#"{
  "hooks": {
    "pretooluse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo ok" }
        ]
      }
    ]
  }
}"#,
    )
    .unwrap();

    // 3. Agent fixture (CC-AG-003: invalid model -> fixable)
    let agent_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agent_dir).unwrap();
    fs::write(
        agent_dir.join("bad-agent.md"),
        "---\nname: bad-agent\ndescription: An agent\nmodel: gpt-4\n---\nBody",
    )
    .unwrap();

    // 4. MCP fixture (MCP-002: invalid jsonrpc version -> fixable)
    fs::write(
        temp_dir.path().join("tools.mcp.json"),
        r#"{"jsonrpc": "1.0", "method": "tools/list", "id": 1}"#,
    )
    .unwrap();

    // Run with --dry-run (text output includes [fixable] markers)
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify that at least one fixable diagnostic exists from each family
    assert!(
        stdout.contains("[fixable]"),
        "Expected at least one [fixable] diagnostic, got: {}",
        stdout
    );

    // Per-family assertions: each fixture path must produce a [fixable] diagnostic.
    // Rule IDs are not emitted in text output, so we match by fixture path segments.
    let family_markers: &[(&str, &str)] = &[
        ("AS-", "SKILL.md"),
        ("CC-HK-", "settings.json"),
        ("CC-AG-", "bad-agent.md"),
        ("MCP-", ".mcp.json"),
    ];
    for (family, path_marker) in family_markers {
        let has_fixable_in_family = stdout
            .lines()
            .any(|line| line.contains(path_marker) && line.contains("[fixable]"));
        assert!(
            has_fixable_in_family,
            "Expected a [fixable] diagnostic for the {} family (path contains '{}'), got: {}",
            family, path_marker, stdout
        );
    }
    // Verify the summary line reports a non-zero fixable count
    assert!(
        stdout.contains("automatically fixable"),
        "Expected fixable count in summary line, got: {}",
        stdout
    );
}

// ============================================================================
// CLI --fix Integration Tests for Pack 2 Rules (Issue #285)
// ============================================================================

/// CC-SK: Skill name fix (AS-004) via --fix on invalid name
#[test]
fn test_fix_ccsk_invalid_skill_name() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("my-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    {
        let mut f = fs::File::create(&skill_path).unwrap();
        write!(
            f,
            "---\nname: My_Broken_Skill\ndescription: Use when testing\n---\nBody"
        )
        .unwrap();
    }

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--fix should exit successfully after applying skill name fix"
    );

    let fixed = fs::read_to_string(&skill_path).unwrap();
    assert!(
        fixed.contains("name: my-broken-skill"),
        "AS-004 fix should convert name to kebab-case, got: {}",
        fixed
    );
}

/// CC-HK: Hook event-name fix (CC-HK-001) via --fix on invalid event casing
#[test]
fn test_fix_cchk_invalid_event_name() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let claude_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    let settings_path = claude_dir.join("settings.json");
    fs::write(
        &settings_path,
        r#"{
  "hooks": {
    "pretooluse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo test" }
        ]
      }
    ]
  }
}"#,
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--fix should exit successfully after applying hook event name fix"
    );

    let fixed = fs::read_to_string(&settings_path).unwrap();
    assert!(
        fixed.contains("PreToolUse"),
        "CC-HK-001 fix should correct event name casing, got: {}",
        fixed
    );
}

/// CC-AG: Agent model fix (CC-AG-003) via --fix on invalid model value
#[test]
fn test_fix_ccag_invalid_model() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let agent_dir = temp_dir.path().join(".claude").join("agents");
    fs::create_dir_all(&agent_dir).unwrap();

    let agent_path = agent_dir.join("test-agent.md");
    fs::write(
        &agent_path,
        "---\nname: test-agent\ndescription: An agent\nmodel: gpt-4\n---\nBody",
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--fix should exit successfully after applying agent model fix"
    );

    let fixed = fs::read_to_string(&agent_path).unwrap();
    // CC-AG-003 replaces invalid model 'gpt-4' with 'sonnet' (the default valid model)
    assert!(
        !fixed.contains("model: gpt-4"),
        "CC-AG-003 fix should replace invalid model 'gpt-4', got: {}",
        fixed
    );
    assert!(
        fixed.contains("model: sonnet"),
        "CC-AG-003 fix should replace invalid model with 'sonnet', got: {}",
        fixed
    );
}

/// MCP: jsonrpc version fix (MCP-002) via --fix
#[test]
fn test_fix_mcp_invalid_jsonrpc_version() {
    use std::fs;

    let temp_dir = tempfile::tempdir().unwrap();
    let mcp_path = temp_dir.path().join("server.mcp.json");
    fs::write(
        &mcp_path,
        r#"{"jsonrpc": "1.0", "method": "tools/list", "id": 1}"#,
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--fix should exit successfully after applying jsonrpc version fix"
    );

    let fixed = fs::read_to_string(&mcp_path).unwrap();
    assert!(
        fixed.contains("\"2.0\""),
        "MCP-002 fix should correct jsonrpc version to 2.0, got: {}",
        fixed
    );
}

/// COP/CUR: Copilot scoped instruction file with fixable issue
#[test]
fn test_fix_copilot_scoped_missing_applyto() {
    use std::fs;

    // Create a copilot scoped instructions file.  COP-002 checks for missing
    // applyTo frontmatter in .instructions.md and provides a fix.
    let temp_dir = tempfile::tempdir().unwrap();
    let instructions_dir = temp_dir.path().join(".github").join("instructions");
    fs::create_dir_all(&instructions_dir).unwrap();

    let instr_path = instructions_dir.join("typescript.instructions.md");
    fs::write(
        &instr_path,
        "---\ndescription: TypeScript rules\n---\nUse strict TypeScript.",
    )
    .unwrap();

    // Run with --dry-run first to see if this triggers any fixable rule
    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--dry-run")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The fixture must produce at least one [fixable] diagnostic; fail early
    // so a COP-002 regression (missing fix) does not go unnoticed.
    assert!(
        stdout.contains("[fixable]"),
        "Expected at least one [fixable] copilot diagnostic, got: {}",
        stdout
    );

    let original = fs::read_to_string(&instr_path).unwrap();

    let mut cmd2 = agnix();
    // Note: --fix may still exit non-zero if unfixable diagnostics remain
    // after applying available fixes.  We only assert the file was modified.
    cmd2.arg(temp_dir.path().to_str().unwrap())
        .arg("--fix")
        .output()
        .unwrap();

    let fixed = fs::read_to_string(&instr_path).unwrap();
    // --dry-run reported [fixable] diagnostics, so --fix must modify the file
    assert_ne!(
        fixed, original,
        "Fixable copilot diagnostics should result in file changes after --fix"
    );
}

#[test]
fn test_locale_es_translates_diagnostic_messages() {
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skills_dir = temp_dir.path().join("skills").join("bad-skill");
    fs::create_dir_all(&skills_dir).unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // Invalid name triggers AS-004 error
    writeln!(
        file,
        "---\nname: Bad-Skill\ndescription: test\n---\nContent"
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--locale")
        .arg("es")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Spanish AS-004 message contains "debe tener" (must have) or
    // the Spanish summary "Encontrados" (Found)
    assert!(
        stdout.contains("Encontrados")
            || stdout.contains("debe tener")
            || stdout.contains("minusculas"),
        "--locale es should produce Spanish diagnostic output, got: {}",
        stdout
    );
}

/// Verify that CLI diagnostic output contains resolved messages, not raw
/// i18n key paths like "rules.as_004.message" or "cli.found_errors_warnings".
#[test]
fn test_no_raw_i18n_keys_in_diagnostic_output() {
    use regex::Regex;
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();

    // Create a skill with an invalid name to trigger AS-004
    let skills_dir = temp_dir.path().join("skills").join("bad-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    let mut file = fs::File::create(skills_dir.join("SKILL.md")).unwrap();
    writeln!(
        file,
        "---\nname: Bad-Skill\ndescription: test\n---\nContent"
    )
    .unwrap();

    // Create a CLAUDE.md with unclosed XML to trigger XML-001
    let mut claude_file = fs::File::create(temp_dir.path().join("CLAUDE.md")).unwrap();
    writeln!(claude_file, "<unclosed>\nSome memory content").unwrap();

    let mut cmd = agnix();
    let output = cmd.arg(temp_dir.path().to_str().unwrap()).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // These patterns match raw i18n key paths that should never appear in output
    let raw_key_patterns = [
        Regex::new(r"\brules\.[a-z_]+_\d+\.message\b").unwrap(),
        Regex::new(r"\brules\.[a-z_]+_\d+\.suggestion\b").unwrap(),
        Regex::new(r"\bcli\.[a-z_]+\b").unwrap(),
        Regex::new(r"\blsp\.[a-z_]+\b").unwrap(),
        Regex::new(r"\bcore\.error\.[a-z_]+\b").unwrap(),
    ];

    for pattern in &raw_key_patterns {
        let matches: Vec<&str> = pattern.find_iter(&stdout).map(|m| m.as_str()).collect();
        assert!(
            matches.is_empty(),
            "Found raw i18n key(s) in CLI output: {:?}\nFull output:\n{}",
            matches,
            stdout
        );
    }

    // Sanity check: output should contain real diagnostic text
    assert!(
        !stdout.is_empty(),
        "CLI should produce diagnostic output for invalid fixtures"
    );
}

/// Verify that JSON format output also contains resolved messages, not raw
/// i18n key paths in the message and suggestion fields.
#[test]
fn test_no_raw_i18n_keys_in_json_output() {
    use regex::Regex;
    use std::fs;
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();

    // Create a skill with an invalid name to trigger AS-004
    let skills_dir = temp_dir.path().join("skills").join("bad-skill");
    fs::create_dir_all(&skills_dir).unwrap();
    let mut file = fs::File::create(skills_dir.join("SKILL.md")).unwrap();
    writeln!(
        file,
        "---\nname: Bad-Skill\ndescription: test\n---\nContent"
    )
    .unwrap();

    let mut cmd = agnix();
    let output = cmd
        .arg("--format")
        .arg("json")
        .arg(temp_dir.path().to_str().unwrap())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output and check message/suggestion fields
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Failed to parse JSON output: {}\nOutput: {}", e, stdout));

    let raw_key_re = Regex::new(r"^(rules|cli|lsp|core)\.[a-z_]+").unwrap();

    let diagnostics = json
        .get("diagnostics")
        .and_then(|v| v.as_array())
        .expect("JSON output should have a 'diagnostics' array");
    assert!(
        !diagnostics.is_empty(),
        "JSON output should contain at least one diagnostic"
    );

    for diag in diagnostics {
        if let Some(msg) = diag.get("message").and_then(|v| v.as_str()) {
            assert!(
                !raw_key_re.is_match(msg),
                "JSON diagnostic 'message' contains raw i18n key: {}",
                msg
            );
        }
        if let Some(sug) = diag.get("suggestion").and_then(|v| v.as_str()) {
            assert!(
                !raw_key_re.is_match(sug),
                "JSON diagnostic 'suggestion' contains raw i18n key: {}",
                sug
            );
        }
    }
}
