//! Skill file validation

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::{split_frontmatter, FrontmatterParts},
    rules::Validator,
    schemas::SkillSchema,
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
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        frontmatter_line,
                        frontmatter_col,
                        "skill::parse",
                        format!("Failed to parse SKILL.md: {}", e),
                    ));
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
                        diagnostics.push(
                            Diagnostic::error(
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
                                "Lowercase the name, replace '_' with '-', and remove invalid characters".to_string(),
                            ),
                        );
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
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                description_line,
                                description_col,
                                "AS-010",
                                "Description should include a 'Use when...' trigger phrase"
                                    .to_string(),
                            )
                            .with_suggestion(
                                "Add 'Use when [condition]' to help Claude understand when to invoke this skill".to_string(),
                            ),
                        );
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
                            for tool in tools {
                                if *tool == "Bash" {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            allowed_tools_line,
                                            allowed_tools_col,
                                            "CC-SK-007",
                                            "Unrestricted Bash access detected. Consider using scoped version for better security.".to_string(),
                                        )
                                        .with_suggestion("Use scoped Bash like 'Bash(git:*)' or 'Bash(npm:*)' instead of plain 'Bash'".to_string()),
                                    );
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
    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            offset += line.len() + 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.trim_start().starts_with(':') {
                let column = line.len() - trimmed.len();
                return Some(offset + column);
            }
        }
        offset += line.len() + 1;
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

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "skill::parse")
            .collect();

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
}
