---
id: cc-sk-011
title: "CC-SK-011: Unreachable Skill - Claude Skills"
sidebar_label: "CC-SK-011"
description: "agnix rule CC-SK-011 checks for unreachable skill configuration in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-011", "unreachable skill", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-011`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: my-skill
description: Use when testing
user-invocable: false
disable-model-invocation: true
---
```

### Valid

```markdown
---
name: my-skill
description: Use when testing
user-invocable: true
disable-model-invocation: true
---
```
