---
title: forgeplan ingest
description: "Ingest engine (PRD-066 / SPEC-004) - apply mapping YAML to plugin outputs, generate forge artifacts with file:line source refs"
---

`forgeplan ingest` is the bridge from foreign tool output to native forgeplan artifacts. It takes a mapping YAML (a SPEC-004 recipe) and a source path, walks the source for files matching the mapping's patterns, runs every matching transformation rule, and writes the resulting drafts as notes, problems, or other forge artifacts - each one carrying a hallucination-proof `## Sources` section that points back at the originating file and line range.

This is the CLI side of the Wave-3 ingest pipeline. The MCP surface (`forgeplan_ingest`) is dry-run-only by design: it validates paths, echoes parameters, and emits a planned view pointing back at this command. Full execution, including the write to LanceDB and the markdown projection, happens here.

## When to use

- When a C4 model, an Obsidian vault, an ADR pack from another tool, or any plugin output needs to land in this workspace as first-class artifacts.
- As a Wave-3 dry-run surface: pair `--dry-run --json` with a CI gate to validate that a mapping + source set produce the expected drafts before writing.
- For repeatable re-imports: pass `--update` to refresh existing artifacts when the source content has changed; re-running without `--update` is a safe no-op on hash-equal sources.

## When NOT to use

- For a one-off import of a single file into a hand-written artifact. `forgeplan new` is faster and you control every field.
- When the mapping does not yet exist. There is no fallback; an empty `rules:` array is a deserialisation error.
- To mutate artifact bodies outside the mapping's templates. The CLI writes drafts; downstream `forgeplan validate`, `forgeplan reason`, and a human reviewer do the rest.

## What it does

1. Validates `--mapping` and `--source` paths. `--mapping` must exist, be at most 1 MiB, and parse as SPEC-004. `--source` must point at an existing file or directory.
2. Parses the mapping YAML into a `Mapping` (sources, rules, optional guards, optional error policy). Unknown top-level fields are rejected.
3. Walks the source (a file or recursively a directory), and for every file picks the first `SourceSpec` whose pattern matches. Hidden directories (those starting with `.`) are descended into so layouts like `.local/spike-*.md` are reachable.
4. Runs every `Rule` whose `when` selectors match the parsed source. Non-matching rules silently skip the file (recorded in the `skipped` array of the report).
5. Renders the artifact body from the rule's templates (Tera 1.x with a fixed filter whitelist). Writes a `## Sources` section per artifact using the rule's `sources_section` format and a source content hash for idempotency.
6. With `--dry-run`: prints the report (text or JSON) and exits. No files are written.
7. Without `--dry-run`: idempotently writes each draft to LanceDB and its markdown projection. Existing artifacts with a matching source hash are skipped; existing artifacts whose source hash differs are skipped with a "differs but --update not passed" message unless `--update` is set, in which case they are rewritten.
8. Adds every auto-link declared by the rule (subject to the per-link `if_exists` policy).

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

The examples below use `crates/forgeplan-cli/tests/fixtures/c4-to-forge-test.yaml`, the Phase-5 Wave-4 test mapping, plus the two-component `c4-mini/` source fixture. Both ship with the repository.

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

Without `--dry-run`, drafts are written to `.forgeplan/<kind>/<ID>-<slug>.md` and indexed in LanceDB.

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

The body of `NOTE-001` carries the SPEC-004 hallucination-proof `## Sources` section:

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

Re-running with the same source content and no `--update` is a hash-match no-op.

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

Modify the source file (even by a byte) and the existing artifact's hash no longer matches. Without `--update`, the run refuses to overwrite and tells you so:

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

Pass `--update` to refresh the differing artifact:

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

The `forbid_overwrite_active: true` guard (set in the mapping's `guards:` block) refuses to overwrite an *active* artifact even with `--update`; only drafts can be refreshed this way.

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

- Mapping YAML: 1 MiB max, 256 levels of opener nesting (`{`/`[`).
- Source files: 10 MiB max, 256 levels of opener nesting.

A file that exceeds either cap is rejected before `serde_yaml` ever sees it, so an adversarial 10 MiB source cannot stack-overflow the parser.

## Notes

- `--source` is matched against the mapping's `sources[].pattern` field textually. The path you pass is walked recursively and the discovered files are compared as-is. A leading `./` or an absolute path on `--source` makes the discovery succeed but the glob match fail silently with `no files matched mapping patterns under <path>` and exit code `0` - verify the pattern and the source prefix agree. Single-file `--source` skips directory-mode glob matching and falls back to the first declared spec.
- The MCP surface (`forgeplan_ingest`) is dry-run-only by Wave-3 design: it validates paths, echoes parameters, and emits a planned JSON view that points back at this CLI. Wave-4 wiring will move full execution into the MCP surface.
- Filter whitelist: only the ten filters declared in `forgeplan_core::ingest::types::ALLOWED_FILTERS` are available inside templates. Anything else is a deserialisation error at mapping load time.
- Templates use Tera 1.x syntax. Note the `default` filter requires a named argument (`default(value="...")`), unlike Tera 2.x; the fixture mapping shows the correct form.
- A mapping with zero `sources:` or zero `rules:` is rejected at load. Every rule must declare a non-empty `fields:` map.
- The `## Sources` section is mandatory on every artifact (per ADR-009); a rule with `sources_section.include: false` is a deserialisation error, not a runtime fallback.

## Mapping file shape (minimal)

The two-rule fixture used in the examples:

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

The `format` template uses Tera variables for the path and line range; this is what becomes the `## Sources` line in the rendered artifact.

## Related commands

- [`forgeplan plugins list`](/docs/cli/plugins-list/) - browse the mapping packs bundled with marketplace plugins.
- [`forgeplan validate`](/docs/cli/validate/) - verify the structural completeness of an ingested draft before activation.
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the derived search index after a bulk ingest.
- [`forgeplan scan-import`](/docs/cli/scan-import/) - rebuild the LanceDB index from on-disk markdown after a manual rewrite.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan_ingest` MCP tool](/docs/mcp/forgeplan_ingest/) - dry-run MCP surface for the same pipeline
- [`forgeplan validate`](/docs/cli/validate/) - structural check after a real write
- [Methodology guide](/docs/methodology/overview/)
