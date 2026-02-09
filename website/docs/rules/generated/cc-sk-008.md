---
id: cc-sk-008
title: "CC-SK-008: Unknown Tool Name - Claude Skills"
sidebar_label: "CC-SK-008"
description: "agnix rule CC-SK-008 checks for unknown tool name in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-008", "unknown tool name", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-008`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/settings

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: find-files
description: Use when finding files by pattern
allowed-tools: Glob, Grep, FooBar
---
Search for files matching the pattern.```

### Valid

```markdown
---
name: find-files
description: Use when finding files by pattern
allowed-tools: Glob, Grep, Read
---
Search for files matching the pattern.```
