---
title: forgeplan link
description: "Link two artifacts with a typed relationship"
---

Create a typed, directed relation from one artifact to another. Relations are
how Forgeplan builds the decision graph — they power health checks, R_eff
scoring, the dependency Mermaid graph, topological sort for sprint planning,
and blind-spot detection.

Every significant artifact should have at least one incoming or outgoing link.
Orphans are the #1 signal surfaced by [`forgeplan health`](/docs/cli/health/).

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
      --relation <RELATION>  Relationship type: informs, based_on, supersedes, contradicts, refines [default: informs]
  -h, --help                 Print help
  -V, --version              Print version
```

## Valid relations (v0.18)

Forgeplan enforces a closed vocabulary. Only these five relation types are
accepted — any other value is rejected by the validator.

| Relation      | Direction reads as...                          | When to use                                                                 |
|---------------|------------------------------------------------|-----------------------------------------------------------------------------|
| `informs`     | *source* informs *target*                      | Evidence supporting a PRD; a Note feeding into an ADR. Most common.         |
| `based_on`    | *source* is based on *target*                  | RFC built on top of an earlier PRD; PRD derived from a ProblemCard.         |
| `supersedes`  | *source* supersedes *target*                   | New ADR replaces an old one. Set automatically by `forgeplan supersede`.    |
| `contradicts` | *source* contradicts *target*                  | Evidence that refutes a decision; RFC that rejects an earlier approach.    |
| `refines`     | *source* refines *target*                      | Spec that sharpens a PRD; ADR that narrows an RFC's design space.          |

## Examples

The canonical evidence flow — link a fresh EvidencePack to the PRD it supports:

```bash
forgeplan new evidence "Benchmark: BM25 vs TF-IDF on Russian corpus"
# ... fill in verdict, congruence_level, evidence_type ...
forgeplan link EVID-001 PRD-039 --relation informs
forgeplan score PRD-039   # R_eff should now be > 0
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

When in doubt, pick `informs` — it's the neutral "this artifact is relevant to
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

- The link is persisted to the LanceDB `links` table.
- [`forgeplan score`](/docs/cli/score/) recomputes R_eff for the target —
  `informs` links from `verdict: supports` evidence raise the score.
- [`forgeplan graph`](/docs/cli/graph/) includes the edge in the Mermaid output.
- [`forgeplan blocked`](/docs/cli/blocked/) and [`forgeplan order`](/docs/cli/order/)
  re-run topological sort across `based_on` / `refines` edges.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan unlink`](/docs/cli/unlink/) — remove a relation
- [`forgeplan score`](/docs/cli/score/) — evaluate R_eff after linking evidence
- [`forgeplan graph`](/docs/cli/graph/) — visualize the relation graph
- [`forgeplan health`](/docs/cli/health/) — find orphans and missing evidence
- [Methodology guide](/docs/methodology/overview/)
