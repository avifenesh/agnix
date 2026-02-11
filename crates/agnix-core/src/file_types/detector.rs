//! Extensible file type detection via the chain-of-responsibility pattern.
//!
//! Third-party code can implement [`FileTypeDetector`] and register it in a
//! [`FileTypeDetectorChain`] to override or extend the built-in detection
//! logic without modifying agnix-core.

use std::path::Path;

use super::detection::detect_file_type;
use super::types::FileType;

/// A single file type detection strategy.
///
/// Implementors inspect a path and return `Some(FileType)` when they
/// recognise the file, or `None` to defer to the next detector in the
/// chain.
///
/// # Object Safety
///
/// This trait is object-safe so that detectors can be stored as
/// `Box<dyn FileTypeDetector>` in a [`FileTypeDetectorChain`].
pub trait FileTypeDetector: Send + Sync {
    /// Attempt to classify `path`. Return `None` to defer.
    fn detect(&self, path: &Path) -> Option<FileType>;

    /// Human-readable name for this detector (used in diagnostics/logging).
    ///
    /// Defaults to the short (unqualified) type name.
    fn name(&self) -> &str {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }
}

/// Built-in detector that wraps [`detect_file_type`].
///
/// This is the detector used by default in every
/// [`FileTypeDetectorChain::with_builtin`] chain.
pub struct BuiltinDetector;

impl FileTypeDetector for BuiltinDetector {
    fn detect(&self, path: &Path) -> Option<FileType> {
        let ft = detect_file_type(path);
        // The builtin always returns *something*, even Unknown.
        // Wrap it in Some so the chain terminates.
        Some(ft)
    }

    fn name(&self) -> &str {
        "BuiltinDetector"
    }
}

/// Ordered chain of [`FileTypeDetector`] implementations.
///
/// Detectors are consulted in order. The first `Some(FileType)` wins.
/// Use [`with_builtin`](FileTypeDetectorChain::with_builtin) to get a chain
/// that falls back to the built-in detection logic.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use agnix_core::file_types::{FileType, FileTypeDetector, FileTypeDetectorChain};
///
/// struct ForceMcp;
/// impl FileTypeDetector for ForceMcp {
///     fn detect(&self, path: &Path) -> Option<FileType> {
///         if path.extension().and_then(|e| e.to_str()) == Some("mcp") {
///             Some(FileType::Mcp)
///         } else {
///             None
///         }
///     }
/// }
///
/// let chain = FileTypeDetectorChain::with_builtin()
///     .prepend(ForceMcp);
///
/// assert_eq!(chain.detect(Path::new("server.mcp")), Some(FileType::Mcp));
/// ```
pub struct FileTypeDetectorChain {
    detectors: Vec<Box<dyn FileTypeDetector>>,
}

impl FileTypeDetectorChain {
    /// Create an empty chain with no detectors.
    ///
    /// An empty chain always returns `None` from [`detect`](Self::detect).
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    /// Create a chain pre-populated with the [`BuiltinDetector`].
    ///
    /// This is the recommended starting point: custom detectors can then
    /// be [`prepend`](Self::prepend)ed to override the built-in logic for
    /// specific paths.
    pub fn with_builtin() -> Self {
        Self {
            detectors: vec![Box::new(BuiltinDetector)],
        }
    }

    /// Add a detector to the **front** of the chain (highest priority).
    ///
    /// Consumes and returns `self` for builder-style chaining.
    pub fn prepend(mut self, detector: impl FileTypeDetector + 'static) -> Self {
        self.detectors.insert(0, Box::new(detector));
        self
    }

    /// Add a detector to the **end** of the chain (lowest priority).
    ///
    /// Consumes and returns `self` for builder-style chaining.
    pub fn push(mut self, detector: impl FileTypeDetector + 'static) -> Self {
        self.detectors.push(Box::new(detector));
        self
    }

    /// Run the chain: consult each detector in order and return the first
    /// `Some(FileType)`.
    ///
    /// Returns `None` if no detector matched (only possible with an empty
    /// chain or a chain where every detector deferred).
    pub fn detect(&self, path: &Path) -> Option<FileType> {
        for detector in &self.detectors {
            if let Some(ft) = detector.detect(path) {
                return Some(ft);
            }
        }
        None
    }

    /// Return the number of detectors in the chain.
    pub fn len(&self) -> usize {
        self.detectors.len()
    }

    /// Return `true` if the chain contains no detectors.
    pub fn is_empty(&self) -> bool {
        self.detectors.is_empty()
    }
}

impl Default for FileTypeDetectorChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- BuiltinDetector ----

    #[test]
    fn builtin_detector_recognises_skill() {
        let detector = BuiltinDetector;
        assert_eq!(
            detector.detect(Path::new("SKILL.md")),
            Some(FileType::Skill)
        );
    }

    #[test]
    fn builtin_detector_returns_unknown_for_non_config() {
        let detector = BuiltinDetector;
        assert_eq!(
            detector.detect(Path::new("main.rs")),
            Some(FileType::Unknown)
        );
    }

    #[test]
    fn builtin_detector_name() {
        assert_eq!(BuiltinDetector.name(), "BuiltinDetector");
    }

    // ---- FileTypeDetectorChain ----

    #[test]
    fn empty_chain_returns_none() {
        let chain = FileTypeDetectorChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.detect(Path::new("anything")), None);
    }

    #[test]
    fn with_builtin_has_one_detector() {
        let chain = FileTypeDetectorChain::with_builtin();
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn with_builtin_falls_back_correctly() {
        let chain = FileTypeDetectorChain::with_builtin();
        assert_eq!(
            chain.detect(Path::new("SKILL.md")),
            Some(FileType::Skill)
        );
        assert_eq!(
            chain.detect(Path::new("main.rs")),
            Some(FileType::Unknown)
        );
    }

    // ---- Custom detector priority ----

    struct AlwaysMcp;
    impl FileTypeDetector for AlwaysMcp {
        fn detect(&self, _path: &Path) -> Option<FileType> {
            Some(FileType::Mcp)
        }
        fn name(&self) -> &str {
            "AlwaysMcp"
        }
    }

    struct NeverMatch;
    impl FileTypeDetector for NeverMatch {
        fn detect(&self, _path: &Path) -> Option<FileType> {
            None
        }
    }

    #[test]
    fn prepend_takes_priority_over_builtin() {
        let chain = FileTypeDetectorChain::with_builtin().prepend(AlwaysMcp);
        assert_eq!(chain.len(), 2);

        // AlwaysMcp overrides everything
        assert_eq!(
            chain.detect(Path::new("SKILL.md")),
            Some(FileType::Mcp)
        );
    }

    #[test]
    fn push_runs_after_builtin() {
        let chain = FileTypeDetectorChain::with_builtin().push(AlwaysMcp);
        assert_eq!(chain.len(), 2);

        // Builtin matches first for SKILL.md
        assert_eq!(
            chain.detect(Path::new("SKILL.md")),
            Some(FileType::Skill)
        );
    }

    #[test]
    fn never_match_defers_to_next() {
        let chain = FileTypeDetectorChain::new()
            .push(NeverMatch)
            .push(AlwaysMcp);

        // NeverMatch defers, AlwaysMcp catches
        assert_eq!(
            chain.detect(Path::new("anything")),
            Some(FileType::Mcp)
        );
    }

    #[test]
    fn chain_with_only_never_match_returns_none() {
        let chain = FileTypeDetectorChain::new().push(NeverMatch);
        assert_eq!(chain.detect(Path::new("anything")), None);
    }

    #[test]
    fn multiple_prepends_maintain_order() {
        struct ReturnSkill;
        impl FileTypeDetector for ReturnSkill {
            fn detect(&self, _path: &Path) -> Option<FileType> {
                Some(FileType::Skill)
            }
        }

        // Prepend order: AlwaysMcp first, then ReturnSkill before it
        let chain = FileTypeDetectorChain::new()
            .push(AlwaysMcp)
            .prepend(ReturnSkill);

        // ReturnSkill is at front, so Skill wins
        assert_eq!(
            chain.detect(Path::new("anything")),
            Some(FileType::Skill)
        );
    }

    // ---- Send + Sync bounds ----

    #[test]
    fn detector_trait_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BuiltinDetector>();
    }

    #[test]
    fn chain_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FileTypeDetectorChain>();
    }

    // ---- Default trait name ----

    #[test]
    fn default_trait_name_uses_type_name() {
        let detector = NeverMatch;
        // The default name() implementation returns the short type name
        assert_eq!(detector.name(), "NeverMatch");
    }
}
