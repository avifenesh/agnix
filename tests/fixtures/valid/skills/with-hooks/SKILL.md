---
name: hooks-skill
description: Use when testing valid hooks
hooks:
  PreToolUse:
    - type: command
      command: echo pre-tool
---
Run the pre-tool hook before tool use.
