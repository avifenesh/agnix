# Rules Reference

This section contains all `100` validation rules generated from `knowledge-base/rules.json`.

| Rule | Name | Severity | Category |
|------|------|----------|----------|
| [AS-001](./generated/as-001) | Missing Frontmatter | HIGH | Agent Skills |
| [AS-002](./generated/as-002) | Missing Required Field: name | HIGH | Agent Skills |
| [AS-003](./generated/as-003) | Missing Required Field: description | HIGH | Agent Skills |
| [AS-004](./generated/as-004) | Invalid Name Format | HIGH | Agent Skills |
| [AS-005](./generated/as-005) | Name Starts/Ends with Hyphen | HIGH | Agent Skills |
| [AS-006](./generated/as-006) | Consecutive Hyphens in Name | HIGH | Agent Skills |
| [AS-007](./generated/as-007) | Reserved Name | HIGH | Agent Skills |
| [AS-008](./generated/as-008) | Description Too Short | HIGH | Agent Skills |
| [AS-009](./generated/as-009) | Description Contains XML | HIGH | Agent Skills |
| [AS-010](./generated/as-010) | Missing Trigger Phrase | MEDIUM | Agent Skills |
| [AS-011](./generated/as-011) | Compatibility Too Long | HIGH | Agent Skills |
| [AS-012](./generated/as-012) | Content Exceeds 500 Lines | MEDIUM | Agent Skills |
| [AS-013](./generated/as-013) | File Reference Too Deep | HIGH | Agent Skills |
| [AS-014](./generated/as-014) | Windows Path Separator | HIGH | Agent Skills |
| [AS-015](./generated/as-015) | Upload Size Exceeds 8MB | HIGH | Agent Skills |
| [AS-016](./generated/as-016) | Skill Parse Error | HIGH | Agent Skills |
| [CC-SK-001](./generated/cc-sk-001) | Invalid Model Value | HIGH | Claude Skills |
| [CC-SK-002](./generated/cc-sk-002) | Invalid Context Value | HIGH | Claude Skills |
| [CC-SK-003](./generated/cc-sk-003) | Context Without Agent | HIGH | Claude Skills |
| [CC-SK-004](./generated/cc-sk-004) | Agent Without Context | HIGH | Claude Skills |
| [CC-SK-005](./generated/cc-sk-005) | Invalid Agent Type | HIGH | Claude Skills |
| [CC-SK-006](./generated/cc-sk-006) | Dangerous Auto-Invocation | HIGH | Claude Skills |
| [CC-SK-007](./generated/cc-sk-007) | Unrestricted Bash | HIGH | Claude Skills |
| [CC-SK-008](./generated/cc-sk-008) | Unknown Tool Name | HIGH | Claude Skills |
| [CC-SK-009](./generated/cc-sk-009) | Too Many Injections | MEDIUM | Claude Skills |
| [CC-HK-001](./generated/cc-hk-001) | Invalid Hook Event | HIGH | Claude Hooks |
| [CC-HK-002](./generated/cc-hk-002) | Prompt Hook on Wrong Event | HIGH | Claude Hooks |
| [CC-HK-003](./generated/cc-hk-003) | Missing Matcher for Tool Events | HIGH | Claude Hooks |
| [CC-HK-004](./generated/cc-hk-004) | Matcher on Non-Tool Event | HIGH | Claude Hooks |
| [CC-HK-005](./generated/cc-hk-005) | Missing Type Field | HIGH | Claude Hooks |
| [CC-HK-006](./generated/cc-hk-006) | Missing Command Field | HIGH | Claude Hooks |
| [CC-HK-007](./generated/cc-hk-007) | Missing Prompt Field | HIGH | Claude Hooks |
| [CC-HK-008](./generated/cc-hk-008) | Script File Not Found | HIGH | Claude Hooks |
| [CC-HK-009](./generated/cc-hk-009) | Dangerous Command Pattern | HIGH | Claude Hooks |
| [CC-HK-010](./generated/cc-hk-010) | Timeout Policy | MEDIUM | Claude Hooks |
| [CC-HK-011](./generated/cc-hk-011) | Invalid Timeout Value | HIGH | Claude Hooks |
| [CC-HK-012](./generated/cc-hk-012) | Hooks Parse Error | HIGH | Claude Hooks |
| [CC-AG-001](./generated/cc-ag-001) | Missing Name Field | HIGH | Claude Agents |
| [CC-AG-002](./generated/cc-ag-002) | Missing Description Field | HIGH | Claude Agents |
| [CC-AG-003](./generated/cc-ag-003) | Invalid Model Value | HIGH | Claude Agents |
| [CC-AG-004](./generated/cc-ag-004) | Invalid Permission Mode | HIGH | Claude Agents |
| [CC-AG-005](./generated/cc-ag-005) | Referenced Skill Not Found | HIGH | Claude Agents |
| [CC-AG-006](./generated/cc-ag-006) | Tool/Disallowed Conflict | HIGH | Claude Agents |
| [CC-AG-007](./generated/cc-ag-007) | Agent Parse Error | HIGH | Claude Agents |
| [CC-MEM-001](./generated/cc-mem-001) | Invalid Import Path | HIGH | Claude Memory |
| [CC-MEM-002](./generated/cc-mem-002) | Circular Import | HIGH | Claude Memory |
| [CC-MEM-003](./generated/cc-mem-003) | Import Depth Exceeds 5 | HIGH | Claude Memory |
| [CC-MEM-004](./generated/cc-mem-004) | Invalid Command Reference | MEDIUM | Claude Memory |
| [CC-MEM-005](./generated/cc-mem-005) | Generic Instruction | HIGH | Claude Memory |
| [CC-MEM-006](./generated/cc-mem-006) | Negative Without Positive | HIGH | Claude Memory |
| [CC-MEM-007](./generated/cc-mem-007) | Weak Constraint Language | HIGH | Claude Memory |
| [CC-MEM-008](./generated/cc-mem-008) | Critical Content in Middle | HIGH | Claude Memory |
| [CC-MEM-009](./generated/cc-mem-009) | Token Count Exceeded | MEDIUM | Claude Memory |
| [CC-MEM-010](./generated/cc-mem-010) | README Duplication | MEDIUM | Claude Memory |
| [AGM-001](./generated/agm-001) | Valid Markdown Structure | HIGH | AGENTS.md |
| [AGM-002](./generated/agm-002) | Missing Section Headers | MEDIUM | AGENTS.md |
| [AGM-003](./generated/agm-003) | Character Limit (Windsurf) | MEDIUM | AGENTS.md |
| [AGM-004](./generated/agm-004) | Missing Project Context | MEDIUM | AGENTS.md |
| [AGM-005](./generated/agm-005) | Platform-Specific Features Without Guard | MEDIUM | AGENTS.md |
| [AGM-006](./generated/agm-006) | Nested AGENTS.md Hierarchy | MEDIUM | AGENTS.md |
| [CC-PL-001](./generated/cc-pl-001) | Plugin Manifest Not in .claude-plugin/ | HIGH | Claude Plugins |
| [CC-PL-002](./generated/cc-pl-002) | Components in .claude-plugin/ | HIGH | Claude Plugins |
| [CC-PL-003](./generated/cc-pl-003) | Invalid Semver | HIGH | Claude Plugins |
| [CC-PL-004](./generated/cc-pl-004) | Missing Required Plugin Field | HIGH | Claude Plugins |
| [CC-PL-005](./generated/cc-pl-005) | Empty Plugin Name | HIGH | Claude Plugins |
| [CC-PL-006](./generated/cc-pl-006) | Plugin Parse Error | HIGH | Claude Plugins |
| [MCP-001](./generated/mcp-001) | Invalid JSON-RPC Version | HIGH | MCP |
| [MCP-002](./generated/mcp-002) | Missing Required Tool Field | HIGH | MCP |
| [MCP-003](./generated/mcp-003) | Invalid JSON Schema | HIGH | MCP |
| [MCP-004](./generated/mcp-004) | Missing Tool Description | HIGH | MCP |
| [MCP-005](./generated/mcp-005) | Tool Without User Consent | HIGH | MCP |
| [MCP-006](./generated/mcp-006) | Untrusted Annotations | HIGH | MCP |
| [MCP-007](./generated/mcp-007) | MCP Parse Error | HIGH | MCP |
| [MCP-008](./generated/mcp-008) | Protocol Version Mismatch | MEDIUM | MCP |
| [COP-001](./generated/cop-001) | Empty Copilot Instruction File | HIGH | GitHub Copilot |
| [COP-002](./generated/cop-002) | Invalid Frontmatter in Scoped Instructions | HIGH | GitHub Copilot |
| [COP-003](./generated/cop-003) | Invalid Glob Pattern in applyTo | HIGH | GitHub Copilot |
| [COP-004](./generated/cop-004) | Unknown Frontmatter Keys | MEDIUM | GitHub Copilot |
| [CUR-001](./generated/cur-001) | Empty Cursor Rule File | HIGH | Cursor |
| [CUR-002](./generated/cur-002) | Missing Frontmatter in .mdc File | MEDIUM | Cursor |
| [CUR-003](./generated/cur-003) | Invalid YAML Frontmatter | HIGH | Cursor |
| [CUR-004](./generated/cur-004) | Invalid Glob Pattern in globs Field | HIGH | Cursor |
| [CUR-005](./generated/cur-005) | Unknown Frontmatter Keys | MEDIUM | Cursor |
| [CUR-006](./generated/cur-006) | Legacy .cursorrules File Detected | MEDIUM | Cursor |
| [XML-001](./generated/xml-001) | Unclosed XML Tag | HIGH | XML |
| [XML-002](./generated/xml-002) | Mismatched Closing Tag | HIGH | XML |
| [XML-003](./generated/xml-003) | Unmatched Closing Tag | HIGH | XML |
| [REF-001](./generated/ref-001) | Import File Not Found | HIGH | References |
| [REF-002](./generated/ref-002) | Broken Markdown Link | HIGH | References |
| [PE-001](./generated/pe-001) | Lost in the Middle | MEDIUM | Prompt Engineering |
| [PE-002](./generated/pe-002) | Chain-of-Thought on Simple Task | MEDIUM | Prompt Engineering |
| [PE-003](./generated/pe-003) | Weak Imperative Language | MEDIUM | Prompt Engineering |
| [PE-004](./generated/pe-004) | Ambiguous Instructions | MEDIUM | Prompt Engineering |
| [XP-001](./generated/xp-001) | Platform-Specific Feature in Generic Config | HIGH | Cross-Platform |
| [XP-002](./generated/xp-002) | AGENTS.md Platform Compatibility | HIGH | Cross-Platform |
| [XP-003](./generated/xp-003) | Hard-Coded Platform Paths | HIGH | Cross-Platform |
| [XP-004](./generated/xp-004) | Conflicting Build/Test Commands | MEDIUM | Cross-Platform |
| [XP-005](./generated/xp-005) | Conflicting Tool Constraints | HIGH | Cross-Platform |
| [XP-006](./generated/xp-006) | Multiple Layers Without Documented Precedence | MEDIUM | Cross-Platform |
| [VER-001](./generated/ver-001) | No Tool/Spec Versions Pinned | LOW | Version Awareness |
