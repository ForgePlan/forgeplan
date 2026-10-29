# F10 acceptance harness — local / federated / ContextPack

F10 (`.local/planning/constellation-minimax/ACCEPTANCE.md`) is designed to
compare 30 reference tasks over the same corpus in three modes — `local`
(single workspace `search`/`get`), `federated` (`constellation search`/`get`
over both workspaces), and `pack` (`constellation pack` under an estimated
token budget). It records must_include recall, wrong refs/omissions, payload
bytes, latency p50/p95 and first useful rank. The intended violation count is
zero. The implemented snapshot and scoring checks have the limits below;
this README is not a record of a passing run.

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

The runner uses `target/debug/forgeplan`; provide a compatible built binary,
Python 3.9+ and a complete BGE-M3 cache before running it. It initializes and
indexes the dedicated corpus workspaces and writes run state under a temporary
HOME. Check inherited workspace bindings first; this is not a read-only check.

```bash
python3 eval/run_eval.py                 # all three modes
python3 eval/run_eval.py --mode local    # one mode only
python3 eval/run_eval.py --out /tmp/res  # alternate output dir
```

On first setup the runner initializes each corpus git repository if needed,
runs `forgeplan index` when `CURRENT.json` is absent or invalid JSON, and
attempts `forgeplan embed` when the published pointer lacks
`embedding_artifacts_head` or `profile_id`. A nonzero embed exit is logged as
a warning; the pointer alone is not proof of complete semantic readiness.
No fixed model-load time is guaranteed.

The runner redirects `HOME`, `USERPROFILE` and `XDG_DATA_HOME` to a temporary
directory and sets `FORGEPLAN_MODEL_CACHE` to the real user's model cache.
It otherwise inherits the environment, including any workspace-binding
variables; this is registry isolation, not a fully hermetic environment.

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
| `version_substitution` | federated get/search compare source `version_key` with setup `generation_id`; pack compares it with setup `source_set_digest`; local get compares `generation_id` only when returned; local search has no version comparison |
| `incomplete_negative` | expected content → empty / not-found / error |
| `negative_fabrication` | no-answer task → hits or a found artifact |
| `omissions_reported` | pack listed entries in `omitted` |

Exit code is non-zero when any violation flag is non-zero.

A zero exit means these implemented flags stayed clear, not that every
read was proven snapshot-pinned or every required substring was found.
Missing `must_include` strings are reported in recall metrics; the violation
rules are narrower. Search scoring uses returned keys/titles (plus local
sections), not a full-body fetch for each hit.

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

After adding or changing corpus artifacts, refresh the dedicated corpus with
`forgeplan index` and, when semantic evaluation is required, `forgeplan embed`
before the next run. Do not delete `CURRENT.json`: it is the publication
pointer, not a disposable cache marker. The runner's existence check does not
detect every corpus change or rebuild an already-published snapshot.
