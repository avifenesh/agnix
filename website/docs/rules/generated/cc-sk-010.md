---
id: cc-sk-010
title: "CC-SK-010: Invalid Hooks in Skill Frontmatter"
sidebar_label: "CC-SK-010"
description: "agnix rule CC-SK-010 checks for invalid hooks in skill frontmatter in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-010", "invalid hooks in skill frontmatter", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-010`
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
