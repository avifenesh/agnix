---
id: cc-mem-007
title: "CC-MEM-007: Weak Constraint Language - Claude Memory"
sidebar_label: "CC-MEM-007"
description: "agnix rule CC-MEM-007 checks for weak constraint language in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-007", "weak constraint language", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-007`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://arxiv.org/abs/2201.11903

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
# Memory
Always be helpful.
```

### Valid

```markdown
# Project Memory
- Use Rust workspace conventions
- Keep AGENTS.md and CLAUDE.md identical
```
