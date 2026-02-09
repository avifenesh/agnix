---
id: cc-ag-009
title: "CC-AG-009: Invalid Tool Name in Tools List - Claude Agents"
sidebar_label: "CC-AG-009"
description: "agnix rule CC-AG-009 checks for invalid tool name in tools list in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-009", "invalid tool name in tools list", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-009`
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
description: Agent with unknown tool
tools:
  - Read
  - MakeFile
---
Agent instructions.```

### Valid

```markdown
---
name: my-agent
description: Agent with valid tools
tools:
  - Read
  - Write
  - Bash
---
Agent instructions.```
