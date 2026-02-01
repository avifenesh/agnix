# Platform Configuration Guide

This file demonstrates hard-coded platform paths that reduce portability.

## Claude Code

Configuration is stored at `.claude/settings.json`.
Skills are located in `.claude/skills/` directory.

## Cursor

Rules are defined in `.cursor/rules/` directory.

## OpenCode

Settings file: `.opencode/config.yaml`

## Cline

Configuration at `.cline/settings.json`

## Better Approach

Instead of hard-coding platform paths, use environment variables
or let each platform discover its own configuration location.
