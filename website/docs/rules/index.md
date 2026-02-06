# Rules Reference

This section contains all `100` validation rules generated from `knowledge-base/rules.json`.

| Rule | Name | Severity | Category |
|------|------|----------|----------|
| [AS-001](./generated/as-001.md) | Missing Frontmatter | HIGH | Agent Skills |
| [AS-002](./generated/as-002.md) | Missing Required Field: name | HIGH | Agent Skills |
| [AS-003](./generated/as-003.md) | Missing Required Field: description | HIGH | Agent Skills |
| [AS-004](./generated/as-004.md) | Invalid Name Format | HIGH | Agent Skills |
| [AS-005](./generated/as-005.md) | Name Starts/Ends with Hyphen | HIGH | Agent Skills |
| [AS-006](./generated/as-006.md) | Consecutive Hyphens in Name | HIGH | Agent Skills |
| [AS-007](./generated/as-007.md) | Reserved Name | HIGH | Agent Skills |
| [AS-008](./generated/as-008.md) | Description Too Short | HIGH | Agent Skills |
| [AS-009](./generated/as-009.md) | Description Contains XML | HIGH | Agent Skills |
| [AS-010](./generated/as-010.md) | Missing Trigger Phrase | MEDIUM | Agent Skills |
| [AS-011](./generated/as-011.md) | Compatibility Too Long | HIGH | Agent Skills |
| [AS-012](./generated/as-012.md) | Content Exceeds 500 Lines | MEDIUM | Agent Skills |
| [AS-013](./generated/as-013.md) | File Reference Too Deep | HIGH | Agent Skills |
| [AS-014](./generated/as-014.md) | Windows Path Separator | HIGH | Agent Skills |
| [AS-015](./generated/as-015.md) | Upload Size Exceeds 8MB | HIGH | Agent Skills |
| [AS-016](./generated/as-016.md) | Skill Parse Error | HIGH | Agent Skills |
| [CC-SK-001](./generated/cc-sk-001.md) | Invalid Model Value | HIGH | Claude Skills |
| [CC-SK-002](./generated/cc-sk-002.md) | Invalid Context Value | HIGH | Claude Skills |
| [CC-SK-003](./generated/cc-sk-003.md) | Context Without Agent | HIGH | Claude Skills |
| [CC-SK-004](./generated/cc-sk-004.md) | Agent Without Context | HIGH | Claude Skills |
| [CC-SK-005](./generated/cc-sk-005.md) | Invalid Agent Type | HIGH | Claude Skills |
| [CC-SK-006](./generated/cc-sk-006.md) | Dangerous Auto-Invocation | HIGH | Claude Skills |
| [CC-SK-007](./generated/cc-sk-007.md) | Unrestricted Bash | HIGH | Claude Skills |
| [CC-SK-008](./generated/cc-sk-008.md) | Unknown Tool Name | HIGH | Claude Skills |
| [CC-SK-009](./generated/cc-sk-009.md) | Too Many Injections | MEDIUM | Claude Skills |
| [CC-HK-001](./generated/cc-hk-001.md) | Invalid Hook Event | HIGH | Claude Hooks |
| [CC-HK-002](./generated/cc-hk-002.md) | Prompt Hook on Wrong Event | HIGH | Claude Hooks |
| [CC-HK-003](./generated/cc-hk-003.md) | Missing Matcher for Tool Events | HIGH | Claude Hooks |
| [CC-HK-004](./generated/cc-hk-004.md) | Matcher on Non-Tool Event | HIGH | Claude Hooks |
| [CC-HK-005](./generated/cc-hk-005.md) | Missing Type Field | HIGH | Claude Hooks |
| [CC-HK-006](./generated/cc-hk-006.md) | Missing Command Field | HIGH | Claude Hooks |
| [CC-HK-007](./generated/cc-hk-007.md) | Missing Prompt Field | HIGH | Claude Hooks |
| [CC-HK-008](./generated/cc-hk-008.md) | Script File Not Found | HIGH | Claude Hooks |
| [CC-HK-009](./generated/cc-hk-009.md) | Dangerous Command Pattern | HIGH | Claude Hooks |
| [CC-HK-010](./generated/cc-hk-010.md) | Timeout Policy | MEDIUM | Claude Hooks |
| [CC-HK-011](./generated/cc-hk-011.md) | Invalid Timeout Value | HIGH | Claude Hooks |
| [CC-HK-012](./generated/cc-hk-012.md) | Hooks Parse Error | HIGH | Claude Hooks |
| [CC-AG-001](./generated/cc-ag-001.md) | Missing Name Field | HIGH | Claude Agents |
| [CC-AG-002](./generated/cc-ag-002.md) | Missing Description Field | HIGH | Claude Agents |
| [CC-AG-003](./generated/cc-ag-003.md) | Invalid Model Value | HIGH | Claude Agents |
| [CC-AG-004](./generated/cc-ag-004.md) | Invalid Permission Mode | HIGH | Claude Agents |
| [CC-AG-005](./generated/cc-ag-005.md) | Referenced Skill Not Found | HIGH | Claude Agents |
| [CC-AG-006](./generated/cc-ag-006.md) | Tool/Disallowed Conflict | HIGH | Claude Agents |
| [CC-AG-007](./generated/cc-ag-007.md) | Agent Parse Error | HIGH | Claude Agents |
| [CC-MEM-001](./generated/cc-mem-001.md) | Invalid Import Path | HIGH | Claude Memory |
| [CC-MEM-002](./generated/cc-mem-002.md) | Circular Import | HIGH | Claude Memory |
| [CC-MEM-003](./generated/cc-mem-003.md) | Import Depth Exceeds 5 | HIGH | Claude Memory |
| [CC-MEM-004](./generated/cc-mem-004.md) | Invalid Command Reference | MEDIUM | Claude Memory |
| [CC-MEM-005](./generated/cc-mem-005.md) | Generic Instruction | HIGH | Claude Memory |
| [CC-MEM-006](./generated/cc-mem-006.md) | Negative Without Positive | HIGH | Claude Memory |
| [CC-MEM-007](./generated/cc-mem-007.md) | Weak Constraint Language | HIGH | Claude Memory |
| [CC-MEM-008](./generated/cc-mem-008.md) | Critical Content in Middle | HIGH | Claude Memory |
| [CC-MEM-009](./generated/cc-mem-009.md) | Token Count Exceeded | MEDIUM | Claude Memory |
| [CC-MEM-010](./generated/cc-mem-010.md) | README Duplication | MEDIUM | Claude Memory |
| [AGM-001](./generated/agm-001.md) | Valid Markdown Structure | HIGH | AGENTS.md |
| [AGM-002](./generated/agm-002.md) | Missing Section Headers | MEDIUM | AGENTS.md |
| [AGM-003](./generated/agm-003.md) | Character Limit (Windsurf) | MEDIUM | AGENTS.md |
| [AGM-004](./generated/agm-004.md) | Missing Project Context | MEDIUM | AGENTS.md |
| [AGM-005](./generated/agm-005.md) | Platform-Specific Features Without Guard | MEDIUM | AGENTS.md |
| [AGM-006](./generated/agm-006.md) | Nested AGENTS.md Hierarchy | MEDIUM | AGENTS.md |
| [CC-PL-001](./generated/cc-pl-001.md) | Plugin Manifest Not in .claude-plugin/ | HIGH | Claude Plugins |
| [CC-PL-002](./generated/cc-pl-002.md) | Components in .claude-plugin/ | HIGH | Claude Plugins |
| [CC-PL-003](./generated/cc-pl-003.md) | Invalid Semver | HIGH | Claude Plugins |
| [CC-PL-004](./generated/cc-pl-004.md) | Missing Required Plugin Field | HIGH | Claude Plugins |
| [CC-PL-005](./generated/cc-pl-005.md) | Empty Plugin Name | HIGH | Claude Plugins |
| [CC-PL-006](./generated/cc-pl-006.md) | Plugin Parse Error | HIGH | Claude Plugins |
| [MCP-001](./generated/mcp-001.md) | Invalid JSON-RPC Version | HIGH | MCP |
| [MCP-002](./generated/mcp-002.md) | Missing Required Tool Field | HIGH | MCP |
| [MCP-003](./generated/mcp-003.md) | Invalid JSON Schema | HIGH | MCP |
| [MCP-004](./generated/mcp-004.md) | Missing Tool Description | HIGH | MCP |
| [MCP-005](./generated/mcp-005.md) | Tool Without User Consent | HIGH | MCP |
| [MCP-006](./generated/mcp-006.md) | Untrusted Annotations | HIGH | MCP |
| [MCP-007](./generated/mcp-007.md) | MCP Parse Error | HIGH | MCP |
| [MCP-008](./generated/mcp-008.md) | Protocol Version Mismatch | MEDIUM | MCP |
| [COP-001](./generated/cop-001.md) | Empty Copilot Instruction File | HIGH | GitHub Copilot |
| [COP-002](./generated/cop-002.md) | Invalid Frontmatter in Scoped Instructions | HIGH | GitHub Copilot |
| [COP-003](./generated/cop-003.md) | Invalid Glob Pattern in applyTo | HIGH | GitHub Copilot |
| [COP-004](./generated/cop-004.md) | Unknown Frontmatter Keys | MEDIUM | GitHub Copilot |
| [CUR-001](./generated/cur-001.md) | Empty Cursor Rule File | HIGH | Cursor |
| [CUR-002](./generated/cur-002.md) | Missing Frontmatter in .mdc File | MEDIUM | Cursor |
| [CUR-003](./generated/cur-003.md) | Invalid YAML Frontmatter | HIGH | Cursor |
| [CUR-004](./generated/cur-004.md) | Invalid Glob Pattern in globs Field | HIGH | Cursor |
| [CUR-005](./generated/cur-005.md) | Unknown Frontmatter Keys | MEDIUM | Cursor |
| [CUR-006](./generated/cur-006.md) | Legacy .cursorrules File Detected | MEDIUM | Cursor |
| [XML-001](./generated/xml-001.md) | Unclosed XML Tag | HIGH | XML |
| [XML-002](./generated/xml-002.md) | Mismatched Closing Tag | HIGH | XML |
| [XML-003](./generated/xml-003.md) | Unmatched Closing Tag | HIGH | XML |
| [REF-001](./generated/ref-001.md) | Import File Not Found | HIGH | References |
| [REF-002](./generated/ref-002.md) | Broken Markdown Link | HIGH | References |
| [PE-001](./generated/pe-001.md) | Lost in the Middle | MEDIUM | Prompt Engineering |
| [PE-002](./generated/pe-002.md) | Chain-of-Thought on Simple Task | MEDIUM | Prompt Engineering |
| [PE-003](./generated/pe-003.md) | Weak Imperative Language | MEDIUM | Prompt Engineering |
| [PE-004](./generated/pe-004.md) | Ambiguous Instructions | MEDIUM | Prompt Engineering |
| [XP-001](./generated/xp-001.md) | Platform-Specific Feature in Generic Config | HIGH | Cross-Platform |
| [XP-002](./generated/xp-002.md) | AGENTS.md Platform Compatibility | HIGH | Cross-Platform |
| [XP-003](./generated/xp-003.md) | Hard-Coded Platform Paths | HIGH | Cross-Platform |
| [XP-004](./generated/xp-004.md) | Conflicting Build/Test Commands | MEDIUM | Cross-Platform |
| [XP-005](./generated/xp-005.md) | Conflicting Tool Constraints | HIGH | Cross-Platform |
| [XP-006](./generated/xp-006.md) | Multiple Layers Without Documented Precedence | MEDIUM | Cross-Platform |
| [VER-001](./generated/ver-001.md) | No Tool/Spec Versions Pinned | LOW | Version Awareness |
