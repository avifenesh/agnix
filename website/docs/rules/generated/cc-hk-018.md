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
- **Auto-Fix**: `No`
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

The following examples are illustrative snippets for this rule category.

### Invalid

```json
{
  "hooks": [
    {
      "event": "PreToolUse",
      "matcher": "*"
    }
  ]
}
```

### Valid

```json
{
  "hooks": [
    {
      "event": "PreToolUse",
      "matcher": "Write",
      "command": "./scripts/validate.sh",
      "timeout": 30
    }
  ]
}
```
