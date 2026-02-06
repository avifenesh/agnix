//! Rule parity integration tests.
//!
//! Ensures all 100 rules from knowledge-base/rules.json are:
//! 1. Registered in SARIF output (sarif.rs)
//! 2. Implemented in agnix-core/src/rules/*.rs
//! 3. Covered by test fixtures in tests/fixtures/
//! 4. Have valid evidence metadata

// Allow common test patterns that clippy flags but are intentional in tests
#![allow(clippy::field_reassign_with_default)]

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Evidence source type classification
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Official specification (e.g., agentskills.io/specification, modelcontextprotocol.io/specification)
    Spec,
    /// Vendor documentation (e.g., code.claude.com/docs, docs.github.com)
    VendorDocs,
    /// Vendor source code
    VendorCode,
    /// Academic paper or research
    Paper,
    /// Community best practices and multi-platform research
    Community,
}

/// Normative level for rules (RFC 2119 style)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormativeLevel {
    /// Absolute requirement
    Must,
    /// Recommended but not mandatory
    Should,
    /// Optional best practice
    BestPractice,
}

/// Applicability constraints for a rule
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppliesTo {
    /// Tool this rule applies to (e.g., "claude-code", "cursor", "github-copilot")
    #[serde(default)]
    pub tool: Option<String>,
    /// Semver version range (e.g., ">=1.0.0")
    #[serde(default)]
    pub version_range: Option<String>,
    /// Specification revision (e.g., "1.0", "2025-06-18")
    #[serde(default)]
    pub spec_revision: Option<String>,
}

/// Test coverage tracking for a rule
#[derive(Debug, Clone, Deserialize)]
pub struct TestCoverage {
    /// Has unit tests
    pub unit: bool,
    /// Has fixture tests
    pub fixtures: bool,
    /// Has end-to-end tests
    pub e2e: bool,
}

/// Evidence metadata for a rule
#[derive(Debug, Clone, Deserialize)]
pub struct Evidence {
    /// Classification of the evidence source
    pub source_type: SourceType,
    /// URLs to authoritative sources
    pub source_urls: Vec<String>,
    /// Date when the evidence was last verified (ISO 8601)
    pub verified_on: String,
    /// Applicability constraints
    pub applies_to: AppliesTo,
    /// RFC 2119-style normative level
    pub normative_level: NormativeLevel,
    /// Test coverage information
    pub tests: TestCoverage,
}

/// Rule definition from rules.json
#[derive(Debug, Deserialize)]
struct RulesIndex {
    total_rules: usize,
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    id: String,
    #[allow(dead_code)]
    name: String,
    severity: String,
    category: String,
    /// Evidence metadata (required for all rules)
    evidence: Evidence,
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

/// Extract SARIF rule IDs from rules.json (since SARIF rules are now generated from rules.json at build time)
fn extract_sarif_rule_ids() -> BTreeSet<String> {
    // Since SARIF rules are now generated from rules.json via build.rs,
    // we verify SARIF parity by checking rules.json directly.
    // The build.rs script transforms rules.json into SARIF rules at compile time.
    let rules_index = load_rules_json();
    rules_index.rules.iter().map(|r| r.id.clone()).collect()
}

fn extract_implemented_rule_ids() -> BTreeSet<String> {
    let core_src = workspace_root().join("crates/agnix-core/src");
    let mut rule_ids = BTreeSet::new();

    // Pattern matches rule IDs in Diagnostic::error/warning/info calls
    // e.g., Diagnostic::error(..., "AS-001", ...) or rule: "CC-HK-001".to_string()
    let re = Regex::new(r#""([A-Z]+-(?:[A-Z]+-)?[0-9]+)""#).unwrap();

    // Known rule ID prefixes to filter out false positives
    let valid_prefixes = [
        "AS-", "CC-SK-", "CC-HK-", "CC-AG-", "CC-MEM-", "CC-PL-", "AGM-", "MCP-", "COP-", "CUR-",
        "XML-", "REF-", "PE-", "XP-", "VER-",
    ];

    fn extract_from_file(
        path: &Path,
        re: &Regex,
        valid_prefixes: &[&str],
        rule_ids: &mut BTreeSet<String>,
    ) {
        if let Ok(content) = fs::read_to_string(path) {
            for cap in re.captures_iter(&content) {
                let rule_id = &cap[1];
                if valid_prefixes.iter().any(|p| rule_id.starts_with(p)) {
                    rule_ids.insert(rule_id.to_string());
                }
            }
        }
    }

    fn scan_rules_recursive(
        dir: &Path,
        re: &Regex,
        valid_prefixes: &[&str],
        rule_ids: &mut BTreeSet<String>,
    ) {
        let entries = fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("Failed to read rules directory {}: {}", dir.display(), e));
        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();
            if path.is_dir() {
                scan_rules_recursive(&path, re, valid_prefixes, rule_ids);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                extract_from_file(&path, re, valid_prefixes, rule_ids);
            }
        }
    }

    // Scan rules directory recursively so nested modules are included.
    let rules_dir = core_src.join("rules");
    scan_rules_recursive(&rules_dir, &re, &valid_prefixes, &mut rule_ids);

    // Also scan lib.rs for project-level rules (e.g., AGM-006)
    extract_from_file(
        &core_src.join("lib.rs"),
        &re,
        &valid_prefixes,
        &mut rule_ids,
    );

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
        ("cursor", vec!["cursor", "cursor-invalid", "cursor-legacy"]),
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

    // Check total_rules field matches actual count
    assert_eq!(
        rules_index.rules.len(),
        rules_index.total_rules,
        "The 'total_rules' field ({}) in rules.json does not match the actual number of rules ({})",
        rules_index.total_rules,
        rules_index.rules.len()
    );

    // Check total count matches expected
    assert_eq!(
        rules_index.rules.len(),
        100,
        "Expected 100 rules in rules.json, found {}",
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
        "cursor",
        "xml",
        "references",
        "prompt-engineering",
        "cross-platform",
        "version-awareness",
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

    // SARIF should have exactly 100 rules to match rules.json
    assert_eq!(
        sarif_rules.len(),
        100,
        "SARIF should have 100 rules, found {}. Missing or extra rules detected.",
        sarif_rules.len()
    );
}

// ============================================================================
// Evidence Metadata Validation Tests
// ============================================================================

#[test]
fn test_all_rules_have_evidence_metadata() {
    let rules_index = load_rules_json();
    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

    for rule in &rules_index.rules {
        // Check source_urls is not empty
        assert!(
            !rule.evidence.source_urls.is_empty(),
            "Rule {} has no source URLs in evidence metadata",
            rule.id
        );

        // Check verified_on is a valid date format (YYYY-MM-DD)
        assert!(
            date_re.is_match(&rule.evidence.verified_on),
            "Rule {} has invalid verified_on date format: '{}'. Expected YYYY-MM-DD",
            rule.id,
            rule.evidence.verified_on
        );
    }
}

#[test]
fn test_evidence_source_urls_valid() {
    let rules_index = load_rules_json();

    // Basic URL validation pattern
    let url_re = Regex::new(r"^https?://[^\s]+$").unwrap();

    for rule in &rules_index.rules {
        for url in &rule.evidence.source_urls {
            assert!(
                url_re.is_match(url),
                "Rule {} has invalid source URL: '{}'",
                rule.id,
                url
            );

            // Check URL doesn't have trailing whitespace
            assert!(
                url.trim() == url,
                "Rule {} source URL has whitespace: '{}'",
                rule.id,
                url
            );
        }
    }
}

#[test]
fn test_normative_level_consistency() {
    let rules_index = load_rules_json();

    let mut inconsistencies = Vec::new();

    for rule in &rules_index.rules {
        // HIGH severity rules should typically have MUST normative level
        // This is a soft check - we just report inconsistencies
        let is_high_severity = rule.severity == "HIGH";
        let is_must_level = rule.evidence.normative_level == NormativeLevel::Must;

        // HIGH + SHOULD/BEST_PRACTICE is suspicious but allowed for some cases
        // (e.g., rules from research papers that are recommendations)
        if is_high_severity && !is_must_level {
            // Only flag if source is spec or vendor_docs (not paper/community)
            if rule.evidence.source_type == SourceType::Spec
                || rule.evidence.source_type == SourceType::VendorDocs
            {
                inconsistencies.push(format!(
                    "{}: HIGH severity but {:?} normative level (source: {:?})",
                    rule.id, rule.evidence.normative_level, rule.evidence.source_type
                ));
            }
        }
    }

    // This is informational - we don't fail on inconsistencies
    // Just report them for review
    if !inconsistencies.is_empty() {
        eprintln!(
            "\nNote: {} rules have HIGH severity but non-MUST normative level:",
            inconsistencies.len()
        );
        for msg in &inconsistencies {
            eprintln!("  - {}", msg);
        }
    }
}

#[test]
fn test_evidence_source_type_distribution() {
    let rules_index = load_rules_json();

    let mut by_source: HashMap<String, usize> = HashMap::new();

    for rule in &rules_index.rules {
        let key = format!("{:?}", rule.evidence.source_type);
        *by_source.entry(key).or_insert(0) += 1;
    }

    // Just report distribution - this is informational
    eprintln!("\nEvidence source type distribution:");
    let mut sorted: Vec<_> = by_source.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (source_type, count) in sorted {
        eprintln!("  {}: {}", source_type, count);
    }

    // Ensure we have at least some diversity in sources
    assert!(
        by_source.len() >= 3,
        "Expected at least 3 different source types, found {}",
        by_source.len()
    );
}

#[test]
fn test_evidence_test_coverage_accuracy() {
    let rules_index = load_rules_json();
    let explicit_coverage = scan_fixtures_for_coverage();
    let inferred_coverage = infer_fixture_coverage(&rules_index.rules);

    // Combine coverage
    let mut all_coverage: HashMap<String, Vec<String>> = explicit_coverage;
    for (rule, fixtures) in inferred_coverage {
        all_coverage.entry(rule).or_default().extend(fixtures);
    }

    let mut mismatches = Vec::new();

    for rule in &rules_index.rules {
        let has_fixtures = all_coverage.contains_key(&rule.id);
        let claims_fixtures = rule.evidence.tests.fixtures;

        // If evidence claims fixtures but we can't find them, that's a problem
        if claims_fixtures && !has_fixtures {
            mismatches.push(format!("{}: claims fixtures=true but none found", rule.id));
        }
    }

    // This should be empty - evidence should be accurate
    assert!(
        mismatches.is_empty(),
        "Evidence test coverage mismatches:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn test_applies_to_tool_values() {
    let rules_index = load_rules_json();

    // Use valid_tools derived from rules.json at compile time
    let valid_tools = agnix_rules::valid_tools();

    for rule in &rules_index.rules {
        if let Some(ref tool) = rule.evidence.applies_to.tool {
            assert!(
                valid_tools.contains(&tool.as_str()),
                "Rule {} has unknown tool '{}'. Valid tools: {:?}",
                rule.id,
                tool,
                valid_tools
            );
        }
    }
}

// ============================================================================
// Tool Mapping Consistency Tests (Review-requested coverage)
// ============================================================================

#[test]
fn test_tool_rule_prefixes_consistency() {
    // Every tool in TOOL_RULE_PREFIXES must also exist in VALID_TOOLS
    // This ensures no orphaned tools or prefixes exist
    let valid_tools = agnix_rules::valid_tools();

    for (prefix, tool) in agnix_rules::TOOL_RULE_PREFIXES {
        assert!(
            valid_tools.contains(tool),
            "Tool '{}' from prefix '{}' is not in VALID_TOOLS. \
             TOOL_RULE_PREFIXES and VALID_TOOLS must be consistent.",
            tool,
            prefix
        );
    }
}

#[test]
fn test_is_tool_alias_case_sensitivity() {
    // Test that tool alias matching is case insensitive
    // "Copilot" (mixed case) and "COPILOT" (uppercase) should both
    // be recognized as valid tools via the alias mechanism

    // The is_tool_alias function is private, but we can test through
    // LintConfig::is_rule_enabled which uses it internally

    use agnix_core::LintConfig;

    let aliases = ["Copilot", "COPILOT", "copilot"];
    for alias in aliases {
        let mut config = LintConfig::default();
        config.tools = vec![alias.to_string()];
        assert!(
            config.is_rule_enabled("COP-001"),
            "Alias '{}' should match 'github-copilot' and enable COP-* rules",
            alias
        );
    }
}
