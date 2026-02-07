---
id: cl-sk-001
title: "CL-SK-001: Cline Skill Uses Unsupported Field - cline-skills"
sidebar_label: "CL-SK-001"
description: "agnix rule CL-SK-001 checks for cline skill uses unsupported field in cline-skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CL-SK-001", "cline skill uses unsupported field", "cline-skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CL-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `cline-skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `cline`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.cline.bot/features/custom-instructions

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
