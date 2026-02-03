//! Rule parity integration tests.
//!
//! Ensures all 84 rules from knowledge-base/rules.json are:
//! 1. Registered in SARIF output (sarif.rs)
//! 2. Implemented in agnix-core/src/rules/*.rs
//! 3. Covered by test fixtures in tests/fixtures/

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Rule definition from rules.json
#[derive(Debug, Deserialize)]
struct RulesIndex {
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    severity: String,
    category: String,
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

fn load_rules_json() -> RulesIndex {
    let rules_path = workspace_root().join("knowledge-base/rules.json");
    let content = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rules_path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", rules_path.display(), e))
}

fn extract_sarif_rule_ids() -> BTreeSet<String> {
    let sarif_path = workspace_root().join("crates/agnix-cli/src/sarif.rs");
    let content = fs::read_to_string(&sarif_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", sarif_path.display(), e));

    // Match rule IDs in the rules_data array
    // Handles both single-line: ("AS-001", "description")
    // And multi-line patterns:
    //     (
    //         "AS-004",
    //         "description",
    //     ),
    // The pattern matches quoted rule IDs that appear in the rules_data context
    let re = Regex::new(r#""([A-Z]+-(?:[A-Z]+-)?[0-9]+)""#).unwrap();

    // Filter to only valid rule ID prefixes to avoid matching test assertions
    let valid_prefixes = [
        "AS-", "CC-SK-", "CC-HK-", "CC-AG-", "CC-MEM-", "CC-PL-", "AGM-", "MCP-", "COP-", "XML-",
        "REF-", "PE-", "XP-",
    ];

    // Find the rules_data array bounds to avoid matching rule IDs in test code
    let rules_data_start = content
        .find("let rules_data = [")
        .expect("Could not find start of `rules_data` array in sarif.rs");
    let rules_data_end = content[rules_data_start..]
        .find("];")
        .map(|i| rules_data_start + i)
        .expect("Could not find end of `rules_data` array in sarif.rs");

    let rules_section = &content[rules_data_start..rules_data_end];

    re.captures_iter(rules_section)
        .filter_map(|cap| {
            let id = cap[1].to_string();
            if valid_prefixes.iter().any(|p| id.starts_with(p)) {
                Some(id)
            } else {
                None
            }
        })
        .collect()
}

fn extract_implemented_rule_ids() -> BTreeSet<String> {
    let core_src = workspace_root().join("crates/agnix-core/src");
    let mut rule_ids = BTreeSet::new();

    // Pattern matches rule IDs in Diagnostic::error/warning/info calls
    // e.g., Diagnostic::error(..., "AS-001", ...) or rule: "CC-HK-001".to_string()
    let re = Regex::new(r#""([A-Z]+-(?:[A-Z]+-)?[0-9]+)""#).unwrap();

    // Known rule ID prefixes to filter out false positives
    let valid_prefixes = [
        "AS-", "CC-SK-", "CC-HK-", "CC-AG-", "CC-MEM-", "CC-PL-", "AGM-", "MCP-", "COP-", "XML-",
        "REF-", "PE-", "XP-",
    ];

    // Helper to extract rule IDs from a file
    let extract_from_file = |path: &Path, rule_ids: &mut BTreeSet<String>| {
        if let Ok(content) = fs::read_to_string(path) {
            for cap in re.captures_iter(&content) {
                let rule_id = &cap[1];
                if valid_prefixes.iter().any(|p| rule_id.starts_with(p)) {
                    rule_ids.insert(rule_id.to_string());
                }
            }
        }
    };

    // Scan rules directory
    let rules_dir = core_src.join("rules");
    for entry in fs::read_dir(&rules_dir).expect("Failed to read rules directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            extract_from_file(&path, &mut rule_ids);
        }
    }

    // Also scan lib.rs for project-level rules (e.g., AGM-006)
    extract_from_file(&core_src.join("lib.rs"), &mut rule_ids);

    rule_ids
}

fn scan_fixtures_for_coverage() -> HashMap<String, Vec<String>> {
    let fixtures_dir = workspace_root().join("tests/fixtures");
    let mut coverage: HashMap<String, Vec<String>> = HashMap::new();

    // Pattern to match rule IDs in fixture file content or directory names
    let re = Regex::new(r"[A-Z]+-(?:[A-Z]+-)?[0-9]+").unwrap();

    fn scan_dir_recursive(dir: &Path, re: &Regex, coverage: &mut HashMap<String, Vec<String>>) {
        if !dir.is_dir() {
            return;
        }

        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                scan_dir_recursive(&path, re, coverage);
            } else if path.is_file() {
                // Check file content for explicit rule references
                if let Ok(content) = fs::read_to_string(&path) {
                    for cap in re.captures_iter(&content) {
                        let rule_id = cap[0].to_string();
                        let fixture_path = path.to_string_lossy().to_string();
                        coverage
                            .entry(rule_id)
                            .or_default()
                            .push(fixture_path.clone());
                    }
                }

                // Also check filename patterns like "xml-001-unclosed.md"
                let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                for cap in re.captures_iter(&filename.to_uppercase()) {
                    let rule_id = cap[0].to_string();
                    let fixture_path = path.to_string_lossy().to_string();
                    coverage.entry(rule_id).or_default().push(fixture_path);
                }
            }
        }
    }

    scan_dir_recursive(&fixtures_dir, &re, &mut coverage);
    coverage
}

/// Infer fixture coverage based on directory structure
fn infer_fixture_coverage(rules: &[RuleEntry]) -> HashMap<String, Vec<String>> {
    let fixtures_dir = workspace_root().join("tests/fixtures");
    let mut coverage: HashMap<String, Vec<String>> = HashMap::new();

    // Map categories to fixture directories
    let category_to_dirs: HashMap<&str, Vec<&str>> = [
        (
            "agent-skills",
            vec!["skills", "invalid/skills", "valid/skills"],
        ),
        (
            "claude-skills",
            vec!["skills", "invalid/skills", "valid/skills"],
        ),
        ("claude-hooks", vec!["valid/hooks", "invalid/hooks"]),
        ("claude-agents", vec!["valid/agents", "invalid/agents"]),
        ("claude-memory", vec!["valid/memory", "invalid/memory"]),
        ("claude-plugins", vec!["valid/plugins", "invalid/plugins"]),
        ("agents-md", vec!["agents_md"]),
        ("mcp", vec!["mcp"]),
        ("copilot", vec!["copilot", "copilot-invalid"]),
        ("xml", vec!["xml"]),
        ("references", vec!["refs"]),
        (
            "prompt-engineering",
            vec!["prompt", "invalid/pe", "valid/pe"],
        ),
        ("cross-platform", vec!["cross_platform"]),
    ]
    .into_iter()
    .collect();

    for rule in rules {
        if let Some(dirs) = category_to_dirs.get(rule.category.as_str()) {
            for dir in dirs {
                let full_path = fixtures_dir.join(dir);
                if full_path.exists() {
                    coverage
                        .entry(rule.id.clone())
                        .or_default()
                        .push(full_path.to_string_lossy().to_string());
                }
            }
        }
    }

    coverage
}

#[test]
fn test_all_rules_registered_in_sarif() {
    let rules_index = load_rules_json();
    let sarif_rules = extract_sarif_rule_ids();

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let missing_from_sarif: Vec<&String> = documented_rules.difference(&sarif_rules).collect();

    let extra_in_sarif: Vec<&String> = sarif_rules.difference(&documented_rules).collect();

    let mut report = String::new();

    if !missing_from_sarif.is_empty() {
        report.push_str(&format!(
            "\nMissing from SARIF ({} rules):\n",
            missing_from_sarif.len()
        ));
        for rule in &missing_from_sarif {
            report.push_str(&format!("  - {}\n", rule));
        }
    }

    if !extra_in_sarif.is_empty() {
        report.push_str(&format!(
            "\nExtra in SARIF (not in rules.json) ({} rules):\n",
            extra_in_sarif.len()
        ));
        for rule in &extra_in_sarif {
            report.push_str(&format!("  - {}\n", rule));
        }
    }

    assert!(
        missing_from_sarif.is_empty() && extra_in_sarif.is_empty(),
        "SARIF rule parity check failed:\n{}\nSARIF has {} rules, rules.json has {} rules",
        report,
        sarif_rules.len(),
        documented_rules.len()
    );
}

#[test]
fn test_all_rules_implemented() {
    let rules_index = load_rules_json();
    let implemented_rules = extract_implemented_rule_ids();

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let not_implemented: Vec<&String> = documented_rules.difference(&implemented_rules).collect();

    if !not_implemented.is_empty() {
        let mut report = format!(
            "Rules documented but not found in implementation ({}):\n",
            not_implemented.len()
        );
        for rule in &not_implemented {
            report.push_str(&format!("  - {}\n", rule));
        }
        report.push_str("\nNote: This may indicate:\n");
        report.push_str("  1. Rule not yet implemented\n");
        report.push_str("  2. Rule ID string not found in source (check spelling)\n");

        eprintln!("{}", report);
    }

    // Strict parity: fail if ANY documented rule is not implemented
    assert!(
        not_implemented.is_empty(),
        "{} rules are documented in rules.json but not implemented:\n{}",
        not_implemented.len(),
        not_implemented
            .iter()
            .map(|r| format!("  - {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_fixture_coverage_exists() {
    let rules_index = load_rules_json();
    let explicit_coverage = scan_fixtures_for_coverage();
    let inferred_coverage = infer_fixture_coverage(&rules_index.rules);

    // Combine explicit and inferred coverage
    let mut all_coverage: HashMap<String, Vec<String>> = explicit_coverage;
    for (rule, fixtures) in inferred_coverage {
        all_coverage.entry(rule).or_default().extend(fixtures);
    }

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let covered_rules: BTreeSet<String> = all_coverage.keys().cloned().collect();

    let not_covered: Vec<&String> = documented_rules.difference(&covered_rules).collect();

    // Strict parity: fail if ANY documented rule has no test coverage
    assert!(
        not_covered.is_empty(),
        "{} rules are documented but have no test fixture coverage:\n{}\nAdd test fixtures for uncovered rules.",
        not_covered.len(),
        not_covered
            .iter()
            .map(|r| format!("  - {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_rules_json_integrity() {
    let rules_index = load_rules_json();

    // Check total count matches expected
    assert_eq!(
        rules_index.rules.len(),
        84,
        "Expected 84 rules in rules.json, found {}",
        rules_index.rules.len()
    );

    // Check no duplicate IDs
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for rule in &rules_index.rules {
        assert!(
            seen.insert(rule.id.clone()),
            "Duplicate rule ID found: {}",
            rule.id
        );
    }

    // Check valid severity values
    let valid_severities = ["HIGH", "MEDIUM", "LOW"];
    for rule in &rules_index.rules {
        assert!(
            valid_severities.contains(&rule.severity.as_str()),
            "Invalid severity '{}' for rule {}",
            rule.severity,
            rule.id
        );
    }

    // Check valid category values
    let valid_categories = [
        "agent-skills",
        "claude-skills",
        "claude-hooks",
        "claude-agents",
        "claude-memory",
        "agents-md",
        "claude-plugins",
        "mcp",
        "copilot",
        "xml",
        "references",
        "prompt-engineering",
        "cross-platform",
    ];
    for rule in &rules_index.rules {
        assert!(
            valid_categories.contains(&rule.category.as_str()),
            "Invalid category '{}' for rule {}",
            rule.category,
            rule.id
        );
    }
}

#[test]
fn test_rules_json_matches_validation_rules_md() {
    // Verify rules.json IDs exist in VALIDATION-RULES.md
    let rules_index = load_rules_json();
    let validation_rules_path = workspace_root().join("knowledge-base/VALIDATION-RULES.md");
    let content = fs::read_to_string(&validation_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", validation_rules_path.display(), e));

    let mut missing_in_md: Vec<String> = Vec::new();

    for rule in &rules_index.rules {
        // Check for rule ID as anchor or heading
        let patterns = [
            format!("<a id=\"{}\"></a>", rule.id.to_lowercase()),
            format!("### {} ", rule.id),
            format!("### {}[", rule.id),
        ];

        let found = patterns.iter().any(|p| content.contains(p));
        if !found {
            missing_in_md.push(rule.id.clone());
        }
    }

    assert!(
        missing_in_md.is_empty(),
        "Rules in rules.json but not found in VALIDATION-RULES.md:\n{}",
        missing_in_md
            .iter()
            .map(|r| format!("  - {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_sarif_rule_count() {
    let sarif_rules = extract_sarif_rule_ids();

    // SARIF should have exactly 84 rules to match rules.json
    assert_eq!(
        sarif_rules.len(),
        84,
        "SARIF should have 84 rules, found {}. Missing or extra rules detected.",
        sarif_rules.len()
    );
}
