---
id: cc-sk-009
title: "CC-SK-009: Too Many Injections - Claude Skills"
sidebar_label: "CC-SK-009"
description: "agnix rule CC-SK-009 checks for too many injections in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-009", "too many injections", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-009`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://platform.claude.com/docs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: template-skill
description: Use when applying a code template
---
Use !`config.json` and !`env.json` and !`secrets.json` and !`overrides.json` for setup.```

### Valid

```markdown
---
name: template-skill
description: Use when applying a code template
---
Apply the template with !`config.json` values.```
