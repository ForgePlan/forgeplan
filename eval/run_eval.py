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

Writes eval/results/results-<ts>.jsonl + report-<ts>.md. Exits non-zero when
any violation flag is non-zero.

Stdlib only. Requires the forgeplan binary built with --features semantic-search.
"""

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

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
    env = dict(os.environ)
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["XDG_DATA_HOME"] = str(home)
    env["FORGEPLAN_MODEL_CACHE"] = MODEL_CACHE
    return env


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


def setup_workspace(ws: str, env: dict, log) -> None:
    root = ws_root(ws)
    # git checkout — constellation snapshot pins Git HEADs.
    if not (root / ".git").exists():
        subprocess.run(["git", "init", "-q", "."], cwd=root, check=True)
    head = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"], cwd=root, capture_output=True
    )
    if head.returncode != 0:
        subprocess.run(["git", "add", "-A"], cwd=root, check=True)
        subprocess.run(
            [
                "git", "-c", "user.email=eval@forgeplan.local", "-c",
                "user.name=f10-eval", "commit", "-qm", "corpus seed",
            ],
            cwd=root, check=True,
        )
        log(f"  [{ws}] git seed commit")
    if not (root / ".forgeplan" / "config.yaml").exists():
        forgeplan(["init", "-y"], root, env)
        log(f"  [{ws}] forgeplan init")
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
        if r.rc != 0:
            log(f"  [{ws}] WARN embed rc={r.rc}: {r.stderr.strip()[:300]}")
        else:
            tail = [ln for ln in r.stdout.splitlines() if ln.startswith("Done:")]
            log(f"  [{ws}] {tail[-1] if tail else 'embed done'} ({r.latency_ms:.0f} ms)")
        cur = read_current(ws) or cur


def setup_constellation(env: dict, log) -> dict:
    """Create/use the eval constellation under the isolated HOME. Returns
    ws-name -> published-version map for version-substitution checks."""
    name = "f10-eval"
    r = forgeplan(["constellation", "list"], CORPUS, env)
    if name not in r.stdout:
        forgeplan(["constellation", "create", name], CORPUS, env)
        for ws in WS_NAMES:
            r = forgeplan(
                ["constellation", "add", str(ws_root(ws) / ".forgeplan")], CORPUS, env
            )
            if r.rc != 0:
                raise SystemExit(f"constellation add {ws} failed:\n{r.stderr}")
        forgeplan(["constellation", "use", name], CORPUS, env)
        log(f"  constellation '{name}': created, {len(WS_NAMES)} sources")
    else:
        log(f"  constellation '{name}': reusing")
    # pin the snapshot for the record (live reads still go through `pinned`
    # channel on the live generation — the snapshot is the fixed point F10
    # asks all three modes to share)
    snap = forgeplan(["constellation", "snapshot"], CORPUS, env)
    m = re.search(r"Snapshot (\S+)", snap.stdout)
    snap_id = m.group(1) if m else None
    if snap_id:
        log(f"  snapshot: {snap_id}")
    versions = {}
    for ws in WS_NAMES:
        cur = read_current(ws) or {}
        versions[ws] = {
            "generation_id": cur.get("generation_id"),
            "source_set_digest": cur.get("source_set_digest"),
            "profile_id": cur.get("profile_id"),
        }
    return {"snapshot": snap_id, "versions": versions}


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
        exp = versions.get(ws, {}).get("generation_id") if ws else None
        if exp and src.get("version_key") != exp:
            out.version_mismatch = True
    if not out.hits:
        out.result = "not_found"


def parse_get_local(run: Run, out: Outcome, ws: str, cur_gen):
    if run.rc != 0 or re.search(r"not found", run.stdout + run.stderr, re.I):
        out.result = "not_found"
        return
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
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
        key = doc_key(data.get("canonical_key") or data.get("display_id") or "")
        out.result = "found"
        out.found_key = key
        out.hits.append(
            {"key": key, "title": data.get("title"), "source_ws": ws,
             "source_id": src.get("source_id"),
             "evidence": f"{key} {data.get('title') or ''} {data.get('body') or ''}"}
        )
        exp = versions.get(ws, {}).get("generation_id") if ws else None
        if exp and src.get("version_key") != exp:
            out.version_mismatch = True
    else:
        out.result = "not_found"


def parse_pack(run: Run, out: Outcome, versions):
    try:
        data = json.loads(run.stdout)
    except json.JSONDecodeError:
        out.result = "error"
        out.note = "non-JSON stdout"
        return
    for u in data.get("units", []):
        src = u.get("source") or {}
        ws = ws_of_path(src.get("path"))
        key = doc_key(u.get("canonical_key") or u.get("display_id") or "")
        excerpt = u.get("excerpt") or ""
        out.hits.append(
            {"key": key, "title": u.get("title"), "source_ws": ws,
             "source_id": src.get("source_id"), "role": u.get("role"),
             "tokens": u.get("tokens"),
             "evidence": f"{key} {u.get('title') or ''} {excerpt}"}
        )
        exp = versions.get(ws, {}).get("source_set_digest") if ws else None
        if exp and src.get("version_key") != exp:
            out.version_mismatch = True
    out.omitted = data.get("omitted", []) or []
    if not out.hits:
        out.result = "not_found"


# ─── task execution ───────────────────────────────────────────────────────────


def local_dirs_for(task) -> list[str]:
    """Where a local-mode task is asked: the ws that hold the answer — or, for
    negatives, every ws (the phantom could hide anywhere)."""
    return task["expected_sources"] or list(WS_NAMES)


def run_task_local(task, env, manifest, versions) -> Outcome:
    out = Outcome("local")
    for ws in local_dirs_for(task):
        cwd = ws_root(ws)
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
        r = forgeplan(["constellation", "get", task["key"], "--json"], CORPUS, env)
        out.runs.append(r)
        parse_get_fed(r, out, versions)
    else:
        # federated search is a *literal substring* match over captured text
        # (lexical_index.rs) — natural-language questions legitimately return
        # empty. `fed_query` carries the same information need as a corpus
        # substring; falling back to `question` keeps single-term tasks terse.
        q = task.get("fed_query") or task["question"]
        r = forgeplan(
            ["constellation", "search", q, "--json", "-n", "20"],
            CORPUS, env,
        )
        out.runs.append(r)
        parse_search_fed(r, out, versions)
    return out


def run_task_pack(task, env, versions) -> Outcome:
    out = Outcome("pack")
    # pack ranks by term overlap, so the task text must speak the corpus's
    # language — `pack_query` carries the same information need rephrased for
    # term matching (cf. `fed_query` for the federated substring surface).
    q = task.get("pack_query") or task["question"]
    r = forgeplan(
        ["constellation", "pack", q,
         "--budget", str(task["budget"]), "--json"],
        CORPUS, env,
    )
    out.runs.append(r)
    parse_pack(r, out, versions)
    return out


# ─── scoring ──────────────────────────────────────────────────────────────────


def score(task, out: Outcome, manifest) -> dict:
    # evidence = raw stdout of every invocation (carries bodies, excerpts,
    # ambiguous-answer paths) + per-hit structured fields
    evidence = "\n".join(
        [r.stdout for r in out.runs] + [h["evidence"] for h in out.hits]
    )
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
            continue  # unattributed — can't judge
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
    for i, h in enumerate(out.hits, 1):
        if (h["key"] and h["key"] in expected) or any(
            s in h["evidence"] for s in task["must_include"]
        ):
            first_useful = i
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
        "wrong_ref_count": len(wrong_refs),
        "wrong_source_hits": wrong_source_hits,
        "omitted": out.omitted,
        "omissions_reported": bool(out.omitted),
        "latency_ms": round(latency, 1),
        "payload_bytes": payload,
        "exit_codes": [r.rc for r in out.runs],
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
    return {
        "mode": mode,
        "tasks": len(sel),
        "constraint_recall_full": f"{full}/{len(recall_tasks)}",
        "constraint_recall_pct": round(100 * full / len(recall_tasks), 1) if recall_tasks else None,
        "wrong_refs": sum(r["wrong_ref_count"] for r in sel),
        "wrong_source_hits": sum(r["wrong_source_hits"] for r in sel),
        "payload_p50": pct([r["payload_bytes"] for r in sel], 0.5),
        "latency_p50_ms": pct([r["latency_ms"] for r in sel], 0.5),
        "latency_p95_ms": pct([r["latency_ms"] for r in sel], 0.95),
        "first_useful_rank_mean": round(statistics.mean(ranks), 2) if ranks else None,
        "violations": {
            k: sum(1 for r in sel if r[k]) for k in
            ("source_boundary_violation", "version_substitution",
             "incomplete_negative", "negative_fabrication",
             "forbidden_content")
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
        "| mode | tasks | constraint-recall | wrong refs | wrong-src hits | payload p50 B | p50 ms | p95 ms | first-useful mean | boundary | version-subst | incomplete-neg | neg-fabrication | forbidden |",
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|",
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
            f"{v['source_boundary_violation']} | {v['version_substitution']} | "
            f"{v['incomplete_negative']} | {v['negative_fabrication']} | "
            f"{v['forbidden_content']} |"
        )
    lines += ["", "## Violations detail", ""]
    any_v = False
    for r in rows:
        if r["violations_total"]:
            any_v = True
            flags = [k for k in ("source_boundary_violation", "version_substitution",
                                 "incomplete_negative", "negative_fabrication",
                                 "forbidden_content") if r[k]]
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
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--mode", choices=[*MODES, "all"], default="all")
    ap.add_argument("--out", default=str(REPO / "eval" / "results"))
    ap.add_argument("--tasks", default=str(CORPUS / "tasks.yaml"))
    args = ap.parse_args()

    if not BIN.exists():
        raise SystemExit(f"binary missing: {BIN} (build with --features semantic-search)")
    ver = subprocess.run([str(BIN), "--version"], capture_output=True, text=True)
    log = lambda *a: print(*a, flush=True)
    log(f"[setup] binary: {ver.stdout.strip()}")

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    tasks = load_tasks(Path(args.tasks))
    manifest = corpus_manifest()
    log(f"[setup] {len(tasks)} tasks; corpus keys: "
        + ", ".join(f"{w}={len(k)}" for w, k in manifest.items()))

    # one isolated HOME per run: the constellation registry is hermetic, the
    # shared model cache stays reachable via FORGEPLAN_MODEL_CACHE.
    home = Path(tempfile.mkdtemp(prefix="f10-home-"))
    env = run_env(home)
    log(f"[setup] isolated HOME={home}")

    for ws in WS_NAMES:
        setup_workspace(ws, env, log)

    modes = list(MODES) if args.mode == "all" else [args.mode]
    cinfo = {"snapshot": None, "versions": {}}
    if any(m in ("federated", "pack") for m in modes):
        cinfo = setup_constellation(env, log)
    else:
        for ws in WS_NAMES:
            cur = read_current(ws) or {}
            cinfo["versions"][ws] = {
                "generation_id": cur.get("generation_id"),
                "source_set_digest": cur.get("source_set_digest"),
                "profile_id": cur.get("profile_id"),
            }

    rows = []
    for task in tasks:
        for mode in modes:
            if mode == "local":
                out = run_task_local(task, env, manifest, cinfo["versions"])
            elif mode == "federated":
                out = run_task_federated(task, env, cinfo["versions"])
            else:
                out = run_task_pack(task, env, cinfo["versions"])
            row = score(task, out, manifest)
            rows.append(row)
            flag = f" VIOL:{row['violations_total']}" if row["violations_total"] else ""
            log(f"  {task['id']:>4} {mode:<9} {out.result:<9} hits={row['n_hits']:<2} "
                f"{row['latency_ms']:>7.0f} ms{flag}")

    ts = time.strftime("%Y%m%d-%H%M%S")
    jsonl = out_dir / f"results-{ts}.jsonl"
    with jsonl.open("w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")
    df = subprocess.run(["df", "-h", str(REPO)], capture_output=True, text=True)
    disk_free = df.stdout.splitlines()[-1].split()[3] if df.returncode == 0 else "?"
    meta = {
        "ts": ts, "bin": str(BIN), "bin_version": ver.stdout.strip(),
        "corpus": str(CORPUS), "snapshot": cinfo["snapshot"], "disk_free": disk_free,
    }
    aggs = [aggregate(rows, m) for m in modes]
    report = out_dir / f"report-{ts}.md"
    write_report(rows, aggs, meta, report)
    log(f"[done] {jsonl.name}, {report.name}")

    total_viol = sum(r["violations_total"] for r in rows)
    if total_viol:
        log(f"[done] {total_viol} violation flag(s) across {len(rows)} rows — nonzero exit")
        return 1
    log(f"[done] 0 violations across {len(rows)} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
