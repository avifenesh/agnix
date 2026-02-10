//! Shared validation utilities

/// Check if a tool name is valid (either known or properly formatted MCP tool).
/// MCP tools must follow the format: mcp__<server>__<tool> (case-sensitive, lowercase prefix).
///
/// # Examples
/// ```
/// use agnix_core::validation::is_valid_mcp_tool_format;
///
/// assert!(is_valid_mcp_tool_format("mcp__filesystem__read_file", &["Read", "Write"]));
/// assert!(is_valid_mcp_tool_format("Read", &["Read", "Write"]));
/// assert!(!is_valid_mcp_tool_format("mcp__", &["Read"])); // Empty
/// assert!(!is_valid_mcp_tool_format("mcp__server", &["Read"])); // No tool part
/// assert!(!is_valid_mcp_tool_format("MCP__server__tool", &["Read"])); // Uppercase
/// ```
pub fn is_valid_mcp_tool_format(tool: &str, known_tools: &[&str]) -> bool {
    // Strip parenthesized parameters from tool names like "Read(file_path)"
    // so that both "Read" and "Read(file_path)" match the known tool "Read".
    let base_name = tool.split('(').next().unwrap_or(tool);

    // Check if it's a known tool
    if known_tools.contains(&base_name) {
        return true;
    }

    // Check if it's a valid MCP tool: mcp__<server>__<tool>
    if let Some(rest) = base_name.strip_prefix("mcp__") {
        // Must have at least one more double-underscore separator for server__tool
        // and both server and tool parts must be non-empty
        if let Some(tool_start) = rest.find("__") {
            let server = &rest[..tool_start];
            let tool_name = &rest[tool_start + 2..];
            return !server.is_empty() && !tool_name.is_empty();
        }
    }

    false
}
