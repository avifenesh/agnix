//! Safe file reading utilities
//!
//! Provides hardened file reading with symlink rejection, regular file checks,
//! and size limits to prevent security issues and resource exhaustion.
//!
//! ## Security Model
//!
//! This module uses `symlink_metadata()` to check file properties without
//! following symlinks. It then verifies the file is a regular file (not a
//! directory, FIFO, socket, or device) before reading.
//!
//! Note: There is an inherent TOCTOU (time-of-check-time-of-use) window
//! between the metadata check and the read. This is acceptable for a linter
//! because: (1) the attack requires local filesystem access, (2) the impact
//! is limited to reading unexpected content, and (3) eliminating TOCTOU
//! entirely would require platform-specific APIs (O_NOFOLLOW, etc.).

use crate::diagnostics::{LintError, LintResult};
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default maximum file size (1 MiB = 1,048,576 bytes = 2^20 bytes)
///
/// **Design Decision**: 1 MiB was chosen as a balance between:
/// - Large enough for realistic documentation files (most are <100KB)
/// - Small enough to prevent memory exhaustion attacks
/// - Prevents YAML bomb attacks (deeply nested structures within size limit)
/// - 2^20 is a clean power-of-2 boundary for memory allocation
///
/// Files exactly at 1,048,576 bytes are accepted.
/// Files at 1,048,577 bytes or larger are rejected.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 1_048_576;

/// Safely read a file with security checks.
///
/// This function:
/// 1. Rejects symlinks (uses `symlink_metadata` to detect without following)
/// 2. Rejects non-regular files (directories, FIFOs, sockets, devices)
/// 3. Enforces a maximum file size limit (files at exactly the limit are accepted)
///
/// # Errors
///
/// Returns `LintError::FileSymlink` if the path is a symlink.
/// Returns `LintError::FileNotRegular` if the path is not a regular file.
/// Returns `LintError::FileTooBig` if the file exceeds the size limit.
/// Returns `LintError::FileRead` for other I/O errors.
pub fn safe_read_file(path: &Path) -> LintResult<String> {
    safe_read_file_with_limit(path, DEFAULT_MAX_FILE_SIZE)
}

/// Safely write to an existing file with security checks.
///
/// This function:
/// 1. Rejects symlinks (uses `symlink_metadata` to detect without following)
/// 2. Rejects non-regular files (directories, FIFOs, sockets, devices)
/// 3. Writes via a temporary file and atomic rename to reduce TOCTOU risk
///
/// # Errors
///
/// Returns `LintError::FileSymlink` if the path is a symlink.
/// Returns `LintError::FileNotRegular` if the path is not a regular file.
/// Returns `LintError::FileWrite` for other I/O errors.
pub fn safe_write_file(path: &Path, content: &str) -> LintResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| LintError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;

    if metadata.file_type().is_symlink() {
        return Err(LintError::FileSymlink {
            path: path.to_path_buf(),
        });
    }

    if !metadata.is_file() {
        return Err(LintError::FileNotRegular {
            path: path.to_path_buf(),
        });
    }

    let permissions = metadata.permissions();
    let parent = path.parent().ok_or_else(|| LintError::FileWrite {
        path: path.to_path_buf(),
        source: io::Error::other("Missing parent directory"),
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");

    // Create unique temporary file with retry logic
    // Uniqueness is ensured by: nanosecond timestamp + attempt counter
    // The probability of collision is negligible with up to 10 retries
    let (temp_path, mut temp_file) = {
        let mut attempt = 0u32;
        loop {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let candidate = parent.join(format!(
                ".{}.agnix.tmp.{}",
                file_name,
                unique + attempt as u128
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => break (candidate, file),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt < 10 => {
                    attempt += 1;
                    continue;
                }
                Err(e) => {
                    return Err(LintError::FileWrite {
                        path: path.to_path_buf(),
                        source: e,
                    });
                }
            }
        }
    };

    temp_file
        .write_all(content.as_bytes())
        .map_err(|e| LintError::FileWrite {
            path: path.to_path_buf(),
            source: e,
        })?;
    temp_file.sync_all().map_err(|e| LintError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;
    drop(temp_file);

    fs::set_permissions(&temp_path, permissions).map_err(|e| LintError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;

    let recheck = fs::symlink_metadata(path).map_err(|e| LintError::FileWrite {
        path: path.to_path_buf(),
        source: e,
    })?;
    if recheck.file_type().is_symlink() {
        let _ = fs::remove_file(&temp_path);
        return Err(LintError::FileSymlink {
            path: path.to_path_buf(),
        });
    }
    if !recheck.is_file() {
        let _ = fs::remove_file(&temp_path);
        return Err(LintError::FileNotRegular {
            path: path.to_path_buf(),
        });
    }

    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        LintError::FileWrite {
            path: path.to_path_buf(),
            source: e,
        }
    })
}

/// Safely read a file with a custom size limit.
///
/// See [`safe_read_file`] for details on security checks.
///
/// The size limit uses `>` comparison, so files at exactly `max_size` bytes
/// are accepted, while files larger than `max_size` are rejected.
pub fn safe_read_file_with_limit(path: &Path, max_size: u64) -> LintResult<String> {
    // Use symlink_metadata to get metadata WITHOUT following symlinks
    // This is the key difference from fs::metadata() which follows symlinks
    let metadata = fs::symlink_metadata(path).map_err(|e| LintError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Reject symlinks for security (prevents path traversal)
    if metadata.file_type().is_symlink() {
        return Err(LintError::FileSymlink {
            path: path.to_path_buf(),
        });
    }

    // Reject non-regular files (prevents hangs on FIFOs, reads from devices)
    if !metadata.is_file() {
        return Err(LintError::FileNotRegular {
            path: path.to_path_buf(),
        });
    }

    // Check file size (prevents DoS via large files)
    let size = metadata.len();
    if size > max_size {
        return Err(LintError::FileTooBig {
            path: path.to_path_buf(),
            size,
            limit: max_size,
        });
    }

    // Read the file
    fs::read_to_string(path).map_err(|e| LintError::FileRead {
        path: path.to_path_buf(),
        source: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_normal_file_read_succeeds() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.md");
        let content = "Hello, world!";
        fs::write(&file_path, content).unwrap();

        let result = safe_read_file(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_empty_file_read_succeeds() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("empty.md");
        fs::write(&file_path, "").unwrap();

        let result = safe_read_file(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_nonexistent_file_returns_error() {
        let result = safe_read_file(Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LintError::FileRead { .. }));
    }

    #[test]
    fn test_oversized_file_rejected() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("large.txt");

        // Create a file larger than a small limit
        let mut file = fs::File::create(&file_path).unwrap();
        let content = vec![b'x'; 1024]; // 1 KB
        file.write_all(&content).unwrap();

        // Use a smaller limit for testing
        let result = safe_read_file_with_limit(&file_path, 512);
        assert!(result.is_err());

        match result.unwrap_err() {
            LintError::FileTooBig { size, limit, .. } => {
                assert_eq!(size, 1024);
                assert_eq!(limit, 512);
            }
            other => panic!("Expected FileTooBig error, got {:?}", other),
        }
    }

    #[test]
    fn test_file_at_exact_limit_succeeds() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("exact.txt");

        let content = vec![b'x'; 512];
        fs::write(&file_path, &content).unwrap();

        // File is exactly at the limit - should succeed
        let result = safe_read_file_with_limit(&file_path, 512);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_one_byte_over_limit_rejected() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("over.txt");

        let content = vec![b'x'; 513];
        fs::write(&file_path, &content).unwrap();

        // File is one byte over - should fail
        let result = safe_read_file_with_limit(&file_path, 512);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LintError::FileTooBig { .. }));
    }

    #[test]
    fn test_default_max_file_size_is_1_mib() {
        // Verify the constant is set to 1 MiB (1,048,576 bytes)
        assert_eq!(DEFAULT_MAX_FILE_SIZE, 1_048_576);
        assert_eq!(DEFAULT_MAX_FILE_SIZE, 1024 * 1024);
    }

    #[test]
    fn test_file_at_1_mib_limit_accepted() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("exactly_1mib.txt");

        // Create a file of exactly 1 MiB
        let content = vec![b'x'; DEFAULT_MAX_FILE_SIZE as usize];
        fs::write(&file_path, &content).unwrap();

        // Should succeed - files at exactly the limit are accepted
        let result = safe_read_file(&file_path);
        assert!(result.is_ok(), "1 MiB file should be accepted");
        assert_eq!(result.unwrap().len(), DEFAULT_MAX_FILE_SIZE as usize);
    }

    #[test]
    fn test_file_over_1_mib_limit_rejected() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("over_1mib.txt");

        // Create a file just over 1 MiB
        let content = vec![b'x'; DEFAULT_MAX_FILE_SIZE as usize + 1];
        fs::write(&file_path, &content).unwrap();

        // Should fail
        let result = safe_read_file(&file_path);
        assert!(result.is_err(), "File over 1 MiB should be rejected");
        match result.unwrap_err() {
            LintError::FileTooBig { size, limit, .. } => {
                assert_eq!(size, DEFAULT_MAX_FILE_SIZE + 1);
                assert_eq!(limit, DEFAULT_MAX_FILE_SIZE);
            }
            other => panic!("Expected FileTooBig error, got: {:?}", other),
        }
    }

    #[test]
    fn test_directory_rejected() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let result = safe_read_file(&dir_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LintError::FileNotRegular { .. }
        ));
    }

    #[test]
    fn test_safe_write_file_updates_content() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("write.md");
        fs::write(&file_path, "before").unwrap();

        let result = safe_write_file(&file_path, "after");
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "after");
    }

    #[test]
    fn test_safe_write_file_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("write_dir");
        fs::create_dir(&dir_path).unwrap();

        let result = safe_write_file(&dir_path, "nope");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LintError::FileNotRegular { .. }
        ));
    }

    #[test]
    fn test_safe_write_file_missing_file_returns_error() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("missing.md");

        let result = safe_write_file(&file_path, "content");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LintError::FileWrite { .. }));
    }

    // Symlink tests - only run on Unix-like systems where symlinks are common
    #[cfg(unix)]
    mod unix_tests {
        use super::*;
        use std::os::unix::fs::symlink;

        #[test]
        fn test_symlink_rejected() {
            let temp = TempDir::new().unwrap();
            let target_path = temp.path().join("target.md");
            let link_path = temp.path().join("link.md");

            fs::write(&target_path, "Target content").unwrap();
            symlink(&target_path, &link_path).unwrap();

            let result = safe_read_file(&link_path);
            assert!(result.is_err());

            match result.unwrap_err() {
                LintError::FileSymlink { path } => {
                    assert_eq!(path, link_path);
                }
                other => panic!("Expected FileSymlink error, got {:?}", other),
            }
        }

        #[test]
        fn test_symlink_to_directory_rejected() {
            let temp = TempDir::new().unwrap();
            let dir_path = temp.path().join("subdir");
            let link_path = temp.path().join("link_to_dir");

            fs::create_dir(&dir_path).unwrap();
            symlink(&dir_path, &link_path).unwrap();

            let result = safe_read_file(&link_path);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
        }

        #[test]
        fn test_dangling_symlink_rejected() {
            let temp = TempDir::new().unwrap();
            let link_path = temp.path().join("dangling.md");

            // Create symlink to non-existent target
            symlink("/nonexistent/target", &link_path).unwrap();

            let result = safe_read_file(&link_path);
            assert!(result.is_err());
            // Dangling symlink is still a symlink
            assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
        }

        #[test]
        fn test_safe_write_file_rejects_symlink() {
            let temp = TempDir::new().unwrap();
            let target_path = temp.path().join("target_write.md");
            let link_path = temp.path().join("link_write.md");

            fs::write(&target_path, "Target content").unwrap();
            symlink(&target_path, &link_path).unwrap();

            let result = safe_write_file(&link_path, "new content");
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
        }
    }

    // Windows symlink tests - require elevated privileges or developer mode
    #[cfg(windows)]
    mod windows_tests {
        use super::*;
        use std::os::windows::fs::symlink_file;

        #[test]
        fn test_symlink_rejected_windows() {
            let temp = TempDir::new().unwrap();
            let target_path = temp.path().join("target.md");
            let link_path = temp.path().join("link.md");

            fs::write(&target_path, "Target content").unwrap();

            // Try to create symlink - may fail without privileges
            if symlink_file(&target_path, &link_path).is_ok() {
                let result = safe_read_file(&link_path);
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
            }
            // If symlink creation fails due to privileges, skip the test
        }
    }

    // ===== TOCTOU Race Condition Tests =====

    #[cfg(unix)]
    #[test]
    fn test_symlink_race_window_documented() {
        // This test documents the TOCTOU (time-of-check-time-of-use) race condition
        // between symlink_metadata check and read operation in safe_read_file.
        //
        // An attacker with local filesystem access could theoretically replace a
        // regular file with a symlink between the check and read. This is acceptable
        // for a linter because:
        // 1. The attack requires local filesystem access
        // 2. The impact is limited to reading unexpected content
        // 3. Eliminating TOCTOU entirely would require platform-specific APIs
        //    (O_NOFOLLOW on Unix, FILE_FLAG_OPEN_REPARSE_POINT on Windows)
        //
        // This test verifies that symlinks are rejected by the initial check.
        use std::os::unix::fs::symlink;
        let temp = TempDir::new().unwrap();
        let target_path = temp.path().join("target.md");
        let link_path = temp.path().join("link.md");

        fs::write(&target_path, "Target content").unwrap();
        symlink(&target_path, &link_path).unwrap();

        // Verify symlink is rejected at check time
        let result = safe_read_file(&link_path);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LintError::FileSymlink { .. }));
    }
}
