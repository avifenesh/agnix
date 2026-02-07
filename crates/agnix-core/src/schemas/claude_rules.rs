//! `.claude/rules/*.md` frontmatter schema helpers
//!
//! Provides parsing and validation for Claude Code rule files that support
//! conditional loading via `paths` glob patterns in YAML frontmatter.

use std::collections::HashSet;

/// Known valid keys for `.claude/rules/*.md` frontmatter
const KNOWN_KEYS: &[&str] = &["paths"];

/// Frontmatter schema for Claude rule files
#[derive(Debug, Clone, Default)]
pub struct ClaudeRuleSchema {
    /// Glob patterns specifying which files this rule applies to
    pub paths: Vec<String>,
}

/// Result of parsing Claude rule file frontmatter
#[derive(Debug, Clone)]
pub struct ParsedRuleFrontmatter {
    /// The parsed schema (if valid YAML)
    pub schema: Option<ClaudeRuleSchema>,
    /// Raw frontmatter string (between --- markers)
    pub raw: String,
    /// Line number where frontmatter starts (1-indexed)
    pub start_line: usize,
    /// Line number where frontmatter ends (1-indexed)
    pub end_line: usize,
    /// Unknown keys found in frontmatter
    pub unknown_keys: Vec<UnknownKey>,
    /// Parse error if YAML is invalid
    pub parse_error: Option<String>,
}

/// An unknown key found in frontmatter
#[derive(Debug, Clone)]
pub struct UnknownKey {
    pub key: String,
    pub line: usize,
    pub column: usize,
}

/// Result of validating a glob pattern
#[derive(Debug, Clone)]
pub struct GlobValidation {
    pub valid: bool,
    pub pattern: String,
    pub error: Option<String>,
}

/// Parse frontmatter from a `.claude/rules/*.md` file
///
/// Returns parsed frontmatter if present, or None if no frontmatter exists.
pub fn parse_frontmatter(content: &str) -> Option<ParsedRuleFrontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return None;
    }

    // Find closing ---
    let mut end_idx = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(i);
            break;
        }
    }

    // If we have an opening --- but no closing ---,
    // treat this as invalid frontmatter rather than missing frontmatter.
    if end_idx.is_none() {
        let frontmatter_lines: Vec<&str> = lines[1..].to_vec();
        let raw = frontmatter_lines.join("\n");

        return Some(ParsedRuleFrontmatter {
            schema: None,
            raw,
            start_line: 1,
            end_line: lines.len(),
            unknown_keys: Vec::new(),
            parse_error: Some("missing closing ---".to_string()),
        });
    }

    let end_idx = end_idx.unwrap();

    // Extract frontmatter content (between --- markers)
    let frontmatter_lines: Vec<&str> = lines[1..end_idx].to_vec();
    let raw = frontmatter_lines.join("\n");

    // Extract unknown keys first (before YAML parsing which may deserialize differently)
    let unknown_keys = find_unknown_keys(&raw, 2); // Start at line 2 (after first ---)

    // Try to parse the paths field from YAML
    let (schema, parse_error) = parse_paths_schema(&raw);

    Some(ParsedRuleFrontmatter {
        schema,
        raw,
        start_line: 1,
        end_line: end_idx + 1,
        unknown_keys,
        parse_error,
    })
}

/// Parse the paths schema from raw YAML frontmatter
fn parse_paths_schema(raw: &str) -> (Option<ClaudeRuleSchema>, Option<String>) {
    // Use serde_yaml to parse as a generic value first
    let value: serde_yaml::Value = match serde_yaml::from_str(raw) {
        Ok(v) => v,
        Err(e) => return (None, Some(e.to_string())),
    };

    let mapping = match value.as_mapping() {
        Some(m) => m,
        None => return (Some(ClaudeRuleSchema::default()), None),
    };

    let mut paths = Vec::new();

    if let Some(paths_value) = mapping.get(&serde_yaml::Value::String("paths".to_string())) {
        match paths_value {
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        paths.push(s.to_string());
                    }
                }
            }
            serde_yaml::Value::String(s) => {
                paths.push(s.to_string());
            }
            _ => {}
        }
    }

    (Some(ClaudeRuleSchema { paths }), None)
}

/// Find unknown keys in frontmatter YAML
fn find_unknown_keys(yaml: &str, start_line: usize) -> Vec<UnknownKey> {
    let known: HashSet<&str> = KNOWN_KEYS.iter().copied().collect();
    let mut unknown = Vec::new();

    for (i, line) in yaml.lines().enumerate() {
        // Heuristic: top-level keys in YAML frontmatter are not indented.
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(colon_idx) = line.find(':') {
            let key_raw = &line[..colon_idx];
            let key = key_raw.trim().trim_matches(|c| c == '\'' || c == '\"');

            if !key.is_empty() && !known.contains(key) {
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

/// Validate a glob pattern
///
/// Uses the glob crate to validate pattern syntax.
pub fn validate_glob_pattern(pattern: &str) -> GlobValidation {
    match glob::Pattern::new(pattern) {
        Ok(_) => GlobValidation {
            valid: true,
            pattern: pattern.to_string(),
            error: None,
        },
        Err(e) => GlobValidation {
            valid: false,
            pattern: pattern.to_string(),
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Frontmatter Parsing =====

    #[test]
    fn test_parse_valid_frontmatter_with_paths() {
        let content = r#"---
paths:
  - "src/**/*.ts"
  - "lib/**/*.js"
---
# Rule content
Use TypeScript strict mode.
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.paths.len(), 2);
        assert_eq!(schema.paths[0], "src/**/*.ts");
        assert_eq!(schema.paths[1], "lib/**/*.js");
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_parse_single_path_string() {
        let content = "---\npaths: \"src/**/*.ts\"\n---\nContent";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.paths.len(), 1);
        assert_eq!(schema.paths[0], "src/**/*.ts");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just markdown without frontmatter";
        let result = parse_frontmatter(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_empty_frontmatter() {
        let content = "---\n---\n# Rule content";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert!(schema.paths.is_empty());
        assert!(result.parse_error.is_none());
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = "---\npaths:\n  - \"src/**/*.ts\"";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.parse_error.is_some());
        assert_eq!(result.parse_error.as_ref().unwrap(), "missing closing ---");
    }

    // ===== Unknown Keys Detection =====

    #[test]
    fn test_detect_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
description: "some rule"
alwaysApply: true
---
# Content
"#;
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 2);
        assert!(result.unknown_keys.iter().any(|k| k.key == "description"));
        assert!(result.unknown_keys.iter().any(|k| k.key == "alwaysApply"));
    }

    #[test]
    fn test_no_unknown_keys() {
        let content = r#"---
paths:
  - "src/**/*.ts"
---
# Content
"#;
        let result = parse_frontmatter(content).unwrap();
        assert!(result.unknown_keys.is_empty());
    }

    #[test]
    fn test_paths_is_known_key() {
        let content = "---\npaths: [\"*.ts\"]\n---\nContent";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.unknown_keys.is_empty());
    }

    // ===== Glob Pattern Validation =====

    #[test]
    fn test_valid_glob_patterns() {
        let patterns = vec![
            "**/*.ts",
            "*.rs",
            "src/**/*.js",
            "tests/unit/*.test.ts",
            "[abc].txt",
            "file?.md",
        ];

        for pattern in patterns {
            let result = validate_glob_pattern(pattern);
            assert!(result.valid, "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let result = validate_glob_pattern("[unclosed");
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    // ===== Line Number Tracking =====

    #[test]
    fn test_frontmatter_line_numbers() {
        let content = "---\npaths:\n  - \"*.ts\"\n---\n# Body";
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.start_line, 1);
        assert_eq!(result.end_line, 4);
    }

    #[test]
    fn test_unknown_key_line_numbers() {
        let content = "---\npaths:\n  - \"*.ts\"\nunknownKey: value\n---\n# Body";
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.unknown_keys.len(), 1);
        // unknownKey is on line 4 (line 1 is ---, line 2 is paths, line 3 is item, line 4 is unknownKey)
        assert_eq!(result.unknown_keys[0].line, 4);
    }
}
