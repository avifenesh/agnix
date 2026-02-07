# Rules Reference

This section contains all `139` validation rules generated from `knowledge-base/rules.json`.
`32` rules have automatic fixes.


| Rule | Name | Severity | Category | Auto-Fix |
|------|------|----------|----------|----------|
| [AS-001](./generated/as-001.md) | Missing Frontmatter | HIGH | Agent Skills | No |
| [AS-002](./generated/as-002.md) | Missing Required Field: name | HIGH | Agent Skills | No |
| [AS-003](./generated/as-003.md) | Missing Required Field: description | HIGH | Agent Skills | No |
| [AS-004](./generated/as-004.md) | Invalid Name Format | HIGH | Agent Skills | Yes (safe/unsafe) |
| [AS-005](./generated/as-005.md) | Name Starts/Ends with Hyphen | HIGH | Agent Skills | Yes (safe) |
| [AS-006](./generated/as-006.md) | Consecutive Hyphens in Name | HIGH | Agent Skills | Yes (safe) |
| [AS-007](./generated/as-007.md) | Reserved Name | HIGH | Agent Skills | No |
| [AS-008](./generated/as-008.md) | Description Too Short | HIGH | Agent Skills | No |
| [AS-009](./generated/as-009.md) | Description Contains XML | HIGH | Agent Skills | No |
| [AS-010](./generated/as-010.md) | Missing Trigger Phrase | MEDIUM | Agent Skills | Yes (unsafe) |
| [AS-011](./generated/as-011.md) | Compatibility Too Long | HIGH | Agent Skills | No |
| [AS-012](./generated/as-012.md) | Content Exceeds 500 Lines | MEDIUM | Agent Skills | No |
| [AS-013](./generated/as-013.md) | File Reference Too Deep | HIGH | Agent Skills | No |
| [AS-014](./generated/as-014.md) | Windows Path Separator | HIGH | Agent Skills | Yes (safe) |
| [AS-015](./generated/as-015.md) | Upload Size Exceeds 8MB | HIGH | Agent Skills | No |
| [AS-016](./generated/as-016.md) | Skill Parse Error | HIGH | Agent Skills | No |
| [CC-SK-001](./generated/cc-sk-001.md) | Invalid Model Value | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-002](./generated/cc-sk-002.md) | Invalid Context Value | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-003](./generated/cc-sk-003.md) | Context Without Agent | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-004](./generated/cc-sk-004.md) | Agent Without Context | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-005](./generated/cc-sk-005.md) | Invalid Agent Type | HIGH | Claude Skills | No |
| [CC-SK-006](./generated/cc-sk-006.md) | Dangerous Auto-Invocation | HIGH | Claude Skills | No |
| [CC-SK-007](./generated/cc-sk-007.md) | Unrestricted Bash | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-008](./generated/cc-sk-008.md) | Unknown Tool Name | HIGH | Claude Skills | No |
| [CC-SK-009](./generated/cc-sk-009.md) | Too Many Injections | MEDIUM | Claude Skills | No |
| [CC-SK-010](./generated/cc-sk-010.md) | Invalid Hooks in Skill Frontmatter | HIGH | Claude Skills | No |
| [CC-SK-011](./generated/cc-sk-011.md) | Unreachable Skill | HIGH | Claude Skills | No |
| [CC-SK-012](./generated/cc-sk-012.md) | Argument Hint Without $ARGUMENTS | MEDIUM | Claude Skills | No |
| [CC-SK-013](./generated/cc-sk-013.md) | Fork Context Without Actionable Instructions | MEDIUM | Claude Skills | No |
| [CC-SK-014](./generated/cc-sk-014.md) | Invalid disable-model-invocation Type | HIGH | Claude Skills | Yes (safe) |
| [CC-SK-015](./generated/cc-sk-015.md) | Invalid user-invocable Type | HIGH | Claude Skills | Yes (safe) |
| [CC-HK-001](./generated/cc-hk-001.md) | Invalid Hook Event | HIGH | Claude Hooks | Yes (safe/unsafe) |
| [CC-HK-002](./generated/cc-hk-002.md) | Prompt Hook on Wrong Event | HIGH | Claude Hooks | No |
| [CC-HK-003](./generated/cc-hk-003.md) | Missing Matcher for Tool Events | HIGH | Claude Hooks | No |
| [CC-HK-004](./generated/cc-hk-004.md) | Matcher on Non-Tool Event | HIGH | Claude Hooks | Yes (safe) |
| [CC-HK-005](./generated/cc-hk-005.md) | Missing Type Field | HIGH | Claude Hooks | No |
| [CC-HK-006](./generated/cc-hk-006.md) | Missing Command Field | HIGH | Claude Hooks | No |
| [CC-HK-007](./generated/cc-hk-007.md) | Missing Prompt Field | HIGH | Claude Hooks | No |
| [CC-HK-008](./generated/cc-hk-008.md) | Script File Not Found | HIGH | Claude Hooks | No |
| [CC-HK-009](./generated/cc-hk-009.md) | Dangerous Command Pattern | HIGH | Claude Hooks | No |
| [CC-HK-010](./generated/cc-hk-010.md) | Timeout Policy | MEDIUM | Claude Hooks | No |
| [CC-HK-011](./generated/cc-hk-011.md) | Invalid Timeout Value | HIGH | Claude Hooks | Yes (unsafe) |
| [CC-HK-012](./generated/cc-hk-012.md) | Hooks Parse Error | HIGH | Claude Hooks | No |
| [CC-HK-013](./generated/cc-hk-013.md) | Async on Non-Command Hook | HIGH | Claude Hooks | No |
| [CC-HK-014](./generated/cc-hk-014.md) | Once Outside Skill/Agent Frontmatter | MEDIUM | Claude Hooks | No |
| [CC-HK-015](./generated/cc-hk-015.md) | Model on Command Hook | MEDIUM | Claude Hooks | No |
| [CC-HK-016](./generated/cc-hk-016.md) | Validate Hook Type Agent | HIGH | Claude Hooks | No |
| [CC-HK-017](./generated/cc-hk-017.md) | Prompt/Agent Hook Missing $ARGUMENTS | MEDIUM | Claude Hooks | No |
| [CC-HK-018](./generated/cc-hk-018.md) | Matcher on UserPromptSubmit/Stop | LOW | Claude Hooks | No |
| [CC-AG-001](./generated/cc-ag-001.md) | Missing Name Field | HIGH | Claude Agents | No |
| [CC-AG-002](./generated/cc-ag-002.md) | Missing Description Field | HIGH | Claude Agents | No |
| [CC-AG-003](./generated/cc-ag-003.md) | Invalid Model Value | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-004](./generated/cc-ag-004.md) | Invalid Permission Mode | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-005](./generated/cc-ag-005.md) | Referenced Skill Not Found | HIGH | Claude Agents | No |
| [CC-AG-006](./generated/cc-ag-006.md) | Tool/Disallowed Conflict | HIGH | Claude Agents | No |
| [CC-AG-007](./generated/cc-ag-007.md) | Agent Parse Error | HIGH | Claude Agents | No |
| [CC-AG-008](./generated/cc-ag-008.md) | Invalid Memory Scope | HIGH | Claude Agents | No |
| [CC-AG-009](./generated/cc-ag-009.md) | Invalid Tool Name in Tools List | HIGH | Claude Agents | No |
| [CC-AG-010](./generated/cc-ag-010.md) | Invalid Tool Name in DisallowedTools | HIGH | Claude Agents | No |
| [CC-AG-011](./generated/cc-ag-011.md) | Invalid Hooks in Agent Frontmatter | HIGH | Claude Agents | No |
| [CC-AG-012](./generated/cc-ag-012.md) | Bypass Permissions Warning | HIGH | Claude Agents | No |
| [CC-AG-013](./generated/cc-ag-013.md) | Invalid Skill Name Format | MEDIUM | Claude Agents | No |
| [CC-MEM-001](./generated/cc-mem-001.md) | Invalid Import Path | HIGH | Claude Memory | No |
| [CC-MEM-002](./generated/cc-mem-002.md) | Circular Import | HIGH | Claude Memory | No |
| [CC-MEM-003](./generated/cc-mem-003.md) | Import Depth Exceeds 5 | HIGH | Claude Memory | No |
| [CC-MEM-004](./generated/cc-mem-004.md) | Invalid Command Reference | MEDIUM | Claude Memory | No |
| [CC-MEM-005](./generated/cc-mem-005.md) | Generic Instruction | HIGH | Claude Memory | Yes (safe) |
| [CC-MEM-006](./generated/cc-mem-006.md) | Negative Without Positive | HIGH | Claude Memory | No |
| [CC-MEM-007](./generated/cc-mem-007.md) | Weak Constraint Language | HIGH | Claude Memory | Yes (safe/unsafe) |
| [CC-MEM-008](./generated/cc-mem-008.md) | Critical Content in Middle | HIGH | Claude Memory | No |
| [CC-MEM-009](./generated/cc-mem-009.md) | Token Count Exceeded | MEDIUM | Claude Memory | No |
| [CC-MEM-010](./generated/cc-mem-010.md) | README Duplication | MEDIUM | Claude Memory | No |
| [CC-MEM-011](./generated/cc-mem-011.md) | Invalid Paths Glob in Rules | HIGH | Claude Memory | No |
| [CC-MEM-012](./generated/cc-mem-012.md) | Rules File Unknown Frontmatter Key | MEDIUM | Claude Memory | Yes (unsafe) |
| [AGM-001](./generated/agm-001.md) | Valid Markdown Structure | HIGH | AGENTS.md | No |
| [AGM-002](./generated/agm-002.md) | Missing Section Headers | MEDIUM | AGENTS.md | No |
| [AGM-003](./generated/agm-003.md) | Character Limit (Windsurf) | MEDIUM | AGENTS.md | No |
| [AGM-004](./generated/agm-004.md) | Missing Project Context | MEDIUM | AGENTS.md | No |
| [AGM-005](./generated/agm-005.md) | Platform-Specific Features Without Guard | MEDIUM | AGENTS.md | No |
| [AGM-006](./generated/agm-006.md) | Nested AGENTS.md Hierarchy | MEDIUM | AGENTS.md | No |
| [CC-PL-001](./generated/cc-pl-001.md) | Plugin Manifest Not in .claude-plugin/ | HIGH | Claude Plugins | No |
| [CC-PL-002](./generated/cc-pl-002.md) | Components in .claude-plugin/ | HIGH | Claude Plugins | No |
| [CC-PL-003](./generated/cc-pl-003.md) | Invalid Semver | HIGH | Claude Plugins | No |
| [CC-PL-004](./generated/cc-pl-004.md) | Missing Required Plugin Field | HIGH | Claude Plugins | No |
| [CC-PL-005](./generated/cc-pl-005.md) | Empty Plugin Name | HIGH | Claude Plugins | Yes (unsafe) |
| [CC-PL-006](./generated/cc-pl-006.md) | Plugin Parse Error | HIGH | Claude Plugins | No |
| [CC-PL-007](./generated/cc-pl-007.md) | Invalid Component Path | HIGH | Claude Plugins | Yes (safe) |
| [CC-PL-008](./generated/cc-pl-008.md) | Component Inside .claude-plugin | HIGH | Claude Plugins | No |
| [CC-PL-009](./generated/cc-pl-009.md) | Invalid Author Object | MEDIUM | Claude Plugins | No |
| [CC-PL-010](./generated/cc-pl-010.md) | Invalid Homepage URL | MEDIUM | Claude Plugins | No |
| [MCP-001](./generated/mcp-001.md) | Invalid JSON-RPC Version | HIGH | MCP | Yes (safe) |
| [MCP-002](./generated/mcp-002.md) | Missing Required Tool Field | HIGH | MCP | No |
| [MCP-003](./generated/mcp-003.md) | Invalid JSON Schema | HIGH | MCP | No |
| [MCP-004](./generated/mcp-004.md) | Missing Tool Description | HIGH | MCP | No |
| [MCP-005](./generated/mcp-005.md) | Tool Without User Consent | HIGH | MCP | No |
| [MCP-006](./generated/mcp-006.md) | Untrusted Annotations | HIGH | MCP | No |
| [MCP-007](./generated/mcp-007.md) | MCP Parse Error | HIGH | MCP | No |
| [MCP-008](./generated/mcp-008.md) | Protocol Version Mismatch | MEDIUM | MCP | Yes (unsafe) |
| [MCP-009](./generated/mcp-009.md) | Missing command for stdio server | HIGH | MCP | No |
| [MCP-010](./generated/mcp-010.md) | Missing url for http/sse server | HIGH | MCP | No |
| [MCP-011](./generated/mcp-011.md) | Invalid MCP server type | HIGH | MCP | No |
| [MCP-012](./generated/mcp-012.md) | Deprecated SSE transport | MEDIUM | MCP | Yes (unsafe) |
| [COP-001](./generated/cop-001.md) | Empty Copilot Instruction File | HIGH | GitHub Copilot | No |
| [COP-002](./generated/cop-002.md) | Invalid Frontmatter in Scoped Instructions | HIGH | GitHub Copilot | No |
| [COP-003](./generated/cop-003.md) | Invalid Glob Pattern in applyTo | HIGH | GitHub Copilot | No |
| [COP-004](./generated/cop-004.md) | Unknown Frontmatter Keys | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-005](./generated/cop-005.md) | Invalid excludeAgent Value | HIGH | GitHub Copilot | No |
| [COP-006](./generated/cop-006.md) | File Length Limit | MEDIUM | GitHub Copilot | No |
| [CUR-001](./generated/cur-001.md) | Empty Cursor Rule File | HIGH | Cursor | No |
| [CUR-002](./generated/cur-002.md) | Missing Frontmatter in .mdc File | MEDIUM | Cursor | No |
| [CUR-003](./generated/cur-003.md) | Invalid YAML Frontmatter | HIGH | Cursor | No |
| [CUR-004](./generated/cur-004.md) | Invalid Glob Pattern in globs Field | HIGH | Cursor | No |
| [CUR-005](./generated/cur-005.md) | Unknown Frontmatter Keys | MEDIUM | Cursor | Yes (safe) |
| [CUR-006](./generated/cur-006.md) | Legacy .cursorrules File Detected | MEDIUM | Cursor | No |
| [CUR-007](./generated/cur-007.md) | alwaysApply with Redundant globs | MEDIUM | Cursor | Yes (safe) |
| [CUR-008](./generated/cur-008.md) | Invalid alwaysApply Type | HIGH | Cursor | No |
| [CUR-009](./generated/cur-009.md) | Missing Description for Agent-Requested Rule | MEDIUM | Cursor | No |
| [CLN-001](./generated/cln-001.md) | Empty Cline Rules File | HIGH | Cline | No |
| [CLN-002](./generated/cln-002.md) | Invalid Paths Glob in Cline Rules | HIGH | Cline | No |
| [CLN-003](./generated/cln-003.md) | Unknown Frontmatter Key in Cline Rules | MEDIUM | Cline | Yes (unsafe) |
| [XML-001](./generated/xml-001.md) | Unclosed XML Tag | HIGH | XML | Yes (unsafe) |
| [XML-002](./generated/xml-002.md) | Mismatched Closing Tag | HIGH | XML | Yes (unsafe) |
| [XML-003](./generated/xml-003.md) | Unmatched Closing Tag | HIGH | XML | Yes (unsafe) |
| [REF-001](./generated/ref-001.md) | Import File Not Found | HIGH | References | No |
| [REF-002](./generated/ref-002.md) | Broken Markdown Link | HIGH | References | No |
| [PE-001](./generated/pe-001.md) | Lost in the Middle | MEDIUM | Prompt Engineering | No |
| [PE-002](./generated/pe-002.md) | Chain-of-Thought on Simple Task | MEDIUM | Prompt Engineering | No |
| [PE-003](./generated/pe-003.md) | Weak Imperative Language | MEDIUM | Prompt Engineering | No |
| [PE-004](./generated/pe-004.md) | Ambiguous Instructions | MEDIUM | Prompt Engineering | No |
| [XP-001](./generated/xp-001.md) | Platform-Specific Feature in Generic Config | HIGH | Cross-Platform | No |
| [XP-002](./generated/xp-002.md) | AGENTS.md Platform Compatibility | HIGH | Cross-Platform | No |
| [XP-003](./generated/xp-003.md) | Hard-Coded Platform Paths | HIGH | Cross-Platform | No |
| [XP-004](./generated/xp-004.md) | Conflicting Build/Test Commands | MEDIUM | Cross-Platform | No |
| [XP-005](./generated/xp-005.md) | Conflicting Tool Constraints | HIGH | Cross-Platform | No |
| [XP-006](./generated/xp-006.md) | Multiple Layers Without Documented Precedence | MEDIUM | Cross-Platform | No |
| [OC-001](./generated/oc-001.md) | Invalid Share Mode | HIGH | OpenCode | No |
| [OC-002](./generated/oc-002.md) | Invalid Instruction Path | HIGH | OpenCode | No |
| [OC-003](./generated/oc-003.md) | opencode.json Parse Error | HIGH | OpenCode | No |
| [VER-001](./generated/ver-001.md) | No Tool/Spec Versions Pinned | LOW | Version Awareness | No |
