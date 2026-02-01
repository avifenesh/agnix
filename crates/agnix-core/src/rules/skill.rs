//! Skill file validation

use crate::{config::LintConfig, diagnostics::Diagnostic, rules::Validator, schemas::SkillSchema};
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

struct FrontmatterParts {
    has_frontmatter: bool,
    has_closing: bool,
    frontmatter: String,
    body: String,
}

static NAME_FORMAT_REGEX: OnceLock<Regex> = OnceLock::new();
static DESCRIPTION_XML_REGEX: OnceLock<Regex> = OnceLock::new();
static REFERENCE_PATH_REGEX: OnceLock<Regex> = OnceLock::new();
static WINDOWS_PATH_REGEX: OnceLock<Regex> = OnceLock::new();

pub struct SkillValidator;

impl Validator for SkillValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if !config.rules.frontmatter_validation {
            return diagnostics;
        }

        let parts = split_frontmatter(content);

        // AS-001: Missing frontmatter
        if config.is_rule_enabled("AS-001") && (!parts.has_frontmatter || !parts.has_closing) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
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
                        1,
                        0,
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
            // AS-002: Missing name field
            if config.is_rule_enabled("AS-002") && frontmatter.name.is_none() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
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
                        1,
                        0,
                        "AS-003",
                        "Skill frontmatter is missing required 'description' field".to_string(),
                    )
                    .with_suggestion(
                        "Add 'description: Use when...' to frontmatter".to_string(),
                    ),
                );
            }

            if let Some(name) = frontmatter.name.as_deref() {
                let name_trimmed = name.trim();

                // AS-004: Invalid name format
                if config.is_rule_enabled("AS-004") {
                    let name_re = NAME_FORMAT_REGEX
                        .get_or_init(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap());
                    if name.len() > 64 || !name_re.is_match(name) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "AS-004",
                                format!(
                                    "Name '{}' must be 1-64 characters of lowercase letters, digits, and hyphens",
                                    name
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
                    && (name.starts_with('-') || name.ends_with('-'))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "AS-005",
                            format!("Name '{}' cannot start or end with hyphen", name),
                        )
                        .with_suggestion(
                            "Remove leading/trailing hyphens from the name".to_string(),
                        ),
                    );
                }

                // AS-006: Name cannot contain consecutive hyphens
                if config.is_rule_enabled("AS-006") && name.contains("--") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "AS-006",
                            format!("Name '{}' cannot contain consecutive hyphens", name),
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
                                1,
                                0,
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
                    if len < 1 || len > 1024 {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
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
                    let xml_re = DESCRIPTION_XML_REGEX
                        .get_or_init(|| Regex::new(r"<[^>]+>").unwrap());
                    if xml_re.is_match(description) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
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
                                1,
                                0,
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
                                1,
                                0,
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

            if let (Some(name), Some(description)) =
                (frontmatter.name.as_deref(), frontmatter.description.as_deref())
            {
                let name_trimmed = name.trim();
                let description_trimmed = description.trim();
                if name_trimmed.is_empty() || description_trimmed.is_empty() {
                    // Schema validation requires both fields to be present and non-empty.
                } else {
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

                    if let Err(error) = schema.validate_model() {
                        diagnostics.push(Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "skill::schema",
                            error,
                        ));
                    }

                    if let Err(error) = schema.validate_context() {
                        diagnostics.push(Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "skill::schema",
                            error,
                        ));
                    }

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
                                    1,
                                    0,
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

                    // CC-SK-007: Unrestricted Bash warning
                    if config.is_rule_enabled("CC-SK-007") {
                        if let Some(tools) = &frontmatter.allowed_tools {
                            let tool_list: Vec<&str> = tools.split_whitespace().collect();
                            for tool in tool_list {
                                if tool == "Bash" {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-SK-007",
                                            "Unrestricted Bash access detected. Consider using scoped version for better security.".to_string(),
                                        )
                                        .with_suggestion("Use scoped Bash like 'Bash(git:*)' or 'Bash(npm:*)' instead of plain 'Bash'".to_string()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // AS-012: Content exceeds 500 lines
        if config.is_rule_enabled("AS-012") {
            let line_count = parts.body.lines().count();
            if line_count > 500 {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "AS-012",
                        format!(
                            "Skill content exceeds 500 lines (got {})",
                            line_count
                        ),
                    )
                    .with_suggestion("Move extra content into references/".to_string()),
                );
            }
        }

        // AS-013: File reference too deep
        if config.is_rule_enabled("AS-013") {
            let paths = extract_reference_paths(&parts.body);
            for ref_path in paths {
                if reference_path_too_deep(&ref_path) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "AS-013",
                            format!("File reference '{}' is deeper than one level", ref_path),
                        )
                        .with_suggestion("Flatten the references/ directory structure".to_string()),
                    );
                }
            }
        }

        // AS-014: Windows path separator
        if config.is_rule_enabled("AS-014") {
            let paths = extract_windows_paths(&parts.body);
            for win_path in paths {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "AS-014",
                        format!(
                            "Windows path separator detected in '{}'; use forward slashes",
                            win_path
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
                            1,
                            0,
                            "AS-015",
                            format!(
                                "Skill directory exceeds 8MB ({} bytes)",
                                size
                            ),
                        )
                        .with_suggestion(
                            "Remove large assets or split the skill into smaller parts"
                                .to_string(),
                        ),
                    );
                }
            }
        }

        diagnostics
    }
}

fn split_frontmatter(content: &str) -> FrontmatterParts {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return FrontmatterParts {
            has_frontmatter: false,
            has_closing: false,
            frontmatter: String::new(),
            body: trimmed.to_string(),
        };
    }

    let rest = &trimmed[3..];
    if let Some(end_pos) = rest.find("\n---") {
        let frontmatter = &rest[..end_pos];
        let body = &rest[end_pos + 4..];
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: true,
            frontmatter: frontmatter.to_string(),
            body: body.trim_start().to_string(),
        }
    } else {
        FrontmatterParts {
            has_frontmatter: true,
            has_closing: false,
            frontmatter: String::new(),
            body: rest.to_string(),
        }
    }
}

fn parse_frontmatter_fields(frontmatter: &str) -> Result<SkillFrontmatter, serde_yaml::Error> {
    if frontmatter.trim().is_empty() {
        return Ok(SkillFrontmatter::default());
    }
    serde_yaml::from_str(frontmatter)
}

fn extract_reference_paths(body: &str) -> Vec<String> {
    let re = REFERENCE_PATH_REGEX.get_or_init(|| {
        Regex::new("(?i)\\b(?:references?|refs)[/\\\\][^\\s)\\]}>\"']+").unwrap()
    });
    let mut paths = HashSet::new();
    for m in re.find_iter(body) {
        let trimmed = trim_path_token(m.as_str());
        if !trimmed.is_empty() {
            paths.insert(trimmed.to_string());
        }
    }
    paths.into_iter().collect()
}

fn extract_windows_paths(body: &str) -> Vec<String> {
    let re = WINDOWS_PATH_REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:[a-z]:)?[a-z0-9._-]+(?:\\[a-z0-9._-]+)+\b").unwrap()
    });
    let mut paths = HashSet::new();
    for m in re.find_iter(body) {
        let trimmed = trim_path_token(m.as_str());
        if !trimmed.is_empty() {
            paths.insert(trimmed.to_string());
        }
    }
    paths.into_iter().collect()
}

fn reference_path_too_deep(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut parts = normalized.split('/').filter(|part| !part.is_empty());
    let Some(prefix) = parts.next() else { return false };
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
        .trim_start_matches(|c: char| matches!(c, '(' | '[' | '{' | '<' | '"' | '\''))
        .trim_end_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | ')' | ']' | '}' | '>' | '"' | '\''))
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
            let Ok(file_type) = entry.file_type() else { continue };
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
        let content = include_str!("../../../../tests/fixtures/skills/missing-frontmatter.md");

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

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
        let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

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
        let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_011_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-011").collect();
        assert_eq!(as_011_errors.len(), 1);
    }

    #[test]
    fn test_as_012_content_too_long() {
        let body = (0..501)
            .map(|_| "line")
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "---\nname: test-skill\ndescription: Use when validating content length\n---\n{}",
            body
        );

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let as_012_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-012").collect();
        assert_eq!(as_012_warnings.len(), 1);
    }

    #[test]
    fn test_as_013_reference_too_deep() {
        let content = include_str!("../../../../tests/fixtures/skills/deep-reference.md");

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

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
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

        let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
        assert_eq!(as_013_errors.len(), 1);
    }

    #[test]
    fn test_as_014_windows_path_separator() {
        let content = include_str!("../../../../tests/fixtures/skills/windows-path.md");

        let validator = SkillValidator;
        let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

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
