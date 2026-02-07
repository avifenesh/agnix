//! Hover documentation provider for LSP.
//!
//! Provides contextual documentation when hovering over fields
//! in agent configuration files, backed by agnix-core authoring metadata.

use agnix_core::FileType;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

/// Get the field name at a position in YAML/JSON-like content.
///
/// Looks for patterns like `field:` or `"field":` and returns
/// the field name if the position is on that key.
pub fn get_field_at_position(content: &str, position: Position) -> Option<String> {
    let line_idx = position.line as usize;
    let line = content.lines().nth(line_idx)?;

    let trimmed = line.trim_start();
    // Strip JSON object/array delimiters for hover detection
    let trimmed = trimmed.trim_start_matches(['{', '[', ',']);
    let trimmed = trimmed.trim_start();
    if let Some(colon_pos) = trimmed.find(':') {
        let mut field = trimmed[..colon_pos].trim();
        field = field.trim_matches('"').trim_matches('\'');

        if !field.is_empty()
            && field
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            let char_pos = position.character as usize;
            let leading_spaces = line.len().saturating_sub(trimmed.len());
            let field_end = leading_spaces + colon_pos;

            if char_pos <= field_end {
                return Some(field.to_string());
            }
        }
    }

    None
}

/// Get hover information for a field.
pub fn get_hover_info(file_type: FileType, field: &str) -> Option<Hover> {
    let doc = agnix_core::authoring::hover_doc(file_type, field)?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.markdown,
        }),
        range: None,
    })
}

/// Get hover information for a position in a document.
pub fn hover_at_position(file_type: FileType, content: &str, position: Position) -> Option<Hover> {
    let field = get_field_at_position(content, position)?;
    get_hover_info(file_type, &field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_field_at_position_yaml() {
        let content = "---\nname: test-skill\nversion: 1.0.0\n---";

        let pos = Position {
            line: 1,
            character: 0,
        };
        assert_eq!(
            get_field_at_position(content, pos),
            Some("name".to_string())
        );

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
    fn test_get_field_at_position_after_colon() {
        let content = "name: test-skill";

        let pos = Position {
            line: 0,
            character: 10,
        };
        assert_eq!(get_field_at_position(content, pos), None);
    }

    #[test]
    fn test_get_hover_info_known_field() {
        let hover = get_hover_info(FileType::Skill, "name");
        assert!(hover.is_some());

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("name"));
            }
            _ => panic!("Expected Markup content"),
        }
    }

    #[test]
    fn test_get_hover_info_unknown_field() {
        let hover = get_hover_info(FileType::Skill, "unknown_field_xyz");
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_at_position_found() {
        let content = "---\nname: test\nversion: 1.0.0\n---";

        let pos = Position {
            line: 1,
            character: 2,
        };
        let hover = hover_at_position(FileType::Skill, content, pos);

        assert!(hover.is_some());
    }

    // ---- Hover tests for non-skill file types ----

    #[test]
    fn test_hover_agent_model_field() {
        let hover = get_hover_info(FileType::Agent, "model");
        assert!(hover.is_some(), "Agent should have hover for 'model'");

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::Markdown);
                assert!(markup.value.contains("model"));
            }
            _ => panic!("Expected Markup content for Agent hover"),
        }
    }

    #[test]
    fn test_hover_hooks_type_field() {
        let hover = get_hover_info(FileType::Hooks, "type");
        assert!(hover.is_some(), "Hooks should have hover for 'type'");

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert!(markup.value.contains("type"));
            }
            _ => panic!("Expected Markup content for Hooks hover"),
        }
    }

    #[test]
    fn test_hover_mcp_jsonrpc_field() {
        let hover = get_hover_info(FileType::Mcp, "jsonrpc");
        assert!(hover.is_some(), "MCP should have hover for 'jsonrpc'");

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert!(markup.value.contains("jsonrpc"));
            }
            _ => panic!("Expected Markup content for MCP hover"),
        }
    }

    #[test]
    fn test_hover_copilot_applyto_field() {
        // Copilot maps to the "copilot" family, which includes CopilotScoped
        let hover = get_hover_info(FileType::CopilotScoped, "applyTo");
        assert!(
            hover.is_some(),
            "CopilotScoped should have hover for 'applyTo'"
        );

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert!(markup.value.contains("applyTo"));
            }
            _ => panic!("Expected Markup content for Copilot hover"),
        }
    }

    #[test]
    fn test_hover_cursor_description_field() {
        let hover = get_hover_info(FileType::CursorRule, "description");
        assert!(
            hover.is_some(),
            "CursorRule should have hover for 'description'"
        );

        let hover = hover.unwrap();
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert!(markup.value.contains("description"));
            }
            _ => panic!("Expected Markup content for CursorRule hover"),
        }
    }

    #[test]
    fn test_hover_at_position_agent_file() {
        let content = "---\nmodel: sonnet\n---";
        let pos = Position {
            line: 1,
            character: 2,
        };
        let hover = hover_at_position(FileType::Agent, content, pos);
        assert!(hover.is_some(), "Should get hover for Agent 'model' field");
    }

    #[test]
    fn test_hover_at_position_hooks_json() {
        let content = r#"{"type": "command"}"#;
        let pos = Position {
            line: 0,
            character: 2,
        };
        let hover = hover_at_position(FileType::Hooks, content, pos);
        assert!(
            hover.is_some(),
            "Should get hover for Hooks 'type' field"
        );
    }

    #[test]
    fn test_hover_at_position_not_found() {
        let content = "---\nunknown_xyz: test\n---";

        let pos = Position {
            line: 1,
            character: 0,
        };
        let hover = hover_at_position(FileType::Skill, content, pos);

        assert!(hover.is_none());
    }
}
