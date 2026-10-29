---
title: "forgeplan constellation"
description: "Constellation registry — named sets of `.forgeplan` sources that `search`/`get` fan out over. The registry lives in `~/.forgeplan/constellations.yaml`: per-user, shared by every ForgePlan instance on the machine"
---

## Usage

The exact signature and options of the selected build appear below. Registration does not prove support for every workspace layout.

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation --help`.

```text
Constellation registry — named sets of `.forgeplan` sources that `search`/`get` fan out over. The registry lives in `~/.forgeplan/constellations.yaml`: per-user, shared by every ForgePlan instance on the machine

Usage: forgeplan constellation <COMMAND>

Commands:
  create    Create a named constellation. The first one created becomes active
  add       Add a `.forgeplan` directory as a source. Validates the path at registration time — a symlink or a non-workspace fails here, not at every later read
  remove    Remove a source from a constellation
  rebind    Rebind a source after its checkout moved. Source identity is path-derived, so a moved checkout is a new source: this rewrites the registry path and, by default, the index's path-bound marker so `forgeplan index` answers at the new location. References qualified by the old `src:` id stop resolving — that is the contract
  list      List all constellations and which is active
  use       Select the active constellation for `search`/`get`
  sources   List the sources of a constellation (default: active)
  doctor    Health-check every registered source: reachable, looks like a workspace, has a published index
  snapshot  Pin every source of the active constellation at its current Git HEAD and write the lockfile to `~/.forgeplan/snapshots/`. Sources that are not git checkouts are reported, not pinned
  search    Search across the active constellation's sources
  get       Fetch one artifact by id across the active constellation. When two sources claim the same id the answer is `ambiguous` and names them
  pack      Stage 6 — build a budgeted ContextPack for a task: lexical ranking, evidence units with roles, omissions and a spend receipt
  graph     The relation graph of the active constellation (or a recorded snapshot): every artifact as a source-qualified node, every declared link as a typed edge with its provenance
  impact    Stage 7 — explainable impact: who depends on this artifact, by which relation path, and where the bounded walk stopped
  export    Stage 9 — portable export: every document and edge of the active constellation (or a recorded snapshot) as JSONL or SCIP-shaped records, with provenance and no absolute local paths
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan constellation create`](/docs/cli/constellation-create/)
- [`forgeplan constellation add`](/docs/cli/constellation-add/)
- [`forgeplan constellation remove`](/docs/cli/constellation-remove/)
- [`forgeplan constellation rebind`](/docs/cli/constellation-rebind/)
- [`forgeplan constellation list`](/docs/cli/constellation-list/)
- [`forgeplan constellation use`](/docs/cli/constellation-use/)
- [`forgeplan constellation sources`](/docs/cli/constellation-sources/)
- [`forgeplan constellation doctor`](/docs/cli/constellation-doctor/)
- [`forgeplan constellation snapshot`](/docs/cli/constellation-snapshot/)
- [`forgeplan constellation search`](/docs/cli/constellation-search/)
- [`forgeplan constellation get`](/docs/cli/constellation-get/)
- [`forgeplan constellation pack`](/docs/cli/constellation-pack/)
- [`forgeplan constellation graph`](/docs/cli/constellation-graph/)
- [`forgeplan constellation impact`](/docs/cli/constellation-impact/)
- [`forgeplan constellation export`](/docs/cli/constellation-export/)
<!-- forgeplan-runtime:end -->

## Examples

- [Reproducible Constellation walkthrough](/docs/guides/constellation/)
- [Artifact quick start](/docs/getting-started/quick-start/)

## See also

- [CLI reference](/docs/cli/)
- [Constellation reference](/docs/reference/constellation/)
