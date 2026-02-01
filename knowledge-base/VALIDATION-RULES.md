# agnix Validation Rules - Master Reference

> Consolidated from 320KB knowledge base, 75+ sources, 5 research agents

**Last Updated**: 2026-01-31
**Coverage**: Agent Skills • MCP • Claude Code • Multi-Platform • Prompt Engineering

---

## Rule Format

```
[RULE-ID] [CERTAINTY] Rule description
  ├─ Detection: How to detect
  ├─ Fix: Auto-fix if available
  └─ Source: Citation
```

**Certainty Levels**:
- **HIGH**: >95% true positive, always report, auto-fix safe
- **MEDIUM**: 75-95% true positive, report in default mode
- **LOW**: <75% true positive, verbose mode only

---

## AGENT SKILLS RULES

### AS-001 [HIGH] Missing Frontmatter
**Requirement**: SKILL.md MUST have YAML frontmatter between `---` delimiters
**Detection**: `!content.starts_with("---")` or no closing `---`
**Fix**: Add template frontmatter
**Source**: agentskills.io/specification

### AS-002 [HIGH] Missing Required Field: name
**Requirement**: `name` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `name` key
**Fix**: Add `name: directory-name`
**Source**: agentskills.io/specification

### AS-003 [HIGH] Missing Required Field: description
**Requirement**: `description` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `description` key
**Fix**: Add `description: "Use when..."`
**Source**: agentskills.io/specification

### AS-004 [HIGH] Invalid Name Format
**Requirement**: name MUST be 1-64 chars, lowercase letters/numbers/hyphens only
**Regex**: `^[a-z0-9]+(-[a-z0-9]+)*$`
**Detection**:
```rust
!Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").matches(name) || name.len() > 64
```
**Fix**: Lowercase + replace `_` with `-` + remove invalid chars
**Source**: agentskills.io/specification

### AS-005 [HIGH] Name Starts/Ends with Hyphen
**Requirement**: name MUST NOT start or end with `-`
**Detection**: `name.starts_with('-') || name.ends_with('-')`
**Fix**: Remove leading/trailing hyphens
**Source**: agentskills.io/specification

### AS-006 [HIGH] Consecutive Hyphens in Name
**Requirement**: name MUST NOT contain `--`
**Detection**: `name.contains("--")`
**Fix**: Replace `--` with `-`
**Source**: agentskills.io/specification

### AS-007 [HIGH] Reserved Name
**Requirement**: name MUST NOT be reserved word (anthropic, claude)
**Detection**: `["anthropic", "claude", "skill"].contains(name.as_str())`
**Fix**: Suggest alternative name
**Source**: platform.claude.com/docs

### AS-008 [HIGH] Description Too Short
**Requirement**: description MUST be 1-1024 characters
**Detection**: `description.len() < 1 || description.len() > 1024`
**Fix**: Add minimal description or truncate
**Source**: agentskills.io/specification

### AS-009 [HIGH] Description Contains XML
**Requirement**: description MUST NOT contain XML tags
**Detection**: `Regex::new(r"<[^>]+>").is_match(description)`
**Fix**: Remove XML tags
**Source**: platform.claude.com/docs

### AS-010 [MEDIUM] Missing Trigger Phrase
**Requirement**: description SHOULD include "Use when" trigger
**Detection**: `!description.to_lowercase().contains("use when")`
**Fix**: Prepend "Use when user asks to..."
**Source**: awesome-slash/enhance-skills, platform.claude.com/docs

### AS-011 [HIGH] Compatibility Too Long
**Requirement**: compatibility field MUST be 1-500 chars if present
**Detection**: `compatibility.len() > 500`
**Fix**: Truncate to 500 chars
**Source**: agentskills.io/specification

### AS-012 [MEDIUM] Content Exceeds 500 Lines
**Requirement**: SKILL.md SHOULD be under 500 lines
**Detection**: `body.lines().count() > 500`
**Fix**: Suggest moving to references/
**Source**: platform.claude.com/docs, agentskills.io

### AS-013 [HIGH] File Reference Too Deep
**Requirement**: File references MUST be one level deep
**Detection**: Check references like `references/guide.md` vs `refs/deep/nested/file.md`
**Fix**: Flatten directory structure
**Source**: agentskills.io/specification

### AS-014 [HIGH] Windows Path Separator
**Requirement**: Paths MUST use forward slashes, even on Windows
**Detection**: `path.contains("\\")`
**Fix**: Replace `\\` with `/`
**Source**: agentskills.io/specification

### AS-015 [HIGH] Upload Size Exceeds 8MB
**Requirement**: Skill directory MUST be under 8MB total
**Detection**: `directory_size > 8 * 1024 * 1024`
**Fix**: Remove large assets or split skill
**Source**: platform.claude.com/docs

---

## CLAUDE CODE RULES (SKILLS)

### CC-SK-001 [HIGH] Invalid Model Value
**Requirement**: model MUST be one of: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with closest valid option
**Source**: code.claude.com/docs/en/skills

### CC-SK-002 [HIGH] Invalid Context Value
**Requirement**: context MUST be "fork" or omitted
**Detection**: `context.is_some() && context != "fork"`
**Fix**: Change to "fork" or remove
**Source**: code.claude.com/docs/en/skills

### CC-SK-003 [HIGH] Context Without Agent
**Requirement**: `context: fork` REQUIRES `agent` field
**Detection**: `context == "fork" && agent.is_none()`
**Fix**: Add `agent: general-purpose`
**Source**: code.claude.com/docs/en/skills

### CC-SK-004 [HIGH] Agent Without Context
**Requirement**: `agent` field REQUIRES `context: fork`
**Detection**: `agent.is_some() && context != Some("fork")`
**Fix**: Add `context: fork`
**Source**: code.claude.com/docs/en/skills

### CC-SK-005 [HIGH] Invalid Agent Type
**Requirement**: agent MUST be: Explore, Plan, general-purpose, or custom agent name
**Detection**: Check against known agent types
**Fix**: Suggest valid agent
**Source**: code.claude.com/docs/en/sub-agents

### CC-SK-006 [HIGH] Dangerous Auto-Invocation
**Requirement**: Side-effect skills MUST have `disable-model-invocation: true`
**Detection**: `name.contains("deploy|ship|publish|delete|drop") && !disable_model_invocation`
**Fix**: Add `disable-model-invocation: true`
**Source**: code.claude.com/docs/en/skills

### CC-SK-007 [HIGH] Unrestricted Bash
**Requirement**: Bash in allowed-tools SHOULD be scoped
**Detection**: `allowed_tools.contains("Bash") && !allowed_tools.contains("Bash(")`
**Fix**: Suggest `Bash(git:*)` based on skill name
**Source**: awesome-slash/enhance-skills

### CC-SK-008 [HIGH] Unknown Tool Name
**Requirement**: Tool names MUST match Claude Code tools
**Known Tools**: Bash, Read, Write, Edit, Grep, Glob, Task, WebFetch, AskUserQuestion, etc.
**Detection**: Check against tool list
**Fix**: Suggest closest match
**Source**: code.claude.com/docs/en/settings

### CC-SK-009 [MEDIUM] Too Many Injections
**Requirement**: Limit dynamic injections (!`cmd`) to 3
**Detection**: `content.matches("!\`").count() > 3`
**Fix**: Remove or move to scripts/
**Source**: platform.claude.com/docs

---

## CLAUDE CODE RULES (HOOKS)

### CC-HK-001 [HIGH] Invalid Hook Event
**Requirement**: Event MUST be one of 12 valid names (case-sensitive)
**Valid**: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, SubagentStart, SubagentStop, Stop, PreCompact, Setup, SessionEnd, Notification
**Detection**: `!VALID_EVENTS.contains(event)`
**Fix**: Suggest closest match (did you mean PreToolUse?)
**Source**: code.claude.com/docs/en/hooks

### CC-HK-002 [HIGH] Prompt Hook on Wrong Event
**Requirement**: `type: "prompt"` ONLY for Stop and SubagentStop
**Detection**: `hook.type == "prompt" && !["Stop", "SubagentStop"].contains(event)`
**Fix**: Change to `type: "command"` or use Stop/SubagentStop
**Source**: code.claude.com/docs/en/hooks

### CC-HK-003 [HIGH] Missing Matcher for Tool Events
**Requirement**: PreToolUse/PermissionRequest/PostToolUse REQUIRE matcher
**Detection**: `["PreToolUse", "PermissionRequest", "PostToolUse"].contains(event) && matcher.is_none()`
**Fix**: Add `"matcher": "*"` or specific tool
**Source**: code.claude.com/docs/en/hooks

### CC-HK-004 [HIGH] Matcher on Non-Tool Event
**Requirement**: Stop/SubagentStop/UserPromptSubmit MUST NOT have matcher
**Detection**: `["Stop", "SubagentStop", "UserPromptSubmit"].contains(event) && matcher.is_some()`
**Fix**: Remove matcher field
**Source**: code.claude.com/docs/en/hooks

### CC-HK-005 [HIGH] Missing Type Field
**Requirement**: Hook MUST have `type: "command"` or `type: "prompt"`
**Detection**: `hook.type.is_none()`
**Fix**: Add `"type": "command"`
**Source**: code.claude.com/docs/en/hooks

### CC-HK-006 [HIGH] Missing Command Field
**Requirement**: `type: "command"` REQUIRES `command` field
**Detection**: `hook.type == "command" && hook.command.is_none()`
**Fix**: Add command field
**Source**: code.claude.com/docs/en/hooks

### CC-HK-007 [HIGH] Missing Prompt Field
**Requirement**: `type: "prompt"` REQUIRES `prompt` field
**Detection**: `hook.type == "prompt" && hook.prompt.is_none()`
**Fix**: Add prompt field
**Source**: code.claude.com/docs/en/hooks

### CC-HK-008 [HIGH] Script File Not Found
**Requirement**: Hook command script MUST exist on filesystem
**Detection**: Check if script path exists (resolve $CLAUDE_PROJECT_DIR)
**Fix**: Show error with correct path
**Source**: code.claude.com/docs/en/hooks

### CC-HK-009 [HIGH] Dangerous Command Pattern
**Requirement**: Hooks SHOULD NOT contain destructive commands
**Patterns**: `rm -rf`, `git reset --hard`, `drop database`, `curl.*|.*sh`
**Detection**: Regex match against dangerous patterns
**Fix**: Warn, suggest safer alternative
**Source**: awesome-slash/enhance-hooks

### CC-HK-010 [MEDIUM] No Timeout Specified
**Requirement**: Timeout is optional, but long-running hooks SHOULD set one
**Detection**: `hook.timeout.is_none()`
**Policy**: Soft warning if `hook.timeout > 60` (exceeds default limit; may be intentional)
**Fix**: Add `"timeout": 30` (or reduce to <= 60 if unintentionally long)
**Source**: docs.claude.com/en/docs/claude-code/hooks

### CC-HK-011 [HIGH] Invalid Timeout Value
**Requirement**: timeout MUST be positive integer
**Detection**: `timeout <= 0`
**Fix**: Set to 30
**Source**: code.claude.com/docs/en/hooks

---

## CLAUDE CODE RULES (SUBAGENTS)

### CC-AG-001 [HIGH] Missing Name Field
**Requirement**: Agent frontmatter REQUIRES `name` field
**Detection**: Parse frontmatter, check for `name`
**Fix**: Add `name: agent-name`
**Source**: code.claude.com/docs/en/sub-agents

### CC-AG-002 [HIGH] Missing Description Field
**Requirement**: Agent frontmatter REQUIRES `description` field
**Detection**: Parse frontmatter, check for `description`
**Fix**: Add description
**Source**: code.claude.com/docs/en/sub-agents

### CC-AG-003 [HIGH] Invalid Model Value
**Requirement**: model MUST be: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

### CC-AG-004 [HIGH] Invalid Permission Mode
**Requirement**: permissionMode MUST be: default, acceptEdits, dontAsk, bypassPermissions, plan
**Detection**: `!VALID_MODES.contains(permission_mode)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

### CC-AG-005 [HIGH] Referenced Skill Not Found
**Requirement**: Skills in `skills` array MUST exist
**Detection**: Check `.claude/skills/{name}/SKILL.md` exists
**Fix**: Remove reference or create skill
**Source**: code.claude.com/docs/en/sub-agents

### CC-AG-006 [HIGH] Tool/Disallowed Conflict
**Requirement**: Tool cannot be in both `tools` and `disallowedTools`
**Detection**: `tools.intersection(disallowedTools).is_empty()`
**Fix**: Remove from one list
**Source**: code.claude.com/docs/en/sub-agents

---

## CLAUDE CODE RULES (MEMORY)

### CC-MEM-001 [HIGH] Invalid Import Path
**Requirement**: @import paths MUST exist on filesystem
**Detection**: Extract `@path` references, check existence
**Fix**: Show error with resolved path
**Source**: code.claude.com/docs/en/memory

### CC-MEM-002 [HIGH] Circular Import
**Requirement**: @imports MUST NOT create circular references
**Detection**: Build import graph, detect cycles
**Fix**: Show cycle path
**Source**: code.claude.com/docs/en/memory

### CC-MEM-003 [HIGH] Import Depth Exceeds 5
**Requirement**: @import chain MUST NOT exceed 5 hops
**Detection**: Track import depth during resolution
**Fix**: Flatten import hierarchy
**Source**: code.claude.com/docs/en/memory

### CC-MEM-004 [HIGH] Invalid Command Reference
**Requirement**: npm scripts referenced MUST exist in package.json
**Detection**: Extract `npm run <script>`, check package.json
**Fix**: Show available scripts
**Source**: awesome-slash/enhance-claude-memory

### CC-MEM-005 [HIGH] Generic Instruction
**Requirement**: Avoid redundant "be helpful" instructions
**Patterns**: `be helpful`, `be accurate`, `think step by step`, `be concise`
**Detection**: Regex match against 8 generic patterns
**Fix**: Remove line
**Source**: awesome-slash/enhance-claude-memory, research papers

### CC-MEM-006 [HIGH] Negative Without Positive
**Requirement**: Negative instructions ("don't") SHOULD include positive alternative
**Detection**: Line contains `don't|never|avoid` without follow-up positive
**Fix**: Suggest "Instead, do..."
**Source**: research: positive framing improves compliance

### CC-MEM-007 [HIGH] Weak Constraint Language
**Requirement**: Critical rules MUST use strong language (must/always/never)
**Detection**: In critical section, check for `should|try to|consider|maybe`
**Fix**: Replace with `must|always|required`
**Source**: research: constraint strength affects compliance

### CC-MEM-008 [HIGH] Critical Content in Middle
**Requirement**: Important rules SHOULD be at START or END (lost in the middle)
**Detection**: "critical" appears after 40% of content
**Fix**: Move to top
**Source**: Liu et al. (2023), TACL

### CC-MEM-009 [MEDIUM] Token Count Exceeded
**Requirement**: File SHOULD be under 1500 tokens (~6000 chars)
**Detection**: `content.len() / 4 > 1500`
**Fix**: Suggest using @imports
**Source**: code.claude.com/docs/en/memory

### CC-MEM-010 [MEDIUM] README Duplication
**Requirement**: CLAUDE.md SHOULD complement README, not duplicate
**Detection**: Compare with README.md, check >40% overlap
**Fix**: Remove duplicated sections
**Source**: awesome-slash/enhance-claude-memory

---

## AGENTS.MD RULES (CROSS-PLATFORM)

### AGM-001 [HIGH] Valid Markdown Structure
**Requirement**: AGENTS.md MUST be valid markdown
**Detection**: Parse as markdown, check for syntax errors
**Fix**: Fix markdown syntax issues
**Source**: developers.openai.com/codex/guides/agents-md, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

### AGM-002 [MEDIUM] Missing Section Headers
**Requirement**: AGENTS.md SHOULD have clear section headers (##)
**Detection**: `!content.contains("## ")` or `!content.contains("# ")`
**Fix**: Add section headers for organization
**Source**: docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

### AGM-003 [HIGH] Character Limit (Windsurf)
**Requirement**: Rules files SHOULD be under 12000 characters for Windsurf compatibility
**Detection**: `content.len() > 12000`
**Fix**: Split into multiple files or reduce content
**Source**: docs.windsurf.com/windsurf/cascade/memories

### AGM-004 [MEDIUM] Missing Project Context
**Requirement**: AGENTS.md SHOULD describe project purpose/stack
**Detection**: Check for project description section
**Fix**: Add "# Project" or "## Overview" section
**Source**: Best practices across platforms

### AGM-005 [HIGH] Platform-Specific Features Without Guard
**Requirement**: Platform-specific instructions SHOULD be labeled
**Detection**: Claude-specific (hooks, context: fork) or Cursor-specific features without platform label
**Fix**: Add platform guard comment (e.g., "## Claude Code Specific")
**Source**: Multi-platform compatibility

### AGM-006 [MEDIUM] Nested AGENTS.md Hierarchy
**Requirement**: Some tools load AGENTS.md hierarchically (multiple files may apply)
**Detection**: Multiple AGENTS.md files in directory tree
**Fix**: Document inheritance behavior
**Source**: developers.openai.com/codex/guides/agents-md, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

---

## CLAUDE CODE RULES (PLUGINS)

### CC-PL-001 [HIGH] Plugin Manifest Not in .claude-plugin/
**Requirement**: plugin.json MUST be in `.claude-plugin/` directory
**Detection**: Check `!.claude-plugin/plugin.json` exists
**Fix**: Move to correct location
**Source**: code.claude.com/docs/en/plugins

### CC-PL-002 [HIGH] Components in .claude-plugin/
**Requirement**: skills/agents/hooks MUST NOT be inside .claude-plugin/
**Detection**: Check for `.claude-plugin/skills/`, etc.
**Fix**: Move to plugin root
**Source**: code.claude.com/docs/en/plugins-reference

### CC-PL-003 [HIGH] Invalid Semver
**Requirement**: version MUST be semver format (major.minor.patch)
**Detection**: `!Regex::new(r"^\d+\.\d+\.\d+$").matches(version)`
**Fix**: Suggest valid semver
**Source**: code.claude.com/docs/en/plugins-reference

### CC-PL-004 [HIGH] Missing Required Plugin Field
**Requirement**: plugin.json REQUIRES name, description, version
**Detection**: Parse JSON, check required fields
**Fix**: Add missing fields
**Source**: code.claude.com/docs/en/plugins-reference

### CC-PL-005 [HIGH] Empty Plugin Name
**Requirement**: name field MUST NOT be empty
**Detection**: `name.trim().is_empty()`
**Fix**: Add plugin name
**Source**: code.claude.com/docs/en/plugins-reference

---

## MCP RULES

### MCP-001 [HIGH] Invalid JSON-RPC Version
**Requirement**: MUST use JSON-RPC 2.0
**Detection**: `message.jsonrpc != "2.0"`
**Fix**: Set `"jsonrpc": "2.0"`
**Source**: modelcontextprotocol.io/specification

### MCP-002 [HIGH] Missing Required Tool Field
**Requirement**: Tool MUST have `name`, `description`, `inputSchema`
**Detection**: Parse tool definition, check fields
**Fix**: Add missing fields
**Source**: modelcontextprotocol.io/docs/concepts/tools

### MCP-003 [HIGH] Invalid JSON Schema
**Requirement**: inputSchema MUST be valid JSON Schema
**Detection**: Validate against JSON Schema spec
**Fix**: Show schema errors
**Source**: modelcontextprotocol.io/specification

### MCP-004 [HIGH] Missing Tool Description
**Requirement**: Tool SHOULD have clear description
**Detection**: `description.is_empty()`
**Fix**: Add description
**Source**: modelcontextprotocol.io/docs/concepts/tools

### MCP-005 [HIGH] Tool Without User Consent
**Requirement**: Tools MUST have user consent before invocation
**Detection**: Check for permission flow
**Fix**: Document consent requirement
**Source**: modelcontextprotocol.io/specification (Security)

### MCP-006 [HIGH] Untrusted Annotations
**Requirement**: Tool annotations MUST be considered untrusted unless from trusted server
**Detection**: Check server trust level
**Fix**: Add validation layer
**Source**: modelcontextprotocol.io/docs/concepts/tools

---

## UNIVERSAL RULES (XML)

### XML-001 [HIGH] Unclosed XML Tag
**Requirement**: All XML tags MUST be properly closed
**Detection**: Parse tags, check balance with stack
**Fix**: Add closing tag
**Source**: platform.claude.com/docs prompt engineering

### XML-002 [HIGH] Mismatched Closing Tag
**Requirement**: Closing tag MUST match opening tag
**Detection**: `stack.last().name != closing_tag.name`
**Fix**: Replace with correct closing tag
**Source**: XML parsing standard

### XML-003 [HIGH] Unmatched Closing Tag
**Requirement**: Closing tag MUST have corresponding opening tag
**Detection**: `stack.is_empty() && found_closing_tag`
**Fix**: Remove or add opening tag
**Source**: XML parsing standard

---

## UNIVERSAL RULES (REFERENCES)

### REF-001 [HIGH] Import File Not Found
**Requirement**: @import references MUST point to existing files
**Detection**: Resolve path, check existence
**Fix**: Show resolved path, suggest alternatives
**Source**: code.claude.com/docs/en/memory

### REF-002 [HIGH] Broken Markdown Link
**Requirement**: Markdown links SHOULD point to existing files
**Detection**: Extract `[text](path)`, check existence
**Fix**: Show available files
**Source**: Standard markdown validation

---

## PROMPT ENGINEERING RULES

### PE-001 [HIGH] Lost in the Middle
**Requirement**: Critical content MUST NOT be in middle 40-60%
**Detection**: Find "critical|important|must" positions, check if in middle
**Fix**: Move to start or end
**Source**: Liu et al. (2023), "Lost in the Middle: How Language Models Use Long Contexts", TACL

### PE-002 [HIGH] Chain-of-Thought on Simple Task
**Requirement**: Don't use "think step by step" for simple operations
**Detection**: Check for CoT phrases in simple skills (file reads, basic commands)
**Fix**: Remove CoT instructions
**Source**: Wei et al. (2022), research shows CoT hurts simple tasks

### PE-003 [MEDIUM] Weak Imperative Language
**Requirement**: Use strong language (must/always/never) for critical rules
**Detection**: Critical section with `should|could|try|consider|maybe`
**Fix**: Replace with must/always/required
**Source**: Multiple prompt engineering studies

### PE-004 [MEDIUM] Ambiguous Instructions
**Requirement**: Instructions SHOULD be specific and measurable
**Detection**: Check for vague terms without concrete criteria
**Fix**: Add specific criteria or examples
**Source**: Anthropic prompt engineering guide

---

## CROSS-PLATFORM RULES

### XP-001 [HIGH] Platform-Specific Feature in Generic Config
**Requirement**: Generic configs MUST NOT use platform-specific features
**Detection**: Check for Claude-only features (hooks, context: fork) in AGENTS.md
**Fix**: Move to CLAUDE.md or add platform guard
**Source**: multi-platform research

### XP-002 [HIGH] AGENTS.md Platform Compatibility
**Requirement**: AGENTS.md is a widely-adopted standard used by multiple platforms
**Supported Platforms**:
- Codex CLI (OpenAI)
- OpenCode
- GitHub Copilot coding agent
- Cursor (alongside `.cursor/rules/`)
- Cline (alongside `.clinerules`)
**Note**: Claude Code uses `CLAUDE.md` (not AGENTS.md)
**Detection**: Validate AGENTS.md follows markdown conventions
**Fix**: Ensure AGENTS.md is valid markdown with clear sections
**Source**: developers.openai.com/codex/guides/agents-md, opencode.ai/docs/guides/project-docs, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

### XP-003 [HIGH] Hard-Coded Platform Paths
**Requirement**: Paths SHOULD use environment variables
**Detection**: Check for `.claude/`, `.opencode/` in configs
**Fix**: Use `$CLAUDE_PROJECT_DIR` or equivalent
**Source**: multi-platform best practices

---

## PRIORITY MATRIX

### P0 (MVP - Week 3)
Implement these 30 rules first:
- AS-001 through AS-009 (Skills frontmatter)
- CC-SK-001 through CC-SK-008 (Claude skills)
- CC-HK-001 through CC-HK-008 (Hooks)
- CC-MEM-001, CC-MEM-005 (Memory critical)
- XML-001 through XML-003 (XML balance)
- REF-001 (Import validation)

### P1 (Week 4)
Add these 15 rules:
- AS-010 through AS-015 (Skills best practices)
- CC-MEM-006 through CC-MEM-010 (Memory quality)
- CC-AG-001 through CC-AG-006 (Agents)
- CC-PL-001 through CC-PL-005 (Plugins)

### P2 (Week 5-6)
Complete coverage:
- MCP-001 through MCP-006 (MCP protocol)
- PE-001 through PE-004 (Prompt engineering)
- XP-001 through XP-003 (Cross-platform)
- Remaining MEDIUM/LOW certainty rules

---

## Implementation Reference

### Detection Pseudocode

```rust
pub fn validate_skill(path: &Path, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // AS-001: Check frontmatter exists
    if !content.starts_with("---") {
        diagnostics.push(Diagnostic::error(
            path, 1, 0, "AS-001",
            "Missing YAML frontmatter".to_string()
        ));
        return diagnostics; // Can't continue without frontmatter
    }

    // Parse frontmatter
    let (frontmatter, body) = parse_frontmatter::<SkillSchema>(content)?;

    // AS-002: Check name exists
    if frontmatter.name.is_empty() {
        diagnostics.push(Diagnostic::error(
            path, 2, 0, "AS-002",
            "Missing required field: name".to_string()
        ));
    }

    // AS-004: Check name format
    let name_re = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
    if !name_re.is_match(&frontmatter.name) || frontmatter.name.len() > 64 {
        diagnostics.push(Diagnostic::error(
            path, 2, 0, "AS-004",
            format!("Invalid name format: {}", frontmatter.name)
        ).with_suggestion("Use lowercase letters, numbers, hyphens only"));
    }

    // Continue with other rules...
    diagnostics
}
```

### Auto-Fix Priority

| Rule | Auto-Fix | Safety |
|------|----------|--------|
| AS-001 | Add frontmatter template | HIGH |
| AS-004 | Lowercase + replace _ with - | HIGH |
| AS-010 | Add "Use when..." prefix | MEDIUM |
| CC-SK-007 | Suggest Bash(git:*) | MEDIUM |
| CC-MEM-005 | Remove line | HIGH |
| XML-001 | Add closing tag | MEDIUM |

---

## Rule Count Summary

| Category | Total Rules | HIGH | MEDIUM | LOW | Auto-Fixable |
|----------|-------------|------|--------|-----|--------------|
| Agent Skills | 15 | 13 | 2 | 0 | 6 |
| Claude Skills | 9 | 7 | 2 | 0 | 3 |
| Claude Hooks | 11 | 9 | 2 | 0 | 2 |
| Claude Agents | 6 | 6 | 0 | 0 | 1 |
| Claude Memory | 10 | 6 | 4 | 0 | 2 |
| AGENTS.md | 6 | 3 | 3 | 0 | 2 |
| Claude Plugins | 5 | 5 | 0 | 0 | 1 |
| MCP | 6 | 6 | 0 | 0 | 1 |
| XML | 3 | 3 | 0 | 0 | 1 |
| References | 2 | 2 | 0 | 0 | 0 |
| Prompt Eng | 4 | 2 | 2 | 0 | 1 |
| Cross-Platform | 3 | 2 | 1 | 0 | 0 |
| **TOTAL** | **80** | **64** | **16** | **0** | **20** |

---

## Sources

### Standards
- agentskills.io (Agent Skills specification)
- modelcontextprotocol.io (MCP specification)
- code.claude.com/docs (Claude Code documentation)
- cursor.com/docs (Cursor AI documentation)
- docs.windsurf.com (Windsurf/Codeium documentation)
- github.com/cline/cline (Cline repository)

### Research Papers
- Liu et al. (2023) - Lost in the middle (TACL)
- Wei et al. (2022) - Chain-of-Thought
- Zhao et al. (2021) - Few-shot calibration

### Production Code
- awesome-slash/plugins/enhance/* (70 patterns, tested on 1000+ files)

### Community
- 15+ platforms researched
- GitHub repos and documentation
- Community conventions and patterns

---

**Total Coverage**: 80 validation rules across 12 categories
**Knowledge Base**: 11,036 lines, 320KB, 75+ sources
**Certainty**: 64 HIGH, 16 MEDIUM, 0 LOW
**Auto-Fixable**: 20 rules (25%)
