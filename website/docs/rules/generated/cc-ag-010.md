---
id: cc-ag-010
title: "CC-AG-010: Invalid Tool Name in DisallowedTools"
sidebar_label: "CC-AG-010"
description: "agnix rule CC-AG-010 checks for invalid tool name in disallowedtools in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-010", "invalid tool name in disallowedtools", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-010`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: my-agent
description: Agent with unknown disallowed tool
disallowedTools:
  - Bash
  - RunCode
---
Agent instructions.```

### Valid

```markdown
---
name: my-agent
description: Agent with valid disallowed tools
disallowedTools:
  - Bash
  - WebFetch
---
Agent instructions.```
