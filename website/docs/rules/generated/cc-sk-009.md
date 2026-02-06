---
id: cc-sk-009
title: "CC-SK-009 Too Many Injections"
sidebar_label: "CC-SK-009"
---

## Summary

- **Rule ID**: `CC-SK-009`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://platform.claude.com/docs

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
