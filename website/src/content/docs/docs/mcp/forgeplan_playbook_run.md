---
title: forgeplan_playbook_run
description: "Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers."
---

Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers.

**Category**: Other

Plugin and agent steps invoke `claude --print`; command steps spawn the configured argv directly (not through an implicit shell); core steps call internal operations. **Skill steps are marker-only in 0.38.0**: they print `[skill-invoke] /<name>` or `/pack/name`, return success and pass through the configured `produces_at`. They do not load a skill, check its installation, wait for an agent, or create the claimed output. A harness would need to interpret that marker separately. This tool is annotated destructive and non-idempotent.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "target": "brownfield-code",
  "dry_run": true
}
```

## What it does

1. **Security gate.** If `yes` is false and `dry_run` is false, the call is refused before anything else happens. Nothing is resolved, read, or executed.
2. Resolves `target` - a path-like string is canonicalized and confined, a bare name goes through discovery (see [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/)).
3. Reads the file under the 1 MiB size cap and the 256-character nesting cap, then parses it with the SPEC-003 loader.
4. Range-checks `step` against the parsed step count.
5. Resolves the execution workspace and attaches its trace to successful responses. **`workspace` selects the execution destination, not target discovery/confinement**: those earlier steps use the server process CWD and plugin roots.
6. On `dry_run`, returns source-order step enumeration. Otherwise opens the journal, builds `RoutingDispatcher` against the selected project root and executes sequentially in topological dependency order. The effective `allow_shell` check occurs when each command step is reached; earlier delegates may already have run.

## When an agent calls this

- An approved workflow says "run this playbook" and a human or orchestrator has already consented - the `yes: true` flag is that consent, and it is the caller's job to obtain it.
- After [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) or [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) confirmed the structure is sound.
- With `dry_run: true` to enumerate steps without dispatching. This still reads/parses YAML and requires successful workspace resolution.
- With `step: N` to deliberately skip earlier positions, after inspecting dependencies and prior side effects. This is not automatic journal-based resume.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `allow_shell` | `boolean` | no | Required ALONGSIDE `yes` when the playbook contains `Delegation::Command` (shell-exec) steps - PRD-074 §FR-1+§FR-6, PROB-053 closure. Workspaces can pre-approve via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Schema default: `false`. |
| `dry_run` | `boolean` | no | Plan-only - no dispatcher invocations, just enumerate steps. Schema default: `false`. |
| `step` | `integer \| null` | no | Optional 1-indexed start step (skip earlier steps). `format`: `uint`. `minimum`: `0`. |
| `target` | `string` | yes | Playbook name or path to a YAML file. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |
| `yes` | `boolean` | no | Required to consent to playbook execution (blanket "are you sure?", ADR-009 / SPEC-003). Without `yes: true` the tool refuses to run. Schema default: `false`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## The two consent gates

**Gate 1 — `yes`.** A real run without `yes: true` returns a plain-text tool error (`isError: true`). Illustrative target:

```text
playbook run requires `yes: true` confirmation (ADR-009 security gate)

_next_action: forgeplan_playbook_run target="marketplace/playbooks/brownfield-code.yaml" yes=true
```

The hint repeats the same call with consent attached, so the caller can either relay the refusal or supply the flag. This check runs before workspace resolution, so it fires even in a workspace that is not initialised.

**Gate 2 - `allow_shell`.** Playbooks that contain a `delegate_to: command` step spawn a subprocess. Execution is default-deny: such a step runs only when the effective allow-shell signal is true, which is the OR of the `allow_shell` parameter and the workspace opt-in `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. A config read failure is logged and treated as `false` - a broken config file never silently grants shell exec. Set the workspace key only for a repository whose playbooks you trust.

Both CLI `--allow-shell` / `--yes` and MCP `allow_shell` / `yes` are **independent**. Shell permission never supplies execution consent. The shell gate applies to command delegates; it does not gate plugin/agent subprocesses.

## Returns

### Dry run

Success is JSON in MCP text content. Values below are illustrative. The dry-run
array follows YAML source order and filters source positions before `step`, not
the topological order used for actual execution.

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
| `report.run_id` | `string` | Fresh 16-character lowercase hex identifier shared with journal entries. |
| `report.success` | `number` | Steps that finished cleanly. |
| `report.failed` | `number` | Steps that errored or returned `success: false`. |
| `report.skipped` | `number` | Steps not attempted because of `step`, a failed/skipped dependency or an abort. |
| `report.per_step` | `array` | Per-step records in execution order. |
| `resolved_via` | `string` | How the workspace was resolved, e.g. `cwd`. |
| `resolved_workspace` | `string` | The `.forgeplan/` directory used for this run. |
| `_next_action` | `string` | `Done.` on a clean run; otherwise the `forgeplan_playbook_show` call for diagnosis. |

A run where `report.failed` or `report.skipped` is greater than zero is not terminal: the hint points back at [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) so the caller inspects the structure before retrying.

Each `report.per_step` entry contains `step_id` and `status` (`success`, `failed`,
`skipped`), with optional `output_path` and `message` omitted when absent.
A report containing failures is still a successful MCP JSON result; inspect counts
and step statuses rather than relying only on `isError`.

Every real call starts a new `run_id`. `step` counts topological positions:
earlier steps are recorded as skipped, and dependants of those skipped steps are
also skipped because their prerequisites did not succeed in this run. The executor
does not restore prior successful states from the journal. With `on_error: abort`,
all remaining steps are skipped; `continue` allows independent steps to proceed,
but dependants of the failed step still skip.

Plugin/agent delegates require a usable `claude` executable, its authenticated
session and the requested agent available to that CLI. Registry detection is not
proof of any of those conditions. Skill marker success is not proof that a skill
ran or produced a file.

## Examples

Plan only. Free, no consent needed, no dispatcher touched:

```json
{ "target": "brownfield-code", "dry_run": true }
```

Execute with consent:

```json
{ "target": "brownfield-code", "yes": true }
```

Request skipping the first two positions (inspect dependency consequences first):

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
| Unrecoverable executor/journal failure | `playbook execution failed: <detail>` | Show the playbook for diagnosis; ordinary step failures are in `report`. |

These refusals use plain-text tool errors with an `_next_action:` line, not JSON
error objects. Workspace write-resolution can instead fail at JSON-RPC level.
Ordinary dispatcher failures are usually recorded in `report.per_step`, whereas
unrecoverable executor/journal failures use the error path.

## Safety notes

- **Obtain consent before passing `yes: true`.** The flag is a blanket confirmation, not a capability check. Passing it on behalf of a user who has not approved the run defeats the ADR-009 gate.
- **Never pre-approve `allow_shell` in a workspace you did not author.** It is the workspace-wide default-deny bypass for shell execution.
- **Runs are journaled** in `<project-root>/.forgeplan/journal/playbook-runs.jsonl`, opened append-only. Use it to inspect prior side effects; it is not an automatic resume checkpoint.
- **Use `dry_run` first on an unfamiliar playbook.** It lists delegates and dependencies without invoking them, but does not check runtime availability, credentials or output files.
- **Write the outcome to artifacts, not to your own notes.** Steps produced by ingestion and creation delegates land in the workspace.

## Related commands

- [`forgeplan playbook run`](/docs/cli/playbook-run/) — CLI equivalent; `--yes` and `--allow-shell` remain independent.
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - inspect the steps this tool will execute.
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - validate before a real run.
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - discover the names this tool accepts.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_playbook_show`](/docs/mcp/forgeplan_playbook_show/) - structure and prerequisites of a playbook
- [`forgeplan_playbook_validate`](/docs/mcp/forgeplan_playbook_validate/) - catch structural errors before a run
- [`forgeplan_playbook_list`](/docs/mcp/forgeplan_playbook_list/) - discoverable playbooks
- [`forgeplan playbook run`](/docs/cli/playbook-run/) - the CLI twin
