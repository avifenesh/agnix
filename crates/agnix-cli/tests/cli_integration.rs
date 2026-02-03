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
    // As of writing, there are 84 rules documented in VALIDATION-RULES.md.
    assert!(
        rules.len() >= 70,
        "Expected at least 70 validation rules, found {} (possible rule registration bug)",
        rules.len()
    );
    assert!(
        rules.len() <= 120,
        "Expected at most 120 validation rules, found {} (unexpected rule explosion)",
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
            help_uri.unwrap().contains("VALIDATION-RULES.md"),
            "helpUri should reference VALIDATION-RULES.md"
        );
    }
}

// ============================================================================
// Text Output Formatting Tests
// ============================================================================

#[test]
fn test_format_text_shows_file_location() {
    let mut cmd = agnix();
    cmd.arg("tests/fixtures/invalid/skills/invalid-name")
        .assert()
        .failure()
        .stdout(predicate::str::is_match(r"[^:]+:\d+:\d+").unwrap());
}

#[test]
fn test_format_text_shows_error_level() {
    let mut cmd = agnix();
    // Match diagnostic line format: file:line:col error: message
    cmd.arg("tests/fixtures/invalid/skills/invalid-name")
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
    cmd.arg("tests/fixtures/invalid/skills/invalid-name")
        .arg("--verbose")
        .assert()
        .failure()
        .stdout(predicate::str::is_match(r"(AS|CC)-\w+-\d+").unwrap());
}

#[test]
fn test_format_text_verbose_shows_suggestion() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/invalid-name")
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
        .arg("tests/fixtures/invalid/skills/invalid-name")
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
fn test_fix_safe_exit_code() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills/invalid-name")
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
fn test_dry_run_with_format_json() {
    let mut cmd = agnix();
    let output = cmd
        .arg("tests/fixtures/invalid/skills")
        .arg("--dry-run")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should still produce valid JSON
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "--dry-run --format json should produce valid JSON, got: {}",
        stdout
    );
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
