//! Skill file validation

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::{FrontmatterParts, split_frontmatter},
    regex_util::static_regex,
    rules::{Validator, ValidatorMetadata},
    schemas::hooks::HooksSchema,
    schemas::skill::SkillSchema,
    validation::is_valid_mcp_tool_format,
};
use regex::Regex;
use rust_i18n::t;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

mod helpers;
use helpers::*;

#[derive(Debug, Default, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    license: Option<String>,
    compatibility: Option<String>,
    metadata: Option<HashMap<String, String>>,
    #[serde(rename = "allowed-tools")]
    allowed_tools: Option<String>,
    #[serde(rename = "argument-hint")]
    argument_hint: Option<String>,
    #[serde(rename = "disable-model-invocation")]
    disable_model_invocation: Option<bool>,
    #[serde(rename = "user-invocable")]
    user_invocable: Option<bool>,
    model: Option<String>,
    context: Option<String>,
    agent: Option<String>,
}

#[derive(Debug, Clone)]
struct PathMatch {
    path: String,
    start: usize,
}

static_regex!(fn name_format_regex, r"^[a-z0-9]+(-[a-z0-9]+)*$");
static_regex!(fn consecutive_hyphen_regex, r"-{2,}");
static_regex!(fn description_xml_regex, r"<[^>]+>");
static_regex!(fn reference_path_regex, "(?i)\\b(?:references?|refs)[/\\\\][^\\s)\\]}>\"']+");
static_regex!(fn windows_path_regex, r"(?i)\b(?:[a-z]:)?[a-z0-9._-]+(?:\\[a-z0-9._-]+)+\b");
static_regex!(fn windows_path_token_regex, r"[^\s]+\\[^\s]+");
static_regex!(fn plain_bash_regex, r"\bBash\b");
static_regex!(fn imperative_verb_regex, r"(?i)\b(run|execute|create|build|deploy|install|configure|update|delete|remove|add|write|read|check|test|validate|ensure|make|use|call|invoke|start|stop|send|fetch|generate|implement|fix|analyze|review|search|find|move|copy|replace|push|pull|commit|clean|format|lint|parse|process|handle|prepare|download|upload|export|import|open|save|load|connect|verify|apply|enable|disable)\b");

/// Valid model values for CC-SK-001
const VALID_MODELS: &[&str] = &["sonnet", "opus", "haiku", "inherit"];

/// Built-in agent types for CC-SK-005
const BUILTIN_AGENTS: &[&str] = &["Explore", "Plan", "general-purpose"];

/// Known Claude Code tools for CC-SK-008
const KNOWN_TOOLS: &[&str] = &[
    "Bash",
    "Read",
    "Write",
    "Edit",
    "Grep",
    "Glob",
    "Task",
    "WebFetch",
    "WebSearch",
    "AskUserQuestion",
    "TodoRead",
    "TodoWrite",
    "MultiTool",
    "NotebookEdit",
    "EnterPlanMode",
    "ExitPlanMode",
    "Skill",
    "StatusBarMessageTool",
    "TaskOutput",
];

/// Maximum dynamic injections for CC-SK-009
const MAX_INJECTIONS: usize = 3;

/// Convert a name to kebab-case format.
/// - Lowercase the name
/// - Replace underscores with hyphens
/// - Remove invalid characters (not a-z, 0-9, or -)
/// - Collapse consecutive hyphens
/// - Trim leading/trailing hyphens
/// - Truncate to 64 characters
fn convert_to_kebab_case(name: &str) -> String {
    let mut kebab = String::with_capacity(name.len());
    let mut last_was_hyphen = true; // Use to trim leading hyphens and collapse consecutive ones

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            kebab.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if matches!(c, '_' | '-' | ' ') && !last_was_hyphen {
            kebab.push('-');
            last_was_hyphen = true;
        }
        // Other characters are skipped
    }

    // Trim trailing hyphen if it exists
    if last_was_hyphen && !kebab.is_empty() {
        kebab.pop();
    }

    // Truncate and re-trim if necessary
    if kebab.len() > 64 {
        kebab.truncate(64);
        while kebab.ends_with('-') {
            kebab.pop();
        }
    }

    kebab
}

/// Find byte positions of plain "Bash" (not scoped like "Bash(...)") in content
/// Returns Vec of (start_byte, end_byte) for each occurrence
fn find_plain_bash_positions(content: &str, search_start: usize) -> Vec<(usize, usize)> {
    let re = plain_bash_regex();

    let search_content = &content[search_start..];
    re.find_iter(search_content)
        .filter_map(|m| {
            let end_pos = search_start + m.end();
            // Check if followed by '(' - if so, it's scoped Bash, skip it
            let next_char = content.get(end_pos..end_pos + 1);
            if next_char == Some("(") {
                None // Scoped Bash like Bash(git:*), skip
            } else {
                Some((search_start + m.start(), end_pos))
            }
        })
        .collect()
}

/// Check if an agent name is valid for CC-SK-005.
/// Valid agents are:
/// - Built-in agents: Explore, Plan, general-purpose
/// - Custom agents: kebab-case format, 1-64 characters
fn is_valid_agent(agent: &str) -> bool {
    // Built-in agents are always valid
    if BUILTIN_AGENTS.contains(&agent) {
        return true;
    }

    // Custom agents must follow kebab-case format (1-64 chars)
    if !(1..=64).contains(&agent.len()) {
        return false;
    }

    // Reuse the same kebab-case regex used for skill names
    name_format_regex().is_match(agent)
}

/// Validation context holding shared state for skill validation.
/// Groups related validation methods and avoids passing many parameters.
struct ValidationContext<'a> {
    /// Path to the skill file being validated
    path: &'a Path,
    /// Raw file content
    content: &'a str,
    /// Lint configuration (rule enablement, filesystem access)
    config: &'a LintConfig,
    /// Parsed frontmatter sections (header, body, byte positions)
    parts: FrontmatterParts,
    /// Byte offsets of line starts for position tracking
    line_starts: Vec<usize>,
    /// Parsed frontmatter YAML (populated by validate_frontmatter_structure, consumed after)
    frontmatter: Option<SkillFrontmatter>,
    /// Accumulated diagnostics (errors, warnings)
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ValidationContext<'a> {
    fn new(path: &'a Path, content: &'a str, config: &'a LintConfig) -> Self {
        let parts = split_frontmatter(content);
        let line_starts = compute_line_starts(content);
        Self {
            path,
            content,
            config,
            parts,
            line_starts,
            frontmatter: None,
            diagnostics: Vec::new(),
        }
    }

    fn line_col_at(&self, offset: usize) -> (usize, usize) {
        line_col_at(offset, &self.line_starts)
    }

    fn frontmatter_key_line_col(&self, key: &str) -> (usize, usize) {
        frontmatter_key_line_col(&self.parts, key, &self.line_starts)
    }

    fn frontmatter_value_byte_range(&self, key: &str) -> Option<(usize, usize)> {
        frontmatter_value_byte_range(self.content, &self.parts, key)
    }

    fn frontmatter_key_line_byte_range(&self, key: &str) -> Option<(usize, usize)> {
        frontmatter_key_line_byte_range(self.content, &self.parts, key)
    }

    /// AS-001, AS-016: Validate frontmatter structure and parse
    fn validate_frontmatter_structure(&mut self) {
        let (frontmatter_line, frontmatter_col) = self.line_col_at(self.parts.frontmatter_start);

        // AS-001: Missing frontmatter
        if self.config.is_rule_enabled("AS-001")
            && (!self.parts.has_frontmatter || !self.parts.has_closing)
        {
            self.diagnostics.push(
                Diagnostic::error(
                    self.path.to_path_buf(),
                    frontmatter_line,
                    frontmatter_col,
                    "AS-001",
                    t!("rules.as_001.message"),
                )
                .with_suggestion(t!("rules.as_001.suggestion")),
            );
        }

        if self.parts.has_frontmatter && self.parts.has_closing {
            match parse_frontmatter_fields(&self.parts.frontmatter) {
                Ok(frontmatter) => {
                    self.frontmatter = Some(frontmatter);
                }
                Err(e) => {
                    if self.config.is_rule_enabled("AS-016") {
                        self.diagnostics.push(
                            Diagnostic::error(
                                self.path.to_path_buf(),
                                frontmatter_line,
                                frontmatter_col,
                                "AS-016",
                                t!("rules.as_016.message", error = e.to_string()),
                            )
                            .with_suggestion(t!("rules.as_016.suggestion")),
                        );
                    }
                }
            }
        }
    }

    /// AS-002, AS-003: Validate required name and description fields
    fn validate_required_fields(&mut self, frontmatter: &SkillFrontmatter) {
        let (name_line, name_col) = self.frontmatter_key_line_col("name");
        let (description_line, description_col) = self.frontmatter_key_line_col("description");

        // AS-002: Missing name field
        if self.config.is_rule_enabled("AS-002") && frontmatter.name.is_none() {
            self.diagnostics.push(
                Diagnostic::error(
                    self.path.to_path_buf(),
                    name_line,
                    name_col,
                    "AS-002",
                    t!("rules.as_002.message"),
                )
                .with_suggestion(t!("rules.as_002.suggestion")),
            );
        }

        // AS-003: Missing description field
        if self.config.is_rule_enabled("AS-003") && frontmatter.description.is_none() {
            self.diagnostics.push(
                Diagnostic::error(
                    self.path.to_path_buf(),
                    description_line,
                    description_col,
                    "AS-003",
                    t!("rules.as_003.message"),
                )
                .with_suggestion(t!("rules.as_003.suggestion")),
            );
        }
    }

    /// AS-004, AS-005, AS-006, AS-007: Validate name format and rules
    fn validate_name_rules(&mut self, name: &str) {
        let (name_line, name_col) = self.frontmatter_key_line_col("name");
        let name_trimmed = name.trim();

        // AS-004: Invalid name format
        if self.config.is_rule_enabled("AS-004") {
            let name_re = name_format_regex();
            if name_trimmed.len() > 64 || !name_re.is_match(name_trimmed) {
                let fixed_name = convert_to_kebab_case(name_trimmed);
                let mut diagnostic = Diagnostic::error(
                    self.path.to_path_buf(),
                    name_line,
                    name_col,
                    "AS-004",
                    t!("rules.as_004.message", name = name_trimmed),
                )
                .with_suggestion(t!("rules.as_004.suggestion"));

                // Add auto-fix if we can find the byte range and the fixed name is valid
                if !fixed_name.is_empty() && name_re.is_match(&fixed_name) {
                    if let Some((start, end)) = self.frontmatter_value_byte_range("name") {
                        // Determine if fix is safe: only case changes are safe
                        let has_structural_changes = name_trimmed.contains('_')
                            || name_trimmed.contains(' ')
                            || name_trimmed
                                .chars()
                                .any(|c| !c.is_ascii_alphanumeric() && c != '-');
                        let is_case_only =
                            !has_structural_changes && name_trimmed.to_lowercase() == fixed_name;
                        let fix = Fix::replace(
                            start,
                            end,
                            &fixed_name,
                            t!("rules.as_004.fix", name = fixed_name.clone()),
                            is_case_only,
                        );
                        diagnostic = diagnostic.with_fix(fix);
                    }
                }

                self.diagnostics.push(diagnostic);
            }
        }

        // AS-005: Name cannot start or end with hyphen
        if self.config.is_rule_enabled("AS-005")
            && (name_trimmed.starts_with('-') || name_trimmed.ends_with('-'))
        {
            let mut diagnostic = Diagnostic::error(
                self.path.to_path_buf(),
                name_line,
                name_col,
                "AS-005",
                t!("rules.as_005.message", name = name_trimmed),
            )
            .with_suggestion(t!("rules.as_005.suggestion"));

            // Safe auto-fix: trim leading/trailing hyphens.
            if let Some((start, end)) = self.frontmatter_value_byte_range("name") {
                let fixed_name = name_trimmed.trim_matches('-').to_string();
                if !fixed_name.is_empty()
                    && fixed_name != name_trimmed
                    && name_format_regex().is_match(&fixed_name)
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        fixed_name,
                        "Remove leading/trailing hyphens from skill name",
                        true,
                    ));
                }
            }

            self.diagnostics.push(diagnostic);
        }

        // AS-006: Name cannot contain consecutive hyphens
        if self.config.is_rule_enabled("AS-006") && name_trimmed.contains("--") {
            let mut diagnostic = Diagnostic::error(
                self.path.to_path_buf(),
                name_line,
                name_col,
                "AS-006",
                t!("rules.as_006.message", name = name_trimmed),
            )
            .with_suggestion(t!("rules.as_006.suggestion"));

            // Safe auto-fix: collapse repeated hyphens.
            if let Some((start, end)) = self.frontmatter_value_byte_range("name") {
                let fixed_name = consecutive_hyphen_regex()
                    .replace_all(name_trimmed, "-")
                    .to_string();
                if !fixed_name.is_empty()
                    && fixed_name != name_trimmed
                    && name_format_regex().is_match(&fixed_name)
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        fixed_name,
                        "Collapse consecutive hyphens in skill name",
                        true,
                    ));
                }
            }

            self.diagnostics.push(diagnostic);
        }

        // AS-007: Reserved name
        if self.config.is_rule_enabled("AS-007") && !name_trimmed.is_empty() {
            let reserved = ["anthropic", "claude", "skill"];
            if reserved.contains(&name_trimmed.to_lowercase().as_str()) {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        name_line,
                        name_col,
                        "AS-007",
                        t!("rules.as_007.message", name = name_trimmed),
                    )
                    .with_suggestion(t!("rules.as_007.suggestion")),
                );
            }
        }
    }

    /// AS-008, AS-009, AS-010: Validate description format and rules
    fn validate_description_rules(&mut self, description: &str) {
        let (description_line, description_col) = self.frontmatter_key_line_col("description");
        let description_trimmed = description.trim();

        // AS-008: Description length
        if self.config.is_rule_enabled("AS-008") {
            let len = description_trimmed.len();
            if !(1..=1024).contains(&len) {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        description_line,
                        description_col,
                        "AS-008",
                        t!("rules.as_008.message", len = len),
                    )
                    .with_suggestion(t!("rules.as_008.suggestion")),
                );
            }
        }

        // AS-009: Description contains XML tags
        if self.config.is_rule_enabled("AS-009") && description_xml_regex().is_match(description) {
            self.diagnostics.push(
                Diagnostic::error(
                    self.path.to_path_buf(),
                    description_line,
                    description_col,
                    "AS-009",
                    t!("rules.as_009.message"),
                )
                .with_suggestion(t!("rules.as_009.suggestion")),
            );
        }

        // AS-010: Description should include trigger phrase
        if self.config.is_rule_enabled("AS-010") && !description_trimmed.is_empty() {
            let desc_lower = description_trimmed.to_lowercase();
            if !desc_lower.contains("use when") {
                let mut diagnostic = Diagnostic::warning(
                    self.path.to_path_buf(),
                    description_line,
                    description_col,
                    "AS-010",
                    t!("rules.as_010.message"),
                )
                .with_suggestion(t!("rules.as_010.suggestion"));

                // Add auto-fix: prepend "Use when user wants to " to description
                if let Some((start, end)) = self.frontmatter_value_byte_range("description") {
                    let new_description = format!("Use when user wants to {}", description_trimmed);
                    // Check if the new description would exceed length limit
                    if new_description.len() <= 1024 {
                        let fix = Fix::replace(
                            start,
                            end,
                            &new_description,
                            t!("rules.as_010.fix"),
                            false, // Not safe - changes semantics
                        );
                        diagnostic = diagnostic.with_fix(fix);
                    }
                }

                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// AS-011: Validate compatibility field length
    fn validate_compatibility(&mut self, frontmatter: &SkillFrontmatter) {
        if self.config.is_rule_enabled("AS-011") {
            if let Some(compat) = frontmatter.compatibility.as_deref() {
                let (compat_line, compat_col) = self.frontmatter_key_line_col("compatibility");
                let len = compat.trim().len();
                if len == 0 || len > 500 {
                    self.diagnostics.push(
                        Diagnostic::error(
                            self.path.to_path_buf(),
                            compat_line,
                            compat_col,
                            "AS-011",
                            t!("rules.as_011.message", len = len),
                        )
                        .with_suggestion(t!("rules.as_011.suggestion")),
                    );
                }
            }
        }
    }

    /// CC-SK-001, CC-SK-002, CC-SK-003, CC-SK-004: Model and context validation
    fn validate_cc_model_context(&mut self, schema: &SkillSchema) {
        let (model_line, model_col) = self.frontmatter_key_line_col("model");
        let (context_line, context_col) = self.frontmatter_key_line_col("context");
        let (agent_line, agent_col) = self.frontmatter_key_line_col("agent");

        // CC-SK-001: Invalid model value
        if self.config.is_rule_enabled("CC-SK-001") {
            if let Some(model) = &schema.model {
                if !VALID_MODELS.contains(&model.as_str()) {
                    let mut diagnostic = Diagnostic::error(
                        self.path.to_path_buf(),
                        model_line,
                        model_col,
                        "CC-SK-001",
                        t!(
                            "rules.cc_sk_001.message",
                            model = model.as_str(),
                            valid = VALID_MODELS.join(", ")
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.cc_sk_001.suggestion",
                        valid = VALID_MODELS.join(", ")
                    ));

                    // Unsafe auto-fix: default invalid model to sonnet.
                    if let Some((start, end)) = self.frontmatter_value_byte_range("model") {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "sonnet",
                            "Replace invalid model with 'sonnet'",
                            false,
                        ));
                    }

                    self.diagnostics.push(diagnostic);
                }
            }
        }

        // CC-SK-002: Invalid context value
        if self.config.is_rule_enabled("CC-SK-002") {
            if let Some(context) = &schema.context {
                if context != "fork" {
                    let mut diagnostic = Diagnostic::error(
                        self.path.to_path_buf(),
                        context_line,
                        context_col,
                        "CC-SK-002",
                        t!("rules.cc_sk_002.message", context = context.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_sk_002.suggestion"));

                    // Unsafe auto-fix: normalize context to fork.
                    if let Some((start, end)) = self.frontmatter_value_byte_range("context") {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "fork",
                            "Replace invalid context with 'fork'",
                            false,
                        ));
                    }

                    self.diagnostics.push(diagnostic);
                }
            }
        }

        // CC-SK-003: Context without agent
        if self.config.is_rule_enabled("CC-SK-003")
            && schema.context.as_deref() == Some("fork")
            && schema.agent.is_none()
        {
            let mut diagnostic = Diagnostic::error(
                self.path.to_path_buf(),
                context_line,
                context_col,
                "CC-SK-003",
                t!("rules.cc_sk_003.message"),
            )
            .with_suggestion(t!("rules.cc_sk_003.suggestion"));

            // Unsafe auto-fix: add default agent when context is fork.
            if let Some((_, context_line_end)) = self.frontmatter_key_line_byte_range("context") {
                diagnostic = diagnostic.with_fix(Fix::insert(
                    context_line_end,
                    "agent: general-purpose\n",
                    "Add required 'agent' for context: fork",
                    false,
                ));
            }

            self.diagnostics.push(diagnostic);
        }

        // CC-SK-004: Agent without context
        if self.config.is_rule_enabled("CC-SK-004")
            && schema.agent.is_some()
            && schema.context.as_deref() != Some("fork")
        {
            let mut diagnostic = Diagnostic::error(
                self.path.to_path_buf(),
                agent_line,
                agent_col,
                "CC-SK-004",
                t!("rules.cc_sk_004.message"),
            )
            .with_suggestion(t!("rules.cc_sk_004.suggestion"));

            // Unsafe auto-fix:
            // - if context exists, normalize to fork
            // - otherwise insert context before agent key
            if let Some((start, end)) = self.frontmatter_value_byte_range("context") {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    "fork",
                    "Set context to 'fork' when agent is configured",
                    false,
                ));
            } else if let Some((agent_line_start, _)) =
                self.frontmatter_key_line_byte_range("agent")
            {
                diagnostic = diagnostic.with_fix(Fix::insert(
                    agent_line_start,
                    "context: fork\n",
                    "Add required 'context: fork' for agent usage",
                    false,
                ));
            }

            self.diagnostics.push(diagnostic);
        }
    }

    /// CC-SK-005: Validate agent type
    fn validate_cc_agent(&mut self, schema: &SkillSchema) {
        if self.config.is_rule_enabled("CC-SK-005") {
            if let Some(agent) = &schema.agent {
                if !is_valid_agent(agent) {
                    let (agent_line, agent_col) = self.frontmatter_key_line_col("agent");
                    let mut diagnostic = Diagnostic::error(
                        self.path.to_path_buf(),
                        agent_line,
                        agent_col,
                        "CC-SK-005",
                        t!("rules.cc_sk_005.message", agent = agent.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_sk_005.suggestion"));

                    // Unsafe auto-fix: replace invalid agent with 'general-purpose'.
                    if let Some((start, end)) = self.frontmatter_value_byte_range("agent") {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "general-purpose",
                            t!("rules.cc_sk_005.fix"),
                            false,
                        ));
                    }

                    self.diagnostics.push(diagnostic);
                }
            }
        }
    }

    /// CC-SK-007, CC-SK-008: Validate allowed tools
    fn validate_cc_tools(&mut self, schema: &SkillSchema) {
        let (allowed_tools_line, allowed_tools_col) =
            self.frontmatter_key_line_col("allowed-tools");

        // Parse allowed_tools once for CC-SK-007 and CC-SK-008
        // Supports both formats:
        // - Comma-separated: "Bash(git:*), Read, Grep" (preferred)
        // - Space-separated: "Read Write Grep" (legacy)
        let tool_list: Option<Vec<&str>> = schema.allowed_tools.as_ref().map(|tools| {
            if tools.contains(',') {
                // Comma-separated format
                tools
                    .split(',')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .collect()
            } else {
                // Space-separated format (legacy)
                tools.split_whitespace().collect()
            }
        });

        // CC-SK-007: Unrestricted Bash warning
        if self.config.is_rule_enabled("CC-SK-007") {
            if let Some(ref tools) = tool_list {
                // Find all plain Bash occurrences in the allowed-tools line only
                // to avoid matching "Bash" in other fields like description
                let search_start = frontmatter_key_offset(&self.parts.frontmatter, "allowed-tools")
                    .map(|offset| self.parts.frontmatter_start + offset)
                    .unwrap_or(self.parts.frontmatter_start);
                let bash_positions = find_plain_bash_positions(self.content, search_start);

                let mut bash_pos_iter = bash_positions.iter();

                for &tool in tools {
                    if tool == "Bash" {
                        let mut diagnostic = Diagnostic::warning(
                            self.path.to_path_buf(),
                            allowed_tools_line,
                            allowed_tools_col,
                            "CC-SK-007",
                            t!("rules.cc_sk_007.message"),
                        )
                        .with_suggestion(t!("rules.cc_sk_007.suggestion"));

                        // Try to attach a fix for each plain Bash
                        if let Some(&(start, end)) = bash_pos_iter.next() {
                            // Default replacement: Bash(git:*) as a common use case
                            // safe=false because we don't know what scope the user wants
                            let fix = Fix::replace(
                                start,
                                end,
                                "Bash(git:*)",
                                t!("rules.cc_sk_007.fix"),
                                false,
                            );
                            diagnostic = diagnostic.with_fix(fix);
                        }

                        self.diagnostics.push(diagnostic);
                    }
                }
            }
        }

        // CC-SK-008: Unknown tool name
        if self.config.is_rule_enabled("CC-SK-008") {
            if let Some(ref tools) = tool_list {
                // Compute known tools list once outside loop
                static KNOWN_TOOLS_LIST: OnceLock<String> = OnceLock::new();
                let known_tools_str = KNOWN_TOOLS_LIST.get_or_init(|| KNOWN_TOOLS.join(", "));

                for &tool in tools {
                    let base_name = tool.split('(').next().unwrap_or(tool);
                    if !is_valid_skill_tool_name(base_name) {
                        self.diagnostics.push(
                            Diagnostic::error(
                                self.path.to_path_buf(),
                                allowed_tools_line,
                                allowed_tools_col,
                                "CC-SK-008",
                                t!(
                                    "rules.cc_sk_008.message",
                                    tool = base_name,
                                    known = known_tools_str.as_str()
                                ),
                            )
                            .with_suggestion(t!(
                                "rules.cc_sk_008.suggestion",
                                known = known_tools_str.as_str()
                            )),
                        );
                    }
                }
            }
        }
    }

    /// CC-SK-006, CC-SK-009: Safety-related validations
    fn validate_cc_safety(&mut self, schema: &SkillSchema, frontmatter: &SkillFrontmatter) {
        let (name_line, name_col) = self.frontmatter_key_line_col("name");
        let (frontmatter_line, frontmatter_col) = self.line_col_at(self.parts.frontmatter_start);

        // CC-SK-006: Dangerous auto-invocation check
        if self.config.is_rule_enabled("CC-SK-006") {
            const DANGEROUS_NAMES: &[&str] =
                &["deploy", "ship", "publish", "delete", "release", "push"];
            let name_lower = schema.name.to_lowercase();
            if DANGEROUS_NAMES.iter().any(|d| name_lower.contains(d))
                && !frontmatter.disable_model_invocation.unwrap_or(false)
            {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        name_line,
                        name_col,
                        "CC-SK-006",
                        t!("rules.cc_sk_006.message", name = schema.name.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_sk_006.suggestion")),
                );
            }
        }

        // CC-SK-009: Too many injections (warning)
        // Count across full content (frontmatter + body) per VALIDATION-RULES.md
        if self.config.is_rule_enabled("CC-SK-009") {
            let injection_count = self.content.matches("!`").count();
            if injection_count > MAX_INJECTIONS {
                self.diagnostics.push(
                    Diagnostic::warning(
                        self.path.to_path_buf(),
                        frontmatter_line,
                        frontmatter_col,
                        "CC-SK-009",
                        t!(
                            "rules.cc_sk_009.message",
                            count = injection_count,
                            max = MAX_INJECTIONS
                        ),
                    )
                    .with_suggestion(t!("rules.cc_sk_009.suggestion")),
                );
            }
        }
    }

    /// CC-SK-010: Validate hooks field in skill frontmatter
    fn validate_cc_hooks(&mut self) {
        if !self.config.is_rule_enabled("CC-SK-010") {
            return;
        }

        // Check if hooks key exists in raw frontmatter
        let frontmatter = &self.parts.frontmatter;
        let has_hooks = frontmatter.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("hooks:") || trimmed == "hooks:"
        });

        if !has_hooks {
            return;
        }

        let (hooks_line, hooks_col) = self.frontmatter_key_line_col("hooks");

        // Try to parse the full frontmatter as a generic YAML Value to extract hooks
        let yaml_value: serde_yaml::Value = match serde_yaml::from_str(frontmatter) {
            Ok(v) => v,
            Err(_) => return, // Parse errors handled by AS-016
        };

        let hooks_value = match yaml_value.get("hooks") {
            Some(v) => v,
            None => return,
        };

        // hooks must be a mapping (object)
        let hooks_map = match hooks_value.as_mapping() {
            Some(m) => m,
            None => {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        hooks_line,
                        hooks_col,
                        "CC-SK-010",
                        t!(
                            "rules.cc_sk_010.message",
                            error = "hooks must be a mapping of event names to hook arrays"
                        ),
                    )
                    .with_suggestion(t!("rules.cc_sk_010.suggestion")),
                );
                return;
            }
        };

        // Validate each event key
        for (key, value) in hooks_map {
            let event = match key.as_str() {
                Some(s) => s,
                None => {
                    self.diagnostics.push(
                        Diagnostic::error(
                            self.path.to_path_buf(),
                            hooks_line,
                            hooks_col,
                            "CC-SK-010",
                            t!(
                                "rules.cc_sk_010.message",
                                error = "hook event key must be a string"
                            ),
                        )
                        .with_suggestion(t!("rules.cc_sk_010.suggestion")),
                    );
                    continue;
                }
            };

            // Validate event name
            if !HooksSchema::VALID_EVENTS.contains(&event) {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        hooks_line,
                        hooks_col,
                        "CC-SK-010",
                        t!(
                            "rules.cc_sk_010.message",
                            error = format!(
                                "invalid hook event '{}', valid events: {}",
                                event,
                                HooksSchema::VALID_EVENTS.join(", ")
                            )
                        ),
                    )
                    .with_suggestion(t!("rules.cc_sk_010.suggestion")),
                );
            }

            // Validate value is a sequence
            if !value.is_sequence() {
                self.diagnostics.push(
                    Diagnostic::error(
                        self.path.to_path_buf(),
                        hooks_line,
                        hooks_col,
                        "CC-SK-010",
                        t!(
                            "rules.cc_sk_010.message",
                            error = format!("hooks for event '{}' must be an array", event)
                        ),
                    )
                    .with_suggestion(t!("rules.cc_sk_010.suggestion")),
                );
            }
        }
    }

    /// CC-SK-011: Validate unreachable skill (both user-invocable=false and disable-model-invocation=true)
    fn validate_cc_unreachable(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-011") {
            return;
        }

        let user_invocable = frontmatter.user_invocable.unwrap_or(true);
        let disable_model = frontmatter.disable_model_invocation.unwrap_or(false);

        if !user_invocable && disable_model {
            let (line, col) = self.frontmatter_key_line_col("user-invocable");
            let mut diagnostic = Diagnostic::error(
                self.path.to_path_buf(),
                line,
                col,
                "CC-SK-011",
                t!("rules.cc_sk_011.message"),
            )
            .with_suggestion(t!("rules.cc_sk_011.suggestion"));

            // Unsafe auto-fix: remove disable-model-invocation line to allow model invocation
            if let Some((start, end)) =
                self.frontmatter_key_line_byte_range("disable-model-invocation")
            {
                diagnostic = diagnostic.with_fix(Fix::delete(
                    start,
                    end,
                    t!("rules.cc_sk_011.fix"),
                    false, // unsafe
                ));
            }

            self.diagnostics.push(diagnostic);
        }
    }

    /// CC-SK-012: Validate argument-hint has matching $ARGUMENTS in body
    fn validate_cc_argument_hint(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-012") {
            return;
        }

        if frontmatter.argument_hint.is_some() {
            let body = if self.parts.body_start <= self.content.len() {
                &self.content[self.parts.body_start..]
            } else {
                ""
            };

            if !body.contains("$ARGUMENTS") {
                let (line, col) = self.frontmatter_key_line_col("argument-hint");
                self.diagnostics.push(
                    Diagnostic::warning(
                        self.path.to_path_buf(),
                        line,
                        col,
                        "CC-SK-012",
                        t!("rules.cc_sk_012.message"),
                    )
                    .with_suggestion(t!("rules.cc_sk_012.suggestion")),
                );
            }
        }
    }

    /// CC-SK-013: Validate context: fork has actionable instructions
    fn validate_cc_fork_instructions(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-013") {
            return;
        }

        if frontmatter.context.as_deref() != Some("fork") {
            return;
        }

        let body = if self.parts.body_start <= self.content.len() {
            &self.content[self.parts.body_start..]
        } else {
            ""
        };

        // Check if the body is empty or lacks imperative verbs
        if body.trim().is_empty() || !imperative_verb_regex().is_match(body) {
            let (line, col) = self.frontmatter_key_line_col("context");
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "CC-SK-013",
                    t!("rules.cc_sk_013.message"),
                )
                .with_suggestion(t!("rules.cc_sk_013.suggestion")),
            );
        }
    }

    /// CC-SK-014, CC-SK-015: Validate boolean field types from raw YAML
    /// Detects quoted string values like "true" or "false" that should be unquoted booleans
    fn validate_cc_boolean_types(&mut self) {
        self.validate_boolean_field("disable-model-invocation", "CC-SK-014", "cc_sk_014");
        self.validate_boolean_field("user-invocable", "CC-SK-015", "cc_sk_015");
    }

    /// Helper to check a single boolean field for string type
    fn validate_boolean_field(&mut self, field_name: &str, rule_id: &str, i18n_key: &str) {
        if !self.config.is_rule_enabled(rule_id) {
            return;
        }

        // Search raw frontmatter for the field with a quoted value
        let frontmatter = &self.parts.frontmatter;
        for line in frontmatter.lines() {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix(field_name) {
                if let Some(after_colon) = rest.trim_start().strip_prefix(':') {
                    // Strip inline YAML comments before checking the value
                    let value_str = after_colon.split('#').next().unwrap_or("").trim();
                    // Check for quoted boolean strings
                    let is_quoted_bool = value_str == "\"true\""
                        || value_str == "\"false\""
                        || value_str == "'true'"
                        || value_str == "'false'";

                    if is_quoted_bool {
                        let inner_value = value_str.trim_matches('"').trim_matches('\'');
                        let fixed_bool = inner_value == "true";

                        let (line_num, col) = self.frontmatter_key_line_col(field_name);

                        let msg_key = format!("rules.{}.message", i18n_key);
                        let sug_key = format!("rules.{}.suggestion", i18n_key);

                        let mut diagnostic = Diagnostic::error(
                            self.path.to_path_buf(),
                            line_num,
                            col,
                            rule_id,
                            t!(&msg_key, value = inner_value),
                        )
                        .with_suggestion(t!(&sug_key));

                        // Add auto-fix: replace quoted string with boolean
                        if let Some((start, end)) = self.frontmatter_value_byte_range(field_name) {
                            // The value range from frontmatter_value_byte_range returns
                            // the inner content for quoted values, but we need to replace
                            // including quotes. Expand to include surrounding quotes.
                            let content_bytes = self.content.as_bytes();
                            let quote_start = if start > 0
                                && (content_bytes[start - 1] == b'"'
                                    || content_bytes[start - 1] == b'\'')
                            {
                                start - 1
                            } else {
                                start
                            };
                            let quote_end = if end < self.content.len()
                                && (content_bytes[end] == b'"' || content_bytes[end] == b'\'')
                            {
                                end + 1
                            } else {
                                end
                            };

                            let fix_key = format!("rules.{}.fix", i18n_key);
                            let fix = Fix::replace(
                                quote_start,
                                quote_end,
                                fixed_bool.to_string(),
                                t!(&fix_key, value = inner_value, fixed = fixed_bool),
                                true, // safe fix
                            );
                            diagnostic = diagnostic.with_fix(fix);
                        }

                        self.diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    /// AS-012, AS-013, AS-014: Validate body content
    fn validate_body_rules(&mut self) {
        let body_raw = if self.parts.body_start <= self.content.len() {
            &self.content[self.parts.body_start..]
        } else {
            ""
        };
        let (body_line, body_col) = self.line_col_at(self.parts.body_start);

        // AS-012: Content exceeds 500 lines
        if self.config.is_rule_enabled("AS-012") {
            let line_count = body_raw.lines().count();
            if line_count > 500 {
                self.diagnostics.push(
                    Diagnostic::warning(
                        self.path.to_path_buf(),
                        body_line,
                        body_col,
                        "AS-012",
                        t!("rules.as_012.message", count = line_count),
                    )
                    .with_suggestion(t!("rules.as_012.suggestion")),
                );
            }
        }

        // AS-013: File reference too deep
        if self.config.is_rule_enabled("AS-013") {
            let paths = extract_reference_paths(body_raw);
            for ref_path in paths {
                if reference_path_too_deep(&ref_path.path) {
                    let (line, col) = self.line_col_at(self.parts.body_start + ref_path.start);
                    self.diagnostics.push(
                        Diagnostic::error(
                            self.path.to_path_buf(),
                            line,
                            col,
                            "AS-013",
                            t!("rules.as_013.message", path = ref_path.path.as_str()),
                        )
                        .with_suggestion(t!("rules.as_013.suggestion")),
                    );
                }
            }
        }

        // AS-014: Windows path separator
        if self.config.is_rule_enabled("AS-014") {
            let paths = extract_windows_paths(body_raw);
            for win_path in paths {
                let (line, col) = self.line_col_at(self.parts.body_start + win_path.start);
                let mut diagnostic = Diagnostic::error(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-014",
                    t!("rules.as_014.message", path = win_path.path.as_str()),
                )
                .with_suggestion(t!("rules.as_014.suggestion"));

                // Safe auto-fix: normalize path separators in-place.
                let replacement = win_path.path.replace('\\', "/");
                let abs_start = self.parts.body_start + win_path.start;
                let abs_end = abs_start + win_path.path.len();
                if replacement != win_path.path
                    && abs_end <= self.content.len()
                    && self.content.is_char_boundary(abs_start)
                    && self.content.is_char_boundary(abs_end)
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        abs_start,
                        abs_end,
                        replacement,
                        "Normalize Windows path separators to '/'",
                        true,
                    ));
                }

                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// AS-015: Validate directory size
    fn validate_directory(&mut self) {
        if self.config.is_rule_enabled("AS-015") && self.path.is_file() {
            if let Some(dir) = self.path.parent() {
                let (frontmatter_line, frontmatter_col) =
                    self.line_col_at(self.parts.frontmatter_start);
                const MAX_BYTES: u64 = 8 * 1024 * 1024;
                let size = directory_size_until(dir, MAX_BYTES, self.config.fs().as_ref());
                if size > MAX_BYTES {
                    self.diagnostics.push(
                        Diagnostic::error(
                            self.path.to_path_buf(),
                            frontmatter_line,
                            frontmatter_col,
                            "AS-015",
                            t!("rules.as_015.message", size = size),
                        )
                        .with_suggestion(t!("rules.as_015.suggestion")),
                    );
                }
            }
        }
    }
}

const RULE_IDS: &[&str] = &[
    "AS-001", "AS-002", "AS-003", "AS-004", "AS-005", "AS-006", "AS-007", "AS-008", "AS-009",
    "AS-010", "AS-011", "AS-012", "AS-013", "AS-014", "AS-015", "AS-016", "CC-SK-001",
    "CC-SK-002", "CC-SK-003", "CC-SK-004", "CC-SK-005", "CC-SK-006", "CC-SK-007", "CC-SK-008",
    "CC-SK-009", "CC-SK-010", "CC-SK-011", "CC-SK-012", "CC-SK-013", "CC-SK-014", "CC-SK-015",
];

pub struct SkillValidator;

/// Helper to check if a tool name is valid for skills (either known or properly formatted MCP tool).
fn is_valid_skill_tool_name(tool: &str) -> bool {
    is_valid_mcp_tool_format(tool, KNOWN_TOOLS)
}

impl Validator for SkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        if !config.rules().frontmatter_validation {
            return Vec::new();
        }

        let mut ctx = ValidationContext::new(path, content, config);

        // Phase 0: Raw YAML type checks (CC-SK-014, CC-SK-015)
        // Run before serde parsing since string booleans cause parse failures
        if ctx.parts.has_frontmatter && ctx.parts.has_closing {
            ctx.validate_cc_boolean_types();
        }

        // Phase 1: Structure validation (AS-001, AS-016)
        ctx.validate_frontmatter_structure();

        // Early return if frontmatter couldn't be parsed
        let Some(frontmatter) = ctx.frontmatter.take() else {
            return ctx.diagnostics;
        };

        // Phase 2: Required fields (AS-002, AS-003)
        ctx.validate_required_fields(&frontmatter);

        // Phase 3: Name validation (AS-004, AS-005, AS-006, AS-007)
        if let Some(name) = frontmatter.name.as_deref() {
            ctx.validate_name_rules(name);
        }

        // Phase 4: Description validation (AS-008, AS-009, AS-010)
        if let Some(description) = frontmatter.description.as_deref() {
            ctx.validate_description_rules(description);
        }

        // Phase 5: Compatibility validation (AS-011)
        ctx.validate_compatibility(&frontmatter);

        // Phase 6: CC-SK-010 (hooks in frontmatter)
        ctx.validate_cc_hooks();

        // Phase 7: CC-SK-011 (unreachable skill)
        ctx.validate_cc_unreachable(&frontmatter);

        // Phase 8: CC-SK-012 (argument-hint without $ARGUMENTS)
        ctx.validate_cc_argument_hint(&frontmatter);

        // Phase 9: CC-SK-013 (fork without actionable instructions)
        ctx.validate_cc_fork_instructions(&frontmatter);

        // Phase 10-13: Claude Code rules (CC-SK-001-009)
        // These require both name and description to be non-empty
        if let (Some(name), Some(description)) = (
            frontmatter.name.as_deref(),
            frontmatter.description.as_deref(),
        ) {
            let name_trimmed = name.trim();
            let description_trimmed = description.trim();
            if !name_trimmed.is_empty() && !description_trimmed.is_empty() {
                let schema = SkillSchema {
                    name: name_trimmed.to_string(),
                    description: description_trimmed.to_string(),
                    license: frontmatter.license.clone(),
                    compatibility: frontmatter.compatibility.clone(),
                    metadata: frontmatter.metadata.clone(),
                    allowed_tools: frontmatter.allowed_tools.clone(),
                    argument_hint: frontmatter.argument_hint.clone(),
                    disable_model_invocation: frontmatter.disable_model_invocation,
                    user_invocable: frontmatter.user_invocable,
                    model: frontmatter.model.clone(),
                    context: frontmatter.context.clone(),
                    agent: frontmatter.agent.clone(),
                };

                // CC-SK-006 (dangerous auto-invocation) and CC-SK-009 (too many injections)
                ctx.validate_cc_safety(&schema, &frontmatter);

                // CC-SK-007 (unrestricted Bash) and CC-SK-008 (unknown tools)
                ctx.validate_cc_tools(&schema);

                // CC-SK-001-004 (model/context validation)
                ctx.validate_cc_model_context(&schema);

                // CC-SK-005 (agent type)
                ctx.validate_cc_agent(&schema);
            }
        }

        // Phase 14: Body validation (AS-012, AS-013, AS-014)
        ctx.validate_body_rules();

        // Phase 15: Directory validation (AS-015)
        ctx.validate_directory();

        ctx.diagnostics
    }
}

#[cfg(test)]
mod tests;
