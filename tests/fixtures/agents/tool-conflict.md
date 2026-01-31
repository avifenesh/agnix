---
name: tool-conflict-agent
description: An agent with conflicting tool configurations
tools:
  - Bash
  - Read
  - Write
disallowedTools:
  - Bash
  - Edit
---
This agent has Bash in both tools and disallowedTools, which is a conflict.
