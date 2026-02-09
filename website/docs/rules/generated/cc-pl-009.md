---
id: cc-pl-009
title: "CC-PL-009: Invalid Author Object - Claude Plugins"
sidebar_label: "CC-PL-009"
description: "agnix rule CC-PL-009 checks for invalid author object in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-009", "invalid author object", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-009`
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
  "author": { "email": "jane@example.com" }
}
```

### Valid

```json
{
  "name": "my-plugin",
  "description": "A useful plugin",
  "version": "1.0.0",
  "author": { "name": "Jane Doe" }
}
```
