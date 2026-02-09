---
id: cc-mem-005
title: "CC-MEM-005: Generic Instruction - Claude Memory"
sidebar_label: "CC-MEM-005"
description: "agnix rule CC-MEM-005 checks for generic instruction in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-005", "generic instruction", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-005`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/awesome-slash
- https://arxiv.org/abs/2201.11903

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
Be helpful and accurate when responding.
Always provide the best possible answer.
```

### Valid

```markdown
# Project Rules

Always use TypeScript strict mode.
Run `cargo test` before committing.
```
