//! Tests to ensure research tracking documentation exists and is consistent.
//!
//! These tests verify that the research tracking infrastructure added in #191
//! remains intact: RESEARCH-TRACKING.md, MONTHLY-REVIEW.md, issue templates,
//! and CONTRIBUTING.md expansions.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn workspace_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_toml) {
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

#[test]
fn test_research_tracking_exists() {
    let root = workspace_root();
    let path = root.join("knowledge-base/RESEARCH-TRACKING.md");
    assert!(
        path.exists(),
        "knowledge-base/RESEARCH-TRACKING.md must exist"
    );

    let content = fs::read_to_string(&path).expect("Failed to read RESEARCH-TRACKING.md");

    let required_sections = [
        "Tool Inventory",
        "Documentation Sources",
        "Academic Research",
        "Community Feedback Log",
    ];

    for section in &required_sections {
        assert!(
            content.contains(section),
            "RESEARCH-TRACKING.md must contain section: {}",
            section
        );
    }

    // Verify S-tier tools are listed
    assert!(
        content.contains("Claude Code"),
        "RESEARCH-TRACKING.md must list Claude Code"
    );
    assert!(
        content.contains("Codex CLI"),
        "RESEARCH-TRACKING.md must list Codex CLI"
    );
    assert!(
        content.contains("OpenCode"),
        "RESEARCH-TRACKING.md must list OpenCode"
    );
}

#[test]
fn test_monthly_review_exists() {
    let root = workspace_root();
    let path = root.join("knowledge-base/MONTHLY-REVIEW.md");
    assert!(path.exists(), "knowledge-base/MONTHLY-REVIEW.md must exist");

    let content = fs::read_to_string(&path).expect("Failed to read MONTHLY-REVIEW.md");

    // Verify the review structure exists (not tied to a specific date)
    assert!(
        content.contains("## Completed Reviews"),
        "MONTHLY-REVIEW.md must contain a Completed Reviews section"
    );
    assert!(
        content.contains("#### Current State"),
        "MONTHLY-REVIEW.md must contain a review with Current State subsection"
    );
    assert!(
        content.contains("#### Coverage Analysis"),
        "MONTHLY-REVIEW.md must contain a review with Coverage Analysis subsection"
    );
    assert!(
        content.contains("#### Findings"),
        "MONTHLY-REVIEW.md must contain a review with Findings subsection"
    );
    assert!(
        content.contains("#### Actions Taken"),
        "MONTHLY-REVIEW.md must contain a review with Actions Taken subsection"
    );
}

#[test]
fn test_index_references_new_docs() {
    let root = workspace_root();
    let path = root.join("knowledge-base/INDEX.md");
    let content = fs::read_to_string(&path).expect("Failed to read INDEX.md");

    assert!(
        content.contains("RESEARCH-TRACKING.md"),
        "INDEX.md must reference RESEARCH-TRACKING.md"
    );
    assert!(
        content.contains("MONTHLY-REVIEW.md"),
        "INDEX.md must reference MONTHLY-REVIEW.md"
    );
}

#[test]
fn test_issue_templates_exist_with_frontmatter() {
    let root = workspace_root();

    // Verify config.yml exists
    let config_path = root.join(".github/ISSUE_TEMPLATE/config.yml");
    assert!(
        config_path.exists(),
        ".github/ISSUE_TEMPLATE/config.yml must exist"
    );

    // Validate rule contribution template
    let rule_template_path = root.join(".github/ISSUE_TEMPLATE/rule_contribution.md");
    let rule_template =
        fs::read_to_string(&rule_template_path).expect("Failed to read rule_contribution.md");
    assert!(
        rule_template.contains("name: Rule Contribution"),
        "rule_contribution.md must have correct name in frontmatter"
    );
    assert!(
        rule_template.contains("rule-proposal"),
        "rule_contribution.md must have rule-proposal label"
    );
    assert!(
        rule_template.contains("## Evidence"),
        "rule_contribution.md must have an Evidence section"
    );

    // Validate tool support template
    let tool_template_path = root.join(".github/ISSUE_TEMPLATE/tool_support_request.md");
    let tool_template =
        fs::read_to_string(&tool_template_path).expect("Failed to read tool_support_request.md");
    assert!(
        tool_template.contains("name: Tool Support Request"),
        "tool_support_request.md must have correct name in frontmatter"
    );
    assert!(
        tool_template.contains("tool-request"),
        "tool_support_request.md must have tool-request label"
    );
    assert!(
        tool_template.contains("## Tier Suggestion"),
        "tool_support_request.md must have a Tier Suggestion section"
    );
}

#[test]
fn test_changelog_documents_research_tracking() {
    let root = workspace_root();
    let changelog =
        fs::read_to_string(root.join("CHANGELOG.md")).expect("Failed to read CHANGELOG.md");

    assert!(
        changelog.contains("RESEARCH-TRACKING.md"),
        "CHANGELOG.md must reference RESEARCH-TRACKING.md"
    );
    assert!(
        changelog.contains("MONTHLY-REVIEW.md"),
        "CHANGELOG.md must reference MONTHLY-REVIEW.md"
    );
    assert!(
        changelog.contains("#191"),
        "CHANGELOG.md must reference issue #191"
    );
}

#[test]
fn test_contributing_expanded() {
    let root = workspace_root();
    let content =
        fs::read_to_string(root.join("CONTRIBUTING.md")).expect("Failed to read CONTRIBUTING.md");

    let required_sections = [
        "Rule Evidence Requirements",
        "Rule ID Conventions",
        "Tool Tier System",
        "Implementing a Validator",
        "Testing Requirements",
        "Community Feedback",
    ];

    for section in &required_sections {
        assert!(
            content.contains(section),
            "CONTRIBUTING.md must contain section: {}",
            section
        );
    }

    // Verify key content within expanded sections
    assert!(
        content.contains("source_type"),
        "Rule Evidence Requirements must document source_type field"
    );
    assert!(
        content.contains("AS-"),
        "Rule ID Conventions must document AS- prefix"
    );
    assert!(
        content.contains("CC-SK-"),
        "Rule ID Conventions must document CC-SK- prefix"
    );
}
