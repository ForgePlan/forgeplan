#!/usr/bin/env python3
"""F10 acceptance harness — 30 reference tasks x {local, federated, pack}.

Measures, per task x mode: latency ms, payload bytes, hit list, must_include /
must_not_include coverage, rank of the first useful hit, and the violation
flags the acceptance matrix names:

    source_boundary_violation  a hit attributed to a source that does not hold
                               that artifact (misattribution), or a local-mode
                               hit whose key does not exist in the searched
                               workspace at all (leak)
    version_substitution       a read served at a generation / source digest
                               different from the one CURRENT.json pinned at
                               setup (stale or substituted snapshot)
    incomplete_negative        a task with expected content produced an empty /
                               not-found answer
    negative_fabrication       a task with no answer produced hits or a found
                               artifact anyway
    omissions_reported         (pack) the pack's `omitted` list was non-empty

Writes raw per-invocation evidence, JSON aggregates and a Markdown report.
Exits non-zero on violations, budget refusals or missing required content.

Requires Python 3.9+, PyYAML and a semantic-search-enabled Forgeplan binary.
"""

import argparse
import hashlib
from contextlib import nullcontext
from datetime import date, datetime
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parent.parent
CORPUS = REPO / "eval" / "corpus"
WS_NAMES = ("ws-a", "ws-b")
BIN = REPO / "target" / "debug" / "forgeplan"
CURRENT_JSON = Path(".forgeplan") / "lance" / "CURRENT.json"

# The embedding model cache is per-machine, not per-run: HOME is redirected to
# a temp dir (so the constellation registry is hermetic), so the real cache
# location must be handed back explicitly.
MODEL_CACHE = str(Path.home() / ".forgeplan" / "models")

MODES = ("local", "federated", "pack")


# ─── tasks.yaml — restricted-subset parser ───────────────────────────────────
# Supported syntax (everything this file uses):
#   # comment                 blank lines ignored
#   - key: value              list item, scalar value
#     key: [a, "b:c", d]      inline list, quotes optional
#     key: "quoted string"    single/double quotes stripped
# No block scalars, no nesting, no anchors — deliberately.


def _scalar(raw: str):
    raw = raw.strip()
    if len(raw) >= 2 and raw[0] == raw[-1] and raw[0] in "\"'":
        return raw[1:-1]
    if raw == "true":
        return True
    if raw == "false":
        return False
    if re.fullmatch(r"-?\d+", raw):
        return int(raw)
    return raw


def _value(raw: str):
    raw = raw.strip()
    if raw.startswith("[") and raw.endswith("]"):
        inner = raw[1:-1].strip()
        if not inner:
            return []
        # split on commas not inside quotes
        parts, cur, q = [], "", None
        for ch in inner:
            if q:
                cur += ch
                if ch == q:
                    q = None
            elif ch in "\"'":
                q = ch
                cur += ch
            elif ch == ",":
                parts.append(cur)
                cur = ""
            else:
                cur += ch
        parts.append(cur)
        return [_scalar(p) for p in parts]
    return _scalar(raw)


def load_tasks(path: Path) -> list[dict]:
    tasks, cur = [], None
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line.startswith("- "):
            cur = {}
            tasks.append(cur)
            line = line[2:]
        m = re.match(r"^\s*([A-Za-z_]+):\s*(.*)$", line)
        if not m or cur is None:
            raise SystemExit(f"{path}:{lineno}: unparsable line {line!r}")
        cur[m.group(1)] = _value(m.group(2))
    for t in tasks:
        t.setdefault("kind", "search")
        t.setdefault("search_mode", "smart")
        t.setdefault("key", None)
        t.setdefault("fed_query", None)
        t.setdefault("pack_query", None)
        t.setdefault("expected_sources", [])
        t.setdefault("expected_artifacts", [])
        t.setdefault("must_include", [])
        t.setdefault("must_not_include", [])
        t.setdefault("expect_found", True)
        t.setdefault("budget", 2000)
        bad = [s for s in t["expected_sources"] if s not in WS_NAMES]
        if bad:
            raise SystemExit(f"{path}: task {t.get('id')}: unknown sources {bad}")
    return tasks


# ─── corpus manifest ─────────────────────────────────────────────────────────


def corpus_manifest() -> dict[str, set[str]]:
    """ws name -> set of canonical keys declared by its frontmatter `id:`."""
    out = {}
    for ws in WS_NAMES:
        keys = set()
        for md in (CORPUS / ws / ".forgeplan").rglob("*.md"):
            for line in md.read_text(encoding="utf-8").splitlines():
                m = re.match(r"^id:\s*(\S+)\s*$", line)
                if m:
                    keys.add(m.group(1))
                    break
        out[ws] = keys
    return out


# ─── subprocess plumbing ─────────────────────────────────────────────────────


class Run:
    __slots__ = ("argv", "cwd", "rc", "stdout", "stderr", "latency_ms")

    def __init__(self, argv, cwd, rc, stdout, stderr, latency_ms):
        self.argv, self.cwd, self.rc = argv, cwd, rc
        self.stdout, self.stderr, self.latency_ms = stdout, stderr, latency_ms


def forgeplan(args: list[str], cwd: Path, env: dict, timeout=300) -> Run:
    argv = [str(BIN), *args]
    t0 = time.perf_counter()
    p = subprocess.run(
        argv, cwd=str(cwd), env=env, capture_output=True, text=True, timeout=timeout
    )
    ms = (time.perf_counter() - t0) * 1000.0
    return Run(argv, cwd, p.returncode, p.stdout, p.stderr, ms)


def run_env(home: Path) -> dict:
    env = {
        key: value for key, value in os.environ.items()
        if not key.startswith("FORGEPLAN_")
    }
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["XDG_DATA_HOME"] = str(home)
    env["FORGEPLAN_MODEL_CACHE"] = MODEL_CACHE
    env["HF_HUB_OFFLINE"] = "1"
    return env


def fixture_basis() -> dict[str, str]:
    """Freeze the actual source bytes, independently of returned receipts."""
    return {
        str(path.relative_to(CORPUS)): hashlib.sha256(path.read_bytes()).hexdigest()
        for ws in WS_NAMES
        for path in sorted((ws_root(ws) / ".forgeplan").rglob("*"))
        if path.is_file() and (path.suffix == ".md" or path.name == "config.yaml")
    }


def require_success(run: Run) -> None:
    if run.rc:
        raise RuntimeError(f"{run.argv!r} exited {run.rc}: {run.stderr}")


def validate_version(source: dict, versions: dict) -> bool:
    ws = ws_of_path(source.get("path"))
    expected = versions.get(ws, {})
    return bool(
        expected
        and source.get("source_id") == expected.get("source_id")
        and source.get("version_key") == expected.get("pinned_version")
    )


# ─── corpus setup (idempotent) ────────────────────────────────────────────────


def ws_root(ws: str) -> Path:
    return CORPUS / ws


def read_current(ws: str):
    p = ws_root(ws) / CURRENT_JSON
    if not p.exists():
        return None
    try:
        return json.loads(p.read_text())
    except json.JSONDecodeError:
        return None


def setup_workspace(ws: str, env: dict, log, prepare_local: bool = True) -> None:
    root = ws_root(ws)
    # git checkout — constellation snapshot pins Git HEADs.
    if not (root / ".git").exists():
        subprocess.run(["git", "init", "-q", "."], cwd=root, check=True)
    head = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"], cwd=root, capture_output=True
    )
    if head.returncode != 0:
        subprocess.run(["git", "add", ".forgeplan"], cwd=root, check=True)
        subprocess.run(
            [
                "git", "-c", "user.email=eval@forgeplan.local", "-c",
                "user.name=Eli Rum", "commit", "-qm", "test: freeze evaluation corpus",
            ],
            cwd=root, check=True,
        )
        log(f"  [{ws}] git seed commit")
    if not (root / ".forgeplan" / "config.yaml").exists():
        require_success(forgeplan(["init", "-y"], root, env))
        log(f"  [{ws}] forgeplan init")
    if not prepare_local:
        return
    cur = read_current(ws)
    if cur is None:
        r = forgeplan(["index"], root, env)
        if r.rc != 0:
            raise SystemExit(f"[{ws}] forgeplan index failed:\n{r.stderr}")
        log(f"  [{ws}] index pass ({r.latency_ms:.0f} ms)")
        cur = read_current(ws)
    if cur is None:
        raise SystemExit(f"[{ws}] index ran but {CURRENT_JSON} still missing")
    # semantic readiness: vectors pinned means embedding heads recorded.
    if not cur.get("embedding_artifacts_head") or not cur.get("profile_id"):
        log(f"  [{ws}] embedding queue drain (model load, one-off)...")
        r = forgeplan(["embed"], root, env, timeout=1200)
        require_success(r)
        log(f"  [{ws}] embedding completed ({r.latency_ms:.0f} ms)")
        cur = read_current(ws) or cur


def load_snapshot_lock(path: Path) -> dict:
    lock = yaml.safe_load(path.read_text(encoding="utf-8"))
    if isinstance(lock.get("taken_at"), (date, datetime)):
        lock["taken_at"] = lock["taken_at"].isoformat()
    return lock


def setup_constellation(env: dict, log) -> dict:
    """Pin the private corpus and verify Git objects against the source bytes."""
    name = "f10-eval"
    require_success(forgeplan(["constellation", "create", name], CORPUS, env))
    for ws in WS_NAMES:
        require_success(forgeplan(
            ["constellation", "add", str(ws_root(ws) / ".forgeplan")], CORPUS, env,
        ))
    require_success(forgeplan(["constellation", "use", name], CORPUS, env))
    snap = forgeplan(["constellation", "snapshot"], CORPUS, env)
    require_success(snap)
    match = re.search(r"Snapshot (\S+)", snap.stdout)
    if not match:
        raise RuntimeError("snapshot command returned no snapshot id")
    snap_id = match.group(1)
    lock = load_snapshot_lock(
        Path(env["HOME"]) / ".forgeplan" / "snapshots" / f"{snap_id}.yaml"
    )
    if lock.get("skipped") or len(lock["sources"]) != len(WS_NAMES):
        raise RuntimeError("evaluation snapshot did not pin every source")
    versions = {"_snapshot": snap_id}
    for entry in lock["sources"]:
        ws = ws_of_path(entry["source_path"])
        if ws not in WS_NAMES or ws in versions:
            raise RuntimeError(f"unexpected snapshot source: {entry}")
        root = ws_root(ws)
        for md in (root / ".forgeplan").rglob("*.md"):
            result = subprocess.run(
                ["git", "show", f"{entry['git_oid']}:{md.relative_to(root)}"],
                cwd=root, check=True, capture_output=True,
            )
            if result.stdout != md.read_bytes():
                raise RuntimeError(f"local and pinned source bytes differ: {md}")
        source_id = "src:" + hashlib.sha256(
            f"{entry['checkout_root']}@{entry['git_oid']}".encode()
        ).hexdigest()[:16]
        cur = read_current(ws) or {}
        versions[ws] = {
            "generation_id": cur.get("generation_id"),
            "source_set_digest": cur.get("source_set_digest"),
            "source_id": source_id,
            "pinned_version": (
                f"{entry['git_oid']}:{entry['tree_oid']}:{entry['bytes_digest']}"
            ),
        }
    log(f"  snapshot: {snap_id}; all local Markdown matches pinned Git objects")
    return {"snapshot": snap_id, "versions": versions, "lock": lock}


# ─── output parsing ───────────────────────────────────────────────────────────


def doc_key(canonical: str) -> str:
    """`PRD-001#s2` (section row) -> `PRD-001`."""
    return canonical.split("#", 1)[0]


def ws_of_path(path):
    if not path:
        return None
    for ws in WS_NAMES:
        if f"/{ws}/" in path or path.endswith(f"/{ws}/.forgeplan"):
            return ws
    return None


class Outcome:
    def __init__(self, mode):
        self.mode = mode
        self.runs: list[Run] = []          # every subprocess, for audit
        self.hits: list[dict] = []         # {key, title, source_ws, evidence}
        self.result = "hits"               # hits | found | not_found | ambiguous | error
        self.evidence = ""                 # text scanned by must_* checks
        self.omitted: list = []            # pack omissions
        self.version_mismatch = False
        self.found_key = None
        self.note = ""
        self.explicit_incomplete = False
        self.contract_errors: list[str] = []
        self.unit_ids: set[str] = set()


def parse_search_local(run: Run, out: Outcome, ws: str, manifest):
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
        return
    for h in data.get("results", []):
        key = doc_key(h.get("document_key") or h.get("canonical_key") or "")
        title = h.get("title") or ""
        section = h.get("section") or ""
        out.hits.append(
            {"key": key, "title": title, "source_ws": ws,
             "evidence": " ".join([key, title, section])}
        )
    if not out.hits:
        out.result = "not_found"


def parse_search_fed(run: Run, out: Outcome, versions):
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
        return
    for h in data.get("hits", []):
        src = h.get("source") or {}
        ws = ws_of_path(src.get("path"))
        key = doc_key(h.get("canonical_key") or h.get("display_id") or "")
        title = h.get("title") or ""
        out.hits.append(
            {"key": key, "title": title, "source_ws": ws,
             "source_id": src.get("source_id"),
             "evidence": " ".join([key, title])}
        )
        if not validate_version(src, versions):
            out.version_mismatch = True
    if not out.hits:
        out.result = "not_found"


def parse_get_local(run: Run, out: Outcome, ws: str, cur_gen):
    if run.rc:
        out.result = "not_found" if re.search(r"not_found|not found", run.stderr, re.I) else "error"
        out.note = run.stderr
        return
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
        return
    if not isinstance(data, dict):
        out.result = "not_found" if data is None else "error"
        return
    key = doc_key(data.get("canonical_key") or data.get("id") or "")
    body = data.get("body") or ""
    title = data.get("title") or ""
    out.result = "found"
    out.found_key = key
    out.hits.append(
        {"key": key, "title": title, "source_ws": ws,
         "evidence": f"{key} {title} {body}"}
    )
    gen = data.get("generation_id")
    if cur_gen and gen and gen != cur_gen:
        out.version_mismatch = True


def parse_get_fed(run: Run, out: Outcome, versions):
    text = run.stdout
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        data = None
    if data is None:
        if re.search(r"claimed by (\d+) sources", text):
            out.result = "ambiguous"
            for m in re.finditer(r"(\S+?/\.forgeplan)\s+\((src:[0-9a-f]+)\)", text):
                out.hits.append(
                    {"key": None, "title": None, "source_ws": ws_of_path(m.group(1)),
                     "source_id": m.group(2), "evidence": m.group(1)}
                )
        elif re.search(r"not found", text + run.stderr, re.I):
            out.result = "not_found"
        else:
            out.result = "error"
            out.note = "unrecognized output"
        return
    if data.get("result") == "found":
        src = data.get("source") or {}
        ws = ws_of_path(src.get("path"))
        body = data.get("body") or ""
        declared = re.search(r"(?m)^id:\s*(\S+)\s*$", body)
        key = doc_key(data.get("canonical_key") or data.get("display_id")
                      or (declared.group(1) if declared else ""))
        out.result = "found"
        out.found_key = key
        out.hits.append(
            {"key": key, "title": data.get("title"), "source_ws": ws,
             "source_id": src.get("source_id"),
             "evidence": f"{key} {data.get('title') or ''} {data.get('body') or ''}"}
        )
        if not validate_version(src, versions):
            out.version_mismatch = True
    elif data.get("result") == "ambiguous":
        out.result = "ambiguous"
        by_id = {
            value["source_id"]: ws for ws, value in versions.items()
            if isinstance(value, dict) and "source_id" in value
        }
        for candidate in data.get("sources", []):
            out.hits.append({
                "key": data.get("id"), "title": None,
                "source_ws": by_id.get(candidate.get("source_id")),
                "source_id": candidate.get("source_id"),
                "evidence": data.get("id") or "",
            })
    else:
        out.result = "not_found"


def valid_whole_body_quote(raw: bytes, unit: dict) -> bool:
    """Resolve this pack profile's body-relative span independently of Core."""
    closing = re.match(rb"\A---[ \t]*\n.*?\n---[ \t]*(?:\n|$)", raw, re.DOTALL)
    if closing is None:
        return False
    body = raw[closing.end():].lstrip(b"\n")
    span = unit.get("span") or {}
    start, end = span.get("byte_start"), span.get("byte_end")
    if (span.get("coordinate_basis") != "artifact_body_utf8_bytes"
            or type(start) is not int or type(end) is not int
            or start != 0 or end != len(body)):
        return False
    return unit.get("excerpt", "").encode("utf-8") == body[start:end]


def parse_pack(run: Run, out: Outcome, versions):
    if run.rc:
        out.result = "refused" if "insufficient_budget" in run.stderr else "error"
        out.note = run.stderr
        return None
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
        return None
    receipt = data.get("receipt", {})
    actual = len(run.stdout.encode("utf-8"))
    if receipt.get("measured_bytes") != actual:
        out.contract_errors.append("receipt does not match emitted bytes")
    cap = receipt.get("byte_cap")
    if not isinstance(cap, int) or actual > cap:
        out.contract_errors.append("successful response exceeds or omits its byte cap")
    for field in ("source_complete", "traversal_complete", "delivery_complete"):
        if not isinstance(data.get(field), bool):
            out.contract_errors.append(f"missing completeness field {field}")
    out.explicit_incomplete = any(data.get(field) is False for field in (
        "source_complete", "traversal_complete", "delivery_complete",
    ))
    for source in data.get("sources", []):
        if not validate_version(source, versions):
            out.version_mismatch = True
    for unit in data.get("units", []):
        src = unit.get("source") or {}
        ws = ws_of_path(src.get("path"))
        key = doc_key(unit.get("canonical_key") or unit.get("display_id") or "")
        excerpt = unit.get("excerpt") or ""
        unit_id = unit.get("unit_id")
        if not unit_id or unit_id in out.unit_ids:
            out.contract_errors.append("missing or repeated evidence unit identity")
        out.unit_ids.add(unit_id)
        relative = Path(unit.get("relative_path") or "")
        if ws in WS_NAMES and not relative.is_absolute() and ".." not in relative.parts:
            root = ws_root(ws)
            source_file = (root / relative if relative.parts[:1] == (".forgeplan",)
                           else root / ".forgeplan" / relative)
            if not source_file.is_file():
                out.contract_errors.append("citation does not resolve to a corpus file")
            else:
                raw = source_file.read_bytes()
                if not valid_whole_body_quote(raw, unit):
                    out.contract_errors.append("quote or whole-body span differs from pinned bytes")
                if unit.get("source_digest") != hashlib.sha256(raw).hexdigest():
                    out.contract_errors.append("source digest differs from the cited file")
        else:
            out.contract_errors.append("citation escapes the admitted source")
        out.hits.append({
            "key": key, "title": unit.get("title"), "source_ws": ws,
            "source_id": src.get("source_id"), "role": unit.get("role"),
            "tokens": unit.get("tokens"),
            "evidence": f"{key} {unit.get('title') or ''} {excerpt}",
        })
        if not validate_version(src, versions):
            out.version_mismatch = True
    out.omitted.extend(data.get("omitted") or [])
    out.result = "hits" if out.hits else "not_found"
    return data.get("next_cursor")


# ─── task execution ───────────────────────────────────────────────────────────


def local_dirs_for(task) -> list[str]:
    """Where a local-mode task is asked: the ws that hold the answer — or, for
    negatives, every ws (the phantom could hide anywhere)."""
    return task["expected_sources"] or list(WS_NAMES)


def run_task_local(task, env, manifest, versions) -> Outcome:
    out = Outcome("local")
    failed = False
    for ws in local_dirs_for(task):
        cwd = ws_root(ws)
        first_new_hit = len(out.hits)
        if task["kind"] == "get":
            r = forgeplan(["get", task["key"], "--json"], cwd, env)
            out.runs.append(r)
            parse_get_local(r, out, ws, versions.get(ws, {}).get("generation_id"))
        else:
            args = ["search", task["question"], "--json"]
            if task["search_mode"] == "keyword":
                args.insert(2, "--keyword")
            elif task["search_mode"] == "semantic":
                args.insert(2, "--semantic")
            r = forgeplan(args, cwd, env)
            out.runs.append(r)
            parse_search_local(r, out, ws, manifest)
        for index in range(first_new_hit, len(out.hits)):
            out.hits[index]["received_in_run"] = len(out.runs) - 1
        failed = failed or out.result == "error"
    if failed:
        out.result = "error"
        return out
    # a multi-ws local fan-out merges hits; result is found if any leg found
    if task["kind"] == "get" and any(h["key"] for h in out.hits):
        out.result = "found"
        out.found_key = next(h["key"] for h in out.hits if h["key"])
    elif not out.hits:
        out.result = "not_found"
    return out


def run_task_federated(task, env, versions) -> Outcome:
    out = Outcome("federated")
    if task["kind"] == "get":
        r = forgeplan(["constellation", "get", task["key"], "--json",
                       "--snapshot", versions["_snapshot"]], CORPUS, env)
        out.runs.append(r)
        parse_get_fed(r, out, versions)
    else:
        # federated search is a *literal substring* match over captured text
        # (lexical_index.rs) — natural-language questions legitimately return
        # empty. `fed_query` carries the same information need as a corpus
        # substring; falling back to `question` keeps single-term tasks terse.
        q = task.get("fed_query") or task["question"]
        r = forgeplan(
            ["constellation", "search", q, "--json", "-n", "20",
             "--snapshot", versions["_snapshot"]],
            CORPUS, env,
        )
        out.runs.append(r)
        parse_search_fed(r, out, versions)
    return out


def run_task_pack(task, env, versions) -> Outcome:
    out = Outcome("pack")
    query = task.get("pack_query") or task["question"]
    base = [
        "constellation", "pack", query, "--budget", str(task["budget"]),
        "--snapshot", versions["_snapshot"], "--json",
    ]
    cursor = None
    cursors = set()
    for _ in range(100):
        args = [*base, "--cursor", cursor] if cursor else base
        run = forgeplan(args, CORPUS, env)
        out.runs.append(run)
        before = len(out.unit_ids)
        first_new_hit = len(out.hits)
        cursor = parse_pack(run, out, versions)
        for index in range(first_new_hit, len(out.hits)):
            out.hits[index]["received_in_run"] = len(out.runs) - 1
        if not cursor:
            return out
        if cursor in cursors or len(out.unit_ids) == before:
            out.contract_errors.append("continuation made no progress")
            return out
        cursors.add(cursor)
    out.contract_errors.append("continuation exceeded evaluation page bound")
    return out


# ─── scoring ──────────────────────────────────────────────────────────────────


def score(task, out: Outcome, manifest) -> dict:
    # Only returned evidence counts. Raw JSON may echo the requested phrase,
    # a missing target or an omission; none proves the phrase was retrieved.
    evidence = "\n".join(h["evidence"] for h in out.hits)
    found_inc = [s for s in task["must_include"] if s in evidence]
    missing_inc = [s for s in task["must_include"] if s not in evidence]
    bad = [s for s in task["must_not_include"] if s in evidence]

    exp_ws = set(task["expected_sources"])
    boundary_violation = False
    wrong_source_hits = 0
    for h in out.hits:
        ws = h.get("source_ws")
        key = h.get("key")
        if ws is None:
            boundary_violation = True
            continue
        if out.mode == "local":
            # a local hit must be a real artifact of the searched ws
            if key and key not in manifest.get(ws, set()):
                boundary_violation = True
        else:
            # federated/pack: hit attributed to a ws whose manifest lacks
            # the key = misattribution; a ws outside the registry = leak
            if key and key not in manifest.get(ws, set()):
                boundary_violation = True
            if exp_ws and ws not in exp_ws:
                wrong_source_hits += 1

    expected = set(task["expected_artifacts"])
    wrong_refs = [h["key"] for h in out.hits if h["key"] and h["key"] not in expected]

    first_useful = None
    first_useful_ms = None
    for i, h in enumerate(out.hits, 1):
        if (h["key"] and h["key"] in expected) or any(
            s in h["evidence"] for s in task["must_include"]
        ):
            first_useful = i
            response_index = h.get("received_in_run", 0 if len(out.runs) == 1 else None)
            if type(response_index) is int and 0 <= response_index < len(out.runs):
                first_useful_ms = round(sum(
                    out.runs[index].latency_ms for index in range(response_index + 1)
                ), 1)
            break

    expect = task["expect_found"]
    incomplete_negative = False
    negative_fabrication = False
    if expect is True:
        # expected content: empty result set / not-found is the violation
        if out.result in ("not_found", "hits") and not out.hits:
            incomplete_negative = bool(task["must_include"] or expected)
        if out.result == "not_found":
            incomplete_negative = bool(task["must_include"] or expected)
        if out.result == "error":
            incomplete_negative = True
    elif expect is False:
        if out.mode == "pack":
            # A pack never claims existence — it returns lexically nearest
            # real units. Fabrication would be a unit claiming the phantom
            # key itself; nearest-context noise is measured in wrong_refs.
            phantom = task.get("key")
            if phantom and any(h["key"] == phantom for h in out.hits):
                negative_fabrication = True
        elif out.result == "found" or (out.result == "hits" and out.hits):
            negative_fabrication = True
        if out.result == "ambiguous":
            negative_fabrication = False  # honest "cannot say which", still fine
    elif expect == "ambiguous":
        # The ambiguous contract is a federated-mode answer. Locally each ws
        # legitimately answers "found" for its own ADR-001; a ContextPack is
        # set-valued and has no disambiguation answer at all — for pack we
        # only ask that an ADR-001 unit exists when the task expects content.
        if out.mode == "federated":
            # must answer `ambiguous` AND name every expected source — an
            # ambiguity that undercounts sources is a hidden wrong answer
            named = {h.get("source_ws") for h in out.hits} - {None}
            if out.result != "ambiguous" or not exp_ws <= named:
                incomplete_negative = True
        elif out.mode == "local" and out.result != "found":
            incomplete_negative = True
        elif out.mode == "pack" and not any(
            h["key"] in expected for h in out.hits
        ):
            incomplete_negative = True

    violations = {
        "source_boundary_violation": boundary_violation,
        "version_substitution": out.version_mismatch,
        "incomplete_negative": incomplete_negative,
        "negative_fabrication": negative_fabrication,
        "pack_contract_violation": bool(out.contract_errors),
        "forbidden_content": bool(bad),
    }

    latency = sum(r.latency_ms for r in out.runs)
    payload = sum(len(r.stdout.encode("utf-8", "replace")) for r in out.runs)
    return {
        "task": task["id"],
        "mode": out.mode,
        "kind": task["kind"],
        "result": out.result,
        "hits": [{"key": h["key"], "source_ws": h.get("source_ws"),
                  "source_id": h.get("source_id")} for h in out.hits],
        "n_hits": len(out.hits),
        "must_include_found": found_inc,
        "must_include_missing": missing_inc,
        "must_not_include_hit": bad,
        "constraint_recall": (
            len(found_inc) / len(task["must_include"])
            if task["must_include"] else None
        ),
        "first_useful_rank": first_useful,
        "first_useful_source": (
            out.hits[first_useful - 1].get("source_ws") if first_useful else None
        ),
        "first_useful_ms": first_useful_ms,
        "wrong_ref_count": len(wrong_refs),
        "wrong_source_hits": wrong_source_hits,
        "omitted": out.omitted,
        "omissions_reported": bool(out.omitted),
        "latency_ms": round(latency, 1),
        "payload_bytes": payload,
        "contract_errors": out.contract_errors,
        "explicit_incomplete": out.explicit_incomplete,
        "exit_codes": [r.rc for r in out.runs],
        "read_count": len(out.runs),
        "raw_runs": [
            {"argv": r.argv, "cwd": str(r.cwd), "exit": r.rc,
             "stdout": r.stdout, "stderr": r.stderr, "latency_ms": r.latency_ms}
            for r in out.runs
        ],
        "commands": [" ".join(r.argv[1:]) for r in out.runs],
        "stderr_tail": [r.stderr.strip()[-300:] for r in out.runs if r.stderr.strip()],
        **violations,
        "violations_total": sum(1 for v in violations.values() if v),
    }


# ─── aggregation + report ─────────────────────────────────────────────────────


def pct(vals, q):
    if not vals:
        return None
    s = sorted(vals)
    k = (len(s) - 1) * q
    f, c = int(k), min(int(k) + 1, len(s) - 1)
    return round(s[f] + (s[c] - s[f]) * (k - f), 1)


def aggregate(rows, mode):
    sel = [r for r in rows if r["mode"] == mode]
    recall_tasks = [r for r in sel if r["constraint_recall"] is not None]
    full = sum(1 for r in recall_tasks if r["constraint_recall"] == 1.0)
    ranks = [r["first_useful_rank"] for r in sel if r["first_useful_rank"]]
    first_response_times = [r["first_useful_ms"] for r in sel
                            if r.get("first_useful_ms") is not None]
    return {
        "mode": mode,
        "tasks": len(sel),
        "constraint_recall_full": f"{full}/{len(recall_tasks)}",
        "constraint_recall_pct": round(100 * full / len(recall_tasks), 1) if recall_tasks else None,
        "wrong_refs": sum(r["wrong_ref_count"] for r in sel),
        "wrong_source_hits": sum(r["wrong_source_hits"] for r in sel),
        "latency_first_pass_p50_ms": pct(
            [r["latency_ms"] for r in sel if r.get("pass") == 0], 0.5),
        "latency_first_pass_p95_ms": pct(
            [r["latency_ms"] for r in sel if r.get("pass") == 0], 0.95),
        "latency_repeat_p50_ms": pct(
            [r["latency_ms"] for r in sel if r.get("pass", 0) > 0], 0.5),
        "latency_repeat_p95_ms": pct(
            [r["latency_ms"] for r in sel if r.get("pass", 0) > 0], 0.95),
        "payload_p50": pct([r["payload_bytes"] for r in sel], 0.5),
        "latency_p50_ms": pct([r["latency_ms"] for r in sel], 0.5),
        "latency_p95_ms": pct([r["latency_ms"] for r in sel], 0.95),
        "first_useful_rank_mean": round(statistics.mean(ranks), 2) if ranks else None,
        "first_useful_p50_ms": pct(first_response_times, 0.5),
        "first_useful_p95_ms": pct(first_response_times, 0.95),
        "first_useful_timed_tasks": len(first_response_times),
        "violations": {
            k: sum(1 for r in sel if r[k]) for k in
            ("source_boundary_violation", "version_substitution",
             "incomplete_negative", "negative_fabrication",
             "forbidden_content", "pack_contract_violation")
        },
        "omissions_tasks": [r["task"] for r in sel if r["omissions_reported"]],
    }


def write_report(rows, aggs, meta, path: Path):
    lines = [
        "# F10 acceptance report — local / federated / pack",
        "",
        f"- run: {meta['ts']}",
        f"- binary: `{meta['bin']}` ({meta['bin_version']})",
        f"- corpus: `{meta['corpus']}`",
        f"- snapshot: `{meta.get('snapshot')}`",
        f"- disk free at end: {meta['disk_free']}",
        "",
        "## Per-mode aggregates",
        "",
        "| mode | tasks | constraint-recall | wrong refs | wrong-src hits | payload p50 B | p50 ms | p95 ms | first-useful rank | first-useful p50/p95 ms (n) | boundary | version-subst | incomplete-neg | neg-fabrication | forbidden |",
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for a in aggs:
        if a["tasks"] == 0:
            continue
        v = a["violations"]
        lines.append(
            f"| {a['mode']} | {a['tasks']} | {a['constraint_recall_full']} "
            f"({a['constraint_recall_pct']}%) | {a['wrong_refs']} | "
            f"{a['wrong_source_hits']} | {a['payload_p50']} | {a['latency_p50_ms']} | "
            f"{a['latency_p95_ms']} | {a['first_useful_rank_mean']} | "
            f"{a['first_useful_p50_ms']}/{a['first_useful_p95_ms']} ({a['first_useful_timed_tasks']}) | "
            f"{v['source_boundary_violation']} | {v['version_substitution']} | "
            f"{v['incomplete_negative']} | {v['negative_fabrication']} | "
            f"{v['forbidden_content']} / pack-contract={v['pack_contract_violation']} |"
        )
    lines += ["", "## Violations detail", ""]
    any_v = False
    for r in rows:
        if r["violations_total"]:
            any_v = True
            flags = [k for k in ("source_boundary_violation", "version_substitution",
                                 "incomplete_negative", "negative_fabrication",
                                 "forbidden_content", "pack_contract_violation") if r[k]]
            lines.append(
                f"- **{r['task']} / {r['mode']}**: {', '.join(flags)}"
                f" — hits={r['hits'][:4]} missing={r['must_include_missing']}"
                f" forbidden={r['must_not_include_hit']} rc={r['exit_codes']}"
            )
    if not any_v:
        lines.append("none — zero violations on the set")
    lines += ["", "## Omissions (pack)", ""]
    for r in rows:
        if r["omissions_reported"]:
            lines.append(f"- {r['task']}: {len(r['omitted'])} omitted")
    lines += ["", "## Per-task rows", "",
              "| task | mode | result | hits | recall | rank1st | ms | B | viol |",
              "|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        rec = "-" if r["constraint_recall"] is None else f"{r['constraint_recall']:.2f}"
        lines.append(
            f"| {r['task']} | {r['mode']} | {r['result']} | {r['n_hits']} | {rec} | "
            f"{r['first_useful_rank'] or '-'} | {r['latency_ms']} | "
            f"{r['payload_bytes']} | {r['violations_total']} |"
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# ─── main ─────────────────────────────────────────────────────────────────────


def main():
    global BIN, CORPUS
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mode", choices=[*MODES, "all"], default="all")
    ap.add_argument("--out", default=str(REPO / "eval" / "results"))
    ap.add_argument("--tasks", default=str(CORPUS / "tasks.yaml"))
    ap.add_argument("--binary", type=Path, default=BIN)
    ap.add_argument("--keep-fixture", action="store_true",
                    help="retain the private temporary Git/index/HOME fixture for review")
    ap.add_argument("--repeats", type=int, default=2,
                    help="first pass plus repeats; no OS-cache purge (default: 2)")
    args = ap.parse_args()
    if not 2 <= args.repeats <= 10:
        ap.error("--repeats must be between 2 and 10")
    BIN = args.binary.resolve()
    if not BIN.is_file():
        ap.error(f"binary missing: {BIN}")
    ver = subprocess.run([str(BIN), "--version"], capture_output=True, text=True, check=True)
    digest = hashlib.sha256()
    with BIN.open("rb") as binary:
        for block in iter(lambda: binary.read(1024 * 1024), b""):
            digest.update(block)
    out_dir = Path(args.out).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)
    tasks_path = Path(args.tasks).resolve()
    tasks = load_tasks(tasks_path)
    if len(tasks) != 30 or len({task["id"] for task in tasks}) != 30:
        ap.error("F10 requires exactly thirty distinct frozen tasks")
    log = lambda *values: print(*values, flush=True)
    modes = list(MODES) if args.mode == "all" else [args.mode]
    ts = time.strftime("%Y%m%d-%H%M%S")
    rows = []
    original_corpus = CORPUS
    fixture_context = (
        nullcontext(tempfile.mkdtemp(prefix="forgeplan-f10-"))
        if args.keep_fixture else tempfile.TemporaryDirectory(prefix="forgeplan-f10-")
    )
    with fixture_context as scratch:
        CORPUS = Path(scratch) / "corpus"
        CORPUS.mkdir()
        # Copy only canonical fixture inputs, never an existing index/registry.
        for ws in WS_NAMES:
            source = original_corpus / ws / ".forgeplan"
            for path in source.rglob("*"):
                if path.is_file() and (path.suffix == ".md" or path.name == "config.yaml"):
                    destination = CORPUS / ws / ".forgeplan" / path.relative_to(source)
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(path, destination)
        home = Path(scratch) / "home"
        home.mkdir()
        env = run_env(home)
        frozen = fixture_basis()
        manifest = corpus_manifest()
        log(f"[setup] {len(tasks)} tasks; private corpus; binary {ver.stdout.strip()}")
        for ws in WS_NAMES:
            setup_workspace(ws, env, log, prepare_local="local" in modes)
        if fixture_basis() != frozen:
            raise RuntimeError("setup changed the canonical evaluation corpus")
        cinfo = setup_constellation(env, log)
        meta = {
            "ts": ts, "bin": str(BIN), "bin_version": ver.stdout.strip(),
            "binary_sha256": digest.hexdigest(), "corpus": str(original_corpus),
            "snapshot": cinfo["snapshot"], "snapshot_lock": cinfo["lock"],
            "corpus_sha256": frozen,
            "tasks_sha256": hashlib.sha256(tasks_path.read_bytes()).hexdigest(),
            "passes": args.repeats,
            "timing": "First pass and repeats use fresh CLI processes; OS cache is not purged.",
            "fixture_root": scratch, "fixture_retained": args.keep_fixture,
            "first_useful_timing": "Cumulative CLI subprocess runtimes through the first useful completed response; excludes evaluator overhead, not streaming time.",
        }
        # Persist the basis before expensive queries; a report failure must not
        # discard snapshot provenance after all rows have already executed.
        (out_dir / f"metadata-{ts}.json").write_text(
            json.dumps(meta, indent=2), encoding="utf-8",
        )
        jsonl = out_dir / f"results-{ts}.jsonl"
        with jsonl.open("w", encoding="utf-8") as output:
            for repetition in range(args.repeats):
                for task in tasks:
                    for mode in modes:
                        if fixture_basis() != frozen:
                            raise RuntimeError("evaluation source basis changed before a query")
                        if mode == "local":
                            outcome = run_task_local(task, env, manifest, cinfo["versions"])
                        elif mode == "federated":
                            outcome = run_task_federated(task, env, cinfo["versions"])
                        else:
                            outcome = run_task_pack(task, env, cinfo["versions"])
                        if fixture_basis() != frozen:
                            raise RuntimeError("evaluation source basis changed during a query")
                        row = score(task, outcome, manifest)
                        row["pass"] = repetition
                        rows.append(row)
                        output.write(json.dumps(row, ensure_ascii=False) + "\n")
                        output.flush()
                        log(f"  {task['id']} {mode} pass={repetition} "
                            f"{outcome.result} recall={row['constraint_recall']} "
                            f"bytes={row['payload_bytes']} violations={row['violations_total']}")
        disk = subprocess.run(["df", "-h", str(REPO)], capture_output=True, text=True, check=True)
        meta["disk_free"] = disk.stdout.splitlines()[-1].split()[3]
        aggregates = [aggregate(rows, mode) for mode in modes]
        (out_dir / f"summary-{ts}.json").write_text(
            json.dumps({"meta": meta, "aggregates": aggregates}, indent=2), encoding="utf-8",
        )
        write_report(rows, aggregates, meta, out_dir / f"report-{ts}.md")
    CORPUS = original_corpus
    violations = sum(row["violations_total"] for row in rows)
    refusals = sum(row["result"] == "refused" for row in rows)
    missing = sum(bool(row["must_include_missing"]) for row in rows)
    log(f"[done] {len(rows)} rows; {violations} violations; "
        f"{refusals} budget refusals; {missing} rows with unmet required content")
    # A bounded refusal is honest, but is not delivery success.
    return 1 if violations or refusals or missing else 0


if __name__ == "__main__":
    sys.exit(main())
