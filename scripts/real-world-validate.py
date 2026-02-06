#!/usr/bin/env python3
"""Real-world validation harness for agnix.

Clones repos from a YAML manifest, runs agnix --format json against each,
and saves per-repo results. Designed for one-time validation runs.

Requires Python 3.9+ and PyYAML.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

try:
    import yaml
except ImportError:
    print("PyYAML required: pip install pyyaml", file=sys.stderr)
    sys.exit(1)


def load_repos(repos_file: Path, filter_pattern: str | None = None, category: str | None = None, status_filter: str = "pending"):
    with open(repos_file) as f:
        data = yaml.safe_load(f)

    repos = data.get("repos", [])

    if status_filter:
        repos = [r for r in repos if r.get("status", "pending") == status_filter]

    if category:
        repos = [r for r in repos if category in r.get("categories", [])]

    if filter_pattern:
        repos = [r for r in repos if filter_pattern.lower() in r["url"].lower()]

    return repos


def repo_slug(url: str) -> str:
    if not url:
        raise ValueError("Empty repository URL")
    parts = url.rstrip("/").split("/")
    if len(parts) < 2:
        raise ValueError(f"Malformed repository URL (need at least owner/repo): {url!r}")
    return f"{parts[-2]}--{parts[-1]}"


def clone_repo(url: str, clone_dir: Path, branch: str | None = None) -> tuple[bool, str]:
    if clone_dir.exists():
        return True, "already cloned"

    clone_dir.parent.mkdir(parents=True, exist_ok=True)

    cmd = ["git", "clone", "--depth", "1", "--single-branch"]
    if branch:
        cmd.extend(["--branch", branch])
    cmd.extend([url, str(clone_dir.resolve())])

    env = os.environ.copy()
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_TERMINAL_PROMPT"] = "0"

    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=120, env=env,
        )
        if result.returncode != 0:
            return False, result.stderr.strip()
        return True, "ok"
    except subprocess.TimeoutExpired:
        return False, "clone timeout"
    except Exception as e:
        return False, str(e)


def run_agnix(agnix_bin: str, repo_path: Path, timeout: int = 120) -> dict:
    cmd = [agnix_bin, str(repo_path), "--format", "json", "--max-files", "5000"]

    start = time.monotonic()
    try:
        # Run from the repo directory so agnix picks up the repo's .agnix.toml
        # (or uses defaults if none exists) rather than inheriting the parent project's config
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=timeout,
            cwd=str(repo_path) if repo_path.is_dir() else str(repo_path.parent),
        )
        wall_time_ms = int((time.monotonic() - start) * 1000)

        try:
            output = json.loads(result.stdout)
        except json.JSONDecodeError:
            output = None

        return {
            "exit_code": result.returncode,
            "output": output,
            "stderr": result.stderr.strip() if result.stderr else None,
            "wall_time_ms": wall_time_ms,
        }
    except subprocess.TimeoutExpired:
        return {
            "exit_code": -1,
            "output": None,
            "stderr": "agnix timeout",
            "wall_time_ms": timeout * 1000,
        }
    except Exception as e:
        return {
            "exit_code": -1,
            "output": None,
            "stderr": str(e),
            "wall_time_ms": 0,
        }


def process_repo(repo: dict, clones_dir: Path, results_dir: Path, agnix_bin: str, skip_clone: bool, timeout: int) -> dict:
    url = repo["url"]
    slug = repo_slug(url)
    clone_dir = clones_dir / slug
    result_file = results_dir / f"{slug}.json"

    if result_file.exists():
        with open(result_file) as f:
            return json.load(f)

    if not skip_clone:
        clone_ok, clone_msg = clone_repo(url, clone_dir, repo.get("branch"))
        if not clone_ok:
            result = {
                "url": url,
                "slug": slug,
                "categories": repo.get("categories", []),
                "clone_success": False,
                "clone_error": clone_msg,
                "agnix": None,
            }
            with open(result_file, "w") as f:
                json.dump(result, f, indent=2)
            return result

    if not clone_dir.exists():
        result = {
            "url": url,
            "slug": slug,
            "categories": repo.get("categories", []),
            "clone_success": False,
            "clone_error": "clone directory not found (use without --skip-clone)",
            "agnix": None,
        }
        with open(result_file, "w") as f:
            json.dump(result, f, indent=2)
        return result

    agnix_result = run_agnix(agnix_bin, clone_dir, timeout)

    result = {
        "url": url,
        "slug": slug,
        "categories": repo.get("categories", []),
        "clone_success": True,
        "agnix": agnix_result,
    }

    with open(result_file, "w") as f:
        json.dump(result, f, indent=2)

    return result


def print_summary(result: dict):
    url = result["url"]
    if not result["clone_success"]:
        print(f"  FAIL clone: {url} -- {result.get('clone_error', '?')}")
        return

    agnix = result.get("agnix", {})
    output = agnix.get("output")
    if output is None:
        print(f"  FAIL agnix: {url} -- {agnix.get('stderr', '?')}")
        return

    diags = output.get("diagnostics", [])
    summary = output.get("summary", {})
    files = output.get("files_checked", 0)
    time_ms = agnix.get("wall_time_ms", 0)

    error_count = summary.get("errors", 0)
    warn_count = summary.get("warnings", 0)
    info_count = summary.get("info", 0)

    rules_hit = {}
    for d in diags:
        rule = d.get("rule", "?")
        rules_hit[rule] = rules_hit.get(rule, 0) + 1

    rules_str = ", ".join(f"{r}({c})" for r, c in sorted(rules_hit.items()))

    status = "CLEAN" if not diags else f"E={error_count} W={warn_count} I={info_count}"
    print(f"  {status}: {url} [{files} files, {time_ms}ms] {rules_str}")


def main():
    parser = argparse.ArgumentParser(description="Real-world validation harness for agnix")
    parser.add_argument("--repos-file", default="tests/real-world/repos.yaml", help="Path to repos.yaml")
    parser.add_argument("--output-dir", default="test-output/real-world", help="Output directory")
    parser.add_argument("--agnix-bin", default="target/release/agnix", help="Path to agnix binary")
    parser.add_argument("--parallel", type=int, default=4, help="Parallel clone/validate workers")
    parser.add_argument("--timeout", type=int, default=120, help="Timeout per repo (seconds)")
    parser.add_argument("--skip-clone", action="store_true", help="Reuse existing clones")
    parser.add_argument("--filter", help="Filter repos by URL substring")
    parser.add_argument("--category", help="Filter repos by category")
    parser.add_argument("--limit", type=int, help="Limit number of repos to process")
    parser.add_argument("--status", default="pending", help="Filter by status (default: pending)")
    args = parser.parse_args()

    repos_file = Path(args.repos_file).resolve()
    if not repos_file.exists():
        print(f"Repos file not found: {repos_file}", file=sys.stderr)
        sys.exit(1)

    output_dir = Path(args.output_dir).resolve()
    clones_dir = output_dir / "clones"
    results_dir = output_dir / "results"
    clones_dir.mkdir(parents=True, exist_ok=True)
    results_dir.mkdir(parents=True, exist_ok=True)

    repos = load_repos(repos_file, args.filter, args.category, args.status)
    if args.limit:
        repos = repos[:args.limit]

    agnix_bin = Path(args.agnix_bin).resolve()
    if not agnix_bin.exists():
        print(f"agnix binary not found: {agnix_bin}", file=sys.stderr)
        sys.exit(1)

    if not repos:
        print("No repos matched filters.", file=sys.stderr)
        sys.exit(0)

    print(f"Processing {len(repos)} repos (parallel={args.parallel}, timeout={args.timeout}s)")

    results = []
    done = 0
    failed = 0

    with ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = {
            pool.submit(
                process_repo, repo, clones_dir, results_dir, str(agnix_bin), args.skip_clone, args.timeout
            ): repo
            for repo in repos
        }

        for future in as_completed(futures):
            done += 1
            try:
                result = future.result()
                results.append(result)
                print_summary(result)
            except Exception as e:
                failed += 1
                repo = futures[future]
                print(f"  ERROR: {repo['url']} -- {e}")

            if done % 10 == 0:
                print(f"--- Progress: {done}/{len(repos)} done, {failed} failures ---")

    print(f"\nDone: {done}/{len(repos)} repos, {failed} failures")

    # Write aggregate summary
    total_diags = 0
    rules_global = {}
    clean_repos = 0
    for r in results:
        if not r.get("clone_success"):
            continue
        agnix = r.get("agnix", {})
        output = agnix.get("output")
        if output is None:
            continue
        diags = output.get("diagnostics", [])
        if not diags:
            clean_repos += 1
        total_diags += len(diags)
        for d in diags:
            rule = d.get("rule", "?")
            rules_global[rule] = rules_global.get(rule, 0) + 1

    print(f"\nTotal diagnostics: {total_diags}")
    print(f"Clean repos: {clean_repos}/{len(results)}")
    print("\nTop rules by frequency:")
    for rule, count in sorted(rules_global.items(), key=lambda x: -x[1])[:20]:
        print(f"  {rule}: {count}")


if __name__ == "__main__":
    main()
