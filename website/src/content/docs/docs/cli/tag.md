---
title: forgeplan tag
description: "Add tags to an artifact"
---

Attach one or more tags to an artifact. Tags are the primary cross-cutting
discovery mechanism - they let you group artifacts by theme, layer, source, or
status flag **without** creating a parent/child relation.

Tags complement, rather than replace, the artifact graph. Use relations
([`forgeplan link`](/docs/cli/link/)) to express decision lineage. Use tags to
slice the workspace along orthogonal axes like "everything touching auth",
"everything tagged `legacy`", or "all artifacts with `source=quint-code`".

## Usage

```text
forgeplan tag <ID> <TAGS>...
```

## Arguments

```text
  <ID>       Artifact ID (e.g. PRD-001)
  <TAGS>...  Tags to add (e.g. source=code layer=auth legacy)
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## What it does

1. Loads the stored record by its exact ID and synchronizes existing projection edits.
2. Canonicalizes new tags: trim, lowercase, replace spaces with `-`, keep alphanumeric characters and `-`, `_`, `=`, `.`; drop empty results and duplicates.
3. Merges them into `tags`, updates the index, and renders the Markdown projection.

Use the exact ID from `list`; this handler does not resolve a display ID to a slug-backed record. Output is text with the added-count estimate and current tags, not JSON. A missing record is an error.

## Examples

Single-word tags:

```bash
forgeplan tag PRD-001 security auth
```

Key-value tags (useful for structured filters):

```bash
forgeplan tag PRD-018 source=openspec layer=storage
```

Multiple styles at once:

```bash
forgeplan tag EVID-012 benchmark performance source=dogfood
```

## Tag canonicalization (v0.18)

As of PROB-026 fix in v0.18.0, tags are canonicalized on write:

- Case-folded to lowercase: `Security` → `security`.
- Trimmed of surrounding whitespace.
- Deduplicated: `forgeplan tag PRD-001 auth auth Auth` adds `auth` once.
- Spaces inside a tag become hyphens; characters other than alphanumeric, `-`, `_`, `=`, and `.` are removed. For example, quoted `"Auth Layer!"` becomes `auth-layer`.

Legacy or manually edited projections can contain mixed-case tags. `reindex` extracts projection tags when creating a missing record; for an existing record it incrementally synchronizes body changes, not the stored tags column. It neither repairs existing tag metadata nor normalizes every tag in place. Correct existing metadata through `untag` / `tag`: remove the exact stored value, then add the desired spelling.

## Discoverability: where tags show up

Tags power several downstream commands:

| Command                                                 | How tags are used                                   |
|---------------------------------------------------------|-----------------------------------------------------|
| [`forgeplan tag <ID> <TAGS>...`](/docs/cli/tag/)        | Add tags to an artifact                             |
| [`forgeplan untag <ID> <TAGS>...`](/docs/cli/untag/)    | Remove tags from an artifact                        |
| [`forgeplan list --tag <t>`](/docs/cli/list/)           | List artifacts matching a tag                       |
| [`forgeplan health`](/docs/cli/health/)                 | Surfaces untagged active artifacts                  |

A consistent tag vocabulary is what makes `list --tag` and the untagged-artifact
count in `health` useful. Tag sprawl makes both noise.

## Tagging conventions (suggested)

The CLI does not enforce a taxonomy, but the following patterns work well:

- **Theme tags** (plain words): `auth`, `security`, `performance`, `docs`.
- **Source tags** (`key=value`): `source=quint-code`, `source=bmad`,
  `source=dogfood` - trace where a decision or evidence came from.
- **Layer tags**: `layer=cli`, `layer=core`, `layer=mcp`.
- **Status flags**: `legacy`, `experimental`, `blocked`, `wip`.

Pick a small, opinionated vocabulary and stick to it. Tag sprawl kills
discovery.

## Source labels versus evidence fields

Tags such as `source_tier=t1`, `source_tier=t2`, and `source_tier=t3` can classify artifacts for discovery. In 0.38.0, the evidence parser reads `source_tier` and `congruence_level` from the **body**, not from `record.tags`. Adding or removing a source-tier tag alone does not set the scoring CL.

For scoring, declare reviewed evidence fields in its body, for example:

```markdown
## Structured Fields

verdict: supports
evidence_type: measurement
source_tier: t2
congruence_level: 2
```

This is an illustrative field fragment, not proof of a measurement. Body `source_tier` maps T1 → CL3, T2 → CL2, T3 → CL1; their CL penalties are 0.0, 0.1, and 0.4. If body tier and explicit CL are both present, the parser uses `min(tier_cl, explicit_cl)`. A label alone cannot establish the provenance or validity of evidence.

Discovery-label examples:

```bash
forgeplan tag PRD-050 source_tier=T3 source=vendor-api
forgeplan tag PRD-051 source_tier=T2 source=team-alpha
forgeplan tag EVID-060 source_tier=T1 source=dogfood
```

These tags become lowercase. Use the artifact mutation workflow to maintain evidence body fields and recompute `score` after substantive evidence changes. Repository agent rules can require CLI/MCP mutations even where the library supports projection synchronization; do not confuse library capability with permission to edit artifact files directly.

## Notes

- Tags are stored in the index and rendered as a YAML list in the projection. Correct tags on an existing record through `tag` / `untag`; `reindex` is not a repair for manual tag edits to that record's frontmatter.
- To remove tags, use [`forgeplan untag`](/docs/cli/untag/).
- The `<TAGS>...` positional argument is variadic - pass as many as you like in
  a single invocation.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan untag`](/docs/cli/untag/) - remove tags
- [`forgeplan list`](/docs/cli/list/) - `--tag <t>` lists artifacts by tag
- [Methodology guide](/docs/methodology/overview/)
