---
title: forgeplan_playbook_run
description: "Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers."
---

Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers.

**Category**: Other

Every step runs for real: plugin and agent steps shell out to `claude --print`, command steps spawn a subprocess, and skill steps instruct the current session. This tool is annotated destructive and non-idempotent.

## What it does

1. **Security gate.** If `yes` is false and `dry_run` is false, the call is refused before anything else happens. Nothing is resolved, read, or executed.
2. Resolves `target` - a path-like string is canonicalized and confined, a bare name goes through discovery (see [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/)).
3. Reads the file under the 1 MiB size cap and the 256-character nesting cap, then parses it with the SPEC-003 loader.
4. Range-checks `step` against the parsed step count.
5. Resolves the workspace (the explicit `workspace` argument, or the server's current working directory) and attaches the workspace trace to the response.
6. On `dry_run`, enumerates the steps and returns. Otherwise it opens the run journal, builds a `RoutingDispatcher` against the project root, resolves the effective `allow_shell` signal, and executes the steps in dependency order.

## When an agent calls this

- An approved workflow says "run this playbook" and a human or orchestrator has already consented - the `yes: true` flag is that consent, and it is the caller's job to obtain it.
- After [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) or [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) confirmed the structure is sound.
- With `dry_run: true` for a free, side-effect-free plan of what a playbook would do.
- With `step: N` to resume a long playbook that failed partway through.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `target` | `string` | yes | Playbook name, or a path to the YAML file. Same resolution rules as [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/). |
| `dry_run` | `boolean` | no (default `false`) | Plan-only: enumerate steps and invoke nothing. A dry run is exempt from the `yes` gate. |
| `step` | `number` | no | 1-indexed start step. Earlier steps are skipped. Must be within `1..=steps_count`. |
| `yes` | `boolean` | yes for a real run (default `false`) | Consent to execute. Without it a non-dry run is refused. |
| `allow_shell` | `boolean` | no (default `false`) | Separate, stronger opt-in for playbooks containing `delegate_to: command` steps (shell exec, CWE-78 surface). Not implied by `yes`, and it does not imply `yes`. |
| `workspace` | `string` | no | Explicit workspace directory for this call. When set, the server resolves the `.forgeplan/` projection from this path instead of its startup directory - use it when the calling agent runs in a git worktree separate from the server. Absolute or tilde-expanded. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::PlaybookRunParams`_

## The two consent gates

**Gate 1 - `yes`.** A non-dry run without `yes: true` returns this refusal, verbatim (captured live):

```text
Error: playbook run requires `yes: true` confirmation (ADR-009 security gate)

_next_action: forgeplan_playbook_run target="marketplace/playbooks/brownfield-code.yaml" yes=true
```

The hint repeats the same call with consent attached, so the caller can either relay the refusal or supply the flag. This check runs before workspace resolution, so it fires even in a workspace that is not initialised.

**Gate 2 - `allow_shell`.** Playbooks that contain a `delegate_to: command` step spawn a subprocess. Execution is default-deny: such a step runs only when the effective allow-shell signal is true, which is the OR of the `allow_shell` parameter and the workspace opt-in `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. A config read failure is logged and treated as `false` - a broken config file never silently grants shell exec. Set the workspace key only for a repository whose playbooks you trust.

Unlike the CLI, the MCP `allow_shell: true` does **not** imply `yes: true`. Both gates must be satisfied independently.

## Returns

### Dry run

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true",
  "dry_run": true,
  "playbook": "brownfield-code",
  "resolved_via": "cwd",
  "resolved_workspace": "<project-root>/.forgeplan",
  "source_path": "marketplace/playbooks/brownfield-code.yaml",
  "steps": [
    {
      "delegate": "plugin:c4-architecture#c4-code",
      "id": "run-c4-architecture",
      "index": 1,
      "requires": null
    },
    {
      "delegate": "forgeplan_core:ingest",
      "id": "ingest-c4",
      "index": 2,
      "requires": ["run-c4-architecture"]
    },
    {
      "delegate": "skill:forge-history-miner (pack: brownfield-code-pack)",
      "id": "run-history-miner",
      "index": 3,
      "requires": ["ingest-c4"]
    },
    {
      "delegate": "forgeplan_core:new",
      "id": "summary-note",
      "index": 4,
      "requires": ["run-history-miner"]
    }
  ]
}
```

`delegate` is a compact label: `plugin:<name>#<target>`, `agent:<name>`, `skill:<name> (pack: <pack>)`, `command:<argv joined>`, or `forgeplan_core:<op>`.

### Real run

The `report` object carries the aggregate outcome and per-step detail:

| Field | Type | Description |
|---|---|---|
| `playbook` | `string` | Playbook name. |
| `source_path` | `string` | Redacted path executed. |
| `report.run_id` | `string` | Run identifier shared with every journal entry. |
| `report.success` | `number` | Steps that finished cleanly. |
| `report.failed` | `number` | Steps that errored or returned `success: false`. |
| `report.skipped` | `number` | Steps not executed because an upstream step aborted. |
| `report.per_step` | `array` | Per-step records in execution order. |
| `resolved_via` | `string` | How the workspace was resolved, e.g. `cwd`. |
| `resolved_workspace` | `string` | The `.forgeplan/` directory used for this run. |
| `_next_action` | `string` | `Done.` on a clean run; otherwise the `forgeplan_playbook_show` call for diagnosis. |

A run where `report.failed` or `report.skipped` is greater than zero is not terminal: the hint points back at [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) so the caller inspects the structure before retrying.

## Examples

Plan only. Free, no consent needed, no dispatcher touched:

```json
{ "target": "brownfield-code", "dry_run": true }
```

Execute with consent:

```json
{ "target": "brownfield-code", "yes": true }
```

Resume from step 3:

```json
{ "target": "brownfield-code", "yes": true, "step": 3 }
```

Execute a playbook that contains a `delegate_to: command` step:

```json
{ "target": "release", "yes": true, "allow_shell": true }
```

Run against a workspace other than the server's launch directory:

```json
{
  "target": "greenfield-kickoff",
  "yes": true,
  "workspace": "~/work/other-project"
}
```

## Errors

| Situation | Message | `_next_action` |
|---|---|---|
| `yes` missing on a real run | `playbook run requires yes: true confirmation (ADR-009 security gate)` | The same call with `yes=true`. |
| Target not resolvable | `playbook target ... not resolvable` | List playbooks. |
| File over the size cap or unreadable | `cannot read playbook: ...` | Verify the file is well-formed and below 1 MiB. |
| Loader rejected the YAML | `playbook parse error: <kind>` | Validate to get structured findings. |
| `step` out of range | `--step out of range: requested N, playbook has M step(s)` | Show the playbook to pick a valid step. |
| Journal cannot be opened | `cannot open journal: <io error>` | Check `.forgeplan/journal/` directory permissions. |
| A step failed mid-run | `playbook execution failed: <detail>` | Show the playbook for diagnosis. |

## Safety notes

- **Obtain consent before passing `yes: true`.** The flag is a blanket confirmation, not a capability check. Passing it on behalf of a user who has not approved the run defeats the ADR-009 gate.
- **Never pre-approve `allow_shell` in a workspace you did not author.** It is the workspace-wide default-deny bypass for shell execution.
- **Runs are journaled.** Every step appends to `<project-root>/.forgeplan/journal/playbook-runs.jsonl`, so a crashed run can be inspected and resumed with `step`. The file is append-only and never truncated.
- **Use `dry_run` first on an unfamiliar playbook.** It costs nothing and shows exactly which delegates, in which order, with which dependencies.
- **Write the outcome to artifacts, not to your own notes.** Steps produced by ingestion and creation delegates land in the workspace.

## Related commands

- [`forgeplan playbook run`](/docs/cli/playbook-run/) - the CLI equivalent; `--allow-shell` there does imply `--yes`.
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - inspect the steps this tool will execute.
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - validate before a real run.
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - discover the names this tool accepts.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - structure and prerequisites of a playbook
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - catch structural errors before a run
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - discoverable playbooks
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - the CLI twin
