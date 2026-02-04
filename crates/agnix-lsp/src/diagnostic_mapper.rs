//! Maps agnix-core diagnostics to LSP diagnostics.

use agnix_core::{Diagnostic, DiagnosticLevel};
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
};

/// Convert an agnix-core diagnostic to an LSP diagnostic.
///
/// Handles the mapping of:
/// - Severity levels (Error, Warning, Info)
/// - Line/column positions (1-indexed to 0-indexed)
/// - Rule codes
/// - Suggestions (appended to message)
pub fn to_lsp_diagnostic(diag: &Diagnostic) -> LspDiagnostic {
    let severity = match diag.level {
        DiagnosticLevel::Error => DiagnosticSeverity::ERROR,
        DiagnosticLevel::Warning => DiagnosticSeverity::WARNING,
        DiagnosticLevel::Info => DiagnosticSeverity::INFORMATION,
    };

    // Convert 1-indexed line/column to 0-indexed
    let line = diag.line.saturating_sub(1) as u32;
    let column = diag.column.saturating_sub(1) as u32;

    // Build message with suggestion if present
    let message = if let Some(ref suggestion) = diag.suggestion {
        format!("{}\n\nSuggestion: {}", diag.message, suggestion)
    } else {
        diag.message.clone()
    };

    LspDiagnostic {
        range: Range {
            start: Position { line, character: column },
            end: Position { line, character: column },
        },
        severity: Some(severity),
        code: Some(NumberOrString::String(diag.rule.clone())),
        code_description: None,
        source: Some("agnix".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
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
        }
    }

    #[test]
    fn test_error_severity_mapping() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Error message", 1, 1, "AS-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn test_warning_severity_mapping() {
        let diag = make_diagnostic(DiagnosticLevel::Warning, "Warning message", 1, 1, "AS-002", None);
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
        assert_eq!(lsp_diag.code, Some(NumberOrString::String("CC-SK-001".to_string())));
    }

    #[test]
    fn test_source_is_agnix() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Test", 1, 1, "AS-001", None);
        let lsp_diag = to_lsp_diagnostic(&diag);
        assert_eq!(lsp_diag.source, Some("agnix".to_string()));
    }

    #[test]
    fn test_message_without_suggestion() {
        let diag = make_diagnostic(DiagnosticLevel::Error, "Error message", 1, 1, "AS-001", None);
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
        assert!(lsp_diag.message.contains("Suggestion: Try doing this instead"));
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
        assert_eq!(lsp_diagnostics[1].severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(lsp_diagnostics[2].severity, Some(DiagnosticSeverity::INFORMATION));
    }
}
