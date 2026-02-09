---
id: cc-sk-014
title: "CC-SK-014: Invalid disable-model-invocation Type"
sidebar_label: "CC-SK-014"
description: "agnix rule CC-SK-014 checks for invalid disable-model-invocation type in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-014", "invalid disable-model-invocation type", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-014`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: quiet-skill
description: Use when running without model invocation
disable-model-invocation: "true"
---
Run silently.```

### Valid

```markdown
---
name: quiet-skill
description: Use when running without model invocation
disable-model-invocation: true
---
Run silently.```
