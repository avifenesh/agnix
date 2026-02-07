---
id: oc-sk-001
title: "OC-SK-001: OpenCode Skill Uses Unsupported Field"
sidebar_label: "OC-SK-001"
description: "agnix rule OC-SK-001 checks for opencode skill uses unsupported field in opencode-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-SK-001", "opencode skill uses unsupported field", "opencode-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `opencode-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/rules

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```text
Configuration omitted required fields for this rule.```

### Valid

```text
Configuration includes required fields and follows the rule.```
