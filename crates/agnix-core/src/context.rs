//! Validator context for dependency injection.
//!
//! This module provides `ValidatorContext`, a struct that bundles all dependencies
//! needed by validators, enabling clean dependency injection and testability.
//!
//! ## Motivation
//!
//! Before `ValidatorContext`, validators received only a `&LintConfig` and had to:
//! - Access the filesystem directly via `std::fs` or `safe_read_file`
//! - Retrieve `root_dir` and `import_cache` from the config
//!
//! This made validators hard to unit test without real filesystem fixtures.
//!
//! With `ValidatorContext`, validators receive all dependencies through a single
//! struct, making it easy to inject mocks for testing.
//!
//! ## Example
//!
//! ```ignore
//! // Production code
//! let ctx = ValidatorContext::new(&config, &RealFileSystem);
//!
//! // Test code
//! let mock_fs = MockFileSystem::new();
//! let ctx = ValidatorContext::new(&config, &mock_fs);
//! ```

use crate::config::LintConfig;
use crate::fs::FileSystem;
use crate::parsers::ImportCache;
use std::path::Path;

/// Context passed to validators containing all dependencies.
///
/// This struct bundles the configuration, filesystem abstraction, and optional
/// project-level state (root directory, import cache) needed by validators.
///
/// # Lifetimes
///
/// - `'a`: Lifetime of the config and filesystem references. The context
///   borrows these for the duration of validation.
///
/// # Thread Safety
///
/// `ValidatorContext` is `Send + Sync` when its contained references are,
/// which enables parallel validation via rayon.
#[derive(Debug)]
pub struct ValidatorContext<'a> {
    /// Lint configuration containing rule settings and options.
    pub config: &'a LintConfig,

    /// Filesystem abstraction for reading files and checking paths.
    /// Use `RealFileSystem` in production or `MockFileSystem` in tests.
    pub fs: &'a dyn FileSystem,

    /// Root directory for project-level validation.
    /// Used to resolve relative paths and enforce path boundaries.
    pub root_dir: Option<&'a Path>,

    /// Shared import cache for project-level validation.
    /// Enables validators to share parsed import data across files,
    /// avoiding redundant parsing during import chain traversal.
    pub import_cache: Option<&'a ImportCache>,
}

impl<'a> ValidatorContext<'a> {
    /// Create a new validator context with the given config and filesystem.
    ///
    /// This creates a minimal context without project-level state (root_dir,
    /// import_cache). Use `with_root_dir()` and `with_import_cache()` to add
    /// project-level state.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ctx = ValidatorContext::new(&config, &RealFileSystem);
    /// ```
    pub fn new(config: &'a LintConfig, fs: &'a dyn FileSystem) -> Self {
        Self {
            config,
            fs,
            root_dir: None,
            import_cache: None,
        }
    }

    /// Set the root directory for project-level validation.
    ///
    /// The root directory is used to:
    /// - Resolve relative import paths
    /// - Enforce path boundary checks (prevent path traversal)
    /// - Locate project-level files like package.json
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ctx = ValidatorContext::new(&config, &fs)
    ///     .with_root_dir(project_root);
    /// ```
    pub fn with_root_dir(mut self, root_dir: &'a Path) -> Self {
        self.root_dir = Some(root_dir);
        self
    }

    /// Set the import cache for project-level validation.
    ///
    /// The import cache enables sharing parsed import data across files,
    /// improving performance during project-level validation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cache = ImportCache::new();
    /// let ctx = ValidatorContext::new(&config, &fs)
    ///     .with_import_cache(&cache);
    /// ```
    pub fn with_import_cache(mut self, cache: &'a ImportCache) -> Self {
        self.import_cache = Some(cache);
        self
    }

    /// Check if a rule is enabled in the configuration.
    ///
    /// This is a convenience method that delegates to `config.is_rule_enabled()`.
    #[inline]
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        self.config.is_rule_enabled(rule_id)
    }

    /// Get the root directory, falling back to the config's root_dir if not set.
    ///
    /// Returns `None` if neither the context nor config has a root directory set.
    pub fn get_root_dir(&self) -> Option<&Path> {
        self.root_dir.or(self.config.root_dir.as_deref())
    }

    /// Get the import cache, falling back to the config's import_cache if not set.
    ///
    /// Returns `None` if neither the context nor config has an import cache set.
    pub fn get_import_cache(&self) -> Option<&ImportCache> {
        self.import_cache.or(self.config.import_cache.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::RealFileSystem;

    #[test]
    fn test_context_new() {
        let config = LintConfig::default();
        let fs = RealFileSystem;
        let ctx = ValidatorContext::new(&config, &fs);

        assert!(ctx.root_dir.is_none());
        assert!(ctx.import_cache.is_none());
    }

    #[test]
    fn test_context_with_root_dir() {
        let config = LintConfig::default();
        let fs = RealFileSystem;
        let root = Path::new("/project");

        let ctx = ValidatorContext::new(&config, &fs).with_root_dir(root);

        assert_eq!(ctx.root_dir, Some(root));
        assert_eq!(ctx.get_root_dir(), Some(root));
    }

    #[test]
    fn test_context_is_rule_enabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AS-001".to_string()];

        let fs = RealFileSystem;
        let ctx = ValidatorContext::new(&config, &fs);

        assert!(!ctx.is_rule_enabled("AS-001"));
        assert!(ctx.is_rule_enabled("AS-002"));
    }

    #[test]
    fn test_context_get_root_dir_fallback() {
        let mut config = LintConfig::default();
        config.root_dir = Some("/config/root".into());

        let fs = RealFileSystem;
        let ctx = ValidatorContext::new(&config, &fs);

        // Falls back to config's root_dir
        assert_eq!(ctx.get_root_dir(), Some(Path::new("/config/root")));

        // Context's root_dir takes precedence
        let ctx2 = ctx.with_root_dir(Path::new("/context/root"));
        assert_eq!(ctx2.get_root_dir(), Some(Path::new("/context/root")));
    }

    #[test]
    fn test_context_builder_pattern() {
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};

        let config = LintConfig::default();
        let fs = RealFileSystem;
        let root = Path::new("/project");
        let cache: ImportCache = Arc::new(RwLock::new(HashMap::new()));

        let ctx = ValidatorContext::new(&config, &fs)
            .with_root_dir(root)
            .with_import_cache(&cache);

        assert_eq!(ctx.root_dir, Some(root));
        assert!(ctx.import_cache.is_some());
    }
}
