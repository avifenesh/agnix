---
id: cc-hk-011
title: "CC-HK-011: Invalid Timeout Value - Claude Hooks"
sidebar_label: "CC-HK-011"
description: "agnix rule CC-HK-011 checks for invalid timeout value in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-011", "invalid timeout value", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-011`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo ok", "timeout": -5 }
        ]
      }
    ]
  }
}
```

### Valid

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo ok", "timeout": 30 }
        ]
      }
    ]
  }
}
```
