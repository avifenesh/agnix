//! Diagnostic types and error reporting

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub type LintResult<T> = Result<T, LintError>;

/// An automatic fix for a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Structured metadata about the rule that triggered a diagnostic.
///
/// Populated automatically from `agnix-rules` build-time data when using
/// the `Diagnostic::error()`, `warning()`, or `info()` constructors, or
/// manually via `Diagnostic::with_metadata()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Rule category (e.g., "agent-skills", "claude-code-hooks").
    pub category: String,
    /// Rule severity from the rules catalog (e.g., "HIGH", "MEDIUM", "LOW").
    pub severity: String,
    /// Tool this rule specifically applies to (e.g., "claude-code", "cursor").
    /// `None` for generic rules that apply to all tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to_tool: Option<String>,
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
    /// Structured metadata about the rule (category, severity, tool).
    ///
    /// Auto-populated from `agnix-rules` at construction time when using the
    /// `error()`, `warning()`, or `info()` constructors. Can also be set
    /// manually via `with_metadata()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RuleMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

/// Build a `RuleMetadata` from the compile-time rules catalog.
fn lookup_rule_metadata(rule_id: &str) -> Option<RuleMetadata> {
    agnix_rules::get_rule_metadata(rule_id).map(|(category, severity, tool)| RuleMetadata {
        category: category.to_string(),
        severity: severity.to_string(),
        applies_to_tool: if tool.is_empty() {
            None
        } else {
            Some(tool.to_string())
        },
    })
}

impl Diagnostic {
    pub fn error(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
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
            metadata,
        }
    }

    pub fn warning(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
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
            metadata,
        }
    }

    pub fn info(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
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
            metadata,
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

    /// Set structured rule metadata on this diagnostic
    pub fn with_metadata(mut self, metadata: RuleMetadata) -> Self {
        self.metadata = Some(metadata);
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

    // ===== Auto-populate metadata tests =====

    #[test]
    fn test_error_auto_populates_metadata_for_known_rule() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(
            diag.metadata.is_some(),
            "Metadata should be auto-populated for known rule AS-001"
        );
        let meta = diag.metadata.unwrap();
        assert_eq!(meta.category, "agent-skills");
        assert_eq!(meta.severity, "HIGH");
        assert!(
            meta.applies_to_tool.is_none(),
            "AS-001 is generic, should have no tool"
        );
    }

    #[test]
    fn test_warning_auto_populates_metadata() {
        let diag = Diagnostic::warning(PathBuf::from("test.md"), 1, 1, "CC-HK-001", "Test");
        assert!(diag.metadata.is_some());
        let meta = diag.metadata.unwrap();
        assert_eq!(meta.applies_to_tool, Some("claude-code".to_string()));
    }

    #[test]
    fn test_info_auto_populates_metadata() {
        let diag = Diagnostic::info(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(diag.metadata.is_some());
    }

    #[test]
    fn test_unknown_rule_has_no_metadata() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "UNKNOWN-999", "Test");
        assert!(
            diag.metadata.is_none(),
            "Unknown rules should not have metadata"
        );
    }

    #[test]
    fn test_lookup_rule_metadata_empty_string() {
        let meta = lookup_rule_metadata("");
        assert!(meta.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_lookup_rule_metadata_special_characters() {
        let meta = lookup_rule_metadata("@#$%^&*()");
        assert!(
            meta.is_none(),
            "Rule ID with special characters should return None"
        );
    }

    // ===== Builder method tests =====

    #[test]
    fn test_with_metadata_builder() {
        let meta = RuleMetadata {
            category: "custom".to_string(),
            severity: "LOW".to_string(),
            applies_to_tool: Some("my-tool".to_string()),
        };
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "UNKNOWN-999", "Test")
            .with_metadata(meta.clone());
        assert_eq!(diag.metadata, Some(meta));
    }

    #[test]
    fn test_with_metadata_overrides_auto_populated() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(diag.metadata.is_some());

        let custom_meta = RuleMetadata {
            category: "custom".to_string(),
            severity: "LOW".to_string(),
            applies_to_tool: None,
        };
        let diag = diag.with_metadata(custom_meta.clone());
        assert_eq!(diag.metadata, Some(custom_meta));
    }

    // ===== Serde roundtrip tests =====

    #[test]
    fn test_rule_metadata_serde_roundtrip() {
        let meta = RuleMetadata {
            category: "agent-skills".to_string(),
            severity: "HIGH".to_string(),
            applies_to_tool: Some("claude-code".to_string()),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: RuleMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, deserialized);
    }

    #[test]
    fn test_rule_metadata_serde_none_tool_omitted() {
        let meta = RuleMetadata {
            category: "agent-skills".to_string(),
            severity: "HIGH".to_string(),
            applies_to_tool: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(
            !json.contains("applies_to_tool"),
            "None tool should be omitted via skip_serializing_if"
        );
    }

    #[test]
    fn test_diagnostic_serde_roundtrip_with_metadata() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 10, 5, "AS-001", "Test error");
        let json = serde_json::to_string(&diag).unwrap();
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata, diag.metadata);
        assert_eq!(deserialized.rule, "AS-001");
    }

    #[test]
    fn test_diagnostic_serde_roundtrip_without_metadata() {
        let diag = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "UNKNOWN".to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
            metadata: None,
        };
        let json = serde_json::to_string(&diag).unwrap();
        assert!(
            !json.contains("metadata"),
            "None metadata should be omitted"
        );
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();
        assert!(deserialized.metadata.is_none());
    }

    #[test]
    fn test_diagnostic_deserialize_without_metadata_field() {
        // Simulate old JSON that doesn't have the metadata field at all
        let json = r#"{
            "level": "Error",
            "message": "Test",
            "file": "test.md",
            "line": 1,
            "column": 1,
            "rule": "AS-001",
            "fixes": []
        }"#;
        let diag: Diagnostic = serde_json::from_str(json).unwrap();
        assert!(
            diag.metadata.is_none(),
            "Missing metadata field should deserialize as None"
        );
    }

    #[test]
    fn test_diagnostic_manual_metadata_serde_roundtrip() {
        let manual_metadata = RuleMetadata {
            category: "custom-category".to_string(),
            severity: "MEDIUM".to_string(),
            applies_to_tool: Some("custom-tool".to_string()),
        };

        let diag = Diagnostic::error(PathBuf::from("test.md"), 5, 10, "CUSTOM-001", "Custom error")
            .with_metadata(manual_metadata.clone());

        // Serialize to JSON
        let json = serde_json::to_string(&diag).unwrap();

        // Deserialize back
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();

        // Verify metadata is preserved
        assert_eq!(deserialized.metadata, Some(manual_metadata));
        assert_eq!(deserialized.rule, "CUSTOM-001");
        assert_eq!(deserialized.message, "Custom error");
    }
}
