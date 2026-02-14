---
id: cc-hk-003
title: "CC-HK-003: Matcher Hint for Tool Events - Claude Hooks"
sidebar_label: "CC-HK-003"
description: "agnix rule CC-HK-003 checks for matcher hint for tool events in claude hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-HK-003", "matcher hint for tool events", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-003`
- **Severity**: `LOW`
- **Category**: `Claude Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
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
        "hooks": [
          { "type": "command", "command": "echo ok", "timeout": 30 }
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
