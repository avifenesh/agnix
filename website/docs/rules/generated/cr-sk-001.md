---
id: cr-sk-001
title: "CR-SK-001: Cursor Skill Uses Unsupported Field"
sidebar_label: "CR-SK-001"
description: "agnix rule CR-SK-001 checks for cursor skill uses unsupported field in cursor-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CR-SK-001", "cursor skill uses unsupported field", "cursor-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CR-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `cursor-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `cursor`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.cursor.com/en/context/skills

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
model: opus
---
# My Skill

Skill instructions here.```

### Valid

```text
---
name: my-skill
description: A useful development skill
disable-model-invocation: true
---
# My Skill

Skill instructions here.```
