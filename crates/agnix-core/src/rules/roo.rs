//! Roo Code validation rules (ROO-001 to ROO-006)
//!
//! Validates:
//! - ROO-001: Empty Roo Code rule file (ERROR) - .roorules or .roo/rules/*.md must have content
//! - ROO-002: Invalid .roomodes configuration (ERROR) - JSON parse, customModes structure
//! - ROO-003: Invalid .rooignore file (WARNING) - glob pattern syntax
//! - ROO-004: Invalid mode slug in rule directory (WARNING) - slug format validation
//! - ROO-005: Invalid .roo/mcp.json configuration (ERROR) - JSON parse, mcpServers structure
//! - ROO-006: Mode slug not recognized (MEDIUM/WARNING) - slug in mode-specific SKILL.md paths

use crate::{
    config::LintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
    schemas::roo::{
        BUILTIN_MODE_SLUGS, VALID_GROUP_NAMES, extract_slug_from_path, is_valid_slug,
        parse_roo_mcp, parse_roomodes,
    },
};
use rust_i18n::t;
use std::collections::HashSet;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "ROO-001", "ROO-002", "ROO-003", "ROO-004", "ROO-005", "ROO-006",
];

pub struct RooCodeValidator;

impl Validator for RooCodeValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());
        let grandparent = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str());

        match filename {
            ".roomodes" => {
                self.validate_roomodes(path, content, config, &mut diagnostics);
            }
            ".rooignore" => {
                self.validate_rooignore(path, content, config, &mut diagnostics);
            }
            ".roorules" => {
                self.validate_roo_rules_content(path, content, config, &mut diagnostics);
            }
            "mcp.json" if parent == Some(".roo") => {
                self.validate_roo_mcp(path, content, config, &mut diagnostics);
            }
            name if name.ends_with(".md") => {
                // Mode-specific rules: .roo/rules-{slug}/*.md
                if parent.is_some_and(|p| p.starts_with("rules-")) && grandparent == Some(".roo") {
                    self.validate_mode_rules(path, content, config, &mut diagnostics);
                } else {
                    // Generic .roo/rules/*.md
                    self.validate_roo_rules_content(path, content, config, &mut diagnostics);
                }
            }
            _ => {}
        }

        diagnostics
    }
}

impl RooCodeValidator {
    /// ROO-001: Empty rule file check
    fn validate_roo_rules_content(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !config.is_rule_enabled("ROO-001") {
            return;
        }

        if content.trim().is_empty() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-001",
                    t!("rules.roo_001.message"),
                )
                .with_suggestion(t!("rules.roo_001.suggestion")),
            );
        }
    }

    /// ROO-002: Validate .roomodes configuration
    fn validate_roomodes(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !config.is_rule_enabled("ROO-002") {
            return;
        }

        let parsed = parse_roomodes(content);

        // Parse error
        if let Some(ref error) = parsed.parse_error {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    error.line,
                    error.column,
                    "ROO-002",
                    t!("rules.roo_002.parse_error", error = error.message.as_str()),
                )
                .with_suggestion(t!("rules.roo_002.suggestion")),
            );
            return;
        }

        let raw = match &parsed.raw_value {
            Some(v) => v,
            None => return,
        };

        // Check for customModes key
        if raw.get("customModes").is_none() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-002",
                    t!("rules.roo_002.missing_custom_modes"),
                )
                .with_suggestion(t!("rules.roo_002.suggestion")),
            );
            return;
        }

        // Check that customModes is an array
        if !raw.get("customModes").is_some_and(|v| v.is_array()) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-002",
                    t!("rules.roo_002.custom_modes_type"),
                )
                .with_suggestion(t!("rules.roo_002.suggestion")),
            );
            return;
        }

        // Validate each mode entry
        let mut seen_slugs = HashSet::new();
        let custom_modes_array = raw.get("customModes").and_then(|v| v.as_array());

        for (idx, mode) in parsed.modes.iter().enumerate() {
            let pos = format!("customModes[{}]", idx);

            // Check if groups field is missing in the raw JSON
            if let Some(modes_array) = custom_modes_array {
                if let Some(mode_obj) = modes_array.get(idx).and_then(|v| v.as_object()) {
                    if !mode_obj.contains_key("groups") {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "ROO-002",
                                t!("rules.roo_002.missing_groups", slug = mode.slug.as_str()),
                            )
                            .with_suggestion(t!("rules.roo_002.suggestion")),
                        );
                    } else if !mode_obj.get("groups").is_some_and(|v| v.is_array()) {
                        // groups exists but is not an array
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "ROO-002",
                                t!("rules.roo_002.groups_type", slug = mode.slug.as_str()),
                            )
                            .with_suggestion(t!("rules.roo_002.suggestion")),
                        );
                    }
                }
            }

            // Missing slug
            if mode.slug.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "ROO-002",
                        t!("rules.roo_002.missing_slug", position = pos.as_str()),
                    )
                    .with_suggestion(t!("rules.roo_002.suggestion")),
                );
                continue;
            }

            // Invalid slug format
            if !is_valid_slug(&mode.slug) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "ROO-002",
                        t!(
                            "rules.roo_002.invalid_slug",
                            slug = mode.slug.as_str(),
                            position = pos.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.roo_002.suggestion")),
                );
            }

            // Duplicate slug
            if !seen_slugs.insert(&mode.slug) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "ROO-002",
                        t!(
                            "rules.roo_002.duplicate_slug",
                            slug = mode.slug.as_str(),
                            position = pos.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.roo_002.suggestion")),
                );
            }

            // Missing name
            if mode.name.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "ROO-002",
                        t!("rules.roo_002.missing_name", position = pos.as_str()),
                    )
                    .with_suggestion(t!("rules.roo_002.suggestion")),
                );
            }

            // Missing roleDefinition
            if mode.role_definition.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "ROO-002",
                        t!(
                            "rules.roo_002.missing_role_definition",
                            position = pos.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.roo_002.suggestion")),
                );
            }

            // Invalid group names
            for group in &mode.groups {
                if !VALID_GROUP_NAMES.contains(&group.as_str()) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "ROO-002",
                            t!(
                                "rules.roo_002.invalid_group",
                                group = group.as_str(),
                                position = pos.as_str(),
                                valid = VALID_GROUP_NAMES.join(", ").as_str()
                            ),
                        )
                        .with_suggestion(t!("rules.roo_002.suggestion")),
                    );
                }
            }
        }
    }

    /// ROO-003: Validate .rooignore file
    fn validate_rooignore(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !config.is_rule_enabled("ROO-003") {
            return;
        }

        // Check if effectively empty
        let has_content = content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        });

        if !has_content {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-003",
                    t!("rules.roo_003.empty"),
                )
                .with_suggestion(t!("rules.roo_003.suggestion")),
            );
            return;
        }

        // Validate each non-comment, non-empty line as a glob pattern
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Strip negation prefix for pattern validation
            let pattern = if let Some(stripped) = trimmed.strip_prefix('!') {
                stripped
            } else {
                trimmed
            };

            if glob::Pattern::new(pattern).is_err() {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line_num + 1,
                        0,
                        "ROO-003",
                        t!(
                            "rules.roo_003.invalid_pattern",
                            line = &(line_num + 1).to_string(),
                            pattern = trimmed
                        ),
                    )
                    .with_suggestion(t!("rules.roo_003.suggestion")),
                );
            }
        }
    }

    /// ROO-004 + ROO-001 + ROO-006: Validate mode-specific rule files
    fn validate_mode_rules(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // ROO-004: Validate slug format
        if config.is_rule_enabled("ROO-004") {
            if let Some(slug) = extract_slug_from_path(path) {
                if !is_valid_slug(&slug) {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "ROO-004",
                            t!("rules.roo_004.message", slug = slug.as_str()),
                        )
                        .with_suggestion(t!("rules.roo_004.suggestion")),
                    );
                }
            }
        }

        // ROO-001: Empty content check
        self.validate_roo_rules_content(path, content, config, diagnostics);

        // ROO-006: Mode slug not recognized (for SKILL.md files in mode-specific dirs)
        if config.is_rule_enabled("ROO-006") {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename == "SKILL.md" {
                if let Some(slug) = extract_slug_from_path(path) {
                    if !BUILTIN_MODE_SLUGS.contains(&slug.as_str()) {
                        // Try to check custom modes from .roomodes
                        let is_custom_mode = self.check_custom_mode(path, &slug, config);

                        if !is_custom_mode {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "ROO-006",
                                    t!("rules.roo_006.message", slug = slug.as_str()),
                                )
                                .with_suggestion(t!("rules.roo_006.suggestion")),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Check if a slug exists in custom modes defined in .roomodes
    fn check_custom_mode(&self, path: &Path, slug: &str, config: &LintConfig) -> bool {
        // Navigate from the file path to find .roomodes
        // For a path like .roo/rules-custom-mode/SKILL.md:
        // 1. Get the .roo directory (grandparent of the file)
        // 2. Get the project root (parent of .roo)
        // 3. Check for .roomodes at the project root

        let roo_dir = path
            .ancestors()
            .find(|p| p.file_name().and_then(|n| n.to_str()) == Some(".roo"));

        if let Some(roo_dir) = roo_dir {
            if let Some(project_root) = roo_dir.parent() {
                let roomodes_path = project_root.join(".roomodes");
                let fs = config.fs();

                if fs.exists(&roomodes_path) {
                    if let Ok(content) = fs.read_to_string(&roomodes_path) {
                        let parsed = crate::schemas::roo::parse_roomodes(&content);
                        return parsed.modes.iter().any(|m| m.slug == slug);
                    }
                }
            }
        }

        false
    }

    /// ROO-005: Validate .roo/mcp.json configuration
    fn validate_roo_mcp(
        &self,
        path: &Path,
        content: &str,
        config: &LintConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !config.is_rule_enabled("ROO-005") {
            return;
        }

        let parsed = parse_roo_mcp(content);

        // Parse error
        if let Some(ref error) = parsed.parse_error {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    error.line,
                    error.column,
                    "ROO-005",
                    t!("rules.roo_005.parse_error", error = error.message.as_str()),
                )
                .with_suggestion(t!("rules.roo_005.suggestion")),
            );
            return;
        }

        let raw = match &parsed.raw_value {
            Some(v) => v,
            None => return,
        };

        // Check for mcpServers key
        if raw.get("mcpServers").is_none() {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-005",
                    t!("rules.roo_005.missing_mcp_servers"),
                )
                .with_suggestion(t!("rules.roo_005.suggestion")),
            );
            return;
        }

        // Check that mcpServers is an object
        if !raw.get("mcpServers").is_some_and(|v| v.is_object()) {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "ROO-005",
                    t!("rules.roo_005.mcp_servers_type"),
                )
                .with_suggestion(t!("rules.roo_005.suggestion")),
            );
            return;
        }

        // Validate each server entry
        for server in &parsed.servers {
            // Check that each server value is an object
            if let Some(server_val) = raw.get("mcpServers").and_then(|v| v.get(&server.name)) {
                if !server_val.is_object() {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "ROO-005",
                            t!(
                                "rules.roo_005.invalid_server_entry",
                                server = server.name.as_str()
                            ),
                        )
                        .with_suggestion(t!("rules.roo_005.suggestion")),
                    );
                    continue;
                }
            } else {
                // Server not found in raw JSON - skip validation
                continue;
            }

            // Check for required fields based on type
            let server_type = server.server_type.as_deref().unwrap_or("stdio");

            match server_type {
                "stdio" => {
                    if !server.has_command {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "ROO-005",
                                t!(
                                    "rules.roo_005.missing_command",
                                    server = server.name.as_str()
                                ),
                            )
                            .with_suggestion(t!("rules.roo_005.suggestion")),
                        );
                    }
                }
                "http" | "sse" => {
                    if !server.has_url {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "ROO-005",
                                t!("rules.roo_005.missing_url", server = server.name.as_str()),
                            )
                            .with_suggestion(t!("rules.roo_005.suggestion")),
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = RooCodeValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_with_config(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = RooCodeValidator;
        validator.validate(Path::new(path), content, config)
    }

    // ===== ROO-001: Empty rule file =====

    #[test]
    fn test_roo_001_empty_roorules() {
        let diagnostics = validate(".roorules", "");
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert_eq!(roo_001.len(), 1);
        assert_eq!(roo_001[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_roo_001_whitespace_only() {
        let diagnostics = validate(".roorules", "   \n   \n");
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert_eq!(roo_001.len(), 1);
    }

    #[test]
    fn test_roo_001_valid_content() {
        let diagnostics = validate(".roorules", "Some rule content here.");
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert!(roo_001.is_empty());
    }

    #[test]
    fn test_roo_001_empty_roo_rules_folder() {
        let diagnostics = validate(".roo/rules/general.md", "");
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert_eq!(roo_001.len(), 1);
    }

    #[test]
    fn test_roo_001_valid_roo_rules_folder() {
        let diagnostics = validate(".roo/rules/general.md", "# General rules\nBe concise.");
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert!(roo_001.is_empty());
    }

    #[test]
    fn test_roo_001_mode_rules_valid_content() {
        let diagnostics = validate(
            ".roo/rules-architect/general.md",
            "# Architect mode rules\n\nFollow the architecture patterns.",
        );
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert!(roo_001.is_empty());
    }

    // ===== ROO-002: Invalid .roomodes =====

    #[test]
    fn test_roo_002_invalid_json() {
        let diagnostics = validate(".roomodes", "{ invalid }");
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 1);
        assert_eq!(roo_002[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_roo_002_missing_custom_modes() {
        let diagnostics = validate(".roomodes", "{}");
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 1);
    }

    #[test]
    fn test_roo_002_custom_modes_not_array() {
        let diagnostics = validate(".roomodes", r#"{"customModes": "invalid"}"#);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 1);
    }

    #[test]
    fn test_roo_002_valid_roomodes() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "You are a designer.",
      "groups": ["read", "edit"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.is_empty());
    }

    #[test]
    fn test_roo_002_missing_slug() {
        let content = r#"{
  "customModes": [
    {
      "name": "Designer",
      "roleDefinition": "You are a designer.",
      "groups": ["read"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.len() >= 1);
    }

    #[test]
    fn test_roo_002_invalid_slug_format() {
        let content = r#"{
  "customModes": [
    {
      "slug": "INVALID SLUG",
      "name": "Bad Mode",
      "roleDefinition": "Role.",
      "groups": ["read"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.len() >= 1);
    }

    #[test]
    fn test_roo_002_invalid_group() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "Role.",
      "groups": ["read", "invalid-group"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 1);
    }

    #[test]
    fn test_roo_002_duplicate_slug() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "Role.",
      "groups": ["read"]
    },
    {
      "slug": "designer",
      "name": "Designer 2",
      "roleDefinition": "Role 2.",
      "groups": ["edit"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.len() >= 1);
    }

    #[test]
    fn test_roo_002_missing_name() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "roleDefinition": "Role.",
      "groups": ["read"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.len() >= 1);
    }

    #[test]
    fn test_roo_002_missing_role_definition() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "groups": ["read"]
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.len() >= 1);
    }

    #[test]
    fn test_roo_002_empty_custom_modes_array() {
        let content = r#"{"customModes": []}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 0);
    }

    #[test]
    fn test_roo_002_empty_groups_array() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "You are a designer.",
      "groups": []
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert_eq!(roo_002.len(), 0);
    }

    #[test]
    fn test_roo_002_missing_groups_field() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "You are a designer."
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        // Should error for missing groups field
        assert!(roo_002.len() >= 1);
        assert!(roo_002.iter().any(|d| d.message.contains("groups")));
    }

    #[test]
    fn test_roo_002_groups_not_array() {
        let content = r#"{
  "customModes": [
    {
      "slug": "designer",
      "name": "Designer",
      "roleDefinition": "You are a designer.",
      "groups": "not-an-array"
    }
  ]
}"#;
        let diagnostics = validate(".roomodes", content);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        // Should error for groups not being an array
        assert!(roo_002.len() >= 1);
        assert!(
            roo_002
                .iter()
                .any(|d| d.message.contains("groups") || d.message.contains("array"))
        );
    }

    // ===== ROO-003: Invalid .rooignore =====

    #[test]
    fn test_roo_003_empty_rooignore() {
        let diagnostics = validate(".rooignore", "");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert_eq!(roo_003.len(), 1);
        assert_eq!(roo_003[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_roo_003_only_comments() {
        let diagnostics = validate(".rooignore", "# Comment\n# Another\n");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert_eq!(roo_003.len(), 1);
    }

    #[test]
    fn test_roo_003_valid_content() {
        let diagnostics = validate(".rooignore", "node_modules/\n*.log\n.env\n");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert!(roo_003.is_empty());
    }

    #[test]
    fn test_roo_003_invalid_pattern() {
        let diagnostics = validate(".rooignore", "[unclosed\n*.log\n");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert_eq!(roo_003.len(), 1);
        assert_eq!(roo_003[0].line, 1);
    }

    #[test]
    fn test_roo_003_valid_negation_patterns() {
        let diagnostics = validate(".rooignore", "*.log\n!important.log\n");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert!(roo_003.is_empty());
    }

    #[test]
    fn test_roo_003_mixed_valid_invalid_patterns() {
        let diagnostics = validate(
            ".rooignore",
            "*.log\n[unclosed\nvalid-pattern.txt\n**[bracket\n",
        );
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert_eq!(roo_003.len(), 2);
        assert_eq!(roo_003[0].line, 2);
        assert_eq!(roo_003[1].line, 4);
    }

    #[test]
    fn test_roo_003_additional_invalid_patterns() {
        let diagnostics = validate(".rooignore", "**[\n[]\n[a-\n");
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert_eq!(roo_003.len(), 3);
    }

    // ===== ROO-004: Invalid mode slug =====

    #[test]
    fn test_roo_004_valid_slug() {
        let diagnostics = validate(".roo/rules-architect/general.md", "# Architect rules");
        let roo_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-004").collect();
        assert!(roo_004.is_empty());
    }

    #[test]
    fn test_roo_004_invalid_slug() {
        let diagnostics = validate(".roo/rules-INVALID SLUG/general.md", "# Bad slug rules");
        let roo_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-004").collect();
        assert_eq!(roo_004.len(), 1);
        assert_eq!(roo_004[0].level, DiagnosticLevel::Warning);
    }

    // ===== ROO-005: Invalid .roo/mcp.json =====

    #[test]
    fn test_roo_005_invalid_json() {
        let diagnostics = validate(".roo/mcp.json", "{ invalid }");
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
        assert_eq!(roo_005[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_roo_005_missing_mcp_servers() {
        let diagnostics = validate(".roo/mcp.json", "{}");
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_mcp_servers_not_object() {
        let diagnostics = validate(".roo/mcp.json", r#"{"mcpServers": "invalid"}"#);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_valid_mcp_config() {
        let content = r#"{
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["server.js"]
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert!(roo_005.is_empty());
    }

    #[test]
    fn test_roo_005_missing_command_for_stdio() {
        let content = r#"{
  "mcpServers": {
    "my-server": {
      "args": ["server.js"]
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_missing_url_for_http() {
        let content = r#"{
  "mcpServers": {
    "remote": {
      "type": "http"
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_valid_http_server() {
        let content = r#"{
  "mcpServers": {
    "remote": {
      "type": "http",
      "url": "https://example.com/mcp"
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert!(roo_005.is_empty());
    }

    #[test]
    fn test_roo_005_sse_server_missing_url() {
        let content = r#"{
  "mcpServers": {
    "my-sse": {
      "type": "sse"
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_invalid_server_entry() {
        let content = r#"{
  "mcpServers": {
    "my-server": "not-an-object"
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert_eq!(roo_005.len(), 1);
    }

    #[test]
    fn test_roo_005_unknown_server_type() {
        let content = r#"{
  "mcpServers": {
    "custom-server": {
      "type": "custom-protocol",
      "command": "custom-command"
    }
  }
}"#;
        let diagnostics = validate(".roo/mcp.json", content);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        // Unknown types are handled gracefully - no error
        assert_eq!(roo_005.len(), 0);
    }

    // ===== ROO-006: Mode slug not recognized =====

    #[test]
    fn test_roo_006_builtin_mode_no_warning() {
        let diagnostics = validate(".roo/rules-code/SKILL.md", "# Code mode skill");
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        assert!(roo_006.is_empty());
    }

    #[test]
    fn test_roo_006_custom_mode_warns() {
        let diagnostics = validate(".roo/rules-custom-mode/SKILL.md", "# Custom mode skill");
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        assert_eq!(roo_006.len(), 1);
        assert_eq!(roo_006[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_roo_006_non_skill_md_no_warning() {
        // ROO-006 only fires for SKILL.md, not other .md files
        let diagnostics = validate(
            ".roo/rules-custom-mode/general.md",
            "# General rules for custom mode",
        );
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        assert!(roo_006.is_empty());
    }

    #[test]
    fn test_roo_006_all_builtin_slugs() {
        // Verify none of the builtin slugs trigger ROO-006
        for slug in BUILTIN_MODE_SLUGS {
            let path = format!(".roo/rules-{}/SKILL.md", slug);
            let diagnostics = validate(&path, "# Skill content");
            let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
            assert!(
                roo_006.is_empty(),
                "Builtin slug '{}' should not trigger ROO-006",
                slug
            );
        }
    }

    #[test]
    fn test_roo_006_custom_mode_with_roomodes_no_warning() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        // Create a mock filesystem with .roomodes defining custom-designer mode
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(
            ".roomodes",
            r#"{
  "customModes": [
    {
      "slug": "custom-designer",
      "name": "Designer",
      "roleDefinition": "You are a UI/UX designer.",
      "groups": ["read", "edit"]
    }
  ]
}"#,
        );
        fs.add_file(
            ".roo/rules-custom-designer/SKILL.md",
            "# Custom designer mode",
        );

        let config = LintConfig::builder().fs(fs).build_unchecked();

        let diagnostics = validate_with_config(
            ".roo/rules-custom-designer/SKILL.md",
            "# Custom designer mode",
            &config,
        );
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        // Should not warn because custom-designer is in .roomodes
        assert!(roo_006.is_empty());
    }

    #[test]
    fn test_roo_006_custom_mode_without_roomodes_warns() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        // Create a mock filesystem without .roomodes
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(".roo/rules-unknown-mode/SKILL.md", "# Unknown mode");

        let config = LintConfig::builder().fs(fs).build_unchecked();

        let diagnostics = validate_with_config(
            ".roo/rules-unknown-mode/SKILL.md",
            "# Unknown mode",
            &config,
        );
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        // Should warn because unknown-mode is not in BUILTIN_MODE_SLUGS and no .roomodes exists
        assert_eq!(roo_006.len(), 1);
    }

    #[test]
    fn test_roo_006_custom_mode_in_roomodes_different_slug_warns() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;

        // Create a mock filesystem with .roomodes defining one mode, but using a different slug
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(
            ".roomodes",
            r#"{
  "customModes": [
    {
      "slug": "custom-designer",
      "name": "Designer",
      "roleDefinition": "You are a UI/UX designer.",
      "groups": ["read"]
    }
  ]
}"#,
        );
        fs.add_file(".roo/rules-different-mode/SKILL.md", "# Different mode");

        let config = LintConfig::builder().fs(fs).build_unchecked();

        let diagnostics = validate_with_config(
            ".roo/rules-different-mode/SKILL.md",
            "# Different mode",
            &config,
        );
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        // Should warn because different-mode is not in customModes
        assert_eq!(roo_006.len(), 1);
    }

    // ===== Rule disabling =====

    #[test]
    fn test_roo_001_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-001".to_string()];

        let diagnostics = validate_with_config(".roorules", "", &config);
        let roo_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-001").collect();
        assert!(roo_001.is_empty());
    }

    #[test]
    fn test_roo_002_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-002".to_string()];

        let diagnostics = validate_with_config(".roomodes", "{ invalid }", &config);
        let roo_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-002").collect();
        assert!(roo_002.is_empty());
    }

    #[test]
    fn test_roo_003_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-003".to_string()];

        let diagnostics = validate_with_config(".rooignore", "", &config);
        let roo_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-003").collect();
        assert!(roo_003.is_empty());
    }

    #[test]
    fn test_roo_004_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-004".to_string()];

        let diagnostics = validate_with_config(
            ".roo/rules-INVALID SLUG/general.md",
            "# Bad slug rules",
            &config,
        );
        let roo_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-004").collect();
        assert!(roo_004.is_empty());
    }

    #[test]
    fn test_roo_005_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-005".to_string()];

        let diagnostics = validate_with_config(".roo/mcp.json", "{ invalid }", &config);
        let roo_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-005").collect();
        assert!(roo_005.is_empty());
    }

    #[test]
    fn test_roo_006_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["ROO-006".to_string()];

        let diagnostics = validate_with_config(
            ".roo/rules-custom-mode/SKILL.md",
            "# Custom mode skill",
            &config,
        );
        let roo_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "ROO-006").collect();
        assert!(roo_006.is_empty());
    }

    #[test]
    fn test_roo_category_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().roo_code = false;

        let diagnostics = validate_with_config(".roorules", "", &config);
        let roo_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("ROO-"))
            .collect();
        assert!(roo_rules.is_empty());
    }

    // ===== Metadata =====

    #[test]
    fn test_roo_validator_metadata() {
        let validator = RooCodeValidator;
        let meta = validator.metadata();
        assert_eq!(meta.name, "RooCodeValidator");
        assert_eq!(
            meta.rule_ids,
            &[
                "ROO-001", "ROO-002", "ROO-003", "ROO-004", "ROO-005", "ROO-006"
            ]
        );
    }
}
