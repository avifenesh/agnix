//! MCP (Model Context Protocol) validation (MCP-001 to MCP-006)

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::Validator,
    schemas::mcp::{validate_json_schema_structure, McpConfigSchema, McpToolSchema},
};
use std::path::Path;

/// Find the line number (1-based) of a JSON field in the raw content
/// Returns (line, column) or (1, 0) if not found
fn find_json_field_location(content: &str, field_name: &str) -> (usize, usize) {
    // Search for "field_name": pattern
    let pattern = format!("\"{}\"", field_name);
    if let Some(pos) = content.find(&pattern) {
        let line = content[..pos].matches('\n').count() + 1;
        let last_newline = content[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let col = pos - last_newline;
        return (line, col);
    }
    (1, 0)
}

/// Find the line number of a tool in a tools array (0-indexed)
fn find_tool_location(content: &str, tool_index: usize) -> (usize, usize) {
    // Find "tools" first, then count opening braces
    if let Some(tools_pos) = content.find("\"tools\"") {
        let after_tools = &content[tools_pos..];
        // Find the opening bracket of the array
        if let Some(bracket_pos) = after_tools.find('[') {
            let after_bracket = &after_tools[bracket_pos + 1..];
            let mut brace_count = 0;
            let mut tool_count = 0;
            let mut in_string = false;
            let mut prev_char = ' ';

            for (i, c) in after_bracket.char_indices() {
                if c == '"' && prev_char != '\\' {
                    in_string = !in_string;
                }
                if !in_string {
                    if c == '{' {
                        if brace_count == 0 {
                            if tool_count == tool_index {
                                // Found our tool - calculate line/col
                                let abs_pos = tools_pos + bracket_pos + 1 + i;
                                let line = content[..abs_pos].matches('\n').count() + 1;
                                let last_newline =
                                    content[..abs_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
                                let col = abs_pos - last_newline;
                                return (line, col);
                            }
                            tool_count += 1;
                        }
                        brace_count += 1;
                    } else if c == '}' {
                        brace_count -= 1;
                    }
                }
                prev_char = c;
            }
        }
    }
    (1, 0)
}

pub struct McpValidator;

impl Validator for McpValidator {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if MCP category is disabled
        if !config.rules.mcp {
            return diagnostics;
        }

        // Try to parse as JSON
        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                diagnostics.push(Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "mcp::parse",
                    format!("Failed to parse MCP configuration: {}", e),
                ));
                return diagnostics;
            }
        };

        // Check for JSON-RPC version (MCP-001)
        if config.is_rule_enabled("MCP-001") {
            validate_jsonrpc_version(&raw_value, path, content, &mut diagnostics);
        }

        // Try to parse as MCP config schema
        let mcp_config: McpConfigSchema = match serde_json::from_value(raw_value.clone()) {
            Ok(config) => config,
            Err(_) => {
                // Not a standard MCP config, may be a tools array or single tool
                // Continue with raw value validation
                McpConfigSchema {
                    mcp_servers: None,
                    tools: None,
                    jsonrpc: None,
                }
            }
        };

        // Get tools array from various locations (also reports parse errors for invalid entries)
        let tools = extract_tools(&raw_value, &mcp_config, path, content, &mut diagnostics);

        // Validate each successfully parsed tool
        for (idx, tool) in tools.iter().enumerate() {
            validate_tool(tool, path, content, config, &mut diagnostics, idx);
        }

        diagnostics
    }
}

/// Extract tools from various MCP config formats, reporting parse errors for invalid entries
fn extract_tools(
    raw_value: &serde_json::Value,
    config: &McpConfigSchema,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<McpToolSchema> {
    let mut tools = Vec::new();

    // Check for tools array in config (preferred source as it's already parsed)
    if let Some(config_tools) = &config.tools {
        tools.extend(config_tools.clone());
        return tools; // Return early to avoid duplication
    }

    // Check for tools array at root level (fallback if config parsing didn't get them)
    if let Some(arr) = raw_value.get("tools").and_then(|v| v.as_array()) {
        for (idx, tool_val) in arr.iter().enumerate() {
            match serde_json::from_value::<McpToolSchema>(tool_val.clone()) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    // Report invalid tool entries instead of silently skipping
                    let (line, col) = find_tool_location(content, idx);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "mcp::invalid_tool",
                            format!("Tool #{}: Invalid tool definition: {}", idx + 1, e),
                        )
                        .with_suggestion(
                            "Ensure tool has valid field types (name: string, description: string, inputSchema: object)".to_string(),
                        ),
                    );
                }
            }
        }
        if !tools.is_empty() || !arr.is_empty() {
            return tools; // Return early (even if some failed to parse)
        }
    }

    // Check if root is a single tool definition (has name OR inputSchema OR description)
    // This allows detecting incomplete tools for validation
    let has_tool_fields = raw_value.get("name").is_some()
        || raw_value.get("inputSchema").is_some()
        || raw_value.get("description").is_some();

    if has_tool_fields {
        match serde_json::from_value::<McpToolSchema>(raw_value.clone()) {
            Ok(tool) => tools.push(tool),
            Err(e) => {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "mcp::invalid_tool",
                        format!("Invalid tool definition: {}", e),
                    )
                    .with_suggestion(
                        "Ensure tool has valid field types (name: string, description: string, inputSchema: object)".to_string(),
                    ),
                );
            }
        }
    }

    tools
}

/// MCP-001: Validate JSON-RPC version is "2.0"
fn validate_jsonrpc_version(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if jsonrpc field exists
    if let Some(jsonrpc) = value.get("jsonrpc") {
        let (line, col) = find_json_field_location(content, "jsonrpc");
        if let Some(version) = jsonrpc.as_str() {
            if version != "2.0" {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-001",
                        format!("Invalid JSON-RPC version '{}', must be '2.0'", version),
                    )
                    .with_suggestion("Set \"jsonrpc\": \"2.0\"".to_string()),
                );
            }
        } else {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-001",
                    "JSON-RPC version must be a string".to_string(),
                )
                .with_suggestion("Set \"jsonrpc\": \"2.0\"".to_string()),
            );
        }
    }
    // Note: jsonrpc field is only required for JSON-RPC messages, not tool definitions
    // So we don't report missing jsonrpc as an error
}

/// Validate a single MCP tool
fn validate_tool(
    tool: &McpToolSchema,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
    tool_index: usize,
) {
    // Always include tool index for clarity, even for first tool
    let tool_prefix = format!("Tool #{}: ", tool_index + 1);

    // Get base location for this tool
    let tool_loc = find_tool_location(content, tool_index);
    // Helper to find field within tool context (searches from beginning for single tools)
    let find_field = |field: &str| -> (usize, usize) {
        let (line, col) = find_json_field_location(content, field);
        if line > 1 || col > 0 {
            (line, col)
        } else {
            tool_loc
        }
    };

    let (has_name, has_desc, has_schema) = tool.has_required_fields();

    // MCP-002: Missing required tool fields
    if config.is_rule_enabled("MCP-002") {
        if !has_name {
            let (line, col) = if tool.name.is_some() {
                find_field("name")
            } else {
                tool_loc
            };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-002",
                    format!("{}Missing required field 'name'", tool_prefix),
                )
                .with_suggestion("Add 'name' field to tool definition".to_string()),
            );
        }
        if !has_desc {
            let (line, col) = if tool.description.is_some() {
                find_field("description")
            } else {
                tool_loc
            };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-002",
                    format!("{}Missing required field 'description'", tool_prefix),
                )
                .with_suggestion("Add 'description' field to tool definition".to_string()),
            );
        }
        if !has_schema {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    tool_loc.0,
                    tool_loc.1,
                    "MCP-002",
                    format!("{}Missing required field 'inputSchema'", tool_prefix),
                )
                .with_suggestion("Add 'inputSchema' field with JSON Schema definition".to_string()),
            );
        }
    }

    // MCP-003: Invalid JSON Schema
    if config.is_rule_enabled("MCP-003") {
        if let Some(schema) = &tool.input_schema {
            let (line, col) = find_field("inputSchema");
            let schema_errors = validate_json_schema_structure(schema);
            for error in schema_errors {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-003",
                        format!("{}Invalid inputSchema: {}", tool_prefix, error),
                    )
                    .with_suggestion("Fix JSON Schema structure".to_string()),
                );
            }
        }
    }

    // MCP-004: Missing or short tool description
    if config.is_rule_enabled("MCP-004") && has_desc && !tool.has_meaningful_description() {
        let (line, col) = find_field("description");
        let desc_len = tool.description.as_ref().map(|d| d.len()).unwrap_or(0);
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-004",
                format!(
                    "{}Tool description is too short ({} chars), should be at least 10 characters",
                    tool_prefix, desc_len
                ),
            )
            .with_suggestion(
                "Add a meaningful description explaining what the tool does".to_string(),
            ),
        );
    }

    // MCP-005: Tool without user consent mechanism
    if config.is_rule_enabled("MCP-005") && !tool.has_consent_fields() {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                tool_loc.0,
                tool_loc.1,
                "MCP-005",
                format!(
                    "{}Tool lacks consent mechanism (no 'requiresApproval' or 'confirmation' field)",
                    tool_prefix
                ),
            )
            .with_suggestion(
                "Consider adding 'requiresApproval: true' for tools that modify data or have side effects".to_string(),
            ),
        );
    }

    // MCP-006: Untrusted annotations
    if config.is_rule_enabled("MCP-006") && tool.has_annotations() {
        let (line, col) = find_field("annotations");
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-006",
                format!(
                    "{}Tool has annotations that should be validated before trusting",
                    tool_prefix
                ),
            )
            .with_suggestion(
                "Ensure annotations are from a trusted source before relying on them".to_string(),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::path::PathBuf;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = McpValidator;
        let path = PathBuf::from("test.mcp.json");
        validator.validate(&path, content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = McpValidator;
        let path = PathBuf::from("test.mcp.json");
        validator.validate(&path, content, config)
    }

    // MCP-001 Tests
    #[test]
    fn test_mcp_001_valid_jsonrpc_version() {
        let content = r#"{"jsonrpc": "2.0", "method": "test"}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    #[test]
    fn test_mcp_001_invalid_jsonrpc_version() {
        let content = r#"{"jsonrpc": "1.0", "method": "test"}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-001"));
        assert!(diagnostics[0].message.contains("Invalid JSON-RPC version"));
    }

    #[test]
    fn test_mcp_001_jsonrpc_not_string() {
        let content = r#"{"jsonrpc": 2.0, "method": "test"}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-001"));
        assert!(diagnostics[0].message.contains("must be a string"));
    }

    #[test]
    fn test_mcp_001_missing_jsonrpc_no_error() {
        // Missing jsonrpc is OK for tool definitions (only required for JSON-RPC messages)
        let content = r#"{"name": "test-tool", "description": "A test tool", "inputSchema": {"type": "object"}}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    // MCP-002 Tests
    #[test]
    fn test_mcp_002_all_fields_present() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    #[test]
    fn test_mcp_002_missing_name() {
        let content = r#"{
            "description": "A test tool",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("Tool #1"));
        assert!(mcp_002[0].message.contains("'name'"));
    }

    #[test]
    fn test_mcp_002_missing_description() {
        let content = r#"{
            "name": "test-tool",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("'description'"));
    }

    #[test]
    fn test_mcp_002_missing_input_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing"
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("'inputSchema'"));
    }

    #[test]
    fn test_mcp_002_empty_name() {
        let content = r#"{
            "name": "",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("'name'"));
    }

    #[test]
    fn test_mcp_002_all_fields_missing() {
        let content = r#"{}"#;
        let diagnostics = validate(content);
        // Empty object won't be detected as a tool, so no MCP-002 errors
        // Tools are detected by having 'name' OR 'inputSchema' at root
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    // MCP-003 Tests
    #[test]
    fn test_mcp_003_valid_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_003_invalid_type_value() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "invalid_type"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Invalid JSON Schema type")));
    }

    #[test]
    fn test_mcp_003_schema_not_object() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": "not an object"
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("must be an object")));
    }

    #[test]
    fn test_mcp_003_properties_not_object() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object", "properties": "not an object"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_003_required_not_array() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object", "required": "not an array"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    // MCP-004 Tests
    #[test]
    fn test_mcp_004_meaningful_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "This is a meaningful description of the tool",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-004"));
    }

    #[test]
    fn test_mcp_004_short_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "Short",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-004"));
        assert!(diagnostics.iter().any(|d| d.message.contains("too short")));
    }

    #[test]
    fn test_mcp_004_empty_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        // Empty description triggers MCP-002 (missing), not MCP-004
        // MCP-004 only triggers when description exists but is short
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    // MCP-005 Tests
    #[test]
    fn test_mcp_005_has_requires_approval() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "requiresApproval": true
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-005"));
    }

    #[test]
    fn test_mcp_005_has_confirmation() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "confirmation": "Are you sure?"
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-005"));
    }

    #[test]
    fn test_mcp_005_missing_consent() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-005"));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("consent mechanism")));
    }

    #[test]
    fn test_mcp_005_requires_approval_false_triggers_warning() {
        // requiresApproval: false should still trigger MCP-005 warning
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "requiresApproval": false
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-005"));
    }

    #[test]
    fn test_mcp_005_empty_confirmation_triggers_warning() {
        // Empty confirmation should still trigger MCP-005 warning
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "confirmation": ""
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-005"));
    }

    // MCP-006 Tests
    #[test]
    fn test_mcp_006_no_annotations() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-006"));
    }

    #[test]
    fn test_mcp_006_has_annotations() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {"untrusted": "data"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-006"));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("annotations")));
    }

    #[test]
    fn test_mcp_006_empty_annotations() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {}
        }"#;
        let diagnostics = validate(content);
        // Empty annotations don't trigger warning
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-006"));
    }

    // Config wiring tests
    #[test]
    fn test_config_disabled_mcp_category() {
        let mut config = LintConfig::default();
        config.rules.mcp = false;

        let content = r#"{"jsonrpc": "1.0"}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["MCP-001".to_string()];

        let content = r#"{"jsonrpc": "1.0"}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    // Parse error test
    #[test]
    fn test_parse_error_handling() {
        let content = r#"not valid json"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "mcp::parse"));
    }

    // Multiple tools test
    #[test]
    fn test_multiple_tools_validation() {
        let content = r#"{
            "tools": [
                {"name": "tool1", "description": "First tool description", "inputSchema": {"type": "object"}},
                {"name": "", "description": "Second tool", "inputSchema": {"type": "object"}}
            ]
        }"#;
        let diagnostics = validate(content);
        // First tool is valid, second tool has empty name
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("Tool #2"));
    }

    // Tools array at root level
    #[test]
    fn test_tools_array_format() {
        let content = r#"{
            "tools": [
                {"name": "tool1", "description": "A tool for testing purposes", "inputSchema": {"type": "object"}, "requiresApproval": true}
            ]
        }"#;
        let diagnostics = validate(content);
        // Only MCP-005 warnings should be present (if consent fields missing)
        // In this case, tool has requiresApproval, so no MCP-005
        let errors = diagnostics
            .iter()
            .filter(|d| d.level == crate::diagnostics::DiagnosticLevel::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty());
    }

    // MCP server config format (should not trigger tool validation)
    #[test]
    fn test_mcp_server_config_format() {
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        // Server config doesn't have tools, no tool validation errors
        assert!(diagnostics.is_empty());
    }
}
