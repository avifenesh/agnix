# Project

This project is a linter for agent configurations.

## Overview

The linter validates SKILL.md, CLAUDE.md, AGENTS.md and other configuration files.

## Build Commands

Run the following commands:

```bash
cargo build
cargo test
```

## Testing

Use `cargo test` to run all tests.

## Claude Code Specific

- type: PreToolExecution
  command: echo "Running pre-tool check"
