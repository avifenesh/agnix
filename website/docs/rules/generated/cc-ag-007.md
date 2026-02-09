---
id: cc-ag-007
title: "CC-AG-007: Agent Parse Error - Claude Agents"
sidebar_label: "CC-AG-007"
description: "agnix rule CC-AG-007 checks for agent parse error in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-007", "agent parse error", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-007`
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
name: my-agent
description: Missing the --- delimiters

Agent instructions here.```

### Valid

```markdown
---
name: my-agent
description: A working agent
---
Agent instructions here.```
