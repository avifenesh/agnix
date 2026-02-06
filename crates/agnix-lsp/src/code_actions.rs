//! Code action generation for LSP.
//!
//! Converts agnix-core Fix structs into LSP CodeAction responses.
//! Code actions appear as quick-fix lightbulbs in editors.

use agnix_core::Fix;
use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, Diagnostic as LspDiagnostic, TextEdit, Url, WorkspaceEdit,
};

use crate::position::byte_range_to_lsp_range;

/// Convert an agnix-core Fix to an LSP CodeAction.
///
/// Creates a workspace edit that applies the fix's replacement text
/// at the specified byte range.
///
/// # Arguments
///
/// * `uri` - The document URI for the workspace edit
/// * `fix` - The agnix-core Fix containing byte range and replacement
/// * `content` - The document content for byte-to-position conversion
///
/// # Returns
///
/// A CodeAction with a WorkspaceEdit to apply the fix.
pub fn fix_to_code_action(uri: &Url, fix: &Fix, content: &str) -> CodeAction {
    fix_to_code_action_with_diagnostic(uri, fix, content, None)
}

/// Convert an agnix-core Fix to an LSP CodeAction with optional source diagnostic metadata.
pub fn fix_to_code_action_with_diagnostic(
    uri: &Url,
    fix: &Fix,
    content: &str,
    diagnostic: Option<&LspDiagnostic>,
) -> CodeAction {
    let range = byte_range_to_lsp_range(content, fix.start_byte, fix.end_byte);

    let text_edit = TextEdit {
        range,
        new_text: fix.replacement.clone(),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![text_edit]);

    let workspace_edit = WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    };

    CodeAction {
        title: fix.description.clone(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: diagnostic.map(|d| vec![d.clone()]),
        edit: Some(workspace_edit),
        command: None,
        is_preferred: Some(fix.safe),
        disabled: None,
        data: None,
    }
}

/// Convert multiple fixes to code actions.
///
/// Convenience function for converting a slice of fixes.
pub fn fixes_to_code_actions(uri: &Url, fixes: &[Fix], content: &str) -> Vec<CodeAction> {
    fixes
        .iter()
        .map(|fix| fix_to_code_action_with_diagnostic(uri, fix, content, None))
        .collect()
}

/// Convert multiple fixes to code actions and attach the originating diagnostic.
pub fn fixes_to_code_actions_with_diagnostic(
    uri: &Url,
    fixes: &[Fix],
    content: &str,
    diagnostic: &LspDiagnostic,
) -> Vec<CodeAction> {
    fixes
        .iter()
        .map(|fix| fix_to_code_action_with_diagnostic(uri, fix, content, Some(diagnostic)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fix(start: usize, end: usize, replacement: &str, description: &str, safe: bool) -> Fix {
        Fix {
            start_byte: start,
            end_byte: end,
            replacement: replacement.to_string(),
            description: description.to_string(),
            safe,
        }
    }

    #[test]
    fn test_fix_to_code_action_basic() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "name: Invalid Name";
        let fix = make_fix(6, 18, "valid-name", "Replace with valid name", true);

        let action = fix_to_code_action(&uri, &fix, content);

        assert_eq!(action.title, "Replace with valid name");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(action.is_preferred, Some(true));
        assert!(action.edit.is_some());

        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "valid-name");
    }

    #[test]
    fn test_fix_to_code_action_unsafe() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "hello";
        let fix = make_fix(0, 5, "world", "Replace hello with world", false);

        let action = fix_to_code_action(&uri, &fix, content);

        assert_eq!(action.is_preferred, Some(false));
    }

    #[test]
    fn test_fix_to_code_action_insertion() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "name: test";
        // Insert at end
        let fix = make_fix(10, 10, "\nversion: 1.0.0", "Add version field", true);

        let action = fix_to_code_action(&uri, &fix, content);

        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits[0].range.start, edits[0].range.end);
        assert_eq!(edits[0].new_text, "\nversion: 1.0.0");
    }

    #[test]
    fn test_fix_to_code_action_deletion() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "hello world";
        // Delete " world"
        let fix = make_fix(5, 11, "", "Remove world", true);

        let action = fix_to_code_action(&uri, &fix, content);

        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        assert_eq!(edits[0].new_text, "");
    }

    #[test]
    fn test_fixes_to_code_actions_multiple() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "hello\nworld";
        let fixes = vec![
            make_fix(0, 5, "hi", "Replace hello", true),
            make_fix(6, 11, "earth", "Replace world", false),
        ];

        let actions = fixes_to_code_actions(&uri, &fixes, content);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Replace hello");
        assert_eq!(actions[0].is_preferred, Some(true));
        assert_eq!(actions[1].title, "Replace world");
        assert_eq!(actions[1].is_preferred, Some(false));
    }

    #[test]
    fn test_fixes_to_code_actions_empty() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "hello";
        let fixes: Vec<Fix> = vec![];

        let actions = fixes_to_code_actions(&uri, &fixes, content);

        assert!(actions.is_empty());
    }

    #[test]
    fn test_fix_to_code_action_multiline() {
        let uri = Url::parse("file:///test.md").unwrap();
        let content = "---\nname: test\n---";
        // Replace the entire YAML frontmatter
        let fix = make_fix(0, 18, "---\nname: fixed\n---", "Fix frontmatter", true);

        let action = fix_to_code_action(&uri, &fix, content);

        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let edits = changes.get(&uri).unwrap();
        // Start should be at beginning
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(edits[0].range.start.character, 0);
        // End should be at end of "---" on line 2
        assert_eq!(edits[0].range.end.line, 2);
        assert_eq!(edits[0].range.end.character, 3);
    }
}
