# AI Agent Knowledge Library

> Comprehensive reference for building production-quality AI agents. Updated January 2026.

This library consolidates best practices from Anthropic, OpenAI, Google, Microsoft, and cutting-edge research for building effective AI agents.

---

## Table of Contents

1. [Agent Architecture & Design Patterns](#1-agent-architecture--design-patterns)
2. [Prompt Engineering](#2-prompt-engineering)
3. [Function Calling & Tool Use](#3-function-calling--tool-use)
4. [Context Efficiency & Token Optimization](#4-context-efficiency--token-optimization)
5. [Multi-Agent Systems & Orchestration](#5-multi-agent-systems--orchestration)
6. [Instruction Following & Reliability](#6-instruction-following--reliability)
7. [Claude Code Specifics](#7-claude-code-specifics)
8. [Extended Thinking & Reasoning](#8-extended-thinking--reasoning)
9. [Cross-Platform References](#9-cross-platform-references)

---

## 1. Agent Architecture & Design Patterns

### Core Philosophy (Anthropic)

> "Success in the LLM space isn't about building the most sophisticated system. It's about building the right system for your needs."

**Key Principles:**
1. Start with simple prompts
2. Optimize with comprehensive evaluation
3. Add multi-step agentic systems only when simpler solutions fall short
4. Build with simple, composable patterns - avoid complex frameworks

### Agent vs Workflow Distinction

| Type | Definition | When to Use |
|------|------------|-------------|
| **Workflow** | LLMs and tools orchestrated through predefined code paths | Deterministic, predictable tasks |
| **Agent** | Workflows with feedback loops where LLM decides next steps | Dynamic, adaptive tasks |

> "Agents are just workflows with feedback loops. The sophistication comes from how you architect those loops."

### Major Design Patterns

#### ReAct (Reason + Act)
Interleaves reasoning with actions in a step-by-step loop.

```text
Thought: I need to find the user's recent commits
Action: git log --oneline -5
Observation: [commits listed]
Thought: Now I can analyze the changes...
```

**Best for:** Agile, exploratory tasks where each step depends on previous results.

#### Plan-and-Execute
Frontloads a structured plan before carrying it out.

```text
Plan:
1. Read the codebase structure
2. Identify affected files
3. Implement changes
4. Run tests
5. Create PR

Execute: [follows plan sequentially]
```

**Best for:** Complex tasks requiring upfront planning, multi-file changes.

#### Orchestrator-Subagent
Main agent delegates focused subtasks to specialized subagents.

```text
Orchestrator: "Review this PR"
  -> Code-Reviewer Subagent: Check code quality
  -> Security-Scanner Subagent: Check vulnerabilities
  -> Test-Analyzer Subagent: Verify test coverage
Orchestrator: Synthesize results
```

**Best for:** Complex tasks benefiting from specialization and parallel execution.

### Error Recovery Patterns

1. **Retry with Reflection**: On failure, ask model to analyze what went wrong
2. **Graceful Degradation**: Fall back to simpler approaches
3. **Human-in-the-Loop**: Escalate uncertain decisions to user
4. **Checkpoint Recovery**: Save state to resume from last known good point

### Key Sources
- [Anthropic: Building Effective Agents](https://www.anthropic.com/research/building-effective-agents)
- [Claude Code Best Practices](https://www.anthropic.com/engineering/claude-code-best-practices)
- [AI Agent Planning: ReAct vs Plan and Execute](https://byaiteam.com/blog/2025/12/09/ai-agent-planning-react-vs-plan-and-execute-for-reliability/)

---

## 2. Prompt Engineering

### System Prompt Structure

A good system prompt reads like a short contract - explicit, bounded, and verifiable:

```markdown
You are: [role - one line]
Goal: [what success looks like]
Constraints: [list]
If unsure: Say so explicitly and ask 1 clarifying question
Output format: [JSON schema OR heading structure OR bullet format]
```

### Key Techniques

#### 1. XML Tags
Claude is fine-tuned to pay special attention to XML tags:

```xml
<instructions>
Your task instructions here
</instructions>

<context>
Background information
</context>

<examples>
Few-shot examples
</examples>
```

#### 2. Chain-of-Thought (CoT)
Let Claude think through problems step by step:

```text
Think through this problem step by step before providing your answer.
```

**When to use:** Complex reasoning, math, multi-step analysis
**When NOT to use:** Simple, factual queries (adds unnecessary tokens)

#### 3. Few-Shot Examples
Provide examples of desired input/output pairs:

```xml
<example>
<input>Fix the typo in README.md</input>
<output>I'll read README.md and fix any typos found.</output>
</example>
```

**Guidelines:**
- 2-5 examples is usually optimal
- Show edge cases
- Demonstrate both good AND bad examples when relevant

#### 4. Role Assignment
```text
You are a senior security engineer specializing in code review.
Focus on: authentication, input validation, injection vulnerabilities.
```

#### 5. Output Prefilling
Start Claude's response to control format:

```python
messages = [
    {"role": "user", "content": "List 3 bugs"},
    {"role": "assistant", "content": "Here are the bugs:\n1."}
]
```

### Anti-Patterns to Avoid

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| Vague instructions | Inconsistent outputs | Be specific and concrete |
| Over-engineering | Wasted tokens, confusion | Start simple, add complexity only when needed |
| No constraints | Scope creep | Define boundaries explicitly |
| Prompt bloat | Token waste, degraded attention | Remove redundant instructions |

### Claude-Specific Tips

1. **Be direct**: Claude responds well to clear, direct instructions
2. **Use markdown**: Structure with headers, lists, tables
3. **Avoid over-engineering**: "Only make changes that are directly requested"
4. **For proactive behavior**: "By default, implement changes rather than only suggesting them"

### Key Sources
- [Anthropic Prompt Engineering Guide](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/overview)
- [Claude 4.x Best Practices](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-4-best-practices)
- [Interactive Prompt Engineering Tutorial](https://github.com/anthropics/prompt-eng-interactive-tutorial)

---

## 3. Function Calling & Tool Use

### Tool Definition Best Practices

#### Schema Design Checklist

- [ ] Clear, descriptive function names (verb + noun)
- [ ] Detailed parameter descriptions with format hints
- [ ] Use enums for constrained values
- [ ] Prefer flat structures over deeply nested
- [ ] Mark all fields as required (use null union for optional)
- [ ] Set `additionalProperties: false` for strict mode

#### Example: Well-Designed Tool

```json
{
  "name": "search_files",
  "description": "Search for files matching a glob pattern. Returns file paths sorted by modification time.",
  "parameters": {
    "type": "object",
    "properties": {
      "pattern": {
        "type": "string",
        "description": "Glob pattern like '**/*.js' or 'src/**/*.ts'"
      },
      "max_results": {
        "type": ["integer", "null"],
        "description": "Maximum files to return. Null for unlimited."
      }
    },
    "required": ["pattern", "max_results"],
    "additionalProperties": false
  }
}
```

### MCP (Model Context Protocol)

Anthropic's open standard for connecting LLMs to external systems.

**Core Primitives:**

| Primitive | Purpose | Side Effects |
|-----------|---------|--------------|
| **Resources** | Read-only data (files, DB records) | None |
| **Tools** | Actions (run computation, call API) | Yes |
| **Prompts** | Reusable templates | None |

**Token Considerations:**
> "Every tool you expose permanently eats a slice of the available context."

**Best Practice:** Present simplified APIs to LLMs, not full developer APIs.

### Security Considerations

1. **Input Validation**: Sanitize all user-provided paths and parameters
2. **Permission Model**: Start from deny-all, allowlist needed commands
3. **Confirmation**: Require explicit confirmation for sensitive actions
4. **Sandboxing**: Restrict file system and network access

```javascript
// Command injection prevention
const { escapeDoubleQuotes } = require('./utils/shell-escape');
const safePath = escapeDoubleQuotes(userPath);
const command = `git diff "${safePath}"`;
```

### Error Handling

```javascript
async function callTool(tool, params) {
  try {
    const result = await tool.execute(params);
    return { success: true, data: result };
  } catch (error) {
    // Return error info for model to reason about
    return {
      success: false,
      error: error.message,
      suggestion: "Try with different parameters"
    };
  }
}
```

### Key Sources
- [OpenAI Function Calling Guide](https://platform.openai.com/docs/guides/function-calling)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Anthropic Tool Use Documentation](https://platform.claude.com/docs/en/build-with-claude/tool-use)

---

## 4. Context Efficiency & Token Optimization

### Context Window Evolution

| Year | Typical Window | Leaders |
|------|----------------|---------|
| 2022 | 4K tokens | GPT-3 |
| 2024 | 128K tokens | Claude 3, GPT-4 |
| 2025 | 1M+ tokens | Gemini 1.5, Claude 3.5 |
| 2026 | 10M tokens | Llama 4 |

### The Quadratic Problem

> Computational cost increases quadratically with context length. Doubling context = 4x compute.

**Implication:** Bigger isn't always better. Right-size context for the task.

### Optimization Strategies

#### 1. RAG (Retrieval-Augmented Generation)

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐
│ User Query  │ --> │ Retrieve Top │ --> │ LLM + Docs  │
└─────────────┘     │ K Documents  │     └─────────────┘
                    └──────────────┘
```

**Benefits:**
- 52% reduction in hallucinations (Berkeley AI Research)
- Supports knowledge bases 1000x larger than context windows
- Reduces token consumption significantly

**Chunking Best Practices:**
- Chunk size: 256-512 tokens for most use cases
- Overlap: 10-20% between chunks
- Semantic chunking > fixed-size chunking

#### 2. Periodic Summarization

After every 8-10 exchanges, generate a summary:
- Captures key decisions and preferences
- Replaces full history
- **Reduces tokens by 60-70%** while preserving essential info

#### 3. Prompt Caching

**Claude Prompt Caching Pricing:**
| Type | Cost Multiplier |
|------|-----------------|
| Base input | 1.0x |
| Cache write (5-min) | 1.25x |
| Cache write (1-hour) | 2.0x |
| Cache read | 0.1x |

**Break-even: 2 API calls**

**Implementation:**
```javascript
{
  "role": "user",
  "content": [
    {
      "type": "text",
      "text": "Long system context...",
      "cache_control": { "type": "ephemeral" }
    }
  ]
}
```

**Rules:**
- Place static content at beginning of prompt
- Minimum 1,024 tokens per cache checkpoint
- Maximum 4 cache breakpoints
- TTL: 5 minutes (extended: 1 hour for Claude 4.x)

#### 4. Context Prioritization

Order content by importance (most important first):
1. Current task/query
2. Relevant code snippets
3. Recent conversation
4. Background context
5. Examples (can be trimmed)

### Cost Impact

> Organizations with inefficient context management spend 40-60% more on API costs.

### Key Sources
- [Anthropic Prompt Caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
- [Context Engineering: Optimizing LLM Memory](https://medium.com/@kuldeep.paul08/context-engineering-optimizing-llm-memory-for-production-ai-agents-6a7c9165a431)
- [RAG Token Optimization](https://apxml.com/courses/optimizing-rag-for-production/chapter-5-cost-optimization-production-rag/minimize-llm-token-usage-rag)

---

## 5. Multi-Agent Systems & Orchestration

### Market Trend

> 72% of enterprise AI projects now involve multi-agent architectures (2025), up from 23% in 2024.

### Framework Comparison

| Framework | Architecture | Best For | Production Ready |
|-----------|--------------|----------|------------------|
| **LangGraph** | Graph-based | Complex workflows, branching | Yes |
| **AutoGen** | Conversational | Dynamic role-playing | Merging with Semantic Kernel |
| **CrewAI** | Role-based | Defined role delegation | Limited scalability |
| **OpenAI Swarm** | Handoff patterns | Simple agent transitions | Yes (March 2025) |

### Orchestration Patterns

#### 1. Sequential Pipeline
```text
Agent A -> Agent B -> Agent C -> Result
```
**Use when:** Tasks have clear dependencies

#### 2. Parallel Execution
```text
         ┌-> Agent B ─┐
Agent A ─┼-> Agent C ─┼-> Aggregator -> Result
         └-> Agent D ─┘
```
**Use when:** Independent subtasks can run simultaneously

#### 3. Hierarchical Delegation
```text
Orchestrator
    ├── Planner Agent
    ├── Executor Agent
    │       ├── Code Writer
    │       └── Test Writer
    └── Reviewer Agent
```
**Use when:** Complex tasks requiring specialization

### Communication Patterns

| Pattern | Pros | Cons |
|---------|------|------|
| **Message Passing** | Clear boundaries | Serialization overhead |
| **Shared State** | Fast, simple | Concurrency issues |
| **Event-Driven** | Loose coupling | Debugging complexity |

### When to Use Multi-Agent vs Single Agent

**Use Single Agent:**
- Simple, well-defined tasks
- Low latency requirements
- Limited compute budget

**Use Multi-Agent:**
- Complex tasks requiring specialization
- Tasks benefiting from parallel execution
- When different contexts/permissions needed

### Key Sources
- [DataCamp: CrewAI vs LangGraph vs AutoGen](https://www.datacamp.com/tutorial/crewai-vs-langgraph-vs-autogen)
- [AI Agent Orchestration Guide](https://www.digitalapplied.com/blog/ai-agent-orchestration-workflows-guide)
- [Top AI Agent Frameworks 2025](https://www.getmaxim.ai/articles/top-5-ai-agent-frameworks-in-2025-a-practical-guide-for-ai-builders/)

---

## 6. Instruction Following & Reliability

### The Problem

> 63% of production AI systems experience dangerous hallucinations within their first 90 days.

### Guardrails Framework

```text
┌────────────────┐
│  User Input    │
└───────┬────────┘
        ↓
┌───────────────────┐
│  Input Guardrails │ ← Block prompt injection, validate format
└───────┬───────────┘
        ↓
┌───────────────────┐
│     LLM Call      │
└───────┬───────────┘
        ↓
┌───────────────────┐
│ Output Guardrails │ ← Validate structure, check grounding
└───────┬───────────┘
        ↓
┌────────────────┐
│    Response    │
└────────────────┘
```

### Guardrail Types

| Type | Function | Example |
|------|----------|---------|
| **Input Validation** | Block malicious inputs | Prompt injection detection |
| **Confidence Scoring** | Detect uncertainty | Hallucination risk score |
| **Output Validation** | Enforce structure | JSON schema validation |
| **Grounding Check** | Verify factual basis | RAG context matching |

### Hallucination Prevention Techniques

1. **RAG Grounding**: Ground responses in retrieved documents
2. **Confidence Thresholds**: Reject low-confidence outputs
3. **Chain-of-Verification**: Model verifies its own answers
4. **Human-in-the-Loop**: Escalate uncertain responses

**Effectiveness:**
- RAG reduces hallucinations by 52%
- Guardian agents can reduce hallucinations to <1%
- 12 well-implemented guardrails can cut risk by 71-89%

### Instruction Hierarchy

Claude's instruction priority (highest to lowest):
1. System prompt safety instructions
2. System prompt task instructions
3. User instructions
4. Context from tools/documents (untrusted)

### Output Validation Pattern

```javascript
const schema = z.object({
  files: z.array(z.string()),
  summary: z.string().max(500),
  confidence: z.number().min(0).max(1)
});

async function validateResponse(response) {
  try {
    return schema.parse(JSON.parse(response));
  } catch (error) {
    // Retry with explicit format instructions
    return retryWithSchema(schema);
  }
}
```

### Key Sources
- [LLM Guardrails: Strategies & Best Practices 2025](https://www.leanware.co/insights/llm-guardrails)
- [NVIDIA NeMo Guardrails](https://developer.nvidia.com/blog/prevent-llm-hallucinations-with-the-cleanlab-trustworthy-language-model-in-nvidia-nemo-guardrails/)
- [Datadog: LLM Guardrails Best Practices](https://www.datadoghq.com/blog/llm-guardrails-best-practices/)

---

## 7. Claude Code Specifics

### Skills (Slash Commands)

**Structure:**
```markdown
---
name: review
description: Review code for quality issues
---

# Code Review Skill

Review the specified files for:
- Code quality issues
- Security vulnerabilities
- Test coverage gaps
```

**Key Fields:**
- `name`: Becomes the `/slash-command`
- `description`: Helps Claude decide when to auto-invoke

**Invocation:**
- User-invoked: Type `/review`
- Auto-invoked: Claude matches description to task context

### Hooks

Event-driven automation that runs on specific triggers.

**Available Hooks:**
| Hook | Trigger | Use Case |
|------|---------|----------|
| `PreToolCall` | Before tool execution | Validation, logging |
| `PostToolCall` | After tool execution | Formatting, auto-actions |
| `SubagentStop` | When subagent completes | Workflow continuation |

**Example Hook Configuration:**
```json
{
  "hooks": {
    "PostToolCall": {
      "Write": {
        "command": "npx prettier --write ${file}",
        "match": "*.js"
      }
    }
  }
}
```

### Subagents

**Built-in Subagents:**
- `Explore`: Fast codebase exploration
- `Plan`: Implementation planning
- `general-purpose`: Multi-purpose tasks

**Custom Subagent Definition:**
```markdown
// .claude/agents/security-reviewer.md
---
name: security-reviewer
tools: [Read, Grep, Glob]  # Restricted tool access
---

You are a security-focused code reviewer.
Check for: injection, XSS, authentication issues.
```

**Spawning Pattern:**
```javascript
Task({
  subagent_type: "security-reviewer",
  prompt: "Review the authentication module",
  run_in_background: true
})
```

### CLAUDE.md Best Practices

Place in project root to encode conventions:

```markdown
# Project Memory

## Architecture
- Monorepo with plugins/
- Each plugin is independent

## Testing
- Run: npm test
- Coverage: 80% minimum

## Code Style
- No console.log in production
- Async/await over callbacks
```

### Permission Model

Start from deny-all baseline:
```text
Treat tool access like production IAM.
Allowlist only needed commands per subagent.
Require explicit confirmations for sensitive actions.
```

### Best Practices (from Claude Code Docs)

**Most Important:** Context window management. Performance degrades as context fills.

**Single highest-leverage improvement:** Give Claude verification criteria.
```text
Before: "implement email validation"
After: "write validateEmail function. test cases: user@example.com=true, invalid=false, user@.com=false. run tests after implementing"
```

**Workflow Pattern:** Explore → Plan → Implement → Commit
1. Enter Plan Mode, read files, understand context
2. Create detailed implementation plan
3. Switch to Normal Mode, implement with verification
4. Commit with descriptive message

**CLAUDE.md Guidelines:**

| Include | Exclude |
|---------|---------|
| Bash commands Claude can't guess | Things Claude can infer from code |
| Code style rules differing from defaults | Standard conventions Claude knows |
| Testing instructions, preferred runners | Detailed API docs (link instead) |
| Architectural decisions specific to project | Information that changes frequently |
| Common gotchas or non-obvious behaviors | Self-evident practices |

**If Claude ignores rules:** The CLAUDE.md is probably too long. Prune ruthlessly.

**Emphasis for important rules:** Use "IMPORTANT:" or "MUST" prefix to improve adherence.

**Common Failure Patterns:**
1. **Kitchen sink session** - Fix: `/clear` between unrelated tasks
2. **Correcting over and over** - Fix: After two failed corrections, `/clear` and write better initial prompt
3. **Over-specified CLAUDE.md** - Fix: Remove anything Claude does correctly without the instruction
4. **Trust-then-verify gap** - Fix: Always provide verification (tests, scripts, screenshots)
5. **Infinite exploration** - Fix: Scope investigations or use subagents

**Subagent Usage:** Delegate research to subagents to keep main context clean:
```text
"Use subagents to investigate how authentication handles token refresh"
```

**Skills with side effects:** Add `disable-model-invocation: true` for manual-only invocation.

### Key Sources
- [Claude Code Slash Commands](https://code.claude.com/docs/en/slash-commands)
- [Claude Code Customization Guide](https://alexop.dev/posts/claude-code-customization-guide-claudemd-skills-subagents/)
- [Create Custom Subagents](https://code.claude.com/docs/en/sub-agents)

---

## 8. Extended Thinking & Reasoning

### Overview

Extended thinking allows Claude to spend more computational effort on complex problems by generating a hidden chain-of-thought before responding.

### How It Works

```text
User Query
    ↓
┌──────────────────────────┐
│ Extended Thinking        │ ← Hidden "scratchpad"
│ (internal reasoning)     │   Token budget controls depth
└──────────────────────────┘
    ↓
Final Response
```

### Configuration

```python
{
  "thinking": {
    "type": "enabled",
    "budget_tokens": 4096  # Minimum: 1024
  }
}
```

**Recommendation:** Start at minimum (1024), increase incrementally.

### Performance

> Accuracy improves logarithmically with thinking tokens.

| Thinking Budget | Use Case |
|-----------------|----------|
| 1024 | Simple clarification |
| 4096 | Standard reasoning |
| 16384 | Complex multi-step problems |
| 65536+ | Deep analysis, math proofs |

### Interleaved Thinking (Claude 4)

Claude 4 models can think between tool calls:

```text
Think -> Tool Call -> Think -> Tool Call -> Response
```

**Benefit:** More sophisticated reasoning after receiving tool results.

### When to Use

**Enable Extended Thinking:**
- Complex multi-step reasoning
- Math problems
- Code architecture decisions
- Security analysis

**Skip Extended Thinking:**
- Simple factual queries
- Quick lookups
- Straightforward edits

### Best Practices

> "Claude often performs better with high-level instructions to think deeply rather than step-by-step prescriptive guidance."

```text
Think carefully about the security implications before proceeding.
```

Is better than:
```text
Step 1: List attack vectors
Step 2: Analyze each one
Step 3: ...
```

### Key Sources
- [Anthropic: Visible Extended Thinking](https://www.anthropic.com/news/visible-extended-thinking)
- [Building with Extended Thinking](https://platform.claude.com/docs/en/build-with-claude/extended-thinking)
- [Extended Thinking Tips](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/extended-thinking-tips)

---

## Quick Reference Card

### Prompt Template

```markdown
<system>
You are: [role]
Goal: [success criteria]
Constraints:
- [constraint 1]
- [constraint 2]
Output: [format specification]
</system>

<context>
[background information]
</context>

<instructions>
[specific task instructions]
</instructions>
```

### Tool Definition Template

```json
{
  "name": "verb_noun",
  "description": "Clear description of what the tool does and when to use it.",
  "parameters": {
    "type": "object",
    "properties": {
      "required_param": {
        "type": "string",
        "description": "What this parameter is for"
      },
      "optional_param": {
        "type": ["string", "null"],
        "description": "Optional: what this does"
      }
    },
    "required": ["required_param", "optional_param"],
    "additionalProperties": false
  }
}
```

### Subagent Template

```markdown
---
name: agent-name
description: When this agent should be used
tools: [Tool1, Tool2]
model: sonnet  # or opus, haiku
---

# Agent Name

## Role
You are a [specialized role].

## Instructions
1. [Step 1]
2. [Step 2]

## Constraints
- [Constraint 1]
- [Constraint 2]

## Output Format
[Expected output structure]
```

---

## 9. Cross-Platform References

### Platform Documentation

| Platform | Reference Doc | Description |
|----------|--------------|-------------|
| **Claude Code** | [CLAUDE-CODE-REFERENCE.md](./CLAUDE-CODE-REFERENCE.md) | Claude Code specifics: hooks, skills, subagents |
| **Codex CLI** | [CODEX-REFERENCE.md](./CODEX-REFERENCE.md) | Codex CLI specifics: config, skills, AGENTS.md |
| **OpenCode** | [OPENCODE-REFERENCE.md](./OPENCODE-REFERENCE.md) | OpenCode specifics: config, commands, AGENTS.md |

### Key Platform Differences

| Feature | Claude Code | Codex CLI | OpenCode |
|---------|-------------|----------|----------|
| Config file | `settings.json` | `config.toml` | `opencode.json` (JSONC supported) |
| State directory | `.claude/` | `.codex/` | `.opencode/` |
| Command prefix | `/` | `$` | `/` |
| Model selection | Anthropic only | OpenAI | 75+ providers |
| User questions | Checkboxes | Numbered list | Numbered list |
| Project instructions | `CLAUDE.md` | `AGENTS.md` | `AGENTS.md` |

### MCP Compatibility

All platforms support MCP (Model Context Protocol):
- Same tool definitions work across platforms
- State isolation via `AI_STATE_DIR` environment variable
- Local (stdio) and remote (HTTP) server types

### Our Cross-Platform Strategy

1. **Single codebase** - Commands/skills shared across platforms
2. **Platform detection** - `lib/cross-platform/` handles differences
3. **State abstraction** - Platform-specific directories, same file formats
4. **MCP server** - Universal tool interface for all platforms

---

## Changelog

- **2026-01-24**: Added Cross-Platform References section, OpenCode research
- **2026-01-22**: Initial compilation from 7 research agents
- Sources: Anthropic, OpenAI, Google, Microsoft, arXiv, production implementations

---

*This library is a living document. Update as new best practices emerge.*
