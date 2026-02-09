---
id: cx-sk-001
title: "CX-SK-001: Codex Skill Uses Unsupported Field - codex-skills"
sidebar_label: "CX-SK-001"
description: "agnix rule CX-SK-001 checks for codex skill uses unsupported field in codex-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CX-SK-001", "codex skill uses unsupported field", "codex-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CX-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `codex-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/guides/agents-md

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
hooks: some-value
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
