---
id: cc-ag-001
title: "CC-AG-001: Missing Name Field - Claude Agents"
sidebar_label: "CC-AG-001"
description: "agnix rule CC-AG-001 checks for missing name field in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-001", "missing name field", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-001`
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
description: Reviews pull requests for quality
---
Review code changes and provide feedback.```

### Valid

```markdown
---
name: code-reviewer
description: Reviews pull requests for quality
---
Review code changes and provide feedback.```
