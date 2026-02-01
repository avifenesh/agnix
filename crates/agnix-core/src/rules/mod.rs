//! Validation rules

pub mod agent;
pub mod claude_md;
pub mod cross_platform;
pub mod hooks;
pub mod imports;
pub mod mcp;
pub mod plugin;
pub mod prompt;
pub mod skill;
pub mod xml;

use crate::{config::LintConfig, diagnostics::Diagnostic};
use std::path::Path;

/// Trait for file validators
pub trait Validator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;
}
