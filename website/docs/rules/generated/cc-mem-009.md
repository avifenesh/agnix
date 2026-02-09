---
id: cc-mem-009
title: "CC-MEM-009: Token Count Exceeded - Claude Memory"
sidebar_label: "CC-MEM-009"
description: "agnix rule CC-MEM-009 checks for token count exceeded in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-009", "token count exceeded", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-009`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-02-04`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Project Rules

[...6000+ characters of instructions that exceed the ~1500 token limit for CLAUDE.md files, causing context window bloat and reduced instruction adherence...]
```

### Valid

```markdown
# Project Rules

Use TypeScript strict mode.
Run tests before committing.
Follow the style guide.
```
