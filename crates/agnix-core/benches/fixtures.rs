//! Shared test data generators for benchmarks.
//!
//! This module provides functions to create realistic test projects
//! for both iai-callgrind (deterministic) and Criterion (wall-clock) benchmarks.
//!
//! The generated data mimics real-world usage patterns:
//! - SKILL.md files with varied frontmatter complexity
//! - hooks (settings.json) with multiple hook types
//! - MCP tool definitions with input schemas
//! - CLAUDE.md memory files with @imports

// Allow dead code since different benchmark targets use different functions
#![allow(dead_code)]

use std::fs;
use tempfile::TempDir;

/// Create a single realistic SKILL.md file for benchmarking.
///
/// Based on real-world patterns from tests/fixtures/valid/skills/.
pub fn create_single_skill_file() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp directory");

    let content = r#"---
name: code-review
description: Use when user asks to review code for quality, security, and performance issues.
version: 1.0.0
model: sonnet
triggers:
  - pattern: "review this code"
  - pattern: "check this PR"
  - pattern: "code review"
tags:
  - quality
  - security
  - review
---

# Code Review Skill

This skill provides comprehensive code review capabilities.

## What It Reviews

1. **Code Quality**
   - Naming conventions
   - Function length and complexity
   - Code duplication
   - Error handling patterns

2. **Security**
   - Input validation
   - SQL injection vulnerabilities
   - XSS prevention
   - Authentication/authorization checks

3. **Performance**
   - Algorithmic complexity
   - Memory usage patterns
   - Database query efficiency
   - Caching opportunities

## Usage

Invoke this skill when you need a thorough code review. The skill will:

1. Analyze the code structure
2. Identify potential issues
3. Provide specific, actionable feedback
4. Suggest improvements with code examples

## Best Practices

- Focus on the most impactful issues first
- Provide context for why something is problematic
- Include code snippets showing the fix
- Be constructive and educational
"#;

    fs::write(temp.path().join("SKILL.md"), content).expect("Failed to write SKILL.md");

    temp
}

/// Create a realistic SKILL.md with varied content.
///
/// The `index` parameter creates variation for scale tests.
pub fn create_realistic_skill(index: usize) -> String {
    let names = [
        "code-review",
        "test-runner",
        "deploy-prod",
        "refactor",
        "debug-issue",
        "write-docs",
        "optimize-perf",
        "security-audit",
        "api-design",
        "database-migration",
    ];
    let name = names[index % names.len()];

    let models = ["sonnet", "opus", "haiku"];
    let model = models[index % models.len()];

    format!(
        r#"---
name: {name}-{index}
description: Skill {index} for {name} operations with detailed instructions.
version: 1.{}.0
model: {model}
triggers:
  - pattern: "run {name}"
  - pattern: "{name} this"
tags:
  - automation
  - {name}
---

# {name} Skill (Variant {index})

This skill handles {name} operations efficiently.

## Capabilities

- Feature A for {name}
- Feature B with extended support
- Feature C for edge cases

## Instructions

When invoked, this skill will:

1. Analyze the current context
2. Apply {name} patterns
3. Validate the results
4. Report findings

## Notes

This is skill variant {index} with unique configuration.
"#,
        index % 10
    )
}

/// Create a scale project with the specified number of files.
///
/// Distribution (mimics real-world usage):
/// - 70% SKILL.md files (in skills/ subdirectories)
/// - 15% hooks (settings.json)
/// - 10% MCP tools (*.mcp.json)
/// - 5% misc (CLAUDE.md, agents, etc.)
pub fn create_scale_project(file_count: usize) -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp directory");

    let skill_count = file_count * 70 / 100;
    let hooks_count = file_count * 15 / 100;
    let mcp_count = file_count * 10 / 100;
    let misc_count = file_count - skill_count - hooks_count - mcp_count;

    // Create skills (70%)
    for i in 0..skill_count {
        let skill_dir = temp.path().join("skills").join(format!("skill-{}", i));
        fs::create_dir_all(&skill_dir).expect("Failed to create skill directory");
        let content = create_realistic_skill(i);
        fs::write(skill_dir.join("SKILL.md"), content).expect("Failed to write SKILL.md");
    }

    // Create hooks (15%)
    let hooks_dir = temp.path().join(".claude");
    fs::create_dir_all(&hooks_dir).expect("Failed to create .claude directory");

    for i in 0..hooks_count {
        let content = create_hooks_config(i);
        // Use settings.json and settings.local.json (recognized by detect_file_type)
        // For additional files beyond those two, create subdirectories
        let filepath = if i == 0 {
            hooks_dir.join("settings.json")
        } else if i == 1 {
            hooks_dir.join("settings.local.json")
        } else {
            // Create subdirectory for additional hooks files
            let subdir = hooks_dir.join(format!("hooks-{}", i));
            fs::create_dir_all(&subdir).expect("Failed to create hooks subdir");
            subdir.join("settings.json")
        };
        fs::write(filepath, content).expect("Failed to write hooks");
    }

    // Create MCP tools (10%)
    let mcp_dir = temp.path().join("mcp");
    fs::create_dir_all(&mcp_dir).expect("Failed to create mcp directory");

    for i in 0..mcp_count {
        let content = create_mcp_tool(i);
        fs::write(mcp_dir.join(format!("tool-{}.mcp.json", i)), content)
            .expect("Failed to write MCP tool");
    }

    // Create misc files (5%)
    // CLAUDE.md
    let claude_content = create_claude_md(skill_count);
    fs::write(temp.path().join("CLAUDE.md"), claude_content).expect("Failed to write CLAUDE.md");

    // Agents
    let agents_dir = temp.path().join("agents");
    fs::create_dir_all(&agents_dir).expect("Failed to create agents directory");

    for i in 0..misc_count.saturating_sub(1) {
        let content = create_agent(i);
        fs::write(agents_dir.join(format!("agent-{}.md", i)), content)
            .expect("Failed to write agent");
    }

    temp
}

/// Create a project optimized for memory tracking benchmarks.
///
/// Features deep import chains to stress the ImportCache.
pub fn create_memory_test_project() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp directory");

    // Create shared documentation files (imported by many)
    let shared_dir = temp.path().join("docs");
    fs::create_dir_all(&shared_dir).expect("Failed to create docs directory");

    for i in 0..10 {
        let content = format!(
            "# Shared Documentation {}\n\nThis is shared content that gets imported by multiple files.\n\n{}",
            i,
            "Lorem ipsum dolor sit amet. ".repeat(50)
        );
        fs::write(shared_dir.join(format!("shared-{}.md", i)), content)
            .expect("Failed to write shared doc");
    }

    // Create skills that import shared docs
    for i in 0..50 {
        let skill_dir = temp.path().join("skills").join(format!("skill-{}", i));
        fs::create_dir_all(&skill_dir).expect("Failed to create skill directory");

        // Each skill imports multiple shared docs
        let imports: Vec<String> = (0..5)
            .map(|j| format!("@../docs/shared-{}.md", (i + j) % 10))
            .collect();

        let content = format!(
            r#"---
name: skill-{}
description: Skill with imports for memory testing.
---

# Skill {}

References: {}

## Content

This skill tests memory usage with import chains.
"#,
            i,
            i,
            imports.join(", ")
        );
        fs::write(skill_dir.join("SKILL.md"), content).expect("Failed to write SKILL.md");
    }

    // Create CLAUDE.md that references all skills
    let skill_refs: Vec<String> = (0..50)
        .map(|i| format!("- @skills/skill-{}/SKILL.md", i))
        .collect();

    let claude_content = format!(
        r#"# Project Memory

## Skills

{}

## Guidelines

- Use imported documentation
- Follow consistent patterns
"#,
        skill_refs.join("\n")
    );
    fs::write(temp.path().join("CLAUDE.md"), claude_content).expect("Failed to write CLAUDE.md");

    temp
}

/// Create a hooks configuration file.
fn create_hooks_config(index: usize) -> String {
    let events = [
        "PreToolUse",
        "PostToolUse",
        "SessionStart",
        "SessionEnd",
        "Stop",
    ];
    let event = events[index % events.len()];

    format!(
        r#"{{
  "hooks": {{
    "{event}": [
      {{
        "matcher": "Bash",
        "hooks": [
          {{ "type": "command", "command": "echo 'hook {index}'", "timeout": 30 }}
        ]
      }},
      {{
        "matcher": "Write",
        "hooks": [
          {{ "type": "command", "command": "echo 'write hook {index}'", "timeout": 30 }}
        ]
      }}
    ]
  }}
}}"#
    )
}

/// Create an MCP tool definition.
fn create_mcp_tool(index: usize) -> String {
    let tools = [
        ("file-reader", "Reads file contents from the filesystem"),
        ("file-writer", "Writes content to files"),
        ("search-tool", "Searches for patterns in files"),
        ("git-status", "Gets git repository status"),
        ("npm-runner", "Runs npm commands"),
    ];
    let (name, desc) = tools[index % tools.len()];

    format!(
        r#"{{
  "name": "{name}-{index}",
  "description": "{desc} (variant {index})",
  "inputSchema": {{
    "type": "object",
    "properties": {{
      "path": {{
        "type": "string",
        "description": "The target path"
      }},
      "options": {{
        "type": "object",
        "description": "Additional options"
      }}
    }},
    "required": ["path"]
  }},
  "requiresApproval": {approval}
}}"#,
        approval = index % 2 == 0
    )
}

/// Create a CLAUDE.md memory file.
fn create_claude_md(skill_count: usize) -> String {
    let skill_refs: Vec<String> = (0..skill_count.min(20))
        .map(|i| format!("- skills/skill-{}/SKILL.md", i))
        .collect();

    format!(
        r#"# Project Memory

## Overview

This is a test project for benchmark validation.

## Architecture

- Rust workspace with multiple crates
- Core validation engine
- CLI interface

## Skills

{}

## Commands

```bash
cargo test
cargo build --release
```

## Guidelines

- Follow Rust idioms
- Write comprehensive tests
- Document public APIs
"#,
        skill_refs.join("\n")
    )
}

/// Create an agent definition file.
fn create_agent(index: usize) -> String {
    let roles = ["reviewer", "tester", "deployer", "documenter", "optimizer"];
    let role = roles[index % roles.len()];

    format!(
        r#"---
name: {role}-agent-{index}
description: Agent specialized in {role} tasks.
model: sonnet
permissions: default
---

# {role} Agent (Variant {index})

This agent handles {role} operations.

## Capabilities

- Primary {role} functionality
- Secondary support features
- Error handling and recovery

## Usage

Invoke this agent for {role}-related tasks.
"#
    )
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::{create_memory_test_project, create_scale_project, create_single_skill_file};
    use std::fs;

    #[test]
    fn test_create_single_skill_file() {
        let temp = create_single_skill_file();
        let skill_path = temp.path().join("SKILL.md");
        assert!(skill_path.exists());
        let content = fs::read_to_string(&skill_path).unwrap();
        assert!(content.contains("name: code-review"));
    }

    #[test]
    fn test_create_scale_project_100() {
        let temp = create_scale_project(100);
        let skills_dir = temp.path().join("skills");
        assert!(skills_dir.exists());
        // Should have ~70 skill directories
        let skill_count = fs::read_dir(&skills_dir).unwrap().count();
        assert!((60..=80).contains(&skill_count));
    }

    #[test]
    fn test_create_memory_test_project() {
        let temp = create_memory_test_project();
        let claude_path = temp.path().join("CLAUDE.md");
        assert!(claude_path.exists());
        let content = fs::read_to_string(&claude_path).unwrap();
        // Should reference skills
        assert!(content.contains("@skills/skill-"));
    }
}
