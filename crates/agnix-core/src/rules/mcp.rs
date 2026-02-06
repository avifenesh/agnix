//! MCP (Model Context Protocol) validation (MCP-001 to MCP-006)

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::Validator,
    schemas::mcp::{
        McpConfigSchema, McpToolSchema, extract_request_protocol_version,
        extract_response_protocol_version, is_initialize_message, is_initialize_response,
        validate_json_schema_structure,
    },
};
use regex::Regex;
use rust_i18n::t;
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

/// Find a unique value span for a JSON scalar key (string/number/bool/null).
/// Returns the full value span (including quotes for strings).
fn find_unique_json_scalar_value_span(content: &str, key: &str) -> Option<(usize, usize)> {
    let pattern = format!(
        r#"("{}"\s*:\s*)((?:"[^"]*")|(?:-?\d+(?:\.\d+)?)|(?:true|false|null))"#,
        regex::escape(key)
    );
    let re = Regex::new(&pattern).ok()?;
    let mut captures = re.captures_iter(content);
    let first = captures.next()?;
    if captures.next().is_some() {
        return None;
    }
    let value = first.get(2)?;
    Some((value.start(), value.end()))
}

/// Find a unique string value span for a key with a known current value.
/// Returns the value-only span (without quotes).
fn find_unique_json_string_value_span(
    content: &str,
    key: &str,
    current_value: &str,
) -> Option<(usize, usize)> {
    let pattern = format!(
        r#"("{}"\s*:\s*)"({})""#,
        regex::escape(key),
        regex::escape(current_value)
    );
    let re = Regex::new(&pattern).ok()?;
    let mut captures = re.captures_iter(content);
    let first = captures.next()?;
    if captures.next().is_some() {
        return None;
    }
    let value = first.get(2)?;
    Some((value.start(), value.end()))
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
                if config.is_rule_enabled("MCP-007") {
                    diagnostics.push(Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "MCP-007",
                        t!("rules.mcp_007.message", error = e.to_string()),
                    ));
                }
                return diagnostics;
            }
        };

        // Check for JSON-RPC version (MCP-001)
        if config.is_rule_enabled("MCP-001") {
            validate_jsonrpc_version(&raw_value, path, content, &mut diagnostics);
        }

        // Check for protocol version mismatch (MCP-008)
        if config.is_rule_enabled("MCP-008") {
            validate_protocol_version(&raw_value, path, content, config, &mut diagnostics);
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
                            t!("rules.invalid_tool", num = idx + 1, error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.invalid_tool_suggestion")),
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
                        t!("rules.invalid_tool_single", error = e.to_string()),
                    )
                    .with_suggestion(t!("rules.invalid_tool_suggestion")),
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
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-001",
                    t!("rules.mcp_001.invalid_version", version = version),
                )
                .with_suggestion(t!("rules.mcp_001.suggestion"));

                // Safe auto-fix: enforce jsonrpc: "2.0"
                if let Some((start, end)) = find_unique_json_scalar_value_span(content, "jsonrpc") {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        "\"2.0\"",
                        "Set jsonrpc version to \"2.0\"",
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        } else {
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                line,
                col,
                "MCP-001",
                t!("rules.mcp_001.not_string"),
            )
            .with_suggestion(t!("rules.mcp_001.suggestion"));

            // Safe auto-fix: normalize non-string jsonrpc values to "2.0"
            if let Some((start, end)) = find_unique_json_scalar_value_span(content, "jsonrpc") {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    "\"2.0\"",
                    "Set jsonrpc version to \"2.0\"",
                    true,
                ));
            }

            diagnostics.push(diagnostic);
        }
    }
    // Note: jsonrpc field is only required for JSON-RPC messages, not tool definitions
    // So we don't report missing jsonrpc as an error
}

/// MCP-008: Validate protocol version matches expected version
fn validate_protocol_version(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected_version = config.get_mcp_protocol_version();
    let version_pinned = config.is_mcp_revision_pinned();

    // Check initialize request
    if is_initialize_message(value) {
        if let Some(actual_version) = extract_request_protocol_version(value) {
            if actual_version != expected_version {
                let (line, col) = find_json_field_location(content, "protocolVersion");
                let mut diag = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-008",
                    t!(
                        "rules.mcp_008.message",
                        found = actual_version.as_str(),
                        expected = expected_version
                    ),
                )
                .with_suggestion(t!(
                    "rules.mcp_008.request_suggestion",
                    expected = expected_version
                ));

                if !version_pinned {
                    diag = diag.with_assumption(t!("rules.mcp_008.assumption"));
                }

                // Unsafe auto-fix only when version is explicitly pinned.
                if version_pinned {
                    if let Some((start, end)) = find_unique_json_string_value_span(
                        content,
                        "protocolVersion",
                        actual_version.as_str(),
                    ) {
                        diag = diag.with_fix(Fix::replace(
                            start,
                            end,
                            expected_version,
                            "Align protocolVersion with pinned MCP revision",
                            false,
                        ));
                    }
                }

                diagnostics.push(diag);
            }
        }
    }

    // Check initialize response
    if is_initialize_response(value) {
        if let Some(actual_version) = extract_response_protocol_version(value) {
            if actual_version != expected_version {
                let (line, col) = find_json_field_location(content, "protocolVersion");
                let mut diag = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-008",
                    t!(
                        "rules.mcp_008.message",
                        found = actual_version.as_str(),
                        expected = expected_version
                    ),
                )
                .with_suggestion(t!(
                    "rules.mcp_008.response_suggestion",
                    found = actual_version.as_str(),
                    expected = expected_version
                ));

                if !version_pinned {
                    diag = diag.with_assumption(t!("rules.mcp_008.assumption"));
                }

                // Unsafe auto-fix only when version is explicitly pinned.
                if version_pinned {
                    if let Some((start, end)) = find_unique_json_string_value_span(
                        content,
                        "protocolVersion",
                        actual_version.as_str(),
                    ) {
                        diag = diag.with_fix(Fix::replace(
                            start,
                            end,
                            expected_version,
                            "Align protocolVersion with pinned MCP revision",
                            false,
                        ));
                    }
                }

                diagnostics.push(diag);
            }
        }
    }
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
                    t!("rules.mcp_002.missing_name", prefix = tool_prefix.as_str()),
                )
                .with_suggestion(t!("rules.mcp_002.missing_name_suggestion")),
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
                    t!(
                        "rules.mcp_002.missing_description",
                        prefix = tool_prefix.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_002.missing_description_suggestion")),
            );
        }
        if !has_schema {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    tool_loc.0,
                    tool_loc.1,
                    "MCP-002",
                    t!(
                        "rules.mcp_002.missing_schema",
                        prefix = tool_prefix.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_002.missing_schema_suggestion")),
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
                        t!(
                            "rules.mcp_003.message",
                            prefix = tool_prefix.as_str(),
                            error = error
                        ),
                    )
                    .with_suggestion(t!("rules.mcp_003.suggestion")),
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
                t!(
                    "rules.mcp_004.message",
                    prefix = tool_prefix.as_str(),
                    len = desc_len
                ),
            )
            .with_suggestion(t!("rules.mcp_004.suggestion")),
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
                t!("rules.mcp_005.message", prefix = tool_prefix.as_str()),
            )
            .with_suggestion(t!("rules.mcp_005.suggestion")),
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
                t!("rules.mcp_006.message", prefix = tool_prefix.as_str()),
            )
            .with_suggestion(t!("rules.mcp_006.suggestion")),
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
        let mcp_001 = diagnostics
            .iter()
            .find(|d| d.rule == "MCP-001")
            .expect("MCP-001 should be reported");
        assert!(mcp_001.message.contains("Invalid JSON-RPC version"));
        assert!(mcp_001.has_fixes());
        let fix = &mcp_001.fixes[0];
        assert_eq!(fix.replacement, "\"2.0\"");
        assert!(fix.safe);
    }

    #[test]
    fn test_mcp_001_jsonrpc_not_string() {
        let content = r#"{"jsonrpc": 2.0, "method": "test"}"#;
        let diagnostics = validate(content);
        let mcp_001 = diagnostics
            .iter()
            .find(|d| d.rule == "MCP-001")
            .expect("MCP-001 should be reported");
        assert!(mcp_001.message.contains("must be a string"));
        assert!(mcp_001.has_fixes());
        let fix = &mcp_001.fixes[0];
        assert_eq!(fix.replacement, "\"2.0\"");
        assert!(fix.safe);
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
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Invalid JSON Schema type"))
        );
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
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("must be an object"))
        );
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
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("consent mechanism"))
        );
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
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("annotations"))
        );
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
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
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

    // MCP-008 Tests
    #[test]
    fn test_mcp_008_initialize_request_matching_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-06-18",
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_request_mismatched_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-008")
            .collect::<Vec<_>>();
        assert_eq!(mcp_008.len(), 1);
        assert!(mcp_008[0].message.contains("Protocol version mismatch"));
        assert!(mcp_008[0].message.contains("2024-11-05"));
        assert!(mcp_008[0].message.contains("2025-06-18"));
        assert!(
            !mcp_008[0].has_fixes(),
            "Unpinned protocol mismatch should be suggestion-only"
        );
    }

    #[test]
    fn test_mcp_008_initialize_response_matching_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-06-18",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_response_mismatched_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-008")
            .collect::<Vec<_>>();
        assert_eq!(mcp_008.len(), 1);
        assert!(mcp_008[0].message.contains("Protocol version mismatch"));
    }

    #[test]
    fn test_mcp_008_custom_expected_version() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = Some("2024-11-05".to_string());

        // This should now match
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_disabled_rule() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["MCP-008".to_string()];

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_non_initialize_message_no_error() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_without_protocol_version_no_error() {
        // Missing protocolVersion should not trigger MCP-008 (version negotiation may handle this)
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"clientInfo": {"name": "test"}}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_warning_level() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");
        assert!(mcp_008.is_some());
        assert_eq!(
            mcp_008.unwrap().level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    // ===== Version-Aware MCP-008 Tests =====

    #[test]
    fn test_mcp_008_assumption_when_version_not_pinned() {
        // Create a config where mcp is NOT pinned
        let mut config = LintConfig::default();
        config.mcp_protocol_version = None;
        config.spec_revisions.mcp_protocol = None;
        assert!(!config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should have an assumption note when version not pinned
        assert!(diag.assumption.is_some());
        let assumption = diag.assumption.as_ref().unwrap();
        assert!(assumption.contains("Using default MCP protocol version"));
        assert!(assumption.contains("[spec_revisions]"));
        assert!(
            !diag.has_fixes(),
            "Unpinned protocol mismatch should not emit auto-fix"
        );
    }

    #[test]
    fn test_mcp_008_no_assumption_when_version_pinned_via_spec_revisions() {
        let mut config = LintConfig::default();
        config.spec_revisions.mcp_protocol = Some("2025-06-18".to_string());
        assert!(config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should NOT have an assumption note when version is pinned
        assert!(diag.assumption.is_none());
        assert!(diag.has_fixes(), "Pinned mismatch should emit auto-fix");
        assert_eq!(diag.fixes[0].replacement, "2025-06-18");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_no_assumption_when_version_pinned_via_legacy() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = Some("2025-06-18".to_string());
        assert!(config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should NOT have an assumption note when version is pinned via legacy field
        assert!(diag.assumption.is_none());
        assert!(diag.has_fixes(), "Pinned mismatch should emit auto-fix");
        assert_eq!(diag.fixes[0].replacement, "2025-06-18");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_response_assumption_when_version_not_pinned() {
        let mut config = LintConfig::default();
        config.mcp_protocol_version = None;
        config.spec_revisions.mcp_protocol = None;

        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        assert!(mcp_008.unwrap().assumption.is_some());
    }

    // ===== Additional MCP-007 Parse Error Tests =====

    #[test]
    fn test_mcp_007_invalid_json_syntax() {
        let content = r#"{ invalid json syntax }"#;

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_truncated_json() {
        let content = r#"{"jsonrpc": "2.0", "method": "test"#;

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_empty_file() {
        let content = "";

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_valid_json_no_error() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_disabled() {
        let mut config = LintConfig::default();
        config.rules.disabled_rules = vec!["MCP-007".to_string()];

        let content = r#"{ invalid }"#;
        let validator = McpValidator;
        let diagnostics = validator.validate(Path::new("test.json"), content, &config);

        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    // ===== Additional MCP rule coverage =====

    #[test]
    fn test_mcp_002_nested_tools_array() {
        let content = r#"{
            "tools": [
                { "name": "tool1", "description": "First tool", "inputSchema": {} },
                { "name": "tool2", "description": "Second tool", "inputSchema": {} }
            ]
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    #[test]
    fn test_mcp_003_nested_schema_valid() {
        let content = r#"{
            "tools": [{
                "name": "complex-tool",
                "description": "A tool with nested schema",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "nested": {
                            "type": "object",
                            "properties": {
                                "value": { "type": "string" }
                            }
                        }
                    }
                }
            }]
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_005_requires_approval_at_tool_level_ok() {
        // requiresApproval must be at tool level, not in annotations
        let content = r#"{
            "tools": [{
                "name": "safe-tool",
                "description": "A tool with approval",
                "inputSchema": {},
                "requiresApproval": true
            }]
        }"#;

        let diagnostics = validate(content);
        let mcp_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-005").collect();
        assert!(mcp_005.is_empty());
    }

    #[test]
    fn test_mcp_006_annotations_triggers_warning() {
        // MCP-006 warns when annotations exist (they should be validated before trusting)
        let content = r#"{
            "tools": [{
                "name": "annotated-tool",
                "description": "A tool with annotations",
                "inputSchema": {},
                "requiresApproval": true,
                "annotations": {
                    "dangerous": false
                }
            }]
        }"#;

        let diagnostics = validate(content);
        // MCP-006 SHOULD trigger (annotations present = warning)
        let mcp_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-006").collect();
        assert!(!mcp_006.is_empty(), "MCP-006 should warn about annotations");
    }

    #[test]
    fn test_all_mcp_rules_can_be_disabled() {
        let rules = [
            "MCP-001", "MCP-002", "MCP-003", "MCP-004", "MCP-005", "MCP-006", "MCP-007", "MCP-008",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules.disabled_rules = vec![rule.to_string()];

            // Use content that would trigger the rule
            let content = match rule {
                "MCP-001" => r#"{"jsonrpc": "1.0"}"#,
                "MCP-007" => r#"{ invalid }"#,
                _ => r#"{"tools": [{"name": "t"}]}"#,
            };

            let validator = McpValidator;
            let diagnostics = validator.validate(Path::new("test.json"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }
}
