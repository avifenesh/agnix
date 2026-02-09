---
id: cl-sk-001
title: "CL-SK-001: Cline Skill Uses Unsupported Field - Cline Skills"
sidebar_label: "CL-SK-001"
description: "agnix rule CL-SK-001 checks for cline skill uses unsupported field in cline skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CL-SK-001", "cline skill uses unsupported field", "cline skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CL-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `Cline Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `cline`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.cline.bot/features/custom-instructions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: my-skill
description: A useful development skill
context: fork
---
# My Skill

Skill instructions here.
```

### Valid

```markdown
---
name: my-skill
description: A useful development skill
---
# My Skill

Skill instructions here.
```
