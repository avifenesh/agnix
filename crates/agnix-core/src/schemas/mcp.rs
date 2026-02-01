//! MCP (Model Context Protocol) schema definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP tool definition schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSchema {
    /// Required: tool name
    pub name: Option<String>,

    /// Required: tool description
    pub description: Option<String>,

    /// Required: JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,

    /// Optional: annotations (should be treated as untrusted)
    #[serde(default)]
    pub annotations: Option<HashMap<String, serde_json::Value>>,

    /// Optional: requires user approval before invocation
    #[serde(rename = "requiresApproval")]
    pub requires_approval: Option<bool>,

    /// Optional: confirmation field for consent
    pub confirmation: Option<String>,
}

/// MCP JSON-RPC message schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcMessage {
    /// Must be "2.0"
    pub jsonrpc: Option<String>,

    /// Request/response ID
    pub id: Option<serde_json::Value>,

    /// Method name
    pub method: Option<String>,

    /// Parameters
    pub params: Option<serde_json::Value>,

    /// Result (for responses)
    pub result: Option<serde_json::Value>,

    /// Error (for error responses)
    pub error: Option<serde_json::Value>,
}

/// MCP server configuration (as used in .mcp.json or settings.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to run the server
    pub command: Option<serde_json::Value>, // Can be string or array

    /// Command arguments
    #[serde(default)]
    pub args: Option<Vec<String>>,

    /// Environment variables
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

/// MCP configuration file schema (standalone .mcp.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigSchema {
    /// Server definitions
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,

    /// Tools array (for tool definition files)
    pub tools: Option<Vec<McpToolSchema>>,

    /// JSON-RPC version (for message files)
    pub jsonrpc: Option<String>,
}

/// Valid JSON Schema types
pub const VALID_JSON_SCHEMA_TYPES: &[&str] = &[
    "string", "number", "integer", "boolean", "object", "array", "null",
];

impl McpToolSchema {
    /// Check if all required fields are present
    pub fn has_required_fields(&self) -> (bool, bool, bool) {
        (
            !self.name.as_deref().unwrap_or("").trim().is_empty(),
            !self.description.as_deref().unwrap_or("").trim().is_empty(),
            self.input_schema.is_some(),
        )
    }

    /// Check if description is meaningful (not empty, reasonably long)
    pub fn has_meaningful_description(&self) -> bool {
        self.description
            .as_deref()
            .is_some_and(|desc| !desc.trim().is_empty() && desc.len() >= 10)
    }

    /// Check if tool has consent-related fields with meaningful values
    /// - requiresApproval must be true (false doesn't indicate consent mechanism)
    /// - confirmation must be a non-empty string
    pub fn has_consent_fields(&self) -> bool {
        self.requires_approval == Some(true)
            || self
                .confirmation
                .as_deref()
                .is_some_and(|c| !c.trim().is_empty())
    }

    /// Check if tool has annotations (which should be validated)
    pub fn has_annotations(&self) -> bool {
        self.annotations.as_ref().is_some_and(|a| !a.is_empty())
    }
}

impl McpJsonRpcMessage {
    /// Check if JSON-RPC version is valid (must be "2.0")
    pub fn has_valid_jsonrpc_version(&self) -> bool {
        match &self.jsonrpc {
            Some(version) => version == "2.0",
            None => false,
        }
    }
}

/// Validate JSON Schema structure (basic structural validation)
pub fn validate_json_schema_structure(schema: &serde_json::Value) -> Vec<String> {
    let mut errors = Vec::new();

    // Must be an object
    if !schema.is_object() {
        errors.push("inputSchema must be an object".to_string());
        return errors;
    }

    let obj = schema.as_object().unwrap();

    // If "type" field exists, must be a valid JSON Schema type
    if let Some(type_val) = obj.get("type") {
        if let Some(type_str) = type_val.as_str() {
            if !VALID_JSON_SCHEMA_TYPES.contains(&type_str) {
                errors.push(format!(
                    "Invalid JSON Schema type '{}', expected one of: {}",
                    type_str,
                    VALID_JSON_SCHEMA_TYPES.join(", ")
                ));
            }
        } else if let Some(type_arr) = type_val.as_array() {
            // Type can also be an array of types (union type)
            for t in type_arr {
                if let Some(t_str) = t.as_str() {
                    if !VALID_JSON_SCHEMA_TYPES.contains(&t_str) {
                        errors.push(format!(
                            "Invalid JSON Schema type '{}' in type array",
                            t_str
                        ));
                    }
                } else {
                    // Non-string element in type array
                    errors.push("'type' array elements must be strings".to_string());
                }
            }
        } else {
            // type field is neither string nor array (e.g., number, object, boolean)
            errors.push("'type' field must be a string or array of strings".to_string());
        }
    }

    // If "properties" field exists, must be an object
    if let Some(props) = obj.get("properties") {
        if !props.is_object() {
            errors.push("'properties' field must be an object".to_string());
        }
    }

    // If "required" field exists, must be an array of strings
    if let Some(required) = obj.get("required") {
        if let Some(arr) = required.as_array() {
            for item in arr {
                if !item.is_string() {
                    errors.push("'required' array must contain only strings".to_string());
                    break;
                }
            }
        } else {
            errors.push("'required' field must be an array".to_string());
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_tool_has_required_fields() {
        let tool = McpToolSchema {
            name: Some("test-tool".to_string()),
            description: Some("A test tool".to_string()),
            input_schema: Some(json!({"type": "object"})),
            annotations: None,
            requires_approval: None,
            confirmation: None,
        };
        assert_eq!(tool.has_required_fields(), (true, true, true));
    }

    #[test]
    fn test_mcp_tool_missing_name() {
        let tool = McpToolSchema {
            name: None,
            description: Some("A test tool".to_string()),
            input_schema: Some(json!({"type": "object"})),
            annotations: None,
            requires_approval: None,
            confirmation: None,
        };
        assert_eq!(tool.has_required_fields(), (false, true, true));
    }

    #[test]
    fn test_mcp_tool_empty_name() {
        let tool = McpToolSchema {
            name: Some("".to_string()),
            description: Some("A test tool".to_string()),
            input_schema: Some(json!({"type": "object"})),
            annotations: None,
            requires_approval: None,
            confirmation: None,
        };
        assert_eq!(tool.has_required_fields(), (false, true, true));
    }

    #[test]
    fn test_meaningful_description() {
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: Some("This is a meaningful description".to_string()),
            input_schema: None,
            annotations: None,
            requires_approval: None,
            confirmation: None,
        };
        assert!(tool.has_meaningful_description());
    }

    #[test]
    fn test_short_description() {
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: Some("Short".to_string()),
            input_schema: None,
            annotations: None,
            requires_approval: None,
            confirmation: None,
        };
        assert!(!tool.has_meaningful_description());
    }

    #[test]
    fn test_consent_fields_requires_approval_true() {
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: None,
            input_schema: None,
            annotations: None,
            requires_approval: Some(true),
            confirmation: None,
        };
        assert!(tool.has_consent_fields());
    }

    #[test]
    fn test_consent_fields_requires_approval_false() {
        // requiresApproval: false should NOT count as having consent mechanism
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: None,
            input_schema: None,
            annotations: None,
            requires_approval: Some(false),
            confirmation: None,
        };
        assert!(!tool.has_consent_fields());
    }

    #[test]
    fn test_consent_fields_confirmation_non_empty() {
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: None,
            input_schema: None,
            annotations: None,
            requires_approval: None,
            confirmation: Some("Are you sure?".to_string()),
        };
        assert!(tool.has_consent_fields());
    }

    #[test]
    fn test_consent_fields_confirmation_empty() {
        // Empty confirmation should NOT count as having consent mechanism
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: None,
            input_schema: None,
            annotations: None,
            requires_approval: None,
            confirmation: Some("".to_string()),
        };
        assert!(!tool.has_consent_fields());
    }

    #[test]
    fn test_consent_fields_confirmation_whitespace() {
        // Whitespace-only confirmation should NOT count as having consent mechanism
        let tool = McpToolSchema {
            name: Some("test".to_string()),
            description: None,
            input_schema: None,
            annotations: None,
            requires_approval: None,
            confirmation: Some("   ".to_string()),
        };
        assert!(!tool.has_consent_fields());
    }

    #[test]
    fn test_jsonrpc_version_valid() {
        let msg = McpJsonRpcMessage {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: None,
            params: None,
            result: None,
            error: None,
        };
        assert!(msg.has_valid_jsonrpc_version());
    }

    #[test]
    fn test_jsonrpc_version_invalid() {
        let msg = McpJsonRpcMessage {
            jsonrpc: Some("1.0".to_string()),
            id: None,
            method: None,
            params: None,
            result: None,
            error: None,
        };
        assert!(!msg.has_valid_jsonrpc_version());
    }

    #[test]
    fn test_validate_schema_structure_valid() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });
        let errors = validate_json_schema_structure(&schema);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_schema_structure_invalid_type() {
        let schema = json!({
            "type": "invalid_type"
        });
        let errors = validate_json_schema_structure(&schema);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Invalid JSON Schema type"));
    }

    #[test]
    fn test_validate_schema_not_object() {
        let schema = json!("not an object");
        let errors = validate_json_schema_structure(&schema);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be an object"));
    }

    #[test]
    fn test_validate_schema_type_not_string_or_array() {
        // type field is a number - should error
        let schema = json!({"type": 123});
        let errors = validate_json_schema_structure(&schema);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be a string or array"));
    }

    #[test]
    fn test_validate_schema_type_array_with_non_string() {
        // type array contains non-string elements
        let schema = json!({"type": ["string", 123]});
        let errors = validate_json_schema_structure(&schema);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be strings"));
    }

    #[test]
    fn test_validate_schema_type_object_value() {
        // type field is an object - should error
        let schema = json!({"type": {"nested": "object"}});
        let errors = validate_json_schema_structure(&schema);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be a string or array"));
    }
}
