---
name: invalid-agent-skill
description: Use when testing invalid agent validation
context: fork
agent: Invalid_Agent
---

This skill has an invalid agent type.
Agent must be a built-in (Explore, Plan, general-purpose) or a custom kebab-case name.
Invalid_Agent is rejected because underscores are not allowed; use hyphens for kebab-case format.
