---
id: cc-ag-004
title: "CC-AG-004: Invalid Permission Mode - Claude Agents"
sidebar_label: "CC-AG-004"
description: "agnix rule CC-AG-004 checks for invalid permission mode in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-004", "invalid permission mode", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-004`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: code-reviewer
description: Reviews code
permissionMode: yolo
---
Review code changes.
```

### Valid

```markdown
---
name: code-reviewer
description: Reviews code
permissionMode: acceptEdits
---
Review code changes.
```
