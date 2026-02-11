---
id: cc-sk-011
title: "CC-SK-011: Unreachable Skill - Claude Skills"
sidebar_label: "CC-SK-011"
description: "agnix rule CC-SK-011 checks for unreachable skill in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-011", "unreachable skill", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-011`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-07`

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
name: internal-skill
description: Use when running internal automation
user-invocable: false
disable-model-invocation: true
---
This skill cannot be reached by anyone.
```

### Valid

```markdown
---
name: internal-skill
description: Use when running internal automation
user-invocable: false
---
This skill can still be invoked by the model.
```
