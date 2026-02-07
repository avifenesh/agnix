---
id: cc-sk-012
title: "CC-SK-012: Argument Hint Without $ARGUMENTS - Claude Skills"
sidebar_label: "CC-SK-012"
description: "agnix rule CC-SK-012 checks for argument-hint without $ARGUMENTS reference in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-012", "argument hint without arguments", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-012`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: greeter
description: Use when greeting users
argument-hint: user-name
---
Greet the user with a friendly message.
```

### Valid

```markdown
---
name: greeter
description: Use when greeting users
argument-hint: user-name
---
Greet the user specified in $ARGUMENTS with a friendly message.
```
