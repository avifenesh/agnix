---
id: cc-ag-013
title: "CC-AG-013: Invalid Skill Name Format - Claude Agents"
sidebar_label: "CC-AG-013"
description: "agnix rule CC-AG-013 checks for invalid skill name format in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-013", "invalid skill name format", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-013`
- **Severity**: `MEDIUM`
- **Category**: `Claude Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-07`

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
