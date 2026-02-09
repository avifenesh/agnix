---
id: cc-hk-004
title: "CC-HK-004: Matcher on Non-Tool Event - Claude Hooks"
sidebar_label: "CC-HK-004"
description: "agnix rule CC-HK-004 checks for matcher on non-tool event in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-004", "matcher on non-tool event", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-004`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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
    "Notification": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo notified", "timeout": 30 }
        ]
      }
    ]
  }
}```

### Valid

```json
{
  "hooks": {
    "Notification": [
      {
        "hooks": [
          { "type": "command", "command": "echo notified", "timeout": 30 }
        ]
      }
    ]
  }
}```
