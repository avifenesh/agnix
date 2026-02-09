---
id: cc-hk-005
title: "CC-HK-005: Missing Type Field - Claude Hooks"
sidebar_label: "CC-HK-005"
description: "agnix rule CC-HK-005 checks for missing type field in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-005", "missing type field", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-005`
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
    "Stop": [
      {
        "hooks": [
          { "command": "echo missing type field" }
        ]
      }
    ]
  }
}```

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
}```
