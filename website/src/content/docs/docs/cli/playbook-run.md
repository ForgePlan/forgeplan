---
title: forgeplan playbook run
description: "Run a playbook - sequential step execution with delegations"
---

`forgeplan playbook run <TARGET>` executes one playbook. The steps are ordered by the dependency
edges declared in each step's `requires:` field and run sequentially, one at a time. A step whose
predecessors did not succeed is skipped rather than run, and a failing step with `on_error: abort`
takes the remainder of the run down with it. Every transition is appended to
`.forgeplan/journal/playbook-runs.jsonl`, so a run is reconstructible after the fact.

Two independent gates stand between you and execution. `--yes` confirms that you meant to run a
playbook at all. `--allow-shell` (or the workspace equivalent) separately authorises
`delegate_to: command` steps, which spawn arbitrary processes. Both exist because a playbook is
data you may have received from someone else; neither is implied by the other.

## What it does

1. **Refuses without confirmation.** If `--yes` is absent and `--dry-run` is not set, the command
   exits 2 with a `Fix:` hint before resolving anything.
2. **Resolves and loads `TARGET`** - a playbook name found by discovery, or a path to a `.yaml`
   file - under the runtime's input caps (1 MiB, 256 nesting brackets).
3. **Validates `--step N`** against the playbook's step count up front, so a bad resume point
   fails before any step has run.
4. **Short-circuits on `--dry-run`.** The plan is printed and the command returns without opening
   the journal or constructing a dispatcher.
5. **Opens the journal and builds the routing dispatcher**, which fans each step out to the
   backend its `delegate_to:` names.
6. **Resolves the effective shell opt-in** as the CLI flag OR the workspace config value, and
   emits a banner on stderr when only the workspace config opened the gate.
7. **Walks the steps in dependency order.** Before each dispatch it re-checks the shell gate; a
   `command:` step without the opt-in is recorded as a step failure, not silently skipped.
8. **Prints the run report** - `run_id`, `success`, `failed`, `skipped` counters plus one
   `[OK]` / `[FAIL]` / `[SKIP]` line per step - and exits 1 if any step failed.

## Usage

```text
forgeplan playbook run [OPTIONS] <TARGET>
```

## Arguments

```text
  <TARGET>  Playbook name or path
```

## Options

```text
      --yes          Confirm execution (required — prevents accidental runs)
      --allow-shell  Allow `Delegation::Command` (shell-exec) steps to run. PROB-053 / PRD-074 §FR-1: default-deny gate for the CWE-78 surface. Implies `--yes`. Alternative: set `[playbook] allow_shell = true` in workspace `config.yaml`
      --dry-run      Print steps without executing
      --step <STEP>  Start from specific step (1-indexed)
      --json         Output as JSON
  -h, --help         Print help
  -V, --version      Print version
```

## The shell gate

A `delegate_to: command` step runs an argv list as a child process - the CWE-78 surface a
playbook materialises if it is authored by someone else. The runtime therefore treats it as
default-deny under PROB-053 / PRD-074 FR-1, and the gate is checked at both ends: once by the
CLI before dispatch, once by the executor per step.

When the gate is closed, the step **fails** with the dedicated message:

```
step `say-hello` uses `delegate_to: command` — pass `--allow-shell` (or set `[playbook] allow_shell = true` in workspace config) to acknowledge arbitrary shell execution
```

This is a failure, not a skip, and it counts towards the `failed` counter - so a run that hits
the gate exits 1 and cannot be mistaken for a clean run by a pipeline.

### Opening the gate for one invocation

Pass `--allow-shell`. Each command step then prints a forensic warning line to stderr before
spawning, and executes:

```bash
forgeplan playbook run demo-shell --yes --allow-shell
```

```
Running playbook: demo-shell (2 steps)
! shell-exec: echo hello-from-playbook
! shell-exec: echo second

Run summary: demo-shell
  run_id:  0868f4bfd100a13d
  success: 2
  failed:  0
  skipped: 0

  [OK] say-hello
  [OK] after

Done.
```

The warning renders the full argv with control characters escaped, so a YAML that smuggles ANSI
escapes into a command cannot forge or disguise the line you are reading. Command steps run with
`cwd` set to the workspace root, a strict environment allow-list, and a 180-second default
timeout.

### Opening the gate for the workspace

The alternative is the workspace config block, which is a checked-in dotfile and therefore a
knife that cuts both ways:

```yaml
playbook:
  allow_shell: true
```

With that in place, `--yes` alone satisfies both gates - the shell opt-in now comes from the file
- and the CLI says so loudly at run start:

```bash
forgeplan playbook run demo-shell --yes
```

```
!! shell-exec: AUTO-APPROVED via [playbook] allow_shell=true in .forgeplan/config.yaml. To force-deny for this run, set the config to false (no CLI override flag exists yet — tracked as PRD-074 follow-up).
Running playbook: demo-shell (2 steps)
! shell-exec: echo hello-from-playbook
! shell-exec: echo second

Run summary: demo-shell
  run_id:  8dfa6c290b4dffcf
  success: 2
  failed:  0
  skipped: 0

  [OK] say-hello
  [OK] after

Done.
```

The workspace block opens the shell gate and nothing else: `--yes` is still mandatory, and a run
without it refuses exactly as before, even in a workspace that has the opt-in set.

There is no CLI flag that *narrows* the config opt-in back to deny for a single run - the banner
tells you when the gate was opened by the file rather than by your command line. Set the config
to `false` instead of papering over it with flags.

### Two gates, not one

`--yes` and `--allow-shell` are independent acknowledgements, and the runtime enforces both in
that order. The generated `--help` text for `--allow-shell` says it implies `--yes`; the runtime
does not, and this is deliberate:

```bash
forgeplan playbook run demo-shell --allow-shell
```

```
Error: playbook run requires --yes confirmation (ADR-009 security gate).
Fix: forgeplan playbook run demo-shell --yes --allow-shell
Note: drop --allow-shell if the playbook contains no `delegate_to: command` steps (PRD-074 §FR-1).
```

Exit code 2. Pass both flags - the hint the refusal prints already contains the pair. The source
records the reason for the split: an earlier build collapsed the two into `yes || allow_shell`,
which made a single toggle stand for two different risk acknowledgements.

Conversely, `--yes` alone does **not** authorise shell steps. It confirms the run, and a command
step without the shell opt-in still fails. The refusal hint appends `--allow-shell`
unconditionally, because the CLI cannot know whether the playbook contains command steps until
it has loaded the YAML.

## Examples

### Read the plan before running it

```bash
forgeplan playbook run greenfield-kickoff --yes --dry-run
```

```
Dry-run: greenfield-kickoff (7 steps)
Source:  /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml

  [1] capture-vision: delegate=forgeplan_core:new
  [2] stack-decision: delegate=forgeplan_core:new
  [3] kickoff-epic: delegate=forgeplan_core:new
  [4] prd-feature-1: delegate=forgeplan_core:new
  [5] prd-feature-2: delegate=forgeplan_core:new
  [6] prd-feature-3: delegate=forgeplan_core:new
  [7] scaffold-docs: delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)

Next: forgeplan playbook run greenfield-kickoff --yes
```

`--dry-run` executes nothing and does not need `--yes` (dropping the flag produces identical
output). The `Next:` line drops `--dry-run`, which is the only difference between reading and
running this command.

### The gate closing on a real command step

With a playbook that contains `command:` steps, `--yes` alone is not enough. The step fails and
its dependents are skipped:

```bash
forgeplan playbook run demo-shell --yes
```

```
Running playbook: demo-shell (2 steps)

Run summary: demo-shell
  run_id:  a99614e19a1ee275
  success: 0
  failed:  1
  skipped: 1

  [FAIL] say-hello
       step `say-hello` uses `delegate_to: command` — pass `--allow-shell` (or set `[playbook] allow_shell = true` in workspace config) to acknowledge arbitrary shell execution
  [SKIP] after
       skipped: prior step failed with on_error: abort

Next: forgeplan playbook show demo-shell
```

Exit code 1. Note the reporting contract: the failure is attributed to the step that needs the
opt-in, the second step is `skipped` rather than failed, and `Next:` points at `show` - the same
follow-up the tool suggests whenever a run was not clean.

### Machine-readable report

```bash
forgeplan playbook run demo-shell --yes --allow-shell --json
```

```json
{
  "_next_action": null,
  "playbook": "demo-shell",
  "report": {
    "failed": 0,
    "per_step": [
      {
        "status": "success",
        "step_id": "say-hello"
      },
      {
        "status": "success",
        "step_id": "after"
      }
    ],
    "run_id": "02bc11f2d5d57138",
    "skipped": 0,
    "success": 2
  }
}
```

`_next_action` is `null` for a clean run and names a follow-up command otherwise. In JSON mode the
human-readable progress lines (`Running playbook:`, the `! shell-exec:` warnings) are suppressed,
so stdout is exactly one JSON document - safe to pipe into `jq`.

### Resume from a later step

```bash
forgeplan playbook run demo-shell --yes --allow-shell --step 2
```

```
Run summary: demo-shell
  run_id:  577420d7dec981a9
  success: 0
  failed:  0
  skipped: 2

  [SKIP] say-hello
       skipped via --step 2
  [SKIP] after
       skipped: predecessor not successful

Next: forgeplan playbook show demo-shell
```

Exit code 0 - a skipped-only run is not a failure, since skipping is a routine outcome of the
dependency policy. But read those two lines carefully before relying on `--step` to resume a
partially-completed playbook: skipped steps are recorded as `skipped`, not as `success`, so any
later step that `requires:` one of them is skipped in turn. On a strictly linear playbook,
resuming at step 2 therefore skips the rest of the run rather than performing it. `--step` is
useful mainly for playbooks whose later steps depend on a *sub-set* of earlier ones.

An out-of-range value is caught before anything executes:

```bash
forgeplan playbook run greenfield-kickoff --yes --step 99
```

```
Error: --step out of range: requested 99, playbook has 7 step(s) (1..=7)
Fix: forgeplan playbook show greenfield-kickoff
```

`--step` is 1-indexed, and `0` is out of range just like `99`.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Every step succeeded, or the run only skipped steps |
| `1` | At least one step failed (including a shell-gate refusal) |
| `2` | The run was refused before execution: missing `--yes`, unresolvable target, unreadable or invalid YAML, or `--step` out of range |

Exit 1 on a partial failure is load-bearing: a pipeline that runs `forgeplan playbook run ...
--yes` in CI must not treat a broken playbook as green. Skipped-only runs stay at 0 because
skips are routine.

## Notes

- **Where runs are recorded.** `.forgeplan/journal/playbook-runs.jsonl`, one JSON object per
  line: `run_start` (with the resolved `allow_shell` and `yes_flag` values and the step count),
  `step_start`, `step_end` (with status, message, and output path), and `run_end` (with the
  final counters). The run ID printed in the report matches the `run_id` field on every entry.
- **Progress versus report.** `Running playbook:` and the `! shell-exec:` warnings go to stderr;
  the report goes to stdout. Redirect stderr to keep only the result.
- **The `Fix:` hint quotes a TARGET that contains spaces**, double-quoting it so the suggested
  command is copy-pasteable.
- **`--json` does not change execution.** It changes rendering only; the gates, the ordering, and
  the exit codes are identical.
- **Delegates are dispatched through a routing layer** that covers plugin, agent, skill, command,
  and `forgeplan_core` steps, so a mixed playbook needs no per-delegate invocation.

## See also

- [`forgeplan playbook`](/docs/cli/playbook/) - the parent command and the family overview
- [`forgeplan playbook show`](/docs/cli/playbook-show/) - read the plan without executing it
- [`forgeplan playbook list`](/docs/cli/playbook-list/) - discover installed playbooks
- [`forgeplan playbook validate`](/docs/cli/playbook-validate/) - schema check before running
- [`forgeplan health`](/docs/cli/health/) - the session-start gate most release playbooks call
