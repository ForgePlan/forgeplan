---
title: forgeplan playbook validate
description: "Validate playbook YAML against SPEC-003 schema"
---

Checks one playbook file against the SPEC-003 schema and reports whether the
runtime can load it. `validate` resolves the target, reads the YAML under a
size and nesting bound, parses it, runs the structural invariants, and stops
there - no step executes, no delegate is dispatched, and the workspace index is
not touched.

Run it before [`forgeplan playbook run`](/docs/cli/playbook-run/). A playbook
that fails here fails at run time too; `validate` only moves the failure to a
place where it costs nothing.

## What it does

1. **Resolves** the `TARGET`. An argument containing a `/` or a `\`, or ending in
   `.yaml` / `.yml`, is used as a filesystem path when that path exists; any
   other argument that names an existing file is used as a path as well.
   Everything else is treated as a playbook name and looked up in the discovery
   roots, first hit wins.
2. **Reads under a bound.** The file must be at most 1 MiB (`1048576` bytes)
   and contain at most 256 `{` or `[` characters. Both checks run before the
   parser, so an oversized or deeply nested file is refused, not parsed.
3. **Parses** the YAML into the SPEC-003 shape. Malformed YAML, missing
   required fields, unknown top-level fields, and unknown `delegate_to:` `type`
   values are all errors.
4. **Checks the structural invariants** - see the matrix below.
5. **Reports.** `OK: <name> (<n> steps)` and exit 0 on success; a headline, the
   offending detail, and a `Fix:` hint on failure.

The name in the success line is the playbook's own `name:` field, not its file
name, and the count is the length of the `steps:` list.

## When to use

- Before the first `forgeplan playbook run <TARGET> --yes` of a session.
- After hand-editing a pack, especially the `requires:` edges - a typo there is
  a run-time abort otherwise.
- In CI, as a cheap gate over `marketplace/playbooks/*.yaml` or your own
  `.forgeplan/playbooks/`.
- While writing a new pack: the error messages name the exact step at fault.

## When NOT to use

- To check whether the delegates a playbook names are actually installed.
  `validate` checks the file, not your machine - that is
  [`forgeplan plugins doctor`](/docs/cli/plugins-doctor/)'s job.
- As a dry run. It answers "is this file loadable", not "what would this do" -
  for that, see [`forgeplan playbook run`](/docs/cli/playbook-run/) with
  `--dry-run`.
- To execute anything. `validate` is the read-only half of the pair.

## Usage

```text
forgeplan playbook validate [OPTIONS] <TARGET>
```

## Arguments

```text
  <TARGET>  Playbook name (e.g. brownfield-code) or path to .yaml file
```

## Options

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## What validate checks

| Condition | Verdict |
|---|---|
| `steps:` empty | error |
| a `requires:` entry names a step ID that does not exist | error |
| a cycle in the `requires:` graph | error |
| `mapping:` set without `produces_at:` | error |
| `schema_version` outside `>=1.0.0, <2.0.0` | error |
| malformed YAML, unknown field, unknown delegate `type` | error |
| file above 1 MiB, or more than 256 `{` / `[` characters | error, refused before parsing |
| `produces_at:` set without `mapping:` | warning only |
| a step whose `delegate_to:` is `type: command` | warning only - the shell gate lives in `run` |

Warning-level conditions do not change the verdict. A playbook containing a
`command:` delegate validates as `OK` and exits 0 - that gate is enforced at
run time, where the step fails unless `--allow-shell` is passed.

## Discovery roots

A bare `TARGET` is a name, and names are looked up in this order:

1. `<workspace>/.forgeplan/playbooks/*.yaml` - workspace-local packs.
2. `<workspace>/marketplace/playbooks/*.yaml` - packs vendored next to the
   workspace. This is how a clone of the Forgeplan repository exposes `audit`,
   `release`, `greenfield-kickoff`, `brownfield-code`, and `brownfield-docs`
   with no install step.
3. `~/.claude/plugins/*/playbooks/*.yaml` - Claude plugin caches.

## Examples

### Example 1: a canonical pack, by name

```bash
forgeplan playbook validate audit
```

```
OK: audit (4 steps)

Done.
```

Exit code 0. The blank line between the verdict and `Done.` is always printed;
the `-` detail bullets appear only when something is wrong.

### Example 2: by path

```bash
forgeplan playbook validate marketplace/playbooks/audit.yaml
```

```
OK: audit (4 steps)

Done.
```

The path form resolves outside a workspace as well, which is what makes this
usable in a CI checkout of a pack repository.

### Example 3: machine-readable

```bash
forgeplan playbook validate audit --json
```

```json
{
  "_next_action": "forgeplan playbook run audit --yes --dry-run",
  "name": "audit",
  "passed": true,
  "source_path": "/private/tmp/fp-demo/marketplace/playbooks/audit.yaml",
  "steps_count": 4,
  "title": "Multi-agent adversarial code review"
}
```

`passed` is the machine-readable verdict, `source_path` is the resolved
absolute file, and `_next_action` is the dry run that should come next.

### Example 4: a cycle in the dependency graph

Given `bad/cycle.yaml`:

```yaml
schema_version: "1.0"
name: cycle-demo
title: Cycle demo
steps:
  - id: a
    delegate_to: { type: agent, name: hello }
    requires: [b]
  - id: b
    delegate_to: { type: agent, name: hello }
    requires: [a]
```

```bash
forgeplan playbook validate bad/cycle.yaml
```

```
Error: cycle detected in step `requires:` graph (bad/cycle.yaml)
  - cycle: a -> b -> a
Fix: forgeplan playbook validate bad/cycle.yaml
```

Exit code 2. The detail line names the cycle in encounter order and closes back
on the step where it started. The same shape is used for an unresolvable
`requires:` reference, with one detail line per bad pair.

### Example 5: unknown target

```bash
forgeplan playbook validate no-such-playbook
```

```
Error: no playbook named `no-such-playbook` and no file at that path
Fix: forgeplan playbook list
```

Exit code 2. A path-shaped argument that does not exist resolves the same way,
so `bad/nope.yaml` produces the same message with the path in the quotes.

### Example 6: oversized input

```bash
forgeplan playbook validate bad/huge.yaml
```

```
Error: playbook bad/huge.yaml is too large (1100111 bytes; limit 1048576 bytes)
Fix: split or trim the playbook below 1048576 bytes, or run a custom build
```

The `Fix:` hint here asks for a different playbook, not a corrected one - the
size bound is a resource guard, not a schema rule.

### Example 7: failure as JSON

```bash
forgeplan playbook validate bad/cycle.yaml --json
```

```json
{
  "_next_action": "forgeplan playbook validate bad/cycle.yaml",
  "details": [
    "cycle: a -> b -> a"
  ],
  "error": "cycle detected in step `requires:` graph",
  "passed": false,
  "source_path": "bad/cycle.yaml"
}
```

`details` is the same list that is printed as `-` bullets in text mode, and can
be empty when the headline says everything (a YAML parse error, for instance).

## Exit codes

| Code | Meaning |
|---|---|
| `0` | the file resolved, read, parsed, and satisfied every invariant |
| `2` | anything else: unresolved target, size or nesting bound, YAML error, or a failed invariant |

Every failure class uses the same code, so parse the text or the `--json`
payload when you need to tell them apart.

## Notes

- `validate` needs no workspace. An absolute path works from any directory.
- Failed-to-parse files are skipped during discovery (with a `warn:` line on
  stderr) by [`forgeplan playbook list`](/docs/cli/playbook-list/), so a
  playbook that is missing from `list` can still be validated by path.
- The check is purely structural. A `delegate_to:` naming a plugin, agent, or
  skill that is not installed is not an error here.
- Nothing is written: no run journal entry, no artifact, no index change.
- `-V` reports the subcommand-scoped version, `forgeplan-playbook-validate 0.37.0`.

## See also

- [`forgeplan playbook`](/docs/cli/playbook/) - the parent command and the five delegate kinds
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - execute, dry-run, and the shell gate
- [`forgeplan playbook show`](/docs/cli/playbook-show/) - read one playbook's steps and delegates
- [`forgeplan playbook list`](/docs/cli/playbook-list/) - discover what is reachable from here
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - the MCP equivalent
