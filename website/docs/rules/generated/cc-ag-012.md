---
id: cc-ag-012
title: "CC-AG-012: Bypass Permissions Warning - Claude Agents"
sidebar_label: "CC-AG-012"
description: "agnix rule CC-AG-012 warns when bypassPermissions is used in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-012", "bypass permissions warning", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-012`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-07`

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
name: my-agent
description: Agent that bypasses permissions
permissionMode: bypassPermissions
---
```

### Valid

```markdown
---
name: my-agent
description: Agent with safe permission mode
permissionMode: dontAsk
---
```
