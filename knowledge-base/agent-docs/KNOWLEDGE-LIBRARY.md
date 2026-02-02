# AI Agent Knowledge Library (Index)

> Curated entrypoint to internal standards and authoritative external sources.  
> Keep this file short to avoid drift; prefer links to official docs.

---

## How to Use This Library

**Implementation**: [VALIDATION-RULES.md](../VALIDATION-RULES.md)  
**Standards**: [standards/](../standards/) (HARD-RULES + OPINIONS)  
**Patterns**: [PATTERNS-CATALOG.md](../PATTERNS-CATALOG.md)  
**Platform References**: This directory (`agent-docs/`)

---

## Internal References (Canonical)

- [Master Index](../INDEX.md)
- [Validation Rules](../VALIDATION-RULES.md)
- [Patterns Catalog](../PATTERNS-CATALOG.md)
- [Claude Code Reference](./CLAUDE-CODE-REFERENCE.md)
- [Codex CLI Reference](./CODEX-REFERENCE.md)
- [OpenCode Reference](./OPENCODE-REFERENCE.md)
- [Prompt Engineering Reference](./PROMPT-ENGINEERING-REFERENCE.md)
- [Function Calling & Tool Use Reference](./FUNCTION-CALLING-TOOL-USE-REFERENCE.md)
- [Instruction Following Reliability](./LLM-INSTRUCTION-FOLLOWING-RELIABILITY.md)
- [Context Optimization Reference](./CONTEXT-OPTIMIZATION-REFERENCE.md)

---

## External Sources (Authoritative)

### Prompt Engineering

- Anthropic Prompt Engineering  
  https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering
- OpenAI Prompt Engineering Guide  
  https://platform.openai.com/docs/guides/prompt-engineering

### Tool Use & Function Calling

- OpenAI Tools / Function Calling  
  https://platform.openai.com/docs/guides/function-calling
- Anthropic Tool Use  
  https://docs.anthropic.com/en/docs/build-with-claude/tool-use
- Model Context Protocol (MCP)  
  https://modelcontextprotocol.io

### Instruction Following & Reliability

- “Lost in the Middle” (context position effects)  
  https://arxiv.org/abs/2307.03172
- Anthropic: Prompting Best Practices  
  https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering

### Platform-Specific Instruction Files

- Claude Code memory (`CLAUDE.md`)  
  https://docs.anthropic.com/en/docs/claude-code/memory
- Codex CLI `AGENTS.md`  
  https://developers.openai.com/codex/guides/agents-md/
- OpenCode `AGENTS.md`
  https://opencode.ai/docs/rules/
- Cursor rules (`.cursor/rules/*.mdc`)  
  https://docs.cursor.com/en/context/rules
- Cline instructions  
  https://docs.cline.bot/features/custom-instructions
- GitHub Copilot coding agent `AGENTS.md` support  
  https://github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions/

---

## Drift Control

- Prefer links to official sources over copied content.
- Keep this index short; move detailed guidance into targeted references.
- When a vendor doc changes, update the matching **HARD-RULES** first.

---

**Last Updated**: 2026-02-01
