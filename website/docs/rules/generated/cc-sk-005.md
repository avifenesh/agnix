---
id: cc-sk-005
title: "CC-SK-005: Invalid Agent Type - Claude Skills"
sidebar_label: "CC-SK-005"
description: "agnix rule CC-SK-005 checks for invalid agent type in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-005", "invalid agent type", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-005`
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
name: search-files
description: Use when searching for files in the project
context: fork
agent: Invalid Agent!!
---
Search the project for relevant files.```

### Valid

```markdown
---
name: search-files
description: Use when searching for files in the project
context: fork
agent: Explore
---
Search the project for relevant files.```
