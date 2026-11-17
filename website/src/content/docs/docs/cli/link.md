---
title: forgeplan link
description: "Link two artifacts with a typed relationship"
---

Create a typed, directed relation from one artifact to another. Relations are
how Forgeplan builds the decision graph - they power health checks, R_eff
scoring, the dependency Mermaid graph, topological sort for sprint planning,
and blind-spot detection.

Every significant artifact should have at least one incoming or outgoing link.
Orphans are the #1 signal surfaced by [`forgeplan health`](/docs/cli/health/).

## Forgeplan 0.38.0

`--replace` changes the type on the same endpoint pair, not direction or lifecycle status. `--auto-activate-complete` requires source kind=evidence, status=draft, and explicit recognized verdict (supports/weakens/refutes) and congruence_level (0–3); evidence_type is not part of that predicate. Autoactivation failure warns after the link is committed, rather than rolling it back. A relation name does not set the evidence verdict. The source must exist; a missing target is accepted as a warned forward reference, not as a scored existing decision.

## Usage

```text
forgeplan link [OPTIONS] <SOURCE> <TARGET>
```

## Arguments

```text
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID
```

## Options

```text
      --relation <RELATION>     Relationship type: informs, based_on, supersedes, contradicts, refines [default: informs]
      --replace                 Issue #286: if an edge already exists between source and target with a DIFFERENT relation, replace it (idempotent upsert). Without this flag, an existing edge with a different relation surfaces as an error and the call fails — matching the pre-issue-#286 behaviour. Used to fix mis-typed `based_on` / `informs` choices without destroying the artifact via `forgeplan delete`
      --auto-activate-complete  Issue #288 Part A: when the link source is a complete evidence artifact (kind=evidence + status=draft + body has verdict + congruence_level), silently activate it draft → active in the same call. Mirrors `auto_activate_source_if_complete` on the MCP `forgeplan_link` tool. No-op when the source doesn't meet the criteria
  -h, --help                    Print help
  -V, --version                 Print version
```

## Valid relations (v0.18)

Forgeplan enforces a closed vocabulary. Only these five relation types are
accepted - any other value is rejected by the validator.

| Relation      | Direction reads as...                          | When to use                                                                 |
|---------------|------------------------------------------------|-----------------------------------------------------------------------------|
| `informs`     | *source* informs *target*                      | Evidence supporting a PRD; a Note feeding into an ADR. Most common.         |
| `based_on`    | *source* is based on *target*                  | RFC built on top of an earlier PRD; PRD derived from a ProblemCard.         |
| `supersedes` | old ID → replacement ID | Direction written by the released `forgeplan supersede` command, not new → old. |
| `contradicts` | *source* contradicts *target*                  | Evidence that refutes a decision; RFC that rejects an earlier approach.    |
| `refines`     | *source* refines *target*                      | Spec that sharpens a PRD; ADR that narrows an RFC's design space.          |

## Examples

The canonical evidence flow - link a fresh EvidencePack to the PRD it supports:

```bash
forgeplan new evidence "Benchmark: BM25 vs TF-IDF on Russian corpus"
# ... fill in verdict, congruence_level, evidence_type ...
forgeplan link EVID-001 PRD-039 --relation informs
forgeplan score PRD-039   # Inspect the actual minimum; support is not guaranteed.
```

Derive an RFC from a PRD:

```bash
forgeplan link RFC-006 PRD-025 --relation based_on
```

Record a contradiction (Evidence that refutes):

```bash
forgeplan link EVID-017 ADR-004 --relation contradicts
```

Refine a PRD with a spec:

```bash
forgeplan link SPEC-003 PRD-018 --relation refines
```

## Common mistakes: relations that don't exist

Older docs and LLM completions sometimes suggest relation types that are
**not** valid in v0.18. Here is the translation table:

| You might type... | Use instead                          | Why                                                       |
|-------------------|---------------------------------------|-----------------------------------------------------------|
| `solves`          | `based_on` (PRD → ProblemCard)       | A solution is *based on* the problem it addresses         |
| `extends`         | `refines` or `based_on`              | `refines` for narrowing scope; `based_on` for inheritance |
| `blocks`          | `based_on` on the blocked artifact   | Dependencies are expressed by what the child is built on  |
| `depends_on`      | `based_on`                            | Same semantics                                            |
| `implements`      | `refines`                             | Spec/RFC refines the intent of a PRD                      |
| `references`      | `informs`                             | Default soft link                                         |

When in doubt, pick `informs` - it's the neutral "this artifact is relevant to
that one" link.

## Direction matters

Relations are directed. `forgeplan link PRD-001 EVID-001 --relation informs`
says *PRD-001 informs EVID-001*, which is almost never what you want. The
standard pattern is always **evidence → decision**:

```bash
forgeplan link EVID-001 PRD-001 --relation informs   # correct
```

If you link the wrong direction, fix it with
[`forgeplan unlink`](/docs/cli/unlink/) followed by a fresh `link`.

## Self-links are blocked

As of PROB-019, `forgeplan link PRD-001 PRD-001` is rejected. This guards
against typos that would otherwise create self-referential nodes in the graph
and confuse topological sort.

## What happens after linking

- The link is persisted to the LanceDB relation store.
- The CLI requests a cached-score refresh for the source, not the target. Run `forgeplan score TARGET` explicitly after changing evidence links. Additional evidence can lower the weakest-link minimum.
- [`forgeplan graph`](/docs/cli/graph/) includes the edge in the Mermaid output.
- [`forgeplan blocked`](/docs/cli/blocked/) and [`forgeplan order`](/docs/cli/order/)
  re-run topological sort across `based_on` / `refines` edges.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan unlink`](/docs/cli/unlink/) - remove a relation
- [`forgeplan score`](/docs/cli/score/) - evaluate R_eff after linking evidence
- [`forgeplan graph`](/docs/cli/graph/) - visualize the relation graph
- [`forgeplan health`](/docs/cli/health/) - find orphans and missing evidence
- [Methodology guide](/docs/methodology/overview/)
