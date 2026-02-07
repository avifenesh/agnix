---
name: hooks-agent
description: Agent with invalid hooks configuration
hooks:
  BadEvent:
    - matcher: "*"
      hooks:
        - type: command
          command: echo test
---
This agent has an invalid hook event name.
