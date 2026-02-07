# Monthly Review Checklist

> Structured process for reviewing AI tool ecosystem changes and maintaining rule accuracy.

**Cadence**: Monthly, 1st week of each month
**Related**: [RESEARCH-TRACKING.md](./RESEARCH-TRACKING.md) | [VALIDATION-RULES.md](./VALIDATION-RULES.md) | [INDEX.md](./INDEX.md)

---

## Pre-Review Preparation

Before starting the monthly review:

- [ ] Run `spec-drift.yml` workflow manually to get fresh baseline comparisons
- [ ] Check open GitHub issues labeled `spec-drift` for unresolved drift reports
- [ ] Review community feedback since last review (GitHub Issues, Discussions)
- [ ] Note current rule count and coverage statistics for comparison
- [ ] Pull latest `.github/spec-baselines.json` to see monitored sources

---

## Per-Tier Review Checklist

### S Tier: Claude Code, Codex CLI, OpenCode

These tools have automated monitoring. The review focuses on confirming automation is working and catching anything automated checks miss.

- [ ] Verify spec-drift weekly alerts are firing (check workflow run history)
- [ ] Review any open `spec-drift` issues for S-tier sources
- [ ] Check for new features or config format changes not covered by spec-drift
- [ ] Check for deprecation announcements
- [ ] Verify rule coverage is comprehensive for any new features
- [ ] Update `spec-baselines.json` if baselines were intentionally refreshed

**Sources to check**:
- https://code.claude.com/docs/en (all sections)
- https://developers.openai.com/codex/ (AGENTS.md guide, changelog)
- https://opencode.ai/docs/ (rules, config)

### A Tier: GitHub Copilot, Cline, Cursor

These tools have automated monthly monitoring. Review confirms accuracy and checks for major changes.

- [ ] Verify spec-drift monthly alerts are firing for A-tier sources
- [ ] Manually check each tool's documentation for changes
- [ ] Verify rule coverage matches current tool capabilities
- [ ] Note any significant feature additions that need new rules

**Sources to check**:
- https://docs.github.com/en/copilot/customizing-copilot
- https://docs.cline.bot/features/cline-rules/overview
- https://cursor.com/docs/context/rules

### B/C Tier: Roo Code, Kiro CLI, amp, pi, gemini cli, continue, Antigravity

Spot-check for significant changes only. Do not invest significant time here.

- [ ] Quick scan of documentation pages for breaking changes
- [ ] Note any tools that have gained significant community adoption (potential tier upgrade)
- [ ] Check if any tools have been deprecated or discontinued

### D/E Tier: Tabnine, Codeium, Amazon Q, Windsurf, Aider, SourceGraph Cody, others

- [ ] Check if any D/E tools have gained enough adoption to warrant a tier upgrade
- [ ] Review any community-submitted issues requesting tool support
- [ ] Note any new AI coding tools that should be added to the watchlist

---

## Cross-Cutting Items

### Academic Research

- [ ] Search for new papers on prompt engineering, instruction-following, and LLM configuration
- [ ] Check if existing research citations are still current
- [ ] Note any findings that could inform new rules

### MCP Ecosystem

- [ ] Check mcp-release-watch.yml for new MCP spec releases
- [ ] Review MCP servers registry for new patterns
- [ ] Check for new transport types or authentication standards

### AGENTS.md Ecosystem

- [ ] Check if additional tools have adopted AGENTS.md format
- [ ] Review for format extensions or version changes

### New Config Patterns

- [ ] Check awesome-slash for updated enhance patterns
- [ ] Note any emerging config patterns across tools
- [ ] Identify patterns that could benefit from new rules

---

## Post-Review Actions

After completing the review:

- [ ] File GitHub issues for any identified gaps or needed updates
- [ ] Update RESEARCH-TRACKING.md with new findings
- [ ] Update spec-baselines.json if sources were verified
- [ ] Update "Last Reviewed" dates in RESEARCH-TRACKING.md tool inventory
- [ ] Document the completed review in the section below

---

## Completed Reviews

### February 2026

**Reviewer**: Automated + manual review
**Date**: 2026-02-05

#### Current State

- **Rules**: 139 validation rules across 17 categories
- **Sources monitored**: 12 sources in `.github/spec-baselines.json`
- **Tests**: 1500+ passing tests

#### Coverage Analysis

| Tool/Category | Rule Count | Coverage Status |
|--------------|------------|-----------------|
| Claude Code (CC-SK, CC-HK, CC-MEM, CC-AG, CC-PL) | 50+ | Comprehensive |
| Cursor (CUR-*) | 6 | Good - covers core validation |
| GitHub Copilot (COP-*) | 6 | Good - covers instruction files and validation |
| AGENTS.md (AGM-*) | 6 | Good - covers structure and cross-platform |
| MCP (MCP-*) | 8 | Good - covers protocol compliance |
| Agent Skills (AS-*) | 16 | Comprehensive |
| Cross-Platform (XP-*) | 6 | Good - covers contradiction detection |
| Prompt Engineering (PE-*) | 4 | Adequate - research-backed |
| Windsurf | 1 (AGM-003 character limit) | Minimal |
| Aider | 0 | No coverage |
| Continue | 0 | No coverage |
| Roo Code | 0 | No coverage |
| Kiro CLI | 0 | No coverage |
| Cline | 0 (monitored, no rules yet) | Monitoring only |

#### Automated Monitoring Status

- **spec-drift.yml**: Running successfully on weekly (S-tier) and monthly (A-tier) schedules
- **mcp-release-watch.yml**: Running successfully, monitoring MCP GitHub releases
- **spec-baselines.json**: 12 sources tracked with SHA256 content hashes

#### Findings

1. **S-tier coverage is strong**: Claude Code has 50+ rules covering skills, hooks, memory, agents, and plugins. Codex CLI and OpenCode share AGM and XP rules.
2. **A-tier has good baseline**: Cursor (6 rules), Copilot (4 rules), and Cline (monitored) provide adequate coverage for the most common config patterns.
3. **B/C/D tier gaps exist**: Roo Code, Kiro CLI, amp, pi, Continue, and Aider have no tool-specific rules. These tools rely on generic rules (AS-*, XP-*, AGM-*) where applicable.
4. **No spec drift detected**: All S-tier and A-tier baselines current as of February 2026.
5. **Issue templates needed**: Added rule contribution and tool support request templates to streamline community contributions for uncovered tools.

#### Actions Taken

- Created RESEARCH-TRACKING.md with complete tool inventory
- Created issue templates for rule contributions and tool support requests
- Expanded CONTRIBUTING.md with rule authoring guide
- Updated INDEX.md to reference new documents
- Added documentation consistency tests
