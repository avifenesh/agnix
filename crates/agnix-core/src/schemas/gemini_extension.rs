//! Gemini CLI extension manifest schema helpers
//!
//! Provides parsing and validation for gemini-extension.json files.
//!
//! Validates:
//! - JSON syntax
//! - Required fields: name, version, description
//! - Extension name format (lowercase alphanumeric with dashes)
//! - Unknown top-level keys

/// Required fields in gemini-extension.json
pub const REQUIRED_FIELDS: &[&str] = &["name", "version", "description"];

/// Known optional top-level keys
pub const KNOWN_OPTIONAL_KEYS: &[&str] =
    &["mcpServers", "contextFileName", "excludeTools", "settings"];

/// A JSON parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Result of parsing gemini-extension.json
#[derive(Debug, Clone)]
pub struct ParsedGeminiExtension {
    /// Parse error if JSON is invalid
    pub parse_error: Option<ParseError>,
    /// The parsed schema (if valid JSON)
    pub schema: Option<GeminiExtensionSchema>,
    /// Top-level keys not in known required or optional sets
    pub unknown_keys: Vec<String>,
}

/// Partial schema for gemini-extension.json
#[derive(Debug, Clone, Default)]
pub struct GeminiExtensionSchema {
    /// Extension name
    pub name: Option<String>,
    /// Extension version
    pub version: Option<String>,
    /// Extension description
    pub description: Option<String>,
    /// Context file name (e.g., "CONTEXT.md")
    pub context_file_name: Option<String>,
}

/// Validate an extension name follows the required format.
///
/// Must be lowercase alphanumeric with dashes, starting with a letter or digit.
/// Pattern: `^[a-z0-9][a-z0-9-]*$`
pub fn is_valid_extension_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();

    // First character must be lowercase letter or digit
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }

    // Remaining characters must be lowercase letters, digits, or dashes
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse gemini-extension.json content
pub fn parse_gemini_extension(content: &str) -> ParsedGeminiExtension {
    // Parse JSON
    let value: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(e) => {
            return ParsedGeminiExtension {
                parse_error: Some(ParseError {
                    message: e.to_string(),
                    line: e.line(),
                    column: e.column(),
                }),
                schema: None,
                unknown_keys: Vec::new(),
            };
        }
    };

    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            return ParsedGeminiExtension {
                parse_error: Some(ParseError {
                    message: "Expected a JSON object".to_string(),
                    line: 1,
                    column: 1,
                }),
                schema: None,
                unknown_keys: Vec::new(),
            };
        }
    };

    // Check for unknown keys
    let all_known: Vec<&str> = REQUIRED_FIELDS
        .iter()
        .chain(KNOWN_OPTIONAL_KEYS.iter())
        .copied()
        .collect();
    let unknown_keys: Vec<String> = obj
        .keys()
        .filter(|k| !all_known.contains(&k.as_str()))
        .cloned()
        .collect();

    // Extract schema fields
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let version = obj
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let context_file_name = obj
        .get("contextFileName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ParsedGeminiExtension {
        parse_error: None,
        schema: Some(GeminiExtensionSchema {
            name,
            version,
            description,
            context_file_name,
        }),
        unknown_keys,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_extension() {
        let content = r#"{
  "name": "my-extension",
  "version": "1.0.0",
  "description": "A test extension"
}"#;
        let result = parse_gemini_extension(content);
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        assert!(result.unknown_keys.is_empty());
        let schema = result.schema.unwrap();
        assert_eq!(schema.name, Some("my-extension".to_string()));
        assert_eq!(schema.version, Some("1.0.0".to_string()));
        assert_eq!(schema.description, Some("A test extension".to_string()));
    }

    #[test]
    fn test_parse_with_optional_fields() {
        let content = r#"{
  "name": "ext",
  "version": "0.1.0",
  "description": "Test",
  "contextFileName": "CONTEXT.md",
  "mcpServers": {},
  "excludeTools": [],
  "settings": {}
}"#;
        let result = parse_gemini_extension(content);
        assert!(result.parse_error.is_none());
        assert!(result.unknown_keys.is_empty());
        let schema = result.schema.unwrap();
        assert_eq!(schema.context_file_name, Some("CONTEXT.md".to_string()));
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_gemini_extension("{ invalid }");
        assert!(result.parse_error.is_some());
        assert!(result.schema.is_none());
    }

    #[test]
    fn test_parse_empty_object() {
        let result = parse_gemini_extension("{}");
        assert!(result.parse_error.is_none());
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert!(schema.name.is_none());
        assert!(schema.version.is_none());
        assert!(schema.description.is_none());
    }

    #[test]
    fn test_unknown_keys_detected() {
        let content = r#"{
  "name": "ext",
  "version": "1.0.0",
  "description": "Test",
  "unknownField": true
}"#;
        let result = parse_gemini_extension(content);
        assert!(result.parse_error.is_none());
        assert_eq!(result.unknown_keys.len(), 1);
        assert_eq!(result.unknown_keys[0], "unknownField");
    }

    #[test]
    fn test_is_valid_extension_name_valid() {
        assert!(is_valid_extension_name("my-extension"));
        assert!(is_valid_extension_name("ext"));
        assert!(is_valid_extension_name("ext123"));
        assert!(is_valid_extension_name("a"));
        assert!(is_valid_extension_name("0ext"));
    }

    #[test]
    fn test_is_valid_extension_name_invalid() {
        assert!(!is_valid_extension_name(""));
        assert!(!is_valid_extension_name("My-Extension")); // uppercase
        assert!(!is_valid_extension_name("-ext")); // starts with dash
        assert!(!is_valid_extension_name("ext_name")); // underscore
        assert!(!is_valid_extension_name("ext name")); // space
        assert!(!is_valid_extension_name("EXT")); // all uppercase
    }

    #[test]
    fn test_parse_error_location() {
        let content = "{\n  \"name\": \n}";
        let result = parse_gemini_extension(content);
        assert!(result.parse_error.is_some());
        let err = result.parse_error.unwrap();
        assert!(err.line > 0);
    }

    #[test]
    fn test_non_string_fields_ignored() {
        // name as number should result in name being None
        let content = r#"{"name": 42, "version": "1.0.0", "description": "Test"}"#;
        let result = parse_gemini_extension(content);
        assert!(result.parse_error.is_none());
        let schema = result.schema.unwrap();
        assert!(schema.name.is_none());
    }
}
