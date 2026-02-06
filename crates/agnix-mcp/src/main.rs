//! MCP server for agnix - AI agent config linter
//!
//! Exposes agnix validation as MCP tools for AI assistants.
//!
//! ## MCP Best Practices Implemented
//!
//! - **Clear tool descriptions**: Each tool has a detailed description explaining
//!   what it does, when to use it, and what it returns
//! - **Rich parameter schemas**: All parameters have descriptions with examples
//! - **Structured outputs**: Returns JSON with predictable schema for easy parsing
//! - **Error handling**: Proper error messages with context
//! - **Server metadata**: Provides name, version, and usage instructions

use agnix_core::{
    config::LintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    validate_file as core_validate_file, validate_project as core_validate_project,
};
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

const TOOL_ALIASES: &[(&str, &str)] =
    &[("copilot", "github-copilot"), ("claudecode", "claude-code")];

const COMPAT_TOOL_NAMES: &[&str] = &["generic", "codex"];

/// Input for validate_file tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Input for validating a single agent configuration file")]
pub struct ValidateFileInput {
    /// Path to the file to validate
    #[schemars(
        description = "Absolute or relative path to the agent configuration file (e.g., 'SKILL.md', '.claude/settings.json', 'mcp-config.json')"
    )]
    pub path: String,
    /// Tools to validate for (preferred over legacy target)
    #[schemars(
        description = "Tools to validate for. Accepts comma-separated string (e.g., 'claude-code,cursor,windsurf') or array (e.g., ['claude-code','cursor']). Uses canonical agnix tool names (case-insensitive), plus compatibility aliases (e.g., 'copilot', 'claudecode'). When non-empty, this overrides legacy target."
    )]
    pub tools: Option<ToolsInput>,
    /// Target tool for validation rules
    #[schemars(
        description = "Legacy single target for validation rules (deprecated). Options: 'generic' (default), 'claude-code', 'cursor', 'codex'. Used only when 'tools' is missing or empty."
    )]
    pub target: Option<String>,
}

/// Input for validate_project tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Input for validating all agent configs in a project directory")]
pub struct ValidateProjectInput {
    /// Path to the project directory
    #[schemars(
        description = "Path to the project directory to validate (e.g., '.' for current directory)"
    )]
    pub path: String,
    /// Tools to validate for (preferred over legacy target)
    #[schemars(
        description = "Tools to validate for. Accepts comma-separated string (e.g., 'claude-code,cursor,windsurf') or array (e.g., ['claude-code','cursor']). Uses canonical agnix tool names (case-insensitive), plus compatibility aliases (e.g., 'copilot', 'claudecode'). When non-empty, this overrides legacy target."
    )]
    pub tools: Option<ToolsInput>,
    /// Target tool for validation rules
    #[schemars(
        description = "Legacy single target for validation rules (deprecated). Options: 'generic' (default), 'claude-code', 'cursor', 'codex'. Used only when 'tools' is missing or empty."
    )]
    pub target: Option<String>,
}

/// Tools input for MCP validate tools.
///
/// Supports either comma-separated string or array syntax.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum ToolsInput {
    Csv(String),
    List(Vec<String>),
}

/// Input for get_rule_docs tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Input for looking up a specific validation rule")]
pub struct GetRuleDocsInput {
    /// Rule ID
    #[schemars(
        description = "Rule ID to look up documentation for. Format: PREFIX-NUMBER (e.g., 'AS-004', 'CC-SK-001', 'PE-003', 'MCP-001')"
    )]
    pub rule_id: String,
}

/// Diagnostic output for JSON serialization
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct DiagnosticOutput {
    /// File path where the issue was found
    file: String,
    /// Line number (1-based)
    line: usize,
    /// Column number (1-based)
    column: usize,
    /// Severity level: error, warning, or info
    level: String,
    /// Rule ID (e.g., AS-004)
    rule: String,
    /// Human-readable message describing the issue
    message: String,
    /// Suggested fix or help text
    suggestion: Option<String>,
    /// Whether this issue can be auto-fixed
    fixable: bool,
}

impl From<&Diagnostic> for DiagnosticOutput {
    fn from(d: &Diagnostic) -> Self {
        Self {
            file: d.file.display().to_string(),
            line: d.line,
            column: d.column,
            level: match d.level {
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Info => "info",
            }
            .to_string(),
            rule: d.rule.clone(),
            message: d.message.clone(),
            suggestion: d.suggestion.clone(),
            fixable: !d.fixes.is_empty(),
        }
    }
}

/// Validation result output
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ValidationResult {
    /// Path that was validated
    path: String,
    /// Number of files checked
    files_checked: usize,
    /// Number of errors found
    errors: usize,
    /// Number of warnings found
    warnings: usize,
    /// Number of issues that can be auto-fixed
    fixable: usize,
    /// List of diagnostics
    diagnostics: Vec<DiagnosticOutput>,
}

/// Rule info for listing
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct RuleInfo {
    /// Rule ID (e.g., AS-004)
    id: String,
    /// Human-readable name
    name: String,
}

/// Rules list output
#[derive(Debug, Serialize, schemars::JsonSchema)]
struct RulesListOutput {
    /// Total number of rules
    count: usize,
    /// List of rules
    rules: Vec<RuleInfo>,
}

fn parse_target(target: Option<String>) -> agnix_core::config::TargetTool {
    use agnix_core::config::TargetTool;

    match target.as_deref() {
        Some("claude-code") | Some("claudecode") => TargetTool::ClaudeCode,
        Some("cursor") => TargetTool::Cursor,
        Some("codex") => TargetTool::Codex,
        _ => TargetTool::Generic,
    }
}

fn normalize_tool_entry(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

fn canonicalize_tool(value: &str) -> Option<&'static str> {
    match value {
        v if v.eq_ignore_ascii_case("generic") => Some("generic"),
        v if v.eq_ignore_ascii_case("codex") => Some("codex"),
        _ => TOOL_ALIASES
            .iter()
            .find(|(alias, _)| value.eq_ignore_ascii_case(alias))
            .map(|(_, canonical)| *canonical)
            .or_else(|| agnix_rules::normalize_tool_name(value)),
    }
}

fn supported_tool_names() -> Vec<&'static str> {
    let mut tools = agnix_rules::valid_tools().to_vec();
    for compat in COMPAT_TOOL_NAMES {
        if !tools.contains(compat) {
            tools.push(compat);
        }
    }
    tools.sort_unstable();
    tools
}

fn alias_help() -> String {
    TOOL_ALIASES
        .iter()
        .map(|(alias, canonical)| format!("{} -> {}", alias, canonical))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_tools(tools: Option<ToolsInput>) -> Result<Vec<String>, McpError> {
    let raw: Vec<String> = match tools {
        None => Vec::new(),
        Some(ToolsInput::Csv(csv)) => csv.split(',').filter_map(normalize_tool_entry).collect(),
        Some(ToolsInput::List(list)) => list
            .into_iter()
            .filter_map(|entry| normalize_tool_entry(&entry))
            .collect(),
    };

    if raw.is_empty() {
        return Ok(Vec::new());
    }

    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for tool in raw {
        let canonical = canonicalize_tool(&tool).ok_or_else(|| {
            make_invalid_params(format!(
                "Unknown tool '{}'. Valid values: {}. Aliases: {}.",
                tool,
                supported_tool_names().join(", "),
                alias_help()
            ))
        })?;
        if seen.insert(canonical) {
            normalized.push(canonical.to_string());
        }
    }

    Ok(normalized)
}

fn apply_tool_selection(
    config: &mut LintConfig,
    tools: Option<ToolsInput>,
    target: Option<String>,
) -> Result<(), McpError> {
    let parsed_tools = parse_tools(tools)?;
    if parsed_tools.is_empty() {
        config.tools.clear();
        config.target = parse_target(target);
    } else {
        config.target = agnix_core::config::TargetTool::Generic;
        config.tools = parsed_tools;
    }

    Ok(())
}

fn diagnostics_to_result(
    path: &str,
    diagnostics: Vec<Diagnostic>,
    files_checked: usize,
) -> ValidationResult {
    let errors = diagnostics
        .iter()
        .filter(|d| matches!(d.level, DiagnosticLevel::Error))
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| matches!(d.level, DiagnosticLevel::Warning))
        .count();
    let fixable = diagnostics.iter().filter(|d| !d.fixes.is_empty()).count();

    ValidationResult {
        path: path.to_string(),
        files_checked,
        errors,
        warnings,
        fixable,
        diagnostics: diagnostics.iter().map(DiagnosticOutput::from).collect(),
    }
}

fn make_error(msg: String) -> McpError {
    McpError::internal_error(msg, None::<Value>)
}

fn make_invalid_params(msg: String) -> McpError {
    McpError::invalid_params(msg, None::<Value>)
}

/// Agnix MCP Server - validates AI agent configurations
///
/// Provides tools to validate SKILL.md, CLAUDE.md, AGENTS.md, hooks,
/// MCP configs, and more against 100 rules.
#[derive(Debug, Clone)]
pub struct AgnixServer {
    tool_router: ToolRouter<AgnixServer>,
}

impl Default for AgnixServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl AgnixServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Validate a single agent configuration file
    #[tool(
        description = "Validate a single agent configuration file against agnix rules. Supports SKILL.md, CLAUDE.md, AGENTS.md, hooks.json, *.mcp.json, .cursor/rules/*.mdc, and other agent config files. Returns diagnostics with errors, warnings, auto-fix suggestions, and rule IDs for lookup."
    )]
    async fn validate_file(
        &self,
        Parameters(input): Parameters<ValidateFileInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut config = LintConfig::default();
        apply_tool_selection(&mut config, input.tools, input.target)?;

        let file_path = Path::new(&input.path);

        let diagnostics = core_validate_file(file_path, &config)
            .map_err(|e| make_error(format!("Failed to validate file: {}", e)))?;

        let result = diagnostics_to_result(&input.path, diagnostics, 1);
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| make_error(format!("Failed to serialize result: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Validate all agent configuration files in a project directory
    #[tool(
        description = "Validate all agent configuration files in a project directory. Recursively finds and validates SKILL.md, CLAUDE.md, AGENTS.md, hooks, MCP configs, Cursor rules, and more. Returns aggregated diagnostics for all files."
    )]
    async fn validate_project(
        &self,
        Parameters(input): Parameters<ValidateProjectInput>,
    ) -> Result<CallToolResult, McpError> {
        let mut config = LintConfig::default();
        apply_tool_selection(&mut config, input.tools, input.target)?;

        let validation_result = core_validate_project(Path::new(&input.path), &config)
            .map_err(|e| make_error(format!("Failed to validate project: {}", e)))?;

        let result = diagnostics_to_result(
            &input.path,
            validation_result.diagnostics,
            validation_result.files_checked,
        );
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| make_error(format!("Failed to serialize result: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get all available validation rules
    #[tool(
        description = "List all 100 validation rules available in agnix. Returns rule IDs and names organized by category (AS-* Agent Skills, CC-* Claude Code, MCP-* Model Context Protocol, COP-* Copilot, CUR-* Cursor, etc.)."
    )]
    async fn get_rules(&self) -> Result<CallToolResult, McpError> {
        let rules: Vec<RuleInfo> = agnix_rules::RULES_DATA
            .iter()
            .map(|(id, name)| RuleInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
            })
            .collect();

        let output = RulesListOutput {
            count: rules.len(),
            rules,
        };

        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| make_error(format!("Failed to serialize rules: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Get documentation for a specific rule
    #[tool(
        description = "Get the name of a specific validation rule by ID. Rule IDs follow patterns like AS-004 (Agent Skills), CC-SK-001 (Claude Code Skills), PE-003 (Prompt Engineering), MCP-001 (Model Context Protocol)."
    )]
    async fn get_rule_docs(
        &self,
        Parameters(input): Parameters<GetRuleDocsInput>,
    ) -> Result<CallToolResult, McpError> {
        let name = agnix_rules::get_rule_name(&input.rule_id).ok_or_else(|| {
            make_invalid_params(format!(
                "Rule not found: {}. Use get_rules to list all available rules.",
                input.rule_id
            ))
        })?;

        let output = RuleInfo {
            id: input.rule_id,
            name: name.to_string(),
        };

        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| make_error(format!("Failed to serialize rule: {}", e)))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for AgnixServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "agnix".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            instructions: Some(
                "Agnix - AI agent configuration linter.\n\n\
                 Validates SKILL.md, CLAUDE.md, AGENTS.md, hooks, MCP configs, \
                 Cursor rules, and more against 100 rules.\n\n\
                 Tools:\n\
                 - validate_project: Validate all agent configs in a directory\n\
                 - validate_file: Validate a single config file\n\
                 - get_rules: List all 100 validation rules\n\
                 - get_rule_docs: Get details about a specific rule\n\n\
                 Preferred input: tools (CSV string or array)\n\
                 Legacy fallback: target\n\
                 Supported tools are derived from agnix rule metadata"
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (stdout is for MCP protocol)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Create and run MCP server on stdio
    let server = AgnixServer::new();
    let service = server.serve(stdio()).await?;

    // Wait for shutdown
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        ToolsInput, ValidateFileInput, ValidateProjectInput, apply_tool_selection, parse_tools,
    };
    use agnix_core::LintConfig;
    use agnix_core::config::TargetTool;
    use serde_json::json;

    #[test]
    fn test_parse_tools_csv_trims_and_discards_empty_entries() {
        let tools = parse_tools(Some(ToolsInput::Csv(
            "claude-code, cursor, ,codex,, ".to_string(),
        )))
        .expect("valid tools should parse");
        assert_eq!(tools, vec!["claude-code", "cursor", "codex"]);
    }

    #[test]
    fn test_parse_tools_array_trims_and_discards_empty_entries() {
        let tools = parse_tools(Some(ToolsInput::List(vec![
            " claude-code ".to_string(),
            "".to_string(),
            " cursor".to_string(),
            "   ".to_string(),
        ])))
        .expect("valid tools should parse");
        assert_eq!(tools, vec!["claude-code", "cursor"]);
    }

    #[test]
    fn test_parse_tools_canonicalizes_and_deduplicates_entries() {
        let tools = parse_tools(Some(ToolsInput::List(vec![
            "copilot".to_string(),
            "github-copilot".to_string(),
            "claudecode".to_string(),
            "claude-code".to_string(),
            "cursor".to_string(),
            "CURSOR".to_string(),
        ])))
        .expect("valid tools should parse");
        assert_eq!(tools, vec!["github-copilot", "claude-code", "cursor"]);
    }

    #[test]
    fn test_parse_tools_allows_compat_tool_names() {
        let tools = parse_tools(Some(ToolsInput::Csv("generic,codex".to_string())))
            .expect("generic and codex should be accepted for compatibility");
        assert_eq!(tools, vec!["generic", "codex"]);
    }

    #[test]
    fn test_parse_tools_rejects_unknown_tools() {
        let result = parse_tools(Some(ToolsInput::List(vec!["claud-code".to_string()])));
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_tool_selection_falls_back_to_target_when_tools_empty() {
        let mut config = LintConfig::default();
        apply_tool_selection(
            &mut config,
            Some(ToolsInput::Csv(" , ".to_string())),
            Some("cursor".to_string()),
        )
        .expect("empty tools should fall back to target");

        assert!(config.tools.is_empty());
        assert_eq!(config.target, TargetTool::Cursor);
    }

    #[test]
    fn test_apply_tool_selection_falls_back_to_target_when_tools_missing() {
        let mut config = LintConfig::default();
        apply_tool_selection(&mut config, None, Some("claude-code".to_string()))
            .expect("missing tools should fall back to target");

        assert!(config.tools.is_empty());
        assert_eq!(config.target, TargetTool::ClaudeCode);
    }

    #[test]
    fn test_apply_tool_selection_falls_back_to_target_when_tools_empty_list() {
        let mut config = LintConfig::default();
        apply_tool_selection(
            &mut config,
            Some(ToolsInput::List(vec![])),
            Some("codex".to_string()),
        )
        .expect("empty list should fall back to target");

        assert!(config.tools.is_empty());
        assert_eq!(config.target, TargetTool::Codex);
    }

    #[test]
    fn test_apply_tool_selection_clears_existing_tools_on_fallback() {
        let mut config = LintConfig::default();
        config.tools = vec!["cursor".to_string()];

        apply_tool_selection(
            &mut config,
            Some(ToolsInput::Csv(" ".to_string())),
            Some("claude-code".to_string()),
        )
        .expect("empty tools should trigger fallback and clear stale tools");

        assert!(config.tools.is_empty());
        assert_eq!(config.target, TargetTool::ClaudeCode);
    }

    #[test]
    fn test_apply_tool_selection_prefers_tools_over_target() {
        let mut config = LintConfig::default();
        config.target = TargetTool::Cursor;
        apply_tool_selection(
            &mut config,
            Some(ToolsInput::Csv("claude-code,cursor".to_string())),
            Some("codex".to_string()),
        )
        .expect("valid tools should override target");

        assert_eq!(config.tools, vec!["claude-code", "cursor"]);
        // target remains default; tools array drives filtering precedence in core.
        assert_eq!(config.target, TargetTool::Generic);
    }

    #[test]
    fn test_apply_tool_selection_rejects_unknown_tools() {
        let mut config = LintConfig::default();
        let result = apply_tool_selection(
            &mut config,
            Some(ToolsInput::Csv("unknown-tool".to_string())),
            Some("claude-code".to_string()),
        );
        assert!(result.is_err());
        assert!(config.tools.is_empty());
        assert_eq!(config.target, TargetTool::Generic);
    }

    #[test]
    fn test_validate_file_input_deserializes_csv_tools_payload() {
        let input: ValidateFileInput = serde_json::from_value(json!({
            "path": "SKILL.md",
            "tools": "claude-code,cursor",
            "target": "codex"
        }))
        .expect("tools CSV payload should deserialize");

        match input.tools {
            Some(ToolsInput::Csv(value)) => assert_eq!(value, "claude-code,cursor"),
            _ => panic!("expected CSV tools variant"),
        }
        assert_eq!(input.target.as_deref(), Some("codex"));
    }

    #[test]
    fn test_validate_file_input_deserializes_array_tools_payload() {
        let input: ValidateFileInput = serde_json::from_value(json!({
            "path": "SKILL.md",
            "tools": ["claude-code", "cursor"]
        }))
        .expect("tools array payload should deserialize");

        match input.tools {
            Some(ToolsInput::List(values)) => {
                assert_eq!(values, vec!["claude-code", "cursor"]);
            }
            _ => panic!("expected array tools variant"),
        }
        assert!(input.target.is_none());
    }

    #[test]
    fn test_validate_project_input_deserializes_csv_tools_payload() {
        let input: ValidateProjectInput = serde_json::from_value(json!({
            "path": ".",
            "tools": "claude-code,cursor"
        }))
        .expect("project CSV tools payload should deserialize");

        match input.tools {
            Some(ToolsInput::Csv(value)) => assert_eq!(value, "claude-code,cursor"),
            _ => panic!("expected CSV tools variant"),
        }
    }

    #[test]
    fn test_validate_project_input_deserializes_array_tools_payload() {
        let input: ValidateProjectInput = serde_json::from_value(json!({
            "path": ".",
            "tools": ["claude-code", "cursor"]
        }))
        .expect("project array tools payload should deserialize");

        match input.tools {
            Some(ToolsInput::List(values)) => {
                assert_eq!(values, vec!["claude-code", "cursor"]);
            }
            _ => panic!("expected array tools variant"),
        }
    }
}
