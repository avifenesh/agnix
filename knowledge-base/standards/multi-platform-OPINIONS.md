# Multi-Platform Agent Standards - OPINIONATED GUIDE

## Document Purpose
This document contains BEST PRACTICES, RECOMMENDATIONS, and STRATEGIC GUIDANCE for building cross-platform AI coding assistant projects. These are informed opinions based on industry patterns and community experience.

**Companion Document:** See `multi-platform-HARD-RULES.md` for non-negotiable technical constraints.

**Last Updated:** 2026-01-31

**NOTE (Cursor):** Cursor's current rules mechanism is `.cursor/rules/*.mdc`. `.cursorrules` is legacy; prefer migrating to Project Rules.

---

## Philosophy: When to Use Platform-Specific Features

### The Cross-Platform Spectrum

```
Pure Portable ←────────────────→ Platform-Specific
│                                 │
│  MCP Servers                    │  Hooks (Claude Code only)
│  Git repos                      │  Custom Modes (Roo-Cline)
│  Markdown docs                  │  Voice (Aider only)
│  Environment vars               │  .cursor/rules/*.mdc (Cursor)
│                                 │
└─ Maximize reach                 └─ Maximize power
```

### Decision Framework

**Use platform-specific features when:**
- The feature provides 10x value over generic alternatives
- Your team standardizes on one platform
- The feature is irreplaceable (e.g., Cursor's `.cursor/rules/*.mdc` for team-wide AI behavior)

**Avoid platform-specific features when:**
- The feature has MCP equivalents (tools, resources, prompts)
- You're building shared libraries or frameworks
- Onboarding new team members with different tool preferences

---

## Recommended File Organization

### The "Platform-Agnostic Core" Pattern

**RECOMMENDED:** Organize your project with a clear separation between portable and platform-specific configuration.

```
project-root/
├── docs/                      # ✅ Universal (Markdown)
│   ├── architecture.md
│   ├── conventions.md
│   └── onboarding.md
│
├── .platform/                 # ✅ All platform configs isolated
│   ├── claude-code/
│   │   ├── CLAUDE.md
│   │   └── .claude/
│   │       ├── skills/
│   │       └── hooks/
│   ├── cursor/
│   │   └── .cursor/rules/*.mdc
│   ├── cline/
│   │   ├── .clinerules/
│   │   └── .cline/
│   ├── continue/
│   │   └── config.yaml
│   └── aider/
│       └── .aider.conf.yml
│
├── .mcp/                      # ✅ Shared MCP servers
│   └── servers/
│       ├── database/
│       └── filesystem/
│
├── .env.example               # ✅ Universal secrets template
└── .gitignore                 # ✅ Ignore state, keep configs
```

**WHY THIS WORKS:**
- Clear visibility: Team members see ALL supported platforms
- Easy maintenance: Update one platform without affecting others
- Safe experimentation: Test new platforms without breaking existing setups
- Onboarding friendly: New developers pick their preferred tool

### Alternative: "Convention Over Configuration"

**RECOMMENDED FOR:** Small teams with unified tooling

```
project-root/
├── CLAUDE.md                  # Project memory (Claude Code)
├── .cursor/rules/*.mdc        # AI behavior rules (Cursor)
├── .aider.conf.yml            # Settings (Aider)
├── .continue/
│   └── config.yaml            # Continue.dev config
└── docs/
    └── CONVENTIONS.md         # Universal coding standards
```

**WHY THIS WORKS:**
- Minimal overhead for small teams
- Follows platform conventions exactly
- No abstraction layer to maintain

---

## Configuration Strategy Recommendations

### API Keys: The Environment Variable Standard

**BEST PRACTICE:** Always use environment variables for API keys, regardless of platform.

```bash
# .env.example (commit this)
ANTHROPIC_API_KEY=your_key_here
OPENAI_API_KEY=your_key_here
AIDER_MODEL=claude-3-5-sonnet-20241022

# .env (DO NOT commit)
ANTHROPIC_API_KEY=sk-ant-actual-key
OPENAI_API_KEY=sk-actual-key
```

**REASONING:**
- Works across ALL platforms (even Cursor via system environment)
- Prevents accidental key commits
- Simplifies CI/CD integration
- Enables per-developer key management

### Model Selection: Abstract When Possible

**RECOMMENDED PATTERN:**

```yaml
# .platform/continue/config.yaml
models:
  - role: chat
    title: "Primary Agent"
    provider: anthropic
    model: claude-3-5-sonnet-20241022
    apiKey: ${ANTHROPIC_API_KEY}

  - role: autocomplete
    title: "Fast Completions"
    provider: openai
    model: gpt-4o-mini
    apiKey: ${OPENAI_API_KEY}
```

```yaml
# .platform/aider/.aider.conf.yml
model: claude-3-5-sonnet-20241022
weak-model: gpt-4o-mini
```

**REASONING:**
- Consistent model hierarchy across platforms (primary + fallback)
- Easy to swap models by changing environment variables
- Cost optimization through appropriate model selection

### Configuration Inheritance: Learn from Aider

**RECOMMENDED:** When designing custom tools, adopt Aider's cascading config pattern:

1. System-wide defaults (`~/.config/tool/`)
2. Project-specific overrides (`./project/.config/tool/`)
3. Environment variables
4. Command-line arguments (highest priority)

**WHY:** This matches developer expectations and enables team-wide standards + individual customization.

---

## Cross-Platform Skills and Prompts Strategy

### Problem: Incompatible Skill Formats

**Reality Check:** You CANNOT share skill/prompt implementations between platforms. Accept this and plan accordingly.

### Recommended Solution: "Contract-Based Skills"

**Define skills as contracts in Markdown, implement per-platform:**

```
skills/
├── contracts/                 # ✅ Portable contracts
│   ├── pr-creation.md         # What the skill does
│   ├── code-review.md
│   └── refactor-assistant.md
│
└── implementations/           # ❌ Platform-specific
    ├── claude-code/
    │   └── .claude/skills/pr-creation/SKILL.md
    ├── continue/
    │   └── pr-creation.yaml
    └── roo-cline/
        └── .roomodes         # Custom mode for PR creation
```

**CONTRACT EXAMPLE (contracts/pr-creation.md):**

```markdown
# Skill: Pull Request Creation

## Purpose
Automate creation of pull requests with:
- Auto-generated title from commit history
- Comprehensive description with checklist
- Linked issues and relevant context

## Inputs
- Branch name (required)
- Target branch (default: main)
- Draft mode (default: false)

## Outputs
- PR URL
- PR number for reference

## Prerequisites
- Git repository with remote
- GitHub CLI (gh) installed
- Valid authentication token
```

**WHY THIS WORKS:**
- Contracts are documentation (always useful)
- Implementation details stay platform-specific
- Team members understand skill capabilities regardless of tool choice
- Easier to maintain consistency across platforms

---

## MCP: The Only True Cross-Platform Standard

### Strong Recommendation: Invest in MCP

**MCP (Model Context Protocol) is the ONLY standardized, cross-platform way to extend AI coding assistants.**

### MCP Adoption Strategy

**TIER 1 (Highest Priority):**
Build MCP servers for functionality you need across platforms:
- Database access
- API integrations
- Custom file operations
- Specialized search/retrieval

**TIER 2 (Secondary):**
Use platform-specific features for UX enhancements:
- Hooks for pre-commit validation (Claude Code)
- Custom modes for specialized workflows (Roo-Cline)
- Voice input for brainstorming (Aider)

**WHY:** MCP servers work in Claude Code, Cline, and Continue.dev TODAY. As the standard matures, expect broader adoption.

### MCP Server Best Practices

**RECOMMENDED ARCHITECTURE:**

```typescript
// mcp-servers/database/
import { Server } from "@modelcontextprotocol/sdk/server/index.js";

const server = new Server({
  name: "database-mcp-server",
  version: "1.0.0"
}, {
  capabilities: {
    tools: {},      // Expose query operations
    resources: {},  // Expose schema information
  }
});

// Define tools (actions)
server.setRequestHandler("tools/list", async () => ({
  tools: [
    {
      name: "query_database",
      description: "Execute SQL query",
      inputSchema: {
        type: "object",
        properties: {
          query: { type: "string" },
          params: { type: "array" }
        }
      }
    }
  ]
}));

// Define resources (data)
server.setRequestHandler("resources/list", async () => ({
  resources: [
    {
      uri: "db://schema/tables",
      name: "Database Schema",
      mimeType: "application/json"
    }
  ]
}));
```

**WHY THIS PATTERN:**
- Clean separation: tools for actions, resources for data
- Self-documenting via JSON schemas
- Testable independently of any AI platform
- Reusable across multiple projects

---

## Migration Strategy Recommendations

### Gradual Platform Adoption

**RECOMMENDED APPROACH:** Don't force platform standardization. Support multiple platforms and let teams choose.

**PHASE 1: Setup (Week 1)**
- Create `.platform/` directory
- Add config for your primary platform
- Document in README which platforms are supported

**PHASE 2: Expansion (Month 1)**
- Team members add configs for their preferred tools
- Share learnings about platform strengths/weaknesses
- Identify common pain points → MCP server opportunities

**PHASE 3: Optimization (Quarter 1)**
- Migrate shared functionality to MCP servers
- Deprecate redundant platform-specific solutions
- Standardize on proven patterns

**PHASE 4: Maturity (Year 1)**
- Most functionality via MCP (portable)
- Platform-specific configs are thin wrappers
- Easy onboarding for any tool preference

### When to Force Platform Standardization

**Consider forcing one platform when:**
- Team < 5 people (overhead not worth it)
- Platform-specific features are critical (e.g., Cursor's context windows)
- Training/onboarding costs exceed flexibility benefits

**Red flags for forced standardization:**
- Developers resist actively ("I prefer X tool")
- High churn on team (new devs have different preferences)
- Platform limitations block common workflows

---

## Rules Files: Best Practices

### Cursor Rules (.cursor/rules/*.mdc) Guidelines

**RECOMMENDED STRUCTURE:**

```
# Project: [Name]
# Purpose: AI coding assistant behavior rules

## Core Principles
- Follow TDD: write tests before implementation
- Prefer composition over inheritance
- Use TypeScript strict mode

## Code Style
- 2-space indentation
- Single quotes for strings
- Trailing commas in multi-line

## Architecture Patterns
- Feature-based directory structure
- Dependency injection via constructors
- Async/await over promises

## Libraries and Frameworks
- React 18+ with hooks (no class components)
- Zustand for state management
- Vitest for testing

## Common Patterns
When creating a new API endpoint:
1. Define TypeScript interface for request/response
2. Create route handler in src/api/
3. Add integration test in tests/api/
4. Update API documentation

## Avoid
- Any usage of `var` (use `const` or `let`)
- Implicit any types
- Large files (>500 lines - split into modules)
```

**WHY THIS FORMAT:**
- Explicit sections make it clear what AI should prioritize
- Examples show expected patterns
- "Avoid" section prevents common mistakes

### Cline .clinerules/ Guidelines

**RECOMMENDED STRUCTURE:**

```
.clinerules/
├── code-style.md          # Language-specific formatting rules
├── architecture.md        # System design principles
├── testing.md             # Testing requirements
└── workflows/
    ├── pr-workflow.md     # How to create PRs
    └── review-process.md  # Code review expectations
```

**WHY THIS STRUCTURE:**
- Modular: update one aspect without affecting others
- Discoverable: clear file names
- Scalable: add new rule categories as needed

---

## State Management: What to Commit

### Recommended .gitignore Additions

```gitignore
# Platform state directories (NEVER COMMIT)
.claude/state/
.cline/
.roo/
.aider/
.continue/

# Secrets (CRITICAL)
.env
.env.local
.env.*.local

# Platform configs WITH secrets (case-by-case)
# Uncomment if your configs contain API keys:
# .aider.conf.yml
# .continue/config.yaml

# Platform configs WITHOUT secrets (COMMIT THESE)
# These should be in version control for team sharing:
# Cursor Rules (Legacy: .cursorrules)
# .clinerules/
# .platform/**/*.yml
# .platform/**/*.yaml
```

### What TO Commit (Recommended)

**DO commit:**
- `.cursor/rules/*.mdc` - Team-wide AI behavior rules (Project Rules)
- `.aider.conf.yml` - If no API keys present
- `config.yaml` - If using env vars for secrets
- `.clinerules/` - Team coding standards
- `.platform/` - All platform configs (if using this pattern)
- `.mcp/` - MCP server configurations

**WHY:** These files enable consistent AI behavior across your team.

---

## Team Workflow Recommendations

### Single-Platform Teams (< 10 people)

**RECOMMENDED:**
1. Pick ONE primary platform (based on team preference)
2. Configure it well (comprehensive rules, good prompts)
3. Document setup in README
4. Provide .env.example for easy onboarding

**AVOID:**
- Trying to support multiple platforms prematurely
- Over-engineering configuration abstractions

### Multi-Platform Teams (10+ people)

**RECOMMENDED:**
1. Use the `.platform/` pattern (see "Recommended File Organization")
2. Invest in MCP servers for shared functionality
3. Create onboarding docs PER PLATFORM
4. Maintain a "platform feature matrix" in your README

**AVOID:**
- Forcing platform choice on team members
- Letting configurations drift (assign owners per platform)

### Open Source Projects

**RECOMMENDED:**
1. Support AT LEAST two platforms (maximize contributor comfort)
2. Choose platforms with different strengths:
   - Cursor (best UX for junior contributors)
   - Claude Code (most powerful for advanced users)
   - Aider (best for terminal-first workflows)
3. Document platform-specific setup in CONTRIBUTING.md

**AVOID:**
- Supporting too many platforms (maintenance burden)
- Assuming contributors have paid subscriptions (Aider is free/OSS)

---

## Testing and Validation Strategies

### Cross-Platform Testing Reality Check

**You cannot assume Cursor rules behave identically in other tools.** Cline can read `.cursor/rules/`, but semantics may differ; verify in both.

### What You CAN Test

**1. MCP Servers (High Value)**
```bash
# Test MCP server independently
npm test  # Unit tests
mcp-inspector  # Interactive testing tool
```

**2. Documentation Completeness**
```bash
# Ensure all supported platforms have configs
ls .platform/*/  # Should show all your supported platforms
```

**3. Environment Variables**
```bash
# Validate .env.example has all required keys
grep "API_KEY" .env.example
```

**4. Git Hooks (Claude Code specific)**
```bash
# Test hooks locally before pushing
.claude/hooks/pre-commit
```

### Recommended CI/CD Validation

```yaml
# .github/workflows/validate-configs.yml
name: Validate Platform Configs

on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # Validate YAML syntax
      - name: Check Aider config
        run: yamllint .platform/aider/.aider.conf.yml

      - name: Check Continue config
        run: yamllint .platform/continue/config.yaml

      # Validate environment variables
      - name: Check .env.example completeness
        run: |
          required_keys=("ANTHROPIC_API_KEY" "OPENAI_API_KEY")
          for key in "${required_keys[@]}"; do
            grep -q "$key" .env.example || exit 1
          done

      # Test MCP servers
      - name: Test MCP servers
        run: |
          cd .mcp/servers
          npm test
```

---

## Security Best Practices

### API Key Management

**TIER 1 (Safest):**
- Use environment variables exclusively
- Store secrets in password manager or secret vault
- Rotate keys regularly

**TIER 2 (Acceptable):**
- Use platform's built-in secret management (Cursor settings)
- Keep secrets in config files with restrictive permissions (chmod 600)
- Add config files with secrets to .gitignore

**TIER 3 (Avoid if possible):**
- Hardcoded keys in config files
- Committing .env files (even to private repos)

### Recommended .env Pattern

```bash
# .env.example (commit this)
# Copy to .env and fill in actual values

# Required: Anthropic API key for Claude models
ANTHROPIC_API_KEY=sk-ant-...

# Required: OpenAI API key for GPT models
OPENAI_API_KEY=sk-...

# Optional: Specific model selections
PRIMARY_MODEL=claude-3-5-sonnet-20241022
FALLBACK_MODEL=gpt-4o-mini

# Optional: Feature flags
ENABLE_VOICE=false
ENABLE_MCP_SERVERS=true
```

---

## Performance Optimization Tips

### Context Window Management

**PROBLEM:** Platforms like Cursor and Claude Code send entire file contents to LLMs.

**RECOMMENDED STRATEGIES:**

1. **Explicit File Selection (Aider pattern)**
   ```bash
   # Only add files you're actively editing
   aider src/main.py src/utils.py
   ```

2. **Repository Mapping (Automatic)**
   - Aider and Continue.dev build automatic repo maps
   - Reduces token usage by understanding project structure
   - Enable this when available

3. **Smart Context Rules**
   ```
   # In .cursor/rules/*.mdc (Cursor) or similar
   ## Context Management
   - Only include files mentioned in the conversation
   - Exclude test files unless explicitly debugging tests
   - Ignore generated files (dist/, build/, etc.)
   ```

### Model Selection Strategy

**RECOMMENDED HIERARCHY:**

| Task | Recommended Model | Reasoning |
|------|-------------------|-----------|
| Complex refactoring | Claude 3.5 Sonnet / o1 | Best reasoning |
| Simple edits | GPT-4o / Claude 3 Haiku | Fast + cheap |
| Autocomplete | DeepSeek Coder / Copilot | Specialized |
| Code review | Claude 3.5 Sonnet | Catches edge cases |
| Documentation | GPT-4o | Natural language |

**COST OPTIMIZATION:**
- Use "weak model" fallbacks for simple tasks (Aider's `--weak-model`)
- Configure different models per role (Continue.dev's role-based models)
- Monitor token usage and adjust file inclusion strategies

---

## Platform-Specific Power User Tips

### Claude Code: Leverage Hooks

**USE CASE:** Enforce code quality before commits

```bash
# .claude/hooks/pre-commit
#!/bin/bash
set -e

echo "Running linter..."
npm run lint

echo "Running tests..."
npm test

echo "Type checking..."
tsc --noEmit
```

**WHY:** Catches issues before they reach CI/CD.

### Cursor: Master `.cursor/rules/*.mdc`

**POWER TIP:** Include project-specific anti-patterns

```
## This Project's Gotchas
- Never use `Date.now()` directly - use our `getCurrentTimestamp()` utility
- Database queries must use prepared statements (prevent SQL injection)
- All API responses must match our standard error format (see ErrorResponse type)
```

**WHY:** AI learns your project's unique constraints.

### Aider: Voice-Driven Development

**USE CASE:** Brainstorming and exploration phases

```bash
# Enable voice input
aider --voice-language en

# Then speak naturally:
# "Add a function to validate email addresses using regex"
# "Create a test for the email validator"
```

**WHY:** Faster iteration during creative phases.

### Continue.dev: Custom Prompts for Workflows

**POWER TIP:** Create slash commands for common tasks

```yaml
# config.yaml
prompts:
  - name: "debug-error"
    description: "Investigate and fix an error"
    prompt: |
      I'm seeing the following error:
      {error}

      Please:
      1. Identify the root cause
      2. Suggest a fix with code examples
      3. Explain why this error occurred
```

**WHY:** Streamlines repetitive debugging workflows.

### Roo-Cline: Custom Modes for Roles

**USE CASE:** Different modes for different tasks

```
# .roomodes
code: Standard development
architect: High-level design (no implementation)
debug: Focus on error investigation
docs: Documentation writing only
```

**WHY:** Forces AI into task-appropriate behavior.

---

## Common Pitfalls and How to Avoid Them

### Pitfall 1: Over-Configuring

**SYMPTOM:** Spending more time configuring tools than coding.

**SOLUTION:**
- Start with defaults
- Only customize what actively blocks your workflow
- Review configurations quarterly, remove unused settings

### Pitfall 2: Configuration Sprawl

**SYMPTOM:** Configs scattered across home directory and projects.

**SOLUTION:**
- Adopt the `.platform/` pattern consistently
- Use symbolic links if needed: `ln -s .platform/cursor/.cursor/rules .cursor/rules`
- Document configuration locations in README

### Pitfall 3: API Key Leaks

**SYMPTOM:** Accidentally committing secrets to git.

**SOLUTION:**
- Use git hooks to scan for API keys before commits
- Add secrets to .gitignore BEFORE creating them
- Use tools like `git-secrets` or `trufflehog`

### Pitfall 4: Platform Lock-In

**SYMPTOM:** Unable to switch platforms because of deep feature dependencies.

**SOLUTION:**
- Invest in MCP servers for critical functionality
- Keep platform-specific features at the "edges" (UX, not logic)
- Regularly evaluate: "Could we switch platforms in 1 week if needed?"

### Pitfall 5: Ignoring Team Preferences

**SYMPTOM:** Forcing everyone to use the same tool, causing friction.

**SOLUTION:**
- Survey team about tool preferences
- Support 2-3 platforms maximum
- Let individuals choose within supported set

---

## Future-Proofing Your Setup

### Trends to Watch

**1. MCP Standardization**
- **Prediction:** MCP will become the "USB-C of AI tools" by 2027
- **Action:** Build MCP servers for any custom integrations now

**2. Cloud-Based Agents**
- **Prediction:** More platforms will offer headless/cloud execution (like Continue.dev)
- **Action:** Design prompts/skills to work without IDE context

**3. Multi-Model Orchestration**
- **Prediction:** Tools will intelligently route tasks to specialized models
- **Action:** Define clear model selection criteria in your configs

**4. Improved Context Management**
- **Prediction:** Better automatic repository understanding (beyond simple mapping)
- **Action:** Structure code for easy LLM comprehension (clear module boundaries)

### Preparing for Change

**Recommended Practices:**

1. **Configuration as Code**
   - Version control ALL platform configs
   - Document WHY each setting exists
   - Review and prune quarterly

2. **Modular Skills/Prompts**
   - Keep skill logic separate from platform implementation
   - Use the "contract-based skills" pattern
   - Document inputs/outputs clearly

3. **MCP-First Mindset**
   - Ask "Can this be an MCP server?" before building platform-specific features
   - Contribute to MCP ecosystem (more servers = more value for everyone)

4. **Team Knowledge Sharing**
   - Maintain a "platform tips" document
   - Share discoveries in team chat
   - Run quarterly "AI tooling" retrospectives

---

## Decision Matrix: Choosing a Platform

### For Individual Developers

| Criteria | Best Platform | Runner-Up |
|----------|---------------|-----------|
| Best UX | Cursor | Cline |
| Most Powerful | Claude Code | Aider |
| Best for Beginners | Cursor | Continue.dev |
| Best for Terminal Users | Aider | Claude Code |
| Best for Customization | Claude Code | Cline |
| Best Free Option | Aider | Continue.dev |

### For Teams

| Criteria | Best Platform | Runner-Up |
|----------|---------------|-----------|
| Easiest Onboarding | Cursor | Continue.dev |
| Best Collaboration | Cursor (.cursor/rules) | Continue.dev (config.yaml) |
| Most Flexible | Claude Code (MCP + hooks) | Continue.dev |
| Best for Remote | Continue.dev (headless) | Aider (CLI) |
| Budget-Conscious | Aider (free) | Continue.dev (bring your own key) |

### For Enterprises

| Criteria | Best Platform | Runner-Up |
|----------|---------------|-----------|
| Security | Claude Code (local-first) | Aider (open source) |
| Compliance | Aider (self-hosted) | Claude Code |
| Customization | Claude Code (hooks, skills, MCP) | Continue.dev |
| Multi-IDE Support | Continue.dev | Cline |
| Support | Cursor (paid support) | Claude Code (Anthropic) |

---

## Summary: The Pragmatic Approach

### Recommended Strategy for Most Teams

1. **Start Simple:** Pick ONE platform based on team preferences (see decision matrix)

2. **Configure Well:** Invest time in comprehensive rules/prompts for that platform

3. **Use MCP Early:** Build MCP servers for any custom integrations

4. **Support Flexibility:** Use the `.platform/` pattern if team wants to use different tools

5. **Iterate:** Review and refine configurations quarterly

### The Golden Rules

1. **MCP for logic, platforms for UX** - Keep portable logic in MCP servers
2. **Environment variables for secrets** - Never hardcode API keys
3. **Configuration as code** - Version control all configs (except secrets)
4. **Team autonomy over tool uniformity** - Let people use what works for them
5. **Document everything** - Future you will thank current you

---

## Getting Started Checklist

### Week 1: Setup
- [ ] Choose primary platform
- [ ] Create .env.example with required API keys
- [ ] Configure basic rules/prompts
- [ ] Document setup in README
- [ ] Add platform state directories to .gitignore

### Month 1: Optimize
- [ ] Identify common workflows → create prompts/skills
- [ ] Build first MCP server (if needed)
- [ ] Share learnings with team
- [ ] Add second platform support (if requested)

### Quarter 1: Scale
- [ ] Migrate common logic to MCP
- [ ] Establish configuration review cadence
- [ ] Create onboarding docs per platform
- [ ] Run team retrospective on tooling

---

## Additional Resources

### Learning MCP
- Official docs: https://modelcontextprotocol.io
- Reference servers: https://github.com/modelcontextprotocol/servers
- MCP Inspector (testing tool): https://github.com/modelcontextprotocol/inspector

### Platform Communities
- Cursor Forum: https://forum.cursor.com
- Aider Discord: (see aider.chat)
- Continue.dev Discord: (see docs.continue.dev)
- Claude Code: Anthropic documentation

### Keep Learning
- This is a rapidly evolving space
- Join communities for your chosen platforms
- Share your learnings (blog posts, conference talks)
- Contribute to open source tools

---

## Contributing to This Guide

This document represents opinions based on:
- 15+ sources of documentation
- Community best practices
- Industry trends as of 2026-01-31

**Got better practices?** Document them and share with your team. These standards will evolve as the ecosystem matures.

---

## Final Thoughts

**There is no "one true way" to configure AI coding assistants.**

The best approach is the one that:
- Aligns with your team's preferences
- Solves your actual problems
- Doesn't get in the way of shipping code

Start simple. Iterate based on real needs. Invest in portability (MCP) when it makes sense. And remember: the goal is to ship better software faster, not to have perfect tool configurations.

Happy coding!
