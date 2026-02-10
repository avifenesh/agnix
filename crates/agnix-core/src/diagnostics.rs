//! Diagnostic types and error reporting

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub type LintResult<T> = Result<T, LintError>;

/// An automatic fix for a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Byte offset start (inclusive)
    pub start_byte: usize,
    /// Byte offset end (exclusive)
    pub end_byte: usize,
    /// Text to insert/replace with
    pub replacement: String,
    /// Human-readable description of what this fix does
    pub description: String,
    /// Whether this fix is safe (HIGH certainty, >95%)
    pub safe: bool,
}

impl Fix {
    /// Create a replacement fix
    pub fn replace(
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        Self {
            start_byte: start,
            end_byte: end,
            replacement: replacement.into(),
            description: description.into(),
            safe,
        }
    }

    /// Create an insertion fix (start == end)
    pub fn insert(
        position: usize,
        text: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        Self {
            start_byte: position,
            end_byte: position,
            replacement: text.into(),
            description: description.into(),
            safe,
        }
    }

    /// Create a deletion fix (replacement is empty)
    pub fn delete(start: usize, end: usize, description: impl Into<String>, safe: bool) -> Self {
        Self {
            start_byte: start,
            end_byte: end,
            replacement: String::new(),
            description: description.into(),
            safe,
        }
    }

    /// Check if this is an insertion (start == end)
    pub fn is_insertion(&self) -> bool {
        self.start_byte == self.end_byte && !self.replacement.is_empty()
    }

    /// Check if this is a deletion (empty replacement)
    pub fn is_deletion(&self) -> bool {
        self.replacement.is_empty() && self.start_byte < self.end_byte
    }
}

/// A diagnostic message from the linter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub rule: String,
    pub suggestion: Option<String>,
    /// Automatic fixes for this diagnostic
    #[serde(default)]
    pub fixes: Vec<Fix>,
    /// Assumption note for version-aware validation
    ///
    /// When tool/spec versions are not pinned, validators may use default
    /// assumptions. This field documents those assumptions to help users
    /// understand what behavior is expected and how to get version-specific
    /// validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assumption: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    pub fn error(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
        }
    }

    pub fn warning(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
        }
    }

    pub fn info(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            level: DiagnosticLevel::Info,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add an assumption note for version-aware validation
    ///
    /// Used when tool/spec versions are not pinned to document what
    /// default behavior the validator is assuming.
    pub fn with_assumption(mut self, assumption: impl Into<String>) -> Self {
        self.assumption = Some(assumption.into());
        self
    }

    /// Add an automatic fix to this diagnostic
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Add multiple automatic fixes to this diagnostic
    pub fn with_fixes(mut self, fixes: impl IntoIterator<Item = Fix>) -> Self {
        self.fixes.extend(fixes);
        self
    }

    /// Check if this diagnostic has any fixes available
    pub fn has_fixes(&self) -> bool {
        !self.fixes.is_empty()
    }

    /// Check if this diagnostic has any safe fixes available
    pub fn has_safe_fixes(&self) -> bool {
        self.fixes.iter().any(|f| f.safe)
    }
}

/// Linter errors
#[derive(Error, Debug)]
pub enum LintError {
    #[error("Failed to read file: {path}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file: {path}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Refusing to read symlink: {path}")]
    FileSymlink { path: PathBuf },

    #[error("File too large: {path} ({size} bytes, limit {limit} bytes)")]
    FileTooBig {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("Not a regular file: {path}")]
    FileNotRegular { path: PathBuf },

    #[error("Invalid exclude pattern: {pattern} ({message})")]
    InvalidExcludePattern { pattern: String, message: String },

    #[error("Too many files to validate: {count} files found, limit is {limit}")]
    TooManyFiles { count: usize, limit: usize },

    #[error(transparent)]
    Other(anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Fix::is_insertion() tests =====

    #[test]
    fn test_fix_is_insertion_true_when_start_equals_end() {
        let fix = Fix::insert(10, "inserted text", "insert something", true);
        assert!(fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_false_when_replacement_empty() {
        // start == end but replacement is empty -> not an insertion
        let fix = Fix {
            start_byte: 5,
            end_byte: 5,
            replacement: String::new(),
            description: "no-op".to_string(),
            safe: true,
        };
        assert!(!fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_false_when_range_differs() {
        let fix = Fix::replace(0, 10, "replacement", "replace", true);
        assert!(!fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_at_zero() {
        let fix = Fix::insert(0, "prepend", "prepend text", true);
        assert!(fix.is_insertion());
    }

    // ===== Fix::is_deletion() tests =====

    #[test]
    fn test_fix_is_deletion_true_when_replacement_empty() {
        let fix = Fix::delete(5, 15, "remove text", true);
        assert!(fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_false_when_replacement_nonempty() {
        let fix = Fix::replace(5, 15, "new text", "replace", true);
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_false_when_start_equals_end() {
        // Empty range with empty replacement -> not a deletion
        let fix = Fix {
            start_byte: 5,
            end_byte: 5,
            replacement: String::new(),
            description: "no-op".to_string(),
            safe: true,
        };
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_single_byte() {
        let fix = Fix::delete(10, 11, "delete one byte", false);
        assert!(fix.is_deletion());
    }

    // ===== Fix constructors =====

    #[test]
    fn test_fix_replace_fields() {
        let fix = Fix::replace(2, 8, "new", "replace old", false);
        assert_eq!(fix.start_byte, 2);
        assert_eq!(fix.end_byte, 8);
        assert_eq!(fix.replacement, "new");
        assert_eq!(fix.description, "replace old");
        assert!(!fix.safe);
        assert!(!fix.is_insertion());
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_insert_fields() {
        let fix = Fix::insert(42, "text", "insert", true);
        assert_eq!(fix.start_byte, 42);
        assert_eq!(fix.end_byte, 42);
        assert_eq!(fix.replacement, "text");
        assert!(fix.safe);
    }

    #[test]
    fn test_fix_delete_fields() {
        let fix = Fix::delete(0, 100, "remove block", true);
        assert_eq!(fix.start_byte, 0);
        assert_eq!(fix.end_byte, 100);
        assert!(fix.replacement.is_empty());
        assert!(fix.safe);
    }

    // ===== Diagnostic builder methods =====

    #[test]
    fn test_diagnostic_with_suggestion() {
        let diag = Diagnostic::warning(PathBuf::from("test.md"), 1, 0, "AS-001", "test message")
            .with_suggestion("try this instead");

        assert_eq!(diag.suggestion, Some("try this instead".to_string()));
        assert_eq!(diag.level, DiagnosticLevel::Warning);
        assert_eq!(diag.message, "test message");
    }

    #[test]
    fn test_diagnostic_with_fix() {
        let fix = Fix::insert(0, "added", "add prefix", true);
        let diag = Diagnostic::error(PathBuf::from("a.md"), 5, 3, "CC-AG-001", "missing prefix")
            .with_fix(fix);

        assert!(diag.has_fixes());
        assert!(diag.has_safe_fixes());
        assert_eq!(diag.fixes.len(), 1);
        assert_eq!(diag.fixes[0].replacement, "added");
    }

    #[test]
    fn test_diagnostic_with_fixes_multiple() {
        let fixes = vec![
            Fix::insert(0, "a", "fix a", true),
            Fix::delete(10, 20, "fix b", false),
        ];
        let diag = Diagnostic::info(PathBuf::from("b.md"), 1, 0, "XML-001", "xml issue")
            .with_fixes(fixes);

        assert_eq!(diag.fixes.len(), 2);
        assert!(diag.has_fixes());
        // One safe, one unsafe
        assert!(diag.has_safe_fixes());
    }

    #[test]
    fn test_diagnostic_with_assumption() {
        let diag = Diagnostic::warning(PathBuf::from("c.md"), 2, 0, "CC-HK-001", "hook issue")
            .with_assumption("Assuming Claude Code >= 1.0.0");

        assert_eq!(
            diag.assumption,
            Some("Assuming Claude Code >= 1.0.0".to_string())
        );
    }

    #[test]
    fn test_diagnostic_builder_chaining() {
        let diag = Diagnostic::error(PathBuf::from("d.md"), 10, 5, "MCP-001", "mcp error")
            .with_suggestion("fix it")
            .with_fix(Fix::replace(0, 5, "fixed", "auto fix", true))
            .with_assumption("Assuming MCP protocol 2025-06-18");

        assert_eq!(diag.suggestion, Some("fix it".to_string()));
        assert_eq!(diag.fixes.len(), 1);
        assert!(diag.assumption.is_some());
        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.rule, "MCP-001");
    }

    #[test]
    fn test_diagnostic_no_fixes_by_default() {
        let diag =
            Diagnostic::warning(PathBuf::from("e.md"), 1, 0, "AS-005", "something wrong");

        assert!(!diag.has_fixes());
        assert!(!diag.has_safe_fixes());
        assert!(diag.fixes.is_empty());
        assert!(diag.suggestion.is_none());
        assert!(diag.assumption.is_none());
    }

    #[test]
    fn test_diagnostic_has_safe_fixes_false_when_all_unsafe() {
        let fixes = vec![
            Fix::delete(0, 5, "remove a", false),
            Fix::delete(10, 15, "remove b", false),
        ];
        let diag = Diagnostic::error(PathBuf::from("f.md"), 1, 0, "CC-AG-002", "agent error")
            .with_fixes(fixes);

        assert!(diag.has_fixes());
        assert!(!diag.has_safe_fixes());
    }

    // ===== Diagnostic level constructors =====

    #[test]
    fn test_diagnostic_error_level() {
        let diag = Diagnostic::error(PathBuf::from("x.md"), 1, 0, "R-001", "err");
        assert_eq!(diag.level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_diagnostic_warning_level() {
        let diag = Diagnostic::warning(PathBuf::from("x.md"), 1, 0, "R-002", "warn");
        assert_eq!(diag.level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_diagnostic_info_level() {
        let diag = Diagnostic::info(PathBuf::from("x.md"), 1, 0, "R-003", "info");
        assert_eq!(diag.level, DiagnosticLevel::Info);
    }

    // ===== Serialization roundtrip =====

    #[test]
    fn test_diagnostic_serialization_roundtrip() {
        let original = Diagnostic::error(
            PathBuf::from("project/CLAUDE.md"),
            42,
            7,
            "CC-AG-003",
            "Agent configuration issue",
        )
        .with_suggestion("Add the required field")
        .with_fix(Fix::insert(100, "new_field: true\n", "add field", true))
        .with_fix(Fix::delete(200, 250, "remove deprecated", false))
        .with_assumption("Assuming Claude Code >= 1.0.0");

        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Diagnostic =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.level, original.level);
        assert_eq!(deserialized.message, original.message);
        assert_eq!(deserialized.file, original.file);
        assert_eq!(deserialized.line, original.line);
        assert_eq!(deserialized.column, original.column);
        assert_eq!(deserialized.rule, original.rule);
        assert_eq!(deserialized.suggestion, original.suggestion);
        assert_eq!(deserialized.assumption, original.assumption);
        assert_eq!(deserialized.fixes.len(), 2);
        assert_eq!(deserialized.fixes[0].replacement, "new_field: true\n");
        assert!(deserialized.fixes[0].safe);
        assert!(deserialized.fixes[1].replacement.is_empty());
        assert!(!deserialized.fixes[1].safe);
    }

    #[test]
    fn test_fix_serialization_roundtrip() {
        let original = Fix::replace(10, 20, "replaced", "test fix", true);
        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Fix =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.start_byte, original.start_byte);
        assert_eq!(deserialized.end_byte, original.end_byte);
        assert_eq!(deserialized.replacement, original.replacement);
        assert_eq!(deserialized.description, original.description);
        assert_eq!(deserialized.safe, original.safe);
    }

    #[test]
    fn test_diagnostic_without_optional_fields_roundtrip() {
        let original =
            Diagnostic::info(PathBuf::from("simple.md"), 1, 0, "AS-001", "simple message");

        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Diagnostic =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.suggestion, None);
        assert_eq!(deserialized.assumption, None);
        assert!(deserialized.fixes.is_empty());
    }

    // ===== DiagnosticLevel ordering =====

    #[test]
    fn test_diagnostic_level_ordering() {
        assert!(DiagnosticLevel::Error < DiagnosticLevel::Warning);
        assert!(DiagnosticLevel::Warning < DiagnosticLevel::Info);
        assert!(DiagnosticLevel::Error < DiagnosticLevel::Info);
    }
}
