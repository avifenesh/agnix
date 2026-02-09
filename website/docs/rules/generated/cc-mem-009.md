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
- Follow coding standard rule 1: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 2: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 3: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 4: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 5: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 6: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 7: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 8: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 9: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 10: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 11: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 12: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 13: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 14: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 15: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 16: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 17: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 18: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 19: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 20: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 21: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 22: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 23: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 24: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 25: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 26: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 27: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 28: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 29: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 30: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 31: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 32: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 33: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 34: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 35: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 36: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 37: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 38: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 39: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 40: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 41: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 42: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 43: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 44: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 45: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 46: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 47: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 48: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 49: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 50: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 51: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 52: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 53: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 54: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 55: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 56: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 57: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 58: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 59: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 60: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 61: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 62: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 63: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 64: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 65: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 66: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 67: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 68: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 69: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 70: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 71: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 72: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 73: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 74: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 75: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 76: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 77: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 78: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 79: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 80: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 81: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 82: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 83: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 84: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 85: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 86: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 87: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 88: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 89: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 90: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 91: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 92: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 93: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 94: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 95: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 96: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 97: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 98: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 99: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 100: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 101: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 102: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 103: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 104: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 105: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 106: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 107: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 108: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 109: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 110: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 111: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 112: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 113: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 114: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 115: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 116: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 117: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 118: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 119: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 120: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 121: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 122: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 123: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 124: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 125: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 126: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 127: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 128: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 129: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 130: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 131: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 132: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 133: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 134: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 135: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 136: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 137: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 138: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 139: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 140: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 141: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 142: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 143: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 144: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 145: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 146: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 147: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 148: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 149: use consistent naming conventions and document all public APIs.
- Follow coding standard rule 150: use consistent naming conventions and document all public APIs.
```

### Valid

```markdown
# Project Rules

Use TypeScript strict mode.
Run tests before committing.
Follow the style guide.
```
