---
id: cc-pl-004
title: "CC-PL-004: Missing Required Plugin Field - Claude Plugins"
sidebar_label: "CC-PL-004"
description: "agnix rule CC-PL-004 checks for missing required plugin field in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-004", "missing required plugin field", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-004`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Verified On**: `2026-02-04`

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
