---
id: cc-hk-016
title: "CC-HK-016: Validate Hook Type Agent - Claude Hooks"
sidebar_label: "CC-HK-016"
description: "agnix rule CC-HK-016 checks for validate hook type agent in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-016", "validate hook type agent", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-016`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-07`

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
    "Stop": [
      {
        "hooks": [
          { "type": "webhook", "url": "https://example.com", "timeout": 30 }
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
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "echo done", "timeout": 30 }
        ]
      }
    ]
  }
}
```
