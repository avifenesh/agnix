//! Cross-platform validation schema helpers
//!
//! Provides detection functions for:
//! - XP-001: Claude-specific features in AGENTS.md
//! - XP-002: AGENTS.md markdown structure validation
//! - XP-003: Hard-coded platform paths in configs

use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

// Static patterns initialized once
static CLAUDE_HOOKS_PATTERN: OnceLock<Regex> = OnceLock::new();
static CONTEXT_FORK_PATTERN: OnceLock<Regex> = OnceLock::new();
static AGENT_FIELD_PATTERN: OnceLock<Regex> = OnceLock::new();
static ALLOWED_TOOLS_PATTERN: OnceLock<Regex> = OnceLock::new();
static HARD_CODED_PATH_PATTERN: OnceLock<Regex> = OnceLock::new();
static MARKDOWN_HEADER_PATTERN: OnceLock<Regex> = OnceLock::new();

// XP-004: Build command patterns
static BUILD_COMMAND_PATTERN: OnceLock<Regex> = OnceLock::new();

// XP-005: Tool constraint patterns
static TOOL_ALLOW_PATTERN: OnceLock<Regex> = OnceLock::new();
static TOOL_DISALLOW_PATTERN: OnceLock<Regex> = OnceLock::new();

// XP-006: Layer type patterns
static LAYER_PRECEDENCE_PATTERN: OnceLock<Regex> = OnceLock::new();

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
        // Match any agent: field in YAML frontmatter (Claude Code specific)
        // The agent: field is used to spawn subagents, which is Claude Code exclusive
        Regex::new(r"(?im)^\s*agent:\s*\S+").unwrap()
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

// ============================================================================
// XP-004: Conflicting Build/Test Commands Detection
// ============================================================================

/// Package manager type for build commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl PackageManager {
    /// Get the display name for this package manager
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageManager::Npm => "npm",
            PackageManager::Pnpm => "pnpm",
            PackageManager::Yarn => "yarn",
            PackageManager::Bun => "bun",
        }
    }
}

/// Command type (build, test, install, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandType {
    Install,
    Build,
    Test,
    Run,
    Other,
}

/// A build command extracted from content
#[derive(Debug, Clone)]
pub struct BuildCommand {
    pub line: usize,
    pub column: usize,
    pub package_manager: PackageManager,
    pub command_type: CommandType,
    pub raw_command: String,
}

fn build_command_pattern() -> &'static Regex {
    BUILD_COMMAND_PATTERN.get_or_init(|| {
        // Match npm/pnpm/yarn/bun commands
        Regex::new(r"(?m)(?:^|\s|`)((?:npm|pnpm|yarn|bun)\s+(?:install|i|add|build|test|run|exec|ci)\b[^\n`]*)")
            .unwrap()
    })
}

/// Extract build commands from content (for XP-004)
///
/// Detects npm, pnpm, yarn, and bun commands in instruction files
pub fn extract_build_commands(content: &str) -> Vec<BuildCommand> {
    let mut results = Vec::new();
    let pattern = build_command_pattern();

    for (line_num, line) in content.lines().enumerate() {
        for caps in pattern.captures_iter(line) {
            // Get the captured command (group 1), not the full match
            let raw = match caps.get(1) {
                Some(m) => m.as_str().trim(),
                None => continue,
            };

            let column = caps.get(1).map(|m| m.start()).unwrap_or(0);

            // Determine package manager
            let package_manager = if raw.starts_with("npm") {
                PackageManager::Npm
            } else if raw.starts_with("pnpm") {
                PackageManager::Pnpm
            } else if raw.starts_with("yarn") {
                PackageManager::Yarn
            } else if raw.starts_with("bun") {
                PackageManager::Bun
            } else {
                continue;
            };

            // Determine command type
            let command_type = if raw.contains(" install")
                || raw.contains(" i ")
                || raw.contains(" add")
                || raw.contains(" ci")
            {
                CommandType::Install
            } else if raw.contains(" build") {
                CommandType::Build
            } else if raw.contains(" test") {
                CommandType::Test
            } else if raw.contains(" run") || raw.contains(" exec") {
                CommandType::Run
            } else {
                CommandType::Other
            };

            results.push(BuildCommand {
                line: line_num + 1,
                column,
                package_manager,
                command_type,
                raw_command: raw.to_string(),
            });
        }
    }

    results
}

/// Conflict between build commands across files
#[derive(Debug, Clone)]
pub struct BuildConflict {
    pub file1: std::path::PathBuf,
    pub file1_line: usize,
    pub file1_manager: PackageManager,
    pub file1_command: String,
    pub file2: std::path::PathBuf,
    pub file2_line: usize,
    pub file2_manager: PackageManager,
    pub file2_command: String,
    pub command_type: CommandType,
}

/// Detect conflicting build commands across instruction files (for XP-004)
///
/// Returns conflicts when different package managers are used for the same command type
pub fn detect_build_conflicts(
    files: &[(std::path::PathBuf, Vec<BuildCommand>)],
) -> Vec<BuildConflict> {
    let mut conflicts = Vec::new();

    // Group commands by type across all files
    for i in 0..files.len() {
        for j in (i + 1)..files.len() {
            let (file1, commands1) = &files[i];
            let (file2, commands2) = &files[j];

            for cmd1 in commands1 {
                for cmd2 in commands2 {
                    // Same command type but different package manager
                    if cmd1.command_type == cmd2.command_type
                        && cmd1.package_manager != cmd2.package_manager
                    {
                        conflicts.push(BuildConflict {
                            file1: file1.clone(),
                            file1_line: cmd1.line,
                            file1_manager: cmd1.package_manager,
                            file1_command: cmd1.raw_command.clone(),
                            file2: file2.clone(),
                            file2_line: cmd2.line,
                            file2_manager: cmd2.package_manager,
                            file2_command: cmd2.raw_command.clone(),
                            command_type: cmd1.command_type,
                        });
                    }
                }
            }
        }
    }

    conflicts
}

// ============================================================================
// XP-005: Conflicting Tool Constraints Detection
// ============================================================================

/// Type of tool constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Allow,
    Disallow,
}

/// A tool constraint extracted from content
#[derive(Debug, Clone)]
pub struct ToolConstraint {
    pub line: usize,
    pub column: usize,
    pub tool_name: String,
    pub constraint_type: ConstraintType,
    pub source_context: String,
}

fn tool_allow_pattern() -> &'static Regex {
    TOOL_ALLOW_PATTERN.get_or_init(|| {
        // Match patterns that allow tools
        // allowed-tools:, tools:, allowedTools:, always allow, can use, may use
        Regex::new(r"(?im)(?:allowed[-_]?tools\s*:|tools\s*:\s*\[|\ballways?\s+allow\s+(\w+)\b|\bcan\s+use\s+(\w+)\b|\bmay\s+use\s+(\w+)\b)")
            .unwrap()
    })
}

fn tool_disallow_pattern() -> &'static Regex {
    TOOL_DISALLOW_PATTERN.get_or_init(|| {
        // Match patterns that disallow tools
        // disallowed-tools:, disallowedTools:, never use, don't use, do not use, forbidden, prohibited
        Regex::new(r"(?im)(?:disallowed[-_]?tools\s*:|\bnever\s+use\s+(\w+)\b|\bdon'?t\s+use\s+(\w+)\b|\bdo\s+not\s+use\s+(\w+)\b|\bforbidden\s*:\s*(\w+)\b|\bprohibited\s*:\s*(\w+)\b|\bno\s+(\w+)\s+tool\b)")
            .unwrap()
    })
}

/// Extract tool constraints from content (for XP-005)
///
/// Detects tool allow/disallow patterns in instruction files
pub fn extract_tool_constraints(content: &str) -> Vec<ToolConstraint> {
    let mut results = Vec::new();
    let allow_pattern = tool_allow_pattern();
    let disallow_pattern = tool_disallow_pattern();

    for (line_num, line) in content.lines().enumerate() {
        // Check for allow patterns
        if let Some(mat) = allow_pattern.find(line) {
            let matched = mat.as_str();

            // Extract tool names from the line after the pattern
            let tools = extract_tool_names_from_line(line, mat.end());
            for tool in tools {
                results.push(ToolConstraint {
                    line: line_num + 1,
                    column: mat.start(),
                    tool_name: tool,
                    constraint_type: ConstraintType::Allow,
                    source_context: matched.to_string(),
                });
            }
        }

        // Check for disallow patterns
        if let Some(mat) = disallow_pattern.find(line) {
            let matched = mat.as_str();

            // Check for inline tool name captures
            if let Some(caps) = disallow_pattern.captures(line) {
                for i in 1..=6 {
                    if let Some(tool_cap) = caps.get(i) {
                        results.push(ToolConstraint {
                            line: line_num + 1,
                            column: mat.start(),
                            tool_name: tool_cap.as_str().to_string(),
                            constraint_type: ConstraintType::Disallow,
                            source_context: matched.to_string(),
                        });
                    }
                }
            }

            // Extract tool names from the line after the pattern
            let tools = extract_tool_names_from_line(line, mat.end());
            for tool in tools {
                results.push(ToolConstraint {
                    line: line_num + 1,
                    column: mat.start(),
                    tool_name: tool,
                    constraint_type: ConstraintType::Disallow,
                    source_context: matched.to_string(),
                });
            }
        }
    }

    results
}

/// Extract tool names from a line after a given position
fn extract_tool_names_from_line(line: &str, start_pos: usize) -> Vec<String> {
    let mut tools = Vec::new();
    let remainder = if start_pos < line.len() {
        &line[start_pos..]
    } else {
        return tools;
    };

    // Known tool names (case-insensitive matching)
    let known_tools = [
        "Bash",
        "Read",
        "Write",
        "Edit",
        "Grep",
        "Glob",
        "Task",
        "WebFetch",
        "AskUserQuestion",
        "TodoRead",
        "TodoWrite",
        "mcp",
        "computer",
        "execute",
    ];

    // Match tool names in the remainder
    for tool in &known_tools {
        if remainder.to_lowercase().contains(&tool.to_lowercase()) {
            tools.push(tool.to_string());
        }
    }

    tools
}

/// Conflict between tool constraints across files
#[derive(Debug, Clone)]
pub struct ToolConflict {
    pub tool_name: String,
    pub allow_file: std::path::PathBuf,
    pub allow_line: usize,
    pub allow_context: String,
    pub disallow_file: std::path::PathBuf,
    pub disallow_line: usize,
    pub disallow_context: String,
}

/// Detect conflicting tool constraints across instruction files (for XP-005)
///
/// Returns conflicts when one file allows a tool and another disallows it
pub fn detect_tool_conflicts(
    files: &[(std::path::PathBuf, Vec<ToolConstraint>)],
) -> Vec<ToolConflict> {
    let mut conflicts = Vec::new();

    // Build maps of allowed and disallowed tools per file
    for i in 0..files.len() {
        for j in 0..files.len() {
            if i == j {
                continue;
            }

            let (file1, constraints1) = &files[i];
            let (file2, constraints2) = &files[j];

            for c1 in constraints1 {
                if c1.constraint_type != ConstraintType::Allow {
                    continue;
                }

                for c2 in constraints2 {
                    if c2.constraint_type != ConstraintType::Disallow {
                        continue;
                    }

                    // Same tool, different constraint
                    if c1.tool_name.to_lowercase() == c2.tool_name.to_lowercase() {
                        // Avoid duplicate conflicts (check if already reported with files swapped)
                        let already_reported = conflicts.iter().any(|conflict: &ToolConflict| {
                            conflict.tool_name.to_lowercase() == c1.tool_name.to_lowercase()
                                && ((conflict.allow_file == *file1
                                    && conflict.disallow_file == *file2)
                                    || (conflict.allow_file == *file2
                                        && conflict.disallow_file == *file1))
                        });

                        if !already_reported {
                            conflicts.push(ToolConflict {
                                tool_name: c1.tool_name.clone(),
                                allow_file: file1.clone(),
                                allow_line: c1.line,
                                allow_context: c1.source_context.clone(),
                                disallow_file: file2.clone(),
                                disallow_line: c2.line,
                                disallow_context: c2.source_context.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    conflicts
}

// ============================================================================
// XP-006: Multiple Layers Without Documented Precedence
// ============================================================================

/// Type of instruction layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// Root-level CLAUDE.md
    ClaudeMd,
    /// Root-level AGENTS.md
    AgentsMd,
    /// Cursor rules (.cursor/rules/*.mdc)
    CursorRules,
    /// Copilot instructions (.github/copilot-instructions.md)
    CopilotInstructions,
    /// Cline rules (.clinerules)
    ClineRules,
    /// OpenCode rules (.opencode/)
    OpenCodeRules,
    /// Other instruction file
    Other,
}

impl LayerType {
    /// Get the display name for this layer type
    pub fn as_str(&self) -> &'static str {
        match self {
            LayerType::ClaudeMd => "CLAUDE.md",
            LayerType::AgentsMd => "AGENTS.md",
            LayerType::CursorRules => "Cursor Rules",
            LayerType::CopilotInstructions => "Copilot Instructions",
            LayerType::ClineRules => "Cline Rules",
            LayerType::OpenCodeRules => "OpenCode Rules",
            LayerType::Other => "Other",
        }
    }
}

/// An instruction layer in the project
#[derive(Debug, Clone)]
pub struct InstructionLayer {
    pub path: std::path::PathBuf,
    pub layer_type: LayerType,
    pub has_precedence_doc: bool,
}

fn layer_precedence_pattern() -> &'static Regex {
    LAYER_PRECEDENCE_PATTERN.get_or_init(|| {
        // Match patterns that document precedence/priority
        Regex::new(r"(?im)(?:precedence|priority|override|hierarchy|takes?\s+precedence|supersede|primary\s+source|authoritative)")
            .unwrap()
    })
}

/// Categorize a file path as an instruction layer (for XP-006)
pub fn categorize_layer(path: &Path, content: &str) -> InstructionLayer {
    let path_str = path.to_string_lossy().to_lowercase();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    let layer_type = if file_name == "claude.md" {
        LayerType::ClaudeMd
    } else if file_name == "agents.md" {
        LayerType::AgentsMd
    } else if path_str.contains(".cursor") && path_str.contains("rules") {
        LayerType::CursorRules
    } else if path_str.contains(".github") && path_str.contains("copilot") {
        LayerType::CopilotInstructions
    } else if file_name == ".clinerules" || path_str.contains(".clinerules") {
        LayerType::ClineRules
    } else if path_str.contains(".opencode") {
        LayerType::OpenCodeRules
    } else {
        LayerType::Other
    };

    let has_precedence_doc = layer_precedence_pattern().is_match(content);

    InstructionLayer {
        path: path.to_path_buf(),
        layer_type,
        has_precedence_doc,
    }
}

/// Issue when multiple instruction layers exist without documented precedence
#[derive(Debug, Clone)]
pub struct LayerPrecedenceIssue {
    pub layers: Vec<InstructionLayer>,
    pub description: String,
}

/// Detect precedence issues when multiple instruction layers exist (for XP-006)
///
/// Returns an issue if multiple layers exist and none document precedence
pub fn detect_precedence_issues(layers: &[InstructionLayer]) -> Option<LayerPrecedenceIssue> {
    // Filter to only include meaningful layers (not Other)
    let meaningful_layers: Vec<_> = layers
        .iter()
        .filter(|l| l.layer_type != LayerType::Other)
        .collect();

    // If there's only one or zero layers, no issue
    if meaningful_layers.len() <= 1 {
        return None;
    }

    // Check if any layer documents precedence
    let has_precedence = meaningful_layers.iter().any(|l| l.has_precedence_doc);

    if !has_precedence {
        let layer_names: Vec<_> = meaningful_layers
            .iter()
            .map(|l| format!("{} ({})", l.layer_type.as_str(), l.path.display()))
            .collect();

        Some(LayerPrecedenceIssue {
            layers: meaningful_layers.into_iter().cloned().collect(),
            description: format!(
                "Multiple instruction layers detected without documented precedence: {}",
                layer_names.join(", ")
            ),
        })
    } else {
        None
    }
}

/// Check if a file is an instruction file (for cross-layer detection)
pub fn is_instruction_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    file_name == "claude.md"
        || file_name == "agents.md"
        || file_name == ".clinerules"
        || (path_str.contains(".cursor")
            && (path_str.ends_with(".mdc") || path_str.contains("rules")))
        || (path_str.contains(".github") && path_str.contains("copilot"))
        || path_str.contains(".opencode")
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

    #[test]
    fn test_detect_custom_agent_name() {
        // Custom agent names should also be flagged (not just Explore/Plan/general-purpose)
        let content = r#"---
name: test
agent: security-reviewer
---
Body"#;
        let results = find_claude_specific_features(content);
        assert!(results.iter().any(|r| r.feature == "agent"));
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

    // ===== XP-004: Build Command Conflicts =====

    #[test]
    fn test_extract_npm_commands() {
        let content = r#"# Build
Run `npm install` to install dependencies.
Then `npm run build` to build the project.
"#;
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.package_manager == PackageManager::Npm));
    }

    #[test]
    fn test_extract_pnpm_commands() {
        let content = r#"# Install
Use pnpm install for dependencies.
"#;
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package_manager, PackageManager::Pnpm);
        assert_eq!(results[0].command_type, CommandType::Install);
    }

    #[test]
    fn test_extract_yarn_commands() {
        let content = "yarn add express\nyarn test";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.package_manager == PackageManager::Yarn));
    }

    #[test]
    fn test_extract_bun_commands() {
        let content = "bun install\nbun run build";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.package_manager == PackageManager::Bun));
    }

    #[test]
    fn test_detect_build_conflicts() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        let commands1 = vec![BuildCommand {
            line: 1,
            column: 0,
            package_manager: PackageManager::Npm,
            command_type: CommandType::Install,
            raw_command: "npm install".to_string(),
        }];

        let commands2 = vec![BuildCommand {
            line: 1,
            column: 0,
            package_manager: PackageManager::Pnpm,
            command_type: CommandType::Install,
            raw_command: "pnpm install".to_string(),
        }];

        let files = vec![(file1, commands1), (file2, commands2)];
        let conflicts = detect_build_conflicts(&files);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].file1_manager, PackageManager::Npm);
        assert_eq!(conflicts[0].file2_manager, PackageManager::Pnpm);
    }

    #[test]
    fn test_no_conflict_same_package_manager() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        let commands1 = vec![BuildCommand {
            line: 1,
            column: 0,
            package_manager: PackageManager::Npm,
            command_type: CommandType::Install,
            raw_command: "npm install".to_string(),
        }];

        let commands2 = vec![BuildCommand {
            line: 1,
            column: 0,
            package_manager: PackageManager::Npm,
            command_type: CommandType::Build,
            raw_command: "npm run build".to_string(),
        }];

        let files = vec![(file1, commands1), (file2, commands2)];
        let conflicts = detect_build_conflicts(&files);

        // No conflict because same package manager, different command types
        assert!(conflicts.is_empty());
    }

    // ===== XP-005: Tool Constraint Conflicts =====

    #[test]
    fn test_extract_tool_allow_constraint() {
        let content = "allowed-tools: Read Write Bash";
        let results = extract_tool_constraints(content);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.tool_name == "Read"));
        assert!(results
            .iter()
            .all(|r| r.constraint_type == ConstraintType::Allow));
    }

    #[test]
    fn test_extract_tool_disallow_constraint() {
        let content = "Never use Bash for this task.";
        let results = extract_tool_constraints(content);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.tool_name == "Bash"));
        assert!(results
            .iter()
            .any(|r| r.constraint_type == ConstraintType::Disallow));
    }

    #[test]
    fn test_detect_tool_conflicts() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        let constraints1 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Allow,
            source_context: "allowed-tools:".to_string(),
        }];

        let constraints2 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Disallow,
            source_context: "never use".to_string(),
        }];

        let files = vec![(file1, constraints1), (file2, constraints2)];
        let conflicts = detect_tool_conflicts(&files);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].tool_name, "Bash");
    }

    #[test]
    fn test_no_tool_conflict_same_constraint_type() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        let constraints1 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Allow,
            source_context: "allowed-tools:".to_string(),
        }];

        let constraints2 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Allow,
            source_context: "allowed-tools:".to_string(),
        }];

        let files = vec![(file1, constraints1), (file2, constraints2)];
        let conflicts = detect_tool_conflicts(&files);

        assert!(conflicts.is_empty());
    }

    // ===== XP-006: Layer Precedence =====

    #[test]
    fn test_categorize_claude_md() {
        use std::path::PathBuf;
        let layer = categorize_layer(&PathBuf::from("project/CLAUDE.md"), "# Project");
        assert_eq!(layer.layer_type, LayerType::ClaudeMd);
    }

    #[test]
    fn test_categorize_agents_md() {
        use std::path::PathBuf;
        let layer = categorize_layer(&PathBuf::from("project/AGENTS.md"), "# Project");
        assert_eq!(layer.layer_type, LayerType::AgentsMd);
    }

    #[test]
    fn test_categorize_cursor_rules() {
        use std::path::PathBuf;
        let layer = categorize_layer(&PathBuf::from("project/.cursor/rules/test.mdc"), "# Rules");
        assert_eq!(layer.layer_type, LayerType::CursorRules);
    }

    #[test]
    fn test_precedence_detected() {
        use std::path::PathBuf;
        let layer = categorize_layer(
            &PathBuf::from("CLAUDE.md"),
            "CLAUDE.md takes precedence over AGENTS.md",
        );
        assert!(layer.has_precedence_doc);
    }

    #[test]
    fn test_precedence_not_detected() {
        use std::path::PathBuf;
        let layer = categorize_layer(&PathBuf::from("CLAUDE.md"), "# Simple rules");
        assert!(!layer.has_precedence_doc);
    }

    #[test]
    fn test_detect_precedence_issues_multiple_layers() {
        use std::path::PathBuf;

        let layers = vec![
            InstructionLayer {
                path: PathBuf::from("CLAUDE.md"),
                layer_type: LayerType::ClaudeMd,
                has_precedence_doc: false,
            },
            InstructionLayer {
                path: PathBuf::from("AGENTS.md"),
                layer_type: LayerType::AgentsMd,
                has_precedence_doc: false,
            },
        ];

        let issue = detect_precedence_issues(&layers);
        assert!(issue.is_some());
        assert!(issue
            .unwrap()
            .description
            .contains("without documented precedence"));
    }

    #[test]
    fn test_no_precedence_issue_with_docs() {
        use std::path::PathBuf;

        let layers = vec![
            InstructionLayer {
                path: PathBuf::from("CLAUDE.md"),
                layer_type: LayerType::ClaudeMd,
                has_precedence_doc: true, // Has precedence documentation
            },
            InstructionLayer {
                path: PathBuf::from("AGENTS.md"),
                layer_type: LayerType::AgentsMd,
                has_precedence_doc: false,
            },
        ];

        let issue = detect_precedence_issues(&layers);
        assert!(issue.is_none());
    }

    #[test]
    fn test_no_precedence_issue_single_layer() {
        use std::path::PathBuf;

        let layers = vec![InstructionLayer {
            path: PathBuf::from("CLAUDE.md"),
            layer_type: LayerType::ClaudeMd,
            has_precedence_doc: false,
        }];

        let issue = detect_precedence_issues(&layers);
        assert!(issue.is_none());
    }

    #[test]
    fn test_is_instruction_file() {
        use std::path::PathBuf;

        assert!(is_instruction_file(&PathBuf::from("CLAUDE.md")));
        assert!(is_instruction_file(&PathBuf::from("AGENTS.md")));
        assert!(is_instruction_file(&PathBuf::from(
            ".cursor/rules/test.mdc"
        )));
        assert!(is_instruction_file(&PathBuf::from(
            ".github/copilot-instructions.md"
        )));
        assert!(is_instruction_file(&PathBuf::from(".clinerules")));

        assert!(!is_instruction_file(&PathBuf::from("README.md")));
        assert!(!is_instruction_file(&PathBuf::from("src/main.rs")));
    }
}
