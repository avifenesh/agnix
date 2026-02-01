//! JSON output format support.
//!
//! Provides a simple, human-readable JSON output format for agnix diagnostics.

use agnix_core::diagnostics::{Diagnostic, DiagnosticLevel};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Root structure for JSON output.
#[derive(Debug, Serialize)]
pub struct JsonOutput {
    /// Version of agnix that produced this output.
    pub version: String,
    /// Number of unique files checked.
    pub files_checked: usize,
    /// List of diagnostics found.
    pub diagnostics: Vec<JsonDiagnostic>,
    /// Summary counts by level.
    pub summary: JsonSummary,
}

/// A single diagnostic in JSON format.
#[derive(Debug, Serialize)]
pub struct JsonDiagnostic {
    /// Severity level: error, warning, or info.
    pub level: String,
    /// Rule identifier (e.g., AS-004).
    pub rule: String,
    /// File path (forward slashes for cross-platform consistency).
    pub file: String,
    /// Line number (1-based).
    pub line: usize,
    /// Column number (1-based).
    pub column: usize,
    /// Diagnostic message.
    pub message: String,
    /// Optional suggestion for fixing the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Summary counts by diagnostic level.
#[derive(Debug, Serialize)]
pub struct JsonSummary {
    /// Number of errors.
    pub errors: usize,
    /// Number of warnings.
    pub warnings: usize,
    /// Number of info messages.
    pub info: usize,
}

fn level_to_string(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
    }
}

fn path_to_string(path: &Path, base_path: &Path) -> String {
    // Convert to relative path if possible, use forward slashes for cross-platform consistency
    path.strip_prefix(base_path)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Convert diagnostics to JSON output format.
pub fn diagnostics_to_json(diagnostics: &[Diagnostic], base_path: &Path) -> JsonOutput {
    // Single pass: count unique files and levels, map diagnostics
    let mut files: HashSet<&std::path::PathBuf> = HashSet::new();
    let mut errors = 0;
    let mut warnings = 0;
    let mut info = 0;

    let json_diagnostics: Vec<JsonDiagnostic> = diagnostics
        .iter()
        .map(|diag| {
            files.insert(&diag.file);
            match diag.level {
                DiagnosticLevel::Error => errors += 1,
                DiagnosticLevel::Warning => warnings += 1,
                DiagnosticLevel::Info => info += 1,
            }
            JsonDiagnostic {
                level: level_to_string(diag.level).to_string(),
                rule: diag.rule.clone(),
                file: path_to_string(&diag.file, base_path),
                line: diag.line.max(1),
                column: diag.column.max(1),
                message: diag.message.clone(),
                suggestion: diag.suggestion.clone(),
            }
        })
        .collect();

    JsonOutput {
        version: env!("CARGO_PKG_VERSION").to_string(),
        files_checked: files.len(),
        diagnostics: json_diagnostics,
        summary: JsonSummary {
            errors,
            warnings,
            info,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_empty_diagnostics() {
        let output = diagnostics_to_json(&[], Path::new("."));
        assert_eq!(output.files_checked, 0);
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.summary.errors, 0);
        assert_eq!(output.summary.warnings, 0);
        assert_eq!(output.summary.info, 0);
    }

    #[test]
    fn test_version_matches_cargo() {
        let output = diagnostics_to_json(&[], Path::new("."));
        assert_eq!(output.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_level_conversion_error() {
        assert_eq!(level_to_string(DiagnosticLevel::Error), "error");
    }

    #[test]
    fn test_level_conversion_warning() {
        assert_eq!(level_to_string(DiagnosticLevel::Warning), "warning");
    }

    #[test]
    fn test_level_conversion_info() {
        assert_eq!(level_to_string(DiagnosticLevel::Info), "info");
    }

    #[test]
    fn test_path_normalization() {
        let path = Path::new("foo\\bar\\baz.md");
        let base = Path::new(".");
        let result = path_to_string(path, base);
        assert!(!result.contains('\\'), "Should use forward slashes");
    }

    #[test]
    fn test_path_relative() {
        let path = PathBuf::from("/project/src/file.md");
        let base = Path::new("/project");
        let result = path_to_string(&path, base);
        assert_eq!(result, "src/file.md");
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

        let output = diagnostics_to_json(&[diag], Path::new("/project"));

        assert_eq!(output.files_checked, 1);
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.summary.errors, 1);

        let json_diag = &output.diagnostics[0];
        assert_eq!(json_diag.level, "error");
        assert_eq!(json_diag.rule, "AS-001");
        assert_eq!(json_diag.file, "test.md");
        assert_eq!(json_diag.line, 10);
        assert_eq!(json_diag.column, 5);
        assert_eq!(json_diag.message, "Missing frontmatter");
    }

    #[test]
    fn test_summary_counts() {
        let diags = vec![
            Diagnostic::error(PathBuf::from("/p/a.md"), 1, 1, "AS-001", "A".to_string()),
            Diagnostic::error(PathBuf::from("/p/b.md"), 2, 2, "AS-002", "B".to_string()),
            Diagnostic::warning(PathBuf::from("/p/c.md"), 3, 3, "AS-003", "C".to_string()),
            Diagnostic {
                level: DiagnosticLevel::Info,
                message: "Info".to_string(),
                file: PathBuf::from("/p/d.md"),
                line: 4,
                column: 4,
                rule: "AS-004".to_string(),
                suggestion: None,
                fixes: vec![],
            },
        ];

        let output = diagnostics_to_json(&diags, Path::new("/p"));

        assert_eq!(output.summary.errors, 2);
        assert_eq!(output.summary.warnings, 1);
        assert_eq!(output.summary.info, 1);
        assert_eq!(output.files_checked, 4);
    }

    #[test]
    fn test_unique_files_counted() {
        // Two diagnostics in the same file should count as 1 file
        let diags = vec![
            Diagnostic::error(PathBuf::from("/p/a.md"), 1, 1, "AS-001", "A".to_string()),
            Diagnostic::error(PathBuf::from("/p/a.md"), 5, 1, "AS-002", "B".to_string()),
        ];

        let output = diagnostics_to_json(&diags, Path::new("/p"));
        assert_eq!(output.files_checked, 1);
    }

    #[test]
    fn test_suggestion_included_when_present() {
        let mut diag = Diagnostic::error(
            PathBuf::from("/p/test.md"),
            1,
            1,
            "AS-004",
            "Invalid name".to_string(),
        );
        diag.suggestion = Some("Use lowercase letters and hyphens only".to_string());

        let output = diagnostics_to_json(&[diag], Path::new("/p"));
        assert_eq!(
            output.diagnostics[0].suggestion,
            Some("Use lowercase letters and hyphens only".to_string())
        );
    }

    #[test]
    fn test_suggestion_omitted_when_none() {
        let diag = Diagnostic::error(
            PathBuf::from("/p/test.md"),
            1,
            1,
            "AS-001",
            "Missing frontmatter".to_string(),
        );

        let output = diagnostics_to_json(&[diag], Path::new("/p"));
        assert!(output.diagnostics[0].suggestion.is_none());
    }

    #[test]
    fn test_json_serialization() {
        let output = diagnostics_to_json(&[], Path::new("."));
        let json = serde_json::to_string(&output);
        assert!(json.is_ok(), "Should serialize to JSON");

        let json_str = json.unwrap();
        assert!(json_str.contains("\"version\""));
        assert!(json_str.contains("\"files_checked\""));
        assert!(json_str.contains("\"diagnostics\""));
        assert!(json_str.contains("\"summary\""));
    }

    #[test]
    fn test_line_column_clamped_to_one() {
        let diag = Diagnostic::error(
            PathBuf::from("/p/test.md"),
            0,
            0,
            "AS-001",
            "Test".to_string(),
        );

        let output = diagnostics_to_json(&[diag], Path::new("/p"));
        assert_eq!(output.diagnostics[0].line, 1);
        assert_eq!(output.diagnostics[0].column, 1);
    }
}
