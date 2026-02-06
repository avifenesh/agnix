---
id: cc-hk-006
title: "CC-HK-006: Missing Command Field - Claude Hooks"
sidebar_label: "CC-HK-006"
description: "agnix rule CC-HK-006 checks for missing command field in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-006", "missing command field", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-006`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks

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
