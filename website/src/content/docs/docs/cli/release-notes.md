---
title: forgeplan release-notes
description: "Generate Keep-a-Changelog-shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` and categorises each touched artifact: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Quality gate: only artifacts with `status==active` or `r_eff_score > 0` are emitted (override with `--draft`)"
---

Generate Keep-a-Changelog-shaped release notes from the artifacts that
changed between two git refs. This closes the sprint pain point where a
changelog had to be reconstructed by hand by reading every artifact that
landed since the last tag.

The command is a thin rind over `forgeplan_core::release_notes`: it resolves
the ref range, walks `git log`, classifies each touched artifact into a
Keep-a-Changelog section, and hands the result to one of three formatters.

## Usage

```text
forgeplan release-notes [OPTIONS]
```

## Options

```text
      --since <SINCE>    Git ref to start from (default: latest tag)
      --until <UNTIL>    Git ref to end at (default: HEAD)
      --output <OUTPUT>  Output format: text, markdown (alias md), json. Default: markdown [default: markdown]
      --draft            Disable the quality gate — include active artifacts without evidence and drafts with r_eff_score=0
  -h, --help             Print help
  -V, --version          Print version
```

## What it does

1. Locates the workspace (`.forgeplan/`) and uses its parent as the git repo root.
2. Resolves the ref range: `--since` defaults to `git describe --tags --abbrev=0`
   and **falls back to `HEAD~50` when tag discovery fails**; `--until`
   defaults to `HEAD`. A repository with fewer than 51 commits may not resolve that fallback; pass a real starting commit/tag explicitly.
3. Runs `git log --name-only --pretty=format:'COMMIT %H' --diff-filter=AM
   <since>..<until> -- <dirs>` over the artifact directories, keeping only
   `.md` files and the commit SHA that touched each one.
4. Maps each basename back to an artifact id and loads the record from the
   store.
5. Applies the quality gate and classifies each surviving record into a
   section.
6. Formats the result as markdown, plain text, or JSON.

## Directories it walks

Only these eight artifact directories are inspected - notes, memories, and
refresh reports are deliberately excluded, because `classify` filters them
out anyway:

```text
.forgeplan/prds/       .forgeplan/problems/   .forgeplan/evidence/
.forgeplan/rfcs/       .forgeplan/adrs/       .forgeplan/specs/
.forgeplan/epics/      .forgeplan/solutions/
```

The walk uses `--diff-filter=AM`, so **only added and modified** files in the
range count - a deletion is not a changelog event.

## The quality gate

An artifact is emitted only when it has signal:

```text
status == "active"  ||  r_eff_score > 0.0
```

On a fresh branch where everything is still `draft` with `r_eff_score = 0`,
the default invocation therefore reports **nothing** even though the range
contains commits. `--draft` waives the gate entirely - it exists so you can
preview a release while unevidenced artifacts are still in flight.

## Categories

| Artifact kind | Section |
|---|---|
| `prd` | Added |
| `problem` | Fixed |
| `evidence` itself tagged `security`, or linked 1-hop to a `security`-tagged artifact | Security |
| `evidence` with no such security tag on itself or neighbours | Internal |
| `rfc`, `adr`, `spec`, `epic`, `solution` | Changed |
| anything else | Internal |
| `note`, `memory`, `refresh` | never emitted |

Sections always render in the fixed order Added, Fixed, Security, Changed,
Internal; empty sections are skipped in markdown and text but are still
present as keys in the JSON payload.

## Examples

Output values below are illustrative, not results of an executed release check. Text/Markdown blocks show the formatted notes; the CLI can append its terminal hint marker.

### Example 1: the gate filters everything out

On a branch where the artifacts are still drafts:

```bash
forgeplan release-notes
```

Typical output:

```text
## [v0.1.0 → HEAD]

_No artifacts matched the requested range._
```

### Example 2: same range with the gate disabled

```bash
forgeplan release-notes --draft
```

Typical output:

```text
## [v0.1.0 → HEAD]
> Draft mode — quality gate disabled.

### Added
- Harden the ingest pipeline (PRD-001, commit 5453008)

### Fixed
- Ingest worker drops rows (PROB-001, commit 5453008)

### Changed
- Ingest worker transport (RFC-001, commit 5453008)
```

Each entry carries the current stored title and artifact ID, an optional incoming EvidencePack ID (`evid` in JSON), and a seven-character SHA selected from commits that touched the file. When several commits touched it, selection is lexicographic by full SHA, not necessarily the newest commit. The evidence choice is an incoming ID-prefix match, not proof of a closing verification or current evidence quality.

### Example 3: plain text for a terminal preview

```bash
forgeplan release-notes --draft --output text
```

Typical output:

```text
v0.1.0 -> HEAD
(draft mode — quality gate disabled)

Added:
  - Harden the ingest pipeline (PRD-001, 5453008)

Fixed:
  - Ingest worker drops rows (PROB-001, 5453008)

Changed:
  - Ingest worker transport (RFC-001, 5453008)
```

### Example 4: JSON for a bot or a build step

```bash
forgeplan release-notes --draft --output json
```

```json
{
  "added": [
    {
      "commit": "5453008",
      "id": "PRD-001",
      "kind": "prd",
      "title": "Harden the ingest pipeline"
    }
  ],
  "changed": [
    {
      "commit": "5453008",
      "id": "RFC-001",
      "kind": "rfc",
      "title": "Ingest worker transport"
    }
  ],
  "draft": true,
  "fixed": [
    {
      "commit": "5453008",
      "id": "PROB-001",
      "kind": "problem",
      "title": "Ingest worker drops rows"
    }
  ],
  "internal": [],
  "security": [],
  "since": "v0.1.0",
  "total": 3,
  "until": "HEAD"
}
```

### Example 5: an explicit ref range

Both flags accept a branch, a tag, or a full SHA, and they are joined as
`since..until`:

```bash
forgeplan release-notes --draft --since 5c2b06abdf0c896c9c96961791012b1b2aafb251 --until 54530087bd1ca826f3b039f5aed4d0621aaa3768
```

Typical output:

```text
## [5c2b06abdf0c896c9c96961791012b1b2aafb251 → 54530087bd1ca826f3b039f5aed4d0621aaa3768]
> Draft mode — quality gate disabled.

### Added
- Harden the ingest pipeline (PRD-001, commit 5453008)

### Fixed
- Ingest worker drops rows (PROB-001, commit 5453008)

### Changed
- Ingest worker transport (RFC-001, commit 5453008)
```

`--output md` is accepted as an alias for `markdown`.

## Notes

- **Refs are validated before git sees them.** `--since` and `--until` go
  through `validate_git_ref`, which rejects a leading `-` (CWE-88: a
  leading-dash ref would be parsed as a git CLI option), a `..` range
  operator, a first character that is not alphanumeric, and any character
  outside `[A-Za-z0-9_/.-]`.
- **A clean error beats a guessed range.** An unsupported `--output` value
  lists the valid set:

```bash
forgeplan release-notes --output xml
```

Typical output:

```text
Error: unsupported --output "xml"; expected one of: text, markdown, json
```

- The command never writes to the workspace - it is a read-only projection
  over git history plus the artifact store.
- Deleted artifacts do not appear: `--diff-filter=AM` excludes `D`.
- A basename is mapped to an id in three shapes: `KIND-NNN-slug.md` and
  `KIND-NNN.md` become uppercase ids (`PRD-074`), while the pre-merge slug
  form is passed through lowercased as-is.
- Git history selects touched paths, but status, title, score, and links come from the current artifact store, not snapshots at `--until`. Synchronize the store before generating notes and do not treat a historical range as a historical-state reconstruction.

## Related commands

- Git tags (created with `git tag`, not the artifact `forgeplan tag` command) drive the default `--since` ref.
- [`forgeplan drift`](/docs/cli/drift/) - the other git-history-backed
  report: which decisions' files moved after the decision was recorded.
- [`forgeplan health`](/docs/cli/health/) - run before cutting a release so
  the notes do not document debt.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan_release_notes`](/docs/mcp/forgeplan_release_notes/) - the MCP mirror of this command
- [`forgeplan export`](/docs/cli/export/) - freeze the whole workspace at a release boundary
- [`forgeplan score`](/docs/cli/score/) - the R_eff the quality gate reads
- [Methodology guide](/docs/methodology/overview/)
