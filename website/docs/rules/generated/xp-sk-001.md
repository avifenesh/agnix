---
id: xp-sk-001
title: "XP-SK-001: Skill Uses Client-Specific Features"
sidebar_label: "XP-SK-001"
description: "agnix rule XP-SK-001 checks for skill uses client-specific features in cross-platform files. Severity: LOW. See examples and fix guidance."
keywords: ["XP-SK-001", "skill uses client-specific features", "cross-platform", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `XP-SK-001`
- **Severity**: `LOW`
- **Category**: `Cross-Platform`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://agentskills.io/specification

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: code-review
description: Reviews code for quality issues
model: opus
context: fork
agent: general-purpose
---
Review the code for bugs and style issues.
```

### Valid

```markdown
---
name: code-review
description: Reviews code for quality issues
---
Review the code for bugs and style issues.
```
