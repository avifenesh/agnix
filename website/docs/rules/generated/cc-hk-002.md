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
