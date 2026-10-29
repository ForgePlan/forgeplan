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

1. Loads the artifact's frontmatter.
2. Canonicalizes each tag (lowercased, whitespace trimmed, deduped - see
   PROB-026 fix in v0.18).
3. Merges new tags into the existing `tags:` list.
4. Rewrites canonical frontmatter and updates the owning index. On Constellation, it requests reconciliation without reopening legacy tables.

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

For old mixed-case tags, inspect canonical Markdown and use `tag`/`untag` to correct
the labels. `index` maintains Constellation; do not assume an index rebuild rewrites
every source tag in place.

## Discoverability: where tags show up

Tags power several downstream commands:

| Command                                                 | How tags are used                                   |
|---------------------------------------------------------|-----------------------------------------------------|
| [`forgeplan list --tag <t>`](/docs/cli/list/) | Filter matching artifacts in the selected workspace |
| [`forgeplan get`](/docs/cli/get/) | Inspect canonical tags in the artifact |

The selected `search` has no `--tag` option, `discover` is a discovery-session
workflow rather than a tag browser, and `health` does not report untagged artifacts
as a distinct health class.

## Tagging conventions (suggested)

The CLI does not enforce a taxonomy, but the following patterns work well:

- **Theme tags** (plain words): `auth`, `security`, `performance`, `docs`.
- **Source tags** (`key=value`): `source=quint-code`, `source=bmad`,
  `source=dogfood` - trace where a decision or evidence came from.
- **Layer tags**: `layer=cli`, `layer=core`, `layer=mcp`.
- **Status flags**: `legacy`, `experimental`, `blocked`, `wip`.

Pick a small, opinionated vocabulary and stick to it. Tag sprawl kills
discovery.

## Source Tier and CL mapping (v0.17.0, PRD-035)

Labels such as `source_tier=T1` are provenance tags. **The current evidence parser
does not read the `tags:` list for scoring.** It reads standalone body fields,
including `source_tier: t1` and `congruence_level: 3`.

| Body field | Derived CL | Product CL penalty |
|---|---|---|
| `source_tier: t1` | CL3 | 0.0 |
| `source_tier: t2` | CL2 | 0.1 |
| `source_tier: t3` | CL1 | 0.4 |

### Security precedence (`min(tier_cl, explicit_cl)`)

When an EvidencePack body declares a legal `verdict` and both `source_tier: ...`
and `congruence_level: ...`, scoring takes the lower CL:

```
final_cl = min(tier_cl, explicit_cl)
```

For example, body fields `verdict: supports`, `source_tier: t1` and
`congruence_level: 0` produce CL0 despite the tier's CL3 mapping.
This conservative rule applies to scoring fields, **not tag manipulation**.
Without a recognized verdict or a CL declaration, scoring falls back to CL0.
These mappings are ForgePlan policy, not a universal current upstream FPF formula.

### Use case: brownfield onboarding

During discovery, tier tags can label provenance for later review. The commands
below do not themselves set the standalone EvidencePack scoring fields:

```bash
# External vendor docs - low trust
forgeplan tag PRD-050 source_tier=T3 source=vendor-api

# Internal PoC from a sibling team - moderate trust
forgeplan tag PRD-051 source_tier=T2 source=team-alpha

# Our own production benchmark - highest trust
forgeplan tag EVID-060 source_tier=T1 source=dogfood
```

See [Evidence methodology - SourceTier precedence](/docs/methodology/evidence/#gotchas-and-migration-notes)
for how this interacts with R_eff scoring after upgrade.

## Notes

- Tags are stored as a frontmatter list. Prefer `tag`/`untag` over direct edits;
  use `index` for Constellation maintenance, not `scan-import`.
- To remove tags, use [`forgeplan untag`](/docs/cli/untag/).
- The `<TAGS>...` positional argument is variadic - pass as many as you like in
  a single invocation.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan untag`](/docs/cli/untag/) - remove tags
- [`forgeplan discover`](/docs/cli/discover/) - protocol-driven discovery sessions
- [`forgeplan list`](/docs/cli/list/) - tag filtering on its supported storage path
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan tag --help`.

```text
Add tags to an artifact

Usage: forgeplan tag <ID> <TAGS>...

Arguments:
  <ID>       Artifact ID (e.g. PRD-001)
  <TAGS>...  Tags to add (e.g. source=code layer=auth legacy)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
