---
title: forgeplan tag
description: "Add tags to an artifact"
---

Attach one or more tags to an artifact. Tags are the primary cross-cutting
discovery mechanism — they let you group artifacts by theme, layer, source, or
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

1. Loads the artifact's frontmatter.
2. Canonicalizes each tag (lowercased, whitespace trimmed, deduped — see
   PROB-026 fix in v0.18).
3. Merges new tags into the existing `tags:` list.
4. Rewrites frontmatter and re-indexes the artifact in LanceDB.

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
- Preserved `=` and `-` for key-value and hyphenated forms.

Workspaces created before v0.18 may contain mixed-case tag rows. Run
[`forgeplan reindex`](/docs/cli/reindex/) to re-canonicalize every artifact in a
single pass.

## Discoverability: where tags show up

Tags power several downstream commands:

| Command                                                 | How tags are used                                   |
|---------------------------------------------------------|-----------------------------------------------------|
| [`forgeplan discover`](/docs/cli/discover/)             | Group artifacts by tag, surface tag frequency       |
| [`forgeplan search --tag <t>`](/docs/cli/search/)       | Filter BM25 results to a tag intersection           |
| [`forgeplan list --tag <t>`](/docs/cli/list/)           | List artifacts matching a tag                       |
| [`forgeplan health`](/docs/cli/health/)                 | Surfaces untagged active artifacts                  |

A well-tagged workspace makes `discover` meaningful. An untagged workspace
makes `discover` noise.

## Tagging conventions (suggested)

The CLI does not enforce a taxonomy, but the following patterns work well:

- **Theme tags** (plain words): `auth`, `security`, `performance`, `docs`.
- **Source tags** (`key=value`): `source=quint-code`, `source=bmad`,
  `source=dogfood` — trace where a decision or evidence came from.
- **Layer tags**: `layer=cli`, `layer=core`, `layer=mcp`.
- **Status flags**: `legacy`, `experimental`, `blocked`, `wip`.

Pick a small, opinionated vocabulary and stick to it. Tag sprawl kills
discovery.

## Source Tier and CL mapping (v0.17.0, PRD-035)

Tags of the form `source_tier=T1`, `source_tier=T2`, or `source_tier=T3` have
special meaning in Forgeplan's trust calculus. They map directly to a
Congruence Level that feeds into R_eff scoring:

| Tag value          | Congruence Level | CL penalty | Meaning                          |
|--------------------|------------------|------------|----------------------------------|
| `source_tier=T1`   | CL3              | 0.0        | Same context — highest trust     |
| `source_tier=T2`   | CL2              | 0.1        | Similar context — minor penalty  |
| `source_tier=T3`   | CL1              | 0.4        | Different context — significant penalty |

### Security precedence (`min(tier_cl, explicit_cl)`)

If an artifact has **both** a `source_tier` tag and an explicit `congruence_level`
in its Structured Fields body, Forgeplan takes the **lower** (more conservative)
of the two:

```
final_cl = min(tier_cl, explicit_cl)
```

This prevents **CL upgrade attacks via tag manipulation**. For example:

- An evidence artifact is tagged `source_tier=T1` (would imply CL3).
- But the body contains `congruence_level: 0` (explicit CL0).
- Final CL = **min(CL3, CL0) = CL0** — the explicit low-trust field wins.

The conservative-by-default rule means a tag can never **increase** trust
beyond what the structured fields claim. Tags can only narrow the CL downward,
never upward.

### Use case: brownfield onboarding

When ingesting artifacts from an external source via the discovery protocol
(`forgeplan discover`), tier tags let you batch-classify trust levels:

```bash
# External vendor docs — low trust
forgeplan tag PRD-050 source_tier=T3 source=vendor-api

# Internal PoC from a sibling team — moderate trust
forgeplan tag PRD-051 source_tier=T2 source=team-alpha

# Our own production benchmark — highest trust
forgeplan tag EVID-060 source_tier=T1 source=dogfood
```

See [Evidence methodology — SourceTier precedence](/docs/methodology/evidence/#gotchas-and-migration-notes)
for how this interacts with R_eff scoring after upgrade.

## Notes

- Tags are stored in frontmatter as a YAML list. Direct markdown edits work,
  but re-run `forgeplan scan-import` afterward.
- To remove tags, use [`forgeplan untag`](/docs/cli/untag/).
- The `<TAGS>...` positional argument is variadic — pass as many as you like in
  a single invocation.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan untag`](/docs/cli/untag/) — remove tags
- [`forgeplan discover`](/docs/cli/discover/) — browse by tag
- [`forgeplan search`](/docs/cli/search/) — filter search by tag
- [Methodology guide](/docs/methodology/overview/)
