---
id: cc-hk-014
title: "CC-HK-014: Once Outside Skill/Agent Frontmatter"
sidebar_label: "CC-HK-014"
description: "agnix rule CC-HK-014 checks for once outside skill/agent frontmatter in claude hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-HK-014", "once outside skill/agent frontmatter", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Hooks`
- **Normative Level**: `SHOULD`
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
