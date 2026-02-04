//! Skill file validation

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::{split_frontmatter, FrontmatterParts},
    rules::Validator,
    schemas::skill::SkillSchema,
};
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

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

static NAME_FORMAT_REGEX: OnceLock<Regex> = OnceLock::new();
static DESCRIPTION_XML_REGEX: OnceLock<Regex> = OnceLock::new();
static REFERENCE_PATH_REGEX: OnceLock<Regex> = OnceLock::new();
static WINDOWS_PATH_REGEX: OnceLock<Regex> = OnceLock::new();
static WINDOWS_PATH_TOKEN_REGEX: OnceLock<Regex> = OnceLock::new();
static PLAIN_BASH_REGEX: OnceLock<Regex> = OnceLock::new();

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
    "AskUserQuestion",
    "TodoRead",
    "TodoWrite",
    "MultiTool",
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
    let re = PLAIN_BASH_REGEX.get_or_init(|| {
        // Match "Bash" at word boundary
        // Note: regex crate doesn't support lookahead, so we'll filter manually
        Regex::new(r"\bBash\b").unwrap()
    });

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
    let re = NAME_FORMAT_REGEX.get_or_init(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap());
    re.is_match(agent)
}

pub struct SkillValidator;

impl Validator for SkillValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules.frontmatter_validation {
            return diagnostics;
        }

        let parts = split_frontmatter(content);
        let line_starts = compute_line_starts(content);
        let body_raw = if parts.body_start <= content.len() {
            &content[parts.body_start..]
        } else {
            ""
        };
        let (frontmatter_line, frontmatter_col) =
            line_col_at(parts.frontmatter_start, &line_starts);
        let (body_line, body_col) = line_col_at(parts.body_start, &line_starts);

        // AS-001: Missing frontmatter
        if config.is_rule_enabled("AS-001") && (!parts.has_frontmatter || !parts.has_closing) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    frontmatter_line,
                    frontmatter_col,
                    "AS-001",
                    "SKILL.md must have YAML frontmatter between --- markers".to_string(),
                )
                .with_suggestion("Add frontmatter between --- markers".to_string()),
            );
        }

        let frontmatter = if parts.has_frontmatter && parts.has_closing {
            match parse_frontmatter_fields(&parts.frontmatter) {
                Ok(frontmatter) => Some(frontmatter),
                Err(e) => {
                    if config.is_rule_enabled("AS-016") {
                        diagnostics.push(Diagnostic::error(
                            path.to_path_buf(),
                            frontmatter_line,
                            frontmatter_col,
                            "AS-016",
                            format!("Failed to parse SKILL.md: {}", e),
                        ));
                    }
                    None
                }
            }
        } else {
            None
        };

        if let Some(frontmatter) = frontmatter {
            let (name_line, name_col) = frontmatter_key_line_col(&parts, "name", &line_starts);
            let (description_line, description_col) =
                frontmatter_key_line_col(&parts, "description", &line_starts);
            let (compat_line, compat_col) =
                frontmatter_key_line_col(&parts, "compatibility", &line_starts);
            let (allowed_tools_line, allowed_tools_col) =
                frontmatter_key_line_col(&parts, "allowed-tools", &line_starts);
            let (model_line, model_col) = frontmatter_key_line_col(&parts, "model", &line_starts);
            let (context_line, context_col) =
                frontmatter_key_line_col(&parts, "context", &line_starts);
            // AS-002: Missing name field
            if config.is_rule_enabled("AS-002") && frontmatter.name.is_none() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        name_line,
                        name_col,
                        "AS-002",
                        "Skill frontmatter is missing required 'name' field".to_string(),
                    )
                    .with_suggestion("Add 'name: your-skill-name' to frontmatter".to_string()),
                );
            }

            // AS-003: Missing description field
            if config.is_rule_enabled("AS-003") && frontmatter.description.is_none() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        description_line,
                        description_col,
                        "AS-003",
                        "Skill frontmatter is missing required 'description' field".to_string(),
                    )
                    .with_suggestion("Add 'description: Use when...' to frontmatter".to_string()),
                );
            }

            if let Some(name) = frontmatter.name.as_deref() {
                let name_trimmed = name.trim();

                // AS-004: Invalid name format
                if config.is_rule_enabled("AS-004") {
                    let name_re = NAME_FORMAT_REGEX
                        .get_or_init(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap());
                    if name_trimmed.len() > 64 || !name_re.is_match(name_trimmed) {
                        let fixed_name = convert_to_kebab_case(name_trimmed);
                        let mut diagnostic = Diagnostic::error(
                            path.to_path_buf(),
                            name_line,
                            name_col,
                            "AS-004",
                            format!(
                                "Name '{}' must be 1-64 characters of lowercase letters, digits, and hyphens",
                                name_trimmed
                            ),
                        )
                        .with_suggestion(
                            "Lowercase and trim the name, replace spaces and '_' with '-', collapse multiple '-' into one, remove invalid characters, and truncate to 64 characters".to_string(),
                        );

                        // Add auto-fix if we can find the byte range and the fixed name is valid
                        if !fixed_name.is_empty() && name_re.is_match(&fixed_name) {
                            if let Some((start, end)) =
                                frontmatter_value_byte_range(content, &parts, "name")
                            {
                                // Determine if fix is safe: only case changes are safe
                                // A fix is case-only if:
                                // 1. No underscores in original (would be converted to hyphens)
                                // 2. No spaces in original (would be converted to hyphens)
                                // 3. No invalid chars removed
                                // 4. Only lowercasing was needed
                                let has_structural_changes = name_trimmed.contains('_')
                                    || name_trimmed.contains(' ')
                                    || name_trimmed
                                        .chars()
                                        .any(|c| !c.is_ascii_alphanumeric() && c != '-');
                                let is_case_only = !has_structural_changes
                                    && name_trimmed.to_lowercase() == fixed_name;
                                let fix = Fix::replace(
                                    start,
                                    end,
                                    &fixed_name,
                                    format!("Convert name to kebab-case: '{}'", fixed_name),
                                    is_case_only,
                                );
                                diagnostic = diagnostic.with_fix(fix);
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }

                // AS-005: Name cannot start or end with hyphen
                if config.is_rule_enabled("AS-005")
                    && (name_trimmed.starts_with('-') || name_trimmed.ends_with('-'))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            name_line,
                            name_col,
                            "AS-005",
                            format!("Name '{}' cannot start or end with hyphen", name_trimmed),
                        )
                        .with_suggestion(
                            "Remove leading/trailing hyphens from the name".to_string(),
                        ),
                    );
                }

                // AS-006: Name cannot contain consecutive hyphens
                if config.is_rule_enabled("AS-006") && name_trimmed.contains("--") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            name_line,
                            name_col,
                            "AS-006",
                            format!("Name '{}' cannot contain consecutive hyphens", name_trimmed),
                        )
                        .with_suggestion("Replace '--' with '-' in the name".to_string()),
                    );
                }

                // AS-007: Reserved name
                if config.is_rule_enabled("AS-007") && !name_trimmed.is_empty() {
                    let reserved = ["anthropic", "claude", "skill"];
                    if reserved.contains(&name_trimmed.to_lowercase().as_str()) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                name_line,
                                name_col,
                                "AS-007",
                                format!("Name '{}' is reserved and cannot be used", name_trimmed),
                            )
                            .with_suggestion("Choose a different skill name".to_string()),
                        );
                    }
                }
            }

            if let Some(description) = frontmatter.description.as_deref() {
                let description_trimmed = description.trim();

                // AS-008: Description length
                if config.is_rule_enabled("AS-008") {
                    let len = description_trimmed.len();
                    if !(1..=1024).contains(&len) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                description_line,
                                description_col,
                                "AS-008",
                                format!("Description must be 1-1024 characters, got {}", len),
                            )
                            .with_suggestion(
                                "Trim the description to 1024 characters or fewer".to_string(),
                            ),
                        );
                    }
                }

                // AS-009: Description contains XML tags
                if config.is_rule_enabled("AS-009") {
                    let xml_re =
                        DESCRIPTION_XML_REGEX.get_or_init(|| Regex::new(r"<[^>]+>").unwrap());
                    if xml_re.is_match(description) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                description_line,
                                description_col,
                                "AS-009",
                                "Description must not contain XML tags".to_string(),
                            )
                            .with_suggestion("Remove XML tags from the description".to_string()),
                        );
                    }
                }

                // AS-010: Description should include trigger phrase
                if config.is_rule_enabled("AS-010") && !description_trimmed.is_empty() {
                    let desc_lower = description_trimmed.to_lowercase();
                    if !desc_lower.contains("use when") {
                        let mut diagnostic = Diagnostic::warning(
                            path.to_path_buf(),
                            description_line,
                            description_col,
                            "AS-010",
                            "Description should include a 'Use when...' trigger phrase"
                                .to_string(),
                        )
                        .with_suggestion(
                            "Add 'Use when [condition]' to help Claude understand when to invoke this skill".to_string(),
                        );

                        // Add auto-fix: prepend "Use when user wants to " to description
                        if let Some((start, end)) =
                            frontmatter_value_byte_range(content, &parts, "description")
                        {
                            let new_description =
                                format!("Use when user wants to {}", description_trimmed);
                            // Check if the new description would exceed length limit
                            if new_description.len() <= 1024 {
                                let fix = Fix::replace(
                                    start,
                                    end,
                                    &new_description,
                                    "Prepend 'Use when user wants to ' to description",
                                    false, // Not safe - changes semantics
                                );
                                diagnostic = diagnostic.with_fix(fix);
                            }
                        }

                        diagnostics.push(diagnostic);
                    }
                }
            }

            // AS-011: Compatibility length
            if config.is_rule_enabled("AS-011") {
                if let Some(compat) = frontmatter.compatibility.as_deref() {
                    let len = compat.trim().len();
                    if len == 0 || len > 500 {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                compat_line,
                                compat_col,
                                "AS-011",
                                format!("Compatibility must be 1-500 characters, got {}", len),
                            )
                            .with_suggestion(
                                "Trim compatibility to 500 characters or fewer".to_string(),
                            ),
                        );
                    }
                }
            }
            let (agent_line, agent_col) = frontmatter_key_line_col(&parts, "agent", &line_starts);

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

                    // CC-SK-006: Dangerous auto-invocation check
                    if config.is_rule_enabled("CC-SK-006") {
                        const DANGEROUS_NAMES: &[&str] =
                            &["deploy", "ship", "publish", "delete", "release", "push"];
                        let name_lower = name_trimmed.to_lowercase();
                        if DANGEROUS_NAMES.iter().any(|d| name_lower.contains(d))
                            && !frontmatter.disable_model_invocation.unwrap_or(false)
                        {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    name_line,
                                    name_col,
                                    "CC-SK-006",
                                    format!(
                                        "Dangerous skill '{}' must set 'disable-model-invocation: true' to prevent accidental invocation",
                                        name_trimmed
                                    ),
                                )
                                .with_suggestion(
                                    "Add 'disable-model-invocation: true' to the frontmatter"
                                        .to_string(),
                                ),
                            );
                        }
                    }

                    // Parse allowed_tools once for CC-SK-007 and CC-SK-008
                    let tool_list: Option<Vec<&str>> = schema
                        .allowed_tools
                        .as_ref()
                        .map(|tools| tools.split_whitespace().collect());

                    // CC-SK-007: Unrestricted Bash warning
                    if config.is_rule_enabled("CC-SK-007") {
                        if let Some(ref tools) = tool_list {
                            // Find all plain Bash occurrences in the allowed-tools line only
                            // to avoid matching "Bash" in other fields like description
                            let search_start =
                                frontmatter_key_offset(&parts.frontmatter, "allowed-tools")
                                    .map(|offset| parts.frontmatter_start + offset)
                                    .unwrap_or(parts.frontmatter_start);
                            let bash_positions = find_plain_bash_positions(content, search_start);

                            let mut bash_pos_iter = bash_positions.iter();

                            for tool in tools {
                                if *tool == "Bash" {
                                    let mut diagnostic = Diagnostic::warning(
                                        path.to_path_buf(),
                                        allowed_tools_line,
                                        allowed_tools_col,
                                        "CC-SK-007",
                                        "Unrestricted Bash access detected. Consider using scoped version for better security.".to_string(),
                                    )
                                    .with_suggestion("Use scoped Bash like 'Bash(git:*)' or 'Bash(npm:*)' instead of plain 'Bash'".to_string());

                                    // Try to attach a fix for each plain Bash
                                    if let Some(&(start, end)) = bash_pos_iter.next() {
                                        // Default replacement: Bash(git:*) as a common use case
                                        // safe=false because we don't know what scope the user wants
                                        let fix = Fix::replace(
                                            start,
                                            end,
                                            "Bash(git:*)",
                                            "Replace unrestricted Bash with scoped Bash(git:*)",
                                            false,
                                        );
                                        diagnostic = diagnostic.with_fix(fix);
                                    }

                                    diagnostics.push(diagnostic);
                                }
                            }
                        }
                    }

                    // CC-SK-001: Invalid model value
                    if config.is_rule_enabled("CC-SK-001") {
                        if let Some(model) = &schema.model {
                            if !VALID_MODELS.contains(&model.as_str()) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        model_line,
                                        model_col,
                                        "CC-SK-001",
                                        format!(
                                            "Invalid model '{}'. Must be one of: {}",
                                            model,
                                            VALID_MODELS.join(", ")
                                        ),
                                    )
                                    .with_suggestion(format!(
                                        "Use one of the valid model values: {}",
                                        VALID_MODELS.join(", ")
                                    )),
                                );
                            }
                        }
                    }

                    // CC-SK-002: Invalid context value
                    if config.is_rule_enabled("CC-SK-002") {
                        if let Some(context) = &schema.context {
                            if context != "fork" {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        context_line,
                                        context_col,
                                        "CC-SK-002",
                                        format!(
                                            "Invalid context '{}'. Must be 'fork' or omitted",
                                            context
                                        ),
                                    )
                                    .with_suggestion(
                                        "Set context to 'fork' or remove the field entirely"
                                            .to_string(),
                                    ),
                                );
                            }
                        }
                    }

                    // CC-SK-003: Context without agent
                    if config.is_rule_enabled("CC-SK-003")
                        && schema.context.as_deref() == Some("fork")
                        && schema.agent.is_none()
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                context_line,
                                context_col,
                                "CC-SK-003",
                                "Context 'fork' requires an 'agent' field".to_string(),
                            )
                            .with_suggestion(
                                "Add 'agent: general-purpose' or another valid agent type"
                                    .to_string(),
                            ),
                        );
                    }

                    // CC-SK-004: Agent without context
                    if config.is_rule_enabled("CC-SK-004")
                        && schema.agent.is_some()
                        && schema.context.as_deref() != Some("fork")
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                agent_line,
                                agent_col,
                                "CC-SK-004",
                                "Agent field requires 'context: fork'".to_string(),
                            )
                            .with_suggestion("Add 'context: fork' to the frontmatter".to_string()),
                        );
                    }

                    // CC-SK-005: Invalid agent type
                    if config.is_rule_enabled("CC-SK-005") {
                        if let Some(agent) = &schema.agent {
                            if !is_valid_agent(agent) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        agent_line,
                                        agent_col,
                                        "CC-SK-005",
                                        format!(
                                            "Invalid agent type '{}'. Must be Explore, Plan, general-purpose, or a custom kebab-case name (1-64 chars)",
                                            agent
                                        ),
                                    )
                                    .with_suggestion(
                                        "Use a built-in agent (Explore, Plan, general-purpose) or a custom kebab-case name".to_string()
                                    ),
                                );
                            }
                        }
                    }

                    // CC-SK-008: Unknown tool name
                    if config.is_rule_enabled("CC-SK-008") {
                        if let Some(ref tools) = tool_list {
                            for tool in tools {
                                let base_name = tool.split('(').next().unwrap_or(tool);
                                if !KNOWN_TOOLS.contains(&base_name) {
                                    diagnostics.push(
                                        Diagnostic::error(
                                            path.to_path_buf(),
                                            allowed_tools_line,
                                            allowed_tools_col,
                                            "CC-SK-008",
                                            format!(
                                                "Unknown tool '{}'. Known tools: {}",
                                                base_name,
                                                KNOWN_TOOLS.join(", ")
                                            ),
                                        )
                                        .with_suggestion(
                                            format!(
                                                "Use one of the known Claude Code tools: {}",
                                                KNOWN_TOOLS.join(", ")
                                            ),
                                        ),
                                    );
                                }
                            }
                        }
                    }

                    // CC-SK-009: Too many injections (warning)
                    // Count across full content (frontmatter + body) per VALIDATION-RULES.md
                    if config.is_rule_enabled("CC-SK-009") {
                        let injection_count = content.matches("!`").count();
                        if injection_count > MAX_INJECTIONS {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    frontmatter_line,
                                    frontmatter_col,
                                    "CC-SK-009",
                                    format!(
                                        "Too many dynamic injections ({}). Limit to {} for better performance",
                                        injection_count, MAX_INJECTIONS
                                    ),
                                )
                                .with_suggestion(
                                    "Consider moving complex logic to a scripts/ directory or reducing injections".to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        // AS-012: Content exceeds 500 lines
        if config.is_rule_enabled("AS-012") {
            let line_count = body_raw.lines().count();
            if line_count > 500 {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        body_line,
                        body_col,
                        "AS-012",
                        format!("Skill content exceeds 500 lines (got {})", line_count),
                    )
                    .with_suggestion("Move extra content into references/".to_string()),
                );
            }
        }

        // AS-013: File reference too deep
        if config.is_rule_enabled("AS-013") {
            let paths = extract_reference_paths(body_raw);
            for ref_path in paths {
                if reference_path_too_deep(&ref_path.path) {
                    let (line, col) = line_col_at(parts.body_start + ref_path.start, &line_starts);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "AS-013",
                            format!(
                                "File reference '{}' is deeper than one level",
                                ref_path.path
                            ),
                        )
                        .with_suggestion("Flatten the references/ directory structure".to_string()),
                    );
                }
            }
        }

        // AS-014: Windows path separator
        if config.is_rule_enabled("AS-014") {
            let paths = extract_windows_paths(body_raw);
            for win_path in paths {
                let (line, col) = line_col_at(parts.body_start + win_path.start, &line_starts);
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "AS-014",
                        format!(
                            "Windows path separator detected in '{}'; use forward slashes",
                            win_path.path
                        ),
                    )
                    .with_suggestion("Replace '\\\\' with '/' in file paths".to_string()),
                );
            }
        }

        // AS-015: Directory size exceeds 8MB
        if config.is_rule_enabled("AS-015") && path.is_file() {
            if let Some(dir) = path.parent() {
                let size = directory_size(dir);
                const MAX_BYTES: u64 = 8 * 1024 * 1024;
                if size > MAX_BYTES {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            frontmatter_line,
                            frontmatter_col,
                            "AS-015",
                            format!("Skill directory exceeds 8MB ({} bytes)", size),
                        )
                        .with_suggestion(
                            "Remove large assets or split the skill into smaller parts".to_string(),
                        ),
                    );
                }
            }
        }

        diagnostics
    }
}

fn parse_frontmatter_fields(frontmatter: &str) -> Result<SkillFrontmatter, serde_yaml::Error> {
    if frontmatter.trim().is_empty() {
        return Ok(SkillFrontmatter::default());
    }
    serde_yaml::from_str(frontmatter)
}

fn extract_reference_paths(body: &str) -> Vec<PathMatch> {
    let re = REFERENCE_PATH_REGEX
        .get_or_init(|| Regex::new("(?i)\\b(?:references?|refs)[/\\\\][^\\s)\\]}>\"']+").unwrap());
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for m in re.find_iter(body) {
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    paths
}

fn extract_windows_paths(body: &str) -> Vec<PathMatch> {
    let re = WINDOWS_PATH_REGEX
        .get_or_init(|| Regex::new(r"(?i)\b(?:[a-z]:)?[a-z0-9._-]+(?:\\[a-z0-9._-]+)+\b").unwrap());
    let token_re = WINDOWS_PATH_TOKEN_REGEX.get_or_init(|| Regex::new(r"[^\s]+\\[^\s]+").unwrap());
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for m in re.find_iter(body) {
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    for m in token_re.find_iter(body) {
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    paths
}

fn reference_path_too_deep(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut parts = normalized.split('/').filter(|part| !part.is_empty());
    let Some(prefix) = parts.next() else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("references")
        && !prefix.eq_ignore_ascii_case("reference")
        && !prefix.eq_ignore_ascii_case("refs")
    {
        return false;
    }
    parts.count() > 1
}

fn trim_path_token(token: &str) -> &str {
    token
        .trim_start_matches(['(', '[', '{', '<', '"', '\''])
        .trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '>', '"', '\''])
}

fn trim_path_token_with_offset(token: &str) -> Option<(String, usize)> {
    let trimmed = trim_path_token(token);
    if trimmed.is_empty() {
        return None;
    }
    let offset = token.find(trimmed).unwrap_or(0);
    Some((trimmed.to_string(), offset))
}

fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut low = 0usize;
    let mut high = line_starts.len();
    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    let line_start = line_starts[low];
    (low + 1, offset - line_start + 1)
}

fn frontmatter_key_line_col(
    parts: &FrontmatterParts,
    key: &str,
    line_starts: &[usize],
) -> (usize, usize) {
    let offset = frontmatter_key_offset(&parts.frontmatter, key)
        .map(|local| parts.frontmatter_start + local)
        .unwrap_or(parts.frontmatter_start);
    line_col_at(offset, line_starts)
}

fn frontmatter_key_offset(frontmatter: &str, key: &str) -> Option<usize> {
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            // Check for CRLF or LF
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.trim_start().starts_with(':') {
                let column = line.len() - trimmed.len();
                return Some(offset + column);
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

/// Find the byte range of a YAML value for a given key in frontmatter.
/// Returns (start, end) byte offsets relative to the full content.
/// Handles both quoted and unquoted values.
fn frontmatter_value_byte_range(
    _content: &str,
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    let frontmatter = &parts.frontmatter;
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(key) {
            if let Some(after_colon) = rest.trim_start().strip_prefix(':') {
                // Found the key, now find the value
                let leading_ws = line.len() - trimmed.len();
                let ws_after_key = rest.len() - rest.trim_start().len();
                let key_end = leading_ws + key.len() + ws_after_key + 1; // +1 for ':'

                let value_str = after_colon.trim_start();
                if value_str.is_empty() {
                    // No value on this line (might be multiline YAML)
                    return None;
                }

                // Calculate value start position in line
                let value_offset_in_line = key_end + (after_colon.len() - value_str.len());

                // Handle quoted values
                let (value_start, value_len) = if let Some(inner) = value_str.strip_prefix('"') {
                    // Double-quoted: find closing quote
                    if let Some(end_quote) = inner.find('"') {
                        (value_offset_in_line + 1, end_quote) // Skip opening quote
                    } else {
                        // Unclosed quote - return None for malformed YAML
                        return None;
                    }
                } else if let Some(inner) = value_str.strip_prefix('\'') {
                    // Single-quoted: find closing quote
                    if let Some(end_quote) = inner.find('\'') {
                        (value_offset_in_line + 1, end_quote) // Skip opening quote
                    } else {
                        // Unclosed quote - return None for malformed YAML
                        return None;
                    }
                } else {
                    // Unquoted value: take until end of line or comment
                    // Check for both " #" (space-hash) and "\t#" (tab-hash)
                    let value_end = value_str
                        .find(" #")
                        .or_else(|| value_str.find("\t#"))
                        .unwrap_or(value_str.len());
                    (value_offset_in_line, value_end)
                };

                let abs_start = parts.frontmatter_start + offset + value_start;
                let abs_end = abs_start + value_len;

                return Some((abs_start, abs_end));
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

fn directory_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if file_type.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total = total.saturating_add(metadata.len());
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    #[test]
    fn test_valid_skill() {
        let content = r#"---
name: test-skill
description: Use when testing skill validation
---
Skill body content"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_invalid_skill_name() {
        let content = r#"---
name: Test-Skill
description: Use when validating skill names
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004_errors.len(), 1);
    }

    #[test]
    fn test_as_001_missing_frontmatter() {
        let content =
            include_str!("../../../../tests/fixtures/skills/missing-frontmatter/SKILL.md");

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let as_001_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
        assert_eq!(as_001_errors.len(), 1);
    }

    #[test]
    fn test_as_002_missing_name() {
        let content = r#"---
description: Use when validating missing name
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_002_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-002").collect();
        assert_eq!(as_002_errors.len(), 1);
    }

    #[test]
    fn test_as_003_missing_description() {
        let content = r#"---
name: test-skill
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
        assert_eq!(as_003_errors.len(), 1);
    }

    #[test]
    fn test_as_004_invalid_name_format() {
        let content = r#"---
name: bad_name
description: Use when validating name format
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004_errors.len(), 1);
    }

    #[test]
    fn test_as_007_reserved_name() {
        let content = r#"---
name: claude
description: Use when validating reserved names
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_007_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-007").collect();
        assert_eq!(as_007_errors.len(), 1);
    }

    #[test]
    fn test_as_008_description_too_long() {
        let long_description = "a".repeat(1025);
        let content = format!(
            "---\nname: test-skill\ndescription: {}\n---\nBody",
            long_description
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
        assert_eq!(as_008_errors.len(), 1);
    }

    #[test]
    fn test_as_008_description_empty_string() {
        let content = r#"---
name: test-skill
description: ""
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
        assert_eq!(as_003_errors.len(), 0);

        let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
        assert_eq!(as_008_errors.len(), 1);
    }

    #[test]
    fn test_as_009_description_contains_xml() {
        let content = r#"---
name: test-skill
description: Use when validating <xml> tags
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_009_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-009").collect();
        assert_eq!(as_009_errors.len(), 1);
    }

    #[test]
    fn test_as_011_compatibility_too_long() {
        let long_compat = "b".repeat(501);
        let content = format!(
            "---\nname: test-skill\ndescription: Use when validating compatibility\ncompatibility: {}\n---\nBody",
            long_compat
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_011_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-011").collect();
        assert_eq!(as_011_errors.len(), 1);
    }

    #[test]
    fn test_as_012_content_too_long() {
        let body = (0..501).map(|_| "line").collect::<Vec<_>>().join("\n");
        let content = format!(
            "---\nname: test-skill\ndescription: Use when validating content length\n---\n{}",
            body
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_012_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-012").collect();
        assert_eq!(as_012_warnings.len(), 1);
    }

    #[test]
    fn test_as_013_reference_too_deep() {
        let content = include_str!("../../../../tests/fixtures/skills/deep-reference/SKILL.md");

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
        assert_eq!(as_013_errors.len(), 1);
    }

    #[test]
    fn test_as_013_reference_single_name_too_deep() {
        let content = r#"---
name: deep-reference
description: Use when validating deep references
---

See reference/deep/guide.md for details."#;

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
        assert_eq!(as_013_errors.len(), 1);
    }

    #[test]
    fn test_as_014_windows_path_separator() {
        let content = include_str!("../../../../tests/fixtures/skills/windows-path/SKILL.md");

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let as_014_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-014").collect();
        assert_eq!(as_014_errors.len(), 1);
    }

    #[test]
    fn test_as_015_directory_size_exceeds() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().unwrap();
        let skill_dir = temp_dir.path().join("big-skill");
        fs::create_dir_all(&skill_dir).unwrap();

        let skill_path = skill_dir.join("SKILL.md");
        let mut skill_file = fs::File::create(&skill_path).unwrap();
        writeln!(
            skill_file,
            "---\nname: big-skill\ndescription: Use when validating directory size\n---\nBody"
        )
        .unwrap();

        let big_file_path = skill_dir.join("big.bin");
        let big_payload = vec![0u8; 8 * 1024 * 1024 + 1];
        fs::write(&big_file_path, big_payload).unwrap();

        let content = fs::read_to_string(&skill_path).unwrap();
        let validator = SkillValidator;
        let diagnostics = validator.validate(&skill_path, &content, &LintConfig::default());

        let as_015_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-015").collect();
        assert_eq!(as_015_errors.len(), 1);
    }

    #[test]
    fn test_cc_sk_006_dangerous_name_without_safety() {
        let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(cc_sk_006_errors.len(), 1);
        assert_eq!(
            cc_sk_006_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_006_dangerous_name_with_safety() {
        let content = r#"---
name: deploy-prod
description: Deploys to production
disable-model-invocation: true
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(cc_sk_006_errors.len(), 0);
    }

    #[test]
    fn test_cc_sk_006_covers_all_dangerous_names() {
        let dangerous_names = vec!["deploy", "ship", "publish", "delete", "release", "push"];

        for name in dangerous_names {
            let content = format!(
                r#"---
name: {}-prod
description: A dangerous skill
---
Body"#,
                name
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            // Should have an error for CC-SK-006
            let cc_sk_006_errors: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-006")
                .collect();

            assert_eq!(
                cc_sk_006_errors.len(),
                1,
                "Expected CC-SK-006 error for name: {}",
                name
            );
        }
    }

    #[test]
    fn test_cc_sk_007_unrestricted_bash() {
        let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should have a warning for CC-SK-007
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 1);
        assert_eq!(
            cc_sk_007_warnings[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_cc_sk_007_scoped_bash_ok() {
        let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash(git:*) Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have a warning for CC-SK-007 (scoped Bash is ok)
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 0);
    }

    #[test]
    fn test_cc_sk_007_no_bash() {
        let content = r#"---
name: reader
description: File reader
allowed-tools: Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Should NOT have a warning for CC-SK-007 (no Bash at all)
        let cc_sk_007_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007_warnings.len(), 0);
    }

    // ===== CC-SK-007 Auto-fix Tests =====

    #[test]
    fn test_cc_sk_007_has_fix() {
        let content = r#"---
name: git-helper
description: Use when doing git operations
allowed-tools: Bash Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007.len(), 1);
        assert!(cc_sk_007[0].has_fixes());

        let fix = &cc_sk_007[0].fixes[0];
        assert_eq!(fix.replacement, "Bash(git:*)");
        assert!(!fix.safe); // Not safe, we don't know user's intended scope
    }

    #[test]
    fn test_cc_sk_007_fix_correct_byte_position() {
        let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        assert_eq!(cc_sk_007.len(), 1);
        assert!(cc_sk_007[0].has_fixes());

        let fix = &cc_sk_007[0].fixes[0];

        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("Bash(git:*)"));
        assert!(!fixed.contains("allowed-tools: Bash "));
    }

    #[test]
    fn test_cc_sk_007_multiple_bash_multiple_fixes() {
        let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read Bash
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        // Each Bash occurrence generates a warning
        assert_eq!(cc_sk_007.len(), 2);
        // Each should have a fix
        assert!(cc_sk_007[0].has_fixes());
        assert!(cc_sk_007[1].has_fixes());
    }

    #[test]
    fn test_cc_sk_007_scoped_bash_no_fix() {
        let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash(git:*) Read
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        // Scoped Bash doesn't trigger the warning
        assert_eq!(cc_sk_007.len(), 0);
    }

    #[test]
    fn test_find_plain_bash_positions() {
        let content = "allowed-tools: Bash Read Bash(git:*) Write Bash";
        let positions = find_plain_bash_positions(content, 0);

        // Should find 2: "Bash" at position 15 and "Bash" at position 43
        // But NOT "Bash(git:*)"
        assert_eq!(positions.len(), 2);
        assert_eq!(&content[positions[0].0..positions[0].1], "Bash");
        assert_eq!(&content[positions[1].0..positions[1].1], "Bash");
    }

    #[test]
    fn test_find_plain_bash_positions_none() {
        let content = "allowed-tools: Bash(git:*) Bash(npm:*) Read";
        let positions = find_plain_bash_positions(content, 0);
        assert_eq!(positions.len(), 0);
    }

    #[test]
    fn test_as_005_leading_hyphen() {
        let content = r#"---
name: -bad-name
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

        assert_eq!(as_005_errors.len(), 1);
        assert_eq!(
            as_005_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_005_trailing_hyphen() {
        let content = r#"---
name: bad-name-
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

        assert_eq!(as_005_errors.len(), 1);
        assert_eq!(
            as_005_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_006_consecutive_hyphens() {
        let content = r#"---
name: bad--name
description: Use when testing validation
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_006_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-006").collect();

        assert_eq!(as_006_errors.len(), 1);
        assert_eq!(
            as_006_errors[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_as_010_missing_trigger() {
        let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 1);
        assert_eq!(
            as_010_warnings[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_as_010_has_use_when_trigger() {
        let content = r#"---
name: code-review
description: Use when user asks for code review
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 0);
    }

    #[test]
    fn test_as_010_use_this_not_accepted() {
        let content = r#"---
name: code-review
description: Use this skill to review code
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(as_010_warnings.len(), 1);
    }

    // ===== CC-SK-001: Invalid Model Value =====

    #[test]
    fn test_cc_sk_001_invalid_model() {
        let content = r#"---
name: test-skill
description: Use when testing
model: gpt-4
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 1);
        assert_eq!(
            cc_sk_001[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_001[0].message.contains("gpt-4"));
    }

    #[test]
    fn test_cc_sk_001_valid_models() {
        for model in &["sonnet", "opus", "haiku", "inherit"] {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
model: {}
---
Body"#,
                model
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_001: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-001")
                .collect();

            assert_eq!(cc_sk_001.len(), 0, "Model '{}' should be valid", model);
        }
    }

    #[test]
    fn test_cc_sk_001_no_model_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 0);
    }

    // ===== CC-SK-002: Invalid Context Value =====

    #[test]
    fn test_cc_sk_002_invalid_context() {
        let content = r#"---
name: test-skill
description: Use when testing
context: split
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 1);
        assert_eq!(
            cc_sk_002[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_002[0].message.contains("split"));
    }

    #[test]
    fn test_cc_sk_002_valid_context_fork() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: general-purpose
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 0);
    }

    #[test]
    fn test_cc_sk_002_no_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_002: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-002")
            .collect();

        assert_eq!(cc_sk_002.len(), 0);
    }

    // ===== CC-SK-003: Context Without Agent =====

    #[test]
    fn test_cc_sk_003_context_fork_without_agent() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-003")
            .collect();

        assert_eq!(cc_sk_003.len(), 1);
        assert_eq!(
            cc_sk_003[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_003_context_fork_with_agent_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-003")
            .collect();

        assert_eq!(cc_sk_003.len(), 0);
    }

    // ===== CC-SK-004: Agent Without Context =====

    #[test]
    fn test_cc_sk_004_agent_without_context() {
        let content = r#"---
name: test-skill
description: Use when testing
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 1);
        assert_eq!(
            cc_sk_004[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
    }

    #[test]
    fn test_cc_sk_004_agent_with_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 0);
    }

    #[test]
    fn test_cc_sk_004_no_agent_no_context_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-004")
            .collect();

        assert_eq!(cc_sk_004.len(), 0);
    }

    // ===== CC-SK-005: Invalid Agent Type =====

    #[test]
    fn test_cc_sk_005_invalid_agent() {
        let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: CustomAgent
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 1);
        assert_eq!(
            cc_sk_005[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_005[0].message.contains("CustomAgent"));
    }

    #[test]
    fn test_cc_sk_005_valid_agents() {
        for agent in &["Explore", "Plan", "general-purpose"] {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
                agent
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_005: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-005")
                .collect();

            assert_eq!(cc_sk_005.len(), 0, "Agent '{}' should be valid", agent);
        }
    }

    #[test]
    fn test_cc_sk_005_valid_custom_agents() {
        // Custom agents in kebab-case should be valid
        for agent in &[
            "my-custom-agent",
            "code-review",
            "deploy-helper",
            "a",
            "agent123",
            "my-agent-v2",
        ] {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
                agent
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_005: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-005")
                .collect();

            assert_eq!(
                cc_sk_005.len(),
                0,
                "Custom agent '{}' should be valid",
                agent
            );
        }
    }

    #[test]
    fn test_cc_sk_005_rejects_invalid_agent_formats() {
        // Consolidated test for all invalid agent formats
        let invalid_agents = [
            ("MyAgent", "uppercase"),
            ("my_custom_agent", "underscore"),
            ("\"\"", "empty"),
            ("-custom-agent", "leading hyphen"),
            ("custom-agent-", "trailing hyphen"),
            ("custom--agent", "consecutive hyphens"),
            ("my@agent", "special char @"),
            ("agent!", "special char !"),
            ("test.agent", "special char ."),
            ("agent/name", "special char /"),
        ];

        for (agent, reason) in invalid_agents {
            let content = format!(
                r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
                agent
            );

            let validator = SkillValidator;
            let diagnostics =
                validator.validate(Path::new("test.md"), &content, &LintConfig::default());

            let cc_sk_005: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-005")
                .collect();

            assert_eq!(
                cc_sk_005.len(),
                1,
                "Agent '{}' ({}) should be rejected",
                agent,
                reason
            );
        }
    }

    #[test]
    fn test_cc_sk_005_rejects_too_long_agent() {
        let long_agent = "a".repeat(65);
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            long_agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 1, "Agent over 64 chars should be rejected");
    }

    #[test]
    fn test_cc_sk_005_accepts_max_length_agent() {
        let max_agent = "a".repeat(64);
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            max_agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 0, "Agent at 64 chars should be accepted");
    }

    #[test]
    fn test_cc_sk_005_fixture_invalid_agent() {
        let content =
            include_str!("../../../../tests/fixtures/invalid/skills/invalid-agent/SKILL.md");

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            1,
            "Invalid agent fixture should trigger CC-SK-005"
        );
    }

    #[test]
    fn test_cc_sk_005_fixture_valid_custom_agent() {
        let content =
            include_str!("../../../../tests/fixtures/valid/skills/with-custom-agent/SKILL.md");

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            0,
            "Valid custom agent fixture should pass CC-SK-005"
        );
    }

    // ===== CC-SK-008: Unknown Tool Name =====

    #[test]
    fn test_cc_sk_008_unknown_tool() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read Write UnknownTool
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 1);
        assert_eq!(
            cc_sk_008[0].level,
            crate::diagnostics::DiagnosticLevel::Error
        );
        assert!(cc_sk_008[0].message.contains("UnknownTool"));
    }

    #[test]
    fn test_cc_sk_008_all_known_tools_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Write Edit Grep Glob Task
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 0);
    }

    #[test]
    fn test_cc_sk_008_scoped_tool_extracts_base_name() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash(git:*) Read Write
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 0);
    }

    #[test]
    fn test_cc_sk_008_multiple_unknown_tools() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool1 Read FakeTool2
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(cc_sk_008.len(), 2);
    }

    #[test]
    fn test_cc_sk_008_scoped_unknown_tool() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool(scope:*) Read
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(
            cc_sk_008.len(),
            1,
            "Should detect FakeTool as unknown even when scoped"
        );
        assert!(cc_sk_008[0].message.contains("FakeTool"));
    }

    // ===== CC-SK-009: Too Many Injections =====

    #[test]
    fn test_cc_sk_009_too_many_injections() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Current date: !`date`
Git status: !`git status`
Branch: !`git branch`
User: !`whoami`
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 1);
        assert_eq!(
            cc_sk_009[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
        assert!(cc_sk_009[0].message.contains("4"));
    }

    #[test]
    fn test_cc_sk_009_exactly_three_injections_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
Date: !`date`
Status: !`git status`
Branch: !`git branch`
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 0);
    }

    #[test]
    fn test_cc_sk_009_no_injections_ok() {
        let content = r#"---
name: test-skill
description: Use when testing
---
No dynamic injections here.
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_009: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-009")
            .collect();

        assert_eq!(cc_sk_009.len(), 0);
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_cc_sk_006_explicit_false_still_triggers() {
        let content = r#"---
name: deploy-prod
description: Use when deploying
disable-model-invocation: false
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(
            cc_sk_006.len(),
            1,
            "Explicit false should still trigger CC-SK-006"
        );
    }

    #[test]
    fn test_cc_sk_007_duplicate_bash_multiple_warnings() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Bash
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-007")
            .collect();

        // Each plain "Bash" triggers warning (2 occurrences = 2 warnings)
        assert_eq!(
            cc_sk_007.len(),
            2,
            "Each Bash occurrence triggers a warning"
        );
    }

    #[test]
    fn test_cc_sk_008_malformed_scope_no_panic() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash( Read Bash() Write
---
Body"#;

        let validator = SkillValidator;
        // Should not panic on malformed scope syntax
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        // Bash( extracts "Bash", which is known
        // Bash() extracts "Bash", which is known
        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        assert_eq!(
            cc_sk_008.len(),
            0,
            "Malformed scopes should extract base name correctly"
        );
    }

    #[test]
    fn test_cc_sk_008_lowercase_tool_unknown() {
        let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: bash read
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let cc_sk_008: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect();

        // Tool names are case-sensitive: bash != Bash
        assert_eq!(cc_sk_008.len(), 2, "lowercase tool names are unknown");
    }

    #[test]
    fn test_as_010_case_insensitive() {
        let content = r#"---
name: test-skill
description: USE WHEN testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();

        assert_eq!(
            as_010.len(),
            0,
            "'USE WHEN' should match case-insensitively"
        );
    }

    #[test]
    fn test_parse_error_handling() {
        let content = r#"---
name: test
description
invalid yaml
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();

        assert_eq!(
            parse_errors.len(),
            1,
            "Invalid YAML should produce parse error"
        );
    }

    // ===== Config Wiring Tests =====

    #[test]
    fn test_config_disabled_skills_category() {
        let mut config = LintConfig::default();
        config.rules.skills = false;

        let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // AS-005 and AS-010 should not fire when skills category is disabled
        let skill_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("AS-") || d.rule.starts_with("CC-SK-"))
            .collect();
        assert_eq!(skill_rules.len(), 0);
    }

    #[test]
    fn test_config_disabled_specific_skill_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["AS-005".to_string()];

        let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // AS-005 should not fire when specifically disabled
        let as_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();
        assert_eq!(as_005.len(), 0);

        // But AS-010 should still fire
        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
    }

    #[test]
    fn test_config_cursor_target_disables_cc_sk_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;

        let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // CC-SK-006 should not fire for Cursor target
        let cc_sk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();
        assert_eq!(cc_sk_006.len(), 0);

        // But AS-010 should still fire (it's not CC- prefix)
        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
    }

    #[test]
    fn test_config_claude_code_target_enables_cc_sk_rules() {
        use crate::config::TargetTool;

        let mut config = LintConfig::default();
        config.target = TargetTool::ClaudeCode;

        let content = r#"---
name: deploy-prod
description: Use when deploying to production
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &config);

        // CC-SK-006 should fire for ClaudeCode target
        let cc_sk_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();
        assert_eq!(cc_sk_006.len(), 1);
    }

    // ===== convert_to_kebab_case Tests =====

    #[test]
    fn test_convert_to_kebab_case_lowercase() {
        assert_eq!(convert_to_kebab_case("TestSkill"), "testskill");
    }

    #[test]
    fn test_convert_to_kebab_case_underscores() {
        assert_eq!(convert_to_kebab_case("test_skill"), "test-skill");
    }

    #[test]
    fn test_convert_to_kebab_case_mixed() {
        assert_eq!(convert_to_kebab_case("Test_Skill_Name"), "test-skill-name");
    }

    #[test]
    fn test_convert_to_kebab_case_consecutive_hyphens() {
        assert_eq!(convert_to_kebab_case("test--skill"), "test-skill");
        assert_eq!(convert_to_kebab_case("test___skill"), "test-skill");
    }

    #[test]
    fn test_convert_to_kebab_case_leading_trailing() {
        assert_eq!(convert_to_kebab_case("-test-skill-"), "test-skill");
        assert_eq!(convert_to_kebab_case("_test_skill_"), "test-skill");
    }

    #[test]
    fn test_convert_to_kebab_case_invalid_chars() {
        assert_eq!(convert_to_kebab_case("test@skill!"), "testskill");
        assert_eq!(convert_to_kebab_case("test.skill"), "testskill");
    }

    #[test]
    fn test_convert_to_kebab_case_truncate() {
        let long_name = "a".repeat(100);
        let result = convert_to_kebab_case(&long_name);
        assert!(result.len() <= 64);
        assert_eq!(result.len(), 64);
    }

    // ===== AS-004 Auto-fix Tests =====

    #[test]
    fn test_as_004_has_fix() {
        let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());
        assert_eq!(as_004[0].fixes[0].replacement, "test-skill");
    }

    #[test]
    fn test_as_004_fix_case_only_is_safe() {
        let content = r#"---
name: TestSkill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());
        // Case-only change (TestSkill -> testskill) is safe
        assert!(as_004[0].fixes[0].safe);
    }

    #[test]
    fn test_as_004_fix_structural_is_unsafe() {
        let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());
        // Structural change (Test_Skill -> test-skill) is not safe
        assert!(!as_004[0].fixes[0].safe);
    }

    #[test]
    fn test_as_004_fix_byte_position() {
        let content = r#"---
name: Bad_Name
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());

        let fix = &as_004[0].fixes[0];
        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("name: bad-name"));
    }

    #[test]
    fn test_as_004_fix_quoted_value() {
        let content = r#"---
name: "Bad_Name"
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());

        let fix = &as_004[0].fixes[0];
        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        // The fix replaces the inner value, keeping quotes in place
        assert!(fixed.contains("bad-name"));
    }

    #[test]
    fn test_as_004_no_fix_when_converts_to_empty() {
        // Name with only special characters should convert to empty string
        let content = r#"---
name: "!@#$%"
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        // Should have no fix since converted name would be empty
        assert!(!as_004[0].has_fixes());
    }

    #[test]
    fn test_as_004_underscore_to_hyphen_is_unsafe() {
        // Test_Name -> test-name involves underscore replacement, should be unsafe
        let content = r#"---
name: test_name
description: Use when testing
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
        assert_eq!(as_004.len(), 1);
        assert!(as_004[0].has_fixes());
        // Underscore to hyphen is structural change, not safe
        assert!(!as_004[0].fixes[0].safe);
    }

    // ===== AS-010 Auto-fix Tests =====

    #[test]
    fn test_as_010_has_fix() {
        let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
        assert!(as_010[0].has_fixes());
        assert_eq!(
            as_010[0].fixes[0].replacement,
            "Use when user wants to Reviews code for quality"
        );
    }

    #[test]
    fn test_as_010_fix_is_unsafe() {
        let content = r#"---
name: code-review
description: Reviews code for quality
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
        assert!(as_010[0].has_fixes());
        // Semantic change is not safe
        assert!(!as_010[0].fixes[0].safe);
    }

    #[test]
    fn test_as_010_fix_byte_position() {
        let content = r#"---
name: helper
description: Helps with tasks
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
        assert!(as_010[0].has_fixes());

        let fix = &as_010[0].fixes[0];
        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("Use when user wants to Helps with tasks"));
    }

    #[test]
    fn test_as_010_fix_quoted_value() {
        let content = r#"---
name: helper
description: "Helps with tasks"
---
Body"#;

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
        assert!(as_010[0].has_fixes());

        let fix = &as_010[0].fixes[0];
        // Apply fix and verify
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("Use when user wants to Helps with tasks"));
    }

    #[test]
    fn test_as_010_no_fix_when_description_too_long() {
        // Create a description that would exceed 1024 chars when prepending trigger phrase
        let long_desc = "a".repeat(1010);
        let content = format!("---\nname: helper\ndescription: {}\n---\nBody", long_desc);

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-010").collect();
        assert_eq!(as_010.len(), 1);
        // Should have no fix since prepending would exceed limit
        assert!(!as_010[0].has_fixes());
    }

    // ===== frontmatter_value_byte_range Tests =====

    #[test]
    fn test_frontmatter_value_byte_range_unquoted() {
        let content = r#"---
name: test-skill
description: A test skill
---
Body"#;
        let parts = split_frontmatter(content);
        let range = frontmatter_value_byte_range(content, &parts, "name");
        assert!(range.is_some());
        let (start, end) = range.unwrap();
        assert_eq!(&content[start..end], "test-skill");
    }

    #[test]
    fn test_frontmatter_value_byte_range_double_quoted() {
        let content = r#"---
name: "test-skill"
description: A test skill
---
Body"#;
        let parts = split_frontmatter(content);
        let range = frontmatter_value_byte_range(content, &parts, "name");
        assert!(range.is_some());
        let (start, end) = range.unwrap();
        assert_eq!(&content[start..end], "test-skill");
    }

    #[test]
    fn test_frontmatter_value_byte_range_single_quoted() {
        let content = r#"---
name: 'test-skill'
description: A test skill
---
Body"#;
        let parts = split_frontmatter(content);
        let range = frontmatter_value_byte_range(content, &parts, "name");
        assert!(range.is_some());
        let (start, end) = range.unwrap();
        assert_eq!(&content[start..end], "test-skill");
    }

    #[test]
    fn test_frontmatter_value_byte_range_not_found() {
        let content = r#"---
name: test-skill
---
Body"#;
        let parts = split_frontmatter(content);
        let range = frontmatter_value_byte_range(content, &parts, "description");
        assert!(range.is_none());
    }
}
