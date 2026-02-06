---
id: cc-hk-009
title: "CC-HK-009: Dangerous Command Pattern - Claude Hooks"
sidebar_label: "CC-HK-009"
description: "agnix rule CC-HK-009 checks for dangerous command pattern in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-009", "dangerous command pattern", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-009`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
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

```json
{
  "hooks": [
    {
      "event": "PreToolUse",
      "matcher": "*"
    }
  ]
}
```

### Valid

```json
{
  "hooks": [
    {
      "event": "PreToolUse",
      "matcher": "Write",
      "command": "./scripts/validate.sh",
      "timeout": 30
    }
  ]
}
```
