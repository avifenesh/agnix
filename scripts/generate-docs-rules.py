#!/usr/bin/env python3
"""Generate Docusaurus rule reference pages from knowledge-base/rules.json."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, Set

ROOT = Path(__file__).resolve().parents[1]
RULES_JSON = ROOT / "knowledge-base" / "rules.json"
OUTPUT_DIR = ROOT / "website" / "docs" / "rules" / "generated"
INDEX_PATH = ROOT / "website" / "docs" / "rules" / "index.md"
SITE_DATA_DIR = ROOT / "website" / "src" / "data"
SITE_DATA_PATH = SITE_DATA_DIR / "siteData.json"

CATEGORY_LABELS: Dict[str, str] = {
    "agent-skills": "Agent Skills",
    "claude-skills": "Claude Skills",
    "claude-hooks": "Claude Hooks",
    "claude-agents": "Claude Agents",
    "claude-memory": "Claude Memory",
    "agents-md": "AGENTS.md",
    "claude-plugins": "Claude Plugins",
    "copilot": "GitHub Copilot",
    "mcp": "MCP",
    "xml": "XML",
    "references": "References",
    "prompt-engineering": "Prompt Engineering",
    "cross-platform": "Cross-Platform",
    "cursor": "Cursor",
    "cline": "Cline",
    "codex": "Codex CLI",
    "gemini-cli": "Gemini CLI",
    "opencode": "OpenCode",
    "roo-code": "Roo Code",
    "version-awareness": "Version Awareness",
    "cursor-skills": "Cursor Skills",
    "cline-skills": "Cline Skills",
    "copilot-skills": "Copilot Skills",
    "codex-skills": "Codex Skills",
    "opencode-skills": "OpenCode Skills",
    "windsurf-skills": "Windsurf Skills",
    "kiro-skills": "Kiro Skills",
    "amp-skills": "Amp Skills",
    "amp-checks": "Amp Checks",
    "roo-code-skills": "Roo Code Skills",
}

TEMPLATES: Dict[str, Dict[str, str]] = {
    "agent-skills": {
        "invalid": """---\ndescription: Deploys production changes\n---\n\n# deploy\nUse the skill now.\n""",
        "valid": """---\nname: deploy-prod\ndescription: Deploy production with explicit checks\n---\n\n# deploy-prod\nRun rollout checks before deployment.\n""",
        "lang": "markdown",
    },
    "claude-skills": {
        "invalid": """---\nname: Deploy_Prod\ndescription: Deploys production changes\n---\n""",
        "valid": """---\nname: deploy-prod\ndescription: Deploy production with explicit checks\n---\n""",
        "lang": "markdown",
    },
    "claude-hooks": {
        "invalid": """{\n  \"hooks\": [\n    {\n      \"event\": \"PreToolUse\",\n      \"matcher\": \"*\"\n    }\n  ]\n}\n""",
        "valid": """{\n  \"hooks\": [\n    {\n      \"event\": \"PreToolUse\",\n      \"matcher\": \"Write\",\n      \"command\": \"./scripts/validate.sh\",\n      \"timeout\": 30\n    }\n  ]\n}\n""",
        "lang": "json",
    },
    "claude-agents": {
        "invalid": """---\nname: reviewer\n---\n""",
        "valid": """---\nname: reviewer\ndescription: Review code for correctness and tests\nmodel: sonnet\ntools: [Read, Grep, Bash]\n---\n""",
        "lang": "markdown",
    },
    "claude-memory": {
        "invalid": """# Memory\nAlways be helpful.\n""",
        "valid": """# Project Memory\n- Use Rust workspace conventions\n- Keep AGENTS.md and CLAUDE.md identical\n""",
        "lang": "markdown",
    },
    "agents-md": {
        "invalid": """# Instructions\nDo everything automatically.\n""",
        "valid": """## Project Instructions\n- Use AGENTS.md as instruction entrypoint\n- Keep commands explicit and test changes\n""",
        "lang": "markdown",
    },
    "claude-plugins": {
        "invalid": """{\n  \"name\": \"plugin\"\n}\n""",
        "valid": """{\n  \"name\": \"agnix-plugin\",\n  \"commands\": [\n    {\"name\": \"validate\", \"entrypoint\": \"./scripts/validate.sh\"}\n  ]\n}\n""",
        "lang": "json",
    },
    "copilot": {
        "invalid": """# Copilot Instructions\nWrite whatever code seems fine.\n""",
        "valid": """# Copilot Instructions\nUse project coding standards and keep tests updated.\n""",
        "lang": "markdown",
    },
    "mcp": {
        "invalid": """{\n  \"jsonrpc\": \"1.0\",\n  \"tools\": []\n}\n""",
        "valid": """{\n  \"jsonrpc\": \"2.0\",\n  \"tools\": [\n    {\n      \"name\": \"validate_file\",\n      \"description\": \"Validate one configuration file\",\n      \"inputSchema\": {\"type\": \"object\"}\n    }\n  ]\n}\n""",
        "lang": "json",
    },
    "xml": {
        "invalid": """<analysis><rule id=\"XML-001\"></analysis>\n""",
        "valid": """<analysis><rule id=\"XML-001\">ok</rule></analysis>\n""",
        "lang": "xml",
    },
    "references": {
        "invalid": """[Spec](./missing-file.md)\n""",
        "valid": """[Spec](./VALIDATION-RULES.md)\n""",
        "lang": "markdown",
    },
    "prompt-engineering": {
        "invalid": """Do the task quickly.\n""",
        "valid": """## Objective\nValidate AGENTS.md files for schema and policy compliance.\n\n## Output Format\nReturn JSON diagnostics grouped by file.\n""",
        "lang": "markdown",
    },
    "cross-platform": {
        "invalid": """Use only CLAUDE.md instructions and ignore AGENTS.md.\n""",
        "valid": """Use both CLAUDE.md and AGENTS.md with explicit precedence and conflict handling.\n""",
        "lang": "markdown",
    },
    "cursor": {
        "invalid": """# Rule\nNo metadata block\n""",
        "valid": """---\ndescription: Cursor rule for repository policy\n---\nUse project-specific guidance.\n""",
        "lang": "markdown",
    },
    "version-awareness": {
        "invalid": """Pin MCP schema to an outdated version without fallback behavior.\n""",
        "valid": """Declare supported version range and degrade gracefully outside the range.\n""",
        "lang": "markdown",
    },
    "cline": {
        "invalid": "# Rules\n",
        "valid": "# Project Rules\n\nFollow coding standards and write tests.\n",
        "lang": "markdown",
    },
    "codex": {
        "invalid": "",
        "valid": "[model]\nmodel = \"o4-mini\"\n",
        "lang": "toml",
    },
    "gemini-cli": {
        "invalid": "# Gemini\n",
        "valid": "# Gemini Instructions\n\nFollow project coding standards.\n",
        "lang": "markdown",
    },
    "opencode": {
        "invalid": "{}",
        "valid": """{"model": "claude-sonnet-4-5-20250929", "share": "manual"}\n""",
        "lang": "json",
    },
    "roo-code": {
        "invalid": "# Rules\n",
        "valid": "# Roo Rules\n\nFollow project coding and review policies.\n",
        "lang": "markdown",
    },
    "cline-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "codex-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "copilot-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "cursor-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "opencode-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "windsurf-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "kiro-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "amp-skills": {"invalid": "", "valid": "", "lang": "markdown"},
    "amp-checks": {"invalid": "", "valid": "", "lang": "markdown"},
    "roo-code-skills": {"invalid": "", "valid": "", "lang": "markdown"},
}


DEFAULT_TEMPLATE = {
    "invalid": "Configuration omitted required fields for this rule.",
    "valid": "Configuration includes required fields and follows the rule.",
    "lang": "text",
}


def slug(rule_id: str) -> str:
    return rule_id.lower()



def render_autofix(rule: dict) -> str:
    """Return a human-readable auto-fix label for a rule."""
    fix = rule.get("fix", {})
    if not fix.get("autofix"):
        return "No"
    safety = fix.get("fix_safety")
    if not safety:
        raise ValueError(f"Rule {rule['id']} has autofix=true but missing fix_safety")
    valid = ("safe", "unsafe", "safe/unsafe")
    if safety not in valid:
        raise ValueError(f"Rule {rule['id']} has invalid fix_safety: '{safety}'. Expected one of: {valid}")
    return f"Yes ({safety})"


def render_rule(rule: dict) -> str:
    rule_id = rule["id"]
    name = rule["name"]
    severity = rule["severity"]
    category = rule["category"]
    evidence = rule["evidence"]
    applies_to = evidence.get("applies_to", {})
    tests = evidence.get("tests", {})

    template = TEMPLATES.get(category, DEFAULT_TEMPLATE)
    # Prefer per-rule examples from rules.json; fall back to category template.
    # Normalize so each example ends with exactly one trailing newline.
    invalid = (rule.get("bad_example") or template["invalid"]).rstrip("\n") + "\n"
    valid = (rule.get("good_example") or template["valid"]).rstrip("\n") + "\n"
    lang = template["lang"]

    # Use longer fence when examples contain triple backticks to avoid breakout
    fence = "````" if ("```" in invalid or "```" in valid) else "```"

    sources = "\n".join(
        f"- {url}" for url in evidence.get("source_urls", [])
    ) or "- None listed"

    tool = applies_to.get("tool") or "all"
    version_range = applies_to.get("version_range") or "unspecified"
    spec_revision = applies_to.get("spec_revision") or "unspecified"

    unit = str(tests.get("unit", False)).lower()
    fixtures = str(tests.get("fixtures", False)).lower()
    e2e = str(tests.get("e2e", False)).lower()

    cat_label = CATEGORY_LABELS.get(category, category)

    # SEO-optimized title (under 60 chars when possible)
    seo_title = f"{rule_id}: {name} - {cat_label}"
    if len(seo_title) > 60:
        seo_title = f"{rule_id}: {name}"
    title = json.dumps(seo_title)
    sidebar_label = json.dumps(rule_id)

    # SEO meta description (120-160 chars)
    seo_desc = (
        f"agnix rule {rule_id} checks for {name.lower()} in {cat_label.lower()} files. "
        f"Severity: {severity}. See examples and fix guidance."
    )
    if len(seo_desc) > 160:
        seo_desc = seo_desc[:157] + "..."
    description = json.dumps(seo_desc)

    autofix_label = render_autofix(rule)

    # SEO keywords (quote each to avoid YAML colon issues)
    kw_list = [rule_id, name.lower(), cat_label.lower(), "validation", "agnix", "linter"]
    keywords = ", ".join(json.dumps(kw) for kw in kw_list)

    return f"""---
id: {slug(rule_id)}
title: {title}
sidebar_label: {sidebar_label}
description: {description}
keywords: [{keywords}]
---

## Summary

- **Rule ID**: `{rule_id}`
- **Severity**: `{severity}`
- **Category**: `{CATEGORY_LABELS.get(category, category)}`
- **Normative Level**: `{evidence.get('normative_level', 'UNKNOWN')}`
- **Auto-Fix**: `{autofix_label}`
- **Verified On**: `{evidence.get('verified_on', 'unknown')}`

## Applicability

- **Tool**: `{tool}`
- **Version Range**: `{version_range}`
- **Spec Revision**: `{spec_revision}`

## Evidence Sources

{sources}

## Test Coverage Metadata

- Unit tests: `{unit}`
- Fixture tests: `{fixtures}`
- E2E tests: `{e2e}`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

{fence}{lang}
{invalid}{fence}

### Valid

{fence}{lang}
{valid}{fence}
"""



def generate_site_data(data: dict) -> None:
    """Generate website/src/data/siteData.json from rules.json."""
    rules = data.get("rules", [])
    categories = data.get("categories", {})

    autofix_count = sum(1 for r in rules if r.get("fix", {}).get("autofix"))

    unique_tools: Set[str] = set()
    for r in rules:
        evidence = r.get("evidence") or {}
        tool = (evidence.get("applies_to") or {}).get("tool")
        if tool:
            unique_tools.add(tool)

    site_data = {
        "totalRules": data.get("total_rules", len(rules)),
        "categoryCount": len(categories),
        "autofixCount": autofix_count,
        "uniqueTools": sorted(unique_tools),
    }

    SITE_DATA_DIR.mkdir(parents=True, exist_ok=True)
    SITE_DATA_PATH.write_text(
        json.dumps(site_data, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Generated site data at {SITE_DATA_PATH}")


def main() -> int:
    with RULES_JSON.open("r", encoding="utf-8") as f:
        data = json.load(f)

    rules = data.get("rules", [])
    total_rules = data.get("total_rules", len(rules))

    def write_docs(target_output_dir: Path, target_index_path: Path) -> None:
        target_output_dir.mkdir(parents=True, exist_ok=True)
        for existing in target_output_dir.glob("*.md"):
            existing.unlink()

        autofix_count = sum(
            1 for r in rules if r.get("fix", {}).get("autofix")
        )

        lines = [
            "# Rules Reference",
            "",
            f"This section contains all `{total_rules}` validation rules generated from `knowledge-base/rules.json`.",
            f"`{autofix_count}` rules have automatic fixes.",
            "",
            "| Rule | Name | Severity | Category | Auto-Fix |",
            "|------|------|----------|----------|----------|",
        ]

        for rule in rules:
            rule_id = rule["id"]
            filename = f"{slug(rule_id)}.md"
            page_path = target_output_dir / filename
            page_path.write_text(render_rule(rule), encoding="utf-8")

            autofix_label = render_autofix(rule)
            lines.append(
                f"| [{rule_id}](./generated/{slug(rule_id)}.md) | {rule['name']} | {rule['severity']} | {CATEGORY_LABELS.get(rule['category'], rule['category'])} | {autofix_label} |"
            )

        target_index_path.parent.mkdir(parents=True, exist_ok=True)
        target_index_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    generate_site_data(data)
    write_docs(OUTPUT_DIR, INDEX_PATH)

    print(f"Generated {len(rules)} rule documentation pages in {OUTPUT_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
