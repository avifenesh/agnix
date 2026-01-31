# Multi-Platform Agent Standards - HARD RULES

## Document Purpose
This document contains FACTUAL, NON-NEGOTIABLE compatibility rules for cross-platform AI coding assistant projects. These are technical constraints, not opinions.

**Last Updated:** 2026-01-31

## Scope and Support Tiers

This document is written in priority order using agnix's support tiers:

### S Tier (test always)
- Claude Code
- Codex CLI
- OpenCode

### A Tier (test on major changes)
- GitHub Copilot coding agent
- Cursor
- Cline

Lower tiers are intentionally out of scope for this HARD-RULES document. If you need them, add a separate appendix with explicit sources and confidence.

---

## 1. Instruction and Rules Files (S + A)

### 1.1 Canonical filenames (do not improvise)

| Tool | Tier | Primary Project Instructions | Other Supported Instruction Files | Rules Files |
|------|------|------------------------------|-----------------------------------|------------|
| Claude Code | S | `CLAUDE.md` | `CLAUDE.local.md`, `.claude/rules/*.md` | `.claude/rules/*.md` |
| Codex CLI | S | `AGENTS.md` | `AGENTS.local.md`, `AGENTS.override.md`, `~/.codex/AGENTS.md` | (N/A) |
| OpenCode | S | `AGENTS.md` | `AGENTS.local.md`, `~/.config/opencode/AGENTS.md`, `Claude.md` (fallback) | (N/A) |
| GitHub Copilot coding agent | A | `AGENTS.md` | nested `AGENTS.md`, `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | (N/A) |
| Cursor | A | `.cursor/rules/*.mdc` | `AGENTS.md` (root; nested planned), `CLAUDE.md` (root), `.cursorrules` (legacy) | `.cursor/rules/*.mdc` |
| Cline | A | `.clinerules` or `.clinerules/` | `AGENTS.md`, plus it can read `.cursor/rules/` | `.clinerules*` |

---

## 2. CLAUDE.md vs AGENTS.md (do not assume aliases)

### 2.1 Claude Code requires CLAUDE.md

- Claude Code documents `CLAUDE.md`/`CLAUDE.local.md` and `.claude/rules/*.md` for project memory/rules.
- If you only create `AGENTS.md`, Claude Code will not treat it as its memory file.

### 2.2 AGENTS.md is real, but not universal

AGENTS.md is used by multiple tools, but it is not a universal standard:
- Codex CLI uses `AGENTS.md` and also supports `AGENTS.override.md` and `AGENTS.local.md`.
- OpenCode uses `AGENTS.md` and falls back to `Claude.md` if AGENTS.md is not present.
- Cursor and Cline support `AGENTS.md` as a rules/instructions source (alongside their native rules systems).
- GitHub Copilot coding agent supports `AGENTS.md` (including nested AGENTS.md for subtrees).

### 2.3 Cross-tool strategy

If you want one set of project instructions to work across S/A tier tools:
1. Treat `CLAUDE.md` as the Claude Code entrypoint.
2. Keep an `AGENTS.md` with the same content (or a compatible subset) for Codex/OpenCode/Cline/Cursor/Copilot.
3. Keep Cursor-native rules in `.cursor/rules/*.mdc` when you need Cursor-specific behavior.

---

## 3. Cursor Rules: `.cursor/rules/*.mdc` is the current mechanism

- Cursor's current rules mechanism is the `.cursor/rules/` directory containing `.mdc` rule files.
- `.cursorrules` is legacy and Cursor recommends migrating to the Project Rules format.

---

## 4. Hierarchy and Precedence (tool-specific)

### 4.1 Codex CLI (AGENTS)

Codex CLI loads project instructions in a defined order including global, per-directory, and override files. This means multiple AGENTS.md files may apply to a single run.

### 4.2 GitHub Copilot coding agent (nested AGENTS)

Copilot coding agent supports nested `AGENTS.md` files that apply only to parts of a repository.

### 4.3 OpenCode (AGENTS + fallback)

OpenCode uses `AGENTS.md` (and `AGENTS.local.md`, plus a global AGENTS file) and may fall back to `Claude.md`.

### 4.4 Cursor (AGENTS + Project Rules)

Cursor supports `AGENTS.md` at the project root, and also supports Project Rules under `.cursor/rules/*.mdc`.

### 4.5 Cline (multiple instruction mechanisms)

Cline checks for `.clinerules` (file or directory) and also supports `AGENTS.md`. It can additionally read `.cursor/rules/` when present.

---

## References (Official / Primary)

- Claude Code memory: https://docs.anthropic.com/en/docs/claude-code/memory
- Codex CLI AGENTS.md: https://developers.openai.com/codex/guides/agents-md/
- OpenCode project docs: https://opencode.ai/docs/guides/project-docs/
- Cursor rules and AGENTS.md:
  - https://docs.cursor.com/en/context/rules
  - https://docs.cursor.com/en/context
  - https://docs.cursor.com/en/cli/using
- Cline custom instructions: https://docs.cline.bot/features/custom-instructions
- Copilot coding agent AGENTS.md support: https://github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions/

---

## Document Maintenance

**This is a HARD RULES document.** Only add information that is:
1. Factually verifiable from official sources
2. A non-negotiable technical constraint (what will or will not be loaded/executed)
3. A clear incompatibility you can reproduce
