---
title: "forgeplan constellation"
description: "Constellation registry — named sets of `.forgeplan` sources that `search`/`get` fan out over. The registry lives in `~/.forgeplan/constellations.yaml`: per-user, shared by every ForgePlan instance on the machine"
---

## Использование

Точная сигнатура и параметры выбранной сборки приведены ниже. Наличие команды не подтверждает поддержку всех раскладок workspace.

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan constellation --help`.

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

- [`forgeplan constellation create`](/ru/docs/cli/constellation-create/)
- [`forgeplan constellation add`](/ru/docs/cli/constellation-add/)
- [`forgeplan constellation remove`](/ru/docs/cli/constellation-remove/)
- [`forgeplan constellation rebind`](/ru/docs/cli/constellation-rebind/)
- [`forgeplan constellation list`](/ru/docs/cli/constellation-list/)
- [`forgeplan constellation use`](/ru/docs/cli/constellation-use/)
- [`forgeplan constellation sources`](/ru/docs/cli/constellation-sources/)
- [`forgeplan constellation doctor`](/ru/docs/cli/constellation-doctor/)
- [`forgeplan constellation snapshot`](/ru/docs/cli/constellation-snapshot/)
- [`forgeplan constellation search`](/ru/docs/cli/constellation-search/)
- [`forgeplan constellation get`](/ru/docs/cli/constellation-get/)
- [`forgeplan constellation pack`](/ru/docs/cli/constellation-pack/)
- [`forgeplan constellation graph`](/ru/docs/cli/constellation-graph/)
- [`forgeplan constellation impact`](/ru/docs/cli/constellation-impact/)
- [`forgeplan constellation export`](/ru/docs/cli/constellation-export/)
<!-- forgeplan-runtime:end -->

## Примеры

- [Воспроизводимый сценарий Constellation](/ru/docs/guides/constellation/)
- [Быстрый старт с артефактами](/ru/docs/getting-started/quick-start/)

## См. также

- [CLI reference](/ru/docs/cli/)
- [Constellation reference](/ru/docs/reference/constellation/)
