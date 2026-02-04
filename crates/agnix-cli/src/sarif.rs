//! SARIF (Static Analysis Results Interchange Format) output support.
//!
//! Implements SARIF 2.1.0 specification for CI/CD integration.
//! https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html

use agnix_core::diagnostics::{Diagnostic, DiagnosticLevel};
use serde::Serialize;
use std::path::Path;
use std::sync::LazyLock;

const SARIF_SCHEMA: &str =
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const TOOL_NAME: &str = "agnix";
const TOOL_INFO_URI: &str = "https://github.com/avifenesh/agnix";

#[derive(Debug, Serialize)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<Run>,
}

#[derive(Debug, Serialize)]
pub struct Run {
    pub tool: Tool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    pub driver: Driver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub name: String,
    pub version: String,
    pub information_uri: String,
    pub rules: Vec<ReportingDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingDescriptor {
    pub id: String,
    pub short_description: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub level: String,
    pub message: Message,
    pub locations: Vec<Location>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub physical_location: PhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    pub region: Region,
}

#[derive(Debug, Serialize)]
pub struct ArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub start_line: usize,
    pub start_column: usize,
}

fn level_to_sarif(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "note",
    }
}

fn path_to_uri(path: &Path, base_path: &Path) -> String {
    // Convert to relative path if possible, otherwise keep absolute
    let uri_path = path
        .strip_prefix(base_path)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    uri_path
}

static RULES: LazyLock<Vec<ReportingDescriptor>> = LazyLock::new(|| {
    let rules_data = [
        // Agent Skills Rules (AS-001 to AS-015)
        ("AS-001", "Missing YAML frontmatter in SKILL.md"),
        ("AS-002", "Missing required field: name"),
        ("AS-003", "Missing required field: description"),
        (
            "AS-004",
            "Invalid name format (must be lowercase letters, numbers, hyphens)",
        ),
        ("AS-005", "Name starts or ends with hyphen"),
        ("AS-006", "Consecutive hyphens in name"),
        ("AS-007", "Reserved name (anthropic, claude, skill)"),
        (
            "AS-008",
            "Description too short or too long (must be 1-1024 chars)",
        ),
        ("AS-009", "Description contains XML tags"),
        (
            "AS-010",
            "Missing trigger phrase (should include 'Use when')",
        ),
        ("AS-011", "Compatibility field too long (max 500 chars)"),
        ("AS-012", "Content exceeds 500 lines"),
        ("AS-013", "File reference too deep (must be one level)"),
        ("AS-014", "Windows path separator (use forward slashes)"),
        ("AS-015", "Upload size exceeds 8MB"),
        ("AS-016", "Failed to parse SKILL.md frontmatter"),
        // Claude Code Skills Rules (CC-SK-001 to CC-SK-009)
        (
            "CC-SK-001",
            "Invalid model value (must be sonnet, opus, haiku, or inherit)",
        ),
        (
            "CC-SK-002",
            "Invalid context value (must be 'fork' or omitted)",
        ),
        ("CC-SK-003", "Context 'fork' requires agent field"),
        ("CC-SK-004", "Agent field requires context: fork"),
        ("CC-SK-005", "Invalid agent type"),
        (
            "CC-SK-006",
            "Dangerous auto-invocation (side-effect skills need disable-model-invocation)",
        ),
        (
            "CC-SK-007",
            "Unrestricted Bash in allowed-tools (should be scoped)",
        ),
        ("CC-SK-008", "Unknown tool name"),
        ("CC-SK-009", "Too many dynamic injections (limit 3)"),
        // Claude Code Hooks Rules (CC-HK-001 to CC-HK-011)
        ("CC-HK-001", "Invalid hook event name"),
        (
            "CC-HK-002",
            "Prompt hook on wrong event (only for Stop/SubagentStop)",
        ),
        ("CC-HK-003", "Missing matcher for tool events"),
        ("CC-HK-004", "Matcher on non-tool event"),
        ("CC-HK-005", "Missing type field (command or prompt)"),
        ("CC-HK-006", "Missing command field for command hook"),
        ("CC-HK-007", "Missing prompt field for prompt hook"),
        ("CC-HK-008", "Hook script file not found"),
        ("CC-HK-009", "Dangerous command pattern detected"),
        ("CC-HK-010", "No timeout specified for hook"),
        (
            "CC-HK-011",
            "Invalid timeout value (must be positive integer)",
        ),
        ("CC-HK-012", "Failed to parse hooks configuration"),
        // Claude Code Agents Rules (CC-AG-001 to CC-AG-007)
        ("CC-AG-001", "Missing name field in agent frontmatter"),
        (
            "CC-AG-002",
            "Missing description field in agent frontmatter",
        ),
        ("CC-AG-003", "Invalid model value in agent"),
        ("CC-AG-004", "Invalid permission mode"),
        ("CC-AG-005", "Referenced skill not found"),
        ("CC-AG-006", "Tool in both tools and disallowedTools"),
        ("CC-AG-007", "Failed to parse agent frontmatter"),
        // Claude Code Memory Rules (CC-MEM-001 to CC-MEM-010)
        ("CC-MEM-001", "Invalid import path"),
        ("CC-MEM-002", "Circular import detected"),
        ("CC-MEM-003", "Import depth exceeds 5"),
        ("CC-MEM-004", "Invalid npm script reference"),
        ("CC-MEM-005", "Generic instruction detected"),
        (
            "CC-MEM-006",
            "Negative instruction without positive alternative",
        ),
        ("CC-MEM-007", "Weak constraint language in critical section"),
        ("CC-MEM-008", "Critical content in middle of document"),
        (
            "CC-MEM-009",
            "Token count exceeded (should be under 1500 tokens)",
        ),
        ("CC-MEM-010", "README duplication detected"),
        // AGENTS.md Rules (AGM-001 to AGM-006)
        ("AGM-001", "Invalid markdown structure"),
        ("AGM-002", "Missing section headers"),
        (
            "AGM-003",
            "Character limit exceeded for Windsurf compatibility",
        ),
        ("AGM-004", "Missing project context"),
        ("AGM-005", "Platform-specific features without guard"),
        ("AGM-006", "Nested AGENTS.md hierarchy detected"),
        // Claude Code Plugins Rules (CC-PL-001 to CC-PL-005)
        (
            "CC-PL-001",
            "Plugin manifest not in .claude-plugin/ directory",
        ),
        ("CC-PL-002", "Components inside .claude-plugin/ directory"),
        ("CC-PL-003", "Invalid semver version format"),
        ("CC-PL-004", "Missing required plugin field"),
        ("CC-PL-005", "Empty plugin name"),
        ("CC-PL-006", "Failed to parse plugin.json"),
        // MCP Rules (MCP-001 to MCP-008)
        ("MCP-001", "Invalid JSON-RPC version (must be 2.0)"),
        ("MCP-002", "Missing required tool field"),
        ("MCP-003", "Invalid JSON Schema in inputSchema"),
        ("MCP-004", "Missing tool description"),
        ("MCP-005", "Tool without user consent"),
        ("MCP-006", "Untrusted annotations from server"),
        ("MCP-007", "Failed to parse MCP configuration"),
        ("MCP-008", "Protocol version mismatch in initialize message"),
        // GitHub Copilot Rules (COP-001 to COP-004)
        ("COP-001", "Empty Copilot instruction file"),
        (
            "COP-002",
            "Invalid frontmatter in scoped instructions (missing applyTo)",
        ),
        ("COP-003", "Invalid glob pattern in applyTo"),
        ("COP-004", "Unknown frontmatter keys in scoped instructions"),
        // Cursor Project Rules (CUR-001 to CUR-006)
        ("CUR-001", "Empty Cursor rule file"),
        ("CUR-002", "Missing frontmatter in .mdc file"),
        ("CUR-003", "Invalid YAML frontmatter in .mdc file"),
        ("CUR-004", "Invalid glob pattern in globs field"),
        ("CUR-005", "Unknown frontmatter keys in .mdc file"),
        (
            "CUR-006",
            "Legacy .cursorrules file detected - consider migrating to .mdc format",
        ),
        // XML Rules (XML-001 to XML-003)
        ("XML-001", "Unclosed XML tag"),
        ("XML-002", "Mismatched closing tag"),
        ("XML-003", "Unmatched closing tag"),
        // Reference Rules (REF-001 to REF-002)
        ("REF-001", "Import file not found"),
        ("REF-002", "Broken markdown link"),
        // Prompt Engineering Rules (PE-001 to PE-004)
        (
            "PE-001",
            "Lost in the middle - critical content in middle section",
        ),
        ("PE-002", "Chain-of-thought on simple task"),
        ("PE-003", "Weak imperative language in critical rules"),
        ("PE-004", "Ambiguous instructions"),
        // Cross-Platform Rules (XP-001 to XP-003)
        ("XP-001", "Platform-specific feature in generic config"),
        ("XP-002", "AGENTS.md platform compatibility issue"),
        ("XP-003", "Hard-coded platform paths"),
    ];

    rules_data
        .iter()
        .map(|(id, desc)| ReportingDescriptor {
            id: id.to_string(),
            short_description: Message {
                text: desc.to_string(),
            },
            help_uri: Some(format!(
                "https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#{}",
                id.to_lowercase()
            )),
        })
        .collect()
});

fn get_all_rules() -> Vec<ReportingDescriptor> {
    RULES.clone()
}

pub fn diagnostics_to_sarif(diagnostics: &[Diagnostic], base_path: &Path) -> SarifLog {
    let results: Vec<SarifResult> = diagnostics
        .iter()
        .map(|diag| SarifResult {
            rule_id: diag.rule.clone(),
            level: level_to_sarif(diag.level).to_string(),
            message: Message {
                text: diag.message.clone(),
            },
            locations: vec![Location {
                physical_location: PhysicalLocation {
                    artifact_location: ArtifactLocation {
                        uri: path_to_uri(&diag.file, base_path),
                    },
                    region: Region {
                        // SARIF requires 1-based positions; clamp to 1 for diagnostics without location
                        start_line: diag.line.max(1),
                        start_column: diag.column.max(1),
                    },
                },
            }],
        })
        .collect();

    SarifLog {
        schema: SARIF_SCHEMA.to_string(),
        version: SARIF_VERSION.to_string(),
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: TOOL_NAME.to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    information_uri: TOOL_INFO_URI.to_string(),
                    rules: get_all_rules(),
                },
            },
            results,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_sarif_version() {
        let sarif = diagnostics_to_sarif(&[], Path::new("."));
        assert_eq!(sarif.version, "2.1.0");
    }

    #[test]
    fn test_sarif_schema() {
        let sarif = diagnostics_to_sarif(&[], Path::new("."));
        assert!(sarif.schema.contains("sarif-schema-2.1.0"));
    }

    #[test]
    fn test_level_mapping_error() {
        assert_eq!(level_to_sarif(DiagnosticLevel::Error), "error");
    }

    #[test]
    fn test_level_mapping_warning() {
        assert_eq!(level_to_sarif(DiagnosticLevel::Warning), "warning");
    }

    #[test]
    fn test_level_mapping_info() {
        assert_eq!(level_to_sarif(DiagnosticLevel::Info), "note");
    }

    #[test]
    fn test_path_normalization_forward_slashes() {
        let path = Path::new("foo\\bar\\baz.md");
        let base = Path::new(".");
        let uri = path_to_uri(path, base);
        assert!(!uri.contains('\\'), "URI should use forward slashes");
        assert!(uri.contains('/') || !uri.contains('\\'));
    }

    #[test]
    fn test_path_normalization_relative() {
        let path = PathBuf::from("/project/src/file.md");
        let base = Path::new("/project");
        let uri = path_to_uri(&path, base);
        assert_eq!(uri, "src/file.md");
    }

    #[test]
    fn test_empty_diagnostics_produces_valid_sarif() {
        let sarif = diagnostics_to_sarif(&[], Path::new("."));
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        assert!(sarif.runs[0].results.is_empty());
        assert_eq!(sarif.runs[0].tool.driver.name, "agnix");
    }

    #[test]
    fn test_rules_array_populated() {
        let sarif = diagnostics_to_sarif(&[], Path::new("."));
        let rules = &sarif.runs[0].tool.driver.rules;
        // Should have 96 rules based on VALIDATION-RULES.md
        assert_eq!(rules.len(), 96, "Expected 96 rules in SARIF driver");

        // Verify some specific rules exist
        let rule_ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        assert!(rule_ids.contains(&"AS-001"));
        assert!(rule_ids.contains(&"CC-HK-001"));
        assert!(rule_ids.contains(&"MCP-001"));
        assert!(rule_ids.contains(&"COP-001"));
        assert!(rule_ids.contains(&"CUR-001"));
        assert!(rule_ids.contains(&"XML-001"));
        assert!(rule_ids.contains(&"XP-003"));
    }

    #[test]
    fn test_diagnostic_conversion() {
        let diag = Diagnostic::error(
            PathBuf::from("/project/test.md"),
            10,
            5,
            "AS-001",
            "Missing frontmatter".to_string(),
        );

        let sarif = diagnostics_to_sarif(&[diag], Path::new("/project"));

        assert_eq!(sarif.runs[0].results.len(), 1);
        let result = &sarif.runs[0].results[0];
        assert_eq!(result.rule_id, "AS-001");
        assert_eq!(result.level, "error");
        assert_eq!(result.message.text, "Missing frontmatter");
        assert_eq!(result.locations[0].physical_location.region.start_line, 10);
        assert_eq!(result.locations[0].physical_location.region.start_column, 5);
        assert_eq!(
            result.locations[0].physical_location.artifact_location.uri,
            "test.md"
        );
    }

    #[test]
    fn test_sarif_json_serialization() {
        let sarif = diagnostics_to_sarif(&[], Path::new("."));
        let json = serde_json::to_string(&sarif);
        assert!(json.is_ok(), "SARIF should serialize to JSON");

        let json_str = json.unwrap();
        assert!(json_str.contains("\"$schema\""));
        assert!(json_str.contains("\"version\":\"2.1.0\""));
        assert!(json_str.contains("\"driver\""));
        assert!(json_str.contains("\"rules\""));
    }

    #[test]
    fn test_path_to_uri_fallback_when_not_prefix() {
        let path = PathBuf::from("/different/absolute/path.md");
        let base = Path::new("/project");
        let uri = path_to_uri(&path, base);
        // Should return full path when base is not a prefix
        assert!(uri.contains("different/absolute/path.md"));
    }

    #[test]
    fn test_diagnostic_single_location() {
        let diag = Diagnostic::error(
            PathBuf::from("/project/test.md"),
            10,
            5,
            "AS-001",
            "Test".to_string(),
        );
        let sarif = diagnostics_to_sarif(&[diag], Path::new("/project"));
        assert_eq!(
            sarif.runs[0].results[0].locations.len(),
            1,
            "Each diagnostic should produce exactly one location"
        );
    }

    #[test]
    fn test_warning_level_conversion() {
        let diag = Diagnostic::warning(
            PathBuf::from("/project/test.md"),
            5,
            1,
            "CC-SK-006",
            "Warning message".to_string(),
        );
        let sarif = diagnostics_to_sarif(&[diag], Path::new("/project"));
        assert_eq!(sarif.runs[0].results[0].level, "warning");
    }

    #[test]
    fn test_info_level_conversion() {
        let diag = Diagnostic {
            level: DiagnosticLevel::Info,
            message: "Info message".to_string(),
            file: PathBuf::from("/project/test.md"),
            line: 1,
            column: 1,
            rule: "info".to_string(),
            suggestion: None,
            fixes: vec![],
        };
        let sarif = diagnostics_to_sarif(&[diag], Path::new("/project"));
        assert_eq!(sarif.runs[0].results[0].level, "note");
    }

    #[test]
    fn test_multiple_diagnostics_different_files() {
        let diags = vec![
            Diagnostic::error(PathBuf::from("/p/a.md"), 1, 1, "AS-001", "A".to_string()),
            Diagnostic::warning(PathBuf::from("/p/b.md"), 2, 2, "AS-002", "B".to_string()),
            Diagnostic::error(PathBuf::from("/p/c.md"), 3, 3, "AS-003", "C".to_string()),
        ];
        let sarif = diagnostics_to_sarif(&diags, Path::new("/p"));
        assert_eq!(sarif.runs[0].results.len(), 3);
        assert_eq!(
            sarif.runs[0].results[0].locations[0]
                .physical_location
                .artifact_location
                .uri,
            "a.md"
        );
        assert_eq!(
            sarif.runs[0].results[1].locations[0]
                .physical_location
                .artifact_location
                .uri,
            "b.md"
        );
        assert_eq!(
            sarif.runs[0].results[2].locations[0]
                .physical_location
                .artifact_location
                .uri,
            "c.md"
        );
    }

    #[test]
    fn test_no_duplicate_rule_ids() {
        let rules = get_all_rules();
        let mut ids: Vec<&str> = rules.iter().map(|r| r.id.as_str()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Should have no duplicate rule IDs");
    }

    #[test]
    fn test_help_uri_format_and_anchor() {
        let rules = get_all_rules();
        const BASE_URL: &str =
            "https://github.com/avifenesh/agnix/blob/main/knowledge-base/VALIDATION-RULES.md#";

        for rule in rules {
            let uri = rule.help_uri.expect("All rules should have help_uri");

            assert!(
                uri.starts_with(BASE_URL),
                "Rule {} has invalid help_uri base: {}",
                rule.id,
                uri
            );

            let anchor = uri
                .strip_prefix(BASE_URL)
                .expect("Anchor should be present");

            assert_eq!(
                anchor,
                rule.id.to_lowercase(),
                "Anchor for rule {} should be its lowercase ID, but was '{}'",
                rule.id,
                anchor
            );
        }
    }
}
