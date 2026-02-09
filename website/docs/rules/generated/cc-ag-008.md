---
id: cc-ag-008
title: "CC-AG-008: Invalid Memory Scope - Claude Agents"
sidebar_label: "CC-AG-008"
description: "agnix rule CC-AG-008 checks for invalid memory scope in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-008", "invalid memory scope", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-008`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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
description: Agent with invalid memory
memory: global
---
Agent instructions.```

### Valid

```markdown
---
name: my-agent
description: Agent with valid memory
memory: project
---
Agent instructions.```
