//! Completion provider adapter for agnix-core authoring catalog.

use std::path::Path;

use agnix_core::authoring::{CompletionKind, completion_candidates};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

use crate::position::position_to_byte;

fn completion_kind(kind: &CompletionKind) -> CompletionItemKind {
    match kind {
        CompletionKind::Key => CompletionItemKind::FIELD,
        CompletionKind::Value => CompletionItemKind::VALUE,
        CompletionKind::Snippet => CompletionItemKind::SNIPPET,
    }
}

/// Return completion items for a document position.
pub fn completion_items_for_document(
    path: &Path,
    content: &str,
    position: Position,
    config: &agnix_core::LintConfig,
) -> Vec<CompletionItem> {
    let file_type = agnix_core::resolve_file_type(path, config);
    if matches!(file_type, agnix_core::FileType::Unknown) {
        return Vec::new();
    }

    let cursor_byte = position_to_byte(content, position);
    completion_candidates(file_type, content, cursor_byte)
        .into_iter()
        .map(|candidate| {
            let docs = candidate.documentation.map(|docs| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: docs,
                })
            });

            let mut detail = candidate.detail;
            if !candidate.rule_links.is_empty() {
                let rule_text = format!("Rules: {}", candidate.rule_links.join(", "));
                detail = Some(match detail {
                    Some(existing) => format!("{} ({})", existing, rule_text),
                    None => rule_text,
                });
            }

            CompletionItem {
                label: candidate.label,
                kind: Some(completion_kind(&candidate.kind)),
                detail,
                documentation: docs,
                insert_text: Some(candidate.insert_text),
                insert_text_format: if matches!(candidate.kind, CompletionKind::Snippet) {
                    Some(InsertTextFormat::SNIPPET)
                } else {
                    Some(InsertTextFormat::PLAIN_TEXT)
                },
                ..Default::default()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_completion_includes_name_field() {
        let content = "---\nna\n---\n";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new("SKILL.md"),
            content,
            Position {
                line: 1,
                character: 1,
            },
            &config,
        );
        assert!(items.iter().any(|item| item.label == "name"));
    }

    #[test]
    fn test_agent_completion_includes_model_field() {
        let content = "---\nmod\n---\n";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            // Agent files are in .claude/agents/*.md
            Path::new(".claude/agents/test.md"),
            content,
            Position {
                line: 1,
                character: 1,
            },
            &config,
        );
        assert!(
            items.iter().any(|item| item.label == "model"),
            "Agent completions should include 'model', got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hooks_completion_includes_type_field() {
        let content = "{\n  \"typ\n}";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new(".claude/settings.json"),
            content,
            Position {
                line: 1,
                character: 4,
            },
            &config,
        );
        assert!(
            items.iter().any(|item| item.label == "type"),
            "Hooks completions should include 'type', got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_mcp_completion_includes_jsonrpc_field() {
        let content = "{\n  \"json\n}";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new("tools.mcp.json"),
            content,
            Position {
                line: 1,
                character: 4,
            },
            &config,
        );
        assert!(
            items.iter().any(|item| item.label == "jsonrpc"),
            "MCP completions should include 'jsonrpc', got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_copilot_scoped_completion_includes_applyto() {
        let content = "---\napp\n---\n";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new(".github/instructions/ts.instructions.md"),
            content,
            Position {
                line: 1,
                character: 1,
            },
            &config,
        );
        assert!(
            items.iter().any(|item| item.label == "applyTo"),
            "Copilot scoped completions should include 'applyTo', got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cursor_rule_completion_includes_description() {
        let content = "---\ndesc\n---\n";
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new(".cursor/rules/test.mdc"),
            content,
            Position {
                line: 1,
                character: 2,
            },
            &config,
        );
        assert!(
            items.iter().any(|item| item.label == "description"),
            "Cursor rule completions should include 'description', got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_unknown_file_type_has_no_completions() {
        let config = agnix_core::LintConfig::default();
        let items = completion_items_for_document(
            Path::new("README.md"),
            "# readme",
            Position {
                line: 0,
                character: 0,
            },
            &config,
        );
        assert!(items.is_empty());
    }
}
