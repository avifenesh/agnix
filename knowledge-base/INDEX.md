# agnix Knowledge Base - Master Index

> 100 validation rules across 15 categories, sourced from 75+ references

---

## Quick Navigation

| What You Need | Start Here |
|---------------|------------|
| **Implement validator** | [VALIDATION-RULES.md](./VALIDATION-RULES.md) - 100 rules with detection logic |
| **Understand a standard** | [standards/](#standards) - HARD-RULES files |
| **Learn best practices** | [standards/](#standards) - OPINIONS files |
| **Find patterns** | [PATTERNS-CATALOG.md](./PATTERNS-CATALOG.md) - 70 patterns from awesome-slash |
| **Get platform context** | [agent-docs/](#agent-docs) - 10 reference docs |
| **Cross-platform support** | [standards/multi-platform-HARD-RULES.md](./standards/multi-platform-HARD-RULES.md) |
| **Track tools/research** | [RESEARCH-TRACKING.md](./RESEARCH-TRACKING.md) - Tool inventory and monitoring |
| **Monthly review** | [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) - Review checklist and completed reviews |

---

## Document Structure

```
knowledge-base/
├── INDEX.md                        # This file
├── README.md                       # Detailed navigation guide
├── VALIDATION-RULES.md             # ⭐ Master validation reference (100 rules)
├── PATTERNS-CATALOG.md             # 70 production-tested patterns
├── RESEARCH-TRACKING.md            # Tool inventory and monitoring process
├── MONTHLY-REVIEW.md               # Monthly review checklist and history
│
├── standards/                      # HARD-RULES and OPINIONS by topic
│   ├── README.md                   # Standards navigation
│   ├── RESEARCH-SUMMARY.md         # Research methodology
│   │
│   ├── agent-skills-HARD-RULES.md  # 19KB - Non-negotiable requirements
│   ├── agent-skills-OPINIONS.md    # 36KB - Best practices
│   │
│   ├── mcp-HARD-RULES.md           # 33KB - Protocol requirements
│   ├── mcp-OPINIONS.md             # 36KB - Design patterns
│   │
│   ├── claude-code-HARD-RULES.md   # 34KB - Technical specs
│   ├── claude-code-OPINIONS.md     # 40KB - Usage patterns
│   │
│   ├── multi-platform-HARD-RULES.md # 15KB - Compatibility matrix
│   ├── multi-platform-OPINIONS.md  # 27KB - Cross-platform tips
│   │
│   ├── prompt-engineering-HARD-RULES.md  # 16KB - Research-backed
│   └── prompt-engineering-OPINIONS.md    # 21KB - Best practices
│
└── agent-docs/                     # 10 reference docs (mixed sources)
    ├── CLAUDE-CODE-REFERENCE.md
    ├── CODEX-REFERENCE.md
    ├── OPENCODE-REFERENCE.md
    ├── PROMPT-ENGINEERING-REFERENCE.md
    ├── FUNCTION-CALLING-TOOL-USE-REFERENCE.md
    ├── LLM-INSTRUCTION-FOLLOWING-RELIABILITY.md
    ├── CONTEXT-OPTIMIZATION-REFERENCE.md
    └── KNOWLEDGE-LIBRARY.md
```

---

## Coverage Summary

### Standards Researched

| Standard | Sources | HARD RULES | OPINIONS | Rules Extracted |
|----------|---------|------------|----------|-----------------|
| **Agent Skills** | 12 | 19KB | 36KB | 15 rules |
| **MCP** | 11 | 33KB | 36KB | 8 rules |
| **Claude Code** | 10 | 34KB | 40KB | 42 rules |
| **Multi-Platform** | 15 | 15KB | 27KB | 6 rules |
| **Prompt Eng** | 15 | 16KB | 21KB | 4 rules |
| **AGENTS.md** | 5 | - | - | 6 rules |
| **Cursor** | 2 | - | - | 6 rules |
| **awesome-slash** | 12 | - | - | 70 patterns |
| **Total** | **75+** | **117KB** | **160KB** | **100 rules** |

### Validation Rules by Category

| Category | Rules | HIGH | MEDIUM | LOW | Auto-Fix |
|----------|-------|------|--------|-----|----------|
| Agent Skills | 16 | 14 | 2 | 0 | 6 |
| Claude Skills | 9 | 8 | 1 | 0 | 3 |
| Claude Hooks | 12 | 11 | 1 | 0 | 2 |
| Claude Agents | 7 | 7 | 0 | 0 | 1 |
| Claude Memory | 10 | 7 | 3 | 0 | 2 |
| AGENTS.md | 6 | 1 | 5 | 0 | 2 |
| Claude Plugins | 6 | 6 | 0 | 0 | 1 |
| GitHub Copilot | 4 | 3 | 1 | 0 | 0 |
| MCP | 8 | 7 | 1 | 0 | 1 |
| XML | 3 | 3 | 0 | 0 | 1 |
| References | 2 | 2 | 0 | 0 | 0 |
| Prompt Eng | 4 | 0 | 4 | 0 | 1 |
| Cross-Platform | 6 | 4 | 2 | 0 | 0 |
| Cursor | 6 | 3 | 3 | 0 | 1 |
| Version Awareness | 1 | 0 | 0 | 1 | 0 |
| **TOTAL** | **100** | **76** | **23** | **1** | **22** |

---

## Key Findings

### Research-Backed Rules (Empirical Evidence)

1. **Lost in the Middle** (Liu et al., 2023) → PE-001
   - Critical content in middle loses recall
   - Position at start or end

2. **Positive Framing** (Multiple studies) → CC-MEM-006
   - "Do X" outperforms "Don't do Y"
   - Measured improvement in compliance

3. **Constraint Strength** (Instruction-following research) → CC-MEM-007
   - MUST > imperatives > should > try to
   - Weak language reduces compliance

4. **Claude Long-Context** (Anthropic, 2023) → PE-001
   - Single prompt change: 27% → 98% accuracy
   - "Here is the most relevant sentence" dramatically improved retrieval

### Surprising Discoveries

1. **AGENTS.md is supported by multiple tools** - but not universal (XP-002)
2. **Prompt hooks restricted** - Only Stop/SubagentStop supported (CC-HK-002)
3. **Windows paths break skills** - Must use `/` even on Windows (AS-014)
4. **No defense against prompt injection** - Unsolved problem (MCP security)

---

## Usage Guide

### For Implementation

**Start here**: [VALIDATION-RULES.md](./VALIDATION-RULES.md)
- 100 rules with rule IDs (AS-001, CC-HK-001, etc.)
- Detection pseudocode
- Auto-fix implementations
- Priority matrix (P0/P1/P2)

**Reference**: [standards/](./standards/)
- HARD-RULES: What will break
- OPINIONS: What's better

### For Understanding Platforms

**Claude Code**:
- [claude-code-HARD-RULES.md](./standards/claude-code-HARD-RULES.md) - Complete technical specs
- [claude-code-OPINIONS.md](./standards/claude-code-OPINIONS.md) - Design patterns

**MCP**:
- [mcp-HARD-RULES.md](./standards/mcp-HARD-RULES.md) - Protocol compliance
- [mcp-OPINIONS.md](./standards/mcp-OPINIONS.md) - Tool design patterns

**Multi-Platform**:
- [multi-platform-HARD-RULES.md](./standards/multi-platform-HARD-RULES.md) - Compatibility matrix
- [multi-platform-OPINIONS.md](./standards/multi-platform-OPINIONS.md) - Best practices

### For Context

**Prompt Engineering**: [prompt-engineering-HARD-RULES.md](./standards/prompt-engineering-HARD-RULES.md)

---

## Validation Implementation Checklist

### Week 3: Core Rules (P0)

Parser Setup:
- [x] YAML frontmatter parser
- [x] JSON config parser
- [x] Markdown @import extractor
- [x] XML tag parser

Skills Validation:
- [x] AS-001: Frontmatter exists
- [x] AS-002: Name field exists
- [x] AS-003: Description field exists
- [x] AS-004: Name format valid
- [x] AS-010: Trigger phrase present
- [x] CC-SK-001: Model value valid
- [x] CC-SK-006: Dangerous auto-invocation
- [x] CC-SK-007: Unrestricted Bash

Hooks Validation:
- [x] CC-HK-001: Valid event name
- [x] CC-HK-002: Prompt hook restriction
- [x] CC-HK-003: Matcher required
- [x] CC-HK-005: Type field exists
- [x] CC-HK-006: Missing command field
- [x] CC-HK-007: Missing prompt field
- [x] CC-HK-008: Script file not found
- [x] CC-HK-009: Dangerous command pattern

Memory Validation:
- [x] CC-MEM-001: Import paths exist
- [x] CC-MEM-005: Generic instructions

XML & References:
- [x] XML-001: Tag balance
- [x] REF-001: Import resolution

### Week 4: Quality Rules (P1)

Skills:
- [x] AS-011 through AS-015
- [x] CC-SK-002 through CC-SK-005

Memory:
- [x] CC-MEM-006 through CC-MEM-010

Agents:
- [x] CC-AG-001 through CC-AG-006

Plugins:
- [x] CC-PL-001 through CC-PL-005

### Week 5-6: Advanced (P2)

- [x] MCP protocol validation
- [x] Prompt engineering analysis
- [x] Cross-platform compatibility

---

## Maintenance

### Update Triggers

Update knowledge base when:
- Agent Skills spec updates
- MCP protocol version change
- Claude Code releases new features
- New research published on prompt engineering
- awesome-slash enhance patterns updated

### Update Process

1. Re-run research agents on updated sources
2. Extract new HARD-RULES
3. Update VALIDATION-RULES.md with new rule IDs
4. Add test fixtures for new patterns
5. Implement new validators
6. Update this index

### Monthly Review

Follow the structured monthly review process in [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) to check for upstream changes across all monitored tools and research sources. The review cadence is the 1st week of each month, with per-tier checklists ensuring S-tier tools get the most attention.

---

## Statistics

```
Total Documents:       31 files
Total Lines:          18,900 lines
Total Size:           650KB
Standards Covered:     5 (Agent Skills, MCP, Claude Code, Multi-Platform, Prompt Eng)
Sources Consulted:    75+ (specs, docs, research papers, repos)
Research Agents:       5 (10+ sources each)
Validation Rules:     100 rules
Auto-Fixable Rules:   22 rules
Test Fixtures:        116 files
Platforms Analyzed:   9 (Claude Code, Codex CLI, OpenCode, Copilot, Cursor, Cline, Roo-Cline, Continue.dev, Aider)
```

---

**Status**: Knowledge base integrated with the active validation engine
**Next**: Keep `rules.json` and `VALIDATION-RULES.md` synchronized as rules evolve
**Confidence**: HIGH - all rules sourced from official specs or research
