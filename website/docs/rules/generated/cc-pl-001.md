---
id: cc-pl-001
title: "CC-PL-001: Plugin Manifest Not in .claude-plugin/"
sidebar_label: "CC-PL-001"
description: "agnix rule CC-PL-001 checks for plugin manifest not in .claude-plugin/ in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-001", "plugin manifest not in .claude-plugin/", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-001`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "my-plugin",
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
