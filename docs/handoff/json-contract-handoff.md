# Handoff: `--json` as a real contract (PROB-099 / ADR-024)

**Status:** foundation built and proven, migration not started.
**Deliberately kept out of v0.36.0** — see *Why this is not in the release*.
**Written:** 2026-09-04, against `forgeplan 0.36.0`.

---

## The problem in one paragraph

`--json` is advertised as forgeplan's machine interface. It is not one. It does
not survive a failure — an agent asking for JSON gets prose exactly when it most
needs to know what went wrong. The shapes disagree with each other. The hint
contract holds in 60% of outputs. And there is no shared mechanism, so every new
command diverges again.

Adding the flag to the 47 commands that lack it would have made this worse, not
better: 82 disagreeing formats instead of 35, frozen as a public contract the
moment agents depend on it.

## Measured state (2026-09-04, bin 0.36.0)

| | |
|---|---|
| Commands total | 82 |
| With `--json` | 35 |
| Without | 47 |
| `serde_json::json!` blocks | **118** |
| …carrying `_next_action` | **71** (60%) |
| Shared helpers | **0** |
| `--json` survives an error | **no** |

Reproduce the last one:

```console
$ forgeplan get NOSUCH-999 --json
Error: Artifact 'NOSUCH-999' not found
Fix: forgeplan list
$ echo $?
1
```

Not JSON. Same for `score --json` and `validate --json`.

The shapes also disagree among the 35 that do have it:

```
health --json  → object, has _next_action
score  --json  → object, has _next_action
list   --json  → bare array, no _next_action
```

One parser cannot read all three.

## What is already built and proven

Everything below **works and is tested**. It lives in
`scratchpad/json-work/` (see *Where the code is*), not on any branch.

### `crates/forgeplan-cli/src/output.rs` — the single envelope

291 lines, 7 tests, all passing.

```json
{
  "schema_version": 1,
  "ok": true,
  "data": { … },
  "_next_action": "forgeplan validate PRD-001"
}
```

```json
{
  "schema_version": 1,
  "ok": false,
  "error": { "message": "Artifact 'NOSUCH-999' not found", "kind": "not_found" },
  "_next_action": "forgeplan list"
}
```

Public surface:

- `emit(data, hints)` — success
- `emit_with_alternative(data, hints, alt)` — success with an `Or:` (PROB-095 shape)
- `emit_error(message, kind, next)` — failure; does **not** exit, so `main` and
  commands can both use it
- `classify(&anyhow::Error) -> ErrorKind` — four classes: `not_found`,
  `invalid`, `refused`, `internal`
- `next_action_from_error(&anyhow::Error)` — lifts a `Fix:` line into
  `_next_action` and strips it from the message

### Centralised failure path in `main.rs`

This was the non-negotiable part. `main` returned `anyhow::Result<()>`, so every
error bubbled past all command code and printed as prose. A per-command flag
cannot reach that path — which is exactly why `--json` never survived a failure.

The change: a **global** `--json` on the root `Cli`, `main` split into
`main() -> ExitCode` + `run() -> anyhow::Result<()>`, and the error arm emits the
envelope when JSON was requested.

Verified end to end:

```console
$ forgeplan get NOSUCH-999 --json
{ "schema_version": 1, "ok": false,
  "error": { "message": "Artifact 'NOSUCH-999' not found", "kind": "not_found" },
  "_next_action": "forgeplan list" }
$ echo $?
1
```

Valid JSON **and** a non-zero exit code. Both invariants hold.

## Why this is not in the release

Pulling it out was a deliberate call, not a shortage of time.

**The half-migrated state lies.** With the global flag in place, `--json` is
*accepted* by all 82 commands but *honoured* by 35. `forgeplan blindspots --json`
takes the flag and prints a human table. An agent believes it asked for machine
output and receives prose — which is the same defect class the whole of v0.36.0
was spent removing, reintroduced by the fix for it.

Shipping either half alone is worse than shipping neither:

- flag without migration → silent no-op on 47 commands
- migration without flag → the failure path stays prose

**And the contract is one-way.** Once agents parse this shape, changing it is
breaking for all of them at once. It deserves its own PR that can be reverted
with one button, not a passenger in a release that already carries six closed
defects.

The **artifacts ship anyway** (PROB-099, ADR-024). Recording the decision before
the implementation is the right order, and they cost nothing to carry.

## What remains

| | |
|---|---|
| Files to touch | **36** |
| Emission sites | **59** |
| Per-command `json: bool` args in `main.rs` to remove | **42** |
| Command handlers taking a `json` param | 38 |

### Order of work

1. **Restore the foundation** — `output.rs`, the `lib.rs` module line, the
   `main.rs` global flag + `run()` split. All in `scratchpad/json-work/`.
2. **Migrate the 35 that already emit JSON.** Swap each hand-rolled
   `println!("{}", serde_json::to_string_pretty(&json!({…})))` for
   `output::emit(payload, &hints)`. Read each one — some build nested
   structures and are not a regex substitution.
3. **Remove the 42 per-command `json: bool` args** from `main.rs` and their
   handler parameters. The global flag replaces them; leaving both is how the
   two-styles problem starts again.
4. **Add the 47 that lack it.** By this point it is one line per command.
5. **`list` changes shape** — from a bare array to `{"ok":true,"data":[…]}`.
   This is the loudest break; call it out in the CHANGELOG by name.

### Guard against the half-migrated state

While migrating, an un-migrated command silently ignoring `--json` is the exact
trap described above. Add a temporary gate: a test that runs every command with
`--json` and asserts the output parses as JSON. It will fail loudly for
everything not yet migrated — which is the point. Delete it when the list is
empty, or keep it as the permanent invariant-1 check (better).

`scripts/cli-surface-exercise.sh` already enumerates all 82 commands and is the
natural place to hang this.

## Decisions already made — do not re-litigate without new evidence

ADR-024 records four, each with its cost:

1. **Envelope, not bare payload.** The agent needs a discriminator that does not
   depend on the exit code, because the exit code is invisible through MCP or a
   log. Cost: breaks all 35 current consumers.
2. **Exit code stays non-zero on failure.** The field does not replace it. Shell
   and CI read the code; removing it for tidiness breaks `set -e` for nothing.
3. **`schema_version` from day one.** One extra field against every future shape
   change being breaking. Bought after watching the opposite cost real time
   twice in one day (#351: a changed command string silently broke a name
   derived by slicing it).
4. **Migrate all 118 sites at once.** Two shapes in one binary is worse than one
   bad shape, because the agent cannot tell which it will get.

## Open questions the ADR does *not* settle

- **`watch`** streams events. An envelope per line is NDJSON — a different
  design. ADR-024 explicitly leaves it out; decide separately.
- **`serve`** is already JSON-RPC. `--json` there is meaningless. Confirm it is
  excluded rather than silently accepting the global flag.
- **MCP parity.** The MCP layer has its own serialisation. Whether the envelope
  should apply there too is unexamined — MCP consumers already get structured
  data, so the pressure is lower, but two shapes across CLI and MCP for the same
  operation is its own drift.

## Where the code is

```
scratchpad/json-work/output.rs         291 lines, 7 passing tests
scratchpad/json-work/cli-wiring.patch  71 lines — lib.rs + main.rs diff
```

Both are outside git on purpose: the release branch had to go out clean. Apply
them to a fresh branch off `dev`:

```bash
git checkout -b feat/prd-json-contract origin/dev
cp scratchpad/json-work/output.rs crates/forgeplan-cli/src/
git apply scratchpad/json-work/cli-wiring.patch
cargo test -p forgeplan --lib -- output::
```

## Artifacts

- **PROB-099** — the measured state; every number above is reproducible from it
- **ADR-024** — the four decisions, their costs, what would justify revisiting
- **#374** — the four diagnostic commands without `--json`; absorbed by this work
- **PRD-071** — the hint contract this makes unconditional in JSON
- **PRD-085** — team readiness; machine-readability belongs there
