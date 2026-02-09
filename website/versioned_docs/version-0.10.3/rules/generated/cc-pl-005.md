---
id: cc-pl-005
title: "CC-PL-005: Empty Plugin Name - Claude Plugins"
sidebar_label: "CC-PL-005"
description: "agnix rule CC-PL-005 checks for empty plugin name in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-005", "empty plugin name", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-005`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "  ",
  "description": "A useful plugin",
  "version": "1.0.0"
}
```

### Valid

```json
{
  "name": "my-plugin",
  "description": "A useful plugin",
  "version": "1.0.0"
}
```
