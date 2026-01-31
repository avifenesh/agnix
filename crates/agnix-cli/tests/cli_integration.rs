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
