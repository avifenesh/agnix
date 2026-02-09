---
id: cc-mem-002
title: "CC-MEM-002: Circular Import - Claude Memory"
sidebar_label: "CC-MEM-002"
description: "agnix rule CC-MEM-002 checks for circular import in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-002", "circular import", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-002`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Project Memory

@import ./CLAUDE.md

This file imports itself, creating a circular dependency.
```

### Valid

```markdown
# Project Memory

@import ./docs/style.md
@import ./docs/testing.md

Follow these standards.
```
