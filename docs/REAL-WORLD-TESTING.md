# Real-World Testing Guide

How to validate agnix against real-world repositories to find false positives, false negatives, and message quality issues.

## Quick Start

```bash
# Setup
python3 -m venv .venv
source .venv/bin/activate
pip install pyyaml

# Build
cargo build --release

# Run against all repos (auto-cleans clones to save disk)
python scripts/real-world-validate.py --parallel 8 --timeout 60

# Run a subset
python scripts/real-world-validate.py --limit 50 --parallel 8
python scripts/real-world-validate.py --category claude-code --parallel 4
python scripts/real-world-validate.py --filter streamlit --parallel 1
```

## Repo Manifest

The curated list is at `tests/real-world/repos.yaml` with 1,236 repos across categories:
- `claude-code`: CLAUDE.md projects
- `agent-config`: AGENTS.md projects
- `cursor`: .cursorrules and .mdc projects
- `github-copilot`: copilot-instructions.md projects
- `mcp`: MCP server configurations
- `claude-code-hooks`: .claude/settings.json projects
- `web-dev`, `general-ai`, `tools`, `other`: Mixed

### Adding Repos

```yaml
  - url: https://github.com/owner/repo
    categories: [claude-code]
    status: pending
```

Search for candidates:
```bash
gh search code --filename CLAUDE.md --limit 100 --json repository
gh search code --filename AGENTS.md --limit 100 --json repository
gh search code --filename .cursorrules --limit 100 --json repository
gh search code --filename copilot-instructions.md --limit 100 --json repository
gh search code --filename mcp.json --limit 100 --json repository
```

## Manual Inspection Process

The automation finds aggregate patterns. Manual inspection finds the subtle issues.

### Step 1: Clone and Run

```bash
mkdir -p test-output/real-world/inspect
git clone --depth 1 https://github.com/owner/repo test-output/real-world/inspect/owner-repo

# Run from the repo directory (avoids inheriting project .agnix.toml)
cd test-output/real-world/inspect/owner-repo
/path/to/target/release/agnix . --format json > ../owner-repo.json
```

### Step 2: Read the Config Files

Find all agent config files:
```bash
find . -name "CLAUDE.md" -o -name "AGENTS.md" -o -name ".cursorrules*" \
  -o -name "*.mdc" -path "*/.cursor/*" -o -name "settings.json" -path "*/.claude/*" \
  -o -name "mcp.json" -o -name "copilot-instructions.md"
```

Read each file carefully. Look for:
- Hard-coded paths (/Users/name/, /home/user/)
- Role-play personas ("You are a senior...")
- Generic instructions ("Be helpful", "Follow best practices")
- Negative instructions without alternatives ("Don't do X")
- Broken @import references
- Platform-specific features in cross-platform files
- Contradictory instructions
- HTML/XML tags that aren't actually XML
- Secrets, tokens, or private IPs
- LLM response artifacts ("Here's the updated...")
- Empty sections (header with no body)

### Step 3: Compare

For each issue you found:
1. Check if agnix flagged it. If not: **false negative**
2. For each agnix diagnostic, verify the actual file content. If wrong: **false positive**
3. Check if the diagnostic message is clear and actionable. If confusing: **message quality issue**

### Step 4: Fix and Verify

```bash
# Edit the validator code
# Rebuild
cargo build --release

# Re-run on the same repo
cd test-output/real-world/inspect/owner-repo
/path/to/target/release/agnix . --format json

# Run full test suite
cargo test --workspace
```

## What to Look For

### Common False Positives
| Pattern | Rule | Fix |
|---------|------|-----|
| HTML in markdown (br, img, details) | XML-001 | Add to void/safe element list |
| @mentions (Java annotations, social handles) | REF-001 | Filter by extension/domain |
| Type parameters (T, Option, HashMap) | XML-001 | Add to type parameter list |
| Path template placeholders (lib/\<feature\>/) | XML-001 | Check for adjacent `/` |
| Regex patterns in hook commands (\.py$) | CC-HK-008 | Filter regex metacharacters |

### Common False Negatives
| Pattern | Should trigger | Why missed |
|---------|---------------|------------|
| "You're a senior engineer" | CC-MEM-005 | Contraction not matched |
| `\|\| true` in hook commands | CC-HK-009 | Not in dangerous patterns |
| .cursorrules.md (with .md extension) | CUR-* | File type not detected |
| @import from project root | REF-001 (should NOT fire) | Resolution only tried file-relative |
| Prompt rules on cursor files | PE-*, CC-MEM-* | Validators not registered |

### Areas That Need More Work
- Contradictory instruction detection
- LLM response artifact detection
- Non-English negative keyword detection (Russian, Japanese, etc.)
- PE-004 firing on descriptive text vs instructions
- CC-MEM-006 firing when positive alternative exists in context

## Metrics

After a full run, analyze results:

```python
import json, os, collections
rules = collections.Counter()
for f in os.listdir('test-output/real-world/results'):
    try:
        data = json.load(open(f'test-output/real-world/results/{f}'))
    except: continue
    if not data.get('clone_success') or not data.get('agnix',{}).get('output'): continue
    for d in data['agnix']['output']['diagnostics']:
        rules[d['rule']] += 1

for rule, count in rules.most_common(30):
    print(f'{count:6d} {rule}')
```

### Quality Targets
- False positive rate: <5% (currently ~3%)
- Rules triggered: 70+ out of 100 (currently 71)
- Repos validated: 1,000+ (currently 1,224)
- All unit tests pass
- Clippy clean
