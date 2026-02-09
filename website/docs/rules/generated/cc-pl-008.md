---
id: cc-pl-008
title: "CC-PL-008: Component Inside .claude-plugin - Claude Plugins"
sidebar_label: "CC-PL-008"
description: "agnix rule CC-PL-008 checks for component inside .claude-plugin in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-008", "component inside .claude-plugin", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-008`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
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
  "agents": ".claude-plugin/agents"
}
```

### Valid

```json
{
  "name": "my-plugin",
  "description": "A useful plugin",
  "version": "1.0.0",
  "agents": "./agents/"
}
```
