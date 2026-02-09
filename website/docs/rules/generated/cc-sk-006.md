---
id: cc-sk-006
title: "CC-SK-006: Dangerous Auto-Invocation - Claude Skills"
sidebar_label: "CC-SK-006"
description: "agnix rule CC-SK-006 checks for dangerous auto-invocation in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-006", "dangerous auto-invocation", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-006`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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
name: deploy-staging
description: Use when deploying to staging environment
---
Deploy the application to staging.
```

### Valid

```markdown
---
name: deploy-staging
description: Use when deploying to staging environment
disable-model-invocation: true
---
Deploy the application to staging.
```
