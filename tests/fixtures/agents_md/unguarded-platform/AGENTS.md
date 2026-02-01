# Project

This project validates configurations.

## Hooks Configuration

- type: PreToolExecution
  command: echo "test"

context: fork

agent: security-reviewer

allowed-tools: Read Write Bash
