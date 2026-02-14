//! GitHub Copilot reusable prompt schema helpers.
//!
//! Supports `.github/prompts/*.prompt.md` files with optional YAML frontmatter.

use serde::{Deserialize, Serialize};

/// Known valid keys for prompt frontmatter.
pub const KNOWN_KEYS: &[&str] = &[
    "description",
    "name",
    "argument-hint",
    "agent",
    "model",
    "tools",
];

/// Valid `agent` values for prompt frontmatter.
pub const VALID_AGENT_MODES: &[&str] = &["none", "ask", "always"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CopilotPromptSchema {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub argument_hint: Option<serde_yaml::Value>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub model: Option<serde_yaml::Value>,
    #[serde(default)]
    pub tools: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone)]
pub struct ParsedPromptFrontmatter {
    pub schema: Option<CopilotPromptSchema>,
    pub raw: String,
    pub start_line: usize,
    pub end_line: usize,
    pub body: String,
    pub unknown_keys: Vec<UnknownKey>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Parse optional frontmatter from `.prompt.md` content.
pub fn parse_prompt_frontmatter(content: &str) -> Option<ParsedPromptFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    if end_idx.is_none() {
        let raw = lines[1..].join("\n");
        return Some(ParsedPromptFrontmatter {
            schema: None,
            raw,
            start_line: 1,
            end_line: lines.len(),
            body: String::new(),
            unknown_keys: Vec::new(),
            parse_error: Some("missing closing ---".to_string()),
        });
    }

    let end_idx = end_idx.expect("checked is_some above");
    let raw = lines[1..end_idx].join("\n");
    let body = lines[end_idx + 1..].join("\n");

    let (schema, parse_error) = match serde_yaml::from_str::<CopilotPromptSchema>(&raw) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e.to_string())),
    };

    let unknown_keys = find_unknown_keys(&raw, 2);

    Some(ParsedPromptFrontmatter {
        schema,
        raw,
        start_line: 1,
        end_line: end_idx + 1,
        body,
        unknown_keys,
        parse_error,
    })
}

fn find_unknown_keys(yaml: &str, start_line: usize) -> Vec<UnknownKey> {
    let mut unknown = Vec::new();

    for (i, line) in yaml.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let key_raw = &line[..colon_idx];
            let key = key_raw.trim().trim_matches(|c| c == '\'' || c == '"');

            if !key.is_empty() && !KNOWN_KEYS.contains(&key) {
                unknown.push(UnknownKey {
                    key: key.to_string(),
                    line: start_line + i,
                    column: key_raw.len() - key_raw.trim_start().len(),
                });
            }
        }
    }

    unknown
}

pub fn is_body_empty(body: &str) -> bool {
    body.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_prompt_frontmatter() {
        let content = r#"---
description: Summarize current file
agent: ask
---
Summarize the current file.
"#;
        let parsed = parse_prompt_frontmatter(content).expect("expected frontmatter");
        assert!(parsed.parse_error.is_none());
        let schema = parsed.schema.expect("expected schema");
        assert_eq!(schema.agent.as_deref(), Some("ask"));
        assert!(parsed.unknown_keys.is_empty());
    }

    #[test]
    fn parse_unknown_keys() {
        let content = r#"---
description: test
weird-key: yes
---
Body
"#;
        let parsed = parse_prompt_frontmatter(content).expect("expected frontmatter");
        assert_eq!(parsed.unknown_keys.len(), 1);
        assert_eq!(parsed.unknown_keys[0].key, "weird-key");
    }

    #[test]
    fn parse_ignores_comment_lines() {
        let content = r#"---
description: test
# agent: ask
---
Body
"#;
        let parsed = parse_prompt_frontmatter(content).expect("expected frontmatter");
        assert!(
            parsed.unknown_keys.is_empty(),
            "comments should not be treated as unknown keys"
        );
    }

    #[test]
    fn parse_no_frontmatter_returns_none() {
        assert!(parse_prompt_frontmatter("Prompt body").is_none());
    }

    #[test]
    fn valid_agent_modes_constant() {
        assert!(VALID_AGENT_MODES.contains(&"none"));
        assert!(VALID_AGENT_MODES.contains(&"ask"));
        assert!(VALID_AGENT_MODES.contains(&"always"));
    }

    #[test]
    fn body_empty_helper() {
        assert!(is_body_empty(" \n"));
        assert!(!is_body_empty("x"));
    }
}
