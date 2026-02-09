---
id: cc-sk-002
title: "CC-SK-002: Invalid Context Value - Claude Skills"
sidebar_label: "CC-SK-002"
description: "agnix rule CC-SK-002 checks for invalid context value in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-002", "invalid context value", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-002`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: parallel-task
description: Use when running tasks in parallel
context: spawn
agent: general-purpose
---
Run the task in a spawned context.```

### Valid

```markdown
---
name: parallel-task
description: Use when running tasks in parallel
context: fork
agent: general-purpose
---
Run the task in a forked context.```
