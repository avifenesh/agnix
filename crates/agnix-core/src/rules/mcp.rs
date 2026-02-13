//! MCP (Model Context Protocol) validation (MCP-001 to MCP-024)

use crate::{
    config::LintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::mcp::{
        McpConfigSchema, McpServerConfig, McpToolSchema, VALID_MCP_ANNOTATION_HINTS,
        VALID_MCP_CAPABILITY_KEYS, VALID_MCP_SERVER_TYPES, extract_request_protocol_version,
        extract_response_protocol_version, is_initialize_message, is_initialize_response,
        validate_json_schema_structure,
    },
};
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
    crate::span_utils::find_unique_json_scalar_span(content, key)
}

use super::find_unique_json_string_value_span;

fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, b) in content.bytes().enumerate() {
        if b == b'\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut low = 0usize;
    let mut high = line_starts.len();
    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    let line_start = line_starts[low];
    (low + 1, offset.saturating_sub(line_start))
}

/// Collect object spans for entries in the first `tools` array.
/// Spans are byte offsets `(start, end)` for each object entry.
fn collect_tools_array_object_spans(content: &str) -> Vec<(usize, usize)> {
    let Some(tools_pos) = content.find("\"tools\"") else {
        return Vec::new();
    };
    let Some(array_rel_pos) = content[tools_pos..].find('[') else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    let array_start = tools_pos + array_rel_pos + 1;
    let mut in_string = false;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let mut current_start: Option<usize> = None;

    for (rel_idx, ch) in content[array_start..].char_indices() {
        let abs_idx = array_start + rel_idx;

        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if brace_depth == 0 {
                    current_start = Some(abs_idx);
                }
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0
                    && let Some(start) = current_start.take()
                {
                    spans.push((start, abs_idx + ch.len_utf8()));
                }
            }
            ']' if brace_depth == 0 => break,
            _ => {}
        }
    }

    spans
}

fn tool_location_from_span(
    tool_spans: &[(usize, usize)],
    tool_index: usize,
    line_starts: &[usize],
) -> Option<(usize, usize)> {
    tool_spans
        .get(tool_index)
        .map(|(start, _)| line_col_at(*start, line_starts))
}

fn find_json_field_location_in_span(
    content: &str,
    field_name: &str,
    span: (usize, usize),
    line_starts: &[usize],
) -> Option<(usize, usize)> {
    let (start, end) = span;
    if start >= end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        return None;
    }

    let pattern = format!("\"{}\"", field_name);
    content[start..end]
        .find(&pattern)
        .map(|rel| line_col_at(start + rel, line_starts))
}

const RULE_IDS: &[&str] = &[
    "MCP-001", "MCP-002", "MCP-003", "MCP-004", "MCP-005", "MCP-006", "MCP-007", "MCP-008",
    "MCP-009", "MCP-010", "MCP-011", "MCP-012", "MCP-013", "MCP-014", "MCP-015", "MCP-016",
    "MCP-017", "MCP-018", "MCP-019", "MCP-020", "MCP-021", "MCP-022", "MCP-023", "MCP-024",
];

pub struct McpValidator;

impl Validator for McpValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Early return if MCP category is disabled
        if !config.rules().mcp {
            return diagnostics;
        }

        // Try to parse as JSON
        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("MCP-007") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "MCP-007",
                            t!("rules.mcp_007.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.mcp_007.suggestion")),
                    );
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
                    resources: None,
                    prompts: None,
                    capabilities: None,
                    jsonrpc: None,
                }
            }
        };

        let line_starts = compute_line_starts(content);
        let tool_spans = collect_tools_array_object_spans(content);

        // Get tools array from various locations (also reports parse errors for invalid entries)
        let tools = extract_tools(
            &raw_value,
            &mcp_config,
            path,
            &mut diagnostics,
            &tool_spans,
            &line_starts,
        );

        // Validate each successfully parsed tool
        for (idx, tool) in tools.iter().enumerate() {
            validate_tool(
                tool,
                path,
                config,
                &mut diagnostics,
                ToolLookupContext {
                    content,
                    line_starts: &line_starts,
                    tool_span: tool_spans.get(idx).copied(),
                    tool_index: idx,
                },
            );
        }

        // Validate resource and prompt schema requirements.
        validate_resource_definitions(&raw_value, path, content, config, &mut diagnostics);
        validate_prompt_definitions(&raw_value, path, content, config, &mut diagnostics);

        // Validate capability keys and duplicate server names.
        validate_capability_keys(&raw_value, path, content, config, &mut diagnostics);
        if config.is_rule_enabled("MCP-023") {
            validate_duplicate_server_names(path, content, &mut diagnostics);
        }

        // Validate MCP server configurations (MCP-009 to MCP-012, MCP-024)
        if let Some(servers) = &mcp_config.mcp_servers {
            for (name, server) in servers {
                validate_server(name, server, path, content, config, &mut diagnostics);
            }
        }

        diagnostics
    }
}

/// Extract tools from various MCP config formats, reporting parse errors for invalid entries
fn extract_tools(
    raw_value: &serde_json::Value,
    config: &McpConfigSchema,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    tool_spans: &[(usize, usize)],
    line_starts: &[usize],
) -> Vec<McpToolSchema> {
    let mut tools = Vec::new();

    // Check for tools array in config (preferred source as it's already parsed)
    if let Some(config_tools) = &config.tools {
        tools.extend(config_tools.clone());
    } else if let Some(arr) = raw_value.get("tools").and_then(|v| v.as_array()) {
        // Check for tools array at root level (fallback if config parsing didn't get them)
        for (idx, tool_val) in arr.iter().enumerate() {
            match serde_json::from_value::<McpToolSchema>(tool_val.clone()) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    // Report invalid tool entries instead of silently skipping
                    let (line, col) =
                        tool_location_from_span(tool_spans, idx, line_starts).unwrap_or((1, 0));
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
    }

    // tools/list response shape: {"result": {"tools": [...]}}
    if let Some(arr) = raw_value
        .get("result")
        .and_then(|v| v.get("tools"))
        .and_then(|v| v.as_array())
    {
        let start = tools.len();
        for (idx, tool_val) in arr.iter().enumerate() {
            match serde_json::from_value::<McpToolSchema>(tool_val.clone()) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    let (line, col) = tool_location_from_span(tool_spans, start + idx, line_starts)
                        .unwrap_or((1, 0));
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
    }

    if !tools.is_empty() {
        return tools;
    }

    // Check if root is a single tool definition (has name OR inputSchema OR description)
    // This allows detecting incomplete tools for validation
    let has_tool_fields = raw_value.get("name").is_some()
        || raw_value.get("inputSchema").is_some()
        || raw_value.get("description").is_some()
        || raw_value.get("title").is_some()
        || raw_value.get("outputSchema").is_some()
        || raw_value.get("icons").is_some();

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

fn iter_root_or_result_array<'a>(
    value: &'a serde_json::Value,
    key: &str,
) -> Vec<&'a Vec<serde_json::Value>> {
    let mut arrays = Vec::new();
    if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
        arrays.push(arr);
    }
    if let Some(arr) = value
        .get("result")
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_array())
    {
        arrays.push(arr);
    }
    arrays
}

fn is_non_empty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

fn validate_resource_definitions(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for resources in iter_root_or_result_array(value, "resources") {
        for (idx, resource) in resources.iter().enumerate() {
            let Some(obj) = resource.as_object() else {
                continue;
            };

            let resource_prefix = format!("Resource #{}: ", idx + 1);
            if config.is_rule_enabled("MCP-015") {
                if !is_non_empty_string(obj.get("uri")) {
                    let (line, col) = find_json_field_location(content, "resources");
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "MCP-015",
                            format!("{}missing required field 'uri'", resource_prefix),
                        )
                        .with_suggestion("Add a non-empty URI to the resource definition"),
                    );
                }
                if !is_non_empty_string(obj.get("name")) {
                    let (line, col) = find_json_field_location(content, "resources");
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "MCP-015",
                            format!("{}missing required field 'name'", resource_prefix),
                        )
                        .with_suggestion("Add a non-empty name to the resource definition"),
                    );
                }
            }
        }
    }
}

fn validate_prompt_definitions(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for prompts in iter_root_or_result_array(value, "prompts") {
        for (idx, prompt) in prompts.iter().enumerate() {
            let Some(obj) = prompt.as_object() else {
                continue;
            };

            let prompt_prefix = format!("Prompt #{}: ", idx + 1);
            let (line, col) = find_json_field_location(content, "prompts");

            if config.is_rule_enabled("MCP-016") && !is_non_empty_string(obj.get("name")) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-016",
                        format!("{}missing required field 'name'", prompt_prefix.as_str()),
                    )
                    .with_suggestion("Add a non-empty prompt name"),
                );
            }
        }
    }
}

fn validate_capability_keys(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("MCP-020") {
        return;
    }

    let Some(caps_obj) = value
        .get("capabilities")
        .and_then(|v| v.as_object())
        .or_else(|| {
            value
                .get("result")
                .and_then(|v| v.get("capabilities"))
                .and_then(|v| v.as_object())
        })
    else {
        return;
    };

    for key in caps_obj.keys() {
        if !VALID_MCP_CAPABILITY_KEYS.contains(&key.as_str()) {
            let (line, col) = find_json_field_location(content, key);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-020",
                    format!("Unknown capability key '{}'", key),
                )
                .with_suggestion("Use only capability keys defined by the MCP specification"),
            );
        }
    }
}

fn validate_duplicate_server_names(path: &Path, content: &str, diagnostics: &mut Vec<Diagnostic>) {
    for duplicate in collect_duplicate_mcp_server_names(content) {
        let (line, col) = find_json_field_location(content, &duplicate);
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                col,
                "MCP-023",
                format!("Duplicate MCP server name '{}'", duplicate),
            )
            .with_suggestion("Rename duplicate mcpServers keys so each server name is unique"),
        );
    }
}

fn collect_duplicate_mcp_server_names(content: &str) -> Vec<String> {
    use std::collections::HashSet;

    let key_pos = match content.find("\"mcpServers\"") {
        Some(pos) => pos,
        None => return Vec::new(),
    };
    let colon_pos = match content[key_pos..].find(':') {
        Some(pos) => key_pos + pos,
        None => return Vec::new(),
    };

    let bytes = content.as_bytes();
    let mut idx = colon_pos + 1;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= bytes.len() || bytes[idx] != b'{' {
        return Vec::new();
    }
    idx += 1;

    let mut depth = 1usize;
    let mut expecting_key = true;
    let mut seen = HashSet::new();
    let mut duplicates = HashSet::new();

    while idx < bytes.len() && depth > 0 {
        let ch = bytes[idx] as char;
        if ch == '"' {
            let (raw, next_idx) = read_json_string_literal(content, idx);
            if depth == 1 && expecting_key {
                if !seen.insert(raw.clone()) {
                    duplicates.insert(raw);
                }
                expecting_key = false;
            }
            idx = next_idx;
            continue;
        }

        match ch {
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth = depth.saturating_sub(1);
            }
            ',' if depth == 1 => expecting_key = true,
            _ => {}
        }
        idx += 1;
    }

    let mut result: Vec<String> = duplicates.into_iter().collect();
    result.sort();
    result
}

fn read_json_string_literal(content: &str, start_quote_idx: usize) -> (String, usize) {
    let bytes = content.as_bytes();
    let mut idx = start_quote_idx + 1;
    let mut escaped = false;
    let mut out = String::new();

    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if escaped {
            out.push(ch);
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if ch == '"' {
            return (out, idx + 1);
        }
        out.push(ch);
        idx += 1;
    }

    (out, bytes.len())
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
struct ToolLookupContext<'a> {
    content: &'a str,
    line_starts: &'a [usize],
    tool_span: Option<(usize, usize)>,
    tool_index: usize,
}

fn validate_tool(
    tool: &McpToolSchema,
    path: &Path,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
    lookup: ToolLookupContext<'_>,
) {
    let ToolLookupContext {
        content,
        line_starts,
        tool_span,
        tool_index,
    } = lookup;

    // Always include tool index for clarity, even for first tool
    let tool_prefix = format!("Tool #{}: ", tool_index + 1);

    // Get base location for this tool (preferring a precomputed span).
    let tool_loc = tool_span
        .map(|(start, _)| line_col_at(start, line_starts))
        .unwrap_or((1, 0));

    // Find field location in the current tool span first, then fall back to global lookup.
    let find_field = |field: &str| -> (usize, usize) {
        if let Some(span) = tool_span
            && let Some(loc) = find_json_field_location_in_span(content, field, span, line_starts)
        {
            return loc;
        }
        let (line, col) = find_json_field_location(content, field);
        if line > 1 || col > 0 {
            (line, col)
        } else {
            tool_loc
        }
    };

    let (has_name, has_desc, has_schema) = tool.has_required_fields();

    // MCP-013: Tool name format validation.
    if config.is_rule_enabled("MCP-013")
        && let Some(name) = tool.name.as_deref().map(str::trim)
        && !name.is_empty()
    {
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
        if name.len() > 128 || !valid_chars {
            let (line, col) = find_field("name");
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-013",
                    format!(
                        "{}invalid tool name '{}': expected 1-128 chars using [a-zA-Z0-9_.-]",
                        tool_prefix, name
                    ),
                )
                .with_suggestion(
                    "Rename the tool to use only letters, numbers, underscore, dot, or hyphen",
                ),
            );
        }
    }

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
            // Check if content has a "parameters" field that might be a misnaming
            let has_parameters_field = content.contains("\"parameters\"");
            let suggestion = if has_parameters_field {
                format!(
                    "{}. Found 'parameters' field - did you mean 'inputSchema'?",
                    t!("rules.mcp_002.missing_schema_suggestion")
                )
            } else {
                t!("rules.mcp_002.missing_schema_suggestion").to_string()
            };
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
                .with_suggestion(suggestion),
            );
        }
    }

    // MCP-003: Invalid inputSchema JSON Schema.
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

    // MCP-014: Invalid outputSchema JSON Schema.
    if config.is_rule_enabled("MCP-014")
        && let Some(schema) = &tool.output_schema
    {
        let (line, col) = find_field("outputSchema");
        let schema_errors = validate_json_schema_structure(schema);
        for error in schema_errors {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-014",
                    format!("{}invalid outputSchema: {}", tool_prefix, error),
                )
                .with_suggestion("Ensure outputSchema is a valid JSON Schema object"),
            );
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

        if let Some(annotations) = &tool.annotations {
            let unknown_keys: Vec<_> = annotations
                .keys()
                .filter(|key| !VALID_MCP_ANNOTATION_HINTS.contains(&key.as_str()))
                .cloned()
                .collect();

            if !unknown_keys.is_empty() {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-006",
                        format!(
                            "{}unknown annotation keys: {}",
                            tool_prefix.as_str(),
                            unknown_keys.join(", ")
                        ),
                    )
                    .with_suggestion(
                        "Use only standard annotation hints: readOnlyHint, destructiveHint, idempotentHint, openWorldHint, title",
                    ),
                );
            }
        }
    }
}

fn extract_http_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let scheme_sep = trimmed.find("://")?;
    let host_and_path = &trimmed[scheme_sep + 3..];
    if host_and_path.is_empty() {
        return None;
    }
    let host_port_end = host_and_path
        .find(|c| ['/', '?', '#'].contains(&c))
        .unwrap_or(host_and_path.len());
    let host_port = &host_and_path[..host_port_end];
    if host_port.is_empty() {
        return None;
    }

    if host_port.starts_with('[') {
        if let Some(end) = host_port.find(']') {
            return Some(host_port[..=end].to_ascii_lowercase());
        }
        return None;
    }

    let host = host_port.split(':').next().unwrap_or(host_port);
    Some(host.to_ascii_lowercase())
}

fn is_local_http_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

fn is_wildcard_http_host(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "::" | "[::]" | "*")
}

fn command_value_as_string(command: &serde_json::Value) -> Option<String> {
    match command {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Array(values) => {
            let parts: Vec<&str> = values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        }
        _ => None,
    }
}

fn seems_plaintext_secret(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("${")
        && !trimmed.starts_with("$(")
        && !trimmed.starts_with("{{")
}

fn is_dangerous_command(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let has_remote_pipe = (normalized.contains("curl") || normalized.contains("wget"))
        && normalized.contains('|')
        && (normalized.contains("| sh")
            || normalized.contains("|sh")
            || normalized.contains("| bash")
            || normalized.contains("|bash"));
    let has_sudo_rm = normalized.contains("sudo rm");
    let has_exfil_pattern = (normalized.contains("nc ") || normalized.contains("netcat "))
        && (normalized.contains("/etc/")
            || normalized.contains(".ssh")
            || normalized.contains("token"));
    has_remote_pipe || has_sudo_rm || has_exfil_pattern
}

fn has_meaningful_server_config(server: &McpServerConfig) -> bool {
    let has_type = server
        .server_type
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_command = server.command.as_ref().is_some_and(|value| match value {
        serde_json::Value::String(command) => !command.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    });
    let has_args = server
        .args
        .as_ref()
        .is_some_and(|value| value.as_array().is_some_and(|arr| !arr.is_empty()));
    let has_url = server
        .url
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_env = server.env.as_ref().is_some_and(|env| !env.is_empty());
    has_type || has_command || has_args || has_url || has_env
}

/// Validate a single MCP server configuration entry (MCP-009 to MCP-012, MCP-017 to MCP-022, MCP-024)
fn validate_server(
    name: &str,
    server: &McpServerConfig,
    path: &Path,
    content: &str,
    config: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let (line, col) = find_json_field_location(content, name);

    // Determine effective type (default is "stdio" when type is absent)
    let effective_type = server.server_type.as_deref().unwrap_or("stdio");

    // MCP-011: Invalid server type (check first, skip other type-based rules if invalid)
    if config.is_rule_enabled("MCP-011") {
        if let Some(ref server_type) = server.server_type {
            if !VALID_MCP_SERVER_TYPES.contains(&server_type.as_str()) {
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-011",
                    t!(
                        "rules.mcp_011.message",
                        server = name,
                        server_type = server_type.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_011.suggestion"));

                // Unsafe auto-fix: replace with closest valid server type
                if let Some(closest) =
                    super::find_closest_value(server_type.as_str(), VALID_MCP_SERVER_TYPES)
                {
                    if let Some((start, end)) =
                        find_unique_json_string_value_span(content, "type", server_type)
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            closest,
                            t!("rules.mcp_011.fix", fixed = closest),
                            false,
                        ));
                    }
                }

                diagnostics.push(diagnostic);
                // Skip further type-based validation since type is invalid
                return;
            }
        }
    }

    // MCP-009: Missing command for stdio server
    // Treat null and empty string/array as missing (not a usable command)
    if config.is_rule_enabled("MCP-009") && effective_type == "stdio" {
        let has_command = server.command.as_ref().is_some_and(|v| match v {
            serde_json::Value::String(s) => !s.trim().is_empty(),
            serde_json::Value::Array(a) => !a.is_empty(),
            serde_json::Value::Null => false,
            _ => true,
        });
        if !has_command {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-009",
                    t!("rules.mcp_009.message", server = name),
                )
                .with_suggestion(t!("rules.mcp_009.suggestion")),
            );
        }
    }

    // MCP-022: args must be an array of strings when present.
    if config.is_rule_enabled("MCP-022")
        && let Some(args) = &server.args
    {
        let valid_args = args
            .as_array()
            .is_some_and(|arr| arr.iter().all(|item| item.is_string()));
        if !valid_args {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-022",
                    format!(
                        "Server '{}' has invalid 'args' value: expected array of strings",
                        name
                    ),
                )
                .with_suggestion("Set args to an array of strings, e.g. [\"--port\", \"3000\"]"),
            );
        }
    }

    // MCP-010: Missing url for http/sse server
    // Treat empty/whitespace-only URL as missing
    if config.is_rule_enabled("MCP-010") && ["http", "sse"].contains(&effective_type) {
        let has_url = server.url.as_deref().is_some_and(|u| !u.trim().is_empty());
        if !has_url {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-010",
                    t!(
                        "rules.mcp_010.message",
                        server = name,
                        server_type = effective_type
                    ),
                )
                .with_suggestion(t!("rules.mcp_010.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("MCP-017")
        && effective_type == "http"
        && let Some(url) = server.url.as_deref()
    {
        let lower_url = url.trim().to_ascii_lowercase();
        if lower_url.starts_with("http://")
            && let Some(host) = extract_http_host(url)
            && !is_local_http_host(&host)
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-017",
                    format!(
                        "Server '{}' uses insecure HTTP URL '{}'; use HTTPS for non-localhost endpoints",
                        name, url
                    ),
                )
                .with_suggestion("Change the server URL to https:// for remote endpoints"),
            );
        }
    }

    if config.is_rule_enabled("MCP-021")
        && effective_type == "http"
        && let Some(url) = server.url.as_deref()
        && let Some(host) = extract_http_host(url)
        && is_wildcard_http_host(&host)
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-021",
                format!(
                    "Server '{}' binds HTTP to '{}', which exposes all interfaces",
                    name, host
                ),
            )
            .with_suggestion("Prefer localhost bindings unless remote network access is required"),
        );
    }

    if effective_type == "stdio" {
        if config.is_rule_enabled("MCP-018")
            && let Some(env) = &server.env
        {
            for (env_key, env_value) in env {
                let key_upper = env_key.to_ascii_uppercase();
                let looks_sensitive = ["API_KEY", "SECRET", "TOKEN", "PASSWORD"]
                    .iter()
                    .any(|needle| key_upper.contains(needle));
                if looks_sensitive && seems_plaintext_secret(env_value) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "MCP-018",
                            format!(
                                "Server '{}' defines potential plaintext secret in env var '{}'",
                                name, env_key
                            ),
                        )
                        .with_suggestion("Use secret injection from environment/runtime instead of hardcoded values"),
                    );
                }
            }
        }

        if config.is_rule_enabled("MCP-019")
            && let Some(command) = &server.command
            && let Some(command_text) = command_value_as_string(command)
            && is_dangerous_command(&command_text)
        {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-019",
                    format!(
                        "Server '{}' command appears dangerous: {}",
                        name, command_text
                    ),
                )
                .with_suggestion("Avoid remote shell pipes, destructive commands, and potential data exfiltration patterns"),
            );
        }
    }

    // MCP-012: Deprecated SSE transport
    if config.is_rule_enabled("MCP-012") && effective_type == "sse" {
        let mut diag = Diagnostic::error(
            path.to_path_buf(),
            line,
            col,
            "MCP-012",
            t!("rules.mcp_012.message", server = name),
        )
        .with_suggestion(t!("rules.mcp_012.suggestion"));

        // Unsafe auto-fix: change "sse" to "http"
        if let Some((start, end)) = find_unique_json_string_value_span(content, "type", "sse") {
            diag = diag.with_fix(Fix::replace(
                start,
                end,
                "http",
                t!("rules.mcp_012.fix"),
                false,
            ));
        }

        diagnostics.push(diag);
    }

    if config.is_rule_enabled("MCP-024") && !has_meaningful_server_config(server) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                col,
                "MCP-024",
                format!("Server '{}' has an empty configuration object", name),
            )
            .with_suggestion(
                "Define at least one meaningful field such as type, command, url, args, or env",
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

    #[test]
    fn test_mcp_003_validates_output_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "outputSchema": {"type": "not_a_real_type"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-014"));
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

    #[test]
    fn test_mcp_006_unknown_annotation_keys() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {"dangerous": true, "readOnlyHint": true}
        }"#;
        let diagnostics = validate(content);
        let mcp_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-006").collect();
        assert!(
            mcp_006
                .iter()
                .any(|d| d.message.contains("unknown annotation keys")),
            "Expected MCP-006 warning for unknown annotation keys, got: {:?}",
            mcp_006
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        );
    }

    // Config wiring tests
    #[test]
    fn test_config_disabled_mcp_category() {
        let mut config = LintConfig::default();
        config.rules_mut().mcp = false;

        let content = r#"{"jsonrpc": "1.0"}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["MCP-001".to_string()];

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
                "protocolVersion": "2025-11-25",
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
        assert!(mcp_008[0].message.contains("2025-11-25"));
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
                "protocolVersion": "2025-11-25",
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
        config.set_mcp_protocol_version(Some("2024-11-05".to_string()));

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
        config.rules_mut().disabled_rules = vec!["MCP-008".to_string()];

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
        config.set_mcp_protocol_version(None);
        config.spec_revisions_mut().mcp_protocol = None;
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
        config.spec_revisions_mut().mcp_protocol = Some("2025-11-25".to_string());
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
        assert_eq!(diag.fixes[0].replacement, "2025-11-25");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_no_assumption_when_version_pinned_via_legacy() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(Some("2025-11-25".to_string()));
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
        assert_eq!(diag.fixes[0].replacement, "2025-11-25");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_response_assumption_when_version_not_pinned() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(None);
        config.spec_revisions_mut().mcp_protocol = None;

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
        config.rules_mut().disabled_rules = vec!["MCP-007".to_string()];

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
            "MCP-009", "MCP-010", "MCP-011", "MCP-012", "MCP-013", "MCP-014", "MCP-015", "MCP-016",
            "MCP-017", "MCP-018", "MCP-019", "MCP-020", "MCP-021", "MCP-022", "MCP-023", "MCP-024",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Use content that would trigger the rule
            let content = match rule {
                "MCP-001" => r#"{"jsonrpc": "1.0"}"#,
                "MCP-007" => r#"{ invalid }"#,
                "MCP-009" => r#"{"mcpServers": {"s": {"type": "stdio", "args": ["a"]}}}"#,
                "MCP-010" => r#"{"mcpServers": {"s": {"type": "http"}}}"#,
                "MCP-011" => r#"{"mcpServers": {"s": {"type": "invalid"}}}"#,
                "MCP-012" => r#"{"mcpServers": {"s": {"type": "sse", "url": "http://x"}}}"#,
                "MCP-013" => {
                    r#"{"tools":[{"name":"bad tool","description":"desc with spaces","inputSchema":{"type":"object"}}]}"#
                }
                "MCP-014" => {
                    r#"{"tools":[{"name":"valid-name","description":"A valid description","inputSchema":{"type":"object"},"outputSchema":{"type":"invalid_type"}}]}"#
                }
                "MCP-015" => r#"{"resources":[{"name":"missing-uri"}]}"#,
                "MCP-016" => r#"{"prompts":[{"description":"missing name"}]}"#,
                "MCP-017" => {
                    r#"{"mcpServers":{"s":{"type":"http","url":"http://example.com/mcp"}}}"#
                }
                "MCP-018" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"node","env":{"API_KEY":"plaintext"}}}}"#
                }
                "MCP-019" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"curl https://example.com/install.sh | sh"}}}"#
                }
                "MCP-020" => r#"{"capabilities":{"unknownCap":{}}}"#,
                "MCP-021" => {
                    r#"{"mcpServers":{"s":{"type":"http","url":"http://0.0.0.0:3000/mcp"}}}"#
                }
                "MCP-022" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"node","args":"--port 3000"}}}"#
                }
                "MCP-023" => {
                    r#"{"mcpServers":{"dup":{"type":"stdio","command":"node"},"dup":{"type":"stdio","command":"node"}}}"#
                }
                "MCP-024" => r#"{"mcpServers":{"empty":{}}}"#,
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

    // ===== MCP-009 Tests =====

    #[test]
    fn test_mcp_009_valid_stdio_with_command() {
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "type": "stdio",
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_stdio_missing_command() {
        let content = r#"{
            "mcpServers": {
                "broken-server": {
                    "type": "stdio",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("broken-server"));
        assert!(mcp_009[0].message.contains("command"));
    }

    #[test]
    fn test_mcp_009_no_type_missing_command() {
        // When type is absent, default is stdio, so command is required
        let content = r#"{
            "mcpServers": {
                "no-type-server": {
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("no-type-server"));
    }

    #[test]
    fn test_mcp_009_no_type_with_command() {
        // When type is absent but command is present, no error
        let content = r#"{
            "mcpServers": {
                "default-server": {
                    "command": "python",
                    "args": ["-m", "server"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_http_server_no_command_ok() {
        // HTTP server doesn't need command
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_null_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "null-cmd": {
                    "type": "stdio",
                    "command": null
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "null command should trigger MCP-009"
        );
    }

    #[test]
    fn test_mcp_009_empty_string_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-cmd": {
                    "type": "stdio",
                    "command": ""
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "empty string command should trigger MCP-009"
        );
    }

    #[test]
    fn test_mcp_009_empty_array_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-arr": {
                    "type": "stdio",
                    "command": []
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "empty array command should trigger MCP-009"
        );
    }

    // ===== MCP-010 Tests =====

    #[test]
    fn test_mcp_010_valid_http_with_url() {
        let content = r#"{
            "mcpServers": {
                "remote-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_http_missing_url() {
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("http-server"));
        assert!(mcp_010[0].message.contains("url"));
    }

    #[test]
    fn test_mcp_010_sse_missing_url() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("sse-server"));
    }

    #[test]
    fn test_mcp_010_valid_sse_with_url() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_stdio_no_url_ok() {
        // Stdio server doesn't need url
        let content = r#"{
            "mcpServers": {
                "stdio-server": {
                    "type": "stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_empty_url_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-url": {
                    "type": "http",
                    "url": ""
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "empty URL should trigger MCP-010"
        );
    }

    #[test]
    fn test_mcp_010_whitespace_url_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "ws-url": {
                    "type": "http",
                    "url": "   "
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "whitespace-only URL should trigger MCP-010"
        );
    }

    // ===== MCP-011 Tests =====

    #[test]
    fn test_mcp_011_valid_types() {
        for server_type in &["stdio", "http", "sse"] {
            let content = format!(
                r#"{{
                    "mcpServers": {{
                        "server": {{
                            "type": "{}",
                            "command": "node",
                            "url": "http://localhost"
                        }}
                    }}
                }}"#,
                server_type
            );
            let diagnostics = validate(&content);
            assert!(
                !diagnostics.iter().any(|d| d.rule == "MCP-011"),
                "Type '{}' should be valid",
                server_type
            );
        }
    }

    #[test]
    fn test_mcp_011_invalid_type() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "websocket",
                    "url": "ws://localhost:8080"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        assert!(mcp_011[0].message.contains("bad-server"));
        assert!(mcp_011[0].message.contains("websocket"));
    }

    #[test]
    fn test_mcp_011_invalid_type_skips_other_rules() {
        // When type is invalid, MCP-009/MCP-010 should not trigger
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "grpc"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-011"));
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "MCP-009 should not trigger for invalid type"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "MCP-010 should not trigger for invalid type"
        );
    }

    #[test]
    fn test_mcp_011_no_type_field_ok() {
        // Missing type is not invalid (it defaults to stdio)
        let content = r#"{
            "mcpServers": {
                "server": {
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-011"));
    }

    #[test]
    fn test_mcp_011_autofix_case_insensitive() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "Stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        assert!(
            mcp_011[0].has_fixes(),
            "MCP-011 should have auto-fix for case mismatch"
        );
        let fix = &mcp_011[0].fixes[0];
        assert!(!fix.safe, "MCP-011 fix should be unsafe");
        assert_eq!(fix.replacement, "stdio", "Fix should suggest 'stdio'");
    }

    #[test]
    fn test_mcp_011_no_autofix_nonsense() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "grpc"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        // "grpc" has no close match to stdio/http/sse - no fix
        assert!(
            !mcp_011[0].has_fixes(),
            "MCP-011 should not auto-fix nonsense values"
        );
    }

    // ===== MCP-012 Tests =====

    #[test]
    fn test_mcp_012_sse_deprecated_error() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_012: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-012").collect();
        assert_eq!(mcp_012.len(), 1);
        assert!(mcp_012[0].message.contains("sse-server"));
        assert!(mcp_012[0].message.contains("deprecated"));
        assert_eq!(mcp_012[0].level, crate::diagnostics::DiagnosticLevel::Error);
    }

    #[test]
    fn test_mcp_012_sse_has_autofix() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_012 = diagnostics.iter().find(|d| d.rule == "MCP-012");
        assert!(mcp_012.is_some());
        let diag = mcp_012.unwrap();
        assert!(diag.has_fixes());
        assert_eq!(diag.fixes[0].replacement, "http");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_012_http_no_warning() {
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-012"));
    }

    #[test]
    fn test_mcp_012_stdio_no_warning() {
        let content = r#"{
            "mcpServers": {
                "server": {
                    "type": "stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-012"));
    }

    // ===== MCP-013..MCP-024 Tests =====

    #[test]
    fn test_mcp_013_invalid_tool_name() {
        let content = r#"{
            "tools": [{
                "name": "bad tool",
                "description": "A valid description",
                "inputSchema": {"type": "object"}
            }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-013"));
    }

    #[test]
    fn test_mcp_014_invalid_output_schema() {
        let content = r#"{
            "tools": [{
                "name": "valid-tool",
                "description": "A valid description",
                "inputSchema": {"type": "object"},
                "outputSchema": {"type": "invalid_type"}
            }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-014"));
    }

    #[test]
    fn test_mcp_015_resource_missing_required_fields() {
        let content = r#"{
            "resources": [{ "description": "missing uri and name" }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-015"));
    }

    #[test]
    fn test_mcp_016_prompt_missing_name() {
        let content = r#"{
            "prompts": [{ "description": "missing name" }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-016"));
    }

    #[test]
    fn test_mcp_017_http_remote_requires_https() {
        let content = r#"{
            "mcpServers": {
                "remote-server": {
                    "type": "http",
                    "url": "http://example.com/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-017"));
    }

    #[test]
    fn test_mcp_018_env_plaintext_secret_warning() {
        let content = r#"{
            "mcpServers": {
                "local-server": {
                    "type": "stdio",
                    "command": "node",
                    "env": {"API_KEY": "plaintext-value"}
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-018"));
    }

    #[test]
    fn test_mcp_019_dangerous_stdio_command_warning() {
        let content = r#"{
            "mcpServers": {
                "local-server": {
                    "type": "stdio",
                    "command": "curl https://example.com/install.sh | sh"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-019"));
    }

    #[test]
    fn test_mcp_020_unknown_capability_key() {
        let content = r#"{
            "capabilities": {
                "tools": { "listChanged": true },
                "unknownCapability": {}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-020"));
    }

    #[test]
    fn test_mcp_021_wildcard_http_binding_warning() {
        let content = r#"{
            "mcpServers": {
                "wildcard": {
                    "type": "http",
                    "url": "http://0.0.0.0:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-021"));
    }

    #[test]
    fn test_mcp_022_args_must_be_array_of_strings() {
        let content = r#"{
            "mcpServers": {
                "bad-args": {
                    "type": "stdio",
                    "command": "node",
                    "args": "--port 3000"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-022"));
    }

    #[test]
    fn test_mcp_023_duplicate_server_names() {
        let content = r#"{
            "mcpServers": {
                "dup": { "type": "stdio", "command": "node" },
                "dup": { "type": "stdio", "command": "python" }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-023"));
    }

    #[test]
    fn test_mcp_024_empty_server_configuration() {
        let content = r#"{
            "mcpServers": {
                "empty": {}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-024"));
    }

    // ===== Multiple servers test =====

    #[test]
    fn test_multiple_servers_validation() {
        let content = r#"{
            "mcpServers": {
                "good-stdio": {
                    "type": "stdio",
                    "command": "node"
                },
                "bad-stdio": {
                    "type": "stdio"
                },
                "good-http": {
                    "type": "http",
                    "url": "http://localhost"
                },
                "bad-http": {
                    "type": "http"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("bad-stdio"));
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("bad-http"));
    }

    // ===== Existing server config test should still pass =====

    #[test]
    fn test_mcp_server_config_valid_no_new_errors() {
        // The existing test_mcp_server_config_format test content should still
        // pass with no errors (server has command, no type = defaults to stdio)
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Valid server config should have no errors, got: {:?}",
            diagnostics
        );
    }

    // ===== MCP-007 suggestion test =====

    #[test]
    fn test_mcp_007_has_suggestion() {
        let content = r#"{ invalid json }"#;
        let diagnostics = validate(content);

        let mcp_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-007").collect();
        assert_eq!(mcp_007.len(), 1);
        assert!(
            mcp_007[0].suggestion.is_some(),
            "MCP-007 should have a suggestion"
        );
        assert!(
            mcp_007[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("Validate JSON syntax"),
            "MCP-007 suggestion should mention JSON syntax"
        );
    }

    // ===== MCP-003 improved suggestion test =====

    #[test]
    fn test_mcp_003_suggestion_lists_valid_types() {
        let content = r#"{"name": "test-tool", "description": "A test tool that does things", "inputSchema": {"type": "invalid"}}"#;
        let diagnostics = validate(content);

        let mcp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-003").collect();
        assert_eq!(mcp_003.len(), 1);
        assert!(
            mcp_003[0].suggestion.is_some(),
            "MCP-003 should have a suggestion"
        );
        let suggestion = mcp_003[0].suggestion.as_ref().unwrap();
        assert!(
            suggestion.contains("string")
                && suggestion.contains("number")
                && suggestion.contains("integer")
                && suggestion.contains("boolean")
                && suggestion.contains("object")
                && suggestion.contains("array")
                && suggestion.contains("null"),
            "MCP-003 suggestion should list all valid JSON Schema types, got: {}",
            suggestion
        );
    }

    // ===== MCP-006 improved suggestion test =====

    #[test]
    fn test_mcp_006_suggestion_warns_about_self_reported() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool that does useful things",
            "inputSchema": {"type": "object"},
            "annotations": {"title": "My Tool", "readOnlyHint": true}
        }"#;
        let diagnostics = validate(content);

        let mcp_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-006").collect();
        assert_eq!(mcp_006.len(), 1);
        assert!(
            mcp_006[0].suggestion.is_some(),
            "MCP-006 should have a suggestion"
        );
        let suggestion = mcp_006[0].suggestion.as_ref().unwrap();
        assert!(
            suggestion.contains("self-reported"),
            "MCP-006 suggestion should warn about self-reported annotations, got: {}",
            suggestion
        );
        assert!(
            suggestion.contains("malicious"),
            "MCP-006 suggestion should warn about potential malicious annotations, got: {}",
            suggestion
        );
    }
}
