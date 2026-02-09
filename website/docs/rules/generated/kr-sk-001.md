---
id: kr-sk-001
title: "KR-SK-001: Kiro Skill Uses Unsupported Field - kiro-skills"
sidebar_label: "KR-SK-001"
description: "agnix rule KR-SK-001 checks for kiro skill uses unsupported field in kiro-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SK-001", "kiro skill uses unsupported field", "kiro-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `kiro-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/context/steering

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
model: haiku
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
