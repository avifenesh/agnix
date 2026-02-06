//! AGENTS.md validation schema helpers
//!
//! Provides detection functions for:
//! - AGM-001: Valid Markdown Structure (unclosed code blocks, malformed links)
//! - AGM-002: Missing Section Headers
//! - AGM-003: Character Limit (12000 chars for Windsurf compatibility)
//! - AGM-004: Missing Project Context
//! - AGM-005: Platform-Specific Features Without Guard
//! - AGM-006: Nested AGENTS.md Hierarchy

use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::regex_util::static_regex;

static_regex!(fn code_block_pattern, r"^```");
static_regex!(fn link_pattern, r"\[([^\]]*)\](?:\(([^)]*)\)?|\[([^\]]*)\]?)");
static_regex!(fn markdown_header_pattern, r"^#+\s+.+");
static_regex!(fn project_context_pattern, r"(?im)^#+\s*(project|overview|about|description|introduction|summary|this\s+(project|repository|repo))\b");
static_regex!(fn platform_guard_pattern, r#"(?im)^(?:#+\s*|<!--\s*)(claude|cursor|codex|opencode|cline|copilot|windsurf)(?:\s+code)?(?:\s+specific|\s+only)?(?:\s*-->)?"#);
static_regex!(fn platform_feature_pattern, r#"(?im)(?:^\s*-?\s*(?:type|event):\s*(?:PreToolExecution|PostToolExecution|Notification|Stop|SubagentStop)\b|^\s*context:\s*fork\b|^\s*agent:\s*\S+|^\s*allowed-tools:\s*.+|\.cursor/|@rules)"#);

// ============================================================================
// AGM-001: Valid Markdown Structure
// ============================================================================

/// Markdown validity issue found in content
#[derive(Debug, Clone)]
pub struct MarkdownValidityIssue {
    pub line: usize,
    pub column: usize,
    pub issue_type: MarkdownIssueType,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownIssueType {
    UnclosedCodeBlock,
    MalformedLink,
}

/// Check markdown validity (for AGM-001)
///
/// Detects:
/// - Unclosed code blocks (unmatched ```)
/// - Malformed links ([text]( without closing)
pub fn check_markdown_validity(content: &str) -> Vec<MarkdownValidityIssue> {
    let mut results = Vec::new();
    let code_pattern = code_block_pattern();
    let link_pattern = link_pattern();

    // Track code block state
    let mut in_code_block = false;
    let mut code_block_start_line = 0;

    for (line_num, line) in content.lines().enumerate() {
        // Check for code block markers
        if code_pattern.is_match(line) {
            if in_code_block {
                // Closing code block
                in_code_block = false;
            } else {
                // Opening code block
                in_code_block = true;
                code_block_start_line = line_num + 1;
            }
        }

        // Check for malformed links (only outside code blocks)
        if !in_code_block {
            // Look for [text]( patterns that don't close properly
            for mat in link_pattern.find_iter(line) {
                let matched_str = mat.as_str();
                // Check if link is properly closed
                if matched_str.contains("](") && !matched_str.ends_with(')') {
                    // Verify it's actually malformed (no closing paren on same line after match)
                    let after_match = &line[mat.end()..];
                    if !after_match.starts_with(')') && !matched_str.ends_with(')') {
                        // Check if this is actually incomplete
                        let has_close_paren =
                            matched_str.matches('(').count() == matched_str.matches(')').count();
                        if !has_close_paren {
                            results.push(MarkdownValidityIssue {
                                line: line_num + 1,
                                column: mat.start() + 1,
                                issue_type: MarkdownIssueType::MalformedLink,
                                description:
                                    "Malformed markdown link (missing closing parenthesis)"
                                        .to_string(),
                            });
                        }
                    }
                }
                // Check for [text][ without closing bracket
                if matched_str.contains("][") && !matched_str.ends_with(']') {
                    let has_close_bracket =
                        matched_str.matches('[').count() == matched_str.matches(']').count();
                    if !has_close_bracket {
                        results.push(MarkdownValidityIssue {
                            line: line_num + 1,
                            column: mat.start() + 1,
                            issue_type: MarkdownIssueType::MalformedLink,
                            description:
                                "Malformed markdown link reference (missing closing bracket)"
                                    .to_string(),
                        });
                    }
                }
            }
        }
    }

    // Check if code block was never closed
    if in_code_block {
        results.push(MarkdownValidityIssue {
            line: code_block_start_line,
            column: 0,
            issue_type: MarkdownIssueType::UnclosedCodeBlock,
            description: "Unclosed code block (missing closing ```)".to_string(),
        });
    }

    results
}

// ============================================================================
// AGM-002: Missing Section Headers
// ============================================================================

/// Section header issue
#[derive(Debug, Clone)]
pub struct SectionHeaderIssue {
    pub line: usize,
    pub column: usize,
    pub description: String,
    pub suggestion: String,
}

/// Check for missing section headers (for AGM-002)
///
/// AGENTS.md should have clear section headers for organization
pub fn check_section_headers(content: &str) -> Option<SectionHeaderIssue> {
    let pattern = markdown_header_pattern();

    // Skip empty or whitespace-only content
    if content.trim().is_empty() {
        return None;
    }

    // Check if file has any headers at all
    let has_headers = content.lines().any(|line| pattern.is_match(line));

    if !has_headers {
        Some(SectionHeaderIssue {
            line: 1,
            column: 0,
            description: "No markdown headers found in AGENTS instruction file".to_string(),
            suggestion: "Add section headers (# Title, ## Section) for better organization"
                .to_string(),
        })
    } else {
        None
    }
}

// ============================================================================
// AGM-003: Character Limit
// ============================================================================

/// Character limit exceeded result
#[derive(Debug, Clone)]
pub struct CharacterLimitExceeded {
    pub char_count: usize,
    pub limit: usize,
}

/// Character limit for Windsurf compatibility
pub const WINDSURF_CHAR_LIMIT: usize = 12000;

/// Check if content exceeds character limit (for AGM-003)
///
/// Windsurf requires rules files under 12000 characters
pub fn check_character_limit(content: &str, limit: usize) -> Option<CharacterLimitExceeded> {
    let char_count = content.len();

    if char_count > limit {
        Some(CharacterLimitExceeded { char_count, limit })
    } else {
        None
    }
}

// ============================================================================
// AGM-004: Missing Project Context
// ============================================================================

/// Missing project context result
#[derive(Debug, Clone)]
pub struct MissingProjectContext {
    pub line: usize,
    pub column: usize,
    pub description: String,
    pub suggestion: String,
}

/// Check for missing project context (for AGM-004)
///
/// AGENTS.md should describe project purpose/stack
pub fn check_project_context(content: &str) -> Option<MissingProjectContext> {
    let pattern = project_context_pattern();

    // Skip empty content
    if content.trim().is_empty() {
        return None;
    }

    // Check for project context section
    let has_project_context = pattern.is_match(content);

    // Also check for common project description patterns in content
    let content_lower = content.to_lowercase();
    let has_project_description = content_lower.contains("this project")
        || content_lower.contains("this repository")
        || content_lower.contains("this repo")
        || content_lower.contains("the project")
        || content_lower.contains("# project");

    if !has_project_context && !has_project_description {
        Some(MissingProjectContext {
            line: 1,
            column: 0,
            description: "Missing project context section in AGENTS instruction file".to_string(),
            suggestion:
                "Add a '# Project' or '## Overview' section describing the project purpose and tech stack"
                    .to_string(),
        })
    } else {
        None
    }
}

// ============================================================================
// AGM-005: Platform-Specific Features Without Guard
// ============================================================================

/// Unguarded platform feature
#[derive(Debug, Clone)]
pub struct UnguardedPlatformFeature {
    pub line: usize,
    pub column: usize,
    pub feature: String,
    pub platform: String,
    pub description: String,
}

/// Find platform-specific features without guard comments (for AGM-005)
///
/// Platform-specific instructions should be labeled with guard comments
pub fn find_unguarded_platform_features(content: &str) -> Vec<UnguardedPlatformFeature> {
    let mut results = Vec::new();
    let guard_pattern = platform_guard_pattern();
    let feature_pattern = platform_feature_pattern();

    // Track if we're in a guarded section
    let mut in_guarded_section = false;
    let mut current_platform: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        // Check if this line is a platform guard
        if let Some(cap) = guard_pattern.captures(line) {
            in_guarded_section = true;
            current_platform = cap.get(1).map(|m| m.as_str().to_string());
            continue;
        }

        // Check if we hit a new section header (non-guard)
        if line.starts_with('#') && !guard_pattern.is_match(line) {
            // New section, reset guard status
            in_guarded_section = false;
            current_platform = None;
        }

        // Check for platform-specific features
        if let Some(mat) = feature_pattern.find(line) {
            let matched_str = mat.as_str().trim();

            // Determine which platform this feature belongs to
            let (feature, platform) = if matched_str.contains("PreToolExecution")
                || matched_str.contains("PostToolExecution")
                || matched_str.contains("Notification")
                || matched_str.contains("Stop")
                || matched_str.contains("SubagentStop")
            {
                ("hooks".to_string(), "Claude Code".to_string())
            } else if matched_str.contains("context:") && matched_str.contains("fork") {
                ("context:fork".to_string(), "Claude Code".to_string())
            } else if matched_str.contains("agent:") {
                ("agent field".to_string(), "Claude Code".to_string())
            } else if matched_str.contains("allowed-tools:") {
                ("allowed-tools".to_string(), "Claude Code".to_string())
            } else if matched_str.contains(".cursor/") || matched_str.contains("@rules") {
                ("Cursor paths/rules".to_string(), "Cursor".to_string())
            } else {
                continue;
            };

            // Only report if not in a guarded section for this platform
            if !in_guarded_section
                || current_platform
                    .as_ref()
                    .is_none_or(|p| !platform.to_lowercase().contains(&p.to_lowercase()))
            {
                results.push(UnguardedPlatformFeature {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    feature: feature.clone(),
                    platform: platform.clone(),
                    description: format!(
                        "{} feature '{}' without platform guard",
                        platform, feature
                    ),
                });
            }
        }
    }

    results
}

// ============================================================================
// AGM-006: Nested AGENTS.md Hierarchy
// ============================================================================

/// Nested AGENTS.md file info
#[derive(Debug, Clone)]
pub struct NestedAgentsMd {
    pub path: std::path::PathBuf,
    pub depth: usize,
}

/// Find AGENTS.md files when multiple exist in the directory tree (for AGM-006).
///
/// Returns all AGENTS.md files when more than one exists, including siblings.
pub fn find_multiple_agents_md(paths: &[std::path::PathBuf]) -> Vec<NestedAgentsMd> {
    // Filter to only AGENTS.md files
    let agents_files: Vec<_> = paths
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name == "AGENTS.md")
        })
        .collect();

    // If there's only one or zero AGENTS.md files, no nesting issue
    if agents_files.len() <= 1 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for path in agents_files {
        let depth = path.components().count();
        let path_buf = path.to_path_buf();
        if seen.insert(path_buf.clone()) {
            results.push(NestedAgentsMd {
                path: path_buf,
                depth,
            });
        }
    }

    results.sort_by_key(|item| item.depth);

    results
}

#[deprecated(note = "Use find_multiple_agents_md; this returns all AGENTS.md files when >1 exist.")]
pub fn find_nested_agents_md(paths: &[std::path::PathBuf]) -> Vec<NestedAgentsMd> {
    find_multiple_agents_md(paths)
}

/// Check if an AGENTS.md file has parent AGENTS.md files in its ancestry
///
/// Returns the paths of parent AGENTS.md files if they exist
pub fn check_agents_md_hierarchy(
    current_path: &Path,
    all_paths: &[std::path::PathBuf],
) -> Vec<std::path::PathBuf> {
    let mut parents = Vec::new();

    // Get the directory containing this AGENTS.md
    let Some(current_dir) = current_path.parent() else {
        return parents;
    };

    // Find all AGENTS.md files
    let agents_files: Vec<_> = all_paths
        .iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name == "AGENTS.md")
        })
        .filter(|p| *p != current_path)
        .collect();

    // Check if any AGENTS.md is an ancestor
    for agents_path in agents_files {
        if let Some(agents_dir) = agents_path.parent() {
            if current_dir.starts_with(agents_dir) {
                parents.push(agents_path.clone());
            }
        }
    }

    parents
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_regex_patterns_compile() {
        let _ = code_block_pattern();
        let _ = link_pattern();
        let _ = markdown_header_pattern();
        let _ = project_context_pattern();
        let _ = platform_guard_pattern();
        let _ = platform_feature_pattern();
    }

    // ===== AGM-001: Valid Markdown Structure =====

    #[test]
    fn test_unclosed_code_block() {
        let content = r#"# Example
```rust
fn main() {}
"#;
        let results = check_markdown_validity(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue_type, MarkdownIssueType::UnclosedCodeBlock);
    }

    #[test]
    fn test_closed_code_block() {
        let content = r#"# Example
```rust
fn main() {}
```
"#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_code_blocks_valid() {
        let content = r#"# Examples
```rust
fn main() {}
```

```python
print("hello")
```
"#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_malformed_link_missing_paren() {
        let content = r#"Check [this link](http://example.com for more info."#;
        let results = check_markdown_validity(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].issue_type, MarkdownIssueType::MalformedLink);
    }

    #[test]
    fn test_valid_link() {
        let content = r#"Check [this link](http://example.com) for more info."#;
        let results = check_markdown_validity(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_code_block_in_link_check_ignored() {
        // Links inside code blocks shouldn't be validated
        let content = r#"```markdown
[broken link](no closing
```
"#;
        let results = check_markdown_validity(content);
        // No errors because the "malformed link" is inside a code block
        assert!(results.is_empty());
    }

    // ===== AGM-002: Missing Section Headers =====

    #[test]
    fn test_no_headers() {
        let content = "Just plain text without any headers.";
        let result = check_section_headers(content);
        assert!(result.is_some());
        assert!(result.unwrap().description.contains("No markdown headers"));
    }

    #[test]
    fn test_has_headers() {
        let content = "# Main Title\n\nSome content here.";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_content_no_header_issue() {
        let content = "";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_whitespace_only_no_header_issue() {
        let content = "   \n\n   ";
        let result = check_section_headers(content);
        assert!(result.is_none());
    }

    // ===== AGM-003: Character Limit =====

    #[test]
    fn test_under_char_limit() {
        let content = "x".repeat(11000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_none());
    }

    #[test]
    fn test_over_char_limit() {
        let content = "x".repeat(13000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_some());
        let exceeded = result.unwrap();
        assert_eq!(exceeded.char_count, 13000);
        assert_eq!(exceeded.limit, WINDSURF_CHAR_LIMIT);
    }

    #[test]
    fn test_exact_char_limit() {
        let content = "x".repeat(12000);
        let result = check_character_limit(&content, WINDSURF_CHAR_LIMIT);
        assert!(result.is_none());
    }

    // ===== AGM-004: Missing Project Context =====

    #[test]
    fn test_missing_project_context() {
        let content = r#"# Build Commands
Run npm install and npm build.
"#;
        let result = check_project_context(content);
        assert!(result.is_some());
        assert!(
            result
                .unwrap()
                .description
                .contains("Missing project context")
        );
    }

    #[test]
    fn test_has_project_header() {
        let content = r#"# Project
This is a linter for agent configurations.

## Commands
Run npm test.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_overview_header() {
        let content = r#"## Overview
A comprehensive validation tool.

## Usage
Run the CLI.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_project_mention_in_content() {
        let content = r#"# Guidelines
This project validates agent configurations.

## Build
Use cargo build.
"#;
        let result = check_project_context(content);
        assert!(result.is_none());
    }

    // ===== AGM-005: Unguarded Platform Features =====

    #[test]
    fn test_unguarded_hooks() {
        let content = r#"# Config
- type: PreToolExecution
  command: echo "test"
"#;
        let results = find_unguarded_platform_features(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].feature, "hooks");
        assert_eq!(results[0].platform, "Claude Code");
    }

    #[test]
    fn test_guarded_hooks() {
        let content = r#"## Claude Code Specific
- type: PreToolExecution
  command: echo "test"
"#;
        let results = find_unguarded_platform_features(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_unguarded_context_fork() {
        let content = r#"---
name: test
context: fork
---
Body"#;
        let results = find_unguarded_platform_features(content);
        assert!(results.iter().any(|r| r.feature == "context:fork"));
    }

    #[test]
    fn test_unguarded_agent_field() {
        let content = r#"agent: security-reviewer"#;
        let results = find_unguarded_platform_features(content);
        assert!(results.iter().any(|r| r.feature == "agent field"));
    }

    #[test]
    fn test_guard_section_ends() {
        let content = r#"## Claude Code Specific
- type: Stop
  command: cleanup

## Other Settings
agent: something
"#;
        let results = find_unguarded_platform_features(content);
        // agent field is in a non-guarded section
        assert!(results.iter().any(|r| r.feature == "agent field"));
        // hooks are in guarded section
        assert!(!results.iter().any(|r| r.feature == "hooks"));
    }

    // ===== AGM-006: Nested AGENTS.md Hierarchy =====

    #[test]
    fn test_single_agents_md_no_multiple() {
        let paths = vec![PathBuf::from("project/AGENTS.md")];
        let results = find_multiple_agents_md(&paths);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_agents_md_with_parent() {
        let paths = vec![
            PathBuf::from("project/AGENTS.md"),
            PathBuf::from("project/subdir/AGENTS.md"),
        ];
        let results = find_multiple_agents_md(&paths);
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|r| r.path.to_string_lossy().contains("project/AGENTS.md"))
        );
        assert!(
            results
                .iter()
                .any(|r| r.path.to_string_lossy().contains("subdir"))
        );
    }

    #[test]
    fn test_multiple_agents_md_with_hierarchy() {
        let paths = vec![
            PathBuf::from("project/AGENTS.md"),
            PathBuf::from("project/a/AGENTS.md"),
            PathBuf::from("project/a/b/AGENTS.md"),
        ];
        let results = find_multiple_agents_md(&paths);
        // Should detect all AGENTS.md files
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_multiple_agents_md_no_duplicates() {
        // Test case for AGM-006: Ensure each AGENTS.md file is only reported once
        let paths = vec![
            PathBuf::from("project/AGENTS.md"),
            PathBuf::from("project/a/AGENTS.md"),
            PathBuf::from("project/a/b/AGENTS.md"),
        ];
        let results = find_multiple_agents_md(&paths);

        // Should detect all AGENTS.md files
        assert_eq!(results.len(), 3);

        // Verify no duplicates by checking each path appears only once
        let mut seen_paths = HashSet::new();
        for result in &results {
            let path_str = result.path.to_string_lossy().to_string();
            assert!(
                seen_paths.insert(path_str.clone()),
                "Duplicate path found: {}",
                path_str
            );
        }

        // Verify the correct files are reported
        let result_paths: Vec<String> = results
            .iter()
            .map(|r| r.path.to_string_lossy().to_string())
            .collect();
        assert!(result_paths.iter().any(|p| p.contains("project/AGENTS.md")));
        assert!(
            result_paths
                .iter()
                .any(|p| p.contains("project/a/AGENTS.md"))
        );
        assert!(
            result_paths
                .iter()
                .any(|p| p.contains("project/a/b/AGENTS.md"))
        );
    }

    #[test]
    fn test_sibling_agents_md_multiple() {
        let paths = vec![
            PathBuf::from("project-a/AGENTS.md"),
            PathBuf::from("project-b/AGENTS.md"),
        ];
        let results = find_multiple_agents_md(&paths);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_check_hierarchy_has_parent() {
        let current = PathBuf::from("project/subdir/AGENTS.md");
        let all_paths = vec![
            PathBuf::from("project/AGENTS.md"),
            PathBuf::from("project/subdir/AGENTS.md"),
        ];
        let parents = check_agents_md_hierarchy(&current, &all_paths);
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0], PathBuf::from("project/AGENTS.md"));
    }

    #[test]
    fn test_check_hierarchy_no_parent() {
        let current = PathBuf::from("project/AGENTS.md");
        let all_paths = vec![PathBuf::from("project/AGENTS.md")];
        let parents = check_agents_md_hierarchy(&current, &all_paths);
        assert!(parents.is_empty());
    }
}
