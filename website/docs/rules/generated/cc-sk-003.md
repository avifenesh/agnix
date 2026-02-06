---
id: cc-sk-003
title: "CC-SK-003: Context Without Agent - Claude Skills"
sidebar_label: "CC-SK-003"
description: "agnix rule CC-SK-003 checks for context without agent in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-003", "context without agent", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-003`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
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

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: Deploy_Prod
description: Deploys production changes
---
```

### Valid

```markdown
---
name: deploy-prod
description: Deploy production with explicit checks
---
```
