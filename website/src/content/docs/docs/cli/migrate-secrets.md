---
title: forgeplan migrate-secrets
description: "PRD-077 FR-023 Part B - import known API-key env vars (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) from the process environment into `.forgeplan/secrets.env` so keys live project-local instead of in your shell rc. Default is dry-run; pass `--apply` to write"
---

`migrate-secrets` moves API keys out of your global shell rc and into a workspace-local `.forgeplan/secrets.env` file. After running with `--apply`, the keys remain available to every command in this workspace (resolution order is process env first, then the file), and you can safely unset them from `~/.zshrc` or `~/.bashrc`.

The command is dry-run by default. Nothing is written until you pass `--apply`. With `--apply`, it is idempotent: existing entries are kept, only the missing ones are appended, and a timestamped backup of the pre-existing file is written next to it.

## When to use

- When your API keys currently live in your shell rc and you want project-scoped isolation.
- After cloning a workspace where keys are expected to be in `.forgeplan/secrets.env` but are missing locally.
- In onboarding docs, as the first step after `forgeplan init`.

## When NOT to use

- To copy inline `api_key: "..."` values from `config.yaml`. The `LlmConfig` struct has no `api_key` field - only `api_key_env`, which holds the *name* of an env var, not the value. There is nothing inline to migrate.
- As a credentials vault. The file is plaintext, gitignored, and chmod 0600; it is not encrypted at rest.

## What it does

1. Resolves the workspace: `--workspace` may point at the project root or at the `.forgeplan/` directory. Without the flag it walks up from the current working directory. Exits `2` if no workspace is found.
2. Loads the current `.forgeplan/secrets.env` if it exists, into an in-memory map.
3. Inspects the three canonical keys in priority order: `GEMINI_API_KEY`, then `OPENAI_API_KEY`, then `ANTHROPIC_API_KEY`. Each is classified into one of four states:
   - `AbsentInEnv`: not set, or set to an empty/whitespace-only value. Nothing to migrate.
   - `AlreadyInFile`: present in both process env and the file, with matching values. No-op.
   - `Conflict`: present in both with diverging values. File value is kept; env value is **not** persisted. The run exits `1` so the divergence is visible.
   - `WouldAdd`: present in process env but missing from the file. Appended on `--apply`.
4. With `--apply`, writes the new entries atomically (`tempfile + rename`) and chmods the file to `0600` on Unix. A backup named `.forgeplan/secrets.env.bak-<unix_timestamp>` is written if the file already existed.
5. Prints a per-key summary in text mode, or a structured JSON envelope with `--json`.

Key **names** appear in output but key **values** never do. Only the length of the value (e.g. `17 chars`) is reported, so logs and CI captures do not leak credentials.

## Why a project-local file instead of a shell rc

Three reasons, all grounded in the resolution contract:

1. **Per-project isolation.** Keys for work and keys for a side project stay separate. A script run from `/tmp/acme-api` cannot accidentally pick up a `GEMINI_API_KEY` that belongs to a different project.
2. **No commit risk.** `forgeplan init` adds `.forgeplan/secrets.env` and `.forgeplan/secrets.env.bak-*` to the workspace `.gitignore`. Keys stay local.
3. **Order-of-lookup is well defined.** `forgeplan_core::config::secrets::resolve_api_key` checks the process env first (trim-aware, so `export KEY="   "` is treated as unset) and only falls back to the file. A key present in both places is unambiguous: the process env wins.

After a successful migration you can `unset GEMINI_API_KEY` (and the other two) from `~/.zshrc` and source `.forgeplan/secrets.env` per project (directly or via `direnv`).

## Usage

```text
forgeplan migrate-secrets [OPTIONS]
```

## Options

```text
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)
      --apply                  Apply the migration (write `.forgeplan/secrets.env`). Default is dry-run - only lists what would be migrated
      --json                   Emit a JSON summary to stdout instead of human-readable text. Useful for scripting / CI integration
  -h, --help                   Print help
  -V, --version                Print version
```

## Exit codes

- `0` - success. Nothing to migrate, or `--apply` completed cleanly.
- `1` - at least one error occurred (backup write failed, file write failed, or an env-vs-file value conflict was left unresolved).
- `2` - workspace not found (no `.forgeplan/` in the current dir or any parent).

## Examples

### Example 1: Dry-run with one key set

Default behaviour is dry-run. Nothing is written; the command lists what would happen and exits.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets
```

```
forgeplan migrate-secrets - DRY-RUN
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  -> GEMINI_API_KEY: in process env (17 chars) - would add
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Would apply 1 new entries. Re-run with --apply to write.

Next: forgeplan migrate-secrets --apply
```

Exit code `0`. The `Next:` hint follows PRD-071 and tells the agent (or a CI step) exactly what to run next.

### Example 2: Apply the migration

Pass `--apply` to actually append the entry to the file. The file is written atomically and chmod'd to `0600`.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  -> GEMINI_API_KEY: in process env (17 chars) - added
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Applied: 1 new entries appended to secrets.env.
Backup: /private/tmp/acme-api/.forgeplan/secrets.env.bak-1789566786

Verify secrets.env contents are correct, then optionally unset
the migrated vars from your shell rc to enforce project-local isolation.

Next: forgeplan reason <ID>
```

The result on disk:

```bash
ls -l .forgeplan/secrets.env*
# -rw------- 1 user user 689 ... secrets.env
# -rw-r--r-- 1 user user 460 ... secrets.env.bak-1789566786
```

Exit code `0`.

### Example 3: Re-run is a no-op

Running `--apply` again with the same env value is idempotent. The existing entry is kept and no new backup is created.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  v GEMINI_API_KEY: already in secrets.env (values match) - skip
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Nothing to migrate. v

Done.
```

Exit code `0`. Safe to run unconditionally in CI.

### Example 4: Env-vs-file conflict

If the same key is set in both places with a different value, the file wins and the env value is not persisted. The run exits `1` so the divergence is visible - silent no-op was the pre-L1 bug.

```bash
GEMINI_API_KEY=a-rotated-key-value forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  ! GEMINI_API_KEY: env value (19 chars) DIFFERS from secrets.env - file value kept, env value NOT persisted
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Nothing to migrate. v

Done.
```

Exit code `1`. Resolve by editing `.forgeplan/secrets.env` directly, or by unsetting the env var for this project.

### Example 5: JSON for CI

Pass `--json` to get a structured envelope suitable for dashboards or CI gates. The `_next_action` field follows the PRD-071 hint contract.

```bash
GEMINI_API_KEY=a-rotated-key-value forgeplan migrate-secrets --json
```

```json
{
  "_next_action": null,
  "added_count": 0,
  "applied": false,
  "backup_path": null,
  "entries": [
    { "name": "GEMINI_API_KEY", "status": "conflict", "value_len": 19 },
    { "name": "OPENAI_API_KEY", "status": "absent_in_env", "value_len": null },
    { "name": "ANTHROPIC_API_KEY", "status": "absent_in_env", "value_len": null }
  ],
  "secrets_path": "/private/tmp/acme-api/.forgeplan/secrets.env"
}
```

`_next_action` is `null` because the run is terminal (a conflict is not auto-resolved). On a successful dry-run with new entries, `_next_action` is `forgeplan migrate-secrets --apply`. After a successful `--apply`, it is `forgeplan reason <ID>` so an agent can verify the keys resolve.

### Example 6: Workspace not found

Outside a forgeplan workspace the command exits `2` and points at the fix.

```bash
cd /tmp
forgeplan migrate-secrets
```

```
Error: no `.forgeplan/` workspace found in current dir or any parent.
Fix: cd into a forgeplan workspace, or run `forgeplan init` to create one.
```

Exit code `2`.

## File format

`.forgeplan/secrets.env` is a `source`-able shell file. `forgeplan init` writes a template that looks like this (comments are stripped here for brevity):

```bash
# secrets.env - local-only API keys. NEVER commit this file.
# forgeplan reads keys from env vars whose NAMES are declared in config.yaml::llm.api_key_env.
# This file is a convenience: a place to keep your real keys during local dev.
# Copy any line below and uncomment it (and `source .forgeplan/secrets.env` or use direnv).

# export GEMINI_API_KEY=<your-key-here>
# export OPENAI_API_KEY=<your-key-here>
# export ANTHROPIC_API_KEY=<your-key-here>

# Imported from process env by `forgeplan migrate-secrets`.
# Safe to remove these lines from your shell rc once this file
# is sourced (or once forgeplan reads it directly per FR-023).
export GEMINI_API_KEY=<redacted>
```

A future revision may read the file directly without sourcing; until then, run `source .forgeplan/secrets.env` (or use `direnv`) to load the keys into the shell.

## Notes

- The set of keys is fixed: `GEMINI_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, in that order. Adding a new provider requires a code change, not a flag.
- Whitespace-only env values (`export GEMINI_API_KEY="   "`) are treated as unset, matching `resolve_api_key`. This closes the L3-audit-class footgun where a literal `   ` leaked through to the LLM provider as a 3-character key.
- `secrets.env` is chmod `0600` after every successful write. The `.bak-<timestamp>` snapshot is created with the file's pre-existing mode and then rotated on the next run.
- The write is atomic via `tempfile + rename`. A crash mid-write leaves the original file intact.
- The migration is per-workspace, not per-user. Different clones of the same project need their own migration.

## Related commands

- [`forgeplan init`](/docs/cli/init/) - creates the workspace, the template `secrets.env`, and the `.gitignore` entries.
- [`forgeplan migrate`](/docs/cli/migrate/) - schema migration companion (different concern, unrelated file).
- [`forgeplan health`](/docs/cli/health/) - session-start dashboard.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan init`](/docs/cli/init/) - sets up the secrets.env template
- [`forgeplan health`](/docs/cli/health/) - session-start health check
- [Getting started](/docs/getting-started/installation/) - shell environment setup
- [Methodology guide](/docs/methodology/overview/)
