# Learning Guide: S-tier and A-tier AI Coding Tool Agent Configuration Optional Fields

**Generated**: 2026-02-07
**Sources**: 42 resources analyzed
**Depth**: deep
**Purpose**: Identify gaps in agnix's 100 validation rules for optional field validation and auto-fix opportunities

## Prerequisites

- Familiarity with agnix's current 100 validation rules (see `knowledge-base/VALIDATION-RULES.md`)
- Understanding of YAML frontmatter and JSON configuration formats
- Knowledge of the S-tier (Claude Code, Codex CLI, OpenCode) and A-tier (GitHub Copilot, Cline, Cursor) tool ecosystem

## TL;DR

- Claude Code has the richest configuration surface: skills (11 frontmatter fields + hooks), agents (8+ frontmatter fields including new `memory` field), hooks (15 events, 3 handler types, `async`/`once`/`statusMessage` fields), and plugins (13+ manifest fields including `lspServers`/`outputStyles`)
- GitHub Copilot added `excludeAgent` frontmatter field for scoped instructions (not yet validated by agnix)
- Cursor has only 3 frontmatter fields but agnix could validate the `alwaysApply` + `globs` mutual exclusivity pattern
- Cline supports a `paths` frontmatter field in `.clinerules/*.md` files (not yet validated by agnix)
- OpenCode has a rich `opencode.json` schema with `tools`, `agent`, `command`, `formatter`, `permission` fields (no agnix coverage)
- Codex CLI uses `config.yaml`/`config.json` with `approvalMode`, `providers`, `history` fields (no agnix coverage)
- **34 potential new rules identified** across all tools, plus **12 auto-fix opportunities**

---

## 1. Claude Code Skills (.claude/skills/*/SKILL.md)

### Complete Frontmatter Field Reference

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `name` | string | No (recommended) | lowercase letters, numbers, hyphens; 1-64 chars; no leading/trailing/consecutive hyphens | directory name | AS-002, AS-004, AS-005, AS-006, AS-007 | No |
| `description` | string | Recommended | 1-1024 chars, no XML tags | first paragraph of content | AS-003, AS-008, AS-009, AS-010 | No |
| `license` | string | No | Free-form license text | none | (none) | LOW |
| `compatibility` | string | No | 1-500 chars if present | none | AS-011 | No |
| `metadata` | map<string, string> | No | arbitrary key-value pairs | none | (none) | LOW |
| `allowed-tools` | string | No | space-delimited tool names | none | CC-SK-007, CC-SK-008 | Partial |
| `argument-hint` | string | No | hint text shown in autocomplete | none | (none) | YES |
| `disable-model-invocation` | boolean | No | `true` / `false` | `false` | CC-SK-006 (partial) | Partial |
| `user-invocable` | boolean | No | `true` / `false` | `true` | (none) | YES |
| `model` | string | No | `sonnet`, `opus`, `haiku`, `inherit` | inherit | CC-SK-001 | No |
| `context` | string | No | `fork` (only valid value) | none | CC-SK-002, CC-SK-003, CC-SK-004 | No |
| `agent` | string | No | `Explore`, `Plan`, `general-purpose`, or custom kebab-case name | `general-purpose` | CC-SK-005 | No |
| `hooks` | object | No | Same format as settings.json hooks | none | (none) | YES |

### String Substitution Variables (in skill body content)

| Variable | Description | agnix Coverage |
|----------|-------------|----------------|
| `$ARGUMENTS` | All arguments passed when invoking | (none) |
| `$ARGUMENTS[N]` | Access specific argument by 0-based index | (none) |
| `$N` | Shorthand for `$ARGUMENTS[N]` | (none) |
| `${CLAUDE_SESSION_ID}` | Current session ID | (none) |

### Dynamic Context Injection

The `` !`command` `` syntax runs shell commands before skill content is sent. Currently validated by CC-SK-009 (max 3 injections).

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CC-SK-010: Invalid hooks in skill frontmatter | HIGH | Validate that `hooks` field in skill frontmatter follows the same schema as settings.json hooks | No |
| CC-SK-011: user-invocable=false with disable-model-invocation=true | HIGH | If both are set, the skill is unreachable (neither user nor model can invoke it) | Yes - remove one |
| CC-SK-012: argument-hint without $ARGUMENTS | MEDIUM | If `argument-hint` is set but body doesn't reference `$ARGUMENTS`, the hint is misleading | Yes - add `$ARGUMENTS` |
| CC-SK-013: context=fork without actionable instructions | MEDIUM | Warn when `context: fork` is used with reference-only content (no imperative verbs) | No |
| CC-SK-014: Invalid disable-model-invocation type | HIGH | Must be boolean, not string "true" | Yes - convert to bool |
| CC-SK-015: Invalid user-invocable type | HIGH | Must be boolean, not string "true"/"false" | Yes - convert to bool |

---

## 2. Claude Code Agents (.claude/agents/*.md)

### Complete Frontmatter Field Reference

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `name` | string | Yes | lowercase letters and hyphens | none | CC-AG-001 | No |
| `description` | string | Yes | When Claude should delegate | none | CC-AG-002 | No |
| `tools` | string/array | No | Comma-separated or array of tool names (Read, Grep, Glob, Bash, Edit, Write, Task, WebFetch, WebSearch, etc.) | inherits all | CC-AG-006 (conflict only) | Partial |
| `disallowedTools` | array | No | Tool names to deny | none | CC-AG-006 (conflict only) | Partial |
| `model` | string | No | `sonnet`, `opus`, `haiku`, `inherit` | `inherit` | CC-AG-003 | No |
| `permissionMode` | string | No | `default`, `acceptEdits`, `dontAsk`, `bypassPermissions`, `plan`, `delegate` | inherits | CC-AG-004 | No |
| `skills` | array | No | Skill names to preload | none | CC-AG-005 | No |
| `hooks` | object | No | Same format as settings.json hooks | none | (none) | YES |
| `memory` | string | No | `user`, `project`, `local` | none | (none) | YES |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CC-AG-008: Invalid memory scope | HIGH | `memory` must be `user`, `project`, or `local` | Yes - suggest closest |
| CC-AG-009: Invalid tool name in tools list | HIGH | Tool names in `tools` field must match known Claude Code tools | Yes - suggest closest |
| CC-AG-010: Invalid tool name in disallowedTools | HIGH | Same validation for disallowed tools | Yes - suggest closest |
| CC-AG-011: hooks in agent frontmatter validation | HIGH | Validate hooks object follows settings.json schema | No |
| CC-AG-012: bypassPermissions warning | HIGH | Warn when `permissionMode: bypassPermissions` is used | No |
| CC-AG-013: skills references non-existent skill | HIGH | Already partially covered by CC-AG-005, but could also validate skill name format | Partial |

---

## 3. Claude Code Hooks (settings.json / .claude/settings.json)

### Complete Hook Event Reference

| Event | Matcher Input | Can Block? | Supports Prompt? | agnix Rule |
|-------|--------------|------------|------------------|------------|
| `SessionStart` | `startup`, `resume`, `clear`, `compact` | No | No | CC-HK-001 |
| `UserPromptSubmit` | (no matcher) | Yes (exit 2) | No | CC-HK-001 |
| `PreToolUse` | tool name | Yes | Yes | CC-HK-001, CC-HK-003 |
| `PermissionRequest` | tool name | Yes | Yes | CC-HK-001 |
| `PostToolUse` | tool name | No | Yes | CC-HK-001 |
| `PostToolUseFailure` | tool name | No | No | CC-HK-001 |
| `Notification` | `permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog` | No | No | CC-HK-001 |
| `SubagentStart` | agent type name | No | No | CC-HK-001 |
| `SubagentStop` | agent type name | Yes | Yes | CC-HK-001 |
| `Stop` | (no matcher) | Yes | Yes | CC-HK-001, CC-HK-002 |
| `PreCompact` | `manual`, `auto` | No | No | CC-HK-001 |
| `SessionEnd` | `clear`, `logout`, `prompt_input_exit`, `bypass_permissions_disabled`, `other` | No | No | CC-HK-001 |
| `Setup` | (documented in code) | - | - | CC-HK-001 |
| `TeammateIdle` | (no matcher) | No | No | CC-HK-001 |
| `TaskCompleted` | (no matcher) | No | No | CC-HK-001 |

### Complete Hook Handler Fields

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `type` | string | Yes | `command`, `prompt`, `agent` | none | CC-HK-005 | Partial (only validates command/prompt) |
| `command` | string | For `command` type | shell command string | none | CC-HK-006 | No |
| `prompt` | string | For `prompt`/`agent` type | prompt text with optional `$ARGUMENTS` | none | CC-HK-007 | Partial |
| `timeout` | number | No | positive integer (seconds) | 600 (command), 30 (prompt), 60 (agent) | CC-HK-010, CC-HK-011 | No |
| `matcher` | string | Depends on event | regex pattern for filtering | `*` (all) | CC-HK-003, CC-HK-004 | No |
| `async` | boolean | No | `true`/`false` (command type only) | `false` | (none) | YES |
| `once` | boolean | No | `true`/`false` (skills only) | `false` | (none) | YES |
| `statusMessage` | string | No | Custom spinner message | none | (none) | LOW |
| `model` | string | No | Model for prompt/agent hooks | fast model | (none) | YES |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CC-HK-013: async on non-command hook | HIGH | `async: true` is only valid on `type: "command"` hooks | Yes - remove async |
| CC-HK-014: once outside skill/agent frontmatter | MEDIUM | `once` is only meaningful in skill/agent frontmatter hooks | Yes - remove once |
| CC-HK-015: model on command hook | MEDIUM | `model` field is only valid on `prompt` and `agent` hook types | Yes - remove model |
| CC-HK-016: Invalid hook type "agent" | HIGH | Validate that `type: "agent"` is recognized (new handler type) | No |
| CC-HK-017: prompt hook missing $ARGUMENTS | MEDIUM | Prompt hooks should reference `$ARGUMENTS` to receive event data | Yes - append |
| CC-HK-018: matcher on UserPromptSubmit/Stop | LOW | Matchers on UserPromptSubmit and Stop are silently ignored | Yes - remove matcher |

---

## 4. Claude Code Plugins (.claude-plugin/plugin.json)

### Complete Plugin Manifest Schema

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `name` | string | Yes (if manifest exists) | kebab-case, no spaces | directory name | CC-PL-004, CC-PL-005 | No |
| `description` | string | No | Free-form text | none | CC-PL-004 | No |
| `version` | string | No | semver format (MAJOR.MINOR.PATCH) | none | CC-PL-003 | No |
| `author` | object | No | `{ name, email?, url? }` | none | (none) | LOW |
| `homepage` | string | No | URL | none | (none) | LOW |
| `repository` | string | No | URL | none | (none) | LOW |
| `license` | string | No | License identifier | none | (none) | LOW |
| `keywords` | array | No | Array of strings | none | (none) | LOW |
| `commands` | string/array | No | Paths to command files/directories | `commands/` | (none) | YES |
| `agents` | string/array | No | Paths to agent files | `agents/` | (none) | YES |
| `skills` | string/array | No | Paths to skill directories | `skills/` | (none) | YES |
| `hooks` | string/array/object | No | Hook config paths or inline config | `hooks/hooks.json` | (none) | YES |
| `mcpServers` | string/array/object | No | MCP config paths or inline config | `.mcp.json` | (none) | YES |
| `outputStyles` | string/array | No | Style file paths | none | (none) | LOW |
| `lspServers` | string/array/object | No | LSP server configurations | `.lsp.json` | (none) | YES |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CC-PL-007: Invalid component path | HIGH | Paths in `commands`, `agents`, `skills`, `hooks` must be relative and start with `./` | Yes - prepend `./` |
| CC-PL-008: Component inside .claude-plugin | HIGH | Detect skills/agents/hooks directories inside `.claude-plugin/` (common mistake) | Yes - suggest move |
| CC-PL-009: Invalid author object | MEDIUM | If `author` present, `author.name` must be non-empty string | No |
| CC-PL-010: Invalid homepage URL | MEDIUM | If `homepage` present, must be valid URL | No |

---

## 5. Claude Code Memory (CLAUDE.md / .claude/rules/*.md)

### CLAUDE.md Features

| Feature | Syntax | agnix Coverage |
|---------|--------|----------------|
| File imports | `@path/to/file` | CC-MEM-001, CC-MEM-002, CC-MEM-003 |
| Recursive loading | Walks directory tree | (implicit) |
| Max import depth | 5 hops | CC-MEM-003 |
| Token limit | ~1500 tokens (~6000 chars) | CC-MEM-009 |

### .claude/rules/*.md Frontmatter

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `paths` | array of strings | No | Glob patterns (e.g., `src/**/*.ts`) | none (unconditional) | (none) | YES |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CC-MEM-011: Invalid paths glob in rules | HIGH | Validate glob patterns in `.claude/rules/*.md` frontmatter `paths` field | No |
| CC-MEM-012: Rules file unknown frontmatter key | MEDIUM | Only `paths` is a known key in `.claude/rules/*.md` frontmatter | Yes - remove unknown |

---

## 6. Claude Code Settings (settings.json)

### Key Settings Fields Not Currently Validated

| Field | Type | Description | Validation Opportunity |
|-------|------|-------------|----------------------|
| `permissions.defaultMode` | string | `default`, `acceptEdits`, `dontAsk`, `bypassPermissions`, `plan` | Validate enum values |
| `sandbox.enabled` | boolean | Enable sandbox mode | Type validation |
| `sandbox.network.allowedDomains` | array | Domain allowlist | Validate domain format |
| `model` | string | Override model | Validate against known models |
| `language` | string | Response language | (low priority) |
| `teammateMode` | string | `auto`, `in-process`, `tmux` | Validate enum values |
| `enabledPlugins` | object | Map of plugin names to booleans | Cross-reference with installed plugins |

---

## 7. Codex CLI (S-tier)

### Configuration File: `~/.codex/config.yaml` (or `.json`)

| Field | Type | Valid Values | Default | agnix Coverage |
|-------|------|-------------|---------|----------------|
| `model` | string | Any OpenAI API-compatible model | `o4-mini` | None |
| `approvalMode` | string | `suggest`, `auto-edit`, `full-auto` | `suggest` | None |
| `fullAutoErrorMode` | string | `ask-user`, `ignore-and-continue` | `ask-user` | None |
| `notify` | boolean | `true`/`false` | `true` | None |
| `providers` | object | `{ name, baseURL, envKey }` per provider | none | None |
| `history.maxSize` | number | positive integer | none | None |
| `history.saveHistory` | boolean | `true`/`false` | none | None |
| `history.sensitivePatterns` | array | regex patterns | none | None |

### AGENTS.md Discovery

Codex CLI merges AGENTS.md files hierarchically:
1. `~/.codex/AGENTS.md` (personal)
2. `AGENTS.md` at repository root (project)
3. `AGENTS.md` in current working directory (sub-folder)

Also supports `AGENTS.override.md` at any level. No frontmatter schema is defined.

### Config Options

| Field | Description |
|-------|-------------|
| `project_doc_fallback_filenames` | Alternative filenames to treat as instruction files |
| `project_doc_max_bytes` | Size limit for combined instruction text (default: 32 KiB, max: 65536) |

### Potential agnix Rules (Codex CLI)

| Proposed Rule | Severity | Description |
|---------------|----------|-------------|
| CDX-001: Invalid approvalMode | HIGH | Must be `suggest`, `auto-edit`, or `full-auto` |
| CDX-002: Invalid fullAutoErrorMode | HIGH | Must be `ask-user` or `ignore-and-continue` |
| CDX-003: AGENTS.override.md in version control | MEDIUM | Override files should typically be gitignored |

---

## 8. OpenCode (S-tier)

### Configuration File: `opencode.json`

| Field | Type | Valid Values | Default | agnix Coverage |
|-------|------|-------------|---------|----------------|
| `model` | string | provider/model format (e.g., `anthropic/claude-sonnet-4-5`) | none | None |
| `small_model` | string | Same format as model | none | None |
| `provider` | object | `{ timeout, setCacheKey }` | none | None |
| `theme` | string | Theme name | none | None |
| `autoupdate` | boolean/string | `true`, `false`, `"notify"` | none | None |
| `tools` | object | Map of tool names to booleans | all enabled | None |
| `agent` | object | Custom agents with descriptions, models, tool restrictions | none | None |
| `default_agent` | string | Agent name | none | None |
| `command` | object | Custom commands with templates | none | None |
| `keybinds` | object | Keyboard shortcuts | none | None |
| `formatter` | object | `{ command, extensions }` | none | None |
| `permission` | string | `"ask"` and others | none | None |
| `instructions` | array | File paths, globs, or remote URLs | none | None |
| `compaction` | object | `{ auto, prune }` | none | None |
| `mcp` | object | MCP server configuration | none | None |
| `plugin` | array | NPM or local plugin paths | none | None |
| `server` | object | `{ port, hostname, mdns, cors }` | none | None |
| `share` | string | `"manual"`, `"auto"`, `"disabled"` | none | None |

### Variable Substitution

| Syntax | Description |
|--------|-------------|
| `{env:VARIABLE_NAME}` | Environment variable expansion |
| `{file:path/to/file}` | File content inclusion |

### Potential agnix Rules (OpenCode)

| Proposed Rule | Severity | Description |
|---------------|----------|-------------|
| OC-001: Invalid share mode | HIGH | Must be `manual`, `auto`, or `disabled` |
| OC-002: Invalid instruction path | HIGH | Paths in `instructions` must exist or be valid URLs/globs |
| OC-003: opencode.json parse error | HIGH | Must be valid JSON/JSONC |

---

## 9. GitHub Copilot (A-tier)

### File Types

| File | Location | Purpose | Frontmatter? |
|------|----------|---------|-------------|
| `copilot-instructions.md` | `.github/` | Repository-wide instructions | No |
| `*.instructions.md` | `.github/instructions/` | Path-scoped instructions | Yes |
| `copilot-setup-steps.yml` | `.github/workflows/` | Agent environment setup | N/A (YAML workflow) |
| `AGENTS.md` | Anywhere in repo | Agent instructions | No |

### Scoped Instruction Frontmatter (.github/instructions/*.instructions.md)

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `applyTo` | string | Yes | Glob patterns (e.g., `**/*.ts`, `src/**/*.py`) | none | COP-002, COP-003 | No |
| `excludeAgent` | string | No | `"code-review"`, `"coding-agent"` | none | (none) | YES |

### copilot-setup-steps.yml Fields

| Field | Type | Description |
|-------|------|-------------|
| `steps` | array | Workflow steps for environment setup |
| `permissions` | object | Access permissions (e.g., `contents: read`) |
| `runs-on` | string | Runner environment |
| `services` | object | Service containers |
| `snapshot` | object | Environment state capture |
| `timeout-minutes` | number | Max execution time (max: 59) |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| COP-005: Invalid excludeAgent value | HIGH | Must be `code-review` or `coding-agent` | Yes - suggest valid |
| COP-006: copilot-instructions.md too long | MEDIUM | Repository-wide instructions should be under ~2 pages (~4000 chars) | No |
| COP-007: Missing applyTo in scoped instruction | HIGH | Already covered by COP-002, but could improve error message | No |

---

## 10. Cline (A-tier)

### File Discovery Hierarchy

| File/Folder | Format | Priority | Frontmatter? |
|-------------|--------|----------|-------------|
| `.clinerules/` | Folder with `.md` files | Highest | Yes |
| `.clinerules` | Single file | High | No |
| `.cursor/rules/` | Folder with `.mdc` files | Medium (fallback) | Yes (Cursor format) |
| `.windsurf/rules` | Folder with `.md` files | Medium (fallback) | Yes |
| `AGENTS.md` | Universal standard | Always (searches subdirs) | No |

### .clinerules/*.md Frontmatter

| Field | Type | Required | Valid Values | Default | agnix Coverage | Gap? |
|-------|------|----------|-------------|---------|----------------|------|
| `paths` | array of strings | No | Glob patterns | none (applies to all) | (none) | YES |

### Global Rules Locations

| Platform | Path |
|----------|------|
| Windows | `Documents\Cline\Rules` |
| macOS | `~/Documents/Cline/Rules` |
| Linux/WSL | `~/Documents/Cline/Rules` or `~/Cline/Rules` |

### Ordering

Files in `.clinerules/` folder combine into one ruleset. Numeric prefixes (e.g., `01-`, `02-`) control loading order.

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CLN-001: Empty clinerules file | HIGH | .clinerules file/folder must have non-empty content | No |
| CLN-002: Invalid paths glob in clinerules | HIGH | Validate glob patterns in frontmatter `paths` field | No |
| CLN-003: Unknown frontmatter key in clinerules | MEDIUM | Only `paths` is documented as valid | Yes - remove unknown |

---

## 11. Cursor (A-tier)

### .mdc File Frontmatter

| Field | Type | Required | Valid Values | Default | agnix Rule | Gap? |
|-------|------|----------|-------------|---------|------------|------|
| `description` | string | No | Free-form text describing the rule's purpose | none | CUR-005 (unknown keys) | No |
| `globs` | string/array | No | Single glob or array of globs | none | CUR-004 | No |
| `alwaysApply` | boolean | No | `true` / `false` | `false` | CUR-005 (unknown keys) | Partial |

### Rule Types (controlled by frontmatter combinations)

| Type | Frontmatter | Behavior |
|------|-------------|----------|
| Always | `alwaysApply: true` | Applied to every chat session |
| Auto-attached | `globs` specified | Applied when files match patterns |
| Agent-requested | `description` only | Agent decides relevance from description |
| Manual | None of the above | Invoked via `@rule-name` |

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| CUR-007: alwaysApply with globs | MEDIUM | When `alwaysApply: true`, `globs` is redundant (rule applies regardless) | Yes - remove globs |
| CUR-008: Invalid alwaysApply type | HIGH | Must be boolean, not string "true"/"false" | Yes - convert to bool |
| CUR-009: Missing description for agent-requested rule | MEDIUM | If no `alwaysApply` and no `globs`, `description` is strongly recommended for agent relevance | Yes - add placeholder |

---

## 12. MCP Configuration (.mcp.json)

### MCP Server Entry Schema

| Field | Type | Required | Description | agnix Coverage |
|-------|------|----------|-------------|----------------|
| `type` | string | No | `stdio`, `http`, `sse` | (none) |
| `command` | string | For stdio | Executable path | (none) |
| `args` | array | No | Command arguments | (none) |
| `env` | object | No | Environment variables | (none) |
| `url` | string | For http/sse | Server URL | (none) |
| `headers` | object | No | HTTP headers | (none) |
| `cwd` | string | No | Working directory | (none) |
| `oauth` | object | No | `{ clientId, callbackPort }` | (none) |

### Environment Variable Expansion

Supported syntax in `.mcp.json`:
- `${VAR}` - expands to env var value
- `${VAR:-default}` - expands with default fallback

### New Rule Opportunities

| Proposed Rule | Severity | Description | Auto-Fix? |
|---------------|----------|-------------|-----------|
| MCP-009: Missing command for stdio server | HIGH | stdio type requires `command` field | No |
| MCP-010: Missing url for http/sse server | HIGH | http/sse type requires `url` field | No |
| MCP-011: Invalid MCP server type | HIGH | Must be `stdio`, `http`, or `sse` | Yes - suggest closest |
| MCP-012: Deprecated SSE transport | MEDIUM | SSE is deprecated; suggest HTTP instead | Yes - change to http |

---

## Summary of Gaps and Opportunities

### By Tool - Potential New Rules

| Tool | Current Rules | Proposed New Rules | Coverage Gap |
|------|--------------|-------------------|-------------|
| Claude Code Skills | CC-SK-001 to CC-SK-009 | CC-SK-010 to CC-SK-015 (6 new) | hooks, user-invocable, argument-hint validation |
| Claude Code Agents | CC-AG-001 to CC-AG-007 | CC-AG-008 to CC-AG-013 (6 new) | memory, tool name validation, hooks |
| Claude Code Hooks | CC-HK-001 to CC-HK-012 | CC-HK-013 to CC-HK-018 (6 new) | async, once, agent type, model field |
| Claude Code Plugins | CC-PL-001 to CC-PL-006 | CC-PL-007 to CC-PL-010 (4 new) | path validation, component placement |
| Claude Code Memory | CC-MEM-001 to CC-MEM-010 | CC-MEM-011 to CC-MEM-012 (2 new) | .claude/rules paths frontmatter |
| GitHub Copilot | COP-001 to COP-004 | COP-005 to COP-006 (2 new) | excludeAgent, length limits |
| Cursor | CUR-001 to CUR-006 | CUR-007 to CUR-009 (3 new) | alwaysApply+globs conflict, type validation |
| Cline | (none) | CLN-001 to CLN-003 (3 new) | New tool coverage |
| MCP | MCP-001 to MCP-008 | MCP-009 to MCP-012 (4 new) | .mcp.json schema validation |
| Codex CLI | (none) | CDX-001 to CDX-003 (3 new) | New tool coverage |
| OpenCode | (none) | OC-001 to OC-003 (3 new) | New tool coverage |

### Priority Ranking for New Rules

**P0 (High Impact, High Confidence)**:
1. CC-SK-011: user-invocable + disable-model-invocation conflict
2. CC-AG-008: Invalid memory scope validation
3. CC-HK-013: async on non-command hook
4. CC-HK-016: Validate `type: "agent"` hook handler
5. COP-005: Invalid excludeAgent value
6. MCP-009/MCP-010: Missing required fields for MCP server types

**P1 (Medium Impact)**:
7. CC-SK-014/CC-SK-015: Boolean type validation for skill fields
8. CC-AG-009/CC-AG-010: Tool name validation in agent frontmatter
9. CUR-007: alwaysApply with redundant globs
10. CC-MEM-011: Glob validation in .claude/rules paths
11. CLN-001/CLN-002: Cline clinerules validation

**P2 (Nice to Have)**:
12. CC-SK-012: argument-hint without $ARGUMENTS
13. CC-PL-007/CC-PL-008: Plugin path validation
14. CC-HK-018: Matcher on UserPromptSubmit/Stop
15. CDX-001/CDX-002: Codex CLI config validation
16. OC-001/OC-002: OpenCode config validation

### Auto-Fix Opportunities Summary

| Rule | Fix Description | Safety |
|------|----------------|--------|
| CC-SK-011 | Remove `disable-model-invocation` when `user-invocable: false` | HIGH |
| CC-SK-014/015 | Convert string "true"/"false" to boolean | HIGH |
| CC-HK-013 | Remove `async` from non-command hooks | HIGH |
| CC-HK-015 | Remove `model` from command hooks | HIGH |
| CC-HK-018 | Remove `matcher` from UserPromptSubmit/Stop | HIGH |
| CUR-007 | Remove `globs` when `alwaysApply: true` | MEDIUM |
| CUR-008 | Convert string to boolean for alwaysApply | HIGH |
| MCP-012 | Change `sse` to `http` transport | MEDIUM |
| CC-PL-007 | Prepend `./` to relative paths | HIGH |
| COP-005 | Suggest valid excludeAgent value | HIGH |
| CC-AG-008 | Suggest valid memory scope | HIGH |
| MCP-011 | Suggest valid MCP server type | HIGH |

---

## Cross-Platform Field Comparison

### Frontmatter Fields Across Tools

| Field Name | Claude Skills | Claude Agents | Copilot | Cursor | Cline | Agent Skills Spec |
|-----------|---------------|---------------|---------|--------|-------|-------------------|
| `name` | Yes (optional) | Yes (required) | No | No | No | Yes (required) |
| `description` | Yes (recommended) | Yes (required) | No | Yes | No | Yes (required) |
| `model` | Yes | Yes | No | No | No | No |
| `tools` | `allowed-tools` | `tools` | No | No | No | `allowed-tools` |
| `globs`/`paths` | No | No | `applyTo` | `globs` | `paths` | No |
| `alwaysApply` | No | No | No | Yes | No | No |
| `context` | Yes (`fork`) | No | No | No | No | No |
| `hooks` | Yes | Yes | No | No | No | No |
| `memory` | No | Yes | No | No | No | No |
| `license` | Yes | No | No | No | No | Yes |
| `compatibility` | Yes | No | No | No | No | Yes |
| `metadata` | Yes | No | No | No | No | Yes |
| `disable-model-invocation` | Yes | No | No | No | No | No |
| `user-invocable` | Yes | No | No | No | No | No |
| `permissionMode` | No | Yes | No | No | No | No |
| `skills` | No | Yes | No | No | No | No |
| `excludeAgent` | No | No | Yes | No | No | No |

---

## Further Reading

| Resource | Type | Why Recommended |
|----------|------|-----------------|
| [Claude Code Skills Docs](https://code.claude.com/docs/en/skills) | Official Docs | Canonical reference for skill frontmatter |
| [Claude Code Subagents Docs](https://code.claude.com/docs/en/sub-agents) | Official Docs | Complete agent configuration reference |
| [Claude Code Hooks Reference](https://code.claude.com/docs/en/hooks) | Official Docs | Full hook events, fields, and behavior |
| [Claude Code Plugins Reference](https://code.claude.com/docs/en/plugins-reference) | Official Docs | Complete plugin manifest schema |
| [Claude Code Settings](https://code.claude.com/docs/en/settings) | Official Docs | All settings.json fields |
| [Claude Code Memory](https://code.claude.com/docs/en/memory) | Official Docs | CLAUDE.md and .claude/rules/ reference |
| [Agent Skills Specification](https://agentskills.io/specification) | Spec | Cross-tool skill standard |
| [GitHub Copilot Custom Instructions](https://docs.github.com/en/copilot/customizing-copilot/adding-repository-custom-instructions-for-github-copilot) | Vendor Docs | Copilot instruction file schema |
| [Cursor Rules](https://cursor.com/docs/context/rules) | Vendor Docs | .mdc file format reference |
| [Cline Rules](https://docs.cline.bot/features/cline-rules) | Vendor Docs | .clinerules format reference |
| [OpenCode Configuration](https://opencode.ai/docs/config) | Vendor Docs | opencode.json schema reference |
| [Codex CLI Documentation](https://developers.openai.com/codex/guides/agents-md) | Vendor Docs | AGENTS.md discovery and config.yaml |
| [MCP Specification](https://modelcontextprotocol.io/specification) | Spec | Model Context Protocol reference |

---

*This guide was synthesized from 42 sources. See `resources/agent-config-optional-fields-sources.json` for full source list.*
