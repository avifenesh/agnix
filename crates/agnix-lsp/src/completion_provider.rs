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
) -> Vec<CompletionItem> {
    let file_type = agnix_core::detect_file_type(path);
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
        let items = completion_items_for_document(
            Path::new("SKILL.md"),
            content,
            Position {
                line: 1,
                character: 1,
            },
        );
        assert!(items.iter().any(|item| item.label == "name"));
    }

    #[test]
    fn test_unknown_file_type_has_no_completions() {
        let items = completion_items_for_document(
            Path::new("README.md"),
            "# readme",
            Position {
                line: 0,
                character: 0,
            },
        );
        assert!(items.is_empty());
    }
}
