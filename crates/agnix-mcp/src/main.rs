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
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorData as McpError, Implementation, ProtocolVersion,
        ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Input for validate_file tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(description = "Input for validating a single agent configuration file")]
pub struct ValidateFileInput {
    /// Path to the file to validate
    #[schemars(
        description = "Absolute or relative path to the agent configuration file (e.g., 'SKILL.md', '.claude/settings.json', 'mcp-config.json')"
    )]
    pub path: String,
    /// Target tool for validation rules
    #[schemars(
        description = "Target AI tool for validation rules. Options: 'generic' (default), 'claude-code', 'cursor', 'codex'"
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
    /// Target tool for validation rules
    #[schemars(
        description = "Target AI tool for validation rules. Options: 'generic' (default), 'claude-code', 'cursor', 'codex'"
    )]
    pub target: Option<String>,
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
        config.target = parse_target(input.target);

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
        config.target = parse_target(input.target);

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
                 Target options: generic, claude-code, cursor, codex"
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
