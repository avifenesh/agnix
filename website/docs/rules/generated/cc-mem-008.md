---
id: cc-mem-008
title: "CC-MEM-008: Critical Content in Middle - Claude Memory"
sidebar_label: "CC-MEM-008"
description: "agnix rule CC-MEM-008 checks for critical content in middle in claude memory files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-MEM-008", "critical content in middle", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-008`
- **Severity**: `HIGH`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
## Setup

Install dependencies with npm install.

## Architecture

The project uses a monorepo structure.

## Testing

Run unit tests.

## Deployment

Deploy to staging.

## Monitoring

Check dashboards.

IMPORTANT: Always run tests before committing.

## Cleanup

Remove temp files.```

### Valid

```markdown
# Critical Rules

IMPORTANT: Always run tests before committing.

## Setup

Install dependencies with npm install.

## Architecture

The project uses a monorepo structure.```
