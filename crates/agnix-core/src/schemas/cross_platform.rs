//! Cross-platform validation schema helpers
//!
//! Provides detection functions for:
//! - XP-001: Claude-specific features in AGENTS.md
//! - XP-002: AGENTS.md markdown structure validation
//! - XP-003: Hard-coded platform paths in configs

use regex::Regex;
use std::sync::OnceLock;

// Static patterns initialized once
static CLAUDE_HOOKS_PATTERN: OnceLock<Regex> = OnceLock::new();
static CONTEXT_FORK_PATTERN: OnceLock<Regex> = OnceLock::new();
static AGENT_FIELD_PATTERN: OnceLock<Regex> = OnceLock::new();
static ALLOWED_TOOLS_PATTERN: OnceLock<Regex> = OnceLock::new();
static HARD_CODED_PATH_PATTERN: OnceLock<Regex> = OnceLock::new();
static MARKDOWN_HEADER_PATTERN: OnceLock<Regex> = OnceLock::new();

// ============================================================================
// XP-001: Claude-Specific Features Detection
// ============================================================================

/// Claude-specific feature found in content
#[derive(Debug, Clone)]
pub struct ClaudeSpecificFeature {
    pub line: usize,
    pub column: usize,
    pub feature: String,
    pub description: String,
}

fn claude_hooks_pattern() -> &'static Regex {
    CLAUDE_HOOKS_PATTERN.get_or_init(|| {
        // Match hooks configuration patterns in markdown/YAML
        Regex::new(r"(?im)^\s*-?\s*(?:type|event):\s*(?:PreToolExecution|PostToolExecution|Notification|Stop|SubagentStop)\b").unwrap()
    })
}

fn context_fork_pattern() -> &'static Regex {
    CONTEXT_FORK_PATTERN.get_or_init(|| {
        // Match context: fork in YAML frontmatter or content
        Regex::new(r"(?im)^\s*context:\s*fork\b").unwrap()
    })
}

fn agent_field_pattern() -> &'static Regex {
    AGENT_FIELD_PATTERN.get_or_init(|| {
        // Match agent: field in YAML frontmatter
        Regex::new(r"(?im)^\s*agent:\s*(?:Explore|Plan|general-purpose)\b").unwrap()
    })
}

fn allowed_tools_pattern() -> &'static Regex {
    ALLOWED_TOOLS_PATTERN.get_or_init(|| {
        // Match allowed-tools: field (Claude Code specific)
        Regex::new(r"(?im)^\s*allowed-tools:\s*.+").unwrap()
    })
}

/// Find Claude-specific features in content (for XP-001)
///
/// Detects features that only work in Claude Code but not in other platforms
/// that read AGENTS.md (Codex CLI, OpenCode, GitHub Copilot, Cursor, Cline).
pub fn find_claude_specific_features(content: &str) -> Vec<ClaudeSpecificFeature> {
    let mut results = Vec::new();

    // Iterate directly over lines without collecting to Vec (memory optimization)
    for (line_num, line) in content.lines().enumerate() {
        // Check for hooks patterns
        if let Some(mat) = claude_hooks_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start(),
                feature: "hooks".to_string(),
                description: "Claude Code hooks are not supported by other AGENTS.md readers"
                    .to_string(),
            });
        }

        // Check for context: fork
        if let Some(mat) = context_fork_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start(),
                feature: "context:fork".to_string(),
                description: "Context forking is Claude Code specific".to_string(),
            });
        }

        // Check for agent: field
        if let Some(mat) = agent_field_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start(),
                feature: "agent".to_string(),
                description: "Agent field is Claude Code specific".to_string(),
            });
        }

        // Check for allowed-tools: field
        if let Some(mat) = allowed_tools_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start(),
                feature: "allowed-tools".to_string(),
                description: "Tool restrictions are Claude Code specific".to_string(),
            });
        }
    }

    results
}

// ============================================================================
// XP-002: AGENTS.md Markdown Structure Validation
// ============================================================================

/// Markdown structure issue
#[derive(Debug, Clone)]
pub struct MarkdownStructureIssue {
    pub line: usize,
    pub column: usize,
    pub issue: String,
    pub suggestion: String,
}

fn markdown_header_pattern() -> &'static Regex {
    MARKDOWN_HEADER_PATTERN.get_or_init(|| Regex::new(r"^#+\s+.+").unwrap())
}

/// Check AGENTS.md markdown structure (for XP-002)
///
/// Validates that AGENTS.md follows good markdown conventions for
/// cross-platform compatibility.
pub fn check_markdown_structure(content: &str) -> Vec<MarkdownStructureIssue> {
    let mut results = Vec::new();
    let pattern = markdown_header_pattern();

    // Check if file has any headers at all (single pass)
    let has_headers = content.lines().any(|line| pattern.is_match(line));

    if !has_headers && !content.trim().is_empty() {
        results.push(MarkdownStructureIssue {
            line: 1,
            column: 0,
            issue: "No markdown headers found".to_string(),
            suggestion: "Add headers (# Section) to structure the document for better readability"
                .to_string(),
        });
    }

    // Check for proper header hierarchy (no skipping levels)
    let mut last_level = 0;
    for (line_num, line) in content.lines().enumerate() {
        if pattern.is_match(line) {
            let current_level = line.chars().take_while(|&c| c == '#').count();

            // Warn if header level jumps by more than 1
            if last_level > 0 && current_level > last_level + 1 {
                results.push(MarkdownStructureIssue {
                    line: line_num + 1,
                    column: 0,
                    issue: format!(
                        "Header level skipped from {} to {}",
                        last_level, current_level
                    ),
                    suggestion: format!(
                        "Use h{} instead of h{} for proper hierarchy",
                        last_level + 1,
                        current_level
                    ),
                });
            }

            last_level = current_level;
        }
    }

    results
}

// ============================================================================
// XP-003: Hard-Coded Platform Paths Detection
// ============================================================================

/// Hard-coded platform path found in content
#[derive(Debug, Clone)]
pub struct HardCodedPath {
    pub line: usize,
    pub column: usize,
    pub path: String,
    pub platform: String,
}

fn hard_coded_path_pattern() -> &'static Regex {
    HARD_CODED_PATH_PATTERN.get_or_init(|| {
        // Match common platform-specific config directories
        Regex::new(r"(?i)(?:\.claude/|\.opencode/|\.cursor/|\.cline/|\.github/copilot/)").unwrap()
    })
}

/// Find hard-coded platform-specific paths (for XP-003)
///
/// Detects paths like `.claude/`, `.opencode/`, `.cursor/` that may cause
/// portability issues when the same config is used across different platforms.
pub fn find_hard_coded_paths(content: &str) -> Vec<HardCodedPath> {
    let mut results = Vec::new();
    let pattern = hard_coded_path_pattern();

    for (line_num, line) in content.lines().enumerate() {
        for mat in pattern.find_iter(line) {
            let path = mat.as_str().to_lowercase();
            let platform = if path.contains(".claude") {
                "Claude Code"
            } else if path.contains(".opencode") {
                "OpenCode"
            } else if path.contains(".cursor") {
                "Cursor"
            } else if path.contains(".cline") {
                "Cline"
            } else if path.contains(".github/copilot") {
                "GitHub Copilot"
            } else {
                "Unknown"
            };

            results.push(HardCodedPath {
                line: line_num + 1,
                column: mat.start(),
                path: mat.as_str().to_string(),
                platform: platform.to_string(),
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== XP-001: Claude-Specific Features =====

    #[test]
    fn test_detect_hooks_in_content() {
        let content = r#"# Agent Config
- type: PreToolExecution
  command: echo "test"
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].feature, "hooks");
    }

    #[test]
    fn test_detect_context_fork() {
        let content = r#"---
name: test
context: fork
agent: Explore
---
Body"#;
        let results = find_claude_specific_features(content);
        assert!(results.iter().any(|r| r.feature == "context:fork"));
    }

    #[test]
    fn test_detect_agent_field() {
        let content = r#"---
name: test
agent: general-purpose
---
Body"#;
        let results = find_claude_specific_features(content);
        assert!(results.iter().any(|r| r.feature == "agent"));
    }

    #[test]
    fn test_detect_allowed_tools() {
        let content = r#"---
name: test
allowed-tools: Read Write Bash
---
Body"#;
        let results = find_claude_specific_features(content);
        assert!(results.iter().any(|r| r.feature == "allowed-tools"));
    }

    #[test]
    fn test_no_claude_features_in_clean_content() {
        let content = r#"# Project Guidelines

Follow the coding style guide.

## Commands
- npm run build
- npm run test
"#;
        let results = find_claude_specific_features(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_claude_features() {
        let content = r#"---
name: test
context: fork
agent: Plan
allowed-tools: Read Write
---
Body"#;
        let results = find_claude_specific_features(content);
        // Should detect context:fork, agent, and allowed-tools
        assert!(results.len() >= 3);
    }

    // ===== XP-002: Markdown Structure =====

    #[test]
    fn test_detect_no_headers() {
        let content = "Just some text without any headers.\nMore text here.";
        let results = check_markdown_structure(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].issue.contains("No markdown headers"));
    }

    #[test]
    fn test_valid_markdown_structure() {
        let content = r#"# Main Title

Some content here.

## Section One

More content.

### Subsection

Details.
"#;
        let results = check_markdown_structure(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_detect_skipped_header_level() {
        let content = r#"# Title

#### Skipped to h4
"#;
        let results = check_markdown_structure(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].issue.contains("skipped"));
    }

    #[test]
    fn test_empty_content_no_issue() {
        let content = "";
        let results = check_markdown_structure(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_whitespace_only_no_issue() {
        let content = "   \n\n   ";
        let results = check_markdown_structure(content);
        assert!(results.is_empty());
    }

    // ===== XP-003: Hard-Coded Paths =====

    #[test]
    fn test_detect_claude_path() {
        let content = "Check the config at .claude/settings.json";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].platform, "Claude Code");
    }

    #[test]
    fn test_detect_opencode_path() {
        let content = "OpenCode stores settings in .opencode/config.yaml";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].platform, "OpenCode");
    }

    #[test]
    fn test_detect_cursor_path() {
        let content = "Cursor rules are in .cursor/rules/";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].platform, "Cursor");
    }

    #[test]
    fn test_detect_multiple_platform_paths() {
        let content = r#"
Platform configs:
- Claude: .claude/settings.json
- Cursor: .cursor/rules/
- OpenCode: .opencode/config.yaml
"#;
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_no_hard_coded_paths() {
        let content = r#"# Project Config

Use environment variables for configuration.
Check the project root for settings.
"#;
        let results = find_hard_coded_paths(content);
        assert!(results.is_empty());
    }

    #[test]
    fn test_case_insensitive_path_detection() {
        let content = "Config at .CLAUDE/Settings.json";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
    }

    // ===== Additional edge case tests from review =====

    #[test]
    fn test_detect_hooks_event_variant() {
        // Tests event: variant in addition to type:
        let content = r#"hooks:
  - event: Notification
    command: notify-send
  - event: SubagentStop
    command: cleanup
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.feature == "hooks"));
    }

    #[test]
    fn test_detect_cline_path() {
        let content = "Cline config is in .cline/settings.json";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].platform, "Cline");
    }

    #[test]
    fn test_detect_github_copilot_path() {
        let content = "GitHub Copilot config at .github/copilot/config.json";
        let results = find_hard_coded_paths(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].platform, "GitHub Copilot");
    }

    #[test]
    fn test_extreme_header_skip_h1_to_h6() {
        let content = r#"# Title

###### Deep header
"#;
        let results = check_markdown_structure(content);
        assert_eq!(results.len(), 1);
        assert!(results[0].issue.contains("skipped from 1 to 6"));
    }

    #[test]
    fn test_no_false_positive_relative_paths() {
        let content = r#"# Project

Files are at:
- ./src/config.js
- ../parent/file.ts
- src/helpers/utils.rs
"#;
        let results = find_hard_coded_paths(content);
        assert!(results.is_empty());
    }
}
