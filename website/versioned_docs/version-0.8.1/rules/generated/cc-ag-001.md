---
id: cc-ag-001
title: "CC-AG-001 Missing Name Field"
sidebar_label: "CC-AG-001"
---

## Summary

- **Rule ID**: `CC-AG-001`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: reviewer
---
```

### Valid

```markdown
---
name: reviewer
description: Review code for correctness and tests
model: sonnet
tools: [Read, Grep, Bash]
---
```
