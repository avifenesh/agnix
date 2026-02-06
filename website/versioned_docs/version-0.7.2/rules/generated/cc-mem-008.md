---
id: cc-mem-008
title: "CC-MEM-008 Critical Content in Middle"
sidebar_label: "CC-MEM-008"
---

## Summary

- **Rule ID**: `CC-MEM-008`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://aclanthology.org/2024.tacl-1.9/

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
