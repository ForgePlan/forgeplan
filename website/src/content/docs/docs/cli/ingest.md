---
title: forgeplan ingest
description: "Ingest engine (PRD-066 / SPEC-004) - apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs"
---

`forgeplan ingest` transforms external tool output into workspace artifacts using a mapping YAML and a source path. Matching rules render draft bodies and a mandatory `## Sources` section with file/line references. These references provide provenance, not proof that the mapped content is true.

This is the CLI side of the Wave-3 ingest pipeline. The MCP surface (`forgeplan_ingest`) is dry-run-only by design: it validates paths, echoes parameters, and emits a planned view pointing back at this command. Full execution, including the write to LanceDB and the markdown projection, happens here.

## When to use

- When a C4 model, an Obsidian vault, an ADR pack from another tool, or any plugin output needs to land in this workspace as first-class artifacts.
- Preview mappings with `--dry-run --json` before writing. Inspect the `errors` array explicitly: dry-run can return exit `0` with rule-application errors.
- For repeatable re-imports with usable stored hash markers: unchanged hash inputs skip; `--update` permits body replacement when the hash changes. Keep `sources_section.source_hash: true` in mappings used for this workflow.

## When NOT to use

- For a one-off import of a single file into a hand-written artifact. `forgeplan new` is faster and you control every field.
- When the mapping does not yet exist. There is no fallback; an empty `rules:` array is a deserialisation error.
- To mutate artifact bodies outside the mapping's templates. The CLI writes drafts; downstream `forgeplan validate`, `forgeplan reason`, and a human reviewer do the rest.

## What it does

1. Validates `--mapping` and `--source` paths. `--mapping` must exist, be at most 1 MiB, and parse as SPEC-004. `--source` must point at an existing file or directory.
2. Parses the mapping YAML into a `Mapping` (sources, rules, optional guards, optional error policy). Unknown top-level fields are rejected.
3. Walks the source (a file or recursively a directory), and for every file picks the first `SourceSpec` whose pattern matches. Hidden directories (those starting with `.`) are descended into so layouts like `.local/spike-*.md` are reachable.
4. Runs every `Rule` whose `when` selectors match the parsed source. Non-matching rules silently skip the file (recorded in the `skipped` array of the report).
5. Renders the artifact body from the rule's templates (Tera 1.x with a fixed filter whitelist) and a mandatory `## Sources` section. With `sources_section.source_hash: true`, the section includes the marker used for repeat imports.
6. With `--dry-run`: prints the report (text or JSON) and exits without artifact writes.
7. Otherwise matches existing artifacts by kind and case-insensitive title, then compares source hashes. Equal hashes skip; changed hashes skip without `--update`, or replace the body with it. A title change can create a new artifact. Writes are file-first, then indexed in LanceDB.
8. Attempts auto-links using `if_exists` handling. Link failures are not collected into `write_errors`; even `if_exists: error` can print a message without failing the run.

Reuse requires a same-kind/title match **and a usable stored hash marker**.
Without a usable marker, the CLI creates another artifact, even with `--update`.
The hash covers the rule ID and full source text, not the rendered template:
template-only edits leave it unchanged, so the existing body is skipped even
with `--update`. Changing the title can also cause creation rather than reuse.
Keep `sources_section.source_hash: true` explicitly for the examples below.
With `false`, the Sources renderer omits the marker, although body assembly
adds a fallback when the whole body contains no `source_hash:` substring.
Do not treat that fallback as a check that an existing marker is usable.

## Usage

```text
forgeplan ingest [OPTIONS] --mapping <MAPPING> --source <SOURCE>
```

## Options

```text
      --mapping <MAPPING>  Path to mapping YAML file
      --source <SOURCE>    Source path (file or dir to ingest)
      --dry-run            Print drafts without writing
      --update             Allow update of existing artifacts (vs skip on hash match)
      --json               Output as JSON
  -h, --help               Print help
  -V, --version            Print version
```

## Examples

Illustrative examples below use the repository's C4 mapping/source fixtures. Copy the mapping to `mapping.yaml` and make the matching `c4-mini/` source tree available before using these commands. IDs, paths, hashes, counts, and output values are illustrative, not results of a run on your workspace.

### Example 1: Dry-run, text mode

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --dry-run
```

```
  - dry-run: 2 draft(s) would be written
    + note     "Auth Service"  (c4-component-to-note)
    + note     "Database"  (c4-component-to-note)

Done.
```

Exit code `0`. Two drafts would be written; no files changed.

### Example 2: Dry-run, JSON

The JSON report carries `drafts`, `skipped` (selector mismatches), and `errors`. Each draft has `source_path`, `source_hash`, `rule_id`, `kind`, `title`, and the auto-link plan.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --dry-run --json
```

```json
{
  "_next_action": null,
  "drafts": [
    {
      "kind": "note",
      "links": [],
      "rule_id": "c4-component-to-note",
      "source_hash": "8e566a017d00c0a4b27df9174d9e2d7681586cff5b282a10eb1c90799390245d",
      "source_path": "c4-mini/components/auth.md",
      "title": "Auth Service"
    },
    {
      "kind": "note",
      "links": [],
      "rule_id": "c4-component-to-note",
      "source_hash": "c6ffcfc0e79d217f33e855b0b644756d68f523cac9b33d6d44ba55bba9334ad1",
      "source_path": "c4-mini/components/database.md",
      "title": "Database"
    }
  ],
  "errors": [],
  "skipped": [
    { "SelectorMismatch": { "rule_id": "c4-struct-to-note", "source_path": "c4-mini/components/auth.md" } },
    { "SelectorMismatch": { "rule_id": "c4-struct-to-note", "source_path": "c4-mini/components/database.md" } }
  ]
}
```

`_next_action` is `null` because dry-run is terminal. After a real write the value flips to the next PRD-071 hint, e.g. `forgeplan validate <id>`.

### Example 3: Real write

Without `--dry-run`, new drafts are written under the kind's projection directory (for example `.forgeplan/notes/`) and indexed in LanceDB.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Written:
    + NOTE-001 note     "Auth Service"  (c4-component-to-note)
    + NOTE-002 note     "Database"  (c4-component-to-note)

Next: forgeplan validate NOTE-001
```

The illustrative body of `NOTE-001` carries the mandatory `## Sources` section:

```
---
depth: tactical
id: NOTE-001
kind: note
status: draft
title: Auth Service
---

# Auth Service

## Summary

(no purpose section)

## Sources

- c4-mini/components/auth.md:1-35

<!-- source_hash: 8e566a017d00c0a4b27df9174d9e2d7681586cff5b282a10eb1c90799390245d -->
```

The `## Sources` line points back at the source file and line range; the HTML comment holds the content hash used for idempotent re-runs.

### Example 4: Re-running on unchanged sources

For this mapping with `source_hash: true`, an existing same-kind/title artifact and an intact matching marker, re-running on the same source content is a no-op, with or without `--update`.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Idempotent skips:
    ~ NOTE-001 unchanged (hash match)
    ~ NOTE-002 unchanged (hash match)

Done.
```

Exit code `0`.

### Example 5: Source changed, no `--update`

With the same rule ID and title and a usable existing marker, modifying the source file changes the hash. Without `--update`, the run skips overwriting and reports it:

```bash
printf '\n<!-- tweak -->\n' >> c4-mini/components/auth.md
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Idempotent skips:
    ~ NOTE-001 differs but --update not passed
    ~ NOTE-002 unchanged (hash match)

Done.
```

Pass `--update` to replace the differing artifact's body, provided the same-kind/title match has a usable stored marker. It does not force a rewrite for template-only edits or repair a missing marker:

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --update
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Written:
    + NOTE-001 note     "Auth Service"  (c4-component-to-note)

  Idempotent skips:
    ~ NOTE-002 unchanged (hash match)

Next: forgeplan validate NOTE-001
```

Do not rely on `guards.forbid_overwrite_active: true` for protection in 0.38.0:
the engine accepts this field but does not enforce it, and the CLI update
path does not check status. `--update` can replace an active artifact's body.
Review targets and preserve needed edits before allowing updates.

### Example 6: Single-file source

A `--source` path can be a single file. In that mode the engine picks the first `SourceSpec` whose pattern matches (or falls back to the first declared spec) and ingests just that file.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini/components/auth.md --dry-run
```

```
  - dry-run: 1 draft(s) would be written
    + note     "Auth Service"  (c4-component-to-note)

Done.
```

Exit code `0`. Useful when debugging a single rule against a single source.

## Resource limits

Both files are bounded before parsing:

- Mapping YAML: 1 MiB maximum and at most 256 `{`/`[` opener tokens in total
  (a conservative token count, not actual nesting depth).
- Source files: 10 MiB maximum and a string-aware peak bracket-depth cap of 256.

These are parser resource bounds, not a general guarantee against every
malformed source.

## Notes

- Directory enumeration tests both paths relative to `--source` and the discovered path against `sources[].pattern`. Rule selectors then match the parsed source path. A mismatched prefix can produce a successful empty result; use patterns consistent with both discovery and rule selectors rather than assuming every absolute path or `./` prefix fails.
- The MCP surface in 0.38.0 validates inputs and returns a planned view; it does not run the CLI engine or persist drafts.
- Filter whitelist: only the ten filters declared in `forgeplan_core::ingest::types::ALLOWED_FILTERS` are available inside templates. Anything else is a deserialisation error at mapping load time.
- Templates use Tera 1.x syntax. Note the `default` filter requires a named argument (`default(value="...")`), unlike Tera 2.x; the fixture mapping shows the correct form.
- A mapping with zero `sources:` or zero `rules:` is rejected at load. Every rule must declare a non-empty `fields:` map.
- The `## Sources` section is mandatory on every artifact (per ADR-009); a rule with `sources_section.include: false` is a deserialisation error, not a runtime fallback.

## Mapping file shape (minimal)

Minimal one-rule excerpt from the fixture family (not the full two-rule mapping used for the illustrative counts above):

```yaml
schema_version: "1.0"
name: c4-to-forge-test
title: "C4 -> Forge (Wave 4 test fixture)"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge

sources:
  - pattern: "c4-mini/**/*.md"
    type: markdown
    parser: front_matter_plus_sections

rules:
  - id: c4-component-to-note
    when:
      file_glob: "**/c4-mini/components/*.md"
      front_matter:
        kind: component
    target:
      kind: note
    fields:
      title: "{{ front_matter.name | default(value=\"Unnamed component\") }}"
      summary: "{{ section.purpose | default(value=\"(no purpose section)\") }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
```

`sources_section.format` substitutes the literal placeholders `{path}`, `{line_start}`, and `{line_end}`. It is not evaluated as a Tera template.

## Exit status and JSON after writes

Missing paths, invalid mappings, collection failures, and fatal engine errors
exit `2`. A real write with collected rule/write errors emits its report then
exits `1`; ordinary skips and no matched files succeed. Dry-run reports
rule errors but returns `0`, so CI must inspect `.errors`.

Real-write JSON adds `written`, `skipped_existing`, and `write_errors` to
`drafts`, `skipped`, `errors`, and `_next_action`. A written entry contains
`id`, `kind`, `title`, `rule_id`, and `source_path`. Draft JSON omits body text
and includes links with `target`, `relation`, and `if_exists`. An empty-source
JSON report also includes `written: []`. Early structured errors use
`error` and `_next_action`; unexpected I/O/config errors may use ordinary CLI stderr.

## Related commands

- [`forgeplan plugins list`](/docs/cli/plugins-list/) - browse the mapping packs bundled with marketplace plugins.
- [`forgeplan validate`](/docs/cli/validate/) - verify the structural completeness of an ingested draft before activation.
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the derived search index after a bulk ingest.
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the LanceDB index from on-disk markdown after a manual rewrite.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan_ingest` MCP tool](/docs/mcp/forgeplan_ingest/) - dry-run MCP surface for the same pipeline
- [`forgeplan validate`](/docs/cli/validate/) - structural check after a real write
- [Methodology guide](/docs/methodology/overview/)
