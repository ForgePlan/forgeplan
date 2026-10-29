---
title: Constellation reference
description: Sources, index generations, read contracts, MCP profiles, semantic readiness and current limitations.
sidebar:
  order: 2
---

This reference describes the local **`0.39.0+constellation.s1.1`** interface. It is not a published `0.38.0` feature list or a claim of full S1-G/S2 acceptance. For a runnable sequence, use [the two-project guide](/docs/guides/constellation/). Compatibility observations apply only to the binary and scenarios actually exercised; a changed source snapshot requires its own compilation and runtime evidence.

Recorded compatibility checks exercised existing CLI names and argument forms, including CLI validation, on legacy and Constellation-managed layouts. Command, tool and skill names remain unchanged. This does not establish every handler's behavior or semantic readiness.

## Data and ownership

| Location | Responsibility |
|---|---|
| `<project>/.forgeplan/<kind>/*.md` | Canonical engineering artifacts. Preserve these when rebuilding an index. |
| `<project>/.forgeplan/config.yaml` | That workspace's active settings, including opt-in background embedding. |
| `<project>/.forgeplan/lance/` | Derived index. Constellation publishes generations through `CURRENT.json`; a migrated index is not a legacy `artifacts.lance` store. |
| `~/.forgeplan/constellations.yaml` | User registry of named groups and required/optional sources. It does not copy documents or start publishers. |
| `FORGEPLAN_MODEL_CACHE`, otherwise `~/.forgeplan/models` | Shared cached model files, not the document vectors themselves. |
| `~/.forgeplan/snapshots/` | Git snapshot lockfiles. A persisted snapshot ID is different from a process-local MCP handle. |

`constellation add` takes a `.forgeplan/` directory. Creating or switching a group changes registry selection, not canonical artifact content. Starting one full server does not maintain every group member.

## Index and freshness

A generation names exact versions of the artifact, relation and section tables. Readers pin those versions rather than silently opening latest tables. Reconciliation captures canonical input, computes a change set and publishes the derived result.

- `forgeplan index` performs an explicit maintenance pass and can initiate the new layout.
- `forgeplan reindex` delegates to Constellation on an already migrated workspace. It is not a reason to delete `.forgeplan/` or reinitialize config.
- Ordinary canonical reads can capture newer Markdown than the published index. CLI JSON reports `persisted: false` for such an ephemeral view; this is not an acknowledged publication.
- Full MCP writes can report `index_pending: true`. The canonical write completed, but publication has not been acknowledged. Read the returned reason and verify the later published state.
- `index --list`, `--generations` and `--gc` are separate inspection/administration options. See the [exact index command](/docs/cli/index-command/); do not infer options such as `index --json` that this build does not declare.

## Read contracts

Do not compare every `body` to the same byte string.

| Operation | Selection | Body and version |
|---|---|---|
| CLI `forgeplan get ID --json` | Current workspace | Whole captured Markdown file, including frontmatter; freshness metadata distinguishes ephemeral from persisted. |
| Full MCP `forgeplan_get` | `id`, optional `workspace` | Canonical/captured read; `body` includes the full Markdown. Do not assume the pinned-get envelope. |
| Full MCP `forgeplan_constellation_get` | `key`, optional `workspace` | Published row in `artifact`; `artifact.body` is the parsed body after the outer frontmatter. Returns generation identity and `source_digest`. |
| Federation get / read MCP with `scope_handle` | `id`, optionally source-qualified | Inspect `source.channel`. A published `pinned` row has parsed body, not the full-file contract of ordinary CLI get. |
| Read MCP with `snapshot_handle` | `id` within that snapshot | `pinned_git` content is the complete committed Git file, including frontmatter. It is not the current working-tree edit. |

A parsed body may itself contain YAML or Markdown separators; “parsed body” does not mean every frontmatter-looking block has been removed. Use the documented channel and operation, not a text-trimming heuristic.

### Identity and ambiguity

Use returned keys as opaque identifiers. Do not reconstruct them from a title or filename. The same display ID, such as `NOTE-001`, can exist in several projects.

Federated get then returns `result: "ambiguous"` with candidate sources. Qualify the ID as `KEY@source-fragment`, for example with the returned `src:<hash>` or a unique path segment. `result: "found"` and the expected source identity establish a successful selection; exit code 0 alone does not.

Live and Git-pinned source IDs may differ. After selecting a snapshot, search in that snapshot and take its `source_id` again. Reusing a live qualifier can return `error: "no_source"` even though the document exists in the Git snapshot.

## MCP profiles and workspace selection

| Profile | Registered tools in this build | Purpose |
|---|---:|---|
| `full` (default) | 76 | Ordinary artifact operations, including writes, and local published-index reads. Registration is not runtime acceptance for every tool or workspace layout. |
| `constellation-read` | 9 | Registry-bound scope/snapshot reads; no artifact write tools and no arbitrary workspace path argument. |

Only `forgeplan serve` accepts `--profile`. `forgeplan mcp serve` and the standalone `forgeplan-mcp` accept workspace/maintenance options but do not expose this profile selector.

For full requests, workspace resolution is:

1. Request `workspace` parameter, when supported and supplied.
2. `FORGEPLAN_WORKSPACE`.
3. Server default set by `--workspace`.
4. Process working-directory discovery.

At full-server startup, maintenance uses `FORGEPLAN_WORKSPACE` → server default from `--workspace` → working-directory discovery. Bind the environment to the intended project, or explicitly set `FORGEPLAN_WORKSPACE=""`. Unsetting alone is insufficient: the CLI loads `.env` files from ancestors before parsing arguments. That loader preserves an existing empty value, which the resolver skips. A later request directed elsewhere does **not** move the host or start maintenance there. One publisher owns one physical index; do not run old and new publishers against it simultaneously.

On legacy layout, maintenance is opt-in through `--auto-index` or `watch --constellation`. An already migrated workspace also starts maintenance in the full server. The read profile does not create that publisher.

See [MCP setup](/docs/guides/mcp-setup/), [the profile catalogs](/docs/mcp/) and [`serve` help](/docs/cli/serve/).

### Read-profile sequence

| Tool | Essential arguments / result |
|---|---|
| `constellation_open` | Optional group name `constellation`; returns `scope_handle` and sources. |
| `constellation_sources` | `handle` from the same process; inspect registered identities and roles. |
| `constellation_search` | `query` and exactly one scope/snapshot handle. |
| `constellation_get` | `id` and exactly one scope/snapshot handle. |
| `constellation_graph` | Handle, optional `limit`. An empty graph does not establish impact correctness. |
| `constellation_impact` | Handle and changed `id`, optional `depth`/`max_nodes`. |
| `constellation_pack` | Handle, `task`, optional estimated `budget`. |
| `constellation_snapshot` | `handle` of an open scope; returns `snapshot_handle`, `pinned` and `skipped`. |
| `constellation_close` | `handle`; close scopes and snapshots separately. |

Use `scope_handle` **or** `snapshot_handle` on read operations, not both. The `handle` parameter belongs to sources/snapshot/close. The complete nullable/default/schema details are on the [individual tool pages](/docs/mcp/).

Handles belong to one live server process, not a saved client configuration. A closed, unknown or invalidated handle is not a request to read latest. On `snapshot_expired`, obtain a new scope or deliberately select a new snapshot. A scope's source-version checks and a Git snapshot's committed revision are different contracts.

MCP returns JSON text inside a tool result. Decode it and inspect domain fields such as `error`, `result`, `failures` and `partial`; a successful transport exchange is not automatically a successful read.

## Required, optional and partial sources

A required source's outage must not be silently interpreted as “no matching artifact”. Strict operations refuse unavailable required sources. Register `--optional` only when an absent source is acceptable; inspect named failures even when a response is allowed.

CLI `--allow-partial` and read MCP `strict: false` permit partial answers where supported. They do not make an expired immutable handle valid again. Snapshot source membership is fixed by the lockfile; do not assume `strict` changes that set.

Always inspect `failures`, `partial`, `truncated`, expected source identities, and the snapshot's `skipped` list. `doctor` being healthy is not proof that every file was ingested or that all vectors are usable. Incomplete-read and compatibility limits remain tracked in [#538](https://github.com/ForgePlan/forgeplan/issues/538).

## Semantic readiness

These are separate resources: a build with `semantic-search`, cached model files, an active embedding profile, and usable vectors bound to the current captured content.

Automatic drain requires an active workspace configuration, for example:

```yaml
embedding:
  model: bge-m3
  chunk_size: 2000
```

It also requires the supported BGE-M3 snapshot in the selected cache and a live maintenance host for that workspace. Missing `config.embedding` skips the drain; a manual `embed` does not persist it. An unresolved cached profile leaves work pending and emits a reason to stderr rather than silently downloading in the background.

`init --force` replaces active settings with defaults; the former custom embedding block survives only in its backup until restored. This can disable background embedding.

| Signal | Interpretation |
|---|---|
| `semantic_available: true` | The local semantic query used an available profile/vector path. Still check relevance and coverage. |
| `eligible_rows` / `expected_rows` | Usable rows bound to this captured input versus expected artifact-prefix and section rows; not document counts. |
| `semantic_scope: "prefix+sections"` | The local semantic path covers prefix and section units, not just a clipped document prefix. |
| `ready` / `pending` / `failed` / `unavailable` | Row readiness, not whether canonical Markdown exists. |

Forced local `search --semantic` can fail without a published profile or eligible vectors. It is not guaranteed to fall back to keywords. Smart search and federation are different interfaces: the `constellation search`/read-profile search contract here is lexical and has no semantic-mode argument. A returned `semantic_state` does not prove cross-project vector ranking.

### Section windows

`s1-token-layout-v2` derives contiguous windows from the exact parsed `artifact.body`. Offsets are UTF-8 byte positions within that body, not character counts or full-file offsets including the outer frontmatter. Preserve the body used by the plan when applying ranges.

The tokenizer budget is bounded by `min(model_max_length, 512)`, including service tokens and header/context allowance. A long section produces several windows. Heading bytes, preamble, repeated headings and whitespace are not normalized away. A changed layout identity requires new indexing/embedding; old vectors do not prove coverage of the new layout.

`chunk_size` controls the artifact-prefix/profile input; it is not a replacement for the tokenizer's per-window budget. Local forced semantic results can contain several rows for one document; use `document_key` rather than treating every section key as a separate artifact.

## ContextPack and advanced reads

ContextPack selects ranked units with roles, omissions and a receipt. `receipt.token_kind: "estimated_chars_div_4"` describes an estimate for selected units, **not** a strict model-token limit for the entire serialized JSON. Inspect `payload_bytes` where the API returns it; wrappers differ between CLI, live and snapshot surfaces.

Graph/impact accept qualified references in the implementation. That is not a claim that every cross-project dependency walk or fault scenario has passed acceptance. Export availability likewise does not prove an external adapter round-trip. Use the open acceptance record [#538](https://github.com/ForgePlan/forgeplan/issues/538), not a tool's presence, to judge those broader claims.

## Diagnostics and supported recovery

| Observation | Action |
|---|---|
| CLI reports a legacy-store refusal on a Constellation-managed workspace | Check the selected executable and source snapshot against the recorded compatibility evidence. Do not bypass the error by reopening retired tables with an old binary. A verified validation scenario does not establish every other tool's behavior. |
| Get is fresh but `persisted: false`, or a write is `index_pending` | Compare canonical bytes and the published generation separately. Let the one maintenance owner publish, or stop it before an explicit pass. |
| No embedding profile / zero eligible rows | Check active config, build feature, cache/profile compatibility and worker stderr. Starting the server alone is not a readiness check. |
| Queue skipped / failed | Preserve the error and source state. Do not delete the model cache or canonical workspace as a speculative fix. |
| `ambiguous` / `no_source` | Select an identity from the same live/snapshot response. Do not guess a project from the display ID. |
| `snapshot_expired` | Reopen deliberately. Do not silently substitute newer content. |

Rebuild the derived index with the appropriate `index`/`reindex` path, preserving Markdown and config. `scan-import` is not the index-rebuild recipe. A rollback must retain new canonical changes and restore a compatible binary, index and configuration together; swapping only the executable is insufficient.

## Exact interface catalogs

- [All CLI command paths](/docs/cli/) — including [`constellation`](/docs/cli/constellation/) and [`index`](/docs/cli/index-command/).
- [MCP profiles and schemas](/docs/mcp/) — full `forgeplan_constellation_*` tools are not aliases for handle-based `constellation_*` tools.
- [Configuration](/docs/getting-started/configuration/) and [hands-on guide](/docs/guides/constellation/).

The catalogs are captured from the selected binary. Their coverage checks do not semantically validate every handwritten example. Narrow macOS/arm64 observations, site build and interface discovery are not substitutes for Windows/NTFS, crash, large-corpus quality or full S1-G/S2 acceptance.
