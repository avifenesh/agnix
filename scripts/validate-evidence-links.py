#!/usr/bin/env python3
"""Validate evidence source URLs in knowledge-base/rules.json."""

from __future__ import annotations

import argparse
import json
import re
import socket
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_RULES_PATH = ROOT / "knowledge-base" / "rules.json"
USER_AGENT = "agnix-evidence-link-checker/1.0"
RETRYABLE_STATUSES = {408, 429, 500, 502, 503, 504}
TITLE_RE = re.compile(r"<title[^>]*>(.*?)</title>", re.IGNORECASE | re.DOTALL)
TAG_RE = re.compile(r"<[^>]+>")
WHITESPACE_RE = re.compile(r"\s+")
NOT_FOUND_H1_RE = re.compile(
    r"<h1[^>]*>\s*(404(?:\s+not\s+found)?|page\s+not\s+found|not\s+found)\s*</h1>",
    re.IGNORECASE,
)


@dataclass
class ProbeResult:
    status: int | None
    method: str
    final_url: str | None
    content_type: str
    body: str
    error: str | None


@dataclass
class UrlResult:
    ok: bool
    method: str
    status: int | None
    detail: str
    retryable: bool
    final_url: str | None


def normalize_spaces(value: str) -> str:
    return WHITESPACE_RE.sub(" ", value).strip()


def decode_body(raw: bytes, content_type: str) -> str:
    if not raw:
        return ""
    charset_match = re.search(r"charset=([^\s;]+)", content_type, re.IGNORECASE)
    encodings = []
    if charset_match:
        encodings.append(charset_match.group(1).strip("\"'"))
    encodings.extend(["utf-8", "latin-1"])
    for encoding in encodings:
        try:
            return raw.decode(encoding, errors="replace")
        except LookupError:
            continue
    return raw.decode("utf-8", errors="replace")


def fetch(url: str, method: str, timeout: float, max_body_bytes: int) -> ProbeResult:
    request = Request(
        url,
        method=method,
        headers={
            "User-Agent": USER_AGENT,
            "Accept": "text/html,application/json,*/*;q=0.1",
        },
    )
    try:
        with urlopen(request, timeout=timeout) as response:
            raw = response.read(max_body_bytes) if method == "GET" else b""
            content_type = response.headers.get("Content-Type", "")
            return ProbeResult(
                status=response.getcode(),
                method=method,
                final_url=response.geturl(),
                content_type=content_type,
                body=decode_body(raw, content_type),
                error=None,
            )
    except HTTPError as error:
        raw = b""
        if method == "GET":
            try:
                raw = error.read(max_body_bytes)
            except OSError:
                raw = b""
        content_type = error.headers.get("Content-Type", "") if error.headers else ""
        return ProbeResult(
            status=error.code,
            method=method,
            final_url=error.geturl() if hasattr(error, "geturl") else url,
            content_type=content_type,
            body=decode_body(raw, content_type),
            error=f"HTTP {error.code}",
        )
    except (URLError, socket.timeout, TimeoutError, OSError) as error:
        reason = getattr(error, "reason", error)
        return ProbeResult(
            status=None,
            method=method,
            final_url=None,
            content_type="",
            body="",
            error=f"{type(error).__name__}: {reason}",
        )


def is_html(content_type: str) -> bool:
    lowered = content_type.lower()
    return "text/html" in lowered or "application/xhtml+xml" in lowered


def not_found_heuristic(html: str) -> str | None:
    if not html:
        return None
    title_match = TITLE_RE.search(html)
    if title_match:
        title_text = normalize_spaces(TAG_RE.sub(" ", title_match.group(1)).lower())
        if "404" in title_text and "not found" in title_text:
            return "HTML title indicates 404"
        if "page not found" in title_text:
            return "HTML title indicates missing page"
    # Keep body checks conservative to avoid false positives from script bundles
    # that include generic fallback strings not shown to end-users.
    if NOT_FOUND_H1_RE.search(html):
        return "HTML h1 indicates missing page"
    return None


def evaluate_probe(probe: ProbeResult) -> UrlResult:
    if probe.status is None:
        return UrlResult(
            ok=False,
            method=probe.method,
            status=None,
            detail=probe.error or "network failure",
            retryable=True,
            final_url=probe.final_url,
        )
    if probe.status >= 400:
        return UrlResult(
            ok=False,
            method=probe.method,
            status=probe.status,
            detail=probe.error or f"HTTP {probe.status}",
            retryable=probe.status in RETRYABLE_STATUSES,
            final_url=probe.final_url,
        )
    if probe.method == "GET" and is_html(probe.content_type):
        heuristic = not_found_heuristic(probe.body)
        if heuristic:
            return UrlResult(
                ok=False,
                method=probe.method,
                status=probe.status,
                detail=heuristic,
                retryable=False,
                final_url=probe.final_url,
            )
    return UrlResult(
        ok=True,
        method=probe.method,
        status=probe.status,
        detail="ok",
        retryable=False,
        final_url=probe.final_url,
    )


def check_once(url: str, timeout: float, max_body_bytes: int) -> UrlResult:
    head_probe = fetch(url, "HEAD", timeout, max_body_bytes)
    head_result = evaluate_probe(head_probe)
    if head_result.ok and not is_html(head_probe.content_type):
        return head_result
    get_probe = fetch(url, "GET", timeout, max_body_bytes)
    get_result = evaluate_probe(get_probe)
    if head_result.ok:
        get_result.method = "GET (verify-html)"
    elif get_result.method == "GET":
        get_result.method = "GET (fallback)"
    if (not get_result.ok) and head_probe.status and head_probe.status >= 400 and get_probe.status is None:
        return head_result
    return get_result


def check_with_retries(
    url: str, retries: int, retry_delay: float, timeout: float, max_body_bytes: int
) -> tuple[UrlResult, int]:
    attempts = retries + 1
    result = UrlResult(False, "HEAD", None, "uninitialized", True, None)
    for attempt in range(1, attempts + 1):
        result = check_once(url, timeout, max_body_bytes)
        if result.ok:
            return result, attempt
        if result.retryable and attempt < attempts:
            time.sleep(retry_delay * attempt)
            continue
        return result, attempt
    return result, attempts


def load_unique_urls(path: Path) -> tuple[list[str], list[str]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    rules = data.get("rules", [])
    urls: set[str] = set()
    invalid: set[str] = set()
    for rule in rules:
        evidence = rule.get("evidence") or {}
        source_urls = evidence.get("source_urls") or []
        for raw in source_urls:
            if not isinstance(raw, str):
                invalid.add(f"{rule.get('id', '<unknown>')}: non-string URL entry")
                continue
            url = raw.strip()
            if not url:
                invalid.add(f"{rule.get('id', '<unknown>')}: empty URL entry")
                continue
            parsed = urlparse(url)
            if parsed.scheme not in {"http", "https"} or not parsed.netloc:
                invalid.add(f"{rule.get('id', '<unknown>')}: invalid URL '{url}'")
                continue
            urls.add(url)
    return sorted(urls), sorted(invalid)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rules", type=Path, default=DEFAULT_RULES_PATH)
    parser.add_argument("--timeout", type=float, default=12.0, help="Request timeout in seconds")
    parser.add_argument("--retries", type=int, default=2, help="Retries for transient failures")
    parser.add_argument("--retry-delay", type=float, default=1.0, help="Base retry sleep in seconds")
    parser.add_argument(
        "--max-body-bytes",
        type=int,
        default=131072,
        help="Max bytes read for GET body inspection",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    urls, invalid_entries = load_unique_urls(args.rules)
    broken = 0
    ok = 0

    for entry in invalid_entries:
        broken += 1
        print(f"BROKEN [-] parse {entry}")

    for url in urls:
        result, attempt = check_with_retries(
            url=url,
            retries=args.retries,
            retry_delay=args.retry_delay,
            timeout=args.timeout,
            max_body_bytes=args.max_body_bytes,
        )
        status = str(result.status) if result.status is not None else "-"
        redirect = f" -> {result.final_url}" if result.final_url and result.final_url != url else ""
        if result.ok:
            ok += 1
            print(f"OK     [{status}] {result.method} {url}{redirect}")
        else:
            broken += 1
            print(f"BROKEN [{status}] {result.method} {url} ({result.detail}; attempts={attempt}){redirect}")

    total = len(urls) + len(invalid_entries)
    print(f"Summary: checked={total} ok={ok} broken={broken}")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
