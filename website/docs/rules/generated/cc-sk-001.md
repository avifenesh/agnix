---
id: cc-sk-001
title: "CC-SK-001: Invalid Model Value - Claude Skills"
sidebar_label: "CC-SK-001"
description: "agnix rule CC-SK-001 checks for invalid model value in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-001", "invalid model value", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-001`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: fast-review
description: Use when doing quick code reviews
model: gpt-4
---
Review the code for obvious issues.```

### Valid

```markdown
---
name: fast-review
description: Use when doing quick code reviews
model: haiku
---
Review the code for obvious issues.```
