---
id: cc-mem-004
title: "CC-MEM-004: Invalid Command Reference - Claude Memory"
sidebar_label: "CC-MEM-004"
description: "agnix rule CC-MEM-004 checks for invalid command reference in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-004", "invalid command reference", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-004`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Commands

Run tests with `npm run nonexistent`
```

### Valid

```markdown
# Commands

Run tests with `npm run test`
Build with `npm run build`
```
