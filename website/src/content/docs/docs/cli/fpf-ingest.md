---
title: forgeplan fpf ingest
description: "Ingest the FPF spec into the local knowledge base (one-time setup)"
---

`forgeplan fpf ingest` parses externally installed **First Principles Framework**
sections and replaces the auxiliary `fpf_spec` table. The count depends on the
installed revision; there is no fixed 204-section corpus bundled in the binary.
With `semantic-search`, ingestion also encodes sections with BGE-M3.
Rules and checks use configuration/default rules separately from this corpus.

## When to use

- **Right after `forgeplan init -y`** - seed the KB before doing any reasoning work.
- **On a fresh clone** - ingest the missing auxiliary KB; preserve existing state for diagnosis.
- **After updating the installed FPF corpus** - re-ingest the chosen revision deliberately.
- **When `fpf status` reports the KB as stale or empty.**

## When NOT to use

- You don't need to run it before every reasoning session; the KB persists across invocations.
- Don't run it in parallel with `forgeplan reason --fpf` - let ingest finish first.
- Not a replacement for artifact indexing: use `index`/`reindex` to rebuild the index from canonical Markdown. `scan-import` imports external Markdown, including through the migrated file-first path; it is not a canonical index rebuild.

## Usage

```text
forgeplan fpf ingest [OPTIONS]
```

## Options

```text
      --path <PATH>  Path to FPF sections directory
  -h, --help         Print help
  -V, --version      Print version
```

**The FPF spec is not bundled in the binary** — it ships as a Claude Code skill and has to be installed separately. Without it there is nothing to ingest, and `fpf search` will keep returning "no matches", which reads like an empty corpus rather than a missing one.

By default `fpf ingest` looks for the sections in your skills directory, newest name first:

```text
~/.claude/skills/fpf-knowledge/sections
~/.claude/skills/fpf/sections
~/.claude/skills/fpf-simple/sections     # historical name, still accepted
```

Install the `fpf` skill from the [marketplace](https://github.com/ForgePlan/marketplace) to get it. Pass `--path` to ingest from anywhere else — a checkout of the spec, a patched corpus, or the skill directory inside a marketplace clone:

```bash
forgeplan fpf ingest --path <marketplace>/plugins/fpf/skills/fpf-knowledge/sections
```

## Examples

```bash
# First-time setup - uses the installed skill
forgeplan init -y
forgeplan fpf ingest

# Re-ingest after upgrade
forgeplan fpf ingest
forgeplan fpf status

# Ingest from an external FPF sections directory
forgeplan fpf ingest --path ./fpf-sections/
```

## What happens

1. Forgeplan locates the FPF sections — the installed skill, or whatever `--path` points at.
2. Sections are parsed into structured chunks (ID, title, part, body).
3. Existing FPF rows are cleared. With `semantic-search`, BGE-M3 encodes the sections;
   model-load and encoding errors propagate. Builds without that feature write keyword-only rows.
4. The auxiliary `fpf_spec` table stores records and optional embeddings.
5. Each record gets a creation timestamp; `fpf status` does not display ingest time.

No duration is guaranteed. Clearing old rows precedes encoding, so an ingestion
failure can leave the KB empty. Preserve the external source corpus.

## How it fits

Ingest is the bootstrap step for everything else under `forgeplan fpf`:

```
forgeplan fpf ingest      ← one-time
  ├── fpf search          ← works after ingest
  ├── fpf section B.3
  ├── fpf list
  └── fpf status          ← count-based inspection
```

`reason --fpf` requests FPF context, but the selected CLI does not reopen retired
legacy KB tables on migrated workspaces. Ingestion does not establish that every
FPF command is supported there.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - verify ingest succeeded
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - query the ingested KB
- [`forgeplan init`](/docs/cli/init/) - workspace bootstrap that precedes ingest

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf ingest --help`.

```text
Ingest FPF spec into knowledge base

Usage: forgeplan fpf ingest [OPTIONS]

Options:
      --path <PATH>  Path to FPF sections directory
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
