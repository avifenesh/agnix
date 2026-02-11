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
