---
id: cc-mem-006
title: "CC-MEM-006: Negative Without Positive - Claude Memory"
sidebar_label: "CC-MEM-006"
description: "agnix rule CC-MEM-006 checks for negative without positive in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-006", "negative without positive", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-006`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://arxiv.org/abs/2201.11903

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Rules

Never use var in JavaScript.
```

### Valid

```markdown
# Rules

Never use var in JavaScript, instead prefer const or let.
```
