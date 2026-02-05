//! Validation rules

pub mod agent;
pub mod agents_md;
pub mod claude_md;
pub mod copilot;
pub mod cross_platform;
pub mod cursor;
pub mod hooks;
pub mod imports;
pub mod mcp;
pub mod plugin;
pub mod prompt;
pub mod skill;
pub mod xml;

use crate::{context::ValidatorContext, diagnostics::Diagnostic};
use std::path::Path;

/// Trait for file validators.
///
/// Validators implement this trait to provide validation logic for specific
/// file types. Each validator receives the file path, content, and a
/// [`ValidatorContext`] containing configuration and dependencies.
///
/// # Example
///
/// ```ignore
/// use agnix_core::{Diagnostic, ValidatorContext};
/// use std::path::Path;
///
/// struct MyValidator;
///
/// impl Validator for MyValidator {
///     fn validate(&self, path: &Path, content: &str, ctx: &ValidatorContext) -> Vec<Diagnostic> {
///         let mut diagnostics = Vec::new();
///         if ctx.is_rule_enabled("MY-001") && content.is_empty() {
///             diagnostics.push(Diagnostic::error(
///                 path.to_path_buf(), 1, 0, "MY-001", "File is empty".to_string()
///             ));
///         }
///         diagnostics
///     }
/// }
/// ```
pub trait Validator {
    /// Validate a file and return any diagnostics found.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file being validated
    /// * `content` - File content as a UTF-8 string
    /// * `ctx` - Validator context with config, filesystem, and project state
    ///
    /// # Returns
    ///
    /// A vector of diagnostics. Empty if no issues were found.
    fn validate(&self, path: &Path, content: &str, ctx: &ValidatorContext) -> Vec<Diagnostic>;
}
