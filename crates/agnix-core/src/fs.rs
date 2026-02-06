//! FileSystem abstraction for testability
//!
//! This module provides a `FileSystem` trait that abstracts file system operations,
//! enabling validators to be unit tested with `MockFileSystem` instead of requiring
//! real temp files.
//!
//! ## Usage
//!
//! For production code, use `RealFileSystem` which delegates to `std::fs` and
//! the safe file reading utilities in `file_utils`.
//!
//! For tests, use `MockFileSystem` which provides an in-memory HashMap-based
//! storage with `RwLock` for thread safety.
//!
//! ## Security
//!
//! ### Symlink Handling
//!
//! Both implementations reject symlinks in `read_to_string()` to prevent path
//! traversal attacks. The `MockFileSystem` also implements depth limiting in
//! `metadata()` and `canonicalize()` to detect symlink loops.
//!
//! ### TOCTOU (Time-of-Check-Time-of-Use)
//!
//! There is an inherent TOCTOU window between checking file properties and
//! reading content. An attacker with local filesystem access could potentially
//! replace a regular file with a symlink between the check and read operations.
//! This is acceptable for a linter because:
//!
//! 1. The attack requires local filesystem access
//! 2. The impact is limited to reading unexpected content
//! 3. Eliminating TOCTOU entirely would require platform-specific APIs
//!    (O_NOFOLLOW on Unix, FILE_FLAG_OPEN_REPARSE_POINT on Windows)
//!
//! For high-security environments, users should run agnix in a sandboxed
//! environment or on trusted input only.
//!
//! ## Example
//!
//! ```rust,ignore
//! use agnix_core::fs::{FileSystem, MockFileSystem, RealFileSystem};
//! use std::path::Path;
//!
//! // In production code
//! let fs = RealFileSystem;
//! assert!(fs.exists(Path::new("Cargo.toml")));
//!
//! // In tests
//! let mock_fs = MockFileSystem::new();
//! mock_fs.add_file("/test/file.txt", "content");
//! assert!(mock_fs.exists(Path::new("/test/file.txt")));
//! ```

use crate::diagnostics::{LintError, LintResult};
use std::collections::HashMap;
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Metadata information returned by the FileSystem trait.
///
/// This provides a subset of `std::fs::Metadata` that can be mocked.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// Whether this is a regular file
    pub is_file: bool,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
    /// File size in bytes
    pub len: u64,
}

impl FileMetadata {
    /// Create metadata for a regular file
    pub fn file(len: u64) -> Self {
        Self {
            is_file: true,
            is_dir: false,
            is_symlink: false,
            len,
        }
    }

    /// Create metadata for a directory
    pub fn directory() -> Self {
        Self {
            is_file: false,
            is_dir: true,
            is_symlink: false,
            len: 0,
        }
    }

    /// Create metadata for a symlink
    pub fn symlink() -> Self {
        Self {
            is_file: false,
            is_dir: false,
            is_symlink: true,
            len: 0,
        }
    }
}

impl From<&Metadata> for FileMetadata {
    fn from(meta: &Metadata) -> Self {
        Self {
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
            len: meta.len(),
        }
    }
}

/// Directory entry returned by `read_dir`.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Path to this entry
    pub path: PathBuf,
    /// Metadata for this entry
    pub metadata: FileMetadata,
}

/// Trait for abstracting file system operations.
///
/// This trait must be `Send + Sync` to support rayon parallel validation.
/// It also requires `Debug` for use in config structs that derive Debug.
pub trait FileSystem: Send + Sync + std::fmt::Debug {
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;

    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Check if a path is a symlink
    fn is_symlink(&self, path: &Path) -> bool;

    /// Get metadata for a path (follows symlinks)
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// Get metadata for a path without following symlinks
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata>;

    /// Read file contents to string (with security checks)
    fn read_to_string(&self, path: &Path) -> LintResult<String>;

    /// Write content to file (with security checks)
    fn write(&self, path: &Path, content: &str) -> LintResult<()>;

    /// Canonicalize a path
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Read directory contents
    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>>;
}

/// Real file system implementation that delegates to `std::fs` and `file_utils`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_symlink(&self, path: &Path) -> bool {
        path.is_symlink()
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        std::fs::metadata(path).map(|m| FileMetadata::from(&m))
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        std::fs::symlink_metadata(path).map(|m| FileMetadata::from(&m))
    }

    fn read_to_string(&self, path: &Path) -> LintResult<String> {
        crate::file_utils::safe_read_file(path)
    }

    fn write(&self, path: &Path, content: &str) -> LintResult<()> {
        crate::file_utils::safe_write_file(path, content)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry_res| {
                // Skip entries that fail to read (permission denied, etc.)
                // This matches the previous AS-015 behavior of tolerating bad entries
                let entry = entry_res.ok()?;
                let path = entry.path();
                // Use symlink_metadata to avoid following symlinks
                // Skip entries where metadata fails (transient errors)
                let metadata = std::fs::symlink_metadata(&path).ok()?;
                Some(DirEntry {
                    path,
                    metadata: FileMetadata::from(&metadata),
                })
            })
            .collect())
    }
}

/// Mock entry type for the in-memory file system.
#[derive(Debug, Clone)]
enum MockEntry {
    File { content: String },
    Directory,
    Symlink { target: PathBuf },
}

/// Mock file system for testing.
///
/// Provides an in-memory HashMap-based storage with `RwLock` for thread safety.
/// This enables unit testing validators without requiring real temp files.
#[derive(Debug, Default)]
pub struct MockFileSystem {
    entries: RwLock<HashMap<PathBuf, MockEntry>>,
}

impl MockFileSystem {
    /// Create a new empty mock file system
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Add a file with the given content
    pub fn add_file(&self, path: impl AsRef<Path>, content: impl Into<String>) {
        let path = normalize_mock_path(path.as_ref());
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");
        entries.insert(
            path,
            MockEntry::File {
                content: content.into(),
            },
        );
    }

    /// Add a directory
    pub fn add_dir(&self, path: impl AsRef<Path>) {
        let path = normalize_mock_path(path.as_ref());
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");
        entries.insert(path, MockEntry::Directory);
    }

    /// Add a symlink pointing to target
    pub fn add_symlink(&self, path: impl AsRef<Path>, target: impl AsRef<Path>) {
        let path = normalize_mock_path(path.as_ref());
        let target = normalize_mock_path(target.as_ref());
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");
        entries.insert(path, MockEntry::Symlink { target });
    }

    /// Remove an entry
    pub fn remove(&self, path: impl AsRef<Path>) {
        let path = normalize_mock_path(path.as_ref());
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");
        entries.remove(&path);
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");
        entries.clear();
    }

    fn get_entry(&self, path: &Path) -> Option<MockEntry> {
        let path = normalize_mock_path(path);
        let entries = self.entries.read().expect("MockFileSystem lock poisoned");
        entries.get(&path).cloned()
    }

    fn resolve_symlink(&self, path: &Path) -> Option<PathBuf> {
        let path = normalize_mock_path(path);
        let entries = self.entries.read().expect("MockFileSystem lock poisoned");
        match entries.get(&path) {
            Some(MockEntry::Symlink { target }) => Some(target.clone()),
            _ => None,
        }
    }

    /// Maximum depth for symlink resolution to prevent infinite loops.
    ///
    /// This matches the typical OS limit (Linux ELOOP is triggered at 40 levels).
    /// Value chosen to match POSIX `SYMLOOP_MAX` and Linux's internal limit.
    /// See: https://man7.org/linux/man-pages/man3/fpathconf.3.html
    pub const MAX_SYMLINK_DEPTH: u32 = 40;

    /// Internal helper for metadata with depth tracking
    fn metadata_with_depth(&self, path: &Path, depth: u32) -> io::Result<FileMetadata> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(io::Error::other("too many levels of symbolic links"));
        }

        // Follow symlinks - use an enum to handle the result outside the lock
        enum MetaResult {
            Found(FileMetadata),
            FollowSymlink(PathBuf),
        }

        let path = normalize_mock_path(path);

        let result: io::Result<MetaResult> = {
            let entries = self.entries.read().expect("MockFileSystem lock poisoned");
            match entries.get(&path) {
                None => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("path not found: {}", path.display()),
                )),
                Some(MockEntry::File { content }) => {
                    Ok(MetaResult::Found(FileMetadata::file(content.len() as u64)))
                }
                Some(MockEntry::Directory) => Ok(MetaResult::Found(FileMetadata::directory())),
                Some(MockEntry::Symlink { target }) => {
                    Ok(MetaResult::FollowSymlink(target.clone()))
                }
            }
        };

        match result? {
            MetaResult::Found(meta) => Ok(meta),
            MetaResult::FollowSymlink(target) => self.metadata_with_depth(&target, depth + 1),
        }
    }

    /// Internal helper for canonicalize with depth tracking
    fn canonicalize_with_depth(&self, path: &Path, depth: u32) -> io::Result<PathBuf> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(io::Error::other("too many levels of symbolic links"));
        }

        let path_normalized = normalize_mock_path(path);

        if !self.exists(&path_normalized) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("path not found: {}", path.display()),
            ));
        }

        // Follow symlinks if present
        if let Some(target) = self.resolve_symlink(&path_normalized) {
            self.canonicalize_with_depth(&target, depth + 1)
        } else {
            Ok(path_normalized)
        }
    }
}

/// Normalize a path for mock file system storage.
/// Converts backslashes to forward slashes for cross-platform consistency.
fn normalize_mock_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    PathBuf::from(path_str.replace('\\', "/"))
}

impl FileSystem for MockFileSystem {
    fn exists(&self, path: &Path) -> bool {
        self.get_entry(path).is_some()
    }

    fn is_file(&self, path: &Path) -> bool {
        matches!(self.get_entry(path), Some(MockEntry::File { .. }))
    }

    fn is_dir(&self, path: &Path) -> bool {
        matches!(self.get_entry(path), Some(MockEntry::Directory))
    }

    fn is_symlink(&self, path: &Path) -> bool {
        matches!(self.get_entry(path), Some(MockEntry::Symlink { .. }))
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata_with_depth(path, 0)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        // Don't follow symlinks
        let entry = self.get_entry(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("path not found: {}", path.display()),
            )
        })?;

        match entry {
            MockEntry::File { content } => Ok(FileMetadata::file(content.len() as u64)),
            MockEntry::Directory => Ok(FileMetadata::directory()),
            MockEntry::Symlink { .. } => Ok(FileMetadata::symlink()),
        }
    }

    fn read_to_string(&self, path: &Path) -> LintResult<String> {
        let path_normalized = normalize_mock_path(path);
        let entries = self.entries.read().expect("MockFileSystem lock poisoned");

        let entry = entries
            .get(&path_normalized)
            .ok_or_else(|| LintError::FileRead {
                path: path.to_path_buf(),
                source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
            })?;

        match entry {
            MockEntry::File { content } => Ok(content.clone()),
            MockEntry::Directory => Err(LintError::FileNotRegular {
                path: path.to_path_buf(),
            }),
            MockEntry::Symlink { .. } => Err(LintError::FileSymlink {
                path: path.to_path_buf(),
            }),
        }
    }

    fn write(&self, path: &Path, content: &str) -> LintResult<()> {
        let path_normalized = normalize_mock_path(path);
        let mut entries = self.entries.write().expect("MockFileSystem lock poisoned");

        // Check if path exists and is valid for writing
        match entries.get(&path_normalized) {
            Some(MockEntry::File { .. }) => {
                // Overwrite existing file
                entries.insert(
                    path_normalized,
                    MockEntry::File {
                        content: content.to_string(),
                    },
                );
                Ok(())
            }
            Some(MockEntry::Directory) => Err(LintError::FileNotRegular {
                path: path.to_path_buf(),
            }),
            Some(MockEntry::Symlink { .. }) => Err(LintError::FileSymlink {
                path: path.to_path_buf(),
            }),
            None => {
                // File doesn't exist - error like safe_write_file
                Err(LintError::FileWrite {
                    path: path.to_path_buf(),
                    source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
                })
            }
        }
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.canonicalize_with_depth(path, 0)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let path_normalized = normalize_mock_path(path);

        // Check if it's a directory
        match self.get_entry(&path_normalized) {
            Some(MockEntry::Directory) => {}
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "not a directory",
                ));
            }
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "directory not found",
                ));
            }
        }

        let entries = self.entries.read().expect("MockFileSystem lock poisoned");
        let mut result = Vec::new();

        // Normalize the path string for prefix matching
        let prefix = if path_normalized.to_string_lossy().ends_with('/') {
            path_normalized.to_string_lossy().to_string()
        } else {
            format!("{}/", path_normalized.display())
        };

        for (entry_path, entry) in entries.iter() {
            let entry_str = entry_path.to_string_lossy();

            // Check if this entry is a direct child of the directory
            if let Some(rest) = entry_str.strip_prefix(&prefix) {
                // Only include direct children (no further slashes)
                if !rest.contains('/') && !rest.is_empty() {
                    let metadata = match entry {
                        MockEntry::File { content } => FileMetadata::file(content.len() as u64),
                        MockEntry::Directory => FileMetadata::directory(),
                        MockEntry::Symlink { .. } => FileMetadata::symlink(),
                    };
                    result.push(DirEntry {
                        path: entry_path.clone(),
                        metadata,
                    });
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== RealFileSystem tests =====

    #[test]
    fn test_real_fs_exists() {
        let fs = RealFileSystem;
        // Cargo.toml should exist in the project root
        assert!(fs.exists(Path::new("Cargo.toml")));
        assert!(!fs.exists(Path::new("nonexistent_file_xyz.txt")));
    }

    #[test]
    fn test_real_fs_is_file() {
        let fs = RealFileSystem;
        assert!(fs.is_file(Path::new("Cargo.toml")));
        assert!(!fs.is_file(Path::new("src")));
    }

    #[test]
    fn test_real_fs_is_dir() {
        let fs = RealFileSystem;
        assert!(fs.is_dir(Path::new("src")));
        assert!(!fs.is_dir(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_real_fs_read_to_string() {
        let fs = RealFileSystem;
        let content = fs.read_to_string(Path::new("Cargo.toml"));
        assert!(content.is_ok());
        assert!(content.unwrap().contains("[package]"));
    }

    #[test]
    fn test_real_fs_read_nonexistent() {
        let fs = RealFileSystem;
        let result = fs.read_to_string(Path::new("nonexistent_file_xyz.txt"));
        assert!(result.is_err());
    }

    // ===== MockFileSystem tests =====

    #[test]
    fn test_mock_fs_add_and_exists() {
        let fs = MockFileSystem::new();
        assert!(!fs.exists(Path::new("/test/file.txt")));

        fs.add_file("/test/file.txt", "content");
        assert!(fs.exists(Path::new("/test/file.txt")));
    }

    #[test]
    fn test_mock_fs_is_file() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_dir("/test/dir");

        assert!(fs.is_file(Path::new("/test/file.txt")));
        assert!(!fs.is_file(Path::new("/test/dir")));
    }

    #[test]
    fn test_mock_fs_is_dir() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_dir("/test/dir");

        assert!(!fs.is_dir(Path::new("/test/file.txt")));
        assert!(fs.is_dir(Path::new("/test/dir")));
    }

    #[test]
    fn test_mock_fs_is_symlink() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_symlink("/test/link.txt", "/test/file.txt");

        assert!(!fs.is_symlink(Path::new("/test/file.txt")));
        assert!(fs.is_symlink(Path::new("/test/link.txt")));
    }

    #[test]
    fn test_mock_fs_read_to_string() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "hello world");

        let content = fs.read_to_string(Path::new("/test/file.txt"));
        assert!(content.is_ok());
        assert_eq!(content.unwrap(), "hello world");
    }

    #[test]
    fn test_mock_fs_read_nonexistent() {
        let fs = MockFileSystem::new();
        let result = fs.read_to_string(Path::new("/test/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_fs_read_directory_fails() {
        let fs = MockFileSystem::new();
        fs.add_dir("/test/dir");

        let result = fs.read_to_string(Path::new("/test/dir"));
        assert!(matches!(result, Err(LintError::FileNotRegular { .. })));
    }

    #[test]
    fn test_mock_fs_read_symlink_fails() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_symlink("/test/link.txt", "/test/file.txt");

        let result = fs.read_to_string(Path::new("/test/link.txt"));
        assert!(matches!(result, Err(LintError::FileSymlink { .. })));
    }

    #[test]
    fn test_mock_fs_write() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "original");

        let result = fs.write(Path::new("/test/file.txt"), "updated");
        assert!(result.is_ok());

        let content = fs.read_to_string(Path::new("/test/file.txt")).unwrap();
        assert_eq!(content, "updated");
    }

    #[test]
    fn test_mock_fs_write_nonexistent_fails() {
        let fs = MockFileSystem::new();

        let result = fs.write(Path::new("/test/file.txt"), "content");
        assert!(matches!(result, Err(LintError::FileWrite { .. })));
    }

    #[test]
    fn test_mock_fs_metadata_file() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "12345");

        let meta = fs.metadata(Path::new("/test/file.txt")).unwrap();
        assert!(meta.is_file);
        assert!(!meta.is_dir);
        assert!(!meta.is_symlink);
        assert_eq!(meta.len, 5);
    }

    #[test]
    fn test_mock_fs_metadata_directory() {
        let fs = MockFileSystem::new();
        fs.add_dir("/test/dir");

        let meta = fs.metadata(Path::new("/test/dir")).unwrap();
        assert!(!meta.is_file);
        assert!(meta.is_dir);
        assert!(!meta.is_symlink);
    }

    #[test]
    fn test_mock_fs_symlink_metadata() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_symlink("/test/link.txt", "/test/file.txt");

        // symlink_metadata should not follow symlinks
        let meta = fs.symlink_metadata(Path::new("/test/link.txt")).unwrap();
        assert!(meta.is_symlink);

        // metadata should follow symlinks
        let meta = fs.metadata(Path::new("/test/link.txt")).unwrap();
        assert!(meta.is_file);
        assert!(!meta.is_symlink);
    }

    #[test]
    fn test_mock_fs_read_dir() {
        let fs = MockFileSystem::new();
        fs.add_dir("/test");
        fs.add_file("/test/file1.txt", "content1");
        fs.add_file("/test/file2.txt", "content2");
        fs.add_dir("/test/subdir");

        let entries = fs.read_dir(Path::new("/test")).unwrap();
        assert_eq!(entries.len(), 3);

        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.txt".to_string()));
        assert!(names.contains(&"subdir".to_string()));
    }

    #[test]
    fn test_mock_fs_read_dir_nonexistent() {
        let fs = MockFileSystem::new();
        let result = fs.read_dir(Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_fs_read_dir_not_directory() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");

        let result = fs.read_dir(Path::new("/test/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_fs_canonicalize() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");

        let canonical = fs.canonicalize(Path::new("/test/file.txt")).unwrap();
        assert_eq!(canonical, PathBuf::from("/test/file.txt"));
    }

    #[test]
    fn test_mock_fs_canonicalize_follows_symlink() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        fs.add_symlink("/test/link.txt", "/test/file.txt");

        let canonical = fs.canonicalize(Path::new("/test/link.txt")).unwrap();
        assert_eq!(canonical, PathBuf::from("/test/file.txt"));
    }

    #[test]
    fn test_mock_fs_clear() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        assert!(fs.exists(Path::new("/test/file.txt")));

        fs.clear();
        assert!(!fs.exists(Path::new("/test/file.txt")));
    }

    #[test]
    fn test_mock_fs_remove() {
        let fs = MockFileSystem::new();
        fs.add_file("/test/file.txt", "content");
        assert!(fs.exists(Path::new("/test/file.txt")));

        fs.remove("/test/file.txt");
        assert!(!fs.exists(Path::new("/test/file.txt")));
    }

    #[test]
    fn test_mock_fs_windows_path_normalization() {
        let fs = MockFileSystem::new();
        fs.add_file("C:/test/file.txt", "content");

        // Should work with either path separator
        assert!(fs.exists(Path::new("C:/test/file.txt")));
        assert!(fs.exists(Path::new("C:\\test\\file.txt")));
    }

    #[test]
    fn test_mock_fs_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let fs = Arc::new(MockFileSystem::new());
        let mut handles = vec![];

        // Spawn multiple threads that read and write
        for i in 0..10 {
            let fs_clone = Arc::clone(&fs);
            let handle = thread::spawn(move || {
                let path = format!("/test/file{}.txt", i);
                fs_clone.add_file(&path, format!("content{}", i));
                assert!(fs_clone.exists(Path::new(&path)));
                let content = fs_clone.read_to_string(Path::new(&path)).unwrap();
                assert_eq!(content, format!("content{}", i));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all files exist
        for i in 0..10 {
            let path = format!("/test/file{}.txt", i);
            assert!(fs.exists(Path::new(&path)));
        }
    }

    #[test]
    fn test_mock_fs_circular_symlink_metadata() {
        let fs = MockFileSystem::new();
        // Create circular symlinks: a -> b -> a
        fs.add_symlink("/test/a", "/test/b");
        fs.add_symlink("/test/b", "/test/a");

        // metadata() follows symlinks and should detect the cycle
        let result = fs.metadata(Path::new("/test/a"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("too many levels of symbolic links")
        );
    }

    #[test]
    fn test_mock_fs_circular_symlink_canonicalize() {
        let fs = MockFileSystem::new();
        // Create circular symlinks: a -> b -> a
        fs.add_symlink("/test/a", "/test/b");
        fs.add_symlink("/test/b", "/test/a");

        // canonicalize() follows symlinks and should detect the cycle
        let result = fs.canonicalize(Path::new("/test/a"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("too many levels of symbolic links")
        );
    }

    #[test]
    fn test_mock_fs_chained_symlinks() {
        let fs = MockFileSystem::new();
        // Create chain: link1 -> link2 -> link3 -> file
        fs.add_file("/test/file.txt", "content");
        fs.add_symlink("/test/link3", "/test/file.txt");
        fs.add_symlink("/test/link2", "/test/link3");
        fs.add_symlink("/test/link1", "/test/link2");

        // metadata() should follow the chain and return file metadata
        let meta = fs.metadata(Path::new("/test/link1")).unwrap();
        assert!(meta.is_file);
        assert_eq!(meta.len, 7); // "content".len()

        // canonicalize() should return the final target
        let canonical = fs.canonicalize(Path::new("/test/link1")).unwrap();
        assert_eq!(canonical, PathBuf::from("/test/file.txt"));
    }

    #[test]
    fn test_mock_fs_max_symlink_depth_boundary() {
        // Test that we can handle chains up to MAX_SYMLINK_DEPTH
        let fs = MockFileSystem::new();
        fs.add_file("/test/target.txt", "content");

        // Create a chain of exactly MAX_SYMLINK_DEPTH links
        let mut prev = PathBuf::from("/test/target.txt");
        for i in 0..MockFileSystem::MAX_SYMLINK_DEPTH {
            let link = PathBuf::from(format!("/test/link{}", i));
            fs.add_symlink(&link, &prev);
            prev = link;
        }

        // Should succeed at the boundary
        let result = fs.metadata(&prev);
        assert!(result.is_ok(), "Should handle MAX_SYMLINK_DEPTH links");
    }

    #[test]
    fn test_mock_fs_exceeds_max_symlink_depth() {
        // Test that MAX_SYMLINK_DEPTH + 1 links fails
        let fs = MockFileSystem::new();
        fs.add_file("/test/target.txt", "content");

        // Create a chain of MAX_SYMLINK_DEPTH + 1 links
        let mut prev = PathBuf::from("/test/target.txt");
        for i in 0..=MockFileSystem::MAX_SYMLINK_DEPTH {
            let link = PathBuf::from(format!("/test/link{}", i));
            fs.add_symlink(&link, &prev);
            prev = link;
        }

        // Should fail beyond the limit
        let result = fs.metadata(&prev);
        assert!(
            result.is_err(),
            "Should fail when exceeding MAX_SYMLINK_DEPTH"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("too many levels of symbolic links")
        );
    }

    // ===== Unix-specific symlink tests for RealFileSystem =====

    #[cfg(unix)]
    mod unix_tests {
        use super::*;
        use std::os::unix::fs::symlink;
        use tempfile::TempDir;

        #[test]
        fn test_real_fs_rejects_symlink_read() {
            let temp = TempDir::new().unwrap();
            let target = temp.path().join("target.txt");
            let link = temp.path().join("link.txt");

            std::fs::write(&target, "content").unwrap();
            symlink(&target, &link).unwrap();

            let fs = RealFileSystem;
            let result = fs.read_to_string(&link);

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
        }

        #[test]
        fn test_real_fs_symlink_metadata() {
            let temp = TempDir::new().unwrap();
            let target = temp.path().join("target.txt");
            let link = temp.path().join("link.txt");

            std::fs::write(&target, "content").unwrap();
            symlink(&target, &link).unwrap();

            let fs = RealFileSystem;

            // symlink_metadata should show symlink
            let meta = fs.symlink_metadata(&link).unwrap();
            assert!(meta.is_symlink);

            // metadata follows symlink and shows file
            let meta = fs.metadata(&link).unwrap();
            assert!(meta.is_file);
            assert!(!meta.is_symlink);
        }

        #[test]
        fn test_real_fs_dangling_symlink() {
            let temp = TempDir::new().unwrap();
            let link = temp.path().join("dangling.txt");

            symlink("/nonexistent/target", &link).unwrap();

            let fs = RealFileSystem;
            let result = fs.read_to_string(&link);

            // Should reject as symlink (caught before we try to read nonexistent target)
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
        }

        #[test]
        fn test_real_fs_is_symlink() {
            let temp = TempDir::new().unwrap();
            let target = temp.path().join("target.txt");
            let link = temp.path().join("link.txt");

            std::fs::write(&target, "content").unwrap();
            symlink(&target, &link).unwrap();

            let fs = RealFileSystem;

            assert!(!fs.is_symlink(&target));
            assert!(fs.is_symlink(&link));
        }

        #[test]
        fn test_real_fs_read_dir_skips_symlinks_in_metadata() {
            let temp = TempDir::new().unwrap();

            // Create a regular file
            std::fs::write(temp.path().join("file.txt"), "content").unwrap();

            // Create a symlink
            symlink(temp.path().join("file.txt"), temp.path().join("link.txt")).unwrap();

            let fs = RealFileSystem;
            let entries = fs.read_dir(temp.path()).unwrap();

            // Both should be returned
            assert_eq!(entries.len(), 2);

            // But the symlink should have is_symlink = true in metadata
            let symlink_entry = entries
                .iter()
                .find(|e| e.path.file_name().unwrap().to_str().unwrap() == "link.txt");
            assert!(symlink_entry.is_some());
            assert!(symlink_entry.unwrap().metadata.is_symlink);

            // And the file should have is_file = true
            let file_entry = entries
                .iter()
                .find(|e| e.path.file_name().unwrap().to_str().unwrap() == "file.txt");
            assert!(file_entry.is_some());
            assert!(file_entry.unwrap().metadata.is_file);
        }
    }
}
