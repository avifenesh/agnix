---
id: cc-mem-001
title: "CC-MEM-001: Invalid Import Path - Claude Memory"
sidebar_label: "CC-MEM-001"
description: "agnix rule CC-MEM-001 checks for invalid import path in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-001", "invalid import path", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-001`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory

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
