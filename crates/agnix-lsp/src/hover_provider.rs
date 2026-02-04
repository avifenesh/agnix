//! Hover documentation provider for LSP.
//!
//! Provides contextual documentation when hovering over fields
//! in agent configuration files.

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

/// Field documentation for hover support.
///
/// Contains the field name pattern and its documentation.
struct FieldDoc {
    /// Field name to match (e.g., "name", "version", "model")
    field: &'static str,
    /// Markdown documentation to display on hover
    docs: &'static str,
}

/// Static documentation for common skill/agent fields.
const FIELD_DOCS: &[FieldDoc] = &[
    FieldDoc {
        field: "name",
        docs: r#"**name** (required)

The skill identifier. Must be lowercase alphanumeric with hyphens.

Example: `my-skill`, `code-review`

Rules: AS-004, CC-SK-001"#,
    },
    FieldDoc {
        field: "version",
        docs: r#"**version** (required)

Semantic version string for the skill.

Format: `MAJOR.MINOR.PATCH` (e.g., `1.0.0`, `2.3.1`)

Rules: AS-005, CC-SK-002"#,
    },
    FieldDoc {
        field: "model",
        docs: r#"**model** (required)

The AI model to use for this skill.

Common values: `sonnet`, `opus`, `haiku`

Rules: AS-006, CC-SK-003"#,
    },
    FieldDoc {
        field: "description",
        docs: r#"**description** (optional)

Human-readable description of what this skill does.

Best practices:
- Keep it concise (1-2 sentences)
- Explain the primary use case
- Mention any prerequisites"#,
    },
    FieldDoc {
        field: "tools",
        docs: r#"**tools** (optional)

List of MCP tools this skill can use.

Format: Array of tool names or tool configurations.

Example:
```yaml
tools:
  - read_file
  - write_file
  - execute_command
```

Rules: CC-SK-TL-001"#,
    },
    FieldDoc {
        field: "allowed_tools",
        docs: r#"**allowed_tools** (optional)

Restricts which tools this skill can access.

This provides a security boundary for skill execution.

Example:
```yaml
allowed_tools:
  - read_file
  - list_directory
```"#,
    },
    FieldDoc {
        field: "triggers",
        docs: r#"**triggers** (optional)

Patterns that activate this skill automatically.

Example:
```yaml
triggers:
  - pattern: "review.*code"
    description: "Code review requests"
```

Rules: CC-HK-001"#,
    },
    FieldDoc {
        field: "hooks",
        docs: r#"**hooks** (optional)

Lifecycle hooks for skill execution.

Available hooks:
- `pre_invoke`: Run before skill starts
- `post_invoke`: Run after skill completes
- `on_error`: Run if skill fails

Rules: CC-HK-002"#,
    },
    FieldDoc {
        field: "memory",
        docs: r#"**memory** (optional)

Memory configuration for the skill.

Controls how context is managed across invocations.

Rules: AS-MEM-001"#,
    },
    FieldDoc {
        field: "context",
        docs: r#"**context** (optional)

Additional context files to include.

Example:
```yaml
context:
  - path: "./docs/api.md"
    description: "API documentation"
```"#,
    },
    FieldDoc {
        field: "prompt",
        docs: r#"**prompt** (optional)

System prompt or instructions for the skill.

Can be inline text or a file reference."#,
    },
    FieldDoc {
        field: "mcpServers",
        docs: r#"**mcpServers** (MCP configuration)

Defines MCP server connections.

Example:
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"]
    }
  }
}
```

Rules: MCP-001"#,
    },
    FieldDoc {
        field: "command",
        docs: r#"**command** (MCP server)

The command to launch an MCP server.

Common values: `npx`, `node`, `python`

Rules: MCP-002"#,
    },
    FieldDoc {
        field: "args",
        docs: r#"**args** (MCP server)

Arguments passed to the MCP server command.

Example:
```json
"args": ["-y", "@modelcontextprotocol/server-filesystem"]
```

Rules: MCP-003"#,
    },
    FieldDoc {
        field: "env",
        docs: r#"**env** (MCP server)

Environment variables for the MCP server.

Example:
```json
"env": {
  "API_KEY": "${API_KEY}"
}
```

Rules: MCP-004"#,
    },
];

/// Get the field name at a position in YAML content.
///
/// Looks for patterns like `field:` or `  field:` and returns
/// the field name if the position is on that line.
///
/// # Arguments
///
/// * `content` - The document content
/// * `position` - The cursor position
///
/// # Returns
///
/// The field name if found, or None if the position is not on a field.
pub fn get_field_at_position(content: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    let line_idx = position.line as usize;
    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];

    // Look for YAML field pattern: optional whitespace, field name, colon
    let trimmed = line.trim_start();
    if let Some(colon_pos) = trimmed.find(':') {
        let field = trimmed[..colon_pos].trim();
        // Verify the field is a valid identifier (alphanumeric + underscore)
        if !field.is_empty()
            && field
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            // Check if cursor is on or before the colon (on the field name)
            let char_pos = position.character as usize;
            let leading_spaces = line.len() - trimmed.len();
            let field_end = leading_spaces + colon_pos;

            if char_pos <= field_end {
                return Some(field.to_string());
            }
        }
    }

    // Also check JSON-style "field": pattern
    // Find first quote in the trimmed string
    if let Some(first_quote) = trimmed.find('"') {
        let after_first_quote = &trimmed[first_quote + 1..];
        if let Some(second_quote) = after_first_quote.find('"') {
            let field = &after_first_quote[..second_quote];
            // Verify it's followed by a colon (with optional whitespace)
            let after_field = &after_first_quote[second_quote + 1..];
            let after_ws = after_field.trim_start();
            if after_ws.starts_with(':') {
                // Check if cursor is on or before the colon
                let char_pos = position.character as usize;
                let leading_spaces = line.len() - trimmed.len();
                // The colon is at: leading_spaces + first_quote + 1 + second_quote + 1 + whitespace
                let colon_offset = trimmed.len() - after_ws.len();
                let field_end = leading_spaces + colon_offset;

                if char_pos <= field_end {
                    return Some(field.to_string());
                }
            }
        }
    }

    None
}

/// Get hover information for a field.
///
/// # Arguments
///
/// * `field` - The field name to look up
///
/// # Returns
///
/// A Hover with markdown documentation if the field is known.
pub fn get_hover_info(field: &str) -> Option<Hover> {
    // Look up the field in our documentation
    let doc = FIELD_DOCS.iter().find(|d| d.field == field)?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.docs.to_string(),
        }),
        range: None,
    })
}

/// Get hover information for a position in a document.
///
/// Combines field detection and documentation lookup.
///
/// # Arguments
///
/// * `content` - The document content
/// * `position` - The cursor position
///
/// # Returns
///
/// A Hover if there's documentation for the field at the position.
pub fn hover_at_position(content: &str, position: Position) -> Option<Hover> {
    let field = get_field_at_position(content, position)?;
    get_hover_info(&field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_field_at_position_yaml() {
        let content = "---\nname: test-skill\nversion: 1.0.0\n---";

        // On "name" field
        let pos = Position {
            line: 1,
            character: 0,
        };
        assert_eq!(
            get_field_at_position(content, pos),
            Some("name".to_string())
        );

        // On "version" field
        let pos = Position {
            line: 2,
            character: 3,
        };
        assert_eq!(
            get_field_at_position(content, pos),
            Some("version".to_string())
        );
    }

    #[test]
    fn test_get_field_at_position_after_colon() {
        let content = "name: test-skill";

        // After the colon (on the value)
        let pos = Position {
            line: 0,
            character: 10,
        };
        assert_eq!(get_field_at_position(content, pos), None);
    }

    #[test]
    fn test_get_field_at_position_indented() {
        let content = "root:\n  nested: value";

        let pos = Position {
            line: 1,
            character: 4,
        };
        assert_eq!(
            get_field_at_position(content, pos),
            Some("nested".to_string())
        );
    }

    #[test]
    fn test_get_field_at_position_json() {
        let content = r#"{"name": "test"}"#;

        let pos = Position {
            line: 0,
            character: 2,
        };
        assert_eq!(
            get_field_at_position(content, pos),
            Some("name".to_string())
        );
    }

    #[test]
    fn test_get_field_at_position_out_of_bounds() {
        let content = "name: test";

        let pos = Position {
            line: 5,
            character: 0,
        };
        assert_eq!(get_field_at_position(content, pos), None);
    }

    #[test]
    fn test_get_hover_info_known_field() {
        let hover = get_hover_info("name");
        assert!(hover.is_some());

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("name"));
                assert!(markup.value.contains("required"));
            }
            _ => panic!("Expected Markup content"),
        }
    }

    #[test]
    fn test_get_hover_info_unknown_field() {
        let hover = get_hover_info("unknown_field_xyz");
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_at_position_found() {
        let content = "---\nname: test\nversion: 1.0.0\n---";

        let pos = Position {
            line: 1,
            character: 2,
        };
        let hover = hover_at_position(content, pos);

        assert!(hover.is_some());
    }

    #[test]
    fn test_hover_at_position_not_found() {
        let content = "---\nunknown_xyz: test\n---";

        let pos = Position {
            line: 1,
            character: 0,
        };
        let hover = hover_at_position(content, pos);

        assert!(hover.is_none());
    }

    #[test]
    fn test_all_documented_fields_have_hover() {
        let fields = [
            "name",
            "version",
            "model",
            "description",
            "tools",
            "mcpServers",
            "command",
            "args",
            "env",
        ];

        for field in fields {
            let hover = get_hover_info(field);
            assert!(hover.is_some(), "Field '{}' should have documentation", field);
        }
    }

    #[test]
    fn test_hover_content_format() {
        // All hovers should be markdown and contain the field name
        for doc in FIELD_DOCS {
            let hover = get_hover_info(doc.field).unwrap();
            match hover.contents {
                HoverContents::Markup(markup) => {
                    assert_eq!(markup.kind, MarkupKind::Markdown);
                    assert!(
                        markup.value.contains(doc.field),
                        "Hover for '{}' should contain field name",
                        doc.field
                    );
                }
                _ => panic!("Expected Markup content for field '{}'", doc.field),
            }
        }
    }
}
