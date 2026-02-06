//! Maps agnix-core diagnostics to LSP diagnostics.

use agnix_core::{Diagnostic, DiagnosticLevel, Fix};
use rust_i18n::t;
use serde_json::Value as JsonValue;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
};

/// Serialize fixes to JSON for storage in diagnostic.data.
///
/// Returns None if there are no fixes, to avoid cluttering diagnostics.
fn serialize_fixes(fixes: &[Fix]) -> Option<JsonValue> {
    if fixes.is_empty() {
        return None;
    }
    serde_json::to_value(fixes).ok()
}

/// Deserialize fixes from diagnostic.data.
///
/// Returns an empty vector if data is None or invalid.
pub fn deserialize_fixes(data: Option<&JsonValue>) -> Vec<Fix> {
    data.and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Convert an agnix-core diagnostic to an LSP diagnostic.
///
/// Handles the mapping of:
/// - Severity levels (Error, Warning, Info)
/// - Line/column positions (1-indexed to 0-indexed)
/// - Rule codes
/// - Suggestions (appended to message)
/// - Fixes (serialized to diagnostic.data for code actions)
pub fn to_lsp_diagnostic(diag: &Diagnostic) -> LspDiagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Info => DiagnosticSeverity::INFORMATION,
    };

    let line = diag.line.saturating_sub(1) as u32;
    let column = diag.column.saturating_sub(1) as u32;

    let message = if let Some(ref suggestion) = diag.suggestion {
        format!(
            "{}\n\n{} {}",
            diag.message,
            t!("lsp.suggestion_label"),
            suggestion
        )
    } else {
        diag.message.clone()
    };

    let data = serialize_fixes(&diag.fixes);

    LspDiagnostic {
        range: Range {
            start: Position {
                line,
                character: column,
            },
            end: Position {
                line,
                character: column,
            },
        },
        severity: Some(severity),
        code: Some(NumberOrString::String(diag.rule.clone())),
        code_description: None,
        source: Some("agnix".to_string()),
        message,
        related_information: None,
        tags: None,
        data,
    }
}

/// Convert a vector of agnix-core diagnostics to LSP diagnostics.
pub fn to_lsp_diagnostics(diagnostics: Vec<Diagnostic>) -> Vec<LspDiagnostic> {
    diagnostics.iter().map(to_lsp_diagnostic).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_diagnostic(
        level: DiagnosticLevel,
        message: &str,
        line: usize,
        column: usize,
        rule: &str,
        suggestion: Option<&str>,
    ) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            file: PathBuf::from("test.md"),
            line,
            column,
            rule: rule.to_string(),
            suggestion: suggestion.map(String::from),
            fixes: vec![],
            assumption: None,
        }
    }

    fn make_diagnostic_with_fixes(
        level: DiagnosticLevel,
        message: &str,
        line: usize,
        column: usize,
        rule: &str,
        fixes: Vec<Fix>,
    ) -> Diagnostic {
        Diagnostic {
            level,
            message: message.to_string(),
            file: PathBuf::from("test.md"),
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes,
            assumption: None,
        }
    }

    fn make_fix(start: usize, end: usize, replacement: &str, description: &str, safe: bool) -> Fix {
        Fix {
            start_byte: start,
            end_byte: end,
            replacement: replacement.to_string(),
            description: description.to_string(),
            safe,
        }
    }

    #[test]
    fn test_error_severity_mapping() {
        let diag = make_diagnostic(
            DiagnosticLevel::Error,
            "Error message",
            1,
            1,
            "AS-001",
            None,
        );
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_warning_severity_mapping() {
        let diag = make_diagnostic(
            DiagnosticLevel::Warning,
            "Warning message",
            1,
            1,
            "AS-002",
            None,
        );
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::WARNING));
    }

    #[test]
    fn test_info_severity_mapping() {
        let diag = make_diagnostic(DiagnosticLevel::Info, "Info message", 1, 1, "AS-003", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::INFORMATION));
    }

    #[test]
    fn test_line_column_conversion() {
        // 1-indexed to 0-indexed
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test", 10, 5, "AS-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.range.start.line, 9);
        assert_eq!(lsp_diag.range.start.character, 4);
    }

    #[test]
    fn test_line_zero_saturates() {
        // Line 0 should saturate to 0, not underflow
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test", 0, 0, "AS-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.range.start.line, 0);
        assert_eq!(lsp_diag.range.start.character, 0);
    }

    #[test]
    fn test_rule_code() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test", 1, 1, "CC-SK-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(
            lsp_diag.code,
            Some(NumberOrString::String("CC-SK-001".to_string()))
        );
    }

    #[test]
    fn test_source_is_agnix() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test", 1, 1, "AS-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.source, Some("agnix".to_string()));
    }

    #[test]
    fn test_message_without_suggestion() {
        let diag = make_diagnostic(
            DiagnosticLevel::Error,
            "Error message",
            1,
            1,
            "AS-001",
            None,
        );
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.message, "Error message");
    }

    #[test]
    fn test_message_with_suggestion() {
        let diag = make_diagnostic(
            DiagnosticLevel::Error,
            "Error message",
            1,
            1,
            "AS-001",
            Some("Try doing this instead"),
        );
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert!(lsp_diag.message.contains("Error message"));
        assert!(
            lsp_diag
                .message
                .contains("Suggestion: Try doing this instead")
        );
    }

    #[test]
    fn test_to_lsp_diagnostics_empty() {
        let diagnostics: Vec<Diagnostic> = vec![];
        let lsp_diagnostics = to_lsp_diagnostics(diagnostics);
        assert!(lsp_diagnostics.is_empty());
    }

    #[test]
    fn test_to_lsp_diagnostics_multiple() {
        let diagnostics = vec![
            make_diagnostic(DiagnosticLevel::Error, "Error 1", 1, 1, "AS-001", None),
            make_diagnostic(DiagnosticLevel::Warning, "Warning 1", 2, 1, "AS-002", None),
            make_diagnostic(DiagnosticLevel::Info, "Info 1", 3, 1, "AS-003", None),
        ];
        let lsp_diagnostics = to_lsp_diagnostics(diagnostics);
        assert_eq!(lsp_diagnostics.len(), 3);
        assert_eq!(lsp_diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(
            lsp_diagnostics[1].severity,
            Some(DiagnosticSeverity::WARNING)
        );
        assert_eq!(
            lsp_diagnostics[2].severity,
            Some(DiagnosticSeverity::INFORMATION)
        );
    }

    #[test]
    fn test_diagnostic_with_fixes_has_data() {
        let fixes = vec![make_fix(0, 5, "hello", "Replace text", true)];
        let diag =
            make_diagnostic_with_fixes(DiagnosticLevel::Error, "Error", 1, 1, "AS-001", fixes);

        let lsp_diag = to_lsp_diagnostic(&diag);

        assert!(lsp_diag.data.is_some());
    }

    #[test]
    fn test_diagnostic_without_fixes_no_data() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Error", 1, 1, "AS-001", None);

        let lsp_diag = to_lsp_diagnostic(&diag);

        assert!(lsp_diag.data.is_none());
    }

    #[test]
    fn test_serialize_deserialize_fixes_roundtrip() {
        let fixes = vec![
            make_fix(0, 5, "hello", "Replace text", true),
            make_fix(10, 15, "world", "Another fix", false),
        ];

        let serialized = serialize_fixes(&fixes);
        assert!(serialized.is_some());

        let deserialized = deserialize_fixes(serialized.as_ref());
        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].start_byte, 0);
        assert_eq!(deserialized[0].end_byte, 5);
        assert_eq!(deserialized[0].replacement, "hello");
        assert_eq!(deserialized[0].description, "Replace text");
        assert!(deserialized[0].safe);
        assert_eq!(deserialized[1].start_byte, 10);
        assert!(!deserialized[1].safe);
    }

    #[test]
    fn test_deserialize_fixes_none() {
        let fixes = deserialize_fixes(None);
        assert!(fixes.is_empty());
    }

    #[test]
    fn test_deserialize_fixes_invalid_json() {
        let invalid = JsonValue::String("not an array".to_string());
        let fixes = deserialize_fixes(Some(&invalid));
        assert!(fixes.is_empty());
    }

    #[test]
    fn test_serialize_empty_fixes() {
        let fixes: Vec<Fix> = vec![];
        let serialized = serialize_fixes(&fixes);
        assert!(serialized.is_none());
    }
}
