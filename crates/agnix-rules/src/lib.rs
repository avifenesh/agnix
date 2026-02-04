//! Validation rules for agnix - agent configuration linter.
//!
//! This crate provides the rule definitions used by agnix to validate
//! agent configurations including Skills, Hooks, MCP servers, Memory files,
//! and Plugins.
//!
//! # Usage
//!
//! ```
//! use agnix_rules::RULES_DATA;
//!
//! // RULES_DATA is a static array of (rule_id, rule_name) tuples
//! for (id, name) in RULES_DATA {
//!     println!("{}: {}", id, name);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
}
