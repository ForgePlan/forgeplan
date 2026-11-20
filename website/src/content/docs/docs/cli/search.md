---
title: forgeplan search
description: "Smart search - BM25 + semantic + graph expansion with Russian morphology"
---

Smart search across artifacts. As of v0.18.0, Forgeplan ships **production
BM25** (via the `bm25` crate v2.3.2) with Russian Snowball stemming,
template-noise stripping, and 1-hop graph expansion. Semantic vectors are
used as a booster only with a compatible build, cached model and eligible vectors.
Reads do not download the model or embed missing inputs.

For the local `0.39.0+constellation.s1.1` build, migrated workspaces use
Constellation. Ordinary `search` defaults to **20** results; the separate
`constellation search` command defaults to **10**. Do not infer one
interface's parameters or response shape from the other.

Forced `--semantic` does not guarantee keyword fallback on a migrated
workspace. Check availability and coverage rather than treating empty
results as a complete semantic answer. See the
[Constellation guide](/docs/guides/constellation/) and
[reference](/docs/reference/constellation/).

Forced modes have narrower filtering in this revision: `--keyword` uses query
and kind, not status/depth/evidence/date/limit; migrated `--semantic` uses kind,
status and limit, not the other smart filters. Use smart mode for composite filters.

## When to use

- You remember a concept but not the ID
- Onboarding - "what do we have on authentication?"
- Drafting a new artifact - find related prior decisions to link
- AI agents doing retrieval-augmented reasoning

## Not to use when

- You know the exact ID → use [`forgeplan get`](/docs/cli/get/)
- You want a structural view → use [`forgeplan tree`](/docs/cli/tree/) or
  [`forgeplan graph`](/docs/cli/graph/)
- You want to search personal memories, not artifacts → use
  [`forgeplan recall`](/docs/cli/recall/)

## Usage

```text
forgeplan search [OPTIONS] <QUERY>
```

## Arguments

```text
  <QUERY>  Search query
```

## Options

```text
  -t, --type <TYPE>      Filter by kind (prd, rfc, adr, note, ...)
  -s, --status <STATUS>  Filter by status (draft, active, superseded, deprecated, stale)
      --depth <DEPTH>    Filter by depth (tactical, standard, deep, critical)
      --with-evidence    Only artifacts with evidence linked (R_eff > 0)
      --no-evidence      Only artifacts WITHOUT evidence (blind spots)
      --since <SINCE>    Only artifacts created after this date (YYYY-MM-DD)
      --no-expand        Disable graph expansion (1-hop neighbors in results)
      --keyword          Force keyword-only search (substring grep)
      --semantic         Force semantic-only search (vector similarity)
  -n, --limit <LIMIT>    Max results to return (default: 20) [default: 20]
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

Smart default - BM25 + semantic + graph expansion:

```bash
forgeplan search "authentication flow"
```

Narrow to active PRDs only, top 5 hits:

```bash
forgeplan search "auth" --type prd --status active --limit 5
```

Find decisions that have no evidence backing them (a blind-spot variant):

```bash
forgeplan search "rate limit" --no-evidence
```

## Output interpretation

Illustrative output (not a captured runtime transcript):

```
PRD-001  [active ]  score=8.42  Auth system (BM25 + semantic)
RFC-002  [active ]  score=6.71  Token refresh flow      (graph-expanded from PRD-001)
ADR-004  [active ]  score=5.30  JWT vs session cookies
```

Columns: `ID`, `STATUS`, `SCORE`, `TITLE`, plus a `(graph-expanded)` marker
when a hit came in as a 1-hop neighbor rather than a direct term match.
Scores are not comparable across queries - they rank within one call only.

Smart search's stemming can match inflected forms, not every related word.
Forced keyword mode is substring matching, not Snowball matching.

## How it fits

`search` is the primary discovery surface:

```
search → get → link (during artifact authoring)
search → get → reason (during AI planning)
```

For the full pipeline details (indexing, stemming, BM25 parameters, graph
expansion), see the [Search v2 guide](/docs/guides/search-v2/).

## See also

- [Search v2 guide](/docs/guides/search-v2/) - architecture and tuning
- [`forgeplan get`](/docs/cli/get/) - read a hit in full
- [`forgeplan recall`](/docs/cli/recall/) - search memories (not artifacts)
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the BM25 index

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan search --help`.

```text
Search artifacts (smart by default: keyword + semantic + boosters)

Usage: forgeplan search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  -t, --type <TYPE>      Filter by kind (prd, rfc, adr, note, ...)
  -s, --status <STATUS>  Filter by status (draft, active, superseded, deprecated, stale)
      --depth <DEPTH>    Filter by depth (tactical, standard, deep, critical)
      --with-evidence    Only artifacts with evidence linked (R_eff > 0)
      --no-evidence      Only artifacts WITHOUT evidence (blind spots)
      --since <SINCE>    Only artifacts created after this date (YYYY-MM-DD)
      --no-expand        Disable graph expansion (1-hop neighbors in results)
      --keyword          Force keyword-only search (substring grep)
      --semantic         Force semantic-only search (vector similarity)
  -n, --limit <LIMIT>    Max results to return (default: 20) [default: 20]
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
