---
id: ws-sk-001
title: "WS-SK-001: Windsurf Skill Uses Unsupported Field"
sidebar_label: "WS-SK-001"
description: "agnix rule WS-SK-001 checks for windsurf skill uses unsupported field in windsurf-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["WS-SK-001", "windsurf skill uses unsupported field", "windsurf-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `windsurf-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-09`

## Applicability

- **Tool**: `windsurf`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.windsurf.com/windsurf/cascade/memories

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
