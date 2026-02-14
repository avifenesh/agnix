//! Roo Code configuration file schema helpers
//!
//! Provides parsing and validation for Roo Code configuration files:
//! - `.roomodes` - custom mode definitions
//! - `.roo/mcp.json` - MCP server configuration
//! - `.roo/rules-{slug}/*.md` - mode-specific rule files

use std::path::Path;

/// Valid tool group names for Roo Code custom modes.
pub const VALID_GROUP_NAMES: &[&str] = &["read", "edit", "browser", "command", "mcp"];

/// Built-in mode slugs that ship with Roo Code.
pub const BUILTIN_MODE_SLUGS: &[&str] = &["code", "architect", "ask", "debug", "orchestrator"];

/// A JSON parse error with location information.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// A single custom mode entry from .roomodes.
#[derive(Debug, Clone)]
pub struct RooModeEntry {
    pub slug: String,
    pub name: String,
    pub role_definition: String,
    pub groups: Vec<String>,
}

/// Result of parsing .roomodes.
#[derive(Debug, Clone)]
pub struct ParsedRooModes {
    pub modes: Vec<RooModeEntry>,
    pub parse_error: Option<ParseError>,
    /// The raw JSON value for further inspection.
    pub raw_value: Option<serde_json::Value>,
}

/// A single MCP server entry from .roo/mcp.json.
#[derive(Debug, Clone)]
pub struct RooMcpServer {
    pub name: String,
    pub has_command: bool,
    pub has_url: bool,
    pub server_type: Option<String>,
}

/// Result of parsing .roo/mcp.json.
#[derive(Debug, Clone)]
pub struct ParsedRooMcp {
    pub servers: Vec<RooMcpServer>,
    pub parse_error: Option<ParseError>,
    /// The raw JSON value for further inspection.
    pub raw_value: Option<serde_json::Value>,
}

/// Check if a string is a valid slug (lowercase alphanumeric and hyphens only, non-empty).
pub fn is_valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// Extract the mode slug from a `.roo/rules-{slug}/` path pattern.
///
/// Given a path like `.roo/rules-architect/general.md`, returns `Some("architect")`.
pub fn extract_slug_from_path(path: &Path) -> Option<String> {
    let parent = path.parent()?.file_name()?.to_str()?;
    parent.strip_prefix("rules-").map(|s| s.to_string())
}

/// Parse .roomodes content into structured data.
///
/// The expected format is:
/// ```json
/// {
///   "customModes": [
///     {
///       "slug": "designer",
///       "name": "Designer",
///       "roleDefinition": "You are a UI/UX designer...",
///       "groups": ["read", "edit"]
///     }
///   ]
/// }
/// ```
pub fn parse_roomodes(content: &str) -> ParsedRooModes {
    let value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return ParsedRooModes {
                modes: Vec::new(),
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                }),
                raw_value: None,
            };
        }
    };

    let mut modes = Vec::new();

    if let Some(custom_modes) = value.get("customModes").and_then(|v| v.as_array()) {
        for mode in custom_modes {
            let slug = mode
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = mode
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let role_definition = mode
                .get("roleDefinition")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let groups = mode
                .get("groups")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| item.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            modes.push(RooModeEntry {
                slug,
                name,
                role_definition,
                groups,
            });
        }
    }

    ParsedRooModes {
        modes,
        parse_error: None,
        raw_value: Some(value),
    }
}

/// Parse .roo/mcp.json content into structured data.
///
/// The expected format is:
/// ```json
/// {
///   "mcpServers": {
///     "server-name": {
///       "command": "node",
///       "args": ["server.js"]
///     }
///   }
/// }
/// ```
pub fn parse_roo_mcp(content: &str) -> ParsedRooMcp {
    let value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return ParsedRooMcp {
                servers: Vec::new(),
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                }),
                raw_value: None,
            };
        }
    };

    let mut servers = Vec::new();

    if let Some(mcp_servers) = value.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, server_value) in mcp_servers {
            let server_type = server_value
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let has_command = server_value.get("command").is_some();
            let has_url = server_value.get("url").is_some();

            servers.push(RooMcpServer {
                name: name.clone(),
                has_command,
                has_url,
                server_type,
            });
        }
    }

    ParsedRooMcp {
        servers,
        parse_error: None,
        raw_value: Some(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== is_valid_slug =====

    #[test]
    fn test_valid_slug() {
        assert!(is_valid_slug("architect"));
        assert!(is_valid_slug("code"));
        assert!(is_valid_slug("my-mode"));
        assert!(is_valid_slug("mode123"));
    }

    #[test]
    fn test_invalid_slug() {
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("-starts-with-hyphen"));
        assert!(!is_valid_slug("ends-with-hyphen-"));
        assert!(!is_valid_slug("has spaces"));
        assert!(!is_valid_slug("has_underscores"));
        assert!(!is_valid_slug("UPPERCASE"));
    }

    // ===== extract_slug_from_path =====

    #[test]
    fn test_extract_slug_from_path() {
        assert_eq!(
            extract_slug_from_path(Path::new(".roo/rules-architect/general.md")),
            Some("architect".to_string())
        );
        assert_eq!(
            extract_slug_from_path(Path::new(".roo/rules-code/style.md")),
            Some("code".to_string())
        );
    }

    #[test]
    fn test_extract_slug_no_match() {
        assert_eq!(
            extract_slug_from_path(Path::new(".roo/rules/general.md")),
            None
        );
        assert_eq!(
            extract_slug_from_path(Path::new(".roorules")),
            None
        );
    }

    // ===== parse_roomodes =====

    #[test]
    fn test_parse_valid_roomodes() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "You are a UI/UX designer.",
      "groups": ["read", "edit"]
    }
  ]
}"#;
        let result = parse_roomodes(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.modes.len(), 1);
        assert_eq!(result.modes[0].slug, "designer");
        assert_eq!(result.modes[0].name, "Designer");
        assert_eq!(result.modes[0].groups, vec!["read", "edit"]);
    }

    #[test]
    fn test_parse_empty_roomodes() {
        let result = parse_roomodes("{}");
        assert!(result.parse_error.is_none());
        assert!(result.modes.is_empty());
    }

    #[test]
    fn test_parse_invalid_roomodes_json() {
        let result = parse_roomodes("{ invalid }");
        assert!(result.parse_error.is_some());
    }

    // ===== parse_roo_mcp =====

    #[test]
    fn test_parse_valid_roo_mcp() {
        let content = r#"{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["server.js"]
    }
  }
}"#;
        let result = parse_roo_mcp(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.servers.len(), 1);
        assert_eq!(result.servers[0].name, "my-server");
        assert!(result.servers[0].has_command);
    }

    #[test]
    fn test_parse_empty_roo_mcp() {
        let result = parse_roo_mcp("{}");
        assert!(result.parse_error.is_none());
        assert!(result.servers.is_empty());
    }

    #[test]
    fn test_parse_invalid_roo_mcp_json() {
        let result = parse_roo_mcp("not json");
        assert!(result.parse_error.is_some());
    }

    #[test]
    fn test_parse_roo_mcp_with_url_server() {
        let content = r#"{
  "mcpServers": {
    "remote": {
      "url": "https://example.com/mcp",
      "type": "http"
    }
  }
}"#;
        let result = parse_roo_mcp(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.servers.len(), 1);
        assert!(result.servers[0].has_url);
        assert!(!result.servers[0].has_command);
        assert_eq!(result.servers[0].server_type, Some("http".to_string()));
    }
}
