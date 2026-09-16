---
title: forgeplan playbook
description: "Playbook runtime (PRD-065 / SPEC-003) - declarative YAML orchestration with delegations to external plugins"
---

Playbook runtime (PRD-065 / SPEC-003) - declarative YAML orchestration with delegations to external plugins

A **playbook** is a YAML file that lists ordered steps. Each step names exactly one delegate -
a plugin, an agent, a skill, a shell command, or a built-in `forgeplan_core` operation - and the
runtime walks that list, honouring the dependency edges declared in each step's `requires:`
field and recording every transition in a run journal.

Playbooks are the reviewable form of a multi-step methodology. Instead of an agent improvising
the same six moves on every project, the sequence becomes data: readable, diffable, and
line-by-line gateable before anything executes. The canonical packs (`greenfield-kickoff`,
`brownfield-code`, `brownfield-docs`, `audit`, `release`) ship with the repository and are
picked up automatically.

`forgeplan playbook` is the parent command. It carries four subcommands plus `help`.

## What it does

1. **Discovers** every `*.yaml` / `*.yml` under the search roots and parses it against the
   SPEC-003 schema. See [`forgeplan playbook list`](/docs/cli/playbook-list/).
2. **Resolves** a `TARGET` argument - either a playbook name found during discovery, or a path
   to a YAML file. See [`forgeplan playbook show`](/docs/cli/playbook-show/).
3. **Validates** the file against the schema and reports `OK: <name> (<n> steps)`. See
   [`forgeplan playbook validate`](/docs/cli/playbook-validate/).
4. **Orders** the steps by their `requires:` edges and executes them sequentially, skipping any
   step whose predecessors did not succeed. See [`forgeplan playbook run`](/docs/cli/playbook-run/).
5. **Gates** every `command:` delegate behind an explicit shell opt-in before dispatch.
6. **Journals** the run into `.forgeplan/journal/playbook-runs.jsonl` - one entry per run start,
   step start, step end, and run end.

## When to use

- **Repeatable onboarding** - `brownfield-code`, `brownfield-docs`, and `greenfield-kickoff`
  encode the first hour of a project as an executable sequence.
- **Multi-agent reviews** - `audit.yaml` fans a review out across specialist agents.
- **Release mechanics** - `release.yaml` runs the pre-flight gates (`cargo fmt --check`,
  `cargo clippy`, the workspace test suite, `forgeplan health`) in a fixed order.
- **CI and agent bootstraps** - `--json` output plus the process exit code let a pipeline treat
  a playbook as a job.

## When NOT to use

- For a single artifact operation - call `forgeplan new`, `validate`, or `activate` directly.
- For work that needs human judgement *between* steps - the runtime is sequential and
  unattended; there is no pause-and-ask point.
- For a one-off shell sequence - write a shell script. The runtime only pays for itself when the
  sequence is named, reviewed, and re-run.

## Usage

```text
forgeplan playbook <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

```text
  list      List available playbooks (built-in + installed packs) with applicable recommendations based on project signals
  show      Show playbook structure (steps, delegations, requires)
  validate  Validate playbook YAML against SPEC-003 schema
  run       Run a playbook — sequential step execution with delegations
  help      Print this message or the help of the given subcommand(s)
```

## Subcommand reference

| Subcommand | Reference page | What it is for |
|---|---|---|
| `list` | [`forgeplan playbook list`](/docs/cli/playbook-list/) | Enumerate what is installed here and where each file lives |
| `show` | [`forgeplan playbook show`](/docs/cli/playbook-show/) | Read one playbook's steps, delegates, and requirements |
| `validate` | [`forgeplan playbook validate`](/docs/cli/playbook-validate/) | Schema check, no execution |
| `run` | [`forgeplan playbook run`](/docs/cli/playbook-run/) | Execute, dry-run, resume, and the shell gate |
| `help` | - | `forgeplan playbook help <SUBCOMMAND>` prints the same text as `--help` |

The documentation slugs are hyphenated (`/docs/cli/playbook-list/`); on the command line the
subcommand is a separate word (`forgeplan playbook list`).

## Delegate kinds

A step's `delegate_to:` block is tagged with a `type:` field, and SPEC-003 defines exactly five
variants:

| `type` | Fields | What runs |
|---|---|---|
| `plugin` | `name`, `target` | An external plugin command invoked through the Task tool |
| `agent` | `name` | A named agent invoked through the Task tool |
| `skill` | `name`, optional `pack` | A skill loaded into the caller's context |
| `command` | `command` as an argv list | An arbitrary process - **opt-in only**, see below |
| `forgeplan_core` | `target` | A built-in operation: `ingest`, `new`, `validate`, `activate`, or `search` |

`forgeplan playbook show` renders these as compact `delegate=` labels, for example
`delegate=plugin:<name>#<target>`, `delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)`,
`delegate=command:echo hello`, or `delegate=forgeplan_core:new`. Only the typed delegates carry
their own permission boundary; `command` is wide open by construction, which is why it is the
one that needs a separate acknowledgement.

## Discovery and resolution

`list` (enumerate) and `show` / `validate` / `run` (resolve by name) read the same three roots,
in this order:

1. `<workspace>/.forgeplan/playbooks/*.yaml` - workspace-local playbooks.
2. `<workspace>/marketplace/playbooks/*.yaml` - packs vendored next to the workspace. This is how
   a clone of the Forgeplan repository picks up `audit`, `release`, and the onboarding packs
   without any install step.
3. `~/.claude/plugins/*/playbooks/*.yaml` - Claude plugin caches.

First hit wins on a name collision. `list` deduplicates by playbook name and sorts the result
alphabetically.

`TARGET` resolution is separate: an argument that contains `/` or `\`, or that ends in `.yaml` or
`.yml`, is tried as a filesystem path first; anything else is treated as a name and looked up in
the roots above. That is why `forgeplan playbook show ./foo.yaml` works from any directory, while
`forgeplan playbook show foo` only works from inside a workspace (or with a plugin cache that
defines `foo`).

## Safety notes

- **`command:` steps are default-deny.** `forgeplan playbook run` refuses them unless
  `--allow-shell` is passed or the workspace sets `[playbook] allow_shell = true`. The refusal is
  a step failure, not a silent skip.
- **`run` refuses without `--yes`.** This gate is independent of the shell opt-in - see
  [`forgeplan playbook run`](/docs/cli/playbook-run/) for the observed behaviour of each
  combination.
- **Input is capped before parsing.** A playbook file larger than 1 MiB (1048576 bytes), or with
  more than 256 `{` / `[` brackets, is rejected with a `warn:` line during discovery and a
  structured error from `show` / `run`.
- **`--dry-run` never executes anything and does not require `--yes`.** It is the safe way to
  read an unfamiliar pack before authorising it.

## Examples

Bare `forgeplan playbook` prints the subcommand list and exits 2, because a subcommand is
required. The help text goes to stderr on this path (as it does for every usage error), which
matters if you are capturing only stdout:

```bash
forgeplan playbook
```

```
Usage: forgeplan playbook <COMMAND>

Commands:
  list      List available playbooks (built-in + installed packs) with applicable recommendations based on project signals
  show      Show playbook structure (steps, delegations, requires)
  validate  Validate playbook YAML against SPEC-003 schema
  run       Run a playbook — sequential step execution with delegations
  help      Print this message or the help of the given subcommand(s)
```

The three-command shape of a first contact with a pack:

```bash
forgeplan playbook list                                    # what is reachable from here?
forgeplan playbook show greenfield-kickoff                 # what does it actually do?
forgeplan playbook run greenfield-kickoff --yes --dry-run  # what would it cost?
```

Every subcommand also accepts `-V` / `--version`, which reports the subcommand-scoped version
string:

```bash
forgeplan playbook -V
```

```
forgeplan-playbook 0.37.0
```

## See also

- [`forgeplan playbook list`](/docs/cli/playbook-list/) - discover installed playbooks
- [`forgeplan playbook show`](/docs/cli/playbook-show/) - inspect one playbook's structure
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - execute, dry-run, and the shell gate
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - schema check without execution
- [`forgeplan health`](/docs/cli/health/) - the session-start gate most playbooks call first
