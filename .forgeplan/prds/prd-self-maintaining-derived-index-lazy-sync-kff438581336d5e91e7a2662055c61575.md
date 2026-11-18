---
assigned_number: 92
author: Eli Rum
created: 2026-11-18
depth: standard
domain: general
epic: null
id: PRD-092
predicted_number: 92
priority: P1
projectType: cli_tool
slug: prd-self-maintaining-derived-index-lazy-sync-kff438581336d5e91e7a2662055c61575
status: Draft
stepsCompleted: []
title: Self-maintaining derived index — lazy sync, embedding queue, and model bootstrap
updated: 2026-11-18
---

# PRD-092: Self-maintaining derived index — lazy sync, embedding queue, and model bootstrap

## Executive Summary

### Vision

The derived LanceDB index stays honest with the `.md` source of truth without the operator having to think about it — file edits are picked up by the next command, semantic vectors drain from a durable queue, and the embedding model downloads itself the first time a semantic operation actually needs it.

### Problem

RFC-004 established files-first but left four gaps:

1. **No lazy sync on reads** — editing `.forgeplan/prds/PRD-001.md` in an editor and running `forgeplan get PRD-001` returns the stale LanceDB body. Only `watch` (a separate long-running process), `index` (manual pass), or `git-sync` (git-only) update the index. A CLI user who edits files sees phantom staleness.
2. **Embedding queue drains only via explicit `embed` / `index --embed-profile`** — after every mutation a best-effort `drain_legacy_semantic_after_mutation` runs, but on a workspace where the model has never been warmed this fails silently and the queue stays pending forever.
3. **Model bootstrap is manual and surprising** — `setup --warm-model` is the only documented path; a fresh install that runs `forgeplan search --semantic "x"` hits a wall: first a 2.1 GB download the user didn't ask for, then either a hang or an opaque error. The honest `Fix:` hint exists but pushes work onto the operator.
4. **Two documented bugs in the error path** — `get --json` on a bad selector prints plain-text `Error:` to stdout (breaks machine consumers), and `forgeplan index` succeeds with `verified: nothing staged` on a corrupt `config.yaml` (silently indexes under wrong assumptions).

**Impact**: agents and humans editing `.md` files directly (the whole point of files-first) get answers from a stale index; semantic search appears broken on fresh installs; machine consumers can't trust `--json`.

### Target Users

| Persona | Описание | Ключевая боль |
|---------|----------|---------------|
| Solo developer | Edits `.md` in a real editor, runs `forgeplan` CLI | Stale reads, no signal that the index is behind |
| Agent / orchestrator | Writes `.md` via script, calls `forgeplan` tools | No guarantee a command sees its own just-written file |
| Semantic user | Runs `search --semantic` on a fresh install | 2 GB surprise download or opaque failure |

### Differentiators

- Files-first means the index must *chase* the files — every competing design (explicit `index`, daemon `watch`, git-only `git-sync`) puts that burden on the operator instead of the read path.
- Embedding queue is durable across sessions — pending jobs survive crashes; the drain is the only thing that needs a model.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Edited `.md` visible to next command | stale-read latency | unbounded (requires `index`) | < 1 command | next release | test: write file → `get` returns new body |
| SC-2 | Embedding queue drains without manual `embed` | pending jobs after a mutation | stays >0 on un-warmed workspace | 0 or explicit defer | next release | test: mutate → `index --embed-profile` drains all |
| SC-3 | First semantic op bootstraps model | manual step | `setup --warm-model` required | auto-download with progress | next release | test: cold `search --semantic` succeeds after one download |
| SC-4 | `--json` on error produces JSON | stdout parseable | plain `Error:` | JSON envelope | next release | `get NOPE --json` parses |
| SC-5 | Corrupt config fails the index pass | corrupt-yaml exit | 0 + `verified: nothing staged` | non-zero + named file | next release | `index` on `{{{{` config exits non-zero |

---

## Product Scope

### MVP (In-Scope)

- **Lazy sync on read**: before `get`/`list`/`search`/`graph`/`export`/`release-notes`/`health` (every read command), check `.forgeplan/**/*.md` mtime vs LanceDB `updated_at`; re-parse changed files into the index before answering.
- **Embedding queue drain on every mutation**: after `new`/`update`/`link`/`unlink`/`delete`/`activate`/`deprecate`, append pending embedding jobs to a durable queue; drain on next `index`/`embed`/`search --semantic` when the model is present.
- **Model auto-bootstrap with progress**: first call to `Embedder::new()` when the model files are absent streams the download with a visible progress bar and a `Ctrl-C`-safe resume; failure names the URL and the fix (`setup --warm-model` for manual retry, or offline message).
- **JSON envelope on error**: every `--json` command wraps its result in `{ok, result?, error?}`; stderr carries prose.
- **Fail-closed config**: `maintain_index` parses `config.yaml` up front; parse failure aborts the pass with the file named.

### Out of Scope

- Embedding model *swap* mid-flight (changing models invalidates existing vectors — separate PRD).
- Incremental embedding of only-changed sections (queue is per-artifact today; section-level granularity is a follow-up).
- Concurrent multi-process `watch` (single watcher is the contract).

### Growth Vision

- Section-level embedding jobs → sub-artifact semantic recall.
- Cross-worktree index invalidation (the `stale` path in `mark_dirty`).

---

## User Journeys

### Journey 1: Developer edits a file, runs a command

**Цель**: user edits `.forgeplan/prds/PRD-007.md` in vim, runs `forgeplan get PRD-007`, sees the new body.

| Шаг | Действие пользователя | Ответ системы |
|-----|----------------------|---------------|
| 1 | Edit `.forgeplan/prds/PRD-007.md`, save | — |
| 2 | `forgeplan get PRD-007` | Reads mtime, re-parses file, updates index, returns new body; prints `reconciled: PRD-007` to stderr |
| 3 | `forgeplan search "..."` | Result includes the just-edited body |

**Результат**: no manual `index`/`reindex` needed; the index never lies.

### Journey 2: Agent writes artifacts via file API, calls MCP

**Цель**: an orchestrator writes 3 `.md` files via `Write`, then calls `forgeplan_get` — sees all 3.

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `Write` 3 `.md` files under `.forgeplan/` | — |
| 2 | `forgeplan list` or MCP `forgeplan_list` | Lazy sync picks up all 3 new files, indexes them, list returns them |
| 3 | `forgeplan search "title"` | Keyword hit on new titles |

### Journey 3: Fresh install, first semantic search

**Цель**: user installs `forgeplan` with `semantic-search`, runs `search --semantic "auth"` on a repo with no warmed model.

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan search --semantic "auth"` | Model absent → prints `Downloading embedding model (~2.1 GB)...` with progress, downloads to `~/.forgeplan/models/`, caches |
| 2 | Search completes | Returns semantic results |
| 3 | Second `search --semantic` | No download, cached model loads in < 1 s |

### Journey 4: Scripted `--json` consumer hits an error

**Цель**: a CI script calls `forgeplan get $ID --json` where `$ID` is wrong.

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan get NOPE-999 --json` | stdout: `{"ok": false, "error": "selector 'NOPE-999' is not a valid artifact reference"}`, stderr: prose, exit non-zero |
| 2 | Script parses stdout | Gets structured error, no crash on `Error:` text |

### Journey 5: Operator runs `index` on corrupt workspace

**Цель**: `config.yaml` was hand-edited badly; `forgeplan index` must not silently proceed.

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan index` | Reads `config.yaml`, parse fails → `Error: .forgeplan/config.yaml: invalid YAML at line 3` + exit non-zero |
| 2 | Fix config, re-run | Normal pass |

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | Before any read command, the system shall compare each `.forgeplan/**/*.md` mtime against the stored `updated_at` and re-index changed files before answering | J1, J2 |
| FR-002 | Core | Must | After any write command, the system shall append a pending embedding job for the changed artifact to a durable queue under `.forgeplan/lance/` | J1 |
| FR-003 | Core | Must | The embedding queue shall drain on `index`, `embed`, and `search --semantic` when the model is available, and stay pending otherwise | J1, J3 |
| FR-004 | Core | Must | The first call needing the embedding model shall download it automatically with a visible progress indicator and resumable transfer | J3 |
| FR-005 | Output | Must | Every `--json` command shall emit a `{ok, result?, error?}` envelope on stdout in both success and error paths | J4 |
| FR-006 | Core | Must | `maintain_index` shall fail fast when `config.yaml` does not parse, naming the file and the line | J5 |
| FR-007 | UX | Should | Lazy-sync reconciliations shall be announced on stderr (`reconciled: N file(s)`) so the user knows the index caught up | J1 |
| FR-008 | UX | Should | Model download shall be interruptible (Ctrl-C) and resumable without corrupting the cache | J3 |
| FR-009 | Core | Should | `watch` and `git-sync` shall enqueue embedding jobs on the same queue as CLI mutations | J1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Lazy sync shall add no more than X ms to a command | < 50 ms p95 | Workspace with ≤ 500 artifacts | `hyperfine` on `get` before/after |
| NFR-002 | Performance | Model load after warmup shall be fast | < 1 s | Second and subsequent semantic ops | benchmark |
| NFR-003 | Reliability | Embedding queue shall survive process kill | 100% | SIGKILL between enqueue and drain | test: kill mid-mutation, next `embed` drains all |
| NFR-004 | Correctness | `--json` stdout shall parse as JSON | 100% | All `--json` commands, success and error paths | jq on every output |
| NFR-005 | Security | Model download shall verify checksum | SHA-256 match | Fresh download | signature check in fastembed |

---

## Acceptance Criteria

### AC-1: File edit visible to next command

```gherkin
Given a workspace with PRD-001 indexed
When I edit .forgeplan/prds/PRD-001.md and run `forgeplan get PRD-001`
Then the output contains the edited body
And stderr reports `reconciled: 1 file(s)`
And LanceDB `updated_at` reflects the file mtime
```

### AC-2: Embedding queue drains on index

```gherkin
Given a workspace with semantic-search enabled and the model warmed
And a mutation has left a pending embedding job
When I run `forgeplan index --embed-profile <profile>`
Then all pending jobs drain
And a subsequent `search --semantic` finds the mutated artifact
```

### AC-3: Model auto-downloads on first semantic op

```gherkin
Given a fresh install with semantic-search feature and no warmed model
When I run `forgeplan search --semantic "test"`
Then a progress bar shows the model downloading
And the search completes with results
And a second `search --semantic` loads the model in under 1 second
```

### AC-4: JSON envelope on error

```gherkin
Given any workspace
When I run `forgeplan get NOPE-999 --json`
Then stdout is a single JSON object
And `ok` is false
And `error` names the selector
And exit code is non-zero
```

### AC-5: Corrupt config blocks index

```gherkin
Given a workspace where `.forgeplan/config.yaml` is invalid YAML
When I run `forgeplan index`
Then the command exits non-zero
And stderr names `.forgeplan/config.yaml`
And no index pass runs
```

---

## Dependencies

- **RFC-004** (Files-First): this PRD implements its Phase 1 (lazy sync) and extends Phase 2 (queue-driven embeddings).
- **ADR-003** (Markdown = source of truth): lazy sync is the mechanism that keeps that promise on the read path.
- **PROB-067 / SPEC-005 / RFC-009 / ADR-012 / #552**: Constellation identity work — embedding queue and lazy sync operate on the same artifact store.
- **PROB-088** (deprecated): release binaries without semantic-search — the `Fix:` hint already routes to the right install.

## Risks

| Risk | Mitigation |
|------|------------|
| Lazy sync adds latency to every command | Gate to read commands only; benchmark under NFR-001; mtime check is O(files) with early exit on unchanged |
| Model download on metered link | `--offline` flag on semantic ops; clear `Fix:` for `--skip-model` |
| Queue drain racing concurrent `watch` | Single-writer lock on the queue file; drain is idempotent |
| JSON envelope changes shape for existing consumers | Envelope is additive — `ok`, `result`, `error` are new top-level keys; `result` preserves the old shape |
