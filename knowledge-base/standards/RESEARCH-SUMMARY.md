# Cross-Platform Agent Standards Research Summary

**Research Date:** 2026-01-31

## Goal
Establish the minimum set of factual, verifiable constraints required to maintain cross-platform project instructions across the top supported tools.

## Scope (support tiers order)

### S Tier (test always)
- Claude Code
- Codex CLI
- OpenCode

### A Tier (test on major changes)
- GitHub Copilot coding agent
- Cursor
- Cline

---

## Key Findings

1. **No universal instruction filename exists**
   - Claude Code uses `CLAUDE.md` (plus `.claude/rules/*.md`).
   - Several other tools use `AGENTS.md`.

2. **AGENTS.md is widely supported, but not universal**
   - Supported by Codex CLI, OpenCode, GitHub Copilot coding agent, Cursor, and Cline.
   - Not documented as a Claude Code memory filename.

3. **Cursor rules are now file-based in `.cursor/rules/*.mdc`**
   - `.cursorrules` is legacy and Cursor recommends migrating to Project Rules.

4. **Precedence and nesting is tool-specific**
   - Codex CLI supports layered instructions (global + per-directory + override).
   - Copilot coding agent supports nested `AGENTS.md` for subtrees.
   - OpenCode supports local + global AGENTS and may fall back to `Claude.md`.
   - Cursor supports root-level `AGENTS.md` (nested planned) and `.cursor/rules/`.
   - Cline supports `.clinerules*`, `AGENTS.md`, and can read `.cursor/rules/`.

---

## Sources Consulted (Official / Primary)

- Claude Code memory: https://docs.anthropic.com/en/docs/claude-code/memory
- Codex CLI AGENTS.md: https://developers.openai.com/codex/guides/agents-md/
- OpenCode project docs: https://opencode.ai/docs/guides/project-docs/
- OpenCode config: https://opencode.ai/docs/config/
- Cursor rules and AGENTS.md:
  - https://docs.cursor.com/en/context/rules
  - https://docs.cursor.com/en/context
  - https://docs.cursor.com/en/cli/using
- Cline custom instructions: https://docs.cline.bot/features/custom-instructions
- Copilot coding agent AGENTS.md support: https://github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions/

---

## Outputs

1. `multi-platform-HARD-RULES.md`
   - A sourced, S/A-tier-focused compatibility matrix and hard constraints.
2. `multi-platform-OPINIONS.md`
   - Recommendations, patterns, and best practices (may include broader ecosystem content; keep clearly separated from HARD rules).

---

## Known Gaps

1. **Cursor `.mdc` schema details**
   - Cursor documents file location and format type, but the full schema/grammar for `.mdc` is not described as a stable standard.
2. **Cross-tool standardization**
   - AGENTS.md is an emerging convention across multiple tools, but precedence rules and supported companion files differ by tool.

---

## Maintenance

Re-run this research when:
- Any S/A tier tool changes its instruction discovery behavior
- Cursor updates its Rules system
- Codex CLI updates AGENTS file precedence or naming
