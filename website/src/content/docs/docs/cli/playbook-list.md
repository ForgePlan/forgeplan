---
title: forgeplan playbook list
description: "List available playbooks (built-in + installed packs) with applicable recommendations based on project signals"
---

`forgeplan playbook list` enumerates every playbook the runtime can see from the current
directory: the workspace's own playbooks, the packs vendored next to the workspace, and the
Claude plugin caches under the home directory. Each file is parsed against the SPEC-003 schema,
and one row per playbook is printed with the columns `NAME`, `TITLE`, `STEPS`, and `SOURCE`.

This is the discovery half of the family. Use it to answer "is `brownfield-code` available
here, and which copy of it wins?" before you point
[`forgeplan playbook show`](/docs/cli/playbook-show/) or
[`forgeplan playbook run`](/docs/cli/playbook-run/) at a target.

## When to use

- Right after `forgeplan init`, to see which canonical packs are reachable from this directory.
- On a fresh clone or a new machine, to confirm the packs you expected are actually on disk.
- Before `playbook run <TARGET>`, to check the `SOURCE` column - if two installations are in
  play, the listed path is the one that will run.
- In a script or an agent bootstrap, with `--json` for a stable machine-readable shape.

## When NOT to use

- To read a playbook's steps - that is [`forgeplan playbook show`](/docs/cli/playbook-show/).
- To check whether a playbook is well-formed - that is
  [`forgeplan playbook validate`](/docs/cli/playbook-validate/).
- To execute anything - `list` never dispatches a step.

## Usage

```text
forgeplan playbook list [OPTIONS]
```

## Options

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## What it does

1. Builds the search-path list: the workspace's `.forgeplan/playbooks/`, the
   `marketplace/playbooks/` directory next to the workspace root, and one
   `playbooks/` directory per entry in `~/.claude/plugins/`.
2. Collects `*.yaml` and `*.yml` files in each root, non-recursively.
3. Reads each file under the runtime's input caps (1 MiB, 256 nesting brackets) and parses it
   against the schema.
4. Skips anything that fails - a corrupt, oversized, or deeply nested file produces a `warn:`
   line on stderr and the listing continues. One bad file never aborts the whole listing.
5. Deduplicates by playbook name, keeping the first occurrence in root order, then sorts the
   surviving entries alphabetically.
6. Prints the table and a `Next:` hint naming `forgeplan playbook show <first-entry>`.

The exit code is 0 whether or not anything was found - an empty listing is a legitimate answer,
not an error.

## Examples

### An empty workspace

```bash
forgeplan playbook list
```

```
No playbooks found.
  Searched: .forgeplan/playbooks/*.yaml, marketplace/playbooks/*.yaml, ~/.claude/plugins/*/playbooks/*.yaml

Done.
```

The `Searched:` line is the fastest way to diagnose a missing playbook: it names the three roots
verbatim. Note that the first two roots are derived from the workspace found by walking up from
the current directory, so from a directory with no `.forgeplan/` above it only the plugin-cache
root is reachable at all.

### With the canonical packs installed

```bash
forgeplan playbook list
```

```
Available playbooks (5):

  NAME                TITLE                                STEPS  SOURCE
  audit               Multi-agent adversarial code review      4  /private/tmp/agent-playbook/.forgeplan/playbooks/audit.yaml
  brownfield-code     Brownfield code-first onboarding         4  /private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-code.yaml
  brownfield-docs     Brownfield docs-first onboarding         3  /private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-docs.yaml
  greenfield-kickoff  Greenfield project bootstrap             7  /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml
  release             Cut a Forgeplan release                 12  /private/tmp/agent-playbook/.forgeplan/playbooks/release.yaml

Next: forgeplan playbook show audit
```

Rows are alphabetical by `NAME`, and `STEPS` is the length of the playbook's `steps:` list - the
number `forgeplan playbook run` will work through, not a count of anything executed so far.

### Machine-readable output

```bash
forgeplan playbook list --json
```

```json
{
  "_next_action": "forgeplan playbook show audit",
  "playbooks": [
    {
      "name": "audit",
      "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/audit.yaml",
      "steps_count": 4,
      "title": "Multi-agent adversarial code review"
    },
    {
      "name": "brownfield-code",
      "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-code.yaml",
      "steps_count": 4,
      "title": "Brownfield code-first onboarding"
    }
  ]
}
```

The payload is `{"playbooks": [...], "_next_action": <string|null>}`. `_next_action` is `null`
when nothing was discovered; otherwise it names the same follow-up command the text mode prints.
In JSON mode nothing else is written to stdout, so the document pipes straight into `jq`.

### A corrupt file does not break the listing

```
warn: skipping playbook file /private/tmp/agent-playbook/.forgeplan/playbooks/huge.yaml: playbook file exceeds size limit (1100064 bytes > 1048576 bytes)
```

Warnings go to stderr while the table goes to stdout, so `forgeplan playbook list --json | jq`
stays parseable even when a stray file is rejected.

## Notes

- `STEPS` counts entries in the YAML `steps:` array, and a playbook is required to have at least
  one. An empty `steps:` list passes YAML parsing but fails the loader, so the file never reaches
  the table:

  ```
  warn: skipping invalid playbook /path/.forgeplan/playbooks/empty.yaml: playbook has no steps (must have at least one)
  ```
- Setting `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY` to a non-empty value other than `0` skips the
  `~/.claude/plugins/*/playbooks` root entirely. The test suite uses it so a developer's
  installed packs cannot leak into assertions.
- The `--help` summary line mentions recommendations based on project signals. That
  recommendation engine (`build_recommendations`, PRD-067) is invoked from `forgeplan init` and
  writes its hints to stderr - suppressed unless stderr is a terminal or `FORGEPLAN_HINTS=1` is
  set. `playbook list` itself prints the discovery table shown above; to silence hints during
  `init`, set `FORGEPLAN_HINTS=0`.
- There is no `--tag`, `--filter`, or name-prefix flag. The only way one entry can hide another is
  a name collision: discovery deduplicates by playbook name and the first root wins, so a
  workspace-local `.forgeplan/playbooks/foo.yaml` masks a vendored `marketplace/playbooks/foo.yaml`
  with the same `name:`. To see what was masked, read the `SOURCE` column of the surviving row or
  point `show` at the file directly.

## See also

- [`forgeplan playbook`](/docs/cli/playbook/) - the parent command and the family overview
- [`forgeplan playbook show`](/docs/cli/playbook-show/) - read one playbook's structure
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - schema check without execution
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - execute a playbook
- [`forgeplan plugins list`](/docs/cli/plugins-list/) - which plugin packs are installed
