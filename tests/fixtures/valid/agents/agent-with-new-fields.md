---
name: full-featured-agent
description: An agent with all new fields configured correctly
model: opus
permissionMode: dontAsk
memory: project
tools:
  - Bash
  - Read
  - Write
  - WebSearch
  - NotebookEdit
disallowedTools:
  - Edit
hooks:
  PreToolUse:
    - matcher: "*"
      hooks:
        - type: command
          command: echo pre-tool
  Stop:
    - hooks:
        - type: prompt
          prompt: Summarize what you did
---
This agent uses all new fields with valid values.
