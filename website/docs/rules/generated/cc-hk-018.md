---
id: cc-hk-018
title: "CC-HK-018: Matcher on UserPromptSubmit/Stop - Claude Hooks"
sidebar_label: "CC-HK-018"
description: "agnix rule CC-HK-018 checks for matcher on userpromptsubmit/stop in claude hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-HK-018", "matcher on userpromptsubmit/stop", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-018`
- **Severity**: `LOW`
- **Category**: `Claude Hooks`
- **Normative Level**: `BEST_PRACTICE`
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
    "UserPromptSubmit": [
      {
        "matcher": "Bash",
        "hooks": [
          { "type": "command", "command": "echo submit", "timeout": 30 }
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
    "UserPromptSubmit": [
      {
        "hooks": [
          { "type": "command", "command": "echo submit", "timeout": 30 }
        ]
      }
    ]
  }
}
```
