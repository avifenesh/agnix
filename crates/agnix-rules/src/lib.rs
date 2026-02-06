//! Validation rules for agnix - agent configuration linter.
//!
//! This crate provides the rule definitions used by agnix to validate
//! agent configurations including Skills, Hooks, MCP servers, Memory files,
//! and Plugins.
//!
//! # Usage
//!
//! ```
//! use agnix_rules::{RULES_DATA, VALID_TOOLS, TOOL_RULE_PREFIXES};
//!
//! // RULES_DATA is a static array of (rule_id, rule_name) tuples
//! for (id, name) in RULES_DATA {
//!     println!("{}: {}", id, name);
//! }
//!
//! // VALID_TOOLS contains all tool names from rules.json
//! for tool in VALID_TOOLS {
//!     println!("Tool: {}", tool);
//! }
//!
//! // TOOL_RULE_PREFIXES maps rule prefixes to their tools
//! for (prefix, tool) in TOOL_RULE_PREFIXES {
//!     println!("Prefix {} -> Tool {}", prefix, tool);
//! }
//! ```
//!
//! # Rule Categories
//!
//! - **AS-xxx**: Agent Skills
//! - **CC-xxx**: Claude Code (Hooks, Skills, Memory, etc.)
//! - **MCP-xxx**: Model Context Protocol
//! - **COP-xxx**: GitHub Copilot
//! - **CUR-xxx**: Cursor
//! - **XML-xxx**: XML/XSLT based configs
//! - **XP-xxx**: Cross-platform rules

// Include the auto-generated rules data from build.rs
include!(concat!(env!("OUT_DIR"), "/rules_data.rs"));

/// Returns the total number of rules.
pub fn rule_count() -> usize {
    RULES_DATA.len()
}

/// Looks up a rule by ID, returning the name if found.
pub fn get_rule_name(id: &str) -> Option<&'static str> {
    RULES_DATA
        .iter()
        .find(|(rule_id, _)| *rule_id == id)
        .map(|(_, name)| *name)
}

/// Returns the list of valid tool names derived from rules.json.
///
/// These are tools that have at least one rule specifically targeting them.
pub fn valid_tools() -> &'static [&'static str] {
    VALID_TOOLS
}

/// Returns authoring family IDs derived from rules.json `authoring.families`.
pub fn authoring_families() -> &'static [&'static str] {
    AUTHORING_FAMILIES
}

/// Returns the raw authoring catalog JSON generated from rules.json.
pub fn authoring_catalog_json() -> &'static str {
    AUTHORING_CATALOG_JSON
}

/// Returns the tool name for a given rule ID prefix, if any.
///
/// Only returns a tool if ALL rules with that prefix have the same tool.
/// Prefixes with mixed tools or no tools return None.
///
/// # Example
/// ```
/// use agnix_rules::get_tool_for_prefix;
///
/// assert_eq!(get_tool_for_prefix("CC-HK-"), Some("claude-code"));
/// assert_eq!(get_tool_for_prefix("COP-"), Some("github-copilot"));
/// assert_eq!(get_tool_for_prefix("CUR-"), Some("cursor"));
/// // Generic prefixes without a consistent tool return None
/// assert_eq!(get_tool_for_prefix("MCP-"), None);
/// ```
pub fn get_tool_for_prefix(prefix: &str) -> Option<&'static str> {
    TOOL_RULE_PREFIXES
        .iter()
        .find(|(p, _)| *p == prefix)
        .map(|(_, tool)| *tool)
}

/// Returns all rule prefixes associated with a tool.
///
/// # Example
/// ```
/// use agnix_rules::get_prefixes_for_tool;
///
/// let prefixes = get_prefixes_for_tool("claude-code");
/// assert!(prefixes.contains(&"CC-HK-"));
/// assert!(prefixes.contains(&"CC-SK-"));
/// ```
pub fn get_prefixes_for_tool(tool: &str) -> Vec<&'static str> {
    TOOL_RULE_PREFIXES
        .iter()
        .filter(|(_, t)| t.eq_ignore_ascii_case(tool))
        .map(|(prefix, _)| *prefix)
        .collect()
}

/// Check if a tool name is valid (exists in rules.json).
///
/// This performs case-insensitive matching.
pub fn is_valid_tool(tool: &str) -> bool {
    VALID_TOOLS.iter().any(|t| t.eq_ignore_ascii_case(tool))
}

/// Normalize a tool name to its canonical form from rules.json.
///
/// Returns the canonical name if found, None otherwise.
/// Performs case-insensitive matching.
///
/// # Example
/// ```
/// use agnix_rules::normalize_tool_name;
///
/// assert_eq!(normalize_tool_name("Claude-Code"), Some("claude-code"));
/// assert_eq!(normalize_tool_name("GITHUB-COPILOT"), Some("github-copilot"));
/// assert_eq!(normalize_tool_name("unknown"), None);
/// ```
pub fn normalize_tool_name(tool: &str) -> Option<&'static str> {
    VALID_TOOLS
        .iter()
        .find(|t| t.eq_ignore_ascii_case(tool))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_rules_data_not_empty() {
        assert!(!RULES_DATA.is_empty(), "RULES_DATA should not be empty");
    }

    #[test]
    fn test_rule_count() {
        assert_eq!(rule_count(), RULES_DATA.len());
    }

    #[test]
    fn test_get_rule_name_exists() {
        // AS-001 should always exist
        let name = get_rule_name("AS-001");
        assert!(name.is_some(), "AS-001 should exist");
    }

    #[test]
    fn test_get_rule_name_not_exists() {
        let name = get_rule_name("NONEXISTENT-999");
        assert!(name.is_none(), "Nonexistent rule should return None");
    }

    #[test]
    fn test_no_duplicate_ids() {
        let mut ids: Vec<&str> = RULES_DATA.iter().map(|(id, _)| *id).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Should have no duplicate rule IDs");
    }

    // ===== VALID_TOOLS Tests =====

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_valid_tools_not_empty() {
        assert!(!VALID_TOOLS.is_empty(), "VALID_TOOLS should not be empty");
    }

    #[test]
    fn test_valid_tools_contains_claude_code() {
        assert!(
            VALID_TOOLS.contains(&"claude-code"),
            "VALID_TOOLS should contain 'claude-code'"
        );
    }

    #[test]
    fn test_valid_tools_contains_github_copilot() {
        assert!(
            VALID_TOOLS.contains(&"github-copilot"),
            "VALID_TOOLS should contain 'github-copilot'"
        );
    }

    #[test]
    fn test_valid_tools_contains_cursor() {
        assert!(
            VALID_TOOLS.contains(&"cursor"),
            "VALID_TOOLS should contain 'cursor'"
        );
    }

    #[test]
    fn test_valid_tools_helper() {
        let tools = valid_tools();
        assert!(!tools.is_empty());
        assert!(tools.contains(&"claude-code"));
    }

    // ===== AUTHORING catalog tests =====

    #[test]
    fn test_authoring_families_not_empty() {
        assert!(
            !AUTHORING_FAMILIES.is_empty(),
            "AUTHORING_FAMILIES should not be empty"
        );
    }

    #[test]
    fn test_authoring_families_contains_core_families() {
        let families = authoring_families();
        assert!(families.contains(&"skill"));
        assert!(families.contains(&"agent"));
        assert!(families.contains(&"hooks"));
        assert!(families.contains(&"mcp"));
    }

    #[test]
    fn test_authoring_catalog_json_is_valid_json() {
        let parsed: serde_json::Value = serde_json::from_str(authoring_catalog_json())
            .expect("AUTHORING_CATALOG_JSON should be valid JSON");
        assert!(
            parsed.is_object(),
            "authoring catalog should be a JSON object"
        );
    }

    // ===== TOOL_RULE_PREFIXES Tests =====

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_tool_rule_prefixes_not_empty() {
        assert!(
            !TOOL_RULE_PREFIXES.is_empty(),
            "TOOL_RULE_PREFIXES should not be empty"
        );
    }

    #[test]
    fn test_tool_rule_prefixes_cc_hk() {
        // CC-HK-* rules are for claude-code
        let found = TOOL_RULE_PREFIXES
            .iter()
            .find(|(prefix, _)| *prefix == "CC-HK-");
        assert!(found.is_some(), "Should have CC-HK- prefix");
        assert_eq!(found.unwrap().1, "claude-code");
    }

    #[test]
    fn test_tool_rule_prefixes_cop() {
        // COP-* rules are for github-copilot
        let found = TOOL_RULE_PREFIXES
            .iter()
            .find(|(prefix, _)| *prefix == "COP-");
        assert!(found.is_some(), "Should have COP- prefix");
        assert_eq!(found.unwrap().1, "github-copilot");
    }

    #[test]
    fn test_tool_rule_prefixes_cur() {
        // CUR-* rules are for cursor
        let found = TOOL_RULE_PREFIXES
            .iter()
            .find(|(prefix, _)| *prefix == "CUR-");
        assert!(found.is_some(), "Should have CUR- prefix");
        assert_eq!(found.unwrap().1, "cursor");
    }

    #[test]
    fn test_get_tool_for_prefix_claude_code() {
        assert_eq!(get_tool_for_prefix("CC-HK-"), Some("claude-code"));
        assert_eq!(get_tool_for_prefix("CC-SK-"), Some("claude-code"));
        assert_eq!(get_tool_for_prefix("CC-AG-"), Some("claude-code"));
        assert_eq!(get_tool_for_prefix("CC-PL-"), Some("claude-code"));
        // Note: CC-MEM- is NOT in the mapping because some CC-MEM-* rules
        // have empty applies_to (making them generic)
        assert_eq!(get_tool_for_prefix("CC-MEM-"), None);
    }

    #[test]
    fn test_get_tool_for_prefix_copilot() {
        assert_eq!(get_tool_for_prefix("COP-"), Some("github-copilot"));
    }

    #[test]
    fn test_get_tool_for_prefix_cursor() {
        assert_eq!(get_tool_for_prefix("CUR-"), Some("cursor"));
    }

    #[test]
    fn test_get_tool_for_prefix_generic() {
        // These prefixes have no tool specified, so they are not in the mapping
        // MCP-*, XML-*, XP-* rules don't specify a tool - they're generic
        assert_eq!(get_tool_for_prefix("MCP-"), None);
        assert_eq!(get_tool_for_prefix("XML-"), None);
        assert_eq!(get_tool_for_prefix("XP-"), None);
        // Note: Some prefixes like AS-*, PE-*, AGM-*, REF-* have mixed tools
        // (some rules have tool, some don't), so the build script excludes them
        // from the mapping to avoid ambiguity
    }

    #[test]
    fn test_get_tool_for_prefix_unknown() {
        assert_eq!(get_tool_for_prefix("UNKNOWN-"), None);
    }

    // ===== Mixed-Tool Prefix Scenario Tests (Review-requested coverage) =====

    #[test]
    fn test_mixed_tool_prefix_as() {
        // AS-* prefix is all generic (no tool specified for any AS-* rule)
        // so it returns None - no consistent tool
        assert_eq!(get_tool_for_prefix("AS-"), None);
    }

    #[test]
    fn test_mixed_tool_prefix_cc_mem() {
        // CC-MEM-* prefix has mixed tools: some rules have "claude-code",
        // some have null/no tool. Since they're not all the same tool,
        // the prefix returns None.
        assert_eq!(get_tool_for_prefix("CC-MEM-"), None);
    }

    #[test]
    fn test_consistent_tool_prefix_cc_hk() {
        // CC-HK-* prefix is consistent: all rules have "claude-code"
        // so it returns Some("claude-code")
        assert_eq!(get_tool_for_prefix("CC-HK-"), Some("claude-code"));
    }

    #[test]
    fn test_get_prefixes_for_tool_claude_code() {
        let prefixes = get_prefixes_for_tool("claude-code");
        assert!(!prefixes.is_empty());
        assert!(prefixes.contains(&"CC-HK-"));
        assert!(prefixes.contains(&"CC-SK-"));
        assert!(prefixes.contains(&"CC-AG-"));
        assert!(prefixes.contains(&"CC-PL-"));
        // Note: CC-MEM- is NOT in the list because some CC-MEM-* rules
        // have empty applies_to (making them generic rules)
        assert!(!prefixes.contains(&"CC-MEM-"));
    }

    #[test]
    fn test_get_prefixes_for_tool_copilot() {
        let prefixes = get_prefixes_for_tool("github-copilot");
        assert!(!prefixes.is_empty());
        assert!(prefixes.contains(&"COP-"));
    }

    #[test]
    fn test_get_prefixes_for_tool_cursor() {
        let prefixes = get_prefixes_for_tool("cursor");
        assert!(!prefixes.is_empty());
        assert!(prefixes.contains(&"CUR-"));
    }

    #[test]
    fn test_get_prefixes_for_tool_unknown() {
        let prefixes = get_prefixes_for_tool("unknown-tool");
        assert!(prefixes.is_empty());
    }

    // ===== is_valid_tool Tests =====

    #[test]
    fn test_is_valid_tool_claude_code() {
        assert!(is_valid_tool("claude-code"));
        assert!(is_valid_tool("Claude-Code")); // case insensitive
        assert!(is_valid_tool("CLAUDE-CODE")); // case insensitive
    }

    #[test]
    fn test_is_valid_tool_copilot() {
        assert!(is_valid_tool("github-copilot"));
        assert!(is_valid_tool("GitHub-Copilot")); // case insensitive
    }

    #[test]
    fn test_is_valid_tool_unknown() {
        assert!(!is_valid_tool("unknown-tool"));
        assert!(!is_valid_tool(""));
    }

    // ===== normalize_tool_name Tests =====

    #[test]
    fn test_normalize_tool_name_claude_code() {
        assert_eq!(normalize_tool_name("claude-code"), Some("claude-code"));
        assert_eq!(normalize_tool_name("Claude-Code"), Some("claude-code"));
        assert_eq!(normalize_tool_name("CLAUDE-CODE"), Some("claude-code"));
    }

    #[test]
    fn test_normalize_tool_name_copilot() {
        assert_eq!(
            normalize_tool_name("github-copilot"),
            Some("github-copilot")
        );
        assert_eq!(
            normalize_tool_name("GitHub-Copilot"),
            Some("github-copilot")
        );
    }

    #[test]
    fn test_normalize_tool_name_unknown() {
        assert_eq!(normalize_tool_name("unknown-tool"), None);
        assert_eq!(normalize_tool_name(""), None);
    }

    // ===== get_prefixes_for_tool Edge Case Tests =====

    #[test]
    fn test_get_prefixes_for_tool_empty_string() {
        // Empty string should return empty Vec (no tool matches empty)
        let prefixes = get_prefixes_for_tool("");
        assert!(
            prefixes.is_empty(),
            "Empty string tool should return empty Vec"
        );
    }

    #[test]
    fn test_get_prefixes_for_tool_unknown_tool() {
        // Unknown tool should return empty Vec
        let prefixes = get_prefixes_for_tool("nonexistent-tool");
        assert!(prefixes.is_empty(), "Unknown tool should return empty Vec");
    }

    #[test]
    fn test_get_prefixes_for_tool_claude_code_multiple_prefixes() {
        // claude-code should have multiple prefixes (CC-HK-, CC-SK-, CC-AG-, CC-PL-)
        let prefixes = get_prefixes_for_tool("claude-code");
        assert!(
            prefixes.len() > 1,
            "claude-code should have multiple prefixes, got {}",
            prefixes.len()
        );
        // Verify some expected prefixes are present
        assert!(
            prefixes.contains(&"CC-HK-"),
            "claude-code prefixes should include CC-HK-"
        );
        assert!(
            prefixes.contains(&"CC-SK-"),
            "claude-code prefixes should include CC-SK-"
        );
    }

    // ===== get_tool_for_prefix Edge Case Tests =====

    #[test]
    fn test_get_tool_for_prefix_empty_string() {
        // Empty prefix should return None
        assert_eq!(
            get_tool_for_prefix(""),
            None,
            "Empty prefix should return None"
        );
    }

    #[test]
    fn test_get_tool_for_prefix_unknown_prefix() {
        // Unknown prefix should return None
        assert_eq!(
            get_tool_for_prefix("NONEXISTENT-"),
            None,
            "Unknown prefix should return None"
        );
        assert_eq!(
            get_tool_for_prefix("XX-"),
            None,
            "XX- prefix should return None"
        );
    }

    #[test]
    fn test_get_tool_for_prefix_partial_match_not_supported() {
        // Partial prefixes should not match
        assert_eq!(
            get_tool_for_prefix("CC-"),
            None,
            "Partial prefix CC- (without HK/SK/AG) should not match"
        );
        assert_eq!(
            get_tool_for_prefix("C"),
            None,
            "Single character should not match"
        );
    }
}
