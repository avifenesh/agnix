---
id: cc-sk-017
title: "CC-SK-017: Unknown Frontmatter Field - Claude Skills"
sidebar_label: "CC-SK-017"
description: "agnix rule CC-SK-017 checks for unknown frontmatter field in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-017", "unknown frontmatter field", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-017`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-14`

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
name: lint-config
description: Use when validating configuration files
allowed_tools: Read, Grep
---
Lint project configuration files.
```

### Valid

```markdown
---
name: lint-config
description: Use when validating configuration files
allowed-tools: Read, Grep
---
Lint project configuration files.
```
