---
id: cc-hk-013
title: "CC-HK-013: Async on Non-Command Hook - Claude Hooks"
sidebar_label: "CC-HK-013"
description: "agnix rule CC-HK-013 checks for async on non-command hook in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-013", "async on non-command hook", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-013`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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
          { "type": "prompt", "prompt": "Summarize", "async": true, "timeout": 30 }
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
