//! Fix application engine for automatic corrections

use crate::diagnostics::{Diagnostic, Fix, LintError, LintResult};
use crate::file_utils::safe_read_file;
use std::collections::HashMap;
use std::path::PathBuf;

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
    // Group diagnostics by file
    let mut by_file: HashMap<PathBuf, Vec<&Diagnostic>> = HashMap::new();
    for diag in diagnostics {
        if diag.has_fixes() {
            by_file.entry(diag.file.clone()).or_default().push(diag);
        }
    }

    let mut results = Vec::new();

    for (path, file_diagnostics) in by_file {
        let original = safe_read_file(&path)?;

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
                std::fs::write(&path, &fixed).map_err(|e| LintError::FileWrite {
                    path: path.clone(),
                    source: e,
                })?;
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
        if fix.start_byte > result.len() || fix.end_byte > result.len() {
            continue;
        }
        if fix.start_byte > fix.end_byte {
            continue;
        }
        if !result.is_char_boundary(fix.start_byte) || !result.is_char_boundary(fix.end_byte) {
            continue;
        }
        // Skip overlapping fixes (sorted descending, so check against previous fix start)
        if fix.end_byte > last_start {
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
}
