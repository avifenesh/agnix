# agnix Knowledge Base - Master Index

> 320KB knowledge, 75+ sources, 5 research agents, 74 validation rules

---

## Quick Navigation

| What You Need | Start Here |
|---------------|------------|
| **Implement validator** | [VALIDATION-RULES.md](./VALIDATION-RULES.md) - 74 rules with detection logic |
| **Understand a standard** | [standards/](#standards) - HARD-RULES files |
| **Learn best practices** | [standards/](#standards) - OPINIONS files |
| **Find patterns** | [PATTERNS-CATALOG.md](./PATTERNS-CATALOG.md) - 70 patterns from awesome-slash |
| **Get architectural context** | [agent-docs/](#agent-docs) - 13 reference docs |
| **Cross-platform support** | [standards/multi-platform-HARD-RULES.md](./standards/multi-platform-HARD-RULES.md) |

---

## Document Structure

```
knowledge-base/
├── INDEX.md                        # This file
├── README.md                       # Detailed navigation guide
├── VALIDATION-RULES.md             # ⭐ Master validation reference (74 rules)
├── PATTERNS-CATALOG.md             # 70 production-tested patterns
│
├── standards/                      # 12 files, 11,036 lines, 288KB
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
└── agent-docs/                     # 13 files, 280KB from awesome-slash
    ├── AI-AGENT-ARCHITECTURE-RESEARCH.md
    ├── CLAUDE-CODE-REFERENCE.md
    ├── CODEX-REFERENCE.md
    ├── OPENCODE-REFERENCE.md
    ├── PROMPT-ENGINEERING-REFERENCE.md
    ├── FUNCTION-CALLING-TOOL-USE-REFERENCE.md
    ├── MULTI-AGENT-SYSTEMS-REFERENCE.md
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
| **MCP** | 11 | 33KB | 36KB | 6 rules |
| **Claude Code** | 10 | 34KB | 40KB | 42 rules |
| **Multi-Platform** | 15 | 15KB | 27KB | 3 rules |
| **Prompt Eng** | 15 | 16KB | 21KB | 4 rules |
| **awesome-slash** | 12 | - | - | 70 patterns |
| **Total** | **75+** | **117KB** | **160KB** | **74 rules** |

### Validation Rules by Category

| Category | Rules | HIGH | MEDIUM | LOW | Auto-Fix |
|----------|-------|------|--------|-----|----------|
| Agent Skills | 15 | 13 | 2 | 0 | 6 |
| Claude Skills | 9 | 7 | 2 | 0 | 3 |
| Claude Hooks | 11 | 9 | 2 | 0 | 2 |
| Claude Agents | 6 | 6 | 0 | 0 | 1 |
| Claude Memory | 10 | 6 | 4 | 0 | 2 |
| Claude Plugins | 5 | 5 | 0 | 0 | 1 |
| MCP | 6 | 6 | 0 | 0 | 1 |
| XML | 3 | 3 | 0 | 0 | 1 |
| References | 2 | 2 | 0 | 0 | 0 |
| Prompt Eng | 4 | 2 | 2 | 0 | 1 |
| Cross-Platform | 3 | 2 | 1 | 0 | 0 |
| **TOTAL** | **74** | **61** | **13** | **0** | **18** |

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

1. **AGENTS.md doesn't exist** - No platform uses this name (XP-002)
2. **Prompt hooks restricted** - Only Stop/SubagentStop supported (CC-HK-002)
3. **Windows paths break skills** - Must use `/` even on Windows (AS-014)
4. **No defense against prompt injection** - Unsolved problem (MCP security)

---

## Usage Guide

### For Implementation

**Start here**: [VALIDATION-RULES.md](./VALIDATION-RULES.md)
- 74 rules with rule IDs (AS-001, CC-HK-001, etc.)
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

**Architecture**: [agent-docs/AI-AGENT-ARCHITECTURE-RESEARCH.md](./agent-docs/AI-AGENT-ARCHITECTURE-RESEARCH.md)
**Prompt Engineering**: [prompt-engineering-HARD-RULES.md](./standards/prompt-engineering-HARD-RULES.md)
**Multi-Agent Systems**: [agent-docs/MULTI-AGENT-SYSTEMS-REFERENCE.md](./agent-docs/MULTI-AGENT-SYSTEMS-REFERENCE.md)

---

## Validation Implementation Checklist

### Week 3: Core Rules (P0)

Parser Setup:
- [ ] YAML frontmatter parser
- [ ] JSON config parser
- [ ] Markdown @import extractor
- [ ] XML tag parser

Skills Validation:
- [ ] AS-001: Frontmatter exists
- [ ] AS-002: Name field exists
- [ ] AS-003: Description field exists
- [ ] AS-004: Name format valid
- [ ] AS-010: Trigger phrase present
- [ ] CC-SK-001: Model value valid
- [ ] CC-SK-006: Dangerous auto-invocation
- [ ] CC-SK-007: Unrestricted Bash

Hooks Validation:
- [ ] CC-HK-001: Valid event name
- [ ] CC-HK-002: Prompt hook restriction
- [ ] CC-HK-003: Matcher required
- [ ] CC-HK-005: Type field exists
- [x] CC-HK-006: Missing command field
- [x] CC-HK-007: Missing prompt field
- [x] CC-HK-008: Script file not found
- [x] CC-HK-009: Dangerous command pattern

Memory Validation:
- [ ] CC-MEM-001: Import paths exist
- [ ] CC-MEM-005: Generic instructions

XML & References:
- [ ] XML-001: Tag balance
- [ ] REF-001: Import resolution

### Week 4: Quality Rules (P1)

Skills:
- [ ] AS-011 through AS-015
- [ ] CC-SK-002 through CC-SK-005

Memory:
- [ ] CC-MEM-006 through CC-MEM-010

Agents:
- [ ] CC-AG-001 through CC-AG-006

Plugins:
- [ ] CC-PL-001 through CC-PL-005

### Week 5-6: Advanced (P2)

- [ ] MCP protocol validation
- [ ] Prompt engineering analysis
- [ ] Cross-platform compatibility

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

---

## Statistics

```
Total Documents:       26 files
Total Lines:          11,036 lines
Total Size:           320KB
Standards Covered:     5 (Agent Skills, MCP, Claude Code, Multi-Platform, Prompt Eng)
Sources Consulted:    75+ (specs, docs, research papers, repos)
Research Agents:       5 (10+ sources each)
Validation Rules:     74 rules
Auto-Fixable Rules:   18 rules
Test Fixtures:        11 files
Platforms Analyzed:   6 (Claude Code, Cursor, Cline, Roo-Cline, Continue.dev, Aider)
```

---

**Status**: Knowledge base complete, ready for implementation
**Next**: Implement validators using VALIDATION-RULES.md
**Confidence**: HIGH - all rules sourced from official specs or research
