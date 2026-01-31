# Claude Code - Hard Rules & Technical Specifications

> **Non-negotiable technical requirements extracted from official documentation**
>
> Last Updated: 2026-01-31
> Sources: 10+ official Claude Code documentation pages

---

## 1. HOOKS

### 1.1 Valid Hook Events (Complete List)

| Event | When It Fires | Can Block |
|-------|--------------|-----------|
| `SessionStart` | Session begins or resumes | No |
| `UserPromptSubmit` | User submits a prompt | Yes (exit 2) |
| `PreToolUse` | Before tool execution | Yes (exit 2) |
| `PermissionRequest` | When permission dialog appears | Yes |
| `PostToolUse` | After tool succeeds | No |
| `PostToolUseFailure` | After tool fails | No |
| `SubagentStart` | When spawning a subagent | No |
| `SubagentStop` | When subagent finishes | Yes |
| `Stop` | Claude finishes responding | Yes |
| `PreCompact` | Before context compaction | No |
| `Setup` | `--init`, `--init-only`, or `--maintenance` flags | No |
| `SessionEnd` | Session terminates | No |
| `Notification` | Claude Code sends notifications | No |

**CRITICAL**: These are the ONLY valid event names. Case-sensitive. Typos will fail silently.

### 1.2 Exit Code Behavior (Definitive)

| Exit Code | Behavior | stdout Processing | stderr Processing |
|-----------|----------|------------------|------------------|
| `0` | Success | Parsed as JSON if valid; plain text added to context for `UserPromptSubmit`/`SessionStart` | Shown in verbose mode only |
| `2` | **BLOCKING ERROR** | **IGNORED** (JSON not processed) | Fed back to Claude as error message |
| Other | Non-blocking error | Ignored | Shown in verbose mode with "Failed with non-blocking status code" |

**CRITICAL RULES**:
- Exit code 2 stderr format: `[command]: {stderr}`
- If JSON in stdout with exit 0, exit 2 completely ignores stdout
- For UserPromptSubmit with exit 2: prompt is erased from context

### 1.3 Exit Code 2 Per-Event Behavior

| Hook Event | What Happens |
|------------|-------------|
| `PreToolUse` | Blocks tool call, shows stderr to Claude |
| `PermissionRequest` | Denies permission, shows stderr to Claude |
| `PostToolUse` | Shows stderr to Claude (tool already ran) |
| `Notification` | N/A, shows stderr to user only |
| `UserPromptSubmit` | Blocks prompt processing, **erases prompt**, shows stderr to user only |
| `Stop` | Blocks stoppage, shows stderr to Claude |
| `SubagentStop` | Blocks stoppage, shows stderr to Claude subagent |
| `PreCompact` | N/A, shows stderr to user only |
| `Setup` | N/A, shows stderr to user only |
| `SessionStart` | N/A, shows stderr to user only |
| `SessionEnd` | N/A, shows stderr to user only |

### 1.4 Hook Input JSON Schema (Common Fields)

**Every hook receives these fields via stdin**:

```typescript
{
  session_id: string          // Required
  transcript_path: string     // Required - path to conversation JSON
  cwd: string                 // Required - current working directory
  permission_mode: string     // Required - "default", "plan", "acceptEdits", "dontAsk", "bypassPermissions"
  hook_event_name: string     // Required - the event name
  // ... event-specific fields
}
```

### 1.5 Hook Output JSON Schema

#### Common Fields (All Hooks)

```json
{
  "continue": true | false,          // Whether Claude should continue (default: true)
  "stopReason": "string",            // Message when continue=false
  "suppressOutput": true | false,    // Hide stdout from transcript (default: false)
  "systemMessage": "string"          // Optional warning to user
}
```

**CRITICAL**: `continue: false` takes precedence over any `decision: "block"` output.

#### PreToolUse Decision Control

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow" | "deny" | "ask",
    "permissionDecisionReason": "string",
    "updatedInput": {
      "field_to_modify": "new value"
    },
    "additionalContext": "string"
  }
}
```

**CRITICAL**: Deprecated fields `decision: "approve"/"block"` still work but map to `permissionDecision: "allow"/"deny"`.

#### PermissionRequest Decision Control

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow" | "deny",
      "updatedInput": { /* optional */ },
      "message": "string",           // for deny
      "interrupt": true | false      // for deny
    }
  }
}
```

#### PostToolUse Decision Control

```json
{
  "decision": "block" | undefined,
  "reason": "string",
  "hookSpecificOutput": {
    "hookEventName": "PostToolUse",
    "additionalContext": "string"
  }
}
```

#### UserPromptSubmit Decision Control

**Two ways to add context**:

1. **Plain text stdout** (simpler): Any non-JSON text with exit 0
2. **JSON format**:

```json
{
  "decision": "block" | undefined,
  "reason": "string",
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "string"
  }
}
```

**CRITICAL**: Exit code 2 with stderr blocks and erases prompt; JSON `decision: "block"` with exit 0 blocks but uses custom reason.

#### Stop/SubagentStop Decision Control

```json
{
  "decision": "block" | undefined,
  "reason": "string"  // MUST be provided when decision="block"
}
```

#### SessionStart Decision Control

```json
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "string"
  }
}
```

**Multiple hooks**: `additionalContext` values are concatenated.

#### Setup Decision Control

```json
{
  "hookSpecificOutput": {
    "hookEventName": "Setup",
    "additionalContext": "string"
  }
}
```

### 1.6 Environment Variables Available in Hooks

| Variable | Availability | Purpose |
|----------|-------------|---------|
| `$CLAUDE_PROJECT_DIR` | All hooks | Absolute path to project root |
| `$CLAUDE_ENV_FILE` | `SessionStart` and `Setup` only | File path for persisting environment variables |
| `$CLAUDE_CODE_REMOTE` | All hooks | `"true"` if remote (web), empty/unset if local CLI |
| `${CLAUDE_PLUGIN_ROOT}` | Plugin hooks only | Absolute path to plugin directory |

**CRITICAL**: `CLAUDE_ENV_FILE` is ONLY available in SessionStart and Setup hooks.

### 1.7 Prompt Hooks (ONLY for Specific Events)

**Hook Type**: `"prompt"`

**ONLY SUPPORTED FOR**: `Stop` and `SubagentStop` (and technically any event, but most useful for these)

**Response Schema** (LLM must return):

```json
{
  "ok": true | false,
  "reason": "string"  // Required when ok=false
}
```

**Configuration Example**:

```json
{
  "hooks": {
    "Stop": [{
      "hooks": [{
        "type": "prompt",
        "prompt": "Evaluate if Claude should stop: $ARGUMENTS",
        "timeout": 30
      }]
    }]
  }
}
```

**CRITICAL**: `$ARGUMENTS` placeholder inserts hook input JSON; if not present, JSON appended to prompt.

### 1.8 Hook Matcher Patterns

**Applicable to**: `PreToolUse`, `PermissionRequest`, `PostToolUse`

| Pattern | Matches |
|---------|---------|
| `Bash` | Exact match: Bash tool only |
| `*` or `""` or blank | All tools |
| `Edit\|Write` | Regex: Edit OR Write |
| `Notebook.*` | Regex: Notebook prefix |
| `mcp__memory__.*` | MCP tools from memory server |
| `mcp__.*__write.*` | All MCP write operations |

**Case-sensitive**: `bash` ≠ `Bash`

### 1.9 Hook Configuration Location Precedence

1. **Managed policy** (highest priority, cannot be overridden)
2. **Settings files** (user, project, local)
3. **Plugin hooks** (merged with settings)
4. **Agent/Skill frontmatter** (scoped to component lifecycle)

**CRITICAL**: Plugin hooks with `allowManagedHooksOnly: true` blocks all user/project/plugin hooks.

### 1.10 Hook Execution Rules

- **Timeout**: 60 seconds default, configurable per command
- **Parallelization**: All matching hooks run in parallel
- **Deduplication**: Identical commands deduplicated automatically
- **Environment**: Runs in cwd with Claude Code's environment

---

## 2. SUBAGENTS

### 2.1 Frontmatter Required Fields

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `name` | **YES** | string | Unique identifier (lowercase, hyphens only) |
| `description` | **YES** | string | When Claude should delegate to this subagent |
| `tools` | No | string or array | Tools subagent can use (inherits all if omitted) |
| `disallowedTools` | No | string or array | Tools to deny |
| `model` | No | string | `sonnet`, `opus`, `haiku`, or `inherit` (default: `inherit`) |
| `permissionMode` | No | string | See [2.2](#22-valid-permissionmode-values) |
| `skills` | No | array | Skills to preload at startup |
| `hooks` | No | object | Lifecycle hooks (see [2.3](#23-valid-hook-events-in-subagent-frontmatter)) |

**CRITICAL**: `name` MUST be lowercase letters, numbers, and hyphens only (max 64 characters).

### 2.2 Valid permissionMode Values

| Value | Behavior |
|-------|----------|
| `default` | Standard permission checking with prompts |
| `acceptEdits` | Auto-accept file edits |
| `dontAsk` | Auto-deny permission prompts (explicitly allowed tools still work) |
| `bypassPermissions` | Skip ALL permission checks |
| `plan` | Plan mode (read-only exploration) |

**CRITICAL**: If parent uses `bypassPermissions`, subagent CANNOT override it.

### 2.3 Valid Hook Events in Subagent Frontmatter

**Supported**: `PreToolUse`, `PostToolUse`, `Stop`

**Format**:

```yaml
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/validate.sh"
  PostToolUse:
    - matcher: "Edit|Write"
      hooks:
        - type: command
          command: "./scripts/lint.sh"
  Stop:
    - hooks:
        - type: prompt
          prompt: "Should this subagent continue?"
```

**CRITICAL**: `Stop` hooks in frontmatter are automatically converted to `SubagentStop` events.

### 2.4 Tool Specification Format

**Three formats allowed**:

```yaml
# Format 1: Comma-separated string
tools: "Read, Grep, Glob, Bash"

# Format 2: Array of strings
tools:
  - Read
  - Grep
  - Glob
  - Bash

# Format 3: Omit (inherits all tools)
# tools: (not specified)
```

**CRITICAL**: Tool names are case-sensitive. `read` ≠ `Read`.

### 2.5 Built-in Subagents

| Agent | Model | Tools | Purpose |
|-------|-------|-------|---------|
| `Explore` | Haiku | Read-only | Fast codebase exploration |
| `Plan` | Inherits | Read-only | Research during plan mode |
| `general-purpose` | Inherits | All tools | Complex multi-step tasks |
| `Bash` | Inherits | Bash | Terminal commands in separate context |

**CRITICAL**: Subagents CANNOT spawn other subagents (no nesting).

### 2.6 Subagent File Locations & Precedence

| Priority | Location | Scope |
|----------|----------|-------|
| 1 (highest) | `--agents` CLI flag | Current session only |
| 2 | `.claude/agents/` | Current project |
| 3 | `~/.claude/agents/` | All your projects |
| 4 (lowest) | Plugin's `agents/` | Where plugin enabled |

**CRITICAL**: When duplicate names exist, higher priority wins.

### 2.7 CLI Flag Format

```bash
claude --agents '{
  "code-reviewer": {
    "description": "Expert code reviewer",
    "prompt": "You are a senior code reviewer...",
    "tools": ["Read", "Grep", "Glob"],
    "model": "sonnet"
  }
}'
```

**CRITICAL**: CLI format uses `prompt` field (not markdown body); equals markdown body in file-based agents.

### 2.8 Subagent Context Management

- **Fresh context**: Each invocation creates new instance
- **Resume**: Ask Claude to resume previous subagent by ID
- **Transcript location**: `~/.claude/projects/{project}/{sessionId}/subagents/agent-{agentId}.jsonl`
- **Auto-compaction**: Triggers at ~95% capacity (override with `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`)

**CRITICAL**: Subagent transcripts persist independently of main conversation compaction.

---

## 3. CLAUDE.md & AGENTS.md

### 3.1 File Hierarchy & Precedence (Highest to Lowest)

| Priority | Location | Platform | Shared |
|----------|----------|----------|--------|
| 1 | Managed policy (`/Library/Application Support/ClaudeCode/CLAUDE.md`, `/etc/claude-code/CLAUDE.md`, `C:\Program Files\ClaudeCode\CLAUDE.md`) | macOS, Linux, Windows | Organization |
| 2 | Project root (`./CLAUDE.md` or `./.claude/CLAUDE.md`) | All | Team (via git) |
| 3 | Project rules (`./.claude/rules/*.md`) | All | Team (via git) |
| 4 | User memory (`~/.claude/CLAUDE.md`) | All | Personal |
| 5 | Project local (`./CLAUDE.local.md`) | All | Personal (gitignored) |

**CRITICAL**: CLAUDE.local.md is automatically added to .gitignore.

### 3.2 @import Syntax Rules

**Format**: `@path/to/file` (either inline or on its own line)

**Examples**:

```markdown
See @README for project overview.

Additional context: @docs/conventions.md

# Individual Preferences
- @~/.claude/my-project-instructions.md
```

**CRITICAL RULES**:
- Both relative and absolute paths allowed
- Imports NOT evaluated in code spans: `` `@anthropic-ai/claude-code` ``
- Imports NOT evaluated in code blocks
- Max recursion depth: 5 hops
- Symlinks are followed during copy

### 3.3 Lookup Behavior

**Recursive discovery**:
1. Start in `cwd`
2. Recurse UP to (but not including) root `/`
3. Read any `CLAUDE.md` or `CLAUDE.local.md` found
4. Also discover CLAUDE.md in subtrees (loaded only when reading those files)

**Additional directories**: Use `--add-dir` flag; set `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1` to load their CLAUDE.md files.

### 3.4 Cross-Platform Compatibility

Claude Code documents `CLAUDE.md`/`CLAUDE.local.md` and `.claude/rules/*.md` for memory/rules and does not document `AGENTS.md` as an alternate filename.

If you need cross-tool compatibility, keep a separate `AGENTS.md` (with compatible content) for tools that use it (Codex CLI, OpenCode, Cursor, Cline, Copilot coding agent).

---

## 4. PLUGINS

### 4.1 Plugin Manifest Schema (plugin.json)

**Location**: `.claude-plugin/plugin.json` (MUST be in this directory)

**Required Fields**:

```json
{
  "name": "plugin-name"  // REQUIRED: kebab-case, no spaces
}
```

**Optional Fields**:

```json
{
  "version": "1.2.0",                    // Semver
  "description": "Brief description",
  "author": {
    "name": "Author Name",
    "email": "author@example.com",
    "url": "https://github.com/author"
  },
  "homepage": "https://docs.example.com",
  "repository": "https://github.com/author/plugin",
  "license": "MIT",
  "keywords": ["keyword1", "keyword2"],
  "commands": ["./custom/cmd.md"],       // Additional command paths
  "agents": "./custom/agents/",          // Additional agent paths
  "skills": "./custom/skills/",          // Additional skill paths
  "hooks": "./config/hooks.json",        // Hook config path or inline object
  "mcpServers": "./mcp-config.json",     // MCP config path or inline object
  "lspServers": "./.lsp.json",           // LSP config path or inline object
  "outputStyles": "./styles/"            // Output style paths
}
```

**CRITICAL**: All paths MUST be relative and start with `./`

### 4.2 Directory Structure Rules

```
plugin-root/
├── .claude-plugin/          # MUST contain plugin.json ONLY
│   └── plugin.json          # REQUIRED
├── commands/                # Default command location (optional)
├── agents/                  # Default agent location (optional)
├── skills/                  # Default skill location (optional)
├── hooks/                   # Hook configurations (optional)
│   └── hooks.json
├── .mcp.json                # MCP server definitions (optional)
├── .lsp.json                # LSP server configurations (optional)
└── scripts/                 # Supporting scripts (optional)
```

**CRITICAL**: DO NOT put `commands/`, `agents/`, `skills/`, or `hooks/` inside `.claude-plugin/`. Only `plugin.json` goes there.

### 4.3 Semver Requirements

**Format**: `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`

**Rules**:
- MAJOR: Breaking changes (incompatible API)
- MINOR: New features (backward-compatible)
- PATCH: Bug fixes (backward-compatible)
- Pre-release: `2.0.0-beta.1`, `1.0.0-alpha.3`

**CRITICAL**: Claude Code validates semver format; invalid versions cause loading failure.

### 4.4 Plugin Environment Variables

| Variable | Purpose | Availability |
|----------|---------|--------------|
| `${CLAUDE_PLUGIN_ROOT}` | Absolute path to plugin directory | All plugin contexts |
| `${CLAUDE_PROJECT_DIR}` | Project root directory | Same as project hooks |

**CRITICAL**: Use `${CLAUDE_PLUGIN_ROOT}` for all plugin-relative paths to ensure portability.

### 4.5 Installation Scopes

| Scope | Settings File | Use Case |
|-------|--------------|----------|
| `user` | `~/.claude/settings.json` | Personal plugins (default) |
| `project` | `.claude/settings.json` | Team plugins (shared via git) |
| `local` | `.claude/settings.local.json` | Project-specific (gitignored) |
| `managed` | `managed-settings.json` | Organization policies (read-only) |

**CLI Syntax**:

```bash
claude plugin install formatter@marketplace --scope project
```

### 4.6 Plugin Caching Behavior

**CRITICAL RULES**:
- Plugins are COPIED to cache directory, not used in-place
- For marketplace plugins: entire `source` path directory is copied recursively
- For `.claude-plugin/plugin.json`: implicit root directory is copied
- Symlinks are followed during copy
- Path traversal (`../shared`) will NOT work after installation
- Max-depth: resolved recursively

**Workaround for external dependencies**:
1. Use symlinks inside plugin directory (will be copied)
2. Or set marketplace `source` to parent directory with full manifest in marketplace entry

### 4.7 Manifest Validation Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `Invalid JSON syntax` | JSON parse error | Check commas, quotes, braces |
| `name: Required` | Missing required field | Add `name` field |
| `Plugin has a corrupt manifest file` | JSON syntax error | Validate JSON |
| `Plugin directory not found` | Wrong `source` path | Fix marketplace `source` |
| `Plugin has conflicting manifests` | Duplicate definitions | Set `strict: true` or remove duplicates |

---

## 5. SKILLS

### 5.1 File Structure Rules

**Location**: `skills/<skill-name>/SKILL.md` (directory structure) OR `commands/<skill-name>.md` (legacy flat file)

**CRITICAL**: Directory name becomes skill name (lowercase, hyphens only, max 64 characters).

**Structure**:

```
skills/
└── my-skill/
    ├── SKILL.md           # REQUIRED: entrypoint
    ├── reference.md       # Optional: detailed docs
    ├── examples/          # Optional: examples
    └── scripts/           # Optional: executable scripts
```

### 5.2 Frontmatter Schema

**All fields optional** (defaults shown):

```yaml
---
name: my-skill                      # Default: directory name
description: What this skill does   # Default: first paragraph of content
argument-hint: "[arg1] [arg2]"      # Default: none
disable-model-invocation: false     # Default: false (Claude CAN invoke)
user-invocable: true                # Default: true (shown in / menu)
allowed-tools: Read, Grep           # Default: none (no restrictions)
model: sonnet                       # Default: none (uses session model)
context: fork                       # Default: none (inline execution)
agent: Explore                      # Default: "general-purpose" when context=fork
hooks:                              # Default: none
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./validate.sh"
once: true                          # Default: false (skill hooks only)
---

Skill instructions here...
```

**CRITICAL**: `name` must match regex `^[a-z0-9-]+$` (lowercase, hyphens only, max 64 chars).

### 5.3 String Substitutions

| Variable | Replaced With |
|----------|--------------|
| `$ARGUMENTS` | All arguments passed to skill |
| `$ARGUMENTS[N]` | Nth argument (0-based index) |
| `$N` | Shorthand for `$ARGUMENTS[N]` |
| `${CLAUDE_SESSION_ID}` | Current session ID |
| `!`command`` | **Preprocessing**: Command output (runs before Claude sees content) |

**CRITICAL**: If `$ARGUMENTS` not present, Claude Code appends `ARGUMENTS: <value>` to skill content.

### 5.4 Invocation Control Matrix

| Frontmatter | You Can Invoke | Claude Can Invoke | When Loaded |
|-------------|----------------|-------------------|-------------|
| (default) | Yes | Yes | Description in context, full skill on invoke |
| `disable-model-invocation: true` | Yes | No | Description NOT in context, full skill on invoke |
| `user-invocable: false` | No | Yes | Description in context, full skill on invoke |

**CRITICAL**: Subagents with `skills` field preload full content at startup (different behavior).

### 5.5 Context Execution Modes

| `context` Value | Behavior |
|----------------|----------|
| (omitted) | Inline: skill content added to current conversation |
| `fork` | Subagent: skill content becomes task prompt in isolated context |

**When `context: fork`**:
- `agent` field determines subagent type (default: `general-purpose`)
- Skill content = task prompt (must be actionable, not just guidelines)
- No access to main conversation history
- Also loads: CLAUDE.md

### 5.6 Skill Tool Restrictions

**Via frontmatter**:

```yaml
allowed-tools: Read, Grep, Glob
```

**Via permission rules**:

```json
{
  "permissions": {
    "deny": ["Skill", "Skill(deploy *)"],
    "allow": ["Skill(commit)", "Skill(review-pr *)"]
  }
}
```

**CRITICAL**: `user-invocable: false` only hides from menu; use `disable-model-invocation: true` to block programmatic invocation.

### 5.7 Skill Discovery Paths

| Priority | Location | Applies To |
|----------|----------|-----------|
| 1 | Enterprise managed | All users in org |
| 2 | `~/.claude/skills/` | All your projects |
| 3 | `.claude/skills/` | This project |
| 4 | Plugin `skills/` | Where plugin enabled |
| 5 | Nested `.claude/skills/` | Subdirectory-specific (automatic discovery) |

**Legacy**: `.claude/commands/` works identically but lacks supporting file structure.

---

## 6. SETTINGS

### 6.1 Configuration File Precedence (Highest to Lowest)

| Priority | Location | Shared | Override |
|----------|----------|--------|----------|
| 1 | Managed policy (`managed-settings.json`) | Yes (IT) | Cannot be overridden |
| 2 | CLI arguments (e.g., `--disallowedTools`) | No | Session only |
| 3 | `.claude/settings.local.json` | No (gitignored) | Overrides project/user |
| 4 | `.claude/settings.json` | Yes (git) | Overrides user |
| 5 | `~/.claude/settings.json` | No | Baseline |

### 6.2 System Paths (Managed Settings)

| Platform | Path |
|----------|------|
| macOS | `/Library/Application Support/ClaudeCode/managed-settings.json` |
| Linux/WSL | `/etc/claude-code/managed-settings.json` |
| Windows | `C:\Program Files\ClaudeCode\managed-settings.json` |

### 6.3 Permission Rule Syntax (DEFINITIVE)

**Format**: `Tool(specifier)` or `Tool` (all uses)

**Wildcards**:
- `*` matches any characters
- Can appear anywhere: `Bash(npm *)`, `Bash(*test*)`, `Bash(*.sh)`
- Spacing matters: `Bash(ls *)` matches `ls -la` but NOT `lsof`

**Evaluation Order (First Match Wins)**:
1. `deny` (highest priority)
2. `ask`
3. `allow` (lowest priority)

**Examples**:

```json
{
  "permissions": {
    "allow": [
      "Bash(npm run lint)",
      "Read(~/.zshrc)"
    ],
    "ask": [
      "Bash(git push *)"
    ],
    "deny": [
      "Bash(curl *)",
      "Read(./.env)",
      "WebFetch(domain:*.example.com)"
    ]
  }
}
```

### 6.4 Sandbox Settings (Schema)

```json
{
  "sandbox": {
    "enabled": true,                          // macOS, Linux, WSL2 only
    "autoAllowBashIfSandboxed": true,        // Auto-approve sandboxed bash
    "excludedCommands": ["git", "docker"],   // Commands running outside sandbox
    "allowUnsandboxedCommands": false,       // Allow escape hatch
    "network": {
      "allowUnixSockets": ["~/.ssh/agent"],  // Unix sockets in sandbox
      "allowLocalBinding": true,             // Bind to localhost (macOS only)
      "httpProxyPort": 8080,                 // HTTP proxy port
      "socksProxyPort": 8081                 // SOCKS5 proxy port
    },
    "enableWeakerNestedSandbox": false       // Weaker sandbox for Docker (Linux/WSL2)
  }
}
```

**CRITICAL**: Sandbox only works on macOS, Linux, and WSL2. Windows native is unsupported.

### 6.5 MCP Server Configuration Schema

**.mcp.json format**:

```json
{
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-memory"],
      "env": {
        "API_KEY": "${API_KEY}"
      },
      "disabled": false
    }
  }
}
```

**Managed MCP policies** (`managed-mcp.json`):

```json
{
  "allowedMcpServers": [
    {"serverName": "github"},
    {"serverName": "memory"}
  ],
  "deniedMcpServers": [
    {"serverName": "filesystem"}
  ]
}
```

**CRITICAL**: Managed policies use `serverName` (not `command` match); deny overrides allow.

### 6.6 LSP Server Configuration Schema

**.lsp.json format**:

```json
{
  "language-id": {
    "command": "language-server-binary",        // REQUIRED: must be in PATH
    "args": ["serve"],                          // Optional
    "extensionToLanguage": {                    // REQUIRED
      ".ext": "language-id"
    },
    "transport": "stdio",                       // Default: "stdio"; also: "socket"
    "env": {},                                  // Optional
    "initializationOptions": {},                // Optional
    "settings": {},                             // Optional
    "workspaceFolder": "/path/to/workspace",    // Optional
    "startupTimeout": 10000,                    // Optional (milliseconds)
    "shutdownTimeout": 5000,                    // Optional (milliseconds)
    "restartOnCrash": true,                     // Optional
    "maxRestarts": 3                            // Optional
  }
}
```

**CRITICAL**: User must install LSP binary separately; plugin only configures connection.

### 6.7 Attribution Settings (Schema)

```json
{
  "attribution": {
    "commit": "Generated with AI\n\nCo-Authored-By: AI <ai@example.com>",
    "pr": ""
  }
}
```

**CRITICAL**: Empty string (`""`) hides attribution; omitting field uses default.

**Default**:

```
Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### 6.8 Plugin Marketplace Schema (Managed)

**strictKnownMarketplaces format** (managed settings only):

```json
{
  "strictKnownMarketplaces": {
    "marketplace-name": {
      "source": {
        "source": "github" | "git" | "url" | "npm" | "file" | "directory" | "hostPattern",
        // ... source-specific fields
      }
    }
  }
}
```

**Source Types**:

1. **GitHub**:

```json
{
  "source": "github",
  "repo": "owner/repo",
  "ref": "main",
  "path": "marketplace"
}
```

2. **Git**:

```json
{
  "source": "git",
  "url": "https://gitlab.example.com/plugins.git",
  "ref": "production",
  "path": "approved"
}
```

3. **URL**:

```json
{
  "source": "url",
  "url": "https://plugins.example.com/marketplace.json",
  "headers": { "Authorization": "Bearer ${TOKEN}" }
}
```

4. **NPM**:

```json
{
  "source": "npm",
  "package": "@company/plugins"
}
```

5. **File**:

```json
{
  "source": "file",
  "path": "/usr/local/share/claude/marketplace.json"
}
```

6. **Directory**:

```json
{
  "source": "directory",
  "path": "/usr/local/share/claude/plugins"
}
```

7. **Host Pattern**:

```json
{
  "source": "hostPattern",
  "hostPattern": "^github\\.example\\.com$"
}
```

---

## 7. ENVIRONMENT VARIABLES (Critical Subset)

### 7.1 Hook-Specific Variables

| Variable | Hook Availability | Type | Purpose |
|----------|------------------|------|---------|
| `CLAUDE_PROJECT_DIR` | All hooks | string | Absolute path to project root |
| `CLAUDE_ENV_FILE` | `SessionStart`, `Setup` only | string | File path for persisting env vars |
| `CLAUDE_CODE_REMOTE` | All hooks | string | `"true"` if remote, empty if local |
| `CLAUDE_PLUGIN_ROOT` | Plugin hooks only | string | Absolute path to plugin directory |

### 7.2 Critical Execution Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `CLAUDE_CODE_MAX_OUTPUT_TOKENS` | 32,000 | Max output tokens (max: 64,000) |
| `MAX_THINKING_TOKENS` | 31,999 | Extended thinking budget |
| `BASH_DEFAULT_TIMEOUT_MS` | 120,000 | Default bash timeout |
| `BASH_MAX_TIMEOUT_MS` | 600,000 | Maximum bash timeout |
| `BASH_MAX_OUTPUT_LENGTH` | 30,000 | Max bash output characters |
| `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` | 95 | Auto-compaction trigger (1-100) |
| `SLASH_COMMAND_TOOL_CHAR_BUDGET` | 15,000 | Skill metadata char limit |

### 7.3 Disable Flags (Boolean)

**Set to `1` to disable**:

| Variable | Disables |
|----------|----------|
| `DISABLE_TELEMETRY` | Statsig telemetry |
| `DISABLE_ERROR_REPORTING` | Sentry error reporting |
| `DISABLE_AUTOUPDATER` | Auto-updates |
| `DISABLE_PROMPT_CACHING` | Prompt caching (all models) |
| `DISABLE_PROMPT_CACHING_HAIKU` | Haiku caching only |
| `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` | Background tasks |
| `CLAUDE_CODE_DISABLE_TERMINAL_TITLE` | Terminal title updates |
| `DISABLE_INSTALLATION_CHECKS` | Installation warnings |

### 7.4 Model Override Variables

| Variable | Overrides |
|----------|-----------|
| `ANTHROPIC_MODEL` | Model setting |
| `ANTHROPIC_DEFAULT_HAIKU_MODEL` | Haiku-class model |
| `ANTHROPIC_DEFAULT_SONNET_MODEL` | Sonnet-class model |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` | Opus-class model |
| `CLAUDE_CODE_SUBAGENT_MODEL` | Subagent model |

---

## 8. TOOL-SPECIFIC RULES

### 8.1 Available Tool Names (Complete List)

**Built-in Tools**:

```
Bash, Glob, Grep, Read, Edit, Write, NotebookEdit, WebFetch, WebSearch,
AskUserQuestion, StatusBarMessageTool, TaskOutput, Task, Skill
```

**MCP Tools**: `mcp__<server>__<tool>` (e.g., `mcp__memory__create_entities`)

**CRITICAL**: Tool names are case-sensitive in all contexts (hooks, permissions, frontmatter).

### 8.2 Task Tool (Subagent Invocation)

**Permission syntax**: `Task(agent-name)`

**Deny subagents**:

```json
{
  "permissions": {
    "deny": ["Task(Explore)", "Task(my-custom-agent)"]
  }
}
```

**Or via CLI**:

```bash
claude --disallowedTools "Task(Explore)"
```

### 8.3 Skill Tool (Skill Invocation)

**Permission syntax**: `Skill(skill-name)` or `Skill(skill-name *)`

**Examples**:

```json
{
  "permissions": {
    "deny": ["Skill", "Skill(deploy *)"],
    "allow": ["Skill(commit)", "Skill(review-pr *)"]
  }
}
```

**CRITICAL**: `Skill` denies ALL skills; `Skill(name)` denies specific skill; `Skill(name *)` denies with any arguments.

---

## 9. CLI FLAGS (Critical Subset)

### 9.1 Hook & Permission Flags

| Flag | Type | Purpose |
|------|------|---------|
| `--disallowedTools` | string | Comma-separated tool deny list |
| `--dangerously-skip-permissions` | boolean | Skip ALL permissions (blocked if `disableBypassPermissionsMode: "disable"`) |

### 9.2 Subagent Flags

| Flag | Type | Purpose |
|------|------|---------|
| `--agents` | JSON string | Define subagents for session |
| `--agent <name>` | string | Start with specific subagent |

**Example**:

```bash
claude --agents '{"reviewer": {"description": "Code reviewer", "prompt": "...", "tools": ["Read"]}}'
```

### 9.3 Plugin Flags

| Flag | Type | Purpose |
|------|------|---------|
| `--plugin-dir <path>` | string | Load plugin from directory (repeatable) |

**Example**:

```bash
claude --plugin-dir ./my-plugin --plugin-dir ./another-plugin
```

### 9.4 Session Flags

| Flag | Type | Purpose |
|------|------|---------|
| `--init` | boolean | Run Setup hooks with trigger="init" |
| `--init-only` | boolean | Run Setup hooks then exit |
| `--maintenance` | boolean | Run Setup hooks with trigger="maintenance" |
| `--resume` | boolean | Resume last session |
| `--continue` | boolean | Resume last session (alias) |

---

## 10. VALIDATION RULES

### 10.1 Name Validation Regex

**Skill/Agent names**: `^[a-z0-9-]+$` (lowercase, hyphens only, max 64 chars)

**Plugin names**: `^[a-z0-9-]+$` (kebab-case, no spaces)

**CRITICAL**: Names violating this regex cause loading failure.

### 10.2 Semver Validation

**Format**: `MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`

**Regex**: `^\d+\.\d+\.\d+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$`

**Examples**:
- Valid: `1.0.0`, `2.3.4-beta.1`, `1.0.0+20130313144700`
- Invalid: `1.0`, `v1.0.0`, `1.0.0-`, `1.0.0+`

### 10.3 JSON Schema Validation

**All settings/manifest JSON must**:
- Parse without syntax errors
- Match expected schema
- Use correct types (string, number, boolean, array, object)

**Common errors**:
- Missing commas between fields
- Trailing commas (invalid JSON)
- Unquoted strings
- Missing required fields

---

## APPENDIX A: GLOSSARY OF EXACT TERMS

| Term | Exact Meaning | Context |
|------|--------------|---------|
| "blocking error" | Exit code 2 | Hooks |
| "kebab-case" | lowercase-with-hyphens | Names |
| "case-sensitive" | Exact match required | Tool names, event names |
| "precedence" | First match wins (deny > ask > allow) | Permissions |
| "matcher" | Pattern for tool matching | Hook configuration |
| "frontmatter" | YAML block between `---` markers | Skills, agents |
| "inline" | Runs in main conversation context | Skills (`context` omitted) |
| "fork" | Runs in isolated subagent context | Skills (`context: fork`) |
| "preload" | Full content injected at startup | Subagents with `skills` field |
| "inherit" | Use parent/session value | Model, tools |
| "managed" | Organization-level, cannot override | Settings, policies |

---

## APPENDIX B: ERROR MESSAGE REFERENCE

| Error | Meaning | Fix |
|-------|---------|-----|
| `Invalid JSON syntax` | JSON parse error | Check commas, quotes, braces |
| `name: Required` | Missing required field | Add `name` field |
| `Executable not found in $PATH` | LSP binary missing | Install language server |
| `Plugin directory not found` | Wrong `source` path | Fix marketplace `source` |
| `Plugin has conflicting manifests` | Duplicate definitions | Set `strict: true` or remove duplicates |
| `Hook command timed out` | Exceeded timeout | Increase `timeout` field or optimize script |
| `Permission denied` | Tool blocked by deny rule | Check `permissions.deny` |

---

**END OF HARD RULES DOCUMENT**
