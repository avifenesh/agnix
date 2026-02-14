---
id: kiro-002
title: "KIRO-002: Missing Required Fields for Inclusion Mode"
sidebar_label: "KIRO-002"
description: "agnix rule KIRO-002 checks for missing required fields for inclusion mode in kiro steering files. Severity: HIGH. See examples and fix guidance."
keywords: ["KIRO-002", "missing required fields for inclusion mode", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-002`
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
inclusion: auto
---
# TypeScript Guidelines
```

### Valid

```markdown
---
inclusion: auto
name: typescript-guidelines
description: Guidelines for TypeScript development
---
# TypeScript Guidelines
```
