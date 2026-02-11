//! File type detection for validator dispatch.
//!
//! This module provides:
//!
//! - [`FileType`] -- enum of all recognised configuration file types
//! - [`detect_file_type`] -- built-in path-based detection function
//! - [`FileTypeDetector`] -- trait for custom detection strategies
//! - [`FileTypeDetectorChain`] -- chain-of-responsibility dispatcher
//!
//! ## Extending detection
//!
//! Implement [`FileTypeDetector`] and register it via
//! [`FileTypeDetectorChain::with_builtin().prepend(your_detector)`](FileTypeDetectorChain::prepend)
//! to override detection for specific paths without modifying agnix-core.
//!
//! **Stability: unstable** -- interface may change on minor releases.

mod detection;
mod detector;
mod types;

// Primary re-exports (backward-compatible with the old single-file module)
pub use detection::detect_file_type;
pub use types::FileType;

// New public API
pub use detection::{
    DOCUMENTATION_DIRECTORIES, EXCLUDED_FILENAMES, EXCLUDED_PARENT_DIRECTORIES,
};
pub use detector::{BuiltinDetector, FileTypeDetector, FileTypeDetectorChain};

