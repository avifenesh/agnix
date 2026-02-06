---
id: cc-pl-002
title: "CC-PL-002 Components in .claude-plugin/"
sidebar_label: "CC-PL-002"
---

## Summary

- **Rule ID**: `CC-PL-002`
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
