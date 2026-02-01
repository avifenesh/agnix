---
name: specialized-agent
description: An agent with Claude-specific forking
context: fork
agent: Explore
allowed-tools: Read Write Grep Glob
---

# Specialized Agent

This agent uses Claude Code's context forking feature which is not
supported by other AGENTS.md readers like Codex CLI or OpenCode.

## Usage

This agent will be invoked in a separate context thread.
