---
title: forgeplan fpf ingest
description: "Ingest the FPF spec into the local knowledge base (one-time setup)"
---

`forgeplan fpf ingest` loads separately installed **First Principles Framework** section files into the workspace's `fpf_spec` table under `.forgeplan/lance/`. It creates one row per nonempty Markdown file in immediate section subdirectories, excluding `_index.md`; top-level Markdown and deeper nested files are not scanned. The number of sections depends on the installed corpus.

Run it after initializing a workspace when you need `fpf search`, `fpf section`, or `fpf list`. `fpf rules`, `fpf check`, and `fpf dashboard` use artifact data and configuration, so they do not need the KB to be ingested.

## When to use

- **Right after `forgeplan init -y`** - seed the KB before doing any reasoning work.
- **In a fresh clone** - the derived LanceDB KB is gitignored and must be populated locally.
- **After updating the separately installed FPF corpus** - a binary upgrade alone does not supply new section files.
- **When `fpf status` reports the KB as stale or empty.**

## When NOT to use

- You don't need to run it before every reasoning session; the KB persists across invocations.
- Don't run it in parallel with `forgeplan reason --fpf` - let ingest finish first.
- Not a replacement for `forgeplan reindex` - that scans workspace **artifact** projections, not the FPF KB. It imports new records, syncs differing bodies, adds missing links, and removes orphans; it does not completely refresh existing-record metadata or remove obsolete links between surviving records.

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

**The FPF spec is not bundled in the binary**: it ships as a Claude Code skill and has to be installed separately. Without it there is nothing to ingest, and `fpf search` will keep returning "no matches", which reads like an empty corpus rather than a missing one.

By default `fpf ingest` looks for the sections in your skills directory, newest name first:

```text
~/.claude/skills/fpf-knowledge/sections
~/.claude/skills/fpf/sections
~/.claude/skills/fpf-simple/sections     # historical name, still accepted
```

Install the `fpf` skill from the [marketplace](https://github.com/ForgePlan/marketplace) to get it. Pass `--path` to ingest from anywhere else: a checkout of the spec, a patched corpus, or the skill directory inside a marketplace clone.

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

1. Forgeplan locates the FPF sections: the installed skill, or whatever `--path` points at.
2. Files are parsed into IDs, parent-section names, titles, bodies, line counts, and source paths.
3. Existing FPF rows are cleared before encoding and insertion. This replaces the KB rather than appending; it does not clear the artifact index.
4. Builds with `semantic-search` encode section title and body with BGE-M3 (1024 dimensions; about 2.1 GB downloaded on first use). A build without that feature inserts keyword-only rows. With the feature enabled, model/encoding failures abort ingest; there is no runtime keyword-only fallback for this command.
5. Rows are inserted with a `created_at` timestamp. There is no separate ingest-status record.

Do not run ingest concurrently with readers. A failure after clearing the table can leave the KB empty; fix the source/model problem and re-ingest. No LLM provider or API key is needed. Cached weights avoid another download but do not eliminate CPU encoding work.

## How it fits

Ingest prepares the KB retrieval commands:

```text
forgeplan fpf ingest
  ├── fpf search
  ├── fpf section B.3
  └── fpf list
```

It also enables optional KB context for `forgeplan reason <ID> --fpf`. If no matching sections are available, `reason --fpf` reports that and continues without KB context; ingest is not a hard reasoning guard. `fpf status` compares source-file and row counts, not content hashes, so equal counts do not prove the corpus is unchanged.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - verify ingest succeeded
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - query the ingested KB
- [`forgeplan init`](/docs/cli/init/) - workspace bootstrap that precedes ingest
