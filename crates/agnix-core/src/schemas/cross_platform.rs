//! Cross-platform validation schema helpers
//!
//! Provides detection functions for:
//! - XP-001: Claude-specific features in AGENTS.md
//! - XP-002: AGENTS.md markdown structure validation
//! - XP-003: Hard-coded platform paths in configs
//!
//! ## Security
//!
//! This module includes size limits to prevent ReDoS (Regular Expression Denial
//! of Service) attacks. Functions that use regex will return early for oversized
//! input.

use regex::Regex;
use std::path::Path;

use crate::parsers::markdown::MAX_REGEX_INPUT_SIZE;
use crate::regex_util::static_regex;

// XP-001: Claude-specific feature patterns
static_regex!(fn claude_hooks_pattern, r"(?im)^\s*-?\s*(?:type|event):\s*(?:PreToolExecution|PostToolExecution|Notification|Stop|SubagentStop)\b");
static_regex!(fn context_fork_pattern, r"(?im)^\s*context:\s*fork\b");
static_regex!(fn agent_field_pattern, r"(?im)^\s*agent:\s*\S+");
static_regex!(fn allowed_tools_pattern, r"(?im)^\s*allowed-tools:\s*.+");
static_regex!(fn at_import_pattern, r"(?m)(?:^|\s)@[\w./*-]+\.\w+");
static_regex!(fn claude_section_guard_pattern, r"(?im)^(?:#+\s*|<!--\s*)claude(?:\s+code)?(?:\s+specific|\s+only)?(?:\s*-->)?");

// XP-002/003: Markdown structure and path patterns
static_regex!(fn markdown_header_pattern, r"^#+\s+.+");

// XP-004: Build command patterns
static_regex!(fn build_command_pattern, r"(?m)(?:^|\s|`)((?:npm|pnpm|yarn|bun)\s+(?:install|i|add|build|test|run|exec|ci)\b[^\n`]*)");

// XP-005: Tool constraint patterns
static_regex!(fn tool_allow_pattern, r"(?im)(?:allowed[-_]?tools\s*:|tools\s*:\s*\[|\ballways?\s+allow\s+(\w+)\b|\bcan\s+use\s+(\w+)\b|\bmay\s+use\s+(\w+)\b)");
static_regex!(fn tool_disallow_pattern, r"(?im)(?:disallowed[-_]?tools\s*:|\bnever\s+use\s+(\w+)\b|\bdon'?t\s+use\s+(\w+)\b|\bdo\s+not\s+use\s+(\w+)\b|\bforbidden\s*:\s*(\w+)\b|\bprohibited\s*:\s*(\w+)\b|\bno\s+(\w+)\s+tool\b)");

// XP-006: Layer type patterns
static_regex!(fn layer_precedence_pattern, r"(?im)(?:precedence|priority|override|hierarchy|takes?\s+precedence|supersede|primary\s+source|authoritative)");

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

/// Find Claude-specific features in content (for XP-001)
///
/// Detects features that only work in Claude Code but not in other platforms
/// that read AGENTS.md (Codex CLI, OpenCode, GitHub Copilot, Cursor, Cline).
///
/// Features inside a Claude-specific section (marked by a header like
/// `## Claude Code Specific` or `<!-- Claude Specific -->`) are not reported,
/// allowing users to document Claude-specific features without triggering errors.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_claude_specific_features(content: &str) -> Vec<ClaudeSpecificFeature> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let guard_pattern = claude_section_guard_pattern();

    let mut in_claude_section = false;
    let mut claude_section_level = 0; // Track the level of the Claude guard header

    for (line_num, line) in content.lines().enumerate() {
        let is_claude_guard = guard_pattern.is_match(line);
        if is_claude_guard {
            in_claude_section = true;
            // Extract header level for Claude section
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                claude_section_level = trimmed.chars().take_while(|c| *c == '#').count();
            } else if trimmed.starts_with("<!--") {
                claude_section_level = 2; // Default to level 2 for HTML comments
            }
            continue;
        }

        // Only reset guard if we encounter a header that is:
        // 1. At the same or higher level (lower number) as the Claude guard
        // 2. Not a Claude-specific guard itself
        if in_claude_section
            && (line.trim_start().starts_with('#') || line.trim_start().starts_with("<!--"))
        {
            let trimmed = line.trim_start();
            let current_level = if trimmed.starts_with('#') {
                trimmed.chars().take_while(|c| *c == '#').count()
            } else if trimmed.starts_with("<!--") {
                2 // Default to level 2 for HTML comments
            } else {
                usize::MAX
            };

            // Reset guard only if new header is at same or higher level (lower number)
            if current_level <= claude_section_level {
                in_claude_section = false;
            }
        }

        if in_claude_section {
            continue;
        }

        if let Some(mat) = claude_hooks_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start() + 1,
                feature: "hooks".to_string(),
                description: "Claude Code hooks are not supported by other AGENTS.md readers"
                    .to_string(),
            });
        }

        // Check for context: fork
        if let Some(mat) = context_fork_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start() + 1,
                feature: "context:fork".to_string(),
                description: "Context forking is Claude Code specific".to_string(),
            });
        }

        // Check for agent: field
        if let Some(mat) = agent_field_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start() + 1,
                feature: "agent".to_string(),
                description: "Agent field is Claude Code specific".to_string(),
            });
        }

        // Check for allowed-tools: field
        if let Some(mat) = allowed_tools_pattern().find(line) {
            results.push(ClaudeSpecificFeature {
                line: line_num + 1,
                column: mat.start() + 1,
                feature: "allowed-tools".to_string(),
                description: "Tool restrictions are Claude Code specific".to_string(),
            });
        }

        // Check for @file import syntax (Claude Code specific)
        if let Some(mat) = at_import_pattern().find(line) {
            // Avoid matching email addresses (user@domain.com)
            let matched = mat.as_str().trim_start();
            if matched.starts_with('@') && matched.matches('@').count() == 1 {
                results.push(ClaudeSpecificFeature {
                    line: line_num + 1,
                    column: mat.start() + 1,
                    feature: "@import".to_string(),
                    description: "The @file import syntax is Claude Code specific".to_string(),
                });
            }
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

// Expanded XP-003 pattern: tool-specific dirs + OS-specific absolute paths
static_regex!(fn hard_coded_path_pattern, r"(?i)(?:\.claude/|\.opencode/|\.cursor/|\.cline/|\.github/copilot/|~/Library/|~/\.[a-z][\w-]*/|/Users/[a-zA-Z][\w.-]*/|/home/[a-zA-Z][\w.-]*/|[A-Z]:\\Users\\[a-zA-Z][\w.-]*\\)");
/// Find hard-coded platform-specific paths (for XP-003)
///
/// Detects paths like `.claude/`, `.opencode/`, `.cursor/` that may cause
/// portability issues when the same config is used across different platforms.
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn find_hard_coded_paths(content: &str) -> Vec<HardCodedPath> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

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
            } else if path.contains("/library/") || path.starts_with("~/library/") {
                "macOS"
            } else if path.starts_with("/users/") || path.starts_with("/home/") {
                "OS-specific absolute path"
            } else if path.contains(":\\users\\") {
                "Windows absolute path"
            } else if path.starts_with("~/.") {
                "User-specific hidden directory"
            } else {
                "OS-specific"
            };

            results.push(HardCodedPath {
                line: line_num + 1,
                column: mat.start() + 1,
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

/// Extract build commands from content (for XP-004)
///
/// Detects npm, pnpm, yarn, and bun commands in instruction files
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn extract_build_commands(content: &str) -> Vec<BuildCommand> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

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
            // Note: " i " requires space after, but "npm i" at end of line needs special handling
            let command_type = if raw.contains(" install")
                || raw.contains(" i ")
                || raw.ends_with(" i")
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
/// Returns conflicts when different package managers are used for the same command type.
/// Uses O(n*m) algorithm by grouping commands by type first, then checking for conflicts.
pub fn detect_build_conflicts(
    files: &[(std::path::PathBuf, Vec<BuildCommand>)],
) -> Vec<BuildConflict> {
    use std::collections::HashMap;

    // Group commands by CommandType: HashMap<CommandType, Vec<(PathBuf, BuildCommand)>>
    let mut by_type: HashMap<CommandType, Vec<(std::path::PathBuf, &BuildCommand)>> =
        HashMap::new();

    for (path, commands) in files {
        for cmd in commands {
            by_type
                .entry(cmd.command_type)
                .or_default()
                .push((path.clone(), cmd));
        }
    }

    let mut conflicts = Vec::new();

    // For each command type, check if different package managers are used
    for (cmd_type, entries) in by_type {
        // Group by package manager within this command type
        let mut by_manager: HashMap<PackageManager, Vec<(std::path::PathBuf, &BuildCommand)>> =
            HashMap::new();

        for (path, cmd) in entries {
            by_manager
                .entry(cmd.package_manager)
                .or_default()
                .push((path, cmd));
        }

        // If there are multiple package managers for the same command type, report conflicts
        if by_manager.len() > 1 {
            let managers: Vec<_> = by_manager.keys().collect();

            // Report conflict between each pair of different package managers
            for i in 0..managers.len() {
                for j in (i + 1)..managers.len() {
                    let manager1 = managers[i];
                    let manager2 = managers[j];

                    let entries1 = &by_manager[manager1];
                    let entries2 = &by_manager[manager2];

                    // Take first entry from each group to report
                    if let (Some((path1, cmd1)), Some((path2, cmd2))) =
                        (entries1.first(), entries2.first())
                    {
                        // Skip if both commands are from the same file (false positive)
                        if path1 != path2 {
                            conflicts.push(BuildConflict {
                                file1: path1.clone(),
                                file1_line: cmd1.line,
                                file1_manager: *manager1,
                                file1_command: cmd1.raw_command.clone(),
                                file2: path2.clone(),
                                file2_line: cmd2.line,
                                file2_manager: *manager2,
                                file2_command: cmd2.raw_command.clone(),
                                command_type: cmd_type,
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

/// Extract tool constraints from content (for XP-005)
///
/// Detects tool allow/disallow patterns in instruction files
///
/// # Security
///
/// Returns early for content exceeding `MAX_REGEX_INPUT_SIZE` to prevent ReDoS.
pub fn extract_tool_constraints(content: &str) -> Vec<ToolConstraint> {
    // Security: Skip regex processing for oversized input to prevent ReDoS
    if content.len() > MAX_REGEX_INPUT_SIZE {
        return Vec::new();
    }

    let mut results = Vec::new();
    let allow_pattern = tool_allow_pattern();
    let disallow_pattern = tool_disallow_pattern();

    for (line_num, line) in content.lines().enumerate() {
        // Check for allow patterns
        if let Some(mat) = allow_pattern.find(line) {
            let matched = mat.as_str();

            // Check for inline tool name captures first
            if let Some(caps) = allow_pattern.captures(line) {
                for i in 1..=6 {
                    if let Some(tool_cap) = caps.get(i) {
                        // Normalize to canonical tool name if it matches a known tool
                        if let Some(canonical) = normalize_tool_name(tool_cap.as_str()) {
                            results.push(ToolConstraint {
                                line: line_num + 1,
                                column: mat.start() + 1,
                                tool_name: canonical,
                                constraint_type: ConstraintType::Allow,
                                source_context: matched.to_string(),
                            });
                        }
                    }
                }
            }

            // Extract tool names from the line after the pattern
            let tools = extract_tool_names_from_line(line, mat.end());
            for tool in tools {
                results.push(ToolConstraint {
                    line: line_num + 1,
                    column: mat.start() + 1,
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
                        // Normalize to canonical tool name if it matches a known tool
                        if let Some(canonical) = normalize_tool_name(tool_cap.as_str()) {
                            results.push(ToolConstraint {
                                line: line_num + 1,
                                column: mat.start() + 1,
                                tool_name: canonical,
                                constraint_type: ConstraintType::Disallow,
                                source_context: matched.to_string(),
                            });
                        }
                    }
                }
            }

            // Extract tool names from the line after the pattern
            let tools = extract_tool_names_from_line(line, mat.end());
            for tool in tools {
                results.push(ToolConstraint {
                    line: line_num + 1,
                    column: mat.start() + 1,
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
///
/// Uses word boundary matching to avoid false positives (e.g., 'Bash' in 'Bashful').
fn extract_tool_names_from_line(line: &str, start_pos: usize) -> Vec<String> {
    let mut tools = Vec::new();
    let remainder = if start_pos < line.len() {
        &line[start_pos..]
    } else {
        return tools;
    };

    let remainder_lower = remainder.to_lowercase();
    let remainder_bytes = remainder_lower.as_bytes();

    // Match tool names with word boundary checking
    for tool in KNOWN_TOOLS {
        let tool_lower = tool.to_lowercase();
        if let Some(pos) = remainder_lower.find(&tool_lower) {
            // Check word boundaries
            let before_ok = pos == 0 || !is_word_char(remainder_bytes[pos - 1]);
            let after_pos = pos + tool_lower.len();
            let after_ok =
                after_pos >= remainder_bytes.len() || !is_word_char(remainder_bytes[after_pos]);

            if before_ok && after_ok {
                tools.push(tool.to_string());
            }
        }
    }

    tools
}

/// Check if a byte is a word character (alphanumeric or underscore)
#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Known tool names for normalization
const KNOWN_TOOLS: &[&str] = &[
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

/// Normalize a tool name to its canonical form if it matches a known tool
fn normalize_tool_name(name: &str) -> Option<String> {
    let name_lower = name.to_lowercase();
    for tool in KNOWN_TOOLS {
        if tool.to_lowercase() == name_lower {
            return Some(tool.to_string());
        }
    }
    None
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
/// Returns conflicts when one file allows a tool and another disallows it.
/// Uses O(n*m) algorithm by grouping constraints by tool name first.
#[allow(clippy::type_complexity)]
pub fn detect_tool_conflicts(
    files: &[(std::path::PathBuf, Vec<ToolConstraint>)],
) -> Vec<ToolConflict> {
    use std::collections::{HashMap, HashSet};

    // Type alias for the grouped constraints
    type ConstraintGroup<'a> = (
        Vec<(std::path::PathBuf, &'a ToolConstraint)>,
        Vec<(std::path::PathBuf, &'a ToolConstraint)>,
    );

    // Group constraints by tool name (lowercase for case-insensitive matching)
    // Key: tool_name (lowercase)
    // Value: (allowed_from: Vec<(path, constraint)>, disallowed_from: Vec<(path, constraint)>)
    let mut by_tool: HashMap<String, ConstraintGroup<'_>> = HashMap::new();

    for (path, constraints) in files {
        for constraint in constraints {
            let tool_key = constraint.tool_name.to_lowercase();
            let entry = by_tool
                .entry(tool_key)
                .or_insert_with(|| (Vec::new(), Vec::new()));

            match constraint.constraint_type {
                ConstraintType::Allow => entry.0.push((path.clone(), constraint)),
                ConstraintType::Disallow => entry.1.push((path.clone(), constraint)),
            }
        }
    }

    let mut conflicts = Vec::new();
    let mut reported: HashSet<(String, std::path::PathBuf, std::path::PathBuf)> = HashSet::new();

    // For each tool, check if there are both allow and disallow constraints
    for (tool_key, (allowed, disallowed)) in by_tool {
        if allowed.is_empty() || disallowed.is_empty() {
            continue;
        }

        // Report conflicts between allow and disallow constraints
        for (allow_path, allow_constraint) in &allowed {
            for (disallow_path, disallow_constraint) in &disallowed {
                // Skip same file conflicts
                if allow_path == disallow_path {
                    continue;
                }

                // Create a normalized key for deduplication (smaller path first)
                let key = if allow_path < disallow_path {
                    (tool_key.clone(), allow_path.clone(), disallow_path.clone())
                } else {
                    (tool_key.clone(), disallow_path.clone(), allow_path.clone())
                };

                if reported.insert(key) {
                    conflicts.push(ToolConflict {
                        tool_name: allow_constraint.tool_name.clone(),
                        allow_file: allow_path.clone(),
                        allow_line: allow_constraint.line,
                        allow_context: allow_constraint.source_context.clone(),
                        disallow_file: disallow_path.clone(),
                        disallow_line: disallow_constraint.line,
                        disallow_context: disallow_constraint.source_context.clone(),
                    });
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

/// Categorize a file path as an instruction layer (for XP-006)
///
/// # Security
///
/// For content exceeding `MAX_REGEX_INPUT_SIZE`, `has_precedence_doc` will be
/// set to false to avoid ReDoS. This is a safe default as it may trigger
/// additional warnings but won't miss security issues.
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

    // Security: Skip regex for oversized input to prevent ReDoS
    // Default to false (safer - may trigger warnings but won't miss issues)
    let has_precedence_doc =
        content.len() <= MAX_REGEX_INPUT_SIZE && layer_precedence_pattern().is_match(content);

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

    // Skip backup/temp files
    if path_str.ends_with(".bak")
        || path_str.ends_with(".old")
        || path_str.ends_with(".tmp")
        || path_str.ends_with(".swp")
        || path_str.ends_with('~')
    {
        return false;
    }

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

    #[test]
    fn test_regex_patterns_compile() {
        let _ = claude_hooks_pattern();
        let _ = context_fork_pattern();
        let _ = agent_field_pattern();
        let _ = allowed_tools_pattern();
        let _ = claude_section_guard_pattern();
        let _ = markdown_header_pattern();
        let _ = hard_coded_path_pattern();
        let _ = build_command_pattern();
        let _ = tool_allow_pattern();
        let _ = tool_disallow_pattern();
        let _ = layer_precedence_pattern();
    }

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
    fn test_detect_at_import_syntax() {
        let content = "Include rules from @path/to/rules.md in your config.";
        let results = find_claude_specific_features(content);
        assert!(
            results.iter().any(|r| r.feature == "@import"),
            "Should detect @path/to/rules.md as @import syntax"
        );
    }

    #[test]
    fn test_detect_at_import_with_wildcard() {
        let content = "Load all rules with @.config/rules/*.md";
        let results = find_claude_specific_features(content);
        assert!(
            results.iter().any(|r| r.feature == "@import"),
            "Should detect @.config/rules/*.md (wildcard @import)"
        );
    }

    #[test]
    fn test_no_false_positive_email_in_at_import() {
        let content = "Contact user@example.com for questions.";
        let results = find_claude_specific_features(content);
        assert!(
            !results.iter().any(|r| r.feature == "@import"),
            "Email addresses should not be flagged as @imports"
        );
    }

    #[test]
    fn test_no_false_positive_email_standalone() {
        let content = "Email admin@domain.org for access.";
        let results = find_claude_specific_features(content);
        assert!(
            !results.iter().any(|r| r.feature == "@import"),
            "Standalone email should not trigger @import detection"
        );
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

    // ===== XP-001: Claude Section Guard Tests =====

    #[test]
    fn test_guarded_hooks_in_claude_section() {
        // Hooks inside a Claude-specific section should NOT be reported
        let content = r#"# Project Guidelines

## Claude Code Specific
- type: PreToolExecution
  command: echo "test"
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Hooks in Claude-specific section should not trigger XP-001"
        );
    }

    #[test]
    fn test_guarded_context_fork() {
        // context:fork inside a Claude section should NOT be reported
        let content = r#"# Config

## Claude Only
context: fork
agent: Explore
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Features in Claude-only section should not trigger XP-001"
        );
    }

    #[test]
    fn test_guarded_agent_field() {
        // agent field inside a Claude section should NOT be reported
        let content = r#"# Settings

## Claude Specific
agent: security-reviewer
allowed-tools: Read Write
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Agent field in Claude-specific section should not trigger XP-001"
        );
    }

    #[test]
    fn test_guard_section_ends_at_new_header() {
        // Guard protection should end when a new header is encountered
        let content = r#"# Main

## Claude Code Specific
- type: Stop
  command: cleanup

## Other Settings
agent: something
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(results.len(), 1, "Expected exactly 1 result");
        assert!(
            !results.iter().any(|r| r.feature == "hooks"),
            "Hooks in Claude section should be guarded"
        );
        assert!(
            results.iter().any(|r| r.feature == "agent"),
            "Agent field outside Claude section should be reported"
        );
    }

    #[test]
    fn test_multiple_claude_sections() {
        // Multiple Claude sections should all be guarded
        let content = r#"# Config

## Claude Code Specific
- type: PreToolExecution
  command: test1

## General Settings
Some general content.

## Claude Only
context: fork
agent: Plan
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Features in any Claude section should be guarded"
        );
    }

    #[test]
    fn test_html_comment_guard() {
        // HTML comment style guard should also work
        let content = r#"# Config

<!-- Claude Code Specific -->
- type: Notification
  command: notify-send
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "HTML comment guard should protect Claude features"
        );
    }

    #[test]
    fn test_case_insensitive_guard() {
        // Guard detection should be case-insensitive
        let content = r#"# Config

## CLAUDE CODE SPECIFIC
- type: SubagentStop
  command: cleanup

## claude specific
allowed-tools: Bash
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Case-insensitive guard should protect Claude features"
        );
    }

    #[test]
    fn test_unguarded_features_still_detected() {
        // Features NOT in a Claude section should still be detected
        let content = r#"# Project Config

## Hooks Setup
- type: PreToolExecution
  command: echo "test"

agent: reviewer
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(results.len(), 2, "Unguarded features should be detected");
        assert!(results.iter().any(|r| r.feature == "hooks"));
        assert!(results.iter().any(|r| r.feature == "agent"));
    }

    #[test]
    fn test_html_comment_header_resets_guard() {
        // A non-Claude HTML comment header should reset the guard protection
        // This tests the fix for handling HTML comment headers like <!-- General Settings -->
        let content = r#"# Config

<!-- Claude Specific -->
- type: PreToolExecution
  command: test

<!-- General Settings -->
agent: reviewer
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(
            results.len(),
            1,
            "Agent field after non-Claude HTML header should be detected"
        );
        assert!(
            results[0].feature == "agent",
            "Should detect agent field outside Claude section"
        );
    }

    #[test]
    fn test_whitespace_before_markdown_header() {
        // Headers with leading whitespace should also reset the guard
        let content = r#"# Config

## Claude Specific
- type: PreToolExecution
  command: test

   ## Other Section
agent: reviewer
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(
            results.len(),
            1,
            "Agent field after indented header should be detected"
        );
        assert!(
            results[0].feature == "agent",
            "Indented header should reset guard protection"
        );
    }

    #[test]
    fn test_subheaders_within_claude_section() {
        // Subheaders (###) within a Claude-specific section (##) should not reset the guard
        // This tests Codex feedback about keeping guard active across subheaders
        let content = r#"# Config

## Claude Specific
### Hooks Setup
- type: PreToolExecution
  command: test

### Context Configuration
context: fork
agent: reviewer
"#;
        let results = find_claude_specific_features(content);
        assert!(
            results.is_empty(),
            "Features under subheaders within Claude section should still be protected"
        );
    }

    #[test]
    fn test_reset_on_same_level_header() {
        // A new top-level header (##) should reset the guard from a ## Claude section
        let content = r#"## Claude Specific
- type: PreToolExecution
  command: test

## Other Settings
agent: reviewer
"#;
        let results = find_claude_specific_features(content);
        assert_eq!(
            results.len(),
            1,
            "Agent field after same-level header should be detected"
        );
        assert!(results[0].feature == "agent");
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
        assert!(
            results
                .iter()
                .all(|r| r.package_manager == PackageManager::Npm)
        );
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
        assert!(
            results
                .iter()
                .all(|r| r.package_manager == PackageManager::Yarn)
        );
    }

    #[test]
    fn test_extract_bun_commands() {
        let content = "bun install\nbun run build";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|r| r.package_manager == PackageManager::Bun)
        );
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
        // Order may vary with HashMap, so check both managers are present
        let managers: std::collections::HashSet<_> =
            [conflicts[0].file1_manager, conflicts[0].file2_manager]
                .into_iter()
                .collect();
        assert!(managers.contains(&PackageManager::Npm));
        assert!(managers.contains(&PackageManager::Pnpm));
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
        assert!(
            results
                .iter()
                .all(|r| r.constraint_type == ConstraintType::Allow)
        );
    }

    #[test]
    fn test_extract_tool_disallow_constraint() {
        let content = "Never use Bash for this task.";
        let results = extract_tool_constraints(content);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.tool_name == "Bash"));
        assert!(
            results
                .iter()
                .any(|r| r.constraint_type == ConstraintType::Disallow)
        );
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
        assert!(
            issue
                .unwrap()
                .description
                .contains("without documented precedence")
        );
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

    // ===== Tool Extraction Word Boundary Tests (review findings) =====

    #[test]
    fn test_tool_extraction_case_insensitive() {
        // "Never use BASH" should detect 'Bash' tool (case-insensitive)
        let content = "Never use BASH for this task.";
        let results = extract_tool_constraints(content);
        assert!(
            results.iter().any(|r| r.tool_name == "Bash"),
            "Should detect 'Bash' from 'BASH' (case-insensitive)"
        );
    }

    #[test]
    fn test_tool_extraction_word_boundaries() {
        // "never use subash" should NOT detect Bash (word boundary check)
        let content = "Never use subash command.";
        let results = extract_tool_constraints(content);
        assert!(
            !results.iter().any(|r| r.tool_name == "Bash"),
            "Should NOT detect 'Bash' from 'subash' (word boundary)"
        );
    }

    #[test]
    fn test_tool_extraction_no_false_positive_bashful() {
        // "Bashful developer" should NOT detect Bash
        let content = "allowed-tools: Bashful developer Read";
        let results = extract_tool_constraints(content);
        assert!(
            !results.iter().any(|r| r.tool_name == "Bash"),
            "Should NOT detect 'Bash' from 'Bashful'"
        );
        // But should detect Read
        assert!(
            results.iter().any(|r| r.tool_name == "Read"),
            "Should detect 'Read'"
        );
    }

    #[test]
    fn test_tool_extraction_no_false_positive_reader() {
        // "Reader mode" should NOT detect Read
        let content = "allowed-tools: Reader mode";
        let results = extract_tool_constraints(content);
        assert!(
            !results.iter().any(|r| r.tool_name == "Read"),
            "Should NOT detect 'Read' from 'Reader'"
        );
    }

    #[test]
    fn test_tool_extraction_valid_word_boundary() {
        // "Read, Write, Bash" should detect all three
        let content = "allowed-tools: Read, Write, Bash";
        let results = extract_tool_constraints(content);
        assert!(results.iter().any(|r| r.tool_name == "Read"));
        assert!(results.iter().any(|r| r.tool_name == "Write"));
        assert!(results.iter().any(|r| r.tool_name == "Bash"));
    }

    // ===== Three-file conflict tests (review findings) =====

    #[test]
    fn test_detect_build_conflicts_three_files() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");
        let file3 = PathBuf::from(".cursor/rules/dev.mdc");

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

        let commands3 = vec![BuildCommand {
            line: 1,
            column: 0,
            package_manager: PackageManager::Yarn,
            command_type: CommandType::Install,
            raw_command: "yarn install".to_string(),
        }];

        let files = vec![(file1, commands1), (file2, commands2), (file3, commands3)];
        let conflicts = detect_build_conflicts(&files);

        // Should detect conflicts between npm/pnpm, npm/yarn, and pnpm/yarn
        assert_eq!(
            conflicts.len(),
            3,
            "Should detect 3 conflicts between 3 different package managers"
        );
    }

    #[test]
    fn test_detect_tool_conflicts_three_files() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");
        let file3 = PathBuf::from(".cursor/rules/dev.mdc");

        // file1 allows Bash
        let constraints1 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Allow,
            source_context: "allowed-tools:".to_string(),
        }];

        // file2 disallows Bash
        let constraints2 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Disallow,
            source_context: "never use".to_string(),
        }];

        // file3 also disallows Bash
        let constraints3 = vec![ToolConstraint {
            line: 1,
            column: 0,
            tool_name: "Bash".to_string(),
            constraint_type: ConstraintType::Disallow,
            source_context: "don't use".to_string(),
        }];

        let files = vec![
            (file1, constraints1),
            (file2, constraints2),
            (file3, constraints3),
        ];
        let conflicts = detect_tool_conflicts(&files);

        // Should detect 2 conflicts: file1 vs file2, file1 vs file3
        assert_eq!(
            conflicts.len(),
            2,
            "Should detect 2 conflicts (allow vs disallow pairs)"
        );
    }

    // ===== Empty file tests (review findings) =====

    #[test]
    fn test_extract_build_commands_empty_file() {
        let content = "";
        let results = extract_build_commands(content);
        assert!(
            results.is_empty(),
            "Empty file should have no build commands"
        );
    }

    #[test]
    fn test_extract_tool_constraints_empty_file() {
        let content = "";
        let results = extract_tool_constraints(content);
        assert!(
            results.is_empty(),
            "Empty file should have no tool constraints"
        );
    }

    #[test]
    fn test_detect_build_conflicts_empty_commands() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        // Both files have no commands
        let files: Vec<(PathBuf, Vec<BuildCommand>)> =
            vec![(file1, Vec::new()), (file2, Vec::new())];
        let conflicts = detect_build_conflicts(&files);

        assert!(
            conflicts.is_empty(),
            "Files with no commands should have no conflicts"
        );
    }

    #[test]
    fn test_detect_tool_conflicts_empty_constraints() {
        use std::path::PathBuf;

        let file1 = PathBuf::from("CLAUDE.md");
        let file2 = PathBuf::from("AGENTS.md");

        // Both files have no constraints
        let files: Vec<(PathBuf, Vec<ToolConstraint>)> =
            vec![(file1, Vec::new()), (file2, Vec::new())];
        let conflicts = detect_tool_conflicts(&files);

        assert!(
            conflicts.is_empty(),
            "Files with no constraints should have no conflicts"
        );
    }

    // ===== Short-form package manager command detection (issue fix) =====

    #[test]
    fn test_npm_i_without_trailing_space() {
        // "npm i" at end of line should be detected as Install
        let content = "Run `npm i` to install";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package_manager, PackageManager::Npm);
        assert_eq!(
            results[0].command_type,
            CommandType::Install,
            "npm i without trailing space should be Install"
        );
    }

    #[test]
    fn test_yarn_i_at_end_of_content() {
        // "yarn i\n" at end of content should be detected as Install
        let content = "Install with yarn i\n";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package_manager, PackageManager::Yarn);
        assert_eq!(
            results[0].command_type,
            CommandType::Install,
            "yarn i at end of line should be Install"
        );
    }

    #[test]
    fn test_pnpm_i_standalone() {
        // "pnpm i" as standalone command
        let content = "pnpm i";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].package_manager, PackageManager::Pnpm);
        assert_eq!(
            results[0].command_type,
            CommandType::Install,
            "pnpm i should be Install"
        );
    }

    #[test]
    fn test_bun_i_end_of_line() {
        // "bun i" at end of line in multi-line content
        let content = "First run bun i\nThen run bun run build";
        let results = extract_build_commands(content);
        assert_eq!(results.len(), 2);

        let install_cmd = results.iter().find(|r| r.raw_command.contains(" i"));
        assert!(install_cmd.is_some());
        assert_eq!(install_cmd.unwrap().command_type, CommandType::Install);
    }

    // ===== Backup file exclusion tests (issue fix) =====

    #[test]
    fn test_backup_file_claude_md_bak() {
        use std::path::PathBuf;
        assert!(
            !is_instruction_file(&PathBuf::from("CLAUDE.md.bak")),
            "CLAUDE.md.bak should NOT be considered an instruction file"
        );
    }

    #[test]
    fn test_backup_file_agents_md_old() {
        use std::path::PathBuf;
        assert!(
            !is_instruction_file(&PathBuf::from("AGENTS.md.old")),
            "AGENTS.md.old should NOT be considered an instruction file"
        );
    }

    #[test]
    fn test_backup_file_cursor_rules_tmp() {
        use std::path::PathBuf;
        assert!(
            !is_instruction_file(&PathBuf::from(".cursor/rules/test.mdc.tmp")),
            ".cursor/rules/test.mdc.tmp should NOT be considered an instruction file"
        );
    }

    #[test]
    fn test_backup_file_swp() {
        use std::path::PathBuf;
        assert!(
            !is_instruction_file(&PathBuf::from("CLAUDE.md.swp")),
            "CLAUDE.md.swp should NOT be considered an instruction file"
        );
    }

    #[test]
    fn test_backup_file_tilde() {
        use std::path::PathBuf;
        assert!(
            !is_instruction_file(&PathBuf::from("AGENTS.md~")),
            "AGENTS.md~ should NOT be considered an instruction file"
        );
    }

    #[test]
    fn test_valid_instruction_files_still_work() {
        use std::path::PathBuf;
        // Ensure normal files still work after adding backup exclusion
        assert!(is_instruction_file(&PathBuf::from("CLAUDE.md")));
        assert!(is_instruction_file(&PathBuf::from("AGENTS.md")));
        assert!(is_instruction_file(&PathBuf::from(
            ".cursor/rules/test.mdc"
        )));
        assert!(is_instruction_file(&PathBuf::from(
            ".github/copilot-instructions.md"
        )));
    }

    // ===== ReDoS Protection Tests =====

    #[test]
    fn test_find_claude_specific_features_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE (65536 bytes)
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_claude_specific_features(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_find_hard_coded_paths_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = find_hard_coded_paths(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_extract_build_commands_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = extract_build_commands(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_extract_tool_constraints_oversized_input() {
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "a".repeat(MAX_REGEX_INPUT_SIZE + 1000);
        let results = extract_tool_constraints(&large_content);
        // Should return empty to prevent ReDoS
        assert!(
            results.is_empty(),
            "Oversized content should be skipped for ReDoS protection"
        );
    }

    #[test]
    fn test_categorize_layer_oversized_input_precedence_doc() {
        use std::path::PathBuf;
        // Create content larger than MAX_REGEX_INPUT_SIZE
        let large_content = "precedence ".repeat((MAX_REGEX_INPUT_SIZE / 11) + 100);
        let layer = categorize_layer(&PathBuf::from("CLAUDE.md"), &large_content);
        // has_precedence_doc should be false for oversized input (safe default)
        assert!(
            !layer.has_precedence_doc,
            "Oversized content should not detect precedence for ReDoS protection"
        );
    }
}
