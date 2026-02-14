---
id: cc-hk-019
title: "CC-HK-019: Deprecated Setup Event - Claude Hooks"
sidebar_label: "CC-HK-019"
description: "agnix rule CC-HK-019 checks for deprecated setup event in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-019", "deprecated setup event", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-019`
- **Severity**: `MEDIUM`
- **Category**: `Claude Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-14`

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
    "Setup": [
      {
        "hooks": [
          { "type": "command", "command": "echo start", "timeout": 30 }
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
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "echo start", "timeout": 30 }
        ]
      }
    ]
  }
}
```
