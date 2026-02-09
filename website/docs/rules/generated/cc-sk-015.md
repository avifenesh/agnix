---
id: cc-sk-015
title: "CC-SK-015: Invalid user-invocable Type - Claude Skills"
sidebar_label: "CC-SK-015"
description: "agnix rule CC-SK-015 checks for invalid user-invocable type in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-015", "invalid user-invocable type", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-015`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

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
name: slash-cmd
description: Use when user types the slash command
user-invocable: "false"
---
Handle the slash command.```

### Valid

```markdown
---
name: slash-cmd
description: Use when user types the slash command
user-invocable: true
---
Handle the slash command.```
