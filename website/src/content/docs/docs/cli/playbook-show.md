---
title: forgeplan playbook show
description: "Show playbook structure (steps, delegations, requires)"
---

`forgeplan playbook show <TARGET>` prints the structure of a single playbook without executing
any of it: name, title, description, the file the runtime resolved, the declared `requires:`
prerequisites, and every step with its delegate and its dependency edges.

It is the read-only half of the family, and it is the cheap way to answer "what is this pack
about to do to my repository?" before authorising a run. `TARGET` is either a playbook name
found by discovery or a path to a `.yaml` file, so `show` also works on a playbook you have not
installed anywhere.

## When to use

- Before `forgeplan playbook run` on an unfamiliar pack - read the steps first.
- When a run fails - the step IDs in the run report map one-to-one onto the `[n] <id>` lines
  printed here, so `show` is where you interpret the failure.
- When authoring a playbook - it is the quickest way to see how the runtime read your
  `requires:` and `delegate_to:` blocks, including which fields it recognised.
- In CI or an agent bootstrap, with `--json`, to get the parsed model rather than the rendering.

## When NOT to use

- For execution - that is [`forgeplan playbook run`](/docs/cli/playbook-run/).
- For a schema verdict on a file you just edited, use
  [`forgeplan playbook validate`](/docs/cli/playbook-validate/).
  `show` uses the same loader and can also reject structural errors such as
  duplicate step IDs, cycles, unknown dependencies, or mapping/output inconsistency.
- For discovery across the workspace - that is [`forgeplan playbook list`](/docs/cli/playbook-list/).

## Usage

```text
forgeplan playbook show [OPTIONS] <TARGET>
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

## What it does

1. Resolves `TARGET`. An argument containing `/` or `\`, or ending in `.yaml` / `.yml`, is tried
   as a filesystem path first; anything else is looked up by name across the three discovery
   roots. A miss prints `no playbook named ... and no file at that path` plus a
   `Fix: forgeplan playbook list` hint and exits 2.
2. Reads the file under the input caps — 1 MiB and at most 256 `{` / `[`
   opener tokens in total. This conservative count is not true nesting depth.
   Violations are reported before YAML parsing.
3. Parses the YAML against the SPEC-003 schema. A loader error is printed with the offending
   file path and a runnable remediation command.
4. Prints the header block: `Playbook`, `Title`, `Description`, and the resolved `Source` path.
5. Prints `Requires plugins:` and `Requires skills:` when the playbook declares them, with the
   version range or pack scope in parentheses when present.
6. Prints `Steps (<n>)`, one indented block per step: the 1-indexed position, the step ID, and a
   compact `delegate=` label. Recognised per-step fields are printed under it - `requires`,
   `produces_at`, `mapping`, `fallback_hint`.
7. Prints `Next: forgeplan playbook run <name> --yes --dry-run`, the safe follow-up.

Nothing is dispatched, nothing is written, and no journal entry is created.

## Examples

### Read an installed kickoff playbook before running it

```bash
forgeplan playbook show greenfield-kickoff
```

Illustrative output, assuming a discoverable playbook with the following
steps and metadata. This is not a claim that this pack or its printed
install hint exists in the current public catalog:

```
Playbook: greenfield-kickoff
Title:    Greenfield project bootstrap
Description:
  Bootstrap a new project end-to-end through ForgeplanCore:
    1. Capture the project vision as a Note
    2. Record the initial stack decision as an ADR
    3. Create the kickoff Epic that groups the first PRDs
    4. Stub three feature PRDs hanging off the Epic
    5. Scaffold a docs/ tree (optional skill — gracefully skipped
       with an install hint if forge-scaffolder is missing)

  Every artifact-creating step is a `forgeplan_core: new` op, so this
  playbook runs identically with or without external plugins
  installed. The only soft dependency is the forge-scaffolder skill
  for step 7; its absence does not abort the run (`on_error: continue`).
Source:   /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml

Requires skills:
  - forge-scaffolder (pack: brownfield-docs-pack)

Steps (7):
  [1] capture-vision: delegate=forgeplan_core:new
  [2] stack-decision: delegate=forgeplan_core:new
      requires: capture-vision
  [3] kickoff-epic: delegate=forgeplan_core:new
      requires: stack-decision
  [4] prd-feature-1: delegate=forgeplan_core:new
      requires: kickoff-epic
  [5] prd-feature-2: delegate=forgeplan_core:new
      requires: kickoff-epic
  [6] prd-feature-3: delegate=forgeplan_core:new
      requires: kickoff-epic
  [7] scaffold-docs: delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)
      requires: kickoff-epic
      fallback_hint: claude skill install ForgePlan/forge-scaffolder

Next: forgeplan playbook run greenfield-kickoff --yes --dry-run
```

Steps 4–6 declare a dependency on step 3. `show` displays that structure; it
does not simulate whether execution aborts or continues. A `fallback_hint`
is install guidance, not a guarantee of a soft failure: execution depends
on `on_error` and the delegate result.

### Point at a file that is not installed

```bash
forgeplan playbook show .forgeplan/playbooks/demo-shell.yaml
```

```
Playbook: demo-shell
Title:    Shell-gate demo
Description:
  Minimal two-step playbook used to demonstrate the default-deny shell gate.
Source:   .forgeplan/playbooks/demo-shell.yaml

Steps (2):
  [1] say-hello: delegate=command:echo hello-from-playbook
  [2] after: delegate=command:echo second
      requires: say-hello

Next: forgeplan playbook run demo-shell --yes --dry-run
```

A path target is echoed back exactly as you typed it (`Source` is the relative path here),
while a name target shows the absolute path discovery resolved to. The `command:` prefix in the
delegate label is the signal that this playbook will hit the shell gate on
[`playbook run`](/docs/cli/playbook-run/#the-shell-gate).

`--yes` and `--allow-shell` are independent. Merely printing a `command:`
delegate here does not authorize execution.

### Machine-readable structure

```bash
forgeplan playbook show demo-shell --json
```


Illustrative values for the two-step example above:
```json
{
  "_next_action": "forgeplan playbook run demo-shell --yes --dry-run",
  "playbook": {
    "description": "Minimal two-step playbook used to demonstrate the default-deny shell gate.\n",
    "name": "demo-shell",
    "schema_version": "1.0.0",
    "steps": [
      {
        "delegate_to": {
          "command": [
            "echo",
            "hello-from-playbook"
          ],
          "type": "command"
        },
        "id": "say-hello",
        "on_error": "abort"
      },
      {
        "delegate_to": {
          "command": ["echo", "second"],
          "type": "command"
        },
        "id": "after",
        "requires": ["say-hello"],
        "on_error": "abort"
      }
    ],
    "title": "Shell-gate demo"
  },
  "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/demo-shell.yaml"
}
```

The `playbook` object is the parsed model, so fields the text renderer does not surface -
`schema_version`, `on_error`, `input`, `timeout_seconds`, `budget_usd`, `allowed_tools` —
are visible when set (optional fields are omitted when absent). A field being
accepted by the loader does not mean every delegate enforces it.

### Unknown target

```bash
forgeplan playbook show nope
```

```
Error: no playbook named `nope` and no file at that path
Fix: forgeplan playbook list
```

Exit code 2. The same shape is returned when a file exists but fails the size or nesting cap:

```
warn: skipping playbook file /private/tmp/agent-playbook/.forgeplan/playbooks/huge.yaml: playbook file exceeds size limit (1100064 bytes > 1048576 bytes)
Error: no playbook named `huge` and no file at that path
Fix: forgeplan playbook list
```

Here the `warn:` line comes from discovery (the file is unusable as a named target) and the
`Error:` from resolution. Pointing `show` directly at the oversized path reports the reason on
its own, without the resolution layer in the way.

## Notes

- Exit codes: `0` on success, `2` for every refusal - unresolved target, unreadable file, cap
  violation, or schema error. There is no partial-success mode.
- `show` does not validate step *semantics*. A playbook whose step references a plugin that is
  not installed still prints; resolution of plugin and skill prerequisites is the runtime's job
  at dispatch time.
- The `Description` block is indented as a whole and its own line breaks are preserved, so a
  multi-paragraph description stays readable.
- `requires plugins:` and `requires skills:` sections are omitted entirely when the playbook
  declares no prerequisites - an absent section means "nothing declared", not "nothing loaded".
- `--json` and text mode carry the same `_next_action`, which is always the dry-run command.
- The suggested dry-run uses `pb.name`, even when you loaded an external file
  by path. If that name is not discoverable, run with the original path instead.
  `--yes` is consent to the run; it does not grant shell permission.
  Command delegation has a separate `--allow-shell` opt-in on `playbook run`.

## See also

- [`forgeplan playbook`](/docs/cli/playbook/) - the parent command and the family overview
- [`forgeplan playbook list`](/docs/cli/playbook-list/) - discover playbooks and their source paths
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - schema check without execution
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - execute, dry-run, and the shell gate
- [`forgeplan plugins info`](/docs/cli/plugins-info/) - details on a plugin a playbook requires
