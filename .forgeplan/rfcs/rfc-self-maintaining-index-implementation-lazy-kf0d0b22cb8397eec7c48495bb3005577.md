---
assigned_number: 21
author: Eli Rum
created: 2026-11-18
depth: standard
id: RFC-021
links:
- target: prd-self-maintaining-derived-index-lazy-sync-kff438581336d5e91e7a2662055c61575
  relation: refines
- target: spec-self-maintaining-index-behavioral-kb75e8c4dd214d20a5a0c66be7f58e3b6
  relation: informs
slug: rfc-self-maintaining-index-implementation-lazy-kf0d0b22cb8397eec7c48495bb3005577
status: Draft
title: Self-maintaining index implementation — lazy sync, durable embedding queue, model bootstrap
updated: 2026-11-18
---

# RFC-021: Self-maintaining index implementation — lazy sync, durable embedding queue, model bootstrap

## Summary

Implement the self-maintaining index promised by RFC-004 by routing read commands through the existing capture → reconcile path, draining pending embedding jobs from the same queue on every mutation surface, bootstrapping the embedding model lazily with progress, and fixing the `--json` / corrupt-config error paths.

## Motivation

Files-first is only honest if readers see the files' current state. Today a `.md` edit outside `forgeplan` is invisible until the operator runs `index` or keeps `watch` running. Semantic search also has three gaps: pending jobs can strand when the model is not warmed; the first semantic use requires a separate `setup --warm-model`; and two error paths lie (`--json` prints prose, `index` succeeds on corrupt config).

Doing nothing leaves users with stale reads and makes `--json` unsafe for automation.

## Goals

- Any read command sees file changes made outside the CLI before answering.
- Mutations enqueue durable embedding jobs; drains happen on `index`, `embed`, and semantic reads.
- First semantic use downloads the model once with visible progress and safe resume.
- `--json` always emits a machine-readable envelope on stdout.
- `maintain_index` fails closed when `config.yaml` cannot be parsed.

## Non-Goals

- Section-level embedding jobs.
- Model replacement / re-embedding migration.
- Running `watch` as a background service.
- Changing the canonical Constellation index layout.

## Options Considered

### Option A: Read-path reconcile via the existing capture → reconcile machinery

**Description**: Introduce a shared `reconcile_workspace_for_read(workspace)` helper in the CLI command layer. Read commands call it before loading data. It runs the same constellation capture/reconcile path used by `index` — content digests, not mtime — so same-size/same-mtime edits are still detected. It reports `reconciled: N file(s)` on stderr only when changes exist.

**Pros**:
- Reuses the identity-safe comparison already proven by capture tests; no mtime-only false negatives.
- One seam covers `get`, `list`, `search`, `graph`, `export`, `release-notes`, `health`, and MCP reads that route through the same workspace resolution.
- No background process or polling.

**Cons**:
- Adds capture work to every read command; needs the NFR-001 benchmark gate.
- Must respect workspace locks and read-only profiles.

### Option B: mtime/size comparison before each command

**Description**: Compare `.md` mtime/size to the index row and re-parse only changed files.

**Pros**: Cheapest implementation.

**Cons**:
- Explicitly rejected by Constellation durability design: mtime and size are hints, not identity; a same-size/same-mtime rewrite would serve stale bytes.
- Duplicates reconciliation logic outside capture.

### Option C: Require `watch` or periodic `index`

**Description**: Keep the current model and document the requirement.

**Pros**: No code change.

**Cons**: Fails the files-first promise for non-watcher users; already observed stale reads.

## Trade-off Analysis

| Критерий | Option A | Option B | Option C |
|----------|----------|----------|----------|
| Complexity | Medium | Low | None |
| Cost | Capture per read | Lowest | Existing |
| Correctness | High | Medium (mtime blind spots) | Low |
| Migration risk | Low | Low | None |
| Developer experience | Best | Good until silent stale | Poor |
| Operational burden | None | None | Watch/index burden |

## Proposed Direction

Option A. The identity-safe capture path is the only one consistent with Constellation's durability rules; making it the read seam removes the operational requirement without weakening detection.

For the embedding model, bootstrap lazily inside `Embedder::new()` callers that are permitted model obtainers: `embed`, `index --embed-profile`, and `search --semantic` (and MCP equivalents through the same facade). Show progress on first download; `setup --warm-model` remains the explicit prefetch path. A build without `semantic-search` keeps the current honest `Fix:` error.

For output/config bugs:
- Add a shared JSON envelope writer for `--json` commands: `{ ok, result?, error? }`; errors still go to stderr and non-zero exit.
- Parse `config.yaml` before `maintain_index`; parse failure aborts naming the file.

## Risks & Open Questions

- Read reconcile must not double-commit with a concurrent `watch`; both already serialize through the maintenance path.
- Model auto-download on a metered/offline link must fail with a named fix and no corrupt cache.
- Embedding queue drain while jobs are still being appended must be idempotent.
- Open question: whether MCP read profile should also reconcile, or stay snapshot/pinned behavior. Recommendation: reconcile for live-scope reads; snapshots remain pinned.

## Implementation Phases

### Phase 1: Read reconcile seam
- [ ] **1.1** Add `reconcile_workspace_for_read` shared helper; call the capture → reconcile → publish path for the workspace owner.
- [ ] **1.2** Wire all read commands through it before data load; stderr only on actual reconciliation.
- [ ] **1.3** Tests: edited/new/deleted `.md` visible to `get`/`list`; unchanged file is a no-op; 500-file bounded cost.

### Phase 2: Embedding queue + model bootstrap
- [ ] **2.1** Ensure every mutation surface appends durable pending jobs (CLI, MCP, watch, git-sync).
- [ ] **2.2** Auto-download model in permitted obtainers with progress/resume; absent model leaves queue pending with a clear note.
- [ ] **2.3** Tests: enqueue survives kill; drain empties queue; skipped drain reports model absent; first semantic use bootstraps.

### Phase 3: Error-path fixes
- [ ] **3.1** JSON envelope on all `--json` commands, success and error.
- [ ] **3.2** `maintain_index` fails closed on unparseable `config.yaml`.
- [ ] **3.3** Existing RED tests in `cli_negative_contracts.rs` turn green.

## Affected Files

- `crates/forgeplan-cli/src/commands/**` — read seams, embed/index/search bootstrap, JSON envelope
- `crates/forgeplan-core/src/constellation/**` — capture/reconcile entry points, embedding queue, maintenance config read
- `crates/forgeplan-core/src/embed/**` — model resolution/download
- `crates/forgeplan-cli/tests/**` — behavioral and negative contracts
- `crates/forgeplan-mcp/src/**` — MCP read/search paths if they bypass CLI helpers

## Related Artifacts

- PRD-092 — product requirements (refines)
- SPEC-013 — behavioral scenarios (implements)
- RFC-004 — Files-First Architecture (completes Phase 1)
- ADR-003 — Markdown source of truth
- PROB-067 / SPEC-005 / RFC-009 / ADR-012 — Constellation identity work
- GitHub #568 — `--json` error-path envelope bug; #569 — corrupt-config index bug
