//! Filesystem abstraction for dependency injection in validators.
//!
//! This module provides a `FileSystem` trait that abstracts filesystem operations,
//! enabling validators to be tested without real filesystem access.
//!
//! ## Usage
//!
//! Production code uses `RealFileSystem` which delegates to the actual filesystem:
//!
//! ```ignore
//! let fs = RealFileSystem;
//! let content = fs.read_file(Path::new("CLAUDE.md"))?;
//! ```
//!
//! Test code can use `MockFileSystem` to simulate filesystem state:
//!
//! ```ignore
//! let mut mock = MockFileSystem::new();
//! mock.add_file("CLAUDE.md", "# Project\n@import config.md");
//! let content = mock.read_file(Path::new("CLAUDE.md"))?;
//! ```

use crate::diagnostics::LintResult;
use std::io;
use std::path::{Path, PathBuf};

/// Trait abstracting filesystem operations for validators.
///
/// This trait enables dependency injection, allowing validators to be tested
/// with mock filesystems instead of requiring real files on disk.
///
/// All methods are designed to be safe and avoid security issues:
/// - `read_file` uses the same security checks as `safe_read_file`
/// - Path operations use standard library functions
pub trait FileSystem: Send + Sync + std::fmt::Debug {
    /// Read the contents of a file as a UTF-8 string.
    ///
    /// This method should perform security checks similar to `safe_read_file`:
    /// - Reject symlinks
    /// - Reject non-regular files
    /// - Enforce size limits
    fn read_file(&self, path: &Path) -> LintResult<String>;

    /// Check if a path exists (file or directory).
    fn exists(&self, path: &Path) -> bool;

    /// Canonicalize a path (resolve symlinks and normalize).
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Check if a path is a file.
    fn is_file(&self, path: &Path) -> bool;

    /// Check if a path is a directory.
    fn is_dir(&self, path: &Path) -> bool;
}

/// Real filesystem implementation that delegates to std::fs and file_utils.
///
/// This is the production implementation used for actual file operations.
/// It maintains all security guarantees from `safe_read_file`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_file(&self, path: &Path) -> LintResult<String> {
        crate::file_utils::safe_read_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}

#[cfg(test)]
pub mod mock {
    //! Mock filesystem for testing validators without real filesystem access.

    use super::*;
    use crate::diagnostics::LintError;
    use std::collections::{HashMap, HashSet};

    /// Mock filesystem for unit testing.
    ///
    /// Provides a fully in-memory filesystem that can be pre-populated
    /// with test files and directories.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut mock = MockFileSystem::new();
    /// mock.add_file("project/CLAUDE.md", "# Instructions");
    /// mock.add_dir("project/.claude");
    ///
    /// assert!(mock.exists(Path::new("project/CLAUDE.md")));
    /// assert!(mock.is_dir(Path::new("project/.claude")));
    /// ```
    #[derive(Debug, Default)]
    pub struct MockFileSystem {
        /// Files mapped by their canonical path to content.
        files: HashMap<PathBuf, String>,
        /// Directories that exist (empty directories are tracked here).
        dirs: HashSet<PathBuf>,
    }

    impl MockFileSystem {
        /// Create a new empty mock filesystem.
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a file with the given content.
        ///
        /// Parent directories are automatically created.
        pub fn add_file(&mut self, path: impl AsRef<Path>, content: impl Into<String>) {
            let path = normalize_path(path.as_ref());
            // Ensure parent directories exist
            if let Some(parent) = path.parent() {
                self.add_dir_recursive(parent);
            }
            self.files.insert(path, content.into());
        }

        /// Add an empty directory.
        ///
        /// Parent directories are automatically created.
        pub fn add_dir(&mut self, path: impl AsRef<Path>) {
            self.add_dir_recursive(path.as_ref());
        }

        fn add_dir_recursive(&mut self, path: &Path) {
            let path = normalize_path(path);
            if path.as_os_str().is_empty() {
                return;
            }
            self.dirs.insert(path.clone());
            if let Some(parent) = path.parent() {
                self.add_dir_recursive(parent);
            }
        }
    }

    impl FileSystem for MockFileSystem {
        fn read_file(&self, path: &Path) -> LintResult<String> {
            let normalized = normalize_path(path);
            self.files
                .get(&normalized)
                .cloned()
                .ok_or_else(|| LintError::FileRead {
                    path: path.to_path_buf(),
                    source: io::Error::new(io::ErrorKind::NotFound, "file not found in mock"),
                })
        }

        fn exists(&self, path: &Path) -> bool {
            let normalized = normalize_path(path);
            self.files.contains_key(&normalized) || self.dirs.contains(&normalized)
        }

        fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
            // For mock filesystem, just normalize the path
            // In tests, we don't resolve actual symlinks
            Ok(normalize_path(path))
        }

        fn is_file(&self, path: &Path) -> bool {
            let normalized = normalize_path(path);
            self.files.contains_key(&normalized)
        }

        fn is_dir(&self, path: &Path) -> bool {
            let normalized = normalize_path(path);
            self.dirs.contains(&normalized)
        }
    }

    /// Normalize a path for consistent comparison.
    ///
    /// This handles differences in path separators across platforms
    /// and removes redundant components like `.` and `..`.
    fn normalize_path(path: &Path) -> PathBuf {
        use std::path::Component;

        let mut result = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    result.pop();
                }
                Component::Normal(s) => result.push(s),
                Component::RootDir => result.push(component.as_os_str()),
                Component::Prefix(p) => result.push(p.as_os_str()),
            }
        }

        // Ensure forward slashes for cross-platform consistency in tests
        #[cfg(windows)]
        {
            PathBuf::from(result.to_string_lossy().replace('\\', "/"))
        }
        #[cfg(not(windows))]
        {
            result
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_add_and_read_file() {
            let mut mock = MockFileSystem::new();
            mock.add_file("project/CLAUDE.md", "# Instructions");

            let content = mock.read_file(Path::new("project/CLAUDE.md")).unwrap();
            assert_eq!(content, "# Instructions");
        }

        #[test]
        fn test_mock_file_not_found() {
            let mock = MockFileSystem::new();
            let result = mock.read_file(Path::new("nonexistent.md"));
            assert!(result.is_err());
        }

        #[test]
        fn test_mock_exists() {
            let mut mock = MockFileSystem::new();
            mock.add_file("file.md", "content");
            mock.add_dir("empty_dir");

            assert!(mock.exists(Path::new("file.md")));
            assert!(mock.exists(Path::new("empty_dir")));
            assert!(!mock.exists(Path::new("nonexistent")));
        }

        #[test]
        fn test_mock_is_file_and_is_dir() {
            let mut mock = MockFileSystem::new();
            mock.add_file("file.md", "content");
            mock.add_dir("dir");

            assert!(mock.is_file(Path::new("file.md")));
            assert!(!mock.is_dir(Path::new("file.md")));

            assert!(mock.is_dir(Path::new("dir")));
            assert!(!mock.is_file(Path::new("dir")));
        }

        #[test]
        fn test_mock_parent_dirs_created() {
            let mut mock = MockFileSystem::new();
            mock.add_file("a/b/c/file.md", "content");

            assert!(mock.is_dir(Path::new("a")));
            assert!(mock.is_dir(Path::new("a/b")));
            assert!(mock.is_dir(Path::new("a/b/c")));
            assert!(mock.is_file(Path::new("a/b/c/file.md")));
        }

        #[test]
        fn test_mock_canonicalize() {
            let mock = MockFileSystem::new();
            let result = mock.canonicalize(Path::new("./a/../b/./c"));
            assert!(result.is_ok());
            let canonical = result.unwrap();
            // Should normalize to "b/c"
            assert!(canonical.ends_with("c"));
            assert!(!canonical.to_string_lossy().contains(".."));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_real_fs_read_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        fs::write(&file_path, "Hello, world!").unwrap();

        let fs = RealFileSystem;
        let content = fs.read_file(&file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }

    #[test]
    fn test_real_fs_exists() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("exists.md");
        fs::write(&file_path, "content").unwrap();

        let fs = RealFileSystem;
        assert!(fs.exists(&file_path));
        assert!(!fs.exists(&temp.path().join("nonexistent.md")));
    }

    #[test]
    fn test_real_fs_is_file_and_dir() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.md");
        let dir_path = temp.path().join("dir");
        fs::write(&file_path, "content").unwrap();
        fs::create_dir(&dir_path).unwrap();

        let fs = RealFileSystem;
        assert!(fs.is_file(&file_path));
        assert!(!fs.is_dir(&file_path));
        assert!(fs.is_dir(&dir_path));
        assert!(!fs.is_file(&dir_path));
    }

    #[test]
    fn test_real_fs_canonicalize() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.md");
        fs::write(&file_path, "content").unwrap();

        let fs = RealFileSystem;
        let canonical = fs.canonicalize(&file_path).unwrap();
        assert!(canonical.is_absolute());
    }
}
