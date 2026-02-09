---
id: cc-ag-006
title: "CC-AG-006: Tool/Disallowed Conflict - Claude Agents"
sidebar_label: "CC-AG-006"
description: "agnix rule CC-AG-006 checks for tool/disallowed conflict in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-006", "tool/disallowed conflict", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-006`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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
name: conflict-agent
description: Agent with conflicting tool lists
tools:
  - Read
  - Bash
disallowedTools:
  - Bash
---
Agent instructions.```

### Valid

```markdown
---
name: safe-agent
description: Agent with separate tool lists
tools:
  - Read
  - Grep
disallowedTools:
  - Bash
---
Agent instructions.```
