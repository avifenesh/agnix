//! File parsers for different config formats
//!
//! This module is internal to agnix-core. Some items may not be used
//! directly but are preserved for potential future use or extensibility.

#![allow(dead_code)]

pub mod frontmatter;
pub mod json;
pub mod markdown;

// Re-export Import for use in ImportCache type alias
pub use markdown::Import;

/// Shared import cache for project-level validation.
///
/// This cache stores parsed imports for each file, allowing multiple validators
/// to share parse results and avoiding redundant parsing of the same files
/// during import chain traversal.
///
/// The cache is thread-safe and uses an `Arc<RwLock<HashMap>>` pattern to allow
/// concurrent reads with exclusive writes. When used in project validation,
/// a single cache instance is shared across all file validations.
pub type ImportCache =
    std::sync::Arc<std::sync::RwLock<std::collections::HashMap<std::path::PathBuf, Vec<Import>>>>;
