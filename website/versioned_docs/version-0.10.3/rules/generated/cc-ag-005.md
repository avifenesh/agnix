---
id: cc-ag-005
title: "CC-AG-005: Referenced Skill Not Found - Claude Agents"
sidebar_label: "CC-AG-005"
description: "agnix rule CC-AG-005 checks for referenced skill not found in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-005", "referenced skill not found", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-005`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: code-reviewer
description: Reviews code
skills:
  - nonexistent-skill
---
Review code changes.
```

### Valid

```markdown
---
name: code-reviewer
description: Reviews code
skills:
  - code-review
---
Review code changes.
```
