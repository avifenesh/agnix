---
id: cc-hk-001
title: "CC-HK-001: Invalid Hook Event - Claude Hooks"
sidebar_label: "CC-HK-001"
description: "agnix rule CC-HK-001 checks for invalid hook event in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-001", "invalid hook event", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-001`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-02-13`

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
    "OnBeforeTool": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo hello", "timeout": 30 }
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
          { "type": "command", "command": "echo pre-tool", "timeout": 30 }
        ]
      }
    ]
  }
}
```
