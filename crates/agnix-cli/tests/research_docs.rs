//! Tests to ensure research tracking documentation exists and is consistent.
//!
//! These tests verify that the research tracking infrastructure added in #191
//! remains intact: RESEARCH-TRACKING.md, MONTHLY-REVIEW.md, issue templates,
//! and CONTRIBUTING.md expansions.

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
fn test_research_tracking_exists() {
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

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
}

#[test]
fn test_monthly_review_exists() {
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

    let path = root.join("knowledge-base/MONTHLY-REVIEW.md");
    assert!(
        path.exists(),
        "knowledge-base/MONTHLY-REVIEW.md must exist"
    );

    let content = fs::read_to_string(&path).expect("Failed to read MONTHLY-REVIEW.md");

    assert!(
        content.contains("February 2026"),
        "MONTHLY-REVIEW.md must contain the completed February 2026 review"
    );
}

#[test]
fn test_index_references_new_docs() {
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

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
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

    let templates = [
        ".github/ISSUE_TEMPLATE/rule_contribution.md",
        ".github/ISSUE_TEMPLATE/tool_support_request.md",
        ".github/ISSUE_TEMPLATE/config.yml",
    ];

    for template in &templates {
        let path = root.join(template);
        assert!(path.exists(), "{} must exist", template);
    }

    // Validate rule contribution template frontmatter and structure
    let rule_template = fs::read_to_string(
        root.join(".github/ISSUE_TEMPLATE/rule_contribution.md"),
    )
    .expect("Failed to read rule_contribution.md");
    assert!(
        rule_template.contains("name: Rule Contribution"),
        "rule_contribution.md must have correct name in frontmatter"
    );
    assert!(
        rule_template.contains("rule-proposal"),
        "rule_contribution.md must have rule-proposal label"
    );

    // Validate tool support template frontmatter and structure
    let tool_template = fs::read_to_string(
        root.join(".github/ISSUE_TEMPLATE/tool_support_request.md"),
    )
    .expect("Failed to read tool_support_request.md");
    assert!(
        tool_template.contains("name: Tool Support Request"),
        "tool_support_request.md must have correct name in frontmatter"
    );
    assert!(
        tool_template.contains("tool-request"),
        "tool_support_request.md must have tool-request label"
    );
}

#[test]
fn test_changelog_documents_research_tracking() {
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

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
    let Some(root) = find_workspace_root() else {
        eprintln!("Skipping test: workspace root not found");
        return;
    };

    let path = root.join("CONTRIBUTING.md");
    let content = fs::read_to_string(&path).expect("Failed to read CONTRIBUTING.md");

    let required_sections = [
        "Rule Evidence Requirements",
        "Rule ID Conventions",
        "Tool Tier System",
    ];

    for section in &required_sections {
        assert!(
            content.contains(section),
            "CONTRIBUTING.md must contain section: {}",
            section
        );
    }
}
