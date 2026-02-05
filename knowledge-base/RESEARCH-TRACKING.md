# Research Tracking

> Master document for tracking AI tool ecosystem changes, research updates, and community feedback.

**Last Updated**: 2026-02-05
**Review Cadence**: Monthly (1st week of each month)
**Related**: [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) | [VALIDATION-RULES.md](./VALIDATION-RULES.md) | [INDEX.md](./INDEX.md)

---

## Tool Inventory

Tools are organized by support tier (see [../CONTRIBUTING.md#tool-tier-system](../CONTRIBUTING.md#tool-tier-system) for definitions). Each entry tracks the config format, documentation source, monitoring approach, review frequency, and rule coverage.

### S Tier (test always)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Claude Code | `CLAUDE.md`, `.claude/settings.json` | https://code.claude.com/docs/en | Automated (spec-drift.yml) | Weekly | 2026-02-05 | CC-SK, CC-HK, CC-MEM, CC-AG, CC-PL |
| Codex CLI | `AGENTS.md`, `codex.toml` | https://developers.openai.com/codex/ | Automated (spec-drift.yml) | Weekly | 2026-02-05 | AGM, XP |
| OpenCode | `AGENTS.md`, `.opencode/config.json` | https://opencode.ai/docs/ | Automated (spec-drift.yml) | Weekly | 2026-02-05 | AGM, XP |

### A Tier (test on major changes)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| GitHub Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | https://docs.github.com/en/copilot/customizing-copilot | Automated (spec-drift.yml) | Monthly | 2026-02-05 | COP |
| Cline | `.clinerules`, `.cline/rules/*.md` | https://docs.cline.bot/features/cline-rules/overview | Automated (spec-drift.yml) | Monthly | 2026-02-05 | -- |
| Cursor | `.cursor/rules/*.mdc`, `.cursorrules` | https://cursor.com/docs/context/rules | Automated (spec-drift.yml) | Monthly | 2026-02-05 | CUR |

### B Tier (test on significant changes if time permits)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Roo Code | `.roo/rules/*.md` | https://github.com/RooVetGit/Roo-Code | Manual | Quarterly | 2026-02-05 | -- |
| Kiro CLI | `kiro.md` | https://kiro.dev/ | Manual | Quarterly | 2026-02-05 | -- |
| amp | `.amp/rules.md` | https://amp.dev/ | Manual | Quarterly | 2026-02-05 | -- |
| pi | `.pi/config.json` | TBD | Manual | Quarterly | 2026-02-05 | -- |

### C Tier (community reports fixes only)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| gemini cli | `GEMINI.md` | https://github.com/google-gemini/gemini-cli | Manual | As reported | 2026-02-05 | -- |
| continue | `.continue/config.json` | https://docs.continue.dev/ | Manual | As reported | 2026-02-05 | -- |
| Antigravity | `.antigravity/config.yml` | TBD | Manual | As reported | 2026-02-05 | -- |

### D Tier (no support, nice to have)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Tabnine | `.tabnine.json` | https://docs.tabnine.com/ | None | Ad hoc | -- | -- |
| Codeium | `.codeium/config.json` | https://codeium.com/docs/ | None | Ad hoc | -- | -- |
| Amazon Q | `.aws/q/config.json` | https://docs.aws.amazon.com/q/ | None | Ad hoc | -- | -- |
| Windsurf | `.windsurfrules` | https://windsurf.com/docs/ | None | Ad hoc | -- | -- |
| Aider | `.aider.conf.yml` | https://aider.chat/docs/ | None | Ad hoc | -- | -- |
| SourceGraph Cody | `.sourcegraph/config.json` | https://sourcegraph.com/docs/cody/ | None | Ad hoc | -- | -- |

### E Tier (no support, community only)

All other AI coding tools. Community contributions welcome via the Tool Support Request issue template.

---

## Documentation Sources

Authoritative sources monitored for changes that may affect validation rules.

### Specifications (Primary)

| Source | URL | Watch Method | Rules Affected |
|--------|-----|-------------|----------------|
| Agent Skills Spec | https://agentskills.io/specification | spec-drift.yml (weekly) | AS-001 through AS-016 |
| MCP Spec | https://modelcontextprotocol.io/specification/2025-11-25 | spec-drift.yml (weekly) | MCP-001 through MCP-008 |
| MCP GitHub Repo | https://github.com/modelcontextprotocol/specification | mcp-release-watch.yml | MCP-001 through MCP-008 |

### Vendor Documentation (Secondary)

| Source | URL | Watch Method | Rules Affected |
|--------|-----|-------------|----------------|
| Claude Code - Memory | https://code.claude.com/docs/en/memory | spec-drift.yml (weekly) | CC-MEM-001 through CC-MEM-010 |
| Claude Code - Hooks | https://code.claude.com/docs/en/hooks | spec-drift.yml (weekly) | CC-HK-001 through CC-HK-012 |
| Claude Code - Skills | https://code.claude.com/docs/en/skills | spec-drift.yml (weekly) | CC-SK-001 through CC-SK-009 |
| Claude Code - Plugins | https://code.claude.com/docs/en/plugins-reference | spec-drift.yml (weekly) | CC-PL-001 through CC-PL-006 |
| Claude Code - Sub-agents | https://code.claude.com/docs/en/sub-agents | spec-drift.yml (weekly) | CC-AG-001 through CC-AG-007 |
| Codex CLI - AGENTS.md | https://developers.openai.com/codex/guides/agents-md/ | spec-drift.yml (weekly) | AGM-001 through AGM-006, XP-001 through XP-006 |
| OpenCode - Rules | https://opencode.ai/docs/rules/ | spec-drift.yml (weekly) | XP-001 through XP-006 |
| Cursor - Rules | https://cursor.com/docs/context/rules | spec-drift.yml (monthly) | CUR-001 through CUR-006 |
| GitHub Copilot | https://docs.github.com/en/copilot/customizing-copilot | spec-drift.yml (monthly) | COP-001 through COP-004 |
| Cline - Rules | https://docs.cline.bot/features/cline-rules/overview | spec-drift.yml (monthly) | -- |

### Community Sources

| Source | URL | What to Watch |
|--------|-----|---------------|
| awesome-slash | https://github.com/anthropics/awesome-slash | Pattern updates, new enhance plugins |
| MCP Servers Registry | https://github.com/modelcontextprotocol/servers | New server patterns, security advisories |
| Stack Overflow AI Survey | https://survey.stackoverflow.co/2025/ai | Developer pain points, tool adoption trends |

---

## Academic Research

Research papers that inform validation rules, particularly prompt engineering and instruction-following rules.

| Paper | Authors | Year | Key Finding | Rules Informed |
|-------|---------|------|-------------|----------------|
| Lost in the Middle: How Language Models Use Long Contexts | Liu et al. | 2023 | Critical content in the middle of long contexts loses recall; position at start or end for best results | PE-001, CC-MEM-008 |
| Anthropic Long Context Research | Anthropic | 2023 | Single prompt change ("here is the most relevant sentence") improved retrieval accuracy from 27% to 98% | PE-001 |
| Positive Framing Studies | Multiple | 2023-2024 | "Do X" instructions outperform "Don't do Y" with measurable improvement in compliance rates | CC-MEM-006 |
| Constraint Strength Research | Instruction-following researchers | 2024 | MUST > imperatives > should > try to; weak language reduces compliance by significant margins | CC-MEM-007, PE-003 |
| Instruction-Following Reliability in LLMs | Multiple | 2024 | LLMs more reliably follow explicit, structured constraints than implicit or conversational ones | PE-003, PE-004 |
| Chain-of-Thought Prompting | Wei et al. | 2022 | CoT improves reasoning on complex tasks but adds overhead on simple tasks | PE-002 |

### Research Watch Areas

- Prompt injection defense mechanisms (currently unsolved, noted in MCP security)
- Multi-agent coordination patterns
- Config format convergence across tools
- Empirical studies on instruction-following in coding contexts

---

## Emerging Tools Watchlist

New developments that may require future rule additions or tool tier changes.

### Agent Protocol Standardization

- **Status**: Active development across multiple vendors
- **Watch**: Whether a universal agent config format emerges
- **Impact**: Could simplify cross-platform rules (XP-*) or require new universal rules
- **Sources**: Vendor announcements, community discussions

### New MCP Patterns

- **Status**: MCP ecosystem rapidly expanding
- **Watch**: New transport types, authentication patterns, tool annotation schemas
- **Impact**: May require updates to MCP-001 through MCP-008, new rules for auth/transport
- **Sources**: https://modelcontextprotocol.io, mcp-release-watch.yml workflow

### AGENTS.md Ecosystem

- **Status**: Adopted by Codex CLI, OpenCode; recognized by Claude Code
- **Watch**: Additional tools adopting AGENTS.md, format extensions
- **Impact**: AGM rules may need updates; XP rules may need to cover more tools
- **Sources**: https://developers.openai.com/codex/guides/agents-md/

### Sub-agent Patterns

- **Status**: Claude Code sub-agents gaining adoption
- **Watch**: How other tools implement sub-agent delegation
- **Impact**: CC-AG rules may need cross-platform equivalents
- **Sources**: https://code.claude.com/docs/en/sub-agents

---

## MCP Ecosystem Tracking

The Model Context Protocol ecosystem is tracked separately due to its rapid evolution and cross-tool impact.

### Current MCP Coverage

- **Protocol version monitored**: 2025-11-25
- **Rules**: MCP-001 through MCP-008
- **Automated monitoring**: mcp-release-watch.yml (GitHub releases), spec-drift.yml (spec content)
- **Baseline hash**: See `.github/spec-baselines.json` for current hash

### MCP Areas to Watch

| Area | Current Status | Potential Rule Impact |
|------|---------------|---------------------|
| Authentication patterns | No standard yet | New MCP-009+ rules for auth validation |
| Remote transport (SSE/WebSocket) | Supported in spec | Transport-specific validation rules |
| Tool annotations | annotations field in spec | MCP-006 may need expansion |
| Resource subscriptions | In spec, limited adoption | New rules for subscription patterns |
| Sampling/completions | Spec-defined, vendor-specific | Cross-vendor compatibility rules |

---

## Community Feedback Log

Tracking community input that influences rule development, tool support decisions, and validation improvements.

### Addressed Items

| Date | Source | Feedback | Action Taken | Issue/PR |
|------|--------|----------|-------------|----------|
| 2026-01-15 | GitHub Issues | Skills invoke at 0% without explicit trigger phrases | Added AS-010 rule for missing trigger phrase detection; sourced from Vercel research | #14 |
| 2026-01-20 | awesome-slash patterns | Enhance plugins identified 70 production-tested config patterns | Created PATTERNS-CATALOG.md; patterns informed CC-SK-007, CC-HK-009, CC-MEM-005 rules | #28 |
| 2026-02-01 | February 2026 monthly review | Coverage gap: no rules for Aider, Continue, Roo Code, Kiro CLI | Documented in tool inventory; awaiting community contributions via issue templates | #191 |
| 2026-02-01 | README pain points | "Almost-right configs" and "skills don't auto-trigger" as top developer frustrations | Prioritized auto-fix for AS-004 (kebab-case), AS-010 (trigger phrase), CC-HK-001 (event name) | #45, #46 |
| 2026-02-01 | Tool tier decisions | Community adoption data used to assign S/A/B/C/D/E tiers | Tier assignments documented in CLAUDE.md; spec-drift frequency matches tier priority | #107 |

### Pending Items

| Date | Source | Feedback | Status |
|------|--------|----------|--------|
| 2026-02-05 | Tool inventory audit | B/C/D tier tools lack rule coverage | Awaiting community contributions; issue templates now available |
| 2026-02-05 | Cross-tool compatibility | Users mixing Cursor + Claude Code + Copilot report silent failures | XP-004/005/006 rules address some cases; more patterns needed |

---

## Update Process

When this document needs updating:

1. **Tool changes**: Update the Tool Inventory table when tools change tiers, config formats, or documentation URLs
2. **New research**: Add papers to the Academic Research table when findings are actionable for rule development
3. **Spec changes**: spec-drift.yml creates issues automatically; update Documentation Sources after addressing drift
4. **Community feedback**: Log feedback in the Community Feedback Log; link to resulting issues/PRs
5. **Monthly reviews**: See [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) for the structured review process
