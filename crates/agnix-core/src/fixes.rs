//! Fix application engine for automatic corrections

use crate::diagnostics::{Diagnostic, Fix, LintResult};
use crate::fs::{FileSystem, RealFileSystem};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Result of applying fixes to a file
#[derive(Debug, Clone)]
pub struct FixResult {
    /// Path to the file
    pub path: PathBuf,
    /// Original file content
    pub original: String,
    /// Content after fixes applied
    pub fixed: String,
    /// Descriptions of applied fixes
    pub applied: Vec<String>,
}

impl FixResult {
    /// Check if any fixes were actually applied
    pub fn has_changes(&self) -> bool {
        self.original != self.fixed
    }
}

/// Apply fixes from diagnostics to files
///
/// # Arguments
/// * `diagnostics` - Diagnostics with potential fixes
/// * `dry_run` - If true, compute fixes but don't write files
/// * `safe_only` - If true, only apply fixes marked as safe
///
/// # Returns
/// Vector of fix results, one per file that had fixes
pub fn apply_fixes(
    diagnostics: &[Diagnostic],
    dry_run: bool,
    safe_only: bool,
) -> LintResult<Vec<FixResult>> {
    apply_fixes_with_fs(diagnostics, dry_run, safe_only, None)
}

/// Apply fixes from diagnostics to files with optional FileSystem abstraction
///
/// # Arguments
/// * `diagnostics` - Diagnostics with potential fixes
/// * `dry_run` - If true, compute fixes but don't write files
/// * `safe_only` - If true, only apply fixes marked as safe
/// * `fs` - Optional FileSystem for reading/writing files. If None, uses RealFileSystem.
///
/// # Returns
/// Vector of fix results, one per file that had fixes
pub fn apply_fixes_with_fs(
    diagnostics: &[Diagnostic],
    dry_run: bool,
    safe_only: bool,
    fs: Option<Arc<dyn FileSystem>>,
) -> LintResult<Vec<FixResult>> {
    let fs = fs.unwrap_or_else(|| Arc::new(RealFileSystem));

    // Group diagnostics by file
    let mut by_file: HashMap<PathBuf, Vec<&Diagnostic>> = HashMap::new();
    for diag in diagnostics {
        if diag.has_fixes() {
            by_file.entry(diag.file.clone()).or_default().push(diag);
        }
    }

    let mut results = Vec::new();

    for (path, file_diagnostics) in by_file {
        let original = fs.read_to_string(&path)?;

        let mut fixes: Vec<&Fix> = file_diagnostics
            .iter()
            .flat_map(|d| &d.fixes)
            .filter(|f| !safe_only || f.safe)
            .collect();

        if fixes.is_empty() {
            continue;
        }

        // Sort descending to apply from end (preserves earlier positions)
        fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        let (fixed, applied) = apply_fixes_to_content(&original, &fixes);

        if fixed != original {
            if !dry_run {
                fs.write(&path, &fixed)?;
            }

            results.push(FixResult {
                path,
                original,
                fixed,
                applied,
            });
        }
    }

    results.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(results)
}

/// Apply fixes to content string, returning new content and applied descriptions.
/// Fixes must be sorted by start_byte descending to preserve positions.
fn apply_fixes_to_content(content: &str, fixes: &[&Fix]) -> (String, Vec<String>) {
    let mut result = content.to_string();
    let mut applied = Vec::new();
    let mut last_start = usize::MAX;

    for fix in fixes {
        // Check end_byte > start_byte first (more fundamental invariant)
        if fix.end_byte < fix.start_byte {
            // Log: Invalid fix range (end before start)
            continue;
        }
        // Then check bounds
        if fix.start_byte > result.len() || fix.end_byte > result.len() {
            // Log: Fix out of bounds
            continue;
        }
        // Check UTF-8 boundaries
        if !result.is_char_boundary(fix.start_byte) || !result.is_char_boundary(fix.end_byte) {
            // Log: UTF-8 boundary violation - this indicates a bug in fix generation
            // The fix byte offsets don't align with character boundaries
            continue;
        }
        // Skip overlapping fixes (sorted descending, so check against previous fix start)
        if fix.end_byte > last_start {
            // Log: Skipping overlapping fix
            continue;
        }

        result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        applied.push(fix.description.clone());
        last_start = fix.start_byte;
    }

    applied.reverse();

    (result, applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DiagnosticLevel, Fix};

    fn make_diagnostic(path: &str, fixes: Vec<Fix>) -> Diagnostic {
        Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test error".to_string(),
            file: PathBuf::from(path),
            line: 1,
            column: 1,
            rule: "TEST-001".to_string(),
            suggestion: None,
            fixes,
            assumption: None,
        }
    }

    #[test]
    fn test_fix_single_replacement() {
        let content = "name: Bad_Name";
        let fix = Fix::replace(6, 14, "good-name", "Fix name format", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "name: good-name");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], "Fix name format");
    }

    #[test]
    fn test_fix_insertion() {
        let content = "hello world";
        let fix = Fix::insert(5, " beautiful", "Add word", true);

        let (result, _) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "hello beautiful world");
    }

    #[test]
    fn test_fix_deletion() {
        let content = "hello beautiful world";
        let fix = Fix::delete(5, 15, "Remove word", true);

        let (result, _) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_fix_multiple_non_overlapping() {
        let content = "aaa bbb ccc";
        let fixes = vec![
            Fix::replace(0, 3, "AAA", "Uppercase first", true),
            Fix::replace(8, 11, "CCC", "Uppercase last", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        // Sort descending by start_byte (as apply_fixes does)
        let mut sorted = fix_refs.clone();
        sorted.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        let (result, applied) = apply_fixes_to_content(content, &sorted);

        assert_eq!(result, "AAA bbb CCC");
        assert_eq!(applied.len(), 2);
    }

    #[test]
    fn test_fix_reverse_order_preserves_positions() {
        // When we have fixes at positions 0-3 and 8-11,
        // applying 8-11 first keeps position 0-3 valid
        let content = "foo bar baz";
        let fixes = vec![
            Fix::replace(0, 3, "FOO", "Fix 1", true),
            Fix::replace(8, 11, "BAZ", "Fix 2", true),
        ];

        // Sort descending (8-11 first, then 0-3)
        let mut sorted: Vec<&Fix> = fixes.iter().collect();
        sorted.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        let (result, _) = apply_fixes_to_content(content, &sorted);

        assert_eq!(result, "FOO bar BAZ");
    }

    #[test]
    fn test_fix_safe_only_filter() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("test.md");
        std::fs::write(&path, "name: Bad_Name").unwrap();

        let diagnostics = vec![make_diagnostic(
            path.to_str().unwrap(),
            vec![
                Fix::replace(6, 14, "safe-name", "Safe fix", true),
                Fix::replace(0, 4, "NAME", "Unsafe fix", false),
            ],
        )];

        // With safe_only = true, only the safe fix should apply
        let results = apply_fixes(&diagnostics, false, true).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fixed, "name: safe-name");
        assert_eq!(results[0].applied.len(), 1);
    }

    #[test]
    fn test_fix_dry_run_no_write() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("test.md");
        let original = "name: Bad_Name";
        std::fs::write(&path, original).unwrap();

        let diagnostics = vec![make_diagnostic(
            path.to_str().unwrap(),
            vec![Fix::replace(6, 14, "good-name", "Fix name", true)],
        )];

        // Dry run
        let results = apply_fixes(&diagnostics, true, false).unwrap();

        // Results should show the fix
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fixed, "name: good-name");

        // But file should be unchanged
        let file_content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(file_content, original);
    }

    #[test]
    fn test_fix_actual_write() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("test.md");
        std::fs::write(&path, "name: Bad_Name").unwrap();

        let diagnostics = vec![make_diagnostic(
            path.to_str().unwrap(),
            vec![Fix::replace(6, 14, "good-name", "Fix name", true)],
        )];

        // Actually apply
        let results = apply_fixes(&diagnostics, false, false).unwrap();

        assert_eq!(results.len(), 1);

        // File should be modified
        let file_content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(file_content, "name: good-name");
    }

    #[test]
    fn test_fix_invalid_positions_skipped() {
        let content = "short";
        let fix = Fix::replace(100, 200, "won't apply", "Bad fix", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "short");
        assert!(applied.is_empty());
    }

    #[test]
    fn test_fix_empty_diagnostics() {
        let results = apply_fixes(&[], false, false).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fix_no_fixes_in_diagnostics() {
        let diagnostics = vec![Diagnostic {
            level: DiagnosticLevel::Error,
            message: "No fix available".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "TEST-001".to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
        }];

        let results = apply_fixes(&diagnostics, false, false).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fix_result_has_changes() {
        let result_with_changes = FixResult {
            path: PathBuf::from("test.md"),
            original: "old".to_string(),
            fixed: "new".to_string(),
            applied: vec!["Fix".to_string()],
        };
        assert!(result_with_changes.has_changes());

        let result_no_changes = FixResult {
            path: PathBuf::from("test.md"),
            original: "same".to_string(),
            fixed: "same".to_string(),
            applied: vec![],
        };
        assert!(!result_no_changes.has_changes());
    }

    #[test]
    fn test_fix_overlapping_skipped() {
        let content = "hello world";
        // Overlapping fixes: first at 6-11, second at 4-8
        // Sorted descending: 6-11 first, then 4-8
        // 4-8 overlaps with 6-11 (end_byte 8 > start 6), should be skipped
        let fixes = vec![
            Fix::replace(6, 11, "universe", "Fix 1", true),
            Fix::replace(4, 8, "XXX", "Fix 2 overlaps", true),
        ];

        let mut sorted: Vec<&Fix> = fixes.iter().collect();
        sorted.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

        let (result, applied) = apply_fixes_to_content(content, &sorted);

        assert_eq!(result, "hello universe");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], "Fix 1");
    }

    // ===== MockFileSystem Integration Tests =====

    #[test]
    fn test_apply_fixes_with_mock_fs_dry_run() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();
        mock_fs.add_file("/project/test.md", "name: Bad_Name");

        let diagnostics = vec![make_diagnostic(
            "/project/test.md",
            vec![Fix::replace(6, 14, "good-name", "Fix name", true)],
        )];

        // Dry run with mock filesystem
        let results =
            apply_fixes_with_fs(&diagnostics, true, false, Some(Arc::new(mock_fs))).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].original, "name: Bad_Name");
        assert_eq!(results[0].fixed, "name: good-name");
        assert!(results[0].has_changes());

        // File should be unchanged (dry run)
        // Note: We can't verify this directly since we passed ownership,
        // but the logic is tested - dry_run=true means no write() call
    }

    #[test]
    fn test_apply_fixes_with_mock_fs_actual_write() {
        use crate::fs::{FileSystem, MockFileSystem};

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/project/test.md", "name: Bad_Name");

        let diagnostics = vec![make_diagnostic(
            "/project/test.md",
            vec![Fix::replace(6, 14, "good-name", "Fix name", true)],
        )];

        // Clone as trait object for apply_fixes_with_fs
        let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;

        // Actually apply with mock filesystem
        let results = apply_fixes_with_fs(&diagnostics, false, false, Some(fs_clone)).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].fixed, "name: good-name");

        // Verify mock filesystem was updated
        let content = mock_fs
            .read_to_string(std::path::Path::new("/project/test.md"))
            .unwrap();
        assert_eq!(content, "name: good-name");
    }

    #[test]
    fn test_apply_fixes_with_mock_fs_safe_only() {
        use crate::fs::{FileSystem, MockFileSystem};

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/project/test.md", "name: Bad_Name");

        let diagnostics = vec![make_diagnostic(
            "/project/test.md",
            vec![
                Fix::replace(6, 14, "safe-name", "Safe fix", true),
                Fix::replace(0, 4, "NAME", "Unsafe fix", false),
            ],
        )];

        // Clone as trait object for apply_fixes_with_fs
        let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;

        // Apply with safe_only = true
        let results = apply_fixes_with_fs(&diagnostics, false, true, Some(fs_clone)).unwrap();

        assert_eq!(results.len(), 1);
        // Only safe fix should be applied
        assert_eq!(results[0].fixed, "name: safe-name");
        assert_eq!(results[0].applied.len(), 1);
        assert_eq!(results[0].applied[0], "Safe fix");

        // Verify mock filesystem reflects only safe fix
        let content = mock_fs
            .read_to_string(std::path::Path::new("/project/test.md"))
            .unwrap();
        assert_eq!(content, "name: safe-name");
    }

    #[test]
    fn test_apply_fixes_with_mock_fs_multiple_files() {
        use crate::fs::{FileSystem, MockFileSystem};

        let mock_fs = Arc::new(MockFileSystem::new());
        mock_fs.add_file("/project/file1.md", "old1");
        mock_fs.add_file("/project/file2.md", "old2");

        let diagnostics = vec![
            make_diagnostic(
                "/project/file1.md",
                vec![Fix::replace(0, 4, "new1", "Fix file1", true)],
            ),
            make_diagnostic(
                "/project/file2.md",
                vec![Fix::replace(0, 4, "new2", "Fix file2", true)],
            ),
        ];

        // Clone as trait object for apply_fixes_with_fs
        let fs_clone: Arc<dyn FileSystem> = Arc::clone(&mock_fs) as Arc<dyn FileSystem>;

        let results = apply_fixes_with_fs(&diagnostics, false, false, Some(fs_clone)).unwrap();

        assert_eq!(results.len(), 2);

        // Verify both files were updated
        let content1 = mock_fs
            .read_to_string(std::path::Path::new("/project/file1.md"))
            .unwrap();
        let content2 = mock_fs
            .read_to_string(std::path::Path::new("/project/file2.md"))
            .unwrap();

        assert_eq!(content1, "new1");
        assert_eq!(content2, "new2");
    }

    #[test]
    fn test_apply_fixes_with_mock_fs_file_not_found() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();
        // Don't add the file - it should error

        let diagnostics = vec![make_diagnostic(
            "/project/nonexistent.md",
            vec![Fix::replace(0, 4, "new", "Fix", true)],
        )];

        let result = apply_fixes_with_fs(&diagnostics, false, false, Some(Arc::new(mock_fs)));

        assert!(result.is_err());
    }

    #[test]
    fn test_apply_fixes_with_mock_fs_no_changes() {
        use crate::fs::MockFileSystem;

        let mock_fs = MockFileSystem::new();
        mock_fs.add_file("/project/test.md", "unchanged");

        // Diagnostic with no fixes
        let diagnostics = vec![Diagnostic {
            level: DiagnosticLevel::Error,
            message: "No fix available".to_string(),
            file: PathBuf::from("/project/test.md"),
            line: 1,
            column: 1,
            rule: "TEST-001".to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
        }];

        let results =
            apply_fixes_with_fs(&diagnostics, false, false, Some(Arc::new(mock_fs))).unwrap();

        // No fixes means no results
        assert!(results.is_empty());
    }

    // ===== Edge Case: Ordering and Overlap Tests =====

    #[test]
    fn test_fix_three_non_overlapping_descending() {
        let content = "aaaa_bbbb_cccc_dddd_eeee_ffff";
        // Fixes at positions 20-24, 10-14, 0-4 (already sorted descending)
        let fixes = vec![
            Fix::replace(20, 24, "EEEE", "Fix third", true),
            Fix::replace(10, 14, "CCCC", "Fix second", true),
            Fix::replace(0, 4, "AAAA", "Fix first", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        let (result, applied) = apply_fixes_to_content(content, &fix_refs);

        assert_eq!(applied.len(), 3);
        assert!(result.contains("AAAA"));
        assert!(result.contains("CCCC"));
        assert!(result.contains("EEEE"));
    }

    #[test]
    fn test_fix_overlapping_middle_skipped() {
        let content = "0123456789abcdef";
        // Sorted descending: 10-14 first, then 6-12 (overlaps), then 0-4
        let fixes = vec![
            Fix::replace(10, 14, "XX", "Fix at 10", true),
            Fix::replace(6, 12, "YY", "Fix at 6 (overlaps)", true),
            Fix::replace(0, 4, "ZZ", "Fix at 0", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        let (result, applied) = apply_fixes_to_content(content, &fix_refs);

        // Fix at 10-14 applied, fix at 6-12 skipped (end_byte 12 > last_start 10),
        // fix at 0-4 applied (end_byte 4 < last_start 10, but after the overlap skip,
        // last_start is still 10 from the first fix)
        assert_eq!(applied.len(), 2, "Only two non-overlapping fixes should apply");
        assert!(result.contains("XX"), "Fix at 10-14 should be applied");
        assert!(result.contains("ZZ"), "Fix at 0-4 should be applied");
        assert!(!result.contains("YY"), "Overlapping fix at 6-12 should be skipped");
    }

    #[test]
    fn test_fix_adjacent_fixes() {
        let content = "hello world";
        // Sorted descending: (5,6) first, then (0,5)
        // end_byte 5 == last_start 5 -> NOT > so both should apply
        let fixes = vec![
            Fix::replace(5, 6, "_", "Replace space", true),
            Fix::replace(0, 5, "HELLO", "Uppercase first word", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        let (result, applied) = apply_fixes_to_content(content, &fix_refs);

        assert_eq!(applied.len(), 2, "Adjacent fixes should both apply");
        assert_eq!(result, "HELLO_world");
    }

    #[test]
    fn test_fix_same_position_insertions() {
        let content = "hello";
        // Two insertions at position 5 (end of string)
        // Both have start==end==5, sorted descending they are equal
        // First gets applied (last_start = 5), second has end_byte 5 <= last_start 5
        // Wait: end_byte > last_start check: 5 > 5 is false, so second also applies
        let fixes = vec![
            Fix::insert(5, "!", "Add exclamation", true),
            Fix::insert(5, "?", "Add question", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        let (result, applied) = apply_fixes_to_content(content, &fix_refs);

        assert_eq!(applied.len(), 2, "Same-position insertions should both apply");
        // Both insertions at position 5: first "!" is inserted, then "?" is inserted
        // at the same position in the original content (but string has shifted)
        assert!(result.contains("!"), "First insertion should be present");
        assert!(result.contains("?"), "Second insertion should be present");
    }

    #[test]
    fn test_fix_length_changing_preserves_positions() {
        let content = "short___long_text";
        // Sorted descending: 12-17 first, then 0-5
        let fixes = vec![
            Fix::replace(12, 17, "replacement_that_is_longer", "Fix second", true),
            Fix::replace(0, 5, "S", "Fix first", true),
        ];
        let fix_refs: Vec<&Fix> = fixes.iter().collect();

        let (result, applied) = apply_fixes_to_content(content, &fix_refs);

        assert_eq!(applied.len(), 2, "Both fixes should apply");
        assert!(
            result.starts_with("S"),
            "First 5 chars should be replaced with 'S'"
        );
        assert!(
            result.contains("replacement_that_is_longer"),
            "Later range should be replaced"
        );
    }

    #[test]
    fn test_fix_end_before_start_skipped() {
        let content = "hello";
        // Invalid fix: start_byte > end_byte (end_byte < start_byte)
        let fix = Fix {
            start_byte: 3,
            end_byte: 1,
            replacement: "X".to_string(),
            description: "Invalid fix".to_string(),
            safe: true,
        };

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "hello", "Content should be unchanged");
        assert!(applied.is_empty(), "Invalid fix should be skipped");
    }

    #[test]
    fn test_fix_unicode_content_boundaries() {
        // \u{00e9} is precomposed e-acute: 2 bytes (0xc3, 0xa9)
        let content = "caf\u{00e9} is great";
        // 'c'=0, 'a'=1, 'f'=2, 'e\u{0301}'=3-4 (2 bytes)
        // Fix byte 3 to 5 (the e-acute), replace with "e"
        let fix = Fix::replace(3, 5, "e", "Normalize accent", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "cafe is great");
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn test_fix_crlf_content() {
        let content = "hello\r\nworld";
        // Replace \r\n (bytes 5-7) with \n
        let fix = Fix::replace(5, 7, "\n", "Normalize line ending", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "hello\nworld");
        assert_eq!(applied.len(), 1);
    }

    #[test]
    fn test_fix_utf8_boundary_skip() {
        // \u{00e9} = 2 bytes: 0xc3 at byte 3, 0xa9 at byte 4
        let content = "caf\u{00e9}";
        // Fix starting at byte 4 (mid-codepoint continuation byte) should be skipped
        let fix = Fix::replace(4, 5, "X", "Mid-codepoint fix", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(
            result,
            "caf\u{00e9}",
            "Content should be unchanged when fix targets mid-codepoint"
        );
        assert!(
            applied.is_empty(),
            "Fix at non-char-boundary should be skipped"
        );
    }

    #[test]
    fn test_fix_replacement_with_unicode() {
        let content = "hello world";
        let fix = Fix::replace(6, 11, "\u{4e16}\u{754c}", "Replace with CJK", true);

        let (result, applied) = apply_fixes_to_content(content, &[&fix]);

        assert_eq!(result, "hello \u{4e16}\u{754c}");
        assert_eq!(applied.len(), 1);
    }
}
