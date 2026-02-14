use super::*;

/// Rule filtering logic encapsulated for clarity.
///
/// This trait and its implementation extract the rule enablement logic
/// from LintConfig, making it easier to test and maintain.
trait RuleFilter {
    /// Check if a specific rule is enabled based on config.
    fn is_rule_enabled(&self, rule_id: &str) -> bool;
}

/// Default implementation of rule filtering logic.
///
/// Determines whether a rule is enabled based on:
/// 1. Explicit disabled_rules list
/// 2. Target tool or tools array filtering
/// 3. Category enablement flags
struct DefaultRuleFilter<'a> {
    rules: &'a RuleConfig,
    target: TargetTool,
    tools: &'a [String],
}

impl<'a> DefaultRuleFilter<'a> {
    fn new(rules: &'a RuleConfig, target: TargetTool, tools: &'a [String]) -> Self {
        Self {
            rules,
            target,
            tools,
        }
    }

    /// Check if a rule applies to the current target tool(s)
    fn is_rule_for_target(&self, rule_id: &str) -> bool {
        // If tools array is specified, use it for filtering
        if !self.tools.is_empty() {
            return self.is_rule_for_tools(rule_id);
        }

        // Legacy: CC-* rules only apply to ClaudeCode or Generic targets
        if rule_id.starts_with("CC-") {
            return matches!(self.target, TargetTool::ClaudeCode | TargetTool::Generic);
        }
        // All other rules apply to all targets (see TOOL_RULE_PREFIXES for tool-specific rules)
        true
    }

    /// Check if a rule applies based on the tools array
    fn is_rule_for_tools(&self, rule_id: &str) -> bool {
        for (prefix, tool) in agnix_rules::TOOL_RULE_PREFIXES {
            if rule_id.starts_with(prefix) {
                // Check if the required tool is in the tools list (case-insensitive)
                // Also accept backward-compat aliases (e.g., "copilot" for "github-copilot")
                return self
                    .tools
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tool) || Self::is_tool_alias(t, tool));
            }
        }

        // Generic rules (AS-*, XML-*, REF-*, XP-*, AGM-*, MCP-*, PE-*) apply to all tools
        true
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        // Backward compatibility: accept short names as aliases
        match canonical_tool {
            "github-copilot" => user_tool.eq_ignore_ascii_case("copilot"),
            _ => false,
        }
    }

    /// Check if a rule's category is enabled
    fn is_category_enabled(&self, rule_id: &str) -> bool {
        match rule_id {
            s if [
                "AS-", "CC-SK-", "CR-SK-", "CL-SK-", "CP-SK-", "CX-SK-", "OC-SK-", "WS-SK-",
                "KR-SK-", "AMP-SK-", "RC-SK-",
            ]
            .iter()
            .any(|p| s.starts_with(p)) =>
            {
                self.rules.skills
            }
            s if s.starts_with("CC-HK-") => self.rules.hooks,
            s if s.starts_with("CC-AG-") => self.rules.agents,
            s if s.starts_with("CC-MEM-") => self.rules.memory,
            s if s.starts_with("CC-PL-") => self.rules.plugins,
            s if s.starts_with("XML-") => self.rules.xml,
            s if s.starts_with("MCP-") => self.rules.mcp,
            s if s.starts_with("REF-") || s.starts_with("imports::") => self.rules.imports,
            s if s.starts_with("XP-") => self.rules.cross_platform,
            s if s.starts_with("AGM-") => self.rules.agents_md,
            s if s.starts_with("COP-") => self.rules.copilot,
            s if s.starts_with("CUR-") => self.rules.cursor,
            s if s.starts_with("CLN-") => self.rules.cline,
            s if s.starts_with("OC-") => self.rules.opencode,
            s if s.starts_with("GM-") => self.rules.gemini_md,
            s if s.starts_with("CDX-") => self.rules.codex,
            s if s.starts_with("ROO-") => self.rules.roo_code,
            s if s.starts_with("PE-") => self.rules.prompt_engineering,
            // Unknown rules are enabled by default
            _ => true,
        }
    }
}

impl RuleFilter for DefaultRuleFilter<'_> {
    fn is_rule_enabled(&self, rule_id: &str) -> bool {
        // Check if explicitly disabled
        if self.rules.disabled_rules.iter().any(|r| r == rule_id) {
            return false;
        }

        // Check if rule applies to target
        if !self.is_rule_for_target(rule_id) {
            return false;
        }

        // Check if category is enabled
        self.is_category_enabled(rule_id)
    }
}

impl LintConfig {
    // =========================================================================
    // Rule Filtering (delegates to DefaultRuleFilter)
    // =========================================================================

    /// Check if a specific rule is enabled based on config
    ///
    /// A rule is enabled if:
    /// 1. It's not in the disabled_rules list
    /// 2. It's applicable to the current target tool
    /// 3. Its category is enabled
    ///
    /// This delegates to `DefaultRuleFilter` which encapsulates the filtering logic.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        let filter = DefaultRuleFilter::new(&self.rules, self.target, &self.tools);
        filter.is_rule_enabled(rule_id)
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    pub fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        DefaultRuleFilter::is_tool_alias(user_tool, canonical_tool)
    }
}
