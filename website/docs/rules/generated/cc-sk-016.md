---
id: cc-sk-016
title: "CC-SK-016: Indexed $ARGUMENTS Without argument-hint"
sidebar_label: "CC-SK-016"
description: "agnix rule CC-SK-016 checks for indexed $arguments without argument-hint in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-016", "indexed $arguments without argument-hint", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-016`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-14`

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
name: run-check
description: Use when validating a provided path
---
Run checks against $ARGUMENTS[0].
```

### Valid

```markdown
---
name: run-check
description: Use when validating a provided path
argument-hint: path-to-target
---
Run checks against $ARGUMENTS[0].
```
