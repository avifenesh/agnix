use super::*;

impl LintConfig {
    /// Validate the configuration and return any warnings.
    ///
    /// This performs semantic validation beyond what TOML parsing can check:
    /// - Validates that disabled_rules match known rule ID patterns
    /// - Validates that tools array contains known tool names
    /// - Warns on deprecated fields
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        // Validate disabled_rules match known patterns
        // Note: imports:: is a legacy prefix used in some internal diagnostics
        let known_prefixes = [
            "AS-",
            "CC-SK-",
            "CC-HK-",
            "CC-AG-",
            "CC-MEM-",
            "CC-PL-",
            "CDX-",
            "XML-",
            "MCP-",
            "REF-",
            "XP-",
            "AGM-",
            "COP-",
            "CUR-",
            "CLN-",
            "OC-",
            "GM-",
            "PE-",
            "VER-",
            "ROO-",
            "AMP-",
            "imports::",
        ];
        for rule_id in &self.rules.disabled_rules {
            let matches_known = known_prefixes
                .iter()
                .any(|prefix| rule_id.starts_with(prefix));
            if !matches_known {
                warnings.push(ConfigWarning {
                    field: "rules.disabled_rules".to_string(),
                    message: t!(
                        "core.config.unknown_rule",
                        rule = rule_id.as_str(),
                        prefixes = known_prefixes.join(", ")
                    )
                    .to_string(),
                    suggestion: Some(t!("core.config.unknown_rule_suggestion").to_string()),
                });
            }
        }

        // Validate tools array contains known tools
        let known_tools = [
            "claude-code",
            "cursor",
            "codex",
            "copilot",
            "github-copilot",
            "cline",
            "opencode",
            "gemini-cli",
            "amp",
            "roo-code",
            "generic",
        ];
        for tool in &self.tools {
            let tool_lower = tool.to_lowercase();
            if !known_tools
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&tool_lower))
            {
                warnings.push(ConfigWarning {
                    field: "tools".to_string(),
                    message: t!(
                        "core.config.unknown_tool",
                        tool = tool.as_str(),
                        valid = known_tools.join(", ")
                    )
                    .to_string(),
                    suggestion: Some(t!("core.config.unknown_tool_suggestion").to_string()),
                });
            }
        }

        // Warn on deprecated fields
        if self.target != TargetTool::Generic && self.tools.is_empty() {
            // Only warn if target is non-default and tools is empty
            // (if both are set, tools takes precedence silently)
            warnings.push(ConfigWarning {
                field: "target".to_string(),
                message: t!("core.config.deprecated_target").to_string(),
                suggestion: Some(t!("core.config.deprecated_target_suggestion").to_string()),
            });
        }
        if self.mcp_protocol_version.is_some() {
            warnings.push(ConfigWarning {
                field: "mcp_protocol_version".to_string(),
                message: t!("core.config.deprecated_mcp_version").to_string(),
                suggestion: Some(t!("core.config.deprecated_mcp_version_suggestion").to_string()),
            });
        }

        // Validate files config glob patterns
        let pattern_lists = [
            ("files.include_as_memory", &self.files.include_as_memory),
            ("files.include_as_generic", &self.files.include_as_generic),
            ("files.exclude", &self.files.exclude),
        ];
        for (field, patterns) in &pattern_lists {
            // Warn if pattern count exceeds recommended limit
            if patterns.len() > MAX_FILE_PATTERNS {
                warnings.push(ConfigWarning {
                    field: field.to_string(),
                    message: t!(
                        "core.config.files_pattern_count_limit",
                        field = *field,
                        count = patterns.len(),
                        limit = MAX_FILE_PATTERNS
                    )
                    .to_string(),
                    suggestion: Some(
                        t!("core.config.files_pattern_count_limit_suggestion").to_string(),
                    ),
                });
            }
            for pattern in *patterns {
                let normalized = pattern.replace('\\', "/");
                if let Err(e) = glob::Pattern::new(&normalized) {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.invalid_files_pattern",
                            pattern = pattern.as_str(),
                            message = e.to_string()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.invalid_files_pattern_suggestion").to_string(),
                        ),
                    });
                }
                // Reject path traversal patterns
                if has_path_traversal(&normalized) {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.files_path_traversal",
                            pattern = pattern.as_str()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.files_path_traversal_suggestion").to_string(),
                        ),
                    });
                }
                // Reject absolute paths (Unix-style leading slash or Windows drive letter)
                if normalized.starts_with('/')
                    || (normalized.len() >= 3
                        && normalized.as_bytes()[0].is_ascii_alphabetic()
                        && normalized.as_bytes().get(1..3) == Some(b":/"))
                {
                    warnings.push(ConfigWarning {
                        field: field.to_string(),
                        message: t!(
                            "core.config.files_absolute_path",
                            pattern = pattern.as_str()
                        )
                        .to_string(),
                        suggestion: Some(
                            t!("core.config.files_absolute_path_suggestion").to_string(),
                        ),
                    });
                }
            }
        }

        warnings
    }
}

/// Warning from configuration validation.
///
/// These warnings indicate potential issues with the configuration that
/// don't prevent validation from running but may indicate user mistakes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    /// The field path that has the issue (e.g., "rules.disabled_rules")
    pub field: String,
    /// Description of the issue
    pub message: String,
    /// Optional suggestion for how to fix the issue
    pub suggestion: Option<String>,
}

/// Generate a JSON Schema for the LintConfig type.
///
/// This can be used to provide editor autocompletion and validation
/// for `.agnix.toml` configuration files.
///
/// # Example
///
/// ```rust
/// use agnix_core::config::generate_schema;
///
/// let schema = generate_schema();
/// let json = serde_json::to_string_pretty(&schema).unwrap();
/// println!("{}", json);
/// ```
pub fn generate_schema() -> schemars::Schema {
    schemars::schema_for!(LintConfig)
}
