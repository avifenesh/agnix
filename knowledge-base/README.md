# agnix Knowledge Base

> Canonical documentation for validation rules, standards, and research sources.

## Start Here

- [INDEX.md](./INDEX.md) - Master navigation and summaries
- [VALIDATION-RULES.md](./VALIDATION-RULES.md) - 212 rules with detection logic

- [PATTERNS-CATALOG.md](./PATTERNS-CATALOG.md) - 70 patterns from agentsys
- [standards/](./standards/) - HARD-RULES and OPINIONS by topic
- [agent-docs/](./agent-docs/) - Platform references and research

## Structure (High-Level)

```
knowledge-base/
├── INDEX.md
├── VALIDATION-RULES.md
├── PATTERNS-CATALOG.md
├── standards/
└── agent-docs/
```

## Update Rules

- Keep rule counts consistent across `README.md`, `SPEC.md`, `CLAUDE.md`, and `knowledge-base/INDEX.md`.
- When facts change, update sources in `knowledge-base/VALIDATION-RULES.md`.
- For cross-platform content, follow support tiers ordering (S tier first, then A).
- Prefer authoritative sources; avoid duplicating long-form guidance from vendor docs.

---

**Last Updated**: 2026-02-14
**Pattern Source**: agentsys v3.6.1 (production-tested)
