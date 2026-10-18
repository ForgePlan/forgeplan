# F10 acceptance harness — local / federated / ContextPack

F10 (`.local/planning/constellation-minimax/ACCEPTANCE.md`): 30 reference tasks
run over the **same corpus snapshots** in three modes — `local` (single
workspace `search`/`get`), `federated` (`constellation search`/`get` over a
registry of both workspaces), `pack` (`constellation pack` under a token
budget). Measured: найденные обязательные ограничения (must_include recall),
неверные refs/omissions, payload bytes, latency p50/p95, первый полезный
источник (rank of first useful hit). Violations tracked: source-boundary,
version-substitution, hidden incomplete-negative — expected count is zero.

## Layout

```
eval/
  corpus/
    ws-a/.forgeplan/…    drain-queue reliability workspace (12 artifacts)
    ws-b/.forgeplan/…    billing-service workspace (12 artifacts)
    tasks.yaml           the 30 tasks
  run_eval.py            the runner (Python 3 stdlib only)
  results/               gitignored — results-*.jsonl + report-*.md per run
```

ws-a and ws-b are real git checkouts with real forgeplan workspaces. Their
`.git/`, `.forgeplan/lance/` and other runtime state are generated on first
run and gitignored — only markdown artifacts + `config.yaml` are committed.

## Run

```bash
python3 eval/run_eval.py                 # all three modes
python3 eval/run_eval.py --mode local    # one mode only
python3 eval/run_eval.py --out /tmp/res  # alternate output dir
```

First run is slow: it `git init`s each corpus workspace, runs
`forgeplan index`, then `forgeplan embed` (bge-m3, cached at
`~/.forgeplan/models`, ~1-3 min of model load per workspace). Later runs skip
all of it via `.forgeplan/lance/CURRENT.json` — that file is the "already
provisioned" marker; delete it to force re-index.

Isolation: every forgeplan invocation gets `HOME=<run tmpdir>` +
`USERPROFILE` + `XDG_DATA_HOME`, so `~/.forgeplan/constellations.yaml` is
hermetic per run (same trick as `cli_constellation_*.rs` tests). The shared
model cache stays reachable because the runner sets
`FORGEPLAN_MODEL_CACHE=<real ~/.forgeplan/models>` explicitly.

## What a row in results-*.jsonl means

| field | meaning |
|---|---|
| `result` | `found` / `not_found` / `ambiguous` / `hits` / `error` |
| `hits` | `{key, source_ws, source_id}` per returned hit/unit |
| `must_include_found` / `_missing` | which required substrings surfaced |
| `constraint_recall` | fraction of `must_include` found (null if none required) |
| `first_useful_rank` | 1-based position of the first hit carrying expected content |
| `wrong_ref_count` | hits outside `expected_artifacts` (precision noise) |
| `wrong_source_hits` | federated/pack hits from a ws not in `expected_sources` |
| `latency_ms` | wall clock of the invocation(s); local fan-out sums its legs |
| `payload_bytes` | stdout bytes |
| `source_boundary_violation` | hit attributed to a ws that does not contain that key (local leak or federated misattribution) |
| `version_substitution` | served `version_key`/`generation_id` ≠ the one CURRENT.json pinned at setup |
| `incomplete_negative` | expected content → empty / not-found / error |
| `negative_fabrication` | no-answer task → hits or a found artifact |
| `omissions_reported` | pack listed entries in `omitted` |

Exit code is non-zero when any violation flag is non-zero.

## Adding a task

Append an item to `corpus/tasks.yaml` (restricted YAML subset — see header
comment). `expected_sources` decides where local mode runs; `must_include`
strings must be verbatim substrings of corpus artifact bodies/ids. For a
negative task set `expect_found: false`; for the duplicate-id case use
`expect_found: ambiguous`. `budget` only affects pack mode (estimated tokens,
chars/4).

Two query-grammar caveats the fields encode:

- `constellation search` is a **literal substring** match, not term ranking —
  natural-language `question`s legitimately return empty. Give search tasks a
  `fed_query`: a verbatim corpus substring expressing the same information
  need (`"dead-letter"`, `"1s, 5s, 30s"`).
- `constellation pack` ranks by **term overlap** — a task in a language the
  corpus doesn't speak (e.g. the Russian semantic tasks) yields an honestly
  empty pack. Give those a `pack_query` with corpus-language terms.

If you add artifacts to a corpus ws, delete its
`.forgeplan/lance/CURRENT.json` afterwards so the next run re-indexes and
re-embeds.
