//! SARIF (Static Analysis Results Interchange Format) output support.
//!
//! Implements SARIF 2.1.0 specification for CI/CD integration.
//! https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
//!
//! Rules are loaded from the agnix-rules crate at compile time.

use agnix_core::diagnostics::{Diagnostic, DiagnosticLevel};
use agnix_rules::RULES_DATA;
use serde::Serialize;
use std::path::Path;
use std::sync::LazyLock;

const SARIF_SCHEMA: &str = "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json";
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
    // Rules loaded from knowledge-base/rules.json at compile time via build.rs
    RULES_DATA
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

fn get_all_rules() -> &'static [ReportingDescriptor] {
    &RULES
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
                    rules: get_all_rules().to_vec(),
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
        // Should have 139 rules based on VALIDATION-RULES.md
        assert_eq!(rules.len(), 139, "Expected 139 rules in SARIF driver");

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
            assumption: None,
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
            let uri = rule
                .help_uri
                .as_ref()
                .expect("All rules should have help_uri");

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

    #[test]
    fn test_zero_line_column_clamped_to_one() {
        // SARIF 2.1.0 requires 1-based positions, so 0 values must be clamped
        let diag = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test error".to_string(),
            file: PathBuf::from("/project/test.md"),
            line: 0,
            column: 0,
            rule: "AS-001".to_string(),
            suggestion: None,
            fixes: vec![],
            assumption: None,
        };

        let sarif = diagnostics_to_sarif(&[diag], Path::new("/project"));

        let region = &sarif.runs[0].results[0].locations[0]
            .physical_location
            .region;

        assert_eq!(
            region.start_line, 1,
            "Line 0 should be clamped to 1 for SARIF compatibility"
        );
        assert_eq!(
            region.start_column, 1,
            "Column 0 should be clamped to 1 for SARIF compatibility"
        );
    }
}
