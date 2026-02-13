use super::*;
use std::path::{Component, Path, PathBuf};

pub(super) fn create_error_diagnostic(code: &str, message: String) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code.to_string())),
        code_description: None,
        source: Some("agnix".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Normalize path components without filesystem access.
/// Resolves `.` and `..` logically -- used when `canonicalize()` fails.
/// Expects absolute paths (LSP URIs always produce absolute paths).
pub(super) fn normalize_path(path: &Path) -> PathBuf {
    let mut components: Vec<Component<'_>> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                match components.last() {
                    Some(Component::Normal(_)) => {
                        components.pop();
                    }
                    // Cannot traverse above root or prefix -- silently drop
                    Some(Component::RootDir) | Some(Component::Prefix(_)) => {}
                    _ => components.push(component),
                }
            }
            _ => components.push(component),
        }
    }
    components.iter().collect()
}

impl Backend {
    /// Check if a file path is relevant to project-level rules.
    ///
    /// Returns true for instruction files (CLAUDE.md, AGENTS.md, .clinerules,
    /// .cursorrules, copilot-instructions.md, etc.) and .agnix.toml config.
    pub(super) fn is_project_level_trigger(path: &Path) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        // .agnix.toml config changes affect all rules
        if file_name.eq_ignore_ascii_case(".agnix.toml") {
            return true;
        }

        // Instruction files that affect project-level cross-file checks
        file_name.eq_ignore_ascii_case("claude.md")
            || file_name.eq_ignore_ascii_case("claude.local.md")
            || file_name.eq_ignore_ascii_case("agents.md")
            || file_name.eq_ignore_ascii_case("agents.local.md")
            || file_name.eq_ignore_ascii_case("agents.override.md")
            || file_name.eq_ignore_ascii_case("gemini.md")
            || file_name.eq_ignore_ascii_case("gemini.local.md")
            || file_name.eq_ignore_ascii_case(".clinerules")
            || file_name.eq_ignore_ascii_case(".cursorrules")
            || file_name.eq_ignore_ascii_case(".cursorrules.md")
            || file_name.eq_ignore_ascii_case("copilot-instructions.md")
            || file_name.to_lowercase().ends_with(".instructions.md")
            || file_name.to_lowercase().ends_with(".mdc")
            || file_name.eq_ignore_ascii_case("opencode.json")
    }

    /// Get cached document content for a URI.
    pub(super) async fn get_document_content(&self, uri: &Url) -> Option<Arc<String>> {
        self.documents.read().await.get(uri).cloned()
    }
}
