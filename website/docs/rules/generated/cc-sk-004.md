---
id: cc-sk-004
title: "CC-SK-004: Agent Without Context - Claude Skills"
sidebar_label: "CC-SK-004"
description: "agnix rule CC-SK-004 checks for agent without context in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-004", "agent without context", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-004`
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
name: explore-code
description: Use when exploring the codebase
agent: Explore
---
Explore the codebase structure.```

### Valid

```markdown
---
name: explore-code
description: Use when exploring the codebase
context: fork
agent: Explore
---
Explore the codebase structure.```
