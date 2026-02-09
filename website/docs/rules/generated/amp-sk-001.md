---
id: amp-sk-001
title: "AMP-SK-001: Amp Skill Uses Unsupported Field - amp-skills"
sidebar_label: "AMP-SK-001"
description: "agnix rule AMP-SK-001 checks for amp skill uses unsupported field in amp-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AMP-SK-001", "amp skill uses unsupported field", "amp-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AMP-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `amp-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `amp`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.amp.dev/setup/customization

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
