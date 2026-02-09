---
id: rc-sk-001
title: "RC-SK-001: Roo Code Skill Uses Unsupported Field"
sidebar_label: "RC-SK-001"
description: "agnix rule RC-SK-001 checks for roo code skill uses unsupported field in roo-code-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["RC-SK-001", "roo code skill uses unsupported field", "roo-code-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `RC-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `roo-code-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `roo-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.roocode.com/features/custom-instructions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: my-skill
description: A useful development skill
disable-model-invocation: false
---
# My Skill

Skill instructions here.
```

### Valid

```text
---
name: my-skill
description: A useful development skill
---
# My Skill

Skill instructions here.
```
