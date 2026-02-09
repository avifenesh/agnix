---
id: cc-pl-010
title: "CC-PL-010: Invalid Homepage URL - Claude Plugins"
sidebar_label: "CC-PL-010"
description: "agnix rule CC-PL-010 checks for invalid homepage url in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-010", "invalid homepage url", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-010`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "my-plugin",
  "description": "A useful plugin",
  "version": "1.0.0",
  "homepage": "not-a-valid-url"
}
```

### Valid

```json
{
  "name": "my-plugin",
  "description": "A useful plugin",
  "version": "1.0.0",
  "homepage": "https://example.com/my-plugin"
}
```
