---
id: cc-mem-012
title: "CC-MEM-012: Rules File Unknown Frontmatter Key - Claude Memory"
sidebar_label: "CC-MEM-012"
description: "agnix rule CC-MEM-012 checks for rules file unknown frontmatter key in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-012", "rules file unknown frontmatter key", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-012`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
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
  - "src/**/*.ts"
description: "This key is not recognized"
alwaysApply: true
---
# Rule content
```

### Valid

```markdown
---
paths:
  - "src/**/*.ts"
---
# Rule content
```
