---
id: cc-sk-013
title: "CC-SK-013: Fork Context Without Actionable Instructions - Claude Skills"
sidebar_label: "CC-SK-013"
description: "agnix rule CC-SK-013 checks for fork context without actionable instructions in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-013", "fork context without actionable instructions", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-013`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-07`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples are illustrative snippets for this rule category.

### Invalid

```markdown
---
name: ref-skill
description: Use when looking up docs
context: fork
agent: general-purpose
---
This is a reference document about the API.
It describes the system architecture.
```

### Valid

```markdown
---
name: build-skill
description: Use when building the project
context: fork
agent: general-purpose
---
Run the build command and check for errors.
Create a report of the results.
```
