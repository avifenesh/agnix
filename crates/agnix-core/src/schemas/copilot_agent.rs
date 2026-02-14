//! GitHub Copilot custom agent schema helpers.
//!
//! Supports `.github/agents/*.agent.md` files with YAML frontmatter.

use serde::{Deserialize, Serialize};

/// Known valid keys for custom agent frontmatter.
pub const KNOWN_KEYS: &[&str] = &[
    "description",
    "tools",
    "model",
    "mcp-servers",
    "target",
    "argument-hint",
    "handoffs",
    "infer",
];

/// Frontmatter schema for custom Copilot agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CopilotAgentSchema {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tools: Option<serde_yaml::Value>,
    #[serde(default)]
    pub model: Option<serde_yaml::Value>,
    #[serde(default)]
    pub mcp_servers: Option<serde_yaml::Value>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub argument_hint: Option<serde_yaml::Value>,
    #[serde(default)]
    pub handoffs: Option<serde_yaml::Value>,
    #[serde(default)]
    pub infer: Option<serde_yaml::Value>,
}

/// Result of parsing custom-agent frontmatter.
#[derive(Debug, Clone)]
pub struct ParsedAgentFrontmatter {
    pub schema: Option<CopilotAgentSchema>,
    pub raw: String,
    pub start_line: usize,
    pub end_line: usize,
    pub body: String,
    pub unknown_keys: Vec<UnknownKey>,
    pub parse_error: Option<String>,
}

/// Unknown top-level key in frontmatter.
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Parse frontmatter from `.agent.md` content.
pub fn parse_agent_frontmatter(content: &str) -> Option<ParsedAgentFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if *line == "---" {
            end_idx = Some(i);
            break;
        }
    }

    if end_idx.is_none() {
        let raw = lines[1..].join("\n");
        return Some(ParsedAgentFrontmatter {
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

    let (schema, parse_error) = match serde_yaml::from_str::<CopilotAgentSchema>(&raw) {
        Ok(s) => (Some(s), None),
        Err(e) => (None, Some(e.to_string())),
    };

    let unknown_keys = find_unknown_keys(&raw, 2);

    Some(ParsedAgentFrontmatter {
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
    let top_level_indent = yaml
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let colon_idx = trimmed.find(':')?;
            let key = trimmed[..colon_idx].trim();
            if key.is_empty() {
                return None;
            }
            Some(line.len() - trimmed.len())
        })
        .min()
        .unwrap_or(0);

    for (i, line) in yaml.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent != top_level_indent {
            continue;
        }

        if let Some(colon_idx) = trimmed.find(':') {
            let key_raw = &trimmed[..colon_idx];
            let key = key_raw.trim().trim_matches(|c| c == '\'' || c == '"');

            if !key.is_empty() && !KNOWN_KEYS.contains(&key) {
                unknown.push(UnknownKey {
                    key: key.to_string(),
                    line: start_line + i,
                    column: indent,
                });
            }
        }
    }

    unknown
}

/// Return `true` when the markdown body is empty after trim.
pub fn is_body_empty(body: &str) -> bool {
    body.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_agent_frontmatter() {
        let content = r#"---
description: Review pull requests
target: vscode
tools: [editFiles]
---
# Agent prompt
"#;
        let parsed = parse_agent_frontmatter(content).expect("expected frontmatter");
        assert!(parsed.parse_error.is_none());
        let schema = parsed.schema.expect("expected parsed schema");
        assert_eq!(schema.description.as_deref(), Some("Review pull requests"));
        assert_eq!(schema.target.as_deref(), Some("vscode"));
        assert!(parsed.unknown_keys.is_empty());
    }

    #[test]
    fn parse_detects_unknown_keys() {
        let content = r#"---
description: Review pull requests
mystery: true
---
Body
"#;
        let parsed = parse_agent_frontmatter(content).expect("expected frontmatter");
        assert_eq!(parsed.unknown_keys.len(), 1);
        assert_eq!(parsed.unknown_keys[0].key, "mystery");
    }

    #[test]
    fn parse_detects_unknown_keys_with_uniform_indentation() {
        let content = r#"---
 description: Review pull requests
 mystery: true
 ---
Body
"#;
        let parsed = parse_agent_frontmatter(content).expect("expected frontmatter");
        assert_eq!(parsed.unknown_keys.len(), 1);
        assert_eq!(parsed.unknown_keys[0].key, "mystery");
    }

    #[test]
    fn parse_ignores_comment_lines() {
        let content = r#"---
description: Review pull requests
# target: vscode
---
Body
"#;
        let parsed = parse_agent_frontmatter(content).expect("expected frontmatter");
        assert!(
            parsed.unknown_keys.is_empty(),
            "comments should not be treated as unknown keys"
        );
    }

    #[test]
    fn parse_handles_indented_fence_in_block_scalar() {
        let content = r#"---
description: |
  Keep this separator literal:
  ---
target: vscode
---
Body
"#;
        let parsed = parse_agent_frontmatter(content).expect("expected frontmatter");
        assert!(
            parsed.parse_error.is_none(),
            "indented '---' should not terminate frontmatter"
        );
        let schema = parsed.schema.expect("expected schema");
        assert_eq!(schema.target.as_deref(), Some("vscode"));
    }

    #[test]
    fn parse_none_when_no_frontmatter() {
        assert!(parse_agent_frontmatter("# no frontmatter").is_none());
    }

    #[test]
    fn parse_unclosed_frontmatter() {
        let content = "---\ndescription: test\n";
        let parsed = parse_agent_frontmatter(content).expect("expected parse result");
        assert!(parsed.parse_error.is_some());
    }

    #[test]
    fn body_empty_helper() {
        assert!(is_body_empty(" \n\t"));
        assert!(!is_body_empty("content"));
    }
}
