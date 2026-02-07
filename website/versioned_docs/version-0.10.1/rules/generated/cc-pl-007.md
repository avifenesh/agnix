---
id: cc-pl-007
title: "CC-PL-007: Invalid Component Path - Claude Plugins"
sidebar_label: "CC-PL-007"
description: "agnix rule CC-PL-007 checks for invalid component path in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-007", "invalid component path", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-007`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```json
{
  "name": "plugin"
}
```

### Valid

```json
{
  "name": "agnix-plugin",
  "commands": [
    {"name": "validate", "entrypoint": "./scripts/validate.sh"}
  ]
}
```
