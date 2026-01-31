# agnix Knowledge Base

> Comprehensive reference for agent config validation patterns and best practices

## Structure

```
knowledge-base/
├── README.md                       # This file
├── PATTERNS-CATALOG.md             # 70 detection patterns (production-tested)
├── agent-docs/                     # Architectural references
│   ├── AI-AGENT-ARCHITECTURE-RESEARCH.md
│   ├── CLAUDE-CODE-REFERENCE.md
│   ├── CODEX-REFERENCE.md
│   ├── OPENCODE-REFERENCE.md
│   ├── PROMPT-ENGINEERING-REFERENCE.md
│   ├── FUNCTION-CALLING-TOOL-USE-REFERENCE.md
│   ├── MULTI-AGENT-SYSTEMS-REFERENCE.md
│   ├── LLM-INSTRUCTION-FOLLOWING-RELIABILITY.md
│   ├── CONTEXT-OPTIMIZATION-REFERENCE.md
│   └── KNOWLEDGE-LIBRARY.md
├── patterns/                       # Extracted from enhance skills
│   └── (to be populated)
└── schemas/                        # JSON schemas
    └── (to be populated)
```

## Quick Reference

### Pattern Statistics

| Category | Patterns | High Certainty | Auto-Fixable |
|----------|----------|----------------|--------------|
| **Skills** | 25 | 18 | 5 |
| **Hooks** | 18 | 14 | 3 |
| **CLAUDE.md** | 15 | 10 | 2 |
| **Agents** | 12 | 9 | 3 |
| **Total** | **70** | **51** | **13** |

### Key Documents

#### For Implementation
- **PATTERNS-CATALOG.md** - All 70 patterns with detection logic and examples
- **../SPEC.md** - Project specification and roadmap
- **../tests/fixtures/** - Real test cases from awesome-slash

#### For Context
- **agent-docs/AI-AGENT-ARCHITECTURE-RESEARCH.md** - Agent system design patterns
- **agent-docs/MULTI-AGENT-SYSTEMS-REFERENCE.md** - Multi-agent coordination
- **agent-docs/PROMPT-ENGINEERING-REFERENCE.md** - Prompt best practices
- **agent-docs/LLM-INSTRUCTION-FOLLOWING-RELIABILITY.md** - How to write effective instructions

### Standards Referenced

| Standard | URL | Coverage |
|----------|-----|----------|
| Agent Skills | https://agentskills.io | Complete |
| MCP | https://modelcontextprotocol.io | Partial |
| Claude Code | https://code.claude.com/docs | Complete |
| A2A | https://google.github.io/A2A | Future |

## Pattern Index

### Skills Patterns (25)

**HIGH Certainty (18)**:
1. Missing frontmatter
2. Invalid name format
3. Missing trigger phrase
4. Dangerous auto-invocation
5. Unrestricted Bash
6. Invalid model value
7. Invalid context value
8. Missing description
9. Description too long (>1024 chars)
10. Name too long (>64 chars)
11. Name starts/ends with hyphen
12. Consecutive hyphens in name
13. Compatibility too long (>500 chars)
14. Context without agent
15. Agent without context
16. Invalid agent type
17. Tool restriction mismatch
18. Unknown tool names

**MEDIUM Certainty (5)**:
19. Oversized content (>500 lines)
20. Too many injections (>3)
21. Missing argument hint
22. Context/agent mismatch
23. Redundant chain-of-thought

**LOW Certainty (2)**:
24. Vague descriptions
25. Missing version field

### Hooks Patterns (18)

**HIGH Certainty (14)**:
1. Invalid hook event name
2. Prompt hook on wrong event
3. Missing matcher for tool-specific events
4. Dangerous command patterns
5. Missing script file
6. Invalid exit code handling
7. Missing type field
8. Unknown hook type
9. Matcher on non-tool event
10. Invalid JSON output schema
11. Missing command field
12. Missing prompt field
13. Unrestricted bash in hooks
14. Security vulnerabilities

**MEDIUM Certainty (3)**:
15. No timeout specified
16. Timeout too long (>60s)
17. Complex regex matchers

**LOW Certainty (1)**:
18. Hook could be simplified

### CLAUDE.md Patterns (15)

**HIGH Certainty (10)**:
1. Missing critical rules section
2. Negative instructions without positives
3. Weak constraint language
4. Critical content in middle section
5. Invalid file references (@imports)
6. Invalid command references (npm scripts)
7. Generic instructions
8. Broken markdown links
9. Missing architecture section
10. Missing commands section

**MEDIUM Certainty (4)**:
11. Token count exceeded (>1500)
12. README duplication (>40%)
13. Long prose blocks (>5 sentences)
14. Deep nesting (>3 levels)

**LOW Certainty (1)**:
15. XML structure recommended

### Agent Patterns (12)

**HIGH Certainty (9)**:
1. Missing frontmatter
2. Invalid model value
3. Invalid permission mode
4. Referenced skill not found
5. Tool/disallowedTools conflict
6. Invalid tool names
7. Missing name field
8. Missing description field
9. Hooks structure invalid

**MEDIUM Certainty (2)**:
10. Skills without context fork
11. Permission mode too permissive

**LOW Certainty (1)**:
12. Agent could be split

## Detection Pseudocode

### High-Level Flow

```rust
pub fn validate_file(path: &Path, config: &LintConfig) -> Vec<Diagnostic> {
    let content = fs::read_to_string(path)?;
    let file_type = detect_file_type(path, &content);

    let mut diagnostics = Vec::new();

    match file_type {
        FileType::Skill => {
            diagnostics.extend(validate_skill(path, &content, config));
        }
        FileType::Agent => {
            diagnostics.extend(validate_agent(path, &content, config));
        }
        FileType::ClaudeMemory => {
            diagnostics.extend(validate_claude_md(path, &content, config));
        }
        FileType::Hooks => {
            diagnostics.extend(validate_hooks(path, &content, config));
        }
        FileType::Plugin => {
            diagnostics.extend(validate_plugin(path, &content, config));
        }
        FileType::Unknown => {}
    }

    // Universal checks
    diagnostics.extend(validate_xml_balance(path, &content));
    diagnostics.extend(validate_imports(path, &content));

    diagnostics
}
```

### File Type Detection

```rust
pub fn detect_file_type(path: &Path, content: &str) -> FileType {
    let filename = path.file_name().unwrap().to_str().unwrap();

    match filename {
        "SKILL.md" => FileType::Skill,
        "CLAUDE.md" | "AGENTS.md" => FileType::ClaudeMemory,
        "plugin.json" => FileType::Plugin,
        "settings.json" | "settings.local.json" if has_hooks_key(content) => FileType::Hooks,
        _ if path.to_str().unwrap().contains("/agents/") && filename.ends_with(".md") => FileType::Agent,
        _ => FileType::Unknown,
    }
}
```

## Usage Examples

### Validate Skills

```rust
use agnix_core::rules::skill::SkillValidator;
use agnix_core::rules::Validator;

let validator = SkillValidator;
let diagnostics = validator.validate(
    Path::new(".claude/skills/review/SKILL.md"),
    &content,
    &config
);

for diag in diagnostics {
    println!("{:?}", diag);
}
```

### Validate CLAUDE.md

```rust
use agnix_core::rules::claude_md::ClaudeMdValidator;

let validator = ClaudeMdValidator;
let diagnostics = validator.validate(
    Path::new("CLAUDE.md"),
    &content,
    &config
);
```

## Test Fixtures

### Location

```
../tests/fixtures/
├── valid/                    # Should pass validation
│   ├── agent-complete-valid.md
│   └── prompt-complete-valid.md
├── invalid/                  # Should fail validation
│   ├── agent-missing-frontmatter.md
│   ├── agent-missing-role.md
│   ├── agent-unrestricted-bash.md
│   ├── prompt-aggressive-emphasis.md
│   ├── prompt-missing-examples.md
│   ├── prompt-missing-output-format.md
│   └── prompt-vague-instructions.md
└── fixes/                    # Before/after pairs
    ├── *-before.md
    └── *-after.md
```

### Coverage

- ✅ Skills: 8 test cases
- ✅ Agents: 3 test cases
- ✅ Prompts: 5 test cases
- ⏳ Hooks: 0 test cases (to add)
- ⏳ CLAUDE.md: 0 test cases (to add)
- ⏳ Plugins: 0 test cases (to add)

## Contributing Patterns

### Pattern Template

```markdown
### N. Pattern Name [CERTAINTY]

**Pattern**: Brief description
**Detection**:
```rust
// Pseudocode or actual implementation
```

**Examples**:
- ❌ Bad example
- ✅ Good example

**Auto-fix** (if applicable): Description
```

### Adding New Patterns

1. Add to PATTERNS-CATALOG.md
2. Create test fixture in tests/fixtures/
3. Implement detection in crates/agnix-core/src/rules/
4. Add to pattern statistics table
5. Update this README

## References

### External Documentation

- [Agent Skills Spec](https://agentskills.io/specification)
- [MCP Specification](https://modelcontextprotocol.io/specification)
- [Claude Code Docs](https://code.claude.com/docs)
- [Anthropic Prompt Engineering](https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering)

### Internal Documentation

- [Project Spec](../SPEC.md)
- [README](../README.md)
- [Cargo Workspace](../Cargo.toml)

## Maintenance

### Updating Patterns

When enhance skills in awesome-slash are updated:

1. Copy latest skill files
2. Re-extract patterns
3. Update PATTERNS-CATALOG.md
4. Update test fixtures
5. Bump version in Cargo.toml

### Pattern Quality

All patterns should meet:
- Clear detection logic
- Real-world examples
- Known false positive rate
- Test coverage

---

**Last Updated**: 2026-01-31
**Pattern Source**: awesome-slash v3.6.1 (production-tested)
**Coverage**: 70 patterns, 13 auto-fixes
