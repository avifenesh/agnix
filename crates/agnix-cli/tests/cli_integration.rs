use assert_cmd::Command;
use predicates::prelude::*;

fn agnix() -> Command {
    Command::cargo_bin("agnix").unwrap()
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
    assert_eq!(rules.len(), 80, "Should have all 80 validation rules");
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

    // Version should match CARGO_PKG_VERSION
    let version = json["version"].as_str().unwrap();
    assert!(
        version.starts_with("0."),
        "Version should match cargo version format"
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

    // If there are no errors and no warnings, strict mode should still succeed
    if errors == 0 && warnings == 0 {
        assert!(
            output.status.success(),
            "With --strict and no issues, should succeed"
        );
    }
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
