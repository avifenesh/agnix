---
id: cc-sk-007
title: "CC-SK-007: Unrestricted Bash - Claude Skills"
sidebar_label: "CC-SK-007"
description: "agnix rule CC-SK-007 checks for unrestricted bash in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-007", "unrestricted bash", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-007`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/awesome-slash

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
