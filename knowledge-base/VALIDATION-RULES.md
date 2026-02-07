# agnix Validation Rules - Master Reference

> Consolidated from 320KB knowledge base, 75+ sources, 5 research agents

**Last Updated**: 2026-02-04
**Coverage**: Agent Skills • MCP • Claude Code • Cursor • Multi-Platform • Prompt Engineering

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

## Evidence Metadata Schema

Each rule in `knowledge-base/rules.json` includes an `evidence` object that documents the authoritative source, applicability, and test coverage. This metadata enables:

- **Traceability**: Link rules to their source specifications or research
- **Filtering**: Apply rules only to relevant tools/versions
- **Quality assurance**: Track test coverage for each rule

### Evidence Fields

| Field | Type | Description |
|-------|------|-------------|
| `source_type` | enum | Classification: `spec`, `vendor_docs`, `vendor_code`, `paper`, `community` |
| `source_urls` | string[] | URLs to authoritative documentation or specifications |
| `verified_on` | string | ISO 8601 date when the source was last verified (YYYY-MM-DD) |
| `applies_to` | object | Tool/version/spec constraints for when the rule applies |
| `normative_level` | enum | RFC 2119 level: `MUST`, `SHOULD`, `BEST_PRACTICE` |
| `tests` | object | Test coverage: `{ unit: bool, fixtures: bool, e2e: bool }` |

### Source Types

| Type | Description | Examples |
|------|-------------|----------|
| `spec` | Official specification | agentskills.io/specification, modelcontextprotocol.io/specification |
| `vendor_docs` | Vendor documentation | code.claude.com/docs, docs.github.com/copilot, docs.cursor.com |
| `vendor_code` | Vendor source code | Reference implementations |
| `paper` | Academic research | Liu et al. (2023) TACL, Wei et al. (2022) |
| `community` | Community research | awesome-slash, multi-platform patterns |

### Applicability Constraints

The `applies_to` object specifies when a rule is relevant:

```json
{
  "applies_to": {
    "tool": "claude-code",       // Optional: specific tool
    "version_range": ">=1.0.0", // Optional: semver range
    "spec_revision": "2025-06-18" // Optional: spec version
  }
}
```

Rules with an empty `applies_to` object (`{}`) apply universally.

### Example Evidence Block

```json
{
  "id": "MCP-001",
  "name": "Invalid JSON-RPC Version",
  "severity": "HIGH",
  "category": "mcp",
  "evidence": {
    "source_type": "spec",
    "source_urls": ["https://modelcontextprotocol.io/specification"],
    "verified_on": "2026-02-04",
    "applies_to": { "spec_revision": "2025-06-18" },
    "normative_level": "MUST",
    "tests": { "unit": true, "fixtures": true, "e2e": false }
  }
}
```

---

## AGENT SKILLS RULES

<a id="as-001"></a>
### AS-001 [HIGH] Missing Frontmatter
**Requirement**: SKILL.md MUST have YAML frontmatter between `---` delimiters
**Detection**: `!content.starts_with("---")` or no closing `---`
**Fix**: Add template frontmatter
**Source**: agentskills.io/specification

<a id="as-002"></a>
### AS-002 [HIGH] Missing Required Field: name
**Requirement**: `name` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `name` key
**Fix**: Add `name: directory-name`
**Source**: agentskills.io/specification

<a id="as-003"></a>
### AS-003 [HIGH] Missing Required Field: description
**Requirement**: `description` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `description` key
**Fix**: Add `description: "Use when..."`
**Source**: agentskills.io/specification

<a id="as-004"></a>
### AS-004 [HIGH] Invalid Name Format
**Requirement**: name MUST be 1-64 chars, lowercase letters/numbers/hyphens only
**Regex**: `^[a-z0-9]+(-[a-z0-9]+)*$`
**Detection**:
```rust
!Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").matches(name) || name.len() > 64
```
**Fix**: [AUTO-FIX] Convert name to kebab-case (lowercase, replace `_` with `-`, remove invalid chars, collapse consecutive hyphens, truncate to 64 chars)
**Source**: agentskills.io/specification

<a id="as-005"></a>
### AS-005 [HIGH] Name Starts/Ends with Hyphen
**Requirement**: name MUST NOT start or end with `-`
**Detection**: `name.starts_with('-') || name.ends_with('-')`
**Fix**: Remove leading/trailing hyphens
**Source**: agentskills.io/specification

<a id="as-006"></a>
### AS-006 [HIGH] Consecutive Hyphens in Name
**Requirement**: name MUST NOT contain `--`
**Detection**: `name.contains("--")`
**Fix**: Replace `--` with `-`
**Source**: agentskills.io/specification

<a id="as-007"></a>
### AS-007 [HIGH] Reserved Name
**Requirement**: name MUST NOT be reserved word (anthropic, claude)
**Detection**: `["anthropic", "claude", "skill"].contains(name.as_str())`
**Fix**: Suggest alternative name
**Source**: platform.claude.com/docs

<a id="as-008"></a>
### AS-008 [HIGH] Description Too Short
**Requirement**: description MUST be 1-1024 characters
**Detection**: `description.len() < 1 || description.len() > 1024`
**Fix**: Add minimal description or truncate
**Source**: agentskills.io/specification

<a id="as-009"></a>
### AS-009 [HIGH] Description Contains XML
**Requirement**: description MUST NOT contain XML tags
**Detection**: `Regex::new(r"<[^>]+>").is_match(description)`
**Fix**: Remove XML tags
**Source**: platform.claude.com/docs

<a id="as-010"></a>
### AS-010 [MEDIUM] Missing Trigger Phrase
**Requirement**: description SHOULD include "Use when" trigger
**Detection**: `!description.to_lowercase().contains("use when")`
**Fix**: [AUTO-FIX] Prepend "Use when user wants to " to description
**Source**: awesome-slash/enhance-skills, platform.claude.com/docs

<a id="as-011"></a>
### AS-011 [HIGH] Compatibility Too Long
**Requirement**: compatibility field MUST be 1-500 chars if present
**Detection**: `compatibility.len() > 500`
**Fix**: Truncate to 500 chars
**Source**: agentskills.io/specification

<a id="as-012"></a>
### AS-012 [MEDIUM] Content Exceeds 500 Lines
**Requirement**: SKILL.md SHOULD be under 500 lines
**Detection**: `body.lines().count() > 500`
**Fix**: Suggest moving to references/
**Source**: platform.claude.com/docs, agentskills.io

<a id="as-013"></a>
### AS-013 [HIGH] File Reference Too Deep
**Requirement**: File references MUST be one level deep
**Detection**: Check references like `references/guide.md` vs `refs/deep/nested/file.md`
**Fix**: Flatten directory structure
**Source**: agentskills.io/specification

<a id="as-014"></a>
### AS-014 [HIGH] Windows Path Separator
**Requirement**: Paths MUST use forward slashes, even on Windows
**Detection**: `path.contains("\\")`
**Fix**: Replace `\\` with `/`
**Source**: agentskills.io/specification

<a id="as-015"></a>
### AS-015 [HIGH] Upload Size Exceeds 8MB
**Requirement**: Skill directory MUST be under 8MB total
**Detection**: `directory_size > 8 * 1024 * 1024`
**Fix**: Remove large assets or split skill
**Source**: platform.claude.com/docs

<a id="as-016"></a>
### AS-016 [HIGH] Skill Parse Error
**Requirement**: SKILL.md frontmatter MUST be valid YAML
**Detection**: YAML parse error on frontmatter content
**Fix**: Fix YAML syntax errors in frontmatter
**Source**: agentskills.io/specification

---

## CLAUDE CODE RULES (SKILLS)

<a id="cc-sk-001"></a>
### CC-SK-001 [HIGH] Invalid Model Value
**Requirement**: model MUST be one of: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with closest valid option
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-002"></a>
### CC-SK-002 [HIGH] Invalid Context Value
**Requirement**: context MUST be "fork" or omitted
**Detection**: `context.is_some() && context != "fork"`
**Fix**: Change to "fork" or remove
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-003"></a>
### CC-SK-003 [HIGH] Context Without Agent
**Requirement**: `context: fork` REQUIRES `agent` field
**Detection**: `context == "fork" && agent.is_none()`
**Fix**: Add `agent: general-purpose`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-004"></a>
### CC-SK-004 [HIGH] Agent Without Context
**Requirement**: `agent` field REQUIRES `context: fork`
**Detection**: `agent.is_some() && context != Some("fork")`
**Fix**: Add `context: fork`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-005"></a>
### CC-SK-005 [HIGH] Invalid Agent Type
**Requirement**: agent MUST be: Explore, Plan, general-purpose, or custom kebab-case name (1-64 chars, pattern: `^[a-z0-9]+(-[a-z0-9]+)*$`)
**Detection**: Check against built-in agents or validate kebab-case format
**Fix**: Suggest valid agent or correct format
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-sk-006"></a>
### CC-SK-006 [HIGH] Dangerous Auto-Invocation
**Requirement**: Side-effect skills MUST have `disable-model-invocation: true`
**Detection**: `name.contains("deploy|ship|publish|delete|drop") && !disable_model_invocation`
**Fix**: Add `disable-model-invocation: true`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-007"></a>
### CC-SK-007 [HIGH] Unrestricted Bash
**Requirement**: Bash in allowed-tools SHOULD be scoped
**Detection**: `allowed_tools.contains("Bash") && !allowed_tools.contains("Bash(")`
**Fix**: [AUTO-FIX] Replace unrestricted Bash with scoped version (e.g., `Bash(git:*)`)
**Source**: awesome-slash/enhance-skills

<a id="cc-sk-008"></a>
### CC-SK-008 [HIGH] Unknown Tool Name
**Requirement**: Tool names MUST match Claude Code tools
**Known Tools**: Bash, Read, Write, Edit, Grep, Glob, Task, WebFetch, AskUserQuestion, etc.
**Detection**: Check against tool list
**Fix**: Suggest closest match
**Source**: code.claude.com/docs/en/settings

<a id="cc-sk-009"></a>
### CC-SK-009 [MEDIUM] Too Many Injections
**Requirement**: Limit dynamic injections (!`cmd`) to 3
**Detection**: `content.matches("!\`").count() > 3`
**Fix**: Remove or move to scripts/
**Source**: platform.claude.com/docs

<a id="cc-sk-010"></a>
### CC-SK-010 [HIGH] Invalid Hooks in Skill Frontmatter
**Requirement**: `hooks` field in skill frontmatter MUST follow the same schema as settings.json hooks (valid events, handler types, required fields)
**Detection**: Parse hooks YAML value and validate against HooksSchema rules
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-011"></a>
### CC-SK-011 [HIGH] Unreachable Skill
**Requirement**: Skill MUST NOT set both `user-invocable: false` and `disable-model-invocation: true`
**Detection**: `user_invocable == false && disable_model_invocation == true`
**Fix**: No auto-fix (intent unclear)
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-012"></a>
### CC-SK-012 [MEDIUM] Argument Hint Without $ARGUMENTS
**Requirement**: If `argument-hint` is set, body SHOULD reference `$ARGUMENTS`
**Detection**: `argument_hint.is_some() && !body.contains("$ARGUMENTS")`
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-013"></a>
### CC-SK-013 [MEDIUM] Fork Context Without Actionable Instructions
**Requirement**: Skills with `context: fork` SHOULD contain imperative instructions for the forked agent
**Detection**: Check body for imperative verbs when context is fork
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-014"></a>
### CC-SK-014 [HIGH] Invalid disable-model-invocation Type
**Requirement**: `disable-model-invocation` MUST be a boolean, not a string
**Detection**: Raw YAML parsing detects quoted "true"/"false" strings
**Fix**: [AUTO-FIX, safe] Convert string to boolean
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-015"></a>
### CC-SK-015 [HIGH] Invalid user-invocable Type
**Requirement**: `user-invocable` MUST be a boolean, not a string
**Detection**: Raw YAML parsing detects quoted "true"/"false" strings
**Fix**: [AUTO-FIX, safe] Convert string to boolean
**Source**: code.claude.com/docs/en/skills

---

## CLAUDE CODE RULES (HOOKS)

<a id="cc-hk-001"></a>
### CC-HK-001 [HIGH] Invalid Hook Event
**Requirement**: Event MUST be one of 12 valid names (case-sensitive)
**Valid**: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, SubagentStart, SubagentStop, Stop, PreCompact, Setup, SessionEnd, Notification
**Detection**: `!VALID_EVENTS.contains(event)`
**Fix**: [AUTO-FIX] Replace with closest matching valid event name
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-002"></a>
### CC-HK-002 [HIGH] Prompt Hook on Wrong Event
**Requirement**: `type: "prompt"` ONLY for Stop and SubagentStop
**Detection**: `hook.type == "prompt" && !["Stop", "SubagentStop"].contains(event)`
**Fix**: Change to `type: "command"` or use Stop/SubagentStop
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-003"></a>
### CC-HK-003 [HIGH] Missing Matcher for Tool Events
**Requirement**: PreToolUse/PermissionRequest/PostToolUse REQUIRE matcher
**Detection**: `["PreToolUse", "PermissionRequest", "PostToolUse"].contains(event) && matcher.is_none()`
**Fix**: Add `"matcher": "*"` or specific tool
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-004"></a>
### CC-HK-004 [HIGH] Matcher on Non-Tool Event
**Requirement**: Stop/SubagentStop/UserPromptSubmit MUST NOT have matcher
**Detection**: `["Stop", "SubagentStop", "UserPromptSubmit"].contains(event) && matcher.is_some()`
**Fix**: Remove matcher field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-005"></a>
### CC-HK-005 [HIGH] Missing Type Field
**Requirement**: Hook MUST have `type: "command"` or `type: "prompt"`
**Detection**: `hook.type.is_none()`
**Fix**: Add `"type": "command"`
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-006"></a>
### CC-HK-006 [HIGH] Missing Command Field
**Requirement**: `type: "command"` REQUIRES `command` field
**Detection**: `hook.type == "command" && hook.command.is_none()`
**Fix**: Add command field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-007"></a>
### CC-HK-007 [HIGH] Missing Prompt Field
**Requirement**: `type: "prompt"` REQUIRES `prompt` field
**Detection**: `hook.type == "prompt" && hook.prompt.is_none()`
**Fix**: Add prompt field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-008"></a>
### CC-HK-008 [HIGH] Script File Not Found
**Requirement**: Hook command script MUST exist on filesystem
**Detection**: Check if script path exists (resolve $CLAUDE_PROJECT_DIR)
**Fix**: Show error with correct path
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-009"></a>
### CC-HK-009 [HIGH] Dangerous Command Pattern
**Requirement**: Hooks SHOULD NOT contain destructive commands
**Patterns**: `rm -rf`, `git reset --hard`, `drop database`, `curl.*|.*sh`
**Detection**: Regex match against dangerous patterns
**Fix**: Warn, suggest safer alternative
**Source**: awesome-slash/enhance-hooks

<a id="cc-hk-010"></a>
### CC-HK-010 [MEDIUM] Timeout Policy
**Requirement**: Hooks SHOULD have explicit timeout; excessive timeouts warn
**Detection**:
  - `hook.timeout.is_none()` - missing timeout
  - Command: `timeout > 600` exceeds 10-min default
  - Prompt: `timeout > 30` exceeds 30s default
**Fix**: Add explicit timeout within default limits (600s for commands, 30s for prompts)
**Source**: code.claude.com/docs/en/hooks
**Version-Aware**: When Claude Code version is not pinned in `.agnix.toml [tool_versions]`, an assumption note is added indicating default timeout behavior is assumed. Pin the version for version-specific validation.

<a id="cc-hk-011"></a>
### CC-HK-011 [HIGH] Invalid Timeout Value
**Requirement**: timeout MUST be positive integer
**Detection**: `timeout <= 0`
**Fix**: Set to 30
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-012"></a>
### CC-HK-012 [HIGH] Hooks Parse Error
**Requirement**: Hooks configuration MUST be valid JSON
**Detection**: JSON parse error on settings.json
**Fix**: Fix JSON syntax errors in hooks configuration
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-013"></a>
### CC-HK-013 [HIGH] Async on Non-Command Hook
**Requirement**: `async: true` MUST only appear on `type: "command"` hooks
**Detection**: Check for `async` field on prompt or agent hook types
**Fix**: Remove the async field or change hook type to command
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-014"></a>
### CC-HK-014 [MEDIUM] Once Outside Skill/Agent Frontmatter
**Requirement**: `once` field SHOULD only appear in skill/agent frontmatter hooks
**Detection**: Check for `once` field in settings.json hooks
**Fix**: Remove the once field from settings.json hooks
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-015"></a>
### CC-HK-015 [MEDIUM] Model on Command Hook
**Requirement**: `model` field MUST only appear on prompt or agent hooks
**Detection**: Check for `model` field on command hook types
**Fix**: Remove the model field or change hook type to prompt/agent
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-016"></a>
### CC-HK-016 [HIGH] Validate Hook Type Agent
**Requirement**: `type: "agent"` MUST be recognized as a valid hook handler type
**Detection**: Ensure agent type is accepted alongside command and prompt
**Fix**: N/A (recognition rule)
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-017"></a>
### CC-HK-017 [MEDIUM] Prompt/Agent Hook Missing $ARGUMENTS
**Requirement**: Prompt and agent hooks SHOULD reference `$ARGUMENTS` to receive event data
**Detection**: Check prompt or agent hook text for `$ARGUMENTS` reference
**Fix**: Include `$ARGUMENTS` in the prompt or agent hook
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-018"></a>
### CC-HK-018 [LOW] Matcher on UserPromptSubmit/Stop
**Requirement**: Matchers on UserPromptSubmit and Stop events are silently ignored
**Detection**: Check for matcher field on UserPromptSubmit or Stop events
**Fix**: Remove the matcher field
**Source**: code.claude.com/docs/en/hooks

---

## CLAUDE CODE RULES (SUBAGENTS)

<a id="cc-ag-001"></a>
### CC-AG-001 [HIGH] Missing Name Field
**Requirement**: Agent frontmatter REQUIRES `name` field
**Detection**: Parse frontmatter, check for `name`
**Fix**: Add `name: agent-name`
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-002"></a>
### CC-AG-002 [HIGH] Missing Description Field
**Requirement**: Agent frontmatter REQUIRES `description` field
**Detection**: Parse frontmatter, check for `description`
**Fix**: Add description
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-003"></a>
### CC-AG-003 [HIGH] Invalid Model Value
**Requirement**: model MUST be: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-004"></a>
### CC-AG-004 [HIGH] Invalid Permission Mode
**Requirement**: permissionMode MUST be: default, acceptEdits, dontAsk, bypassPermissions, plan
**Detection**: `!VALID_MODES.contains(permission_mode)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-005"></a>
### CC-AG-005 [HIGH] Referenced Skill Not Found
**Requirement**: Skills in `skills` array MUST exist
**Detection**: Check `.claude/skills/{name}/SKILL.md` exists
**Fix**: Remove reference or create skill
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-006"></a>
### CC-AG-006 [HIGH] Tool/Disallowed Conflict
**Requirement**: Tool cannot be in both `tools` and `disallowedTools`
**Detection**: `tools.intersection(disallowedTools).is_empty()`
**Fix**: Remove from one list
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-007"></a>
### CC-AG-007 [HIGH] Agent Parse Error
**Requirement**: Agent frontmatter MUST be valid YAML
**Detection**: YAML parse error on agent frontmatter
**Fix**: Fix YAML syntax errors in agent frontmatter
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-008"></a>
### CC-AG-008 [HIGH] Invalid Memory Scope
**Requirement**: `memory` field MUST be `user`, `project`, or `local`
**Detection**: Check `memory` value against allowed list
**Fix**: Use one of: user, project, local
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-009"></a>
### CC-AG-009 [HIGH] Invalid Tool Name in Tools List
**Requirement**: Tool names in `tools` MUST match known Claude Code tools
**Detection**: Check each tool name against known tools list
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-010"></a>
### CC-AG-010 [HIGH] Invalid Tool Name in DisallowedTools
**Requirement**: Tool names in `disallowedTools` MUST match known Claude Code tools
**Detection**: Check each disallowed tool name against known tools list
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-011"></a>
### CC-AG-011 [HIGH] Invalid Hooks in Agent Frontmatter
**Requirement**: `hooks` object MUST follow the same schema as settings.json hooks
**Detection**: Validate hooks object structure (event names, hook types, required fields)
**Fix**: Ensure hooks follow the settings.json hooks schema
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-012"></a>
### CC-AG-012 [HIGH] Bypass Permissions Warning
**Requirement**: `permissionMode: bypassPermissions` SHOULD NOT be used (disables all safety checks)
**Detection**: Check if permissionMode equals `bypassPermissions`
**Fix**: Consider using `dontAsk` or `acceptEdits` for a safer permission mode
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-013"></a>
### CC-AG-013 [MEDIUM] Invalid Skill Name Format
**Requirement**: Skill names in `skills` array SHOULD follow valid naming format (lowercase, hyphens)
**Detection**: Check skill name matches kebab-case pattern
**Fix**: Use kebab-case format (e.g., 'my-skill-name')
**Source**: code.claude.com/docs/en/sub-agents

---

## CLAUDE CODE RULES (MEMORY)

<a id="cc-mem-001"></a>
### CC-MEM-001 [HIGH] Invalid Import Path
**Requirement**: @import paths MUST exist on filesystem
**Detection**: Extract `@path` references, check existence
**Fix**: Show error with resolved path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-002"></a>
### CC-MEM-002 [HIGH] Circular Import
**Requirement**: @imports MUST NOT create circular references
**Detection**: Build import graph, detect cycles
**Fix**: Show cycle path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-003"></a>
### CC-MEM-003 [HIGH] Import Depth Exceeds 5
**Requirement**: @import chain MUST NOT exceed 5 hops
**Detection**: Track import depth during resolution
**Fix**: Flatten import hierarchy
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-004"></a>
### CC-MEM-004 [MEDIUM] Invalid Command Reference
**Requirement**: npm scripts referenced SHOULD exist in package.json
**Detection**: Extract `npm run <script>`, check package.json
**Fix**: Show available scripts
**Source**: awesome-slash/enhance-claude-memory

<a id="cc-mem-005"></a>
### CC-MEM-005 [HIGH] Generic Instruction
**Requirement**: Avoid redundant "be helpful" instructions
**Patterns**: `be helpful`, `be accurate`, `think step by step`, `be concise`
**Detection**: Regex match against 8 generic patterns
**Fix**: Remove line
**Source**: awesome-slash/enhance-claude-memory, research papers

<a id="cc-mem-006"></a>
### CC-MEM-006 [HIGH] Negative Without Positive
**Requirement**: Negative instructions ("don't") SHOULD include positive alternative
**Detection**: Line contains `don't|never|avoid` without follow-up positive
**Fix**: Suggest "Instead, do..."
**Source**: research: positive framing improves compliance

<a id="cc-mem-007"></a>
### CC-MEM-007 [HIGH] Weak Constraint Language
**Requirement**: Critical rules MUST use strong language (must/always/never)
**Detection**: In critical section, check for `should|try to|consider|maybe`
**Fix**: Replace with `must|always|required`
**Source**: research: constraint strength affects compliance

<a id="cc-mem-008"></a>
### CC-MEM-008 [HIGH] Critical Content in Middle
**Requirement**: Important rules SHOULD be at START or END (lost in the middle)
**Detection**: "critical" appears after 40% of content
**Fix**: Move to top
**Source**: Liu et al. (2023), TACL

<a id="cc-mem-009"></a>
### CC-MEM-009 [MEDIUM] Token Count Exceeded
**Requirement**: File SHOULD be under 1500 tokens (~6000 chars)
**Detection**: `content.len() / 4 > 1500`
**Fix**: Suggest using @imports
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-010"></a>
### CC-MEM-010 [MEDIUM] README Duplication
**Requirement**: CLAUDE.md SHOULD complement README, not duplicate
**Detection**: Compare with README.md, check >40% overlap
**Fix**: Remove duplicated sections
**Source**: awesome-slash/enhance-claude-memory

<a id="cc-mem-011"></a>
### CC-MEM-011 [HIGH] Invalid Paths Glob in Rules
**Requirement**: Glob patterns in `.claude/rules/*.md` frontmatter `paths` field MUST be valid
**Detection**: Parse YAML frontmatter, validate each glob pattern in `paths` array
**Fix**: Manual - fix glob syntax
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-012"></a>
### CC-MEM-012 [MEDIUM] Rules File Unknown Frontmatter Key
**Requirement**: `.claude/rules/*.md` frontmatter SHOULD only contain known keys (`paths`)
**Detection**: Parse YAML frontmatter, flag keys not in known set
**Fix**: Auto-fix (unsafe) - remove unknown key line (may miss multi-line values)
**Source**: code.claude.com/docs/en/memory

---

## AGENTS.MD RULES (CROSS-PLATFORM)

<a id="agm-001"></a>
### AGM-001 [HIGH] Valid Markdown Structure
**Requirement**: AGENTS.md MUST be valid markdown
**Detection**: Parse as markdown, check for syntax errors
**Fix**: Fix markdown syntax issues
**Source**: developers.openai.com/codex/guides/agents-md, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

<a id="agm-002"></a>
### AGM-002 [MEDIUM] Missing Section Headers
**Requirement**: AGENTS.md SHOULD have clear section headers (##)
**Detection**: `!content.contains("## ")` or `!content.contains("# ")`
**Fix**: Add section headers for organization
**Source**: docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

<a id="agm-003"></a>
### AGM-003 [MEDIUM] Character Limit (Windsurf)
**Requirement**: Rules files SHOULD be under 12000 characters for Windsurf compatibility
**Detection**: `content.len() > 12000`
**Fix**: Split into multiple files or reduce content
**Source**: docs.windsurf.com/windsurf/cascade/memories

<a id="agm-004"></a>
### AGM-004 [MEDIUM] Missing Project Context
**Requirement**: AGENTS.md SHOULD describe project purpose/stack
**Detection**: Check for project description section
**Fix**: Add "# Project" or "## Overview" section
**Source**: Best practices across platforms

<a id="agm-005"></a>
### AGM-005 [MEDIUM] Platform-Specific Features Without Guard
**Requirement**: Platform-specific instructions SHOULD be labeled
**Detection**: Claude-specific (hooks, context: fork) or Cursor-specific features without platform label
**Fix**: Add platform guard comment (e.g., "## Claude Code Specific")
**Source**: Multi-platform compatibility

<a id="agm-006"></a>
### AGM-006 [MEDIUM] Nested AGENTS.md Hierarchy
**Requirement**: Some tools load AGENTS.md hierarchically (multiple files may apply)
**Detection**: Multiple AGENTS.md files in directory tree
**Fix**: Document inheritance behavior
**Source**: developers.openai.com/codex/guides/agents-md, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

---

## CLAUDE CODE RULES (PLUGINS)

<a id="cc-pl-001"></a>
### CC-PL-001 [HIGH] Plugin Manifest Not in .claude-plugin/
**Requirement**: plugin.json MUST be in `.claude-plugin/` directory
**Detection**: Check `!.claude-plugin/plugin.json` exists
**Fix**: Move to correct location
**Source**: code.claude.com/docs/en/plugins

<a id="cc-pl-002"></a>
### CC-PL-002 [HIGH] Components in .claude-plugin/
**Requirement**: skills/agents/hooks MUST NOT be inside .claude-plugin/
**Detection**: Check for `.claude-plugin/skills/`, etc.
**Fix**: Move to plugin root
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-003"></a>
### CC-PL-003 [HIGH] Invalid Semver
**Requirement**: version MUST be semver format (major.minor.patch)
**Detection**: `!Regex::new(r"^\d+\.\d+\.\d+$").matches(version)`
**Fix**: Suggest valid semver
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-004"></a>
### CC-PL-004 [HIGH] Missing Required Plugin Field
**Requirement**: plugin.json REQUIRES name, description, version
**Detection**: Parse JSON, check required fields
**Fix**: Add missing fields
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-005"></a>
### CC-PL-005 [HIGH] Empty Plugin Name
**Requirement**: name field MUST NOT be empty
**Detection**: `name.trim().is_empty()`
**Fix**: Add plugin name
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-006"></a>
### CC-PL-006 [HIGH] Plugin Parse Error
**Requirement**: plugin.json MUST be valid JSON
**Detection**: JSON parse error on plugin.json
**Fix**: Fix JSON syntax errors in plugin.json
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-007"></a>
### CC-PL-007 [HIGH] Invalid Component Path
**Requirement**: Paths in `commands`, `agents`, `skills`, `hooks` MUST be relative (no absolute paths or `..` traversal)
**Detection**: Check path fields for absolute paths (`/`, `C:\`) or parent traversal (`..`)
**Fix**: Prepend `./` to relative paths [safe autofix]
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-008"></a>
### CC-PL-008 [HIGH] Component Inside .claude-plugin
**Requirement**: Component paths in manifest MUST NOT point inside `.claude-plugin/` directory
**Detection**: Check if path fields reference `.claude-plugin/` subdirectories
**Fix**: Suggest moving components to plugin root
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-009"></a>
### CC-PL-009 [MEDIUM] Invalid Author Object
**Requirement**: If `author` field is present, `author.name` SHOULD be a non-empty string
**Detection**: Check `author.name` exists and is non-empty when `author` is present
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-010"></a>
### CC-PL-010 [MEDIUM] Invalid Homepage URL
**Requirement**: If `homepage` field is present, it SHOULD be a valid URL (http/https)
**Detection**: Validate URL format with http/https scheme check
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins-reference

---

## MCP RULES

<a id="mcp-001"></a>
### MCP-001 [HIGH] Invalid JSON-RPC Version
**Requirement**: MUST use JSON-RPC 2.0
**Detection**: `message.jsonrpc != "2.0"`
**Fix**: Set `"jsonrpc": "2.0"`
**Source**: modelcontextprotocol.io/specification

<a id="mcp-002"></a>
### MCP-002 [HIGH] Missing Required Tool Field
**Requirement**: Tool MUST have `name`, `description`, `inputSchema`
**Detection**: Parse tool definition, check fields
**Fix**: Add missing fields
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-003"></a>
### MCP-003 [HIGH] Invalid JSON Schema
**Requirement**: inputSchema MUST be valid JSON Schema
**Detection**: Validate against JSON Schema spec
**Fix**: Show schema errors
**Source**: modelcontextprotocol.io/specification

<a id="mcp-004"></a>
### MCP-004 [HIGH] Missing Tool Description
**Requirement**: Tool SHOULD have clear description
**Detection**: `description.is_empty()`
**Fix**: Add description
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-005"></a>
### MCP-005 [HIGH] Tool Without User Consent
**Requirement**: Tools MUST have user consent before invocation
**Detection**: Check for permission flow
**Fix**: Document consent requirement
**Source**: modelcontextprotocol.io/specification (Security)

<a id="mcp-006"></a>
### MCP-006 [HIGH] Untrusted Annotations
**Requirement**: Tool annotations MUST be considered untrusted unless from trusted server
**Detection**: Check server trust level
**Fix**: Add validation layer
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-007"></a>
### MCP-007 [HIGH] MCP Parse Error
**Requirement**: MCP configuration MUST be valid JSON
**Detection**: JSON parse error on MCP configuration file
**Fix**: Fix JSON syntax errors in MCP configuration
**Source**: modelcontextprotocol.io/specification

<a id="mcp-008"></a>
### MCP-008 [MEDIUM] Protocol Version Mismatch
**Requirement**: MCP initialize messages SHOULD use the expected protocol version
**Detection**: Check `protocolVersion` field in initialize request params or response result against configured expected version (default: "2025-06-18")
**Fix**: Update protocolVersion to match expected version, or configure `mcp_protocol_version` in agnix config to match your target version
**Note**: This is a warning (not error) because MCP allows version negotiation between client and server
**Source**: modelcontextprotocol.io/specification (Protocol Versioning)
**Version-Aware**: When MCP protocol version is not pinned in `.agnix.toml [spec_revisions]`, an assumption note is added indicating default protocol version is being used. Pin the version with `mcp_protocol = "2025-06-18"` for explicit control.

<a id="mcp-009"></a>
### MCP-009 [HIGH] Missing command for stdio server
**Requirement**: Stdio MCP servers MUST have a `command` field
**Detection**: Server entry has `type: "stdio"` (or no type, since stdio is default) but no `command` field
**Fix**: Add a `command` field specifying the executable to run
**Source**: modelcontextprotocol.io/specification

<a id="mcp-010"></a>
### MCP-010 [HIGH] Missing url for http/sse server
**Requirement**: HTTP and SSE MCP servers MUST have a `url` field
**Detection**: Server entry has `type: "http"` or `type: "sse"` but no `url` field
**Fix**: Add a `url` field specifying the server endpoint
**Source**: modelcontextprotocol.io/specification

<a id="mcp-011"></a>
### MCP-011 [HIGH] Invalid MCP server type
**Requirement**: MCP server `type` MUST be `stdio`, `http`, or `sse`
**Detection**: Server entry has a `type` field with an unrecognized value
**Fix**: Change type to one of: `stdio`, `http`, `sse`
**Source**: modelcontextprotocol.io/specification

<a id="mcp-012"></a>
### MCP-012 [MEDIUM] Deprecated SSE transport
**Requirement**: SSE transport SHOULD be replaced with Streamable HTTP
**Detection**: Server entry has `type: "sse"`
**Fix**: Change `type` from `"sse"` to `"http"` (unsafe: server may not support Streamable HTTP)
**Note**: This is a warning because SSE still works but is deprecated in favor of Streamable HTTP
**Source**: modelcontextprotocol.io/specification

---

## GITHUB COPILOT RULES

<a id="cop-001"></a>
### COP-001 [HIGH] Empty Instruction File
**Requirement**: Copilot instruction files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful instructions
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-002"></a>
### COP-002 [HIGH] Invalid Frontmatter
**Requirement**: Scoped instruction files (.github/instructions/*.instructions.md) MUST have valid YAML frontmatter with `applyTo` field
**Detection**: Parse YAML between `---` markers, check for `applyTo` key
**Fix**: Add valid frontmatter with `applyTo` glob pattern
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-003"></a>
### COP-003 [HIGH] Invalid Glob Pattern
**Requirement**: `applyTo` field MUST contain valid glob patterns
**Detection**: Attempt to parse as glob pattern
**Fix**: Correct the glob syntax
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-004"></a>
### COP-004 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: Scoped instruction frontmatter SHOULD only contain known keys (`applyTo`, `excludeAgent`)
**Detection**: Check for keys other than `applyTo` and `excludeAgent` in frontmatter
**Fix**: Remove unknown keys
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-005"></a>
### COP-005 [HIGH] Invalid excludeAgent Value
**Requirement**: The `excludeAgent` frontmatter field in scoped instruction files MUST be either `"code-review"` or `"coding-agent"`
**Detection**: Parse frontmatter, validate `excludeAgent` value against allowed set
**Fix**: Use a valid `excludeAgent` value
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-006"></a>
### COP-006 [MEDIUM] File Length Limit
**Requirement**: Global instruction files (`.github/copilot-instructions.md`) SHOULD not exceed ~4000 characters
**Detection**: Check `content.chars().count() > 4000`
**Fix**: Reduce content or split into scoped instruction files
**Source**: docs.github.com/en/copilot/customizing-copilot

---

## CURSOR PROJECT RULES

<a id="cur-001"></a>
### CUR-001 [HIGH] Empty Cursor Rule File
**Requirement**: Cursor .mdc rule files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful rules content
**Source**: docs.cursor.com/en/context

<a id="cur-002"></a>
### CUR-002 [MEDIUM] Missing Frontmatter in .mdc File
**Requirement**: Cursor .mdc files SHOULD have YAML frontmatter with metadata
**Detection**: File doesn't start with `---` markers
**Fix**: Add YAML frontmatter with description and globs fields
**Source**: docs.cursor.com/en/context

<a id="cur-003"></a>
### CUR-003 [HIGH] Invalid YAML Frontmatter
**Requirement**: .mdc file frontmatter MUST be valid YAML
**Detection**: YAML parse error on frontmatter content
**Fix**: Fix YAML syntax errors in frontmatter
**Source**: docs.cursor.com/en/context

<a id="cur-004"></a>
### CUR-004 [HIGH] Invalid Glob Pattern in globs Field
**Requirement**: `globs` field MUST contain valid glob patterns
**Detection**: Attempt to parse as glob pattern
**Fix**: Correct the glob syntax
**Source**: docs.cursor.com/en/context

<a id="cur-005"></a>
### CUR-005 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: .mdc frontmatter SHOULD only contain known keys (description, globs, alwaysApply)
**Detection**: Check for keys other than known keys in frontmatter
**Fix**: Remove unknown keys
**Source**: docs.cursor.com/en/context

<a id="cur-006"></a>
### CUR-006 [MEDIUM] Legacy .cursorrules File Detected
**Requirement**: Projects SHOULD migrate from .cursorrules to .cursor/rules/*.mdc format
**Detection**: File named `.cursorrules`
**Fix**: Create `.cursor/rules/` directory and migrate rules to .mdc files
**Source**: docs.cursor.com/en/context

<a id="cur-007"></a>
### CUR-007 [MEDIUM] alwaysApply with Redundant globs
**Requirement**: When `alwaysApply: true`, the `globs` field SHOULD NOT be set (it is redundant)
**Detection**: Frontmatter has both `alwaysApply: true` and a `globs` field
**Fix**: [AUTO-FIX] Remove the `globs` field (safe)
**Source**: docs.cursor.com/en/context

<a id="cur-008"></a>
### CUR-008 [HIGH] Invalid alwaysApply Type
**Requirement**: `alwaysApply` MUST be a boolean (`true`/`false`), not a quoted string
**Detection**: `alwaysApply` value is a string (e.g., `"true"` or `"false"`) instead of a boolean
**Fix**: Remove quotes around the value
**Source**: docs.cursor.com/en/context

<a id="cur-009"></a>
### CUR-009 [MEDIUM] Missing Description for Agent-Requested Rule
**Requirement**: Rules with no `alwaysApply` and no `globs` (agent-requested rules) SHOULD have a `description`
**Detection**: Frontmatter has no `alwaysApply`, no `globs`, and no `description` (or empty description)
**Fix**: Add a `description` field explaining when the rule should apply
**Source**: docs.cursor.com/en/context

---

## CLINE RULES

<a id="cln-001"></a>
### CLN-001 [HIGH] Empty Cline Rules File
**Requirement**: `.clinerules` file or files in `.clinerules/` folder MUST have non-empty content after frontmatter
**Detection**: Parse file, strip optional YAML frontmatter, check remaining body is non-whitespace
**Fix**: No auto-fix (content must be authored by user)
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

<a id="cln-002"></a>
### CLN-002 [HIGH] Invalid Paths Glob in Cline Rules
**Requirement**: `paths` field in `.clinerules/*.md` frontmatter MUST contain valid glob patterns
**Detection**: Parse YAML frontmatter, extract `paths` field, validate each glob pattern
**Fix**: No auto-fix (glob patterns must be manually corrected)
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

<a id="cln-003"></a>
### CLN-003 [MEDIUM] Unknown Frontmatter Key in Cline Rules
**Requirement**: Frontmatter in `.clinerules/*.md` files SHOULD only use documented keys (`paths`)
**Detection**: Parse YAML frontmatter, check all keys against allowlist
**Fix**: [AUTO-FIX unsafe] Remove unknown frontmatter keys
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

---

## OPENCODE RULES

<a id="oc-001"></a>
### OC-001 [HIGH] Invalid Share Mode
**Requirement**: The `share` field in `opencode.json` MUST be `"manual"`, `"auto"`, or `"disabled"`
**Detection**: Parse JSON, validate `share` value against allowed set
**Fix**: Use a valid share mode value
**Source**: opencode.ai/docs/config

<a id="oc-002"></a>
### OC-002 [HIGH] Invalid Instruction Path
**Requirement**: Paths in the `instructions` array MUST exist on disk or be valid glob patterns
**Detection**: Parse JSON, resolve each path in `instructions` array relative to config file location
**Fix**: Fix or remove broken instruction paths
**Source**: opencode.ai/docs/config

<a id="oc-003"></a>
### OC-003 [HIGH] opencode.json Parse Error
**Requirement**: `opencode.json` MUST be valid JSON (or JSONC with comments stripped)
**Detection**: Attempt JSON parse, report errors with line/column location
**Fix**: Fix JSON syntax errors
**Source**: opencode.ai/docs/config

---

## UNIVERSAL RULES (XML)

<a id="xml-001"></a>
### XML-001 [HIGH] Unclosed XML Tag
**Requirement**: All XML tags MUST be properly closed
**Detection**: Parse tags, check balance with stack
**Fix**: [AUTO-FIX] Automatically insert matching closing XML tag
**Source**: platform.claude.com/docs prompt engineering

<a id="xml-002"></a>
### XML-002 [HIGH] Mismatched Closing Tag
**Requirement**: Closing tag MUST match opening tag
**Detection**: `stack.last().name != closing_tag.name`
**Fix**: Replace with correct closing tag
**Source**: XML parsing standard

<a id="xml-003"></a>
### XML-003 [HIGH] Unmatched Closing Tag
**Requirement**: Closing tag MUST have corresponding opening tag
**Detection**: `stack.is_empty() && found_closing_tag`
**Fix**: Remove or add opening tag
**Source**: XML parsing standard

---

## UNIVERSAL RULES (REFERENCES)

<a id="ref-001"></a>
### REF-001 [HIGH] Import File Not Found
**Requirement**: @import references MUST point to existing files
**Detection**: Resolve path, check existence
**Fix**: Show resolved path, suggest alternatives
**Source**: code.claude.com/docs/en/memory

<a id="ref-002"></a>
### REF-002 [HIGH] Broken Markdown Link
**Requirement**: Markdown links SHOULD point to existing files
**Detection**: Extract `[text](path)`, check existence
**Fix**: Show available files
**Source**: Standard markdown validation

---

## PROMPT ENGINEERING RULES

<a id="pe-001"></a>
### PE-001 [MEDIUM] Lost in the Middle
**Requirement**: Critical content SHOULD NOT be in middle 40-60%
**Detection**: Find "critical|important|must" positions, check if in middle
**Fix**: Move to start or end
**Source**: Liu et al. (2023), "Lost in the Middle: How Language Models Use Long Contexts", TACL

<a id="pe-002"></a>
### PE-002 [MEDIUM] Chain-of-Thought on Simple Task
**Requirement**: SHOULD NOT use "think step by step" for simple operations
**Detection**: Check for CoT phrases in simple skills (file reads, basic commands)
**Fix**: Remove CoT instructions
**Source**: Wei et al. (2022), research shows CoT hurts simple tasks

<a id="pe-003"></a>
### PE-003 [MEDIUM] Weak Imperative Language
**Requirement**: Use strong language (must/always/never) for critical rules
**Detection**: Critical section with `should|could|try|consider|maybe`
**Fix**: Replace with must/always/required
**Source**: Multiple prompt engineering studies

<a id="pe-004"></a>
### PE-004 [MEDIUM] Ambiguous Instructions
**Requirement**: Instructions SHOULD be specific and measurable
**Detection**: Check for vague terms without concrete criteria
**Fix**: Add specific criteria or examples
**Source**: Anthropic prompt engineering guide

---

## CROSS-PLATFORM RULES

<a id="xp-001"></a>
### XP-001 [HIGH] Platform-Specific Feature in Generic Config
**Requirement**: Generic configs MUST NOT use platform-specific features
**Detection**: Check for Claude-only features (hooks, context: fork) in AGENTS.md
**Fix**: Move to CLAUDE.md or wrap in a Claude-specific section header
**Example**: Valid guarded section:
```markdown
## Claude Code Specific
- type: PreToolExecution
  command: echo "lint"
context: fork
agent: reviewer
```
**Source**: multi-platform research

<a id="xp-002"></a>
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
**Source**: developers.openai.com/codex/guides/agents-md, opencode.ai/docs/rules, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

<a id="xp-003"></a>
### XP-003 [HIGH] Hard-Coded Platform Paths
**Requirement**: Paths SHOULD use environment variables
**Detection**: Check for `.claude/`, `.opencode/` in configs
**Fix**: Use `$CLAUDE_PROJECT_DIR` or equivalent
**Source**: multi-platform best practices

<a id="xp-004"></a>
### XP-004 [MEDIUM] Conflicting Build/Test Commands
**Requirement**: Instruction files SHOULD use consistent package managers
**Detection**: Extract build commands (npm/pnpm/yarn/bun) from multiple instruction files, detect conflicts when different managers are used for the same command type
**Fix**: Standardize on a single package manager across all instruction files
**Source**: cross-layer consistency best practices

<a id="xp-005"></a>
### XP-005 [HIGH] Conflicting Tool Constraints
**Requirement**: Tool constraints MUST NOT conflict across instruction layers
**Detection**: Extract tool allow/disallow patterns from multiple instruction files, detect when one file allows a tool and another disallows it
**Fix**: Resolve the conflict by consistently allowing or disallowing the tool
**Source**: cross-layer consistency requirements

<a id="xp-006"></a>
### XP-006 [MEDIUM] Multiple Layers Without Documented Precedence
**Requirement**: When multiple instruction layers exist, precedence SHOULD be documented
**Detection**: Detect multiple instruction files (CLAUDE.md, AGENTS.md, .cursor/rules/, etc.) without documented precedence
**Fix**: Document which file takes precedence (e.g., "CLAUDE.md takes precedence over AGENTS.md")
**Source**: multi-platform clarity requirements

---

## VERSION AWARENESS RULES (VER)

<a id="ver-001"></a>
### VER-001 [LOW] No Tool/Spec Versions Pinned
**Requirement**: Projects SHOULD pin tool/spec versions for deterministic validation
**Detection**: Check if any versions are configured in .agnix.toml [tool_versions] or [spec_revisions]
**Fix**: Add version configuration to .agnix.toml:
```toml
[tool_versions]
claude_code = "2.1.3"

[spec_revisions]
mcp_protocol = "2025-06-18"
```
**Source**: Best practice for reproducible validation

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
- CC-AG-001 through CC-AG-013 (Agents)
- CC-PL-001 through CC-PL-010 (Plugins)

### P2 (Week 5-6)
Complete coverage:
- MCP-001 through MCP-006 (MCP protocol)
- PE-001 through PE-004 (Prompt engineering)
- XP-001 through XP-006 (Cross-platform)
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
| AS-004 | Convert name to kebab-case | safe/unsafe |
| AS-005 | Strip leading/trailing hyphens | safe |
| AS-006 | Collapse consecutive hyphens | safe |
| AS-010 | Prepend "Use when user wants to " | unsafe |
| AS-014 | Normalize Windows path separators | safe |
| CC-SK-001 | Default invalid model to sonnet | unsafe |
| CC-SK-002 | Normalize context to fork | unsafe |
| CC-SK-003 | Add default agent for fork context | unsafe |
| CC-SK-004 | Insert context: fork before agent key | unsafe |
| CC-SK-007 | Suggest Bash(git:*) matcher | unsafe |
| CC-HK-001 | Correct event name casing/typo | safe/unsafe |
| CC-HK-004 | Clamp timeout to valid range | safe |
| CC-HK-011 | Remove redundant wildcard matcher | unsafe |
| CC-AG-003 | Default invalid model to sonnet | unsafe |
| CC-AG-004 | Default invalid permission mode | unsafe |
| CC-MEM-005 | Remove generic instruction line | safe |
| CC-MEM-007 | Replace weak language with strong | safe/unsafe |
| CC-PL-005 | Normalize plugin name | unsafe |
| CC-PL-007 | Prepend ./ to relative path | safe |
| MCP-001 | Set jsonrpc to "2.0" | safe |
| MCP-008 | Update protocolVersion | unsafe |
| MCP-012 | Change sse to http | unsafe |
| COP-004 | Remove unknown frontmatter key | safe |
| CUR-005 | Remove unknown frontmatter key | safe |
| CUR-007 | Remove redundant globs field | safe |
| CLN-003 | Remove unknown frontmatter key | unsafe |
| XML-001 | Add missing closing tag | unsafe |
| XML-002 | Fix mismatched closing tag | unsafe |
| XML-003 | Remove orphaned closing tag | unsafe |

---

## Rule Count Summary

| Category | Total Rules | HIGH | MEDIUM | LOW | Auto-Fixable |
|----------|-------------|------|--------|-----|--------------|
| Agent Skills | 16 | 14 | 2 | 0 | 5 |
| Claude Skills | 15 | 12 | 3 | 0 | 7 |
| Claude Hooks | 18 | 13 | 4 | 1 | 3 |
| Claude Agents | 13 | 12 | 1 | 0 | 2 |
| Claude Memory | 12 | 8 | 4 | 0 | 3 |
| AGENTS.md | 6 | 1 | 5 | 0 | 0 |
| Claude Plugins | 10 | 8 | 2 | 0 | 2 |
| GitHub Copilot | 6 | 4 | 2 | 0 | 1 |
| Cursor | 9 | 4 | 5 | 0 | 2 |
| Cline | 3 | 2 | 1 | 0 | 1 |
| OpenCode | 3 | 3 | 0 | 0 | 0 |
| MCP | 12 | 10 | 2 | 0 | 3 |
| XML | 3 | 3 | 0 | 0 | 3 |
| References | 2 | 2 | 0 | 0 | 0 |
| Prompt Eng | 4 | 0 | 4 | 0 | 0 |
| Cross-Platform | 6 | 4 | 2 | 0 | 0 |
| Version Awareness | 1 | 0 | 0 | 1 | 0 |
| **TOTAL** | **139** | **100** | **37** | **2** | **32** |


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

**Total Coverage**: 139 validation rules across 17 categories

**Knowledge Base**: 11,036 lines, 320KB, 75+ sources
**Certainty**: 100 HIGH, 37 MEDIUM, 2 LOW
**Auto-Fixable**: 32 rules (24%)

