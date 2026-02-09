---
id: cc-pl-002
title: "CC-PL-002: Components in .claude-plugin/ - Claude Plugins"
sidebar_label: "CC-PL-002"
description: "agnix rule CC-PL-002 checks for components in .claude-plugin/ in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-002", "components in .claude-plugin/", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-002`
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
  "skills": "./.claude-plugin/skills"
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
