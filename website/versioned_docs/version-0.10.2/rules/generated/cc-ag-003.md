---
id: cc-ag-003
title: "CC-AG-003: Invalid Model Value - Claude Agents"
sidebar_label: "CC-AG-003"
description: "agnix rule CC-AG-003 checks for invalid model value in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-003", "invalid model value", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-003`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: reviewer
---
```

### Valid

```markdown
---
name: reviewer
description: Review code for correctness and tests
model: sonnet
tools: [Read, Grep, Bash]
---
```
