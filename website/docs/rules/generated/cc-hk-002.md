---
id: cc-hk-002
title: "CC-HK-002: Prompt Hook on Wrong Event - Claude Hooks"
sidebar_label: "CC-HK-002"
description: "agnix rule CC-HK-002 checks for prompt hook on wrong event in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-002", "prompt hook on wrong event", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-002`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
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
        "matcher": "Bash",
        "hooks": [
          { "type": "prompt", "prompt": "Check this tool call", "timeout": 30 }
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
          { "type": "prompt", "prompt": "Summarize the session", "timeout": 30 }
        ]
      }
    ]
  }
}
```
