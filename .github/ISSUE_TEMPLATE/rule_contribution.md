---
name: Rule Contribution
about: Propose a new validation rule
title: "[RULE] "
labels: rule-proposal
---

## Rule Summary

Brief description of what the rule validates and why it matters.

## Tool/Platform

Which AI coding tool(s) does this rule apply to? (e.g., Claude Code, Cursor, Copilot, Codex CLI, MCP, cross-platform)

## Evidence

Link to official documentation, specification, or research that supports this rule.

- Documentation URL:
- Specification section (if applicable):
- Research paper (if applicable):

See `knowledge-base/VALIDATION-RULES.md` for the evidence schema reference (`source_type`, `source_urls`, `verified_on`, `applies_to`, `normative_level`, `tests`).

## Example Config

Show a config snippet that demonstrates the problematic pattern this rule would catch:

```
(paste config here -- remove any API keys, tokens, passwords, or other secrets first)
```

## Expected Behavior

What should agnix report when it encounters this pattern? Include:
- Rule ID suggestion (e.g., `CC-SK-010`, `CUR-007` -- check `knowledge-base/rules.json` for the next available number in the prefix)
- Diagnostic message
- Suggested fix (if applicable)

## Severity

- [ ] HIGH (>95% true positive, spec violation or breaking error)
- [ ] MEDIUM (75-95% true positive, best practice violation)
- [ ] LOW (<75% true positive, style suggestion)

## Auto-Fix Possible

- [ ] Yes (describe the automated fix)
- [ ] No (explain why manual intervention is needed)
