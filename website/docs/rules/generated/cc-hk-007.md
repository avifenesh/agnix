---
id: cc-hk-007
title: "CC-HK-007: Missing Prompt Field - Claude Hooks"
sidebar_label: "CC-HK-007"
description: "agnix rule CC-HK-007 checks for missing prompt field in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-007", "missing prompt field", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-007`
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
          { "type": "prompt", "timeout": 30 }
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
          { "type": "prompt", "prompt": "Summarize what was done", "timeout": 30 }
        ]
      }
    ]
  }
}
```
