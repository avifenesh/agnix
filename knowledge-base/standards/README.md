# AI Coding Assistant Standards Documentation

**Last Updated:** 2026-01-31

## Start Here

1. `RESEARCH-SUMMARY.md` - What was validated and which sources were used (S/A tiers first)
2. `multi-platform-HARD-RULES.md` - Cross-tool filename and precedence constraints (S/A tiers)
3. Pick a standard:
   - `claude-code-HARD-RULES.md` / `claude-code-OPINIONS.md`
   - `agent-skills-HARD-RULES.md` / `agent-skills-OPINIONS.md`
   - `mcp-HARD-RULES.md` / `mcp-OPINIONS.md`
   - `prompt-engineering-HARD-RULES.md` / `prompt-engineering-OPINIONS.md`

---

## Support Tiers (Ordering Rule)

When documenting cross-platform behavior, list and validate tools in this order:

### S Tier (test always)
- Claude Code
- Codex CLI
- OpenCode

### A Tier (test on major changes)
- GitHub Copilot coding agent
- Cursor
- Cline

---

## HARD RULES vs OPINIONS

- HARD RULES: only include non-negotiable, sourced facts (what is loaded / ignored / breaks)
- OPINIONS: recommendations and patterns (include rationale; note confidence when uncertain)

---

## Quick File Naming Cheat Sheet (S/A)

- Claude Code: `CLAUDE.md`, `.claude/rules/*.md`
- Codex CLI: `AGENTS.md` (+ `AGENTS.local.md`, `AGENTS.override.md`)
- OpenCode: `AGENTS.md` (+ `AGENTS.local.md`, global `~/.config/opencode/AGENTS.md`, `Claude.md` fallback)
- Cursor: `.cursor/rules/*.mdc` (preferred), `.cursorrules` (legacy), plus `AGENTS.md`/`CLAUDE.md` supported at root
- Cline: `.clinerules` or `.clinerules/`, plus `AGENTS.md`

