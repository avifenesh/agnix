//! Diagnostic types and error reporting

#![allow(unused_assignments)] // LintError fields used by miette derive macros

use miette::{Diagnostic as MietteDiagnostic, SourceSpan};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    pub fn error(file: PathBuf, line: usize, column: usize, rule: &str, message: String) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message,
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
        }
    }

    pub fn warning(file: PathBuf, line: usize, column: usize, rule: &str, message: String) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message,
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
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
#[derive(Error, Debug, MietteDiagnostic)]
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
    #[diagnostic(
        code(agnix::file_symlink),
        help("Symlinks are not supported for security reasons")
    )]
    FileSymlink { path: PathBuf },

    #[error("File too large: {path} ({size} bytes, limit {limit} bytes)")]
    #[diagnostic(
        code(agnix::file_too_big),
        help("Files larger than the configured size limit are not supported")
    )]
    FileTooBig {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("Not a regular file: {path}")]
    #[diagnostic(
        code(agnix::file_not_regular),
        help("Only regular files are supported (not directories, FIFOs, or device nodes)")
    )]
    FileNotRegular { path: PathBuf },

    #[error("Failed to parse YAML frontmatter")]
    #[diagnostic(code(agnix::yaml_parse), help("Check YAML syntax between --- markers"))]
    YamlParse {
        #[source_code]
        src: String,
        #[label("Parse error here")]
        span: SourceSpan,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("Failed to parse JSON")]
    #[diagnostic(code(agnix::json_parse), help("Check JSON syntax"))]
    JsonParse {
        #[source_code]
        src: String,
        #[label("Parse error here")]
        span: SourceSpan,
        #[source]
        source: serde_json::Error,
    },

    #[error("Generic instruction detected")]
    #[diagnostic(
        code(agnix::generic_instruction),
        help("Remove generic instructions. Claude already knows to be helpful.")
    )]
    GenericInstruction {
        #[source_code]
        src: String,
        #[label("This instruction is redundant")]
        span: SourceSpan,
    },

    #[error("Invalid skill name: {name}")]
    #[diagnostic(
        code(agnix::invalid_name),
        help("Use lowercase letters and hyphens only (e.g., 'code-review')")
    )]
    InvalidName {
        name: String,
        #[source_code]
        src: String,
        #[label("Must be lowercase with hyphens")]
        span: SourceSpan,
    },

    #[error("Missing required field: {field}")]
    #[diagnostic(code(agnix::missing_field))]
    MissingField {
        field: String,
        #[source_code]
        src: String,
        #[label("Add '{field}' field here")]
        span: SourceSpan,
    },

    #[error("XML tag mismatch")]
    #[diagnostic(
        code(agnix::xml_mismatch),
        help("Ensure all XML tags are properly closed")
    )]
    XmlMismatch {
        #[source_code]
        src: String,
        #[label("Unclosed or mismatched tag")]
        span: SourceSpan,
        open_tag: String,
    },

    #[error("Import not found: {path}")]
    #[diagnostic(code(agnix::import_not_found), help("Check the file path exists"))]
    ImportNotFound {
        path: String,
        #[source_code]
        src: String,
        #[label("File not found")]
        span: SourceSpan,
    },

    #[error("Unknown tool: {tool}")]
    #[diagnostic(
        code(agnix::unknown_tool),
        help("Check valid tool names for your target")
    )]
    UnknownTool {
        tool: String,
        #[source_code]
        src: String,
        #[label("Unknown tool")]
        span: SourceSpan,
    },

    #[error(transparent)]
    Other(anyhow::Error),
}
