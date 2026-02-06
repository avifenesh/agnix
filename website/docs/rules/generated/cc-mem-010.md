---
id: cc-mem-010
title: "CC-MEM-010: README Duplication - Claude Memory"
sidebar_label: "CC-MEM-010"
description: "agnix rule CC-MEM-010 checks for readme duplication in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-010", "readme duplication", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-010`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/awesome-slash

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
# Memory
Always be helpful.
```

### Valid

```markdown
# Project Memory
- Use Rust workspace conventions
- Keep AGENTS.md and CLAUDE.md identical
```
