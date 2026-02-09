---
id: cc-sk-007
title: "CC-SK-007: Unrestricted Bash - Claude Skills"
sidebar_label: "CC-SK-007"
description: "agnix rule CC-SK-007 checks for unrestricted bash in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-007", "unrestricted bash", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-007`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/awesome-slash

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: git-status
description: Use when checking git status
allowed-tools: Bash, Read
---
Run git status and read the output.
```

### Valid

```markdown
---
name: git-status
description: Use when checking git status
allowed-tools: Bash(git:*), Read
---
Run git status and read the output.
```
