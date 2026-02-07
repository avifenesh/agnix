# agnix Technical Reference

> Linter for agent configs. 139 rules across 17 categories.


## What agnix Validates

| Type | Files | Rules |
|------|-------|-------|
| Skills | SKILL.md | 31 |
| Hooks | settings.json | 18 |
| Memory (Claude Code) | CLAUDE.md, CLAUDE.local.md, .claude/rules/*.md | 12 |
| Instructions (Cross-Tool) | AGENTS.md, AGENTS.local.md, AGENTS.override.md | 6 |
| Agents | agents/*.md | 13 |
| Plugins | plugin.json | 10 |
| Prompt Engineering | CLAUDE.md, AGENTS.md | 4 |
| Cross-Platform | AGENTS.md | 6 |
| MCP | tool definitions | 12 |
| XML | all .md files | 3 |
| References | @imports | 2 |
| GitHub Copilot | .github/copilot-instructions.md, .github/instructions/*.instructions.md | 6 |
| Cursor Project Rules | .cursor/rules/*.mdc, .cursorrules | 9 |
| Cline | .clinerules, .clinerules/*.md | 3 |
| OpenCode | opencode.json | 3 |
| Version Awareness | .agnix.toml | 1 |

## Architecture

```
agnix/
├── crates/
│   ├── agnix-rules/    # Rule metadata generated from rules.json
│   ├── agnix-core/     # Validation library
│   │   ├── parsers/    # YAML, JSON, Markdown
│   │   ├── schemas/    # Type definitions
│   │   └── rules/      # Validators
│   ├── agnix-cli/      # CLI binary
│   ├── agnix-lsp/      # LSP server
│   └── agnix-mcp/      # MCP server
├── editors/            # Neovim, VS Code, JetBrains, Zed integrations
├── knowledge-base/     # 139 rules documented

├── scripts/            # Build/dev automation scripts
├── website/            # Docusaurus documentation website
└── tests/fixtures/     # Test cases
```

### Validation Pipeline

The validation process follows these steps:

1. **Directory Walking** (sequential) - Uses `ignore` crate to traverse directories
2. **File Collection** - Gathers all relevant file paths with exclusion filtering
3. **Parallel Validation** - Processes files in parallel using rayon
4. **Result Sorting** - Deterministic ordering by severity (errors first) then file path

This architecture ensures fast validation on large projects while maintaining consistent, reproducible output.

## Security

agnix implements defense-in-depth security measures:

| Feature | Implementation | Default |
|---------|----------------|---------|
| Symlink rejection | `file_utils::safe_read_file()` | Always on |
| File size limits | `DEFAULT_MAX_FILE_SIZE = 1 MiB` | Always on |
| File count limits | `max_files_to_validate` | 10,000 |
| ReDoS protection | `MAX_REGEX_INPUT_SIZE = 64 KB` | Always on |
| Path traversal detection | `normalize_join()` in imports validator | Always on |

See [knowledge-base/SECURITY-MODEL.md](knowledge-base/SECURITY-MODEL.md) for complete threat model.

## Rule Reference

All rules in `knowledge-base/VALIDATION-RULES.md`

**Rule ID Format:** `[CATEGORY]-[NUMBER]`
- `AS-nnn`: Agent Skills (agentskills.io)
- `CC-SK-nnn`: Claude Code Skills
- `CC-HK-nnn`: Claude Code Hooks
- `CC-MEM-nnn`: Claude Code Memory
- `AGM-nnn`: AGENTS.md (cross-tool instructions)
- `CC-AG-nnn`: Claude Code Agents
- `COP-nnn`: GitHub Copilot Instructions
- `CLN-nnn`: Cline Rules
- `OC-nnn`: OpenCode configuration
- `CC-PL-nnn`: Claude Code Plugins
- `MCP-nnn`: MCP protocol
- `XML-nnn`: XML validation
- `REF-nnn`: @import/reference validation
- `PE-nnn`: Prompt engineering
- `XP-nnn`: Cross-platform compatibility
- `VER-nnn`: Version awareness

## Key Rules

| ID | Severity | Description |
|----|----------|-------------|
| AS-001 | ERROR | YAML frontmatter required |
| AS-004 | ERROR | Name must be kebab-case |
| AS-010 | WARN | Missing trigger phrase |
| CC-SK-001 | ERROR | Invalid model value |
| CC-SK-002 | ERROR | Invalid context value |
| CC-SK-003 | ERROR | Context 'fork' requires agent field |
| CC-SK-004 | ERROR | Agent field requires context: fork |
| CC-SK-005 | ERROR | Invalid agent type |
| CC-SK-006 | ERROR | Dangerous skill without safety flag |
| CC-SK-007 | WARN | Unrestricted Bash access |
| CC-SK-008 | ERROR | Unknown tool name |
| CC-SK-009 | WARN | Too many dynamic injections |
| CC-HK-001 | ERROR | Invalid hook event |
| CC-HK-006 | ERROR | Missing command field |
| CC-HK-007 | ERROR | Missing prompt field |
| CC-HK-008 | ERROR | Script file not found |
| CC-HK-009 | WARN | Dangerous command pattern |
| CC-MEM-004 | WARN | Invalid command reference |
| CC-MEM-005 | WARN | Generic instruction detected |
| AGM-003 | WARN | Character limit exceeded (12000 chars) |
| AGM-005 | WARN | Platform features without guard |
| PE-001 | WARN | Critical content in middle |
| PE-002 | WARN | Chain-of-thought on simple task |
| CC-AG-001 | ERROR | Missing agent name field |
| CC-AG-002 | ERROR | Missing agent description field |
| CC-AG-003 | ERROR | Invalid model value |
| CC-AG-004 | ERROR | Invalid permission mode |
| CC-AG-005 | ERROR | Referenced skill not found |
| CC-AG-006 | ERROR | Tool/disallowed conflict |
| CC-PL-001 | ERROR | Plugin manifest not in .claude-plugin/ |
| CC-PL-002 | ERROR | Components inside .claude-plugin/ |
| CC-PL-003 | ERROR | Invalid semver format |
| CC-PL-004 | ERROR | Missing required plugin field |
| CC-PL-005 | ERROR | Empty plugin name |
| XML-001 | ERROR | Unclosed XML tag |
## CLI

```bash
agnix .                    # Validate directory
agnix --strict .           # Warnings = errors
agnix --target claude-code # Claude-specific rules
agnix --fix .              # Apply automatic fixes
agnix --dry-run .          # Preview fixes without modifying files
agnix --fix-safe .         # Only apply safe (HIGH certainty) fixes
agnix --format json .      # JSON output for programmatic consumption
agnix --format sarif .     # SARIF 2.1.0 output for CI/CD
agnix --locale es .        # Spanish output
agnix --list-locales       # Show available locales
```

## Config (.agnix.toml)

```toml
severity = "Warning"
target = "Generic"  # Options: Generic, ClaudeCode, Cursor, Codex
locale = "en"       # Options: en, es, zh-CN
tools = ["claude-code", "cursor"]  # Preferred over target

[rules]
# Category toggles - enable/disable entire rule categories
skills = true       # AS-*, CC-SK-* rules
hooks = true        # CC-HK-* rules
agents = true       # CC-AG-* rules
copilot = true      # COP-* rules
cursor = true       # CUR-* rules
cline = true        # CLN-* rules
opencode = true     # OC-* rules
memory = true       # CC-MEM-* rules
plugins = true      # CC-PL-* rules
mcp = true          # MCP-* rules
prompt_engineering = true  # PE-* rules
xml = true          # XML-* rules
imports = true      # REF-*, imports::* rules
cross_platform = true  # XP-* rules
agents_md = true       # AGM-* rules

# Legacy flags (still supported)
generic_instructions = true
frontmatter_validation = true
xml_balance = true
import_references = true

# Disable specific rules by ID
disabled_rules = []  # e.g., ["CC-AG-001", "AS-005"]

exclude = ["node_modules/**", ".git/**", "target/**"]
```

### Config Validation

agnix validates `.agnix.toml` files semantically before running validation:

- **Rule ID validation**: `disabled_rules` must match known patterns (AS-, CC-SK-, CC-HK-, CC-AG-, CC-MEM-, CC-PL-, XML-, MCP-, REF-, XP-, AGM-, COP-, CUR-, CLN-, OC-, PE-, VER-, imports::)
- **Tool validation**: `tools` array must contain valid tool names (claude-code, cursor, codex, copilot, github-copilot, cline, opencode, generic)
- **Deprecation warnings**: `mcp_protocol_version` is deprecated (use `spec_revisions.mcp_protocol`)

Warnings are displayed before validation output with suggestions for fixes.

### Target Tool Filtering

When `target` is set to a specific tool, only relevant rules run:
- **ClaudeCode** or **Generic**: All rules enabled
- **Cursor** or **Codex**: CC-* rules disabled (Claude Code specific)

### Rule Categories

| Category | Config Key | Rules | Description |
|----------|------------|-------|-------------|
| Skills | `skills` | AS-*, CC-SK-* | Agent skill validation |
| Hooks | `hooks` | CC-HK-* | Hook configuration validation |
| Agents | `agents` | CC-AG-* | Subagent validation |
| GitHub Copilot | `copilot` | COP-* | Copilot instruction validation |
| Memory | `memory` | CC-MEM-* | Memory/CLAUDE.md validation |
| Plugins | `plugins` | CC-PL-* | Plugin validation |
| MCP | `mcp` | MCP-* | MCP tool validation |
| Prompt Engineering | `prompt_engineering` | PE-* | Prompt engineering best practices |
| XML | `xml` | XML-* | XML tag balance |
| Imports | `imports` | REF-* | Import reference validation |
| Cross-Platform | `cross_platform` | XP-* | Cross-platform consistency checks |
| AGENTS.md | `agents_md` | AGM-* | AGENTS.md-specific validation |
| Cursor | `cursor` | CUR-* | Cursor project rule validation |
| Cline | `cline` | CLN-* | Cline rules validation |
| OpenCode | `opencode` | OC-* | OpenCode configuration validation |

Version awareness (`VER-*`) is always active and controlled through `tool_versions` / `spec_revisions` pins.

## Performance Characteristics

### Performance Targets

| Metric | Target | Typical |
|--------|--------|---------|
| Single file validation | < 100ms | < 10ms |
| 100-file project | < 500ms | ~200ms |
| 1000-file project | < 5s | ~2s |
| Peak memory | < 100MB | ~50MB |
| Binary size | < 10MB | ~5MB |

### Architecture Optimizations

- **Parallel validation**: Uses rayon `par_bridge()` for file processing across all CPU cores
- **Registry caching**: ValidatorRegistry is constructed once and shared (7x speedup vs per-file)
- **Import cache**: `Arc<RwLock<HashMap>>` shared across files reduces redundant @import parsing
- **Static regex patterns**: `static_regex!` macro (in `regex_util.rs`) wraps OnceLock for one-time initialization with descriptive panic messages
- **Directory walking**: Sequential via `ignore` crate (required for .gitignore compatibility)
- **Deterministic output**: Results sorted by severity then path for reproducible runs

### Release Build Optimizations

```toml
[profile.release]
lto = "fat"          # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip symbols from binary
opt-level = 3        # Maximum optimization
panic = "abort"      # Smaller binary, no unwinding
```

### Measurement Methodology

agnix uses a dual-methodology approach for performance measurement:

**CI (blocking on regression)**: iai-callgrind
- Measures CPU instruction counts (100% deterministic)
- Immune to system load, CPU frequency scaling, background processes
- Results are reproducible across CI runs with zero variance
- Blocks merge on regression above configurable threshold

**Development (fast feedback)**: Criterion
- Wall-clock timing for intuitive performance understanding
- Statistical sampling for reliable measurements
- HTML reports with historical comparison

### Running Benchmarks

```bash
# Fast feedback during development (wall-clock)
./scripts/bench.sh criterion

# Pre-PR validation (instruction counts, matches CI)
./scripts/bench.sh iai

# Check binary size breakdown
./scripts/bench.sh bloat

# Run all benchmarks
./scripts/bench.sh all
```

### Interpreting iai-callgrind Results

iai-callgrind reports several metrics:

| Metric | Description | What It Tells You |
|--------|-------------|-------------------|
| Instructions | CPU instructions executed | Primary performance metric |
| L1 Hits/Misses | Level 1 cache performance | Memory access efficiency |
| L2 Hits/Misses | Level 2 cache performance | Working set size |
| RAM Hits | Main memory accesses | Cache effectiveness |
| Estimated Cycles | Weighted cycle estimate | Overall CPU cost |

Instruction counts directly correlate with wall-clock time but without noise from:
- Background processes
- CPU frequency scaling
- VM/container overhead
- Disk I/O variance

### Platform Considerations

- **Linux**: Full support for both iai-callgrind and Criterion
- **macOS x86**: Full support for both iai-callgrind and Criterion
- **macOS ARM**: Valgrind support is experimental; use Criterion for local development
- **Windows**: No Valgrind support; use Criterion only (CI runs iai on Linux)
