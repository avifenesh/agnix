---
id: kiro-001
title: "KIRO-001: Invalid Steering File Inclusion Mode"
sidebar_label: "KIRO-001"
description: "agnix rule KIRO-001 checks for invalid steering file inclusion mode in kiro steering files. Severity: HIGH. See examples and fix guidance."
keywords: ["KIRO-001", "invalid steering file inclusion mode", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-001`
- **Severity**: `HIGH`
- **Category**: `Kiro Steering`
- **Normative Level**: `MUST`
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
# TypeScript Guidelines

Use strict mode.
```

### Valid

```markdown
---
inclusion: always
---
# TypeScript Guidelines

Use strict mode.
```
