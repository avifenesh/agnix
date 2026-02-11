---
id: cp-sk-001
title: "CP-SK-001: Copilot Skill Uses Unsupported Field"
sidebar_label: "CP-SK-001"
description: "agnix rule CP-SK-001 checks for copilot skill uses unsupported field in copilot skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CP-SK-001", "copilot skill uses unsupported field", "copilot skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CP-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `Copilot Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `github-copilot`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.github.com/en/copilot/customizing-copilot/adding-repository-custom-instructions-for-github-copilot

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
agent: general-purpose
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
