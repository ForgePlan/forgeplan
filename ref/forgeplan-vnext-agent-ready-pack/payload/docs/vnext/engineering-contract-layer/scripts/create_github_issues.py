#!/usr/bin/env python3
"""Create/update ForgePlan vNext GitHub issues from the implementation pack.

Requirements:
  - Python 3.10+
  - GitHub CLI (`gh`) installed
  - `gh auth login` completed with issue write access to ForgePlan repositories

The script is idempotent by exact repository + title match. It also applies
GitHub native parent/sub-issue and blocked-by relationships when the connected
GitHub account/repository supports them. Body links remain the portable fallback.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BODIES_PATH = ROOT / "issues" / "bodies.json"
MANIFEST_PATH = ROOT / "issues" / "manifest.json"
STATE_PATH = ROOT / ".created-issues.json"
MASTER_KEY = "FPV-00"


def run(*args: str, input_text: str | None = None, check: bool = True) -> str:
    proc = subprocess.run(
        ["gh", *args],
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and proc.returncode != 0:
        raise RuntimeError(
            f"Command failed: gh {' '.join(args)}\n"
            f"stdout:\n{proc.stdout}\n"
            f"stderr:\n{proc.stderr}"
        )
    if not check and proc.returncode != 0:
        return ""
    return proc.stdout.strip()


def require_gh() -> None:
    if shutil.which("gh") is None:
        raise SystemExit(
            "GitHub CLI is not installed. On macOS run: brew install gh\n"
            "Then authenticate: gh auth login"
        )
    try:
        run("auth", "status")
    except RuntimeError as exc:
        raise SystemExit(f"GitHub CLI is not authenticated:\n{exc}") from exc


def list_issues(repo: str) -> list[dict[str, Any]]:
    raw = run(
        "issue", "list",
        "--repo", repo,
        "--state", "all",
        "--limit", "1000",
        "--json", "number,title,url,state",
    )
    return json.loads(raw)


def find_exact(repo: str, title: str, cache: dict[str, list[dict[str, Any]]]) -> dict[str, Any] | None:
    if repo not in cache:
        cache[repo] = list_issues(repo)
    for issue in cache[repo]:
        if issue["title"] == title:
            return issue
    return None


def create_issue(repo: str, title: str, body: str) -> dict[str, Any]:
    url = run(
        "issue", "create",
        "--repo", repo,
        "--title", title,
        "--body-file", "-",
        input_text=body,
    )
    number = int(url.rstrip("/").split("/")[-1])
    return {"number": number, "title": title, "url": url, "state": "OPEN"}


def edit_issue(repo: str, number: int, body: str) -> None:
    run(
        "issue", "edit", str(number),
        "--repo", repo,
        "--body-file", "-",
        input_text=body,
    )


def program_links(key: str, manifest_by_key: dict[str, dict[str, Any]], created: dict[str, dict[str, Any]]) -> str:
    item = manifest_by_key[key]
    lines = ["## Program Links"]
    if key != MASTER_KEY:
        lines.append(f"- Master epic: {created[MASTER_KEY]['url']}")
    deps = item.get("depends", [])
    if deps:
        lines.append("- Dependencies:")
        for dep in deps:
            lines.append(f"  - {dep}: {created[dep]['url']}")
    else:
        lines.append("- Dependencies: none")
    return "\n".join(lines)


def master_body(base: str, manifest: list[dict[str, Any]], created: dict[str, dict[str, Any]]) -> str:
    lines = [base.rstrip(), "", "## Child Issues"]
    children = [x for x in manifest if x["key"] != MASTER_KEY]
    children.sort(key=lambda x: (x["phase"], x["key"]))
    current_phase = None
    for item in children:
        if item["phase"] != current_phase:
            current_phase = item["phase"]
            lines.extend(["", f"### Phase {current_phase}"])
        linked = created[item["key"]]
        lines.append(f"- [ ] {item['key']} — {linked['url']} — {item['summary']}")
    lines.extend([
        "",
        "## Execution Order",
        "See `docs/vnext/engineering-contract-layer/governance/EXECUTION-ORDER.md` in the implementation pack.",
    ])
    return "\n".join(lines) + "\n"


def issue_relationships(repo: str, number: int) -> dict[str, Any]:
    raw = run(
        "issue", "view", str(number),
        "--repo", repo,
        "--json", "parent,blockedBy",
        check=False,
    )
    if not raw:
        return {}
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return {}


def apply_native_relationships(item: dict[str, Any], created: dict[str, dict[str, Any]]) -> None:
    """Best-effort: bodies always retain portable links even if feature unavailable."""
    key, repo = item["key"], item["repo"]
    issue = created[key]
    rel = issue_relationships(repo, issue["number"])

    parent = rel.get("parent") or {}
    parent_url = parent.get("url") if isinstance(parent, dict) else None
    wanted_parent = created[MASTER_KEY]["url"]
    if parent_url != wanted_parent:
        out = run(
            "issue", "edit", str(issue["number"]),
            "--repo", repo,
            "--parent", wanted_parent,
            check=False,
        )
        if out:
            print(f"PARENT  {key}")
        else:
            print(f"WARN    {key}: native parent relation unavailable; body link retained")

    blocked = rel.get("blockedBy") or []
    blocked_urls = {
        x.get("url") for x in blocked if isinstance(x, dict) and x.get("url")
    }
    for dep in item.get("depends", []):
        dep_url = created[dep]["url"]
        if dep_url in blocked_urls:
            continue
        out = run(
            "issue", "edit", str(issue["number"]),
            "--repo", repo,
            "--add-blocked-by", dep_url,
            check=False,
        )
        if out:
            print(f"BLOCKED {key} <- {dep}")
        else:
            print(f"WARN    {key}: native blocked-by relation to {dep} unavailable; body link retained")


def main() -> int:
    require_gh()
    bodies = json.loads(BODIES_PATH.read_text(encoding="utf-8"))
    manifest_data = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    manifest = manifest_data["issues"]
    by_key = {x["key"]: x for x in manifest}
    cache: dict[str, list[dict[str, Any]]] = {}
    created: dict[str, dict[str, Any]] = {}

    # Create/find the master first, so children can be native sub-issues.
    master = by_key[MASTER_KEY]
    master_existing = find_exact(master["repo"], master["title"], cache)
    if master_existing:
        created[MASTER_KEY] = master_existing
        print(f"FOUND   {MASTER_KEY} {master_existing['url']}")
    else:
        master_issue = create_issue(master["repo"], master["title"], bodies[MASTER_KEY]["body"])
        created[MASTER_KEY] = master_issue
        cache.setdefault(master["repo"], []).append(master_issue)
        print(f"CREATED {MASTER_KEY} {master_issue['url']}")

    # Children in dependency-friendly phase/key order.
    children = [x for x in manifest if x["key"] != MASTER_KEY]
    children.sort(key=lambda x: (x["phase"], x["key"]))
    for item in children:
        key, repo, title = item["key"], item["repo"], item["title"]
        existing = find_exact(repo, title, cache)
        if existing:
            created[key] = existing
            print(f"FOUND   {key} {existing['url']}")
        else:
            issue = create_issue(repo, title, bodies[key]["body"])
            created[key] = issue
            cache.setdefault(repo, []).append(issue)
            print(f"CREATED {key} {issue['url']}")

    # Update master with real child links.
    edit_issue(
        master["repo"],
        created[MASTER_KEY]["number"],
        master_body(bodies[MASTER_KEY]["body"], manifest, created),
    )
    print(f"UPDATED {MASTER_KEY} child checklist")

    # Update child bodies and apply native relationships where supported.
    for item in children:
        key = item["key"]
        body = bodies[key]["body"].rstrip() + "\n\n" + program_links(key, by_key, created) + "\n"
        edit_issue(item["repo"], created[key]["number"], body)
        print(f"LINKED  {key}")
        apply_native_relationships(item, created)

    STATE_PATH.write_text(json.dumps(created, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"\nState written to {STATE_PATH}")
    print(f"Master epic: {created[MASTER_KEY]['url']}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(exc, file=sys.stderr)
        raise SystemExit(1)
