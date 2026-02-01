# Agent Configuration

This AGENTS.md file contains Claude-specific hooks that will not work
on other platforms like Codex CLI, OpenCode, or Cursor.

## Hooks Configuration

- type: PreToolExecution
  command: echo "running pre-tool check"

- type: PostToolExecution
  command: echo "tool completed"

- event: Stop
  message: Task complete

## Guidelines

Follow the project conventions.
