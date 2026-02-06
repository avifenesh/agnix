---
id: cc-hk-001
title: "CC-HK-001 Invalid Hook Event"
sidebar_label: "CC-HK-001"
---

## Summary

- **Rule ID**: `CC-HK-001`
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
