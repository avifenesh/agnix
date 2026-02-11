//! Documentation website parity tests.
//!
//! Ensures docs website rule pages stay synchronized with knowledge-base/rules.json
//! and include required sections such as examples and versioned docs metadata.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct RulesIndex {
    total_rules: usize,
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    id: String,
}

fn workspace_root() -> &'static Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_toml)
                && content.lines().any(|line| {
                    let trimmed = line.trim();
                    trimmed == "[workspace]" || trimmed.starts_with("[workspace.")
                })
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

fn slug(rule_id: &str) -> String {
    rule_id.to_ascii_lowercase()
}

fn load_rules_json() -> RulesIndex {
    let rules_path = workspace_root().join("knowledge-base/rules.json");
    let content = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rules_path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", rules_path.display(), e))
}

fn assert_rules_bundle(root: &Path, rules: &RulesIndex, docs_root: &Path) {
    let docs_dir = docs_root.join("rules/generated");
    assert!(
        docs_dir.exists(),
        "Generated rules docs directory missing: {}",
        docs_dir.display()
    );

    let entries = fs::read_dir(&docs_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", docs_dir.display(), e));
    let mut markdown_count = 0usize;
    for entry_result in entries {
        let entry = entry_result.unwrap_or_else(|e| {
            panic!(
                "Failed to read directory entry in {}: {}",
                docs_dir.display(),
                e
            )
        });
        if entry.path().extension().is_some_and(|ext| ext == "md") {
            markdown_count += 1;
        }
    }

    assert_eq!(
        markdown_count,
        rules.total_rules,
        "Expected {} generated rule docs, found {} in {}",
        rules.total_rules,
        markdown_count,
        docs_dir.display()
    );

    for rule in &rules.rules {
        let doc_path = docs_dir.join(format!("{}.md", slug(&rule.id)));
        assert!(doc_path.exists(), "Missing rule doc for {}", rule.id);

        let content = fs::read_to_string(&doc_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", doc_path.display(), e));
        assert!(
            content.contains("## Examples"),
            "Rule doc {} is missing examples section",
            doc_path.display()
        );
        assert!(
            content.contains("### Invalid") && content.contains("### Valid"),
            "Rule doc {} is missing invalid/valid example blocks",
            doc_path.display()
        );
    }

    let index_path = docs_root.join("rules/index.md");
    assert!(
        index_path.exists(),
        "Missing rules index page: {}",
        index_path.display()
    );
    let index_content = fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", index_path.display(), e));
    for rule in &rules.rules {
        let expected_link = format!("./generated/{}", slug(&rule.id));
        assert!(
            index_content.contains(&expected_link),
            "Rules index {} missing link for {}",
            index_path.display(),
            rule.id
        );
    }

    assert!(
        docs_root.starts_with(root.join("website")),
        "Docs root should live under website/: {}",
        docs_root.display()
    );
}

#[test]
fn generated_rule_docs_match_rules_json() {
    let root = workspace_root();
    let index = load_rules_json();
    assert_rules_bundle(root, &index, &root.join("website/docs"));
}

#[test]
fn docs_site_has_search_and_versioning_configuration() {
    let root = workspace_root();
    let config_path = root.join("website/docusaurus.config.js");
    let config = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", config_path.display(), e));

    assert!(
        config.contains("@easyops-cn/docusaurus-search-local"),
        "Search plugin not configured in {}",
        config_path.display()
    );
    assert!(
        config.contains("docsVersionDropdown"),
        "Docs version dropdown is not configured in {}",
        config_path.display()
    );
    assert!(
        config.contains("routeBasePath: 'docs'"),
        "Docs route base path is missing in {}",
        config_path.display()
    );

    let versions_path = root.join("website/versions.json");
    assert!(
        versions_path.exists(),
        "Missing version metadata file: {}",
        versions_path.display()
    );

    let versions = fs::read_to_string(&versions_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", versions_path.display(), e));
    let parsed: Vec<String> = serde_json::from_str(&versions)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", versions_path.display(), e));

    assert!(
        !parsed.is_empty(),
        "versions.json must contain at least one version entry"
    );

    for version in parsed {
        let version_docs_root = root.join(format!("website/versioned_docs/version-{}", version));
        assert!(
            version_docs_root.exists(),
            "Versioned docs directory missing: {}",
            version_docs_root.display()
        );

        let version_index = version_docs_root.join("rules/index.md");
        assert!(
            version_index.exists(),
            "Versioned rules index missing: {}",
            version_index.display()
        );

        let version_rules_dir = version_docs_root.join("rules/generated");
        assert!(
            version_rules_dir.exists(),
            "Versioned generated rules directory missing: {}",
            version_rules_dir.display()
        );

        let entries = fs::read_dir(&version_rules_dir)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", version_rules_dir.display(), e));
        let mut checked_file: Option<PathBuf> = None;
        let mut count = 0usize;
        for entry_result in entries {
            let entry = entry_result.unwrap_or_else(|e| {
                panic!(
                    "Failed to read directory entry in {}: {}",
                    version_rules_dir.display(),
                    e
                )
            });
            if entry.path().extension().is_some_and(|ext| ext == "md") {
                count += 1;
                if checked_file.is_none() {
                    checked_file = Some(entry.path());
                }
            }
        }
        assert!(
            count > 0,
            "No generated rule docs found in {}",
            version_rules_dir.display()
        );
        let sample_path = checked_file.expect("Expected at least one versioned rule doc");
        let sample_content = fs::read_to_string(&sample_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", sample_path.display(), e));
        assert!(
            sample_content.contains("## Examples")
                && sample_content.contains("### Invalid")
                && sample_content.contains("### Valid"),
            "Versioned rule doc {} is missing example sections",
            sample_path.display()
        );
    }

    let package_path = root.join("website/package.json");
    let package_content = fs::read_to_string(&package_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", package_path.display(), e));
    let package_json: serde_json::Value = serde_json::from_str(&package_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", package_path.display(), e));
    let deps = package_json
        .get("dependencies")
        .and_then(serde_json::Value::as_object)
        .expect("website/package.json.dependencies must be an object");
    assert!(
        deps.contains_key("@easyops-cn/docusaurus-search-local"),
        "Search dependency missing from {}",
        package_path.display()
    );

    let scripts = package_json
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .expect("website/package.json.scripts must be an object");
    assert!(
        scripts.contains_key("version:cut"),
        "version:cut script missing from {}",
        package_path.display()
    );

    let workflow_path = root.join(".github/workflows/docs-site.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", workflow_path.display(), e));
    assert!(
        workflow.contains("rhysd/actionlint@0933c147c9d6587653d45fdcb4c497c57a65f9af"),
        "docs-site workflow is missing pinned actionlint step in {}",
        workflow_path.display()
    );
}

// ---------------------------------------------------------------------------
// Site data parity tests
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SiteData {
    #[serde(rename = "totalRules")]
    total_rules: usize,
    #[serde(rename = "categoryCount")]
    category_count: usize,
    #[serde(rename = "autofixCount")]
    autofix_count: usize,
    #[serde(rename = "uniqueTools")]
    unique_tools: Vec<String>,
}

#[test]
fn site_data_json_matches_rules_json() {
    let root = workspace_root();
    let index = load_rules_json();

    let site_data_path = root.join("website/src/data/siteData.json");
    let site_data_content = fs::read_to_string(&site_data_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", site_data_path.display(), e));
    let site_data: SiteData = serde_json::from_str(&site_data_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", site_data_path.display(), e));

    assert_eq!(
        site_data.total_rules, index.total_rules,
        "siteData.json totalRules ({}) does not match rules.json total_rules ({})",
        site_data.total_rules, index.total_rules
    );

    assert!(
        !site_data.unique_tools.is_empty(),
        "siteData.json uniqueTools should not be empty"
    );

    // Verify uniqueTools is sorted and unique
    let mut sorted = site_data.unique_tools.clone();
    sorted.sort();
    assert_eq!(
        site_data.unique_tools, sorted,
        "siteData.json uniqueTools must be sorted alphabetically"
    );
    let deduped: std::collections::HashSet<&String> = site_data.unique_tools.iter().collect();
    assert_eq!(
        site_data.unique_tools.len(),
        deduped.len(),
        "siteData.json uniqueTools must not contain duplicates"
    );

    // Cross-check autofix count by parsing the full rules.json
    let rules_path = root.join("knowledge-base/rules.json");
    let rules_content = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rules_path.display(), e));
    let rules_value: serde_json::Value = serde_json::from_str(&rules_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", rules_path.display(), e));

    let rules_array = rules_value["rules"]
        .as_array()
        .expect("rules.json must have a 'rules' array");
    let expected_autofix = rules_array
        .iter()
        .filter(|r| r["fix"]["autofix"].as_bool() == Some(true))
        .count();
    assert_eq!(
        site_data.autofix_count, expected_autofix,
        "siteData.json autofixCount ({}) does not match rules.json computed count ({})",
        site_data.autofix_count, expected_autofix
    );

    // Cross-check category count
    let categories = rules_value["categories"]
        .as_object()
        .expect("rules.json must have a 'categories' object");
    assert_eq!(
        site_data.category_count,
        categories.len(),
        "siteData.json categoryCount ({}) does not match rules.json categories ({})",
        site_data.category_count,
        categories.len()
    );
}

#[test]
fn index_js_imports_generated_data() {
    let root = workspace_root();
    let index_path = root.join("website/src/pages/index.js");
    let content = fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", index_path.display(), e));

    assert!(
        content.contains("import siteData from"),
        "index.js must have an import statement for siteData"
    );
    assert!(
        content.contains("siteData.totalRules"),
        "index.js must use siteData.totalRules for dynamic rule count"
    );

    assert!(
        !content.contains("'145 Validation Rules'"),
        "index.js still contains hardcoded '145 Validation Rules' - should use siteData.totalRules"
    );

    assert!(
        !content.contains("value: '145'"),
        "index.js still contains hardcoded stats value: '145' - should use siteData.totalRules"
    );
}

#[test]
fn docusaurus_config_uses_generated_data() {
    let root = workspace_root();
    let config_path = root.join("website/docusaurus.config.js");
    let config = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", config_path.display(), e));

    assert!(
        config.contains("require('./src/data/siteData.json')"),
        "docusaurus.config.js must require siteData.json from generated data"
    );
    assert!(
        config.contains("siteData.totalRules"),
        "docusaurus.config.js should use siteData.totalRules in JSON-LD description"
    );
}
