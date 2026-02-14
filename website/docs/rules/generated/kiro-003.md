---
id: kiro-003
title: "KIRO-003: Invalid fileMatchPattern Glob - Kiro Steering"
sidebar_label: "KIRO-003"
description: "agnix rule KIRO-003 checks for invalid filematchpattern glob in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-003", "invalid filematchpattern glob", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-003`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-14`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/steering/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
inclusion: fileMatch
fileMatchPattern: "[unclosed"
---
# TypeScript Guidelines
```

### Valid

```markdown
---
inclusion: fileMatch
fileMatchPattern: "**/*.ts"
---
# TypeScript Guidelines
```
