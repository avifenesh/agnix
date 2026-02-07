---
id: cc-mem-011
title: "CC-MEM-011: Invalid Paths Glob in Rules - Claude Memory"
sidebar_label: "CC-MEM-011"
description: "agnix rule CC-MEM-011 checks for invalid paths glob in rules in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-011", "invalid paths glob in rules", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-011`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-07`

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
---
paths:
  - "[unclosed"
---
# Rule content
```

### Valid

```markdown
---
paths:
  - "src/**/*.ts"
  - "lib/**/*.js"
---
# Rule content
```
