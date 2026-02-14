---
id: kiro-004
title: "KIRO-004: Empty Kiro Steering File - Kiro Steering"
sidebar_label: "KIRO-004"
description: "agnix rule KIRO-004 checks for empty kiro steering file in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-004", "empty kiro steering file", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-004`
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
inclusion: invalid_mode
---
# Guidelines
```

### Valid

```markdown
---
inclusion: always
---
# TypeScript Guidelines

Use strict mode.
```
