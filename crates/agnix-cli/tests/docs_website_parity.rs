//! Documentation website parity tests.
//!
//! Ensures docs website rule pages stay synchronized with knowledge-base/rules.json
//! and include required sections such as examples and versioned docs metadata.

use serde::Deserialize;
use std::collections::BTreeSet;
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

    let markdown_count = fs::read_dir(&docs_dir)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", docs_dir.display(), e))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        .count();

    assert_eq!(
        markdown_count, rules.total_rules,
        "Expected {} generated rule docs, found {} in {}",
        rules.total_rules, markdown_count, docs_dir.display()
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

    // Sanity check location to help debug failures quickly.
    assert!(
        docs_root.starts_with(root.join("website")),
        "Docs root should live under website/: {}",
        docs_root.display()
    );
}

fn collect_markdown_relpaths(dir: &Path) -> BTreeSet<PathBuf> {
    fn visit(base: &Path, current: &Path, out: &mut BTreeSet<PathBuf>) {
        let entries = fs::read_dir(current)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", current.display(), e));
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                visit(base, &path, out);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "md") {
                let rel = path
                    .strip_prefix(base)
                    .unwrap_or_else(|e| panic!("Failed to relativize {}: {}", path.display(), e))
                    .to_path_buf();
                out.insert(rel);
            }
        }
    }

    let mut out = BTreeSet::new();
    visit(dir, dir, &mut out);
    out
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

    let rules = load_rules_json();
    let current_docs_root = root.join("website/docs");
    let current_docs = collect_markdown_relpaths(&current_docs_root);
    assert!(
        current_docs.contains(Path::new("intro.md")),
        "Expected intro.md in {}",
        current_docs_root.display()
    );
    assert!(
        current_docs.contains(Path::new("getting-started.md")),
        "Expected getting-started.md in {}",
        current_docs_root.display()
    );
    for version in parsed {
        let version_docs_root = root.join(format!("website/versioned_docs/version-{}", version));
        assert_rules_bundle(root, &rules, &version_docs_root);

        let version_docs = collect_markdown_relpaths(&version_docs_root);
        for rel in &current_docs {
            assert!(
                version_docs.contains(rel),
                "Versioned docs {} missing file {}",
                version_docs_root.display(),
                rel.display()
            );
        }
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
        workflow.contains("rhysd/actionlint@62dc61a45fc95efe8c800af7a557ab0b9165d63b"),
        "docs-site workflow is missing pinned actionlint step in {}",
        workflow_path.display()
    );
}
