//! WebAssembly bindings for agnix validation engine.
//!
//! Provides browser-compatible validation of agent configuration files
//! without filesystem dependencies. Used by the web playground.

use agnix_core::{
    Diagnostic, DiagnosticLevel, FileType, LintConfig, ValidatorRegistry, detect_file_type,
    validate_content,
};
use serde::Serialize;
use std::path::Path;
use std::sync::LazyLock;
use wasm_bindgen::prelude::*;

/// Cached validator registry (created once, reused across all validate() calls).
static REGISTRY: LazyLock<ValidatorRegistry> = LazyLock::new(ValidatorRegistry::with_defaults);

/// Initialize panic hook for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[derive(Serialize)]
struct WasmFix {
    start_byte: usize,
    end_byte: usize,
    replacement: String,
    description: String,
    safe: bool,
}

#[derive(Serialize)]
struct WasmDiagnostic {
    level: &'static str,
    rule: String,
    message: String,
    line: usize,
    column: usize,
    suggestion: Option<String>,
    assumption: Option<String>,
    fixes: Vec<WasmFix>,
}

impl WasmDiagnostic {
    fn from_diagnostic(d: &Diagnostic) -> Self {
        Self {
            level: match d.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "info",
            },
            rule: d.rule.clone(),
            message: d.message.clone(),
            line: d.line,
            column: d.column,
            suggestion: d.suggestion.clone(),
            assumption: d.assumption.clone(),
            fixes: d
                .fixes
                .iter()
                .map(|f| WasmFix {
                    start_byte: f.start_byte,
                    end_byte: f.end_byte,
                    replacement: f.replacement.clone(),
                    description: f.description.clone(),
                    safe: f.safe,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct ValidationResponse {
    diagnostics: Vec<WasmDiagnostic>,
    file_type: String,
}

/// Validate agent configuration content.
///
/// # Arguments
/// * `filename` - Filename for file type detection (e.g. "CLAUDE.md")
/// * `content` - File content to validate
/// * `tool` - Optional tool name filter (e.g. "claude-code", "cursor")
///
/// # Returns
/// JSON object with `diagnostics` array and `file_type` string.
/// Returns early with empty diagnostics if content exceeds 1 MiB.
#[wasm_bindgen]
pub fn validate(filename: &str, content: &str, tool: Option<String>) -> JsValue {
    let path = Path::new(filename);
    let detected_type = detect_file_type(path);

    // Reject oversized content to prevent memory exhaustion (1 MiB limit,
    // matching agnix-core's safe_read_file limit).
    if content.len() > 1_048_576 {
        let response = ValidationResponse {
            diagnostics: vec![],
            file_type: detected_type.to_string(),
        };
        return serde_wasm_bindgen::to_value(&response).unwrap_or(JsValue::NULL);
    }

    let mut builder = LintConfig::builder();
    if let Some(ref tool_name) = tool {
        builder.tools(vec![tool_name.clone()]);
    }
    // Use `build_unchecked()` to bypass tool-name validation so the WASM
    // playground can accept newer/unknown tools without a core library update.
    // Only `tools` is set on the builder, which cannot produce unsafe config.
    let config = builder.build_unchecked();

    let diagnostics = validate_content(path, content, &config, &REGISTRY);

    let response = ValidationResponse {
        diagnostics: diagnostics
            .iter()
            .map(WasmDiagnostic::from_diagnostic)
            .collect(),
        file_type: detected_type.to_string(),
    };

    serde_wasm_bindgen::to_value(&response).unwrap_or(JsValue::NULL)
}

/// Get the list of supported file type examples.
///
/// Returns an array of `[filename, file_type]` pairs.
#[wasm_bindgen]
pub fn get_supported_file_types() -> JsValue {
    let types: Vec<(&str, &str)> = vec![
        ("CLAUDE.md", "ClaudeMd"),
        ("AGENTS.md", "ClaudeMd"),
        ("SKILL.md", "Skill"),
        (".cursorrules", "CursorRulesLegacy"),
        (".cursor/rules/example.mdc", "CursorRule"),
        (".cursor/hooks.json", "CursorHooks"),
        (".cursor/agents/reviewer.md", "CursorAgent"),
        (".cursor/environment.json", "CursorEnvironment"),
        (".github/copilot-instructions.md", "Copilot"),
        ("GEMINI.md", "GeminiMd"),
        (".clinerules", "ClineRules"),
        ("CODEX.md", "Codex"),
        (".opencode/instructions.md", "OpenCode"),
        ("mcp.json", "Mcp"),
        (".claude/settings.json", "Hooks"),
    ];

    serde_wasm_bindgen::to_value(&types).unwrap_or(JsValue::NULL)
}

/// Get the list of supported tool names for filtering.
#[wasm_bindgen]
pub fn get_supported_tools() -> JsValue {
    let tools: Vec<(&str, &str)> = vec![
        ("claude-code", "Claude Code"),
        ("cursor", "Cursor"),
        ("github-copilot", "GitHub Copilot"),
        ("codex", "Codex CLI"),
        ("cline", "Cline"),
        ("opencode", "OpenCode"),
        ("gemini-cli", "Gemini CLI"),
        ("roo-code", "Roo Code"),
        ("kiro", "Kiro CLI"),
        ("amp", "amp"),
    ];

    serde_wasm_bindgen::to_value(&tools).unwrap_or(JsValue::NULL)
}

/// Detect the file type for a given filename.
#[wasm_bindgen]
pub fn detect_type(filename: &str) -> String {
    let file_type = detect_file_type(Path::new(filename));
    if file_type == FileType::Unknown {
        String::new()
    } else {
        file_type.to_string()
    }
}
