---
title: Constellation — work across projects
description: Register two projects, verify shared artifact search, keep MCP writes, and check automatic embeddings.
sidebar:
  order: 3
---

Constellation searches engineering artifacts from several `.forgeplan/` workspaces together. It does **not** automatically index every source-code file in those repositories. Each project keeps its own canonical Markdown and derived index; the user registry connects them.

:::caution[Unreleased interface]
This guide targets the local `0.39.0+constellation.s1.1` build, not the published `0.38.0`. Select a binary built from the Constellation checkout. For a source build, enable `semantic-search` if you want vectors. The walkthrough is a scoped check, not acceptance of every runtime, platform or migration scenario.
:::

## 1. Check the selected binary

```bash
forgeplan --version
forgeplan constellation --help
forgeplan serve --help
```

The server help must contain `--profile`, `--auto-index` and `--workspace`. Installing the stable release is not a way to obtain an unpublished feature.

For keyword search, no LLM key or model download is needed. For vectors, prepare a complete BGE-M3 cache first. For model-only preparation, use `forgeplan setup --skip-alias --skip-hooks` so setup does not also create a command alias or Git hooks. On a **new** workspace, `forgeplan init -y --with-model` explicitly permits the approximately 2.1 GB download. Plain `init -y` never downloads it. Do not use `init --force` just to obtain a model: it resets the active configuration.

The cache is `FORGEPLAN_MODEL_CACHE` when set, otherwise `~/.forgeplan/models`. `HF_HOME` does not override this tract cache. See [configuration](/docs/getting-started/configuration/) for the fallback paths and prerequisites.

## 2. Create two disposable projects

Run the following in a **separate terminal shell**. The private `HOME` isolates the registry from your real projects. Keep this shell for the later steps; `exit` returns to its parent environment.

```bash
set -eu
MODEL_CACHE="${FORGEPLAN_MODEL_CACHE:-$HOME/.forgeplan/models}"
DEMO=$(mktemp -d "${TMPDIR:-/tmp}/forgeplan-docs.XXXXXX")
export HOME="$DEMO/home"
export FORGEPLAN_MODEL_CACHE="$MODEL_CACHE"
export FORGEPLAN_WORKSPACE=""
mkdir -p "$HOME" "$DEMO/alpha" "$DEMO/beta"

for name in alpha beta; do
  (
    cd "$DEMO/$name"
    git init --quiet
    forgeplan init -y
    if [ ! -f .forgeplan/config.yaml ]; then
      printf '%s\n' 'Refusing artifact writes: init did not create the local workspace. Choose TMPDIR outside any existing workspace.' >&2
      exit 1
    fi
    forgeplan new note "Constellation pilot marker $name"
    printf '# Pilot %s\n\n## Decision\n\nconstellation-pilot-marker belongs to %s.\n\n## Verification\n\nDistinguish projects by their source identity.\n' \
      "$name" "$name" > "$DEMO/$name-body.md"
    forgeplan update NOTE-001 --body @"$DEMO/$name-body.md"
    forgeplan index
  )
done

forgeplan constellation create documentation-demo
forgeplan constellation use documentation-demo
forgeplan constellation add "$DEMO/alpha/.forgeplan" --to documentation-demo
forgeplan constellation add "$DEMO/beta/.forgeplan" --to documentation-demo
forgeplan constellation sources
forgeplan constellation doctor
printf 'DEMO=%s\nMCP_HOME=%s\nMODEL_CACHE=%s\n' "$DEMO" "$HOME" "$FORGEPLAN_MODEL_CACHE"
```

Both new projects have their own `NOTE-001`. Use the IDs returned by `new` if you adapt the example to existing data. Register the **`.forgeplan/` directory**, not the whole checkout.

`doctor` should report both sources as indexed and healthy. This checks the source/index boundary, not vector readiness or relevance. `doctor` and `sources` do not have a `--json` flag in this build.

If `init` reports an ancestor rather than the new project, stop. Workspace discovery walks up the directory tree; do not continue writing to the wrong project.

## 3. Verify discovery and disambiguation

```bash
forgeplan constellation search constellation-pilot-marker --json
forgeplan constellation get NOTE-001 --json
```

Inspect the decoded JSON, not only exit code:

- Search must return the two intended titles with different `source.source_id` values and the expected paths.
- `failures` must be empty; inspect `partial` and `truncated` as well.
- Unqualified get must report `result: "ambiguous"`, not arbitrarily choose one project. Ambiguity can have exit code 0.

Qualify the ID with a `source_id` from that response, for example `NOTE-001@src:<returned-hash>`. A unique path segment such as `@alpha` can also select the corresponding source; do not assume a segment is unique in a real constellation.

For a local exact-file check, use ordinary `get` inside the project:

```bash
(cd "$DEMO/alpha" && forgeplan get NOTE-001 --json)
```

Its `body` contains the **whole canonical Markdown**, including frontmatter. Pinned federation responses have a different body contract; see the [read matrix](/docs/reference/constellation/#read-contracts).

## 4. Keep write-enabled MCP and automatic embeddings

`full` remains the default write-enabled MCP profile. `constellation-read` is an additional connection, not a replacement.

For the semantic part, add this **active** block to alpha's `.forgeplan/config.yaml`, preserving its other settings:

```yaml
embedding:
  model: bge-m3
  chunk_size: 2000
```

All of these are required: a `semantic-search` build, the active embedding configuration, a complete cached model, and a live maintenance process for **alpha**. A one-off `embed` does not save this configuration. `chunk_size` is not the token limit of every section window.

Configure an MCP client with the selected binary. Replace the literal paths below with the printed sandbox paths and the real executable path; MCP does not run a shell to expand `$DEMO` in JSON.

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/absolute/path/to/forgeplan",
      "args": ["serve", "--profile", "full", "--auto-index", "--workspace", "/absolute/path/to/demo/alpha"],
      "env": {
        "HOME": "/absolute/path/to/demo/home",
        "FORGEPLAN_WORKSPACE": "/absolute/path/to/demo/alpha",
        "FORGEPLAN_MODEL_CACHE": "/absolute/path/to/model-cache",
        "HF_HUB_OFFLINE": "1"
      }
    },
    "forgeplan-constellation": {
      "command": "/absolute/path/to/forgeplan",
      "args": ["serve", "--profile", "constellation-read"],
      "env": { "HOME": "/absolute/path/to/demo/home", "FORGEPLAN_WORKSPACE": "/absolute/path/to/demo/alpha" }
    }
  }
}
```

Use `HF_HUB_OFFLINE=1` only after the model cache is complete. The second connection needs the same registry `HOME`. Restart the client so the new process actually runs. `serve` is stdio MCP, not an interactive shell command that reports a completed check on its own.

Bind `FORGEPLAN_WORKSPACE` explicitly as shown: an external client has its own environment. At full-server startup this value wins over `--workspace`; a later request does not move the host. The demo shell exports an empty value rather than unsetting it so the CLI's `.env` loader cannot restore an unrelated binding. The resolver skips an explicitly empty value.

Through the **full** connection, call `forgeplan_update` with alpha explicitly selected:

```json
{
  "id": "NOTE-001",
  "workspace": "/absolute/path/to/demo/alpha",
  "body": "# Pilot alpha\n\n## Decision\n\nconstellation-pilot-marker belongs to alpha. constellation-auto-refresh-marker was written through full MCP.\n\n## Verification\n\nThe background worker must publish usable vectors without a manual embed.\n"
}
```

Then, in the sandbox shell:

```bash
(cd "$DEMO/alpha" && forgeplan search constellation-auto-refresh-marker --semantic --json)
```

Embedding is asynchronous. An initial `No embedding profile is published` is **not** a successful semantic check; wait for maintenance and inspect its stderr if it remains unavailable. The successful response must have `semantic_available: true`, `semantic_scope: "prefix+sections"`, and `eligible_rows == expected_rows > 0`. Check that the intended document/section is relevant. Rows include the artifact prefix and section windows, so their count is not the document count.

Do not insert a manual `embed` between the update and this check: that would no longer test automation. For an explicit initial bootstrap instead, stop the background writer, run `forgeplan embed` with the configured cached model, then restart maintenance. Starting a server alone is not proof that the initial queue has finished.

One full server maintains one selected workspace, not every source in the registry. Beta needs its own maintenance process if you also want its vectors updated automatically.

Start a real-project pilot with one workspace, then measure resource use before adding more. Sharing model files does not guarantee one shared inference engine across separate server processes.

## 5. Read through handles

On the **read** connection, use this sequence. The JSON below contains tool arguments, not raw JSON-RPC transport frames. Replace handle placeholders with values returned by that same server process.

1. `constellation_open`:

   ```json
   { "constellation": "documentation-demo" }
   ```

2. Take `scope_handle` from the response and call `constellation_search`:

   ```json
   { "scope_handle": "<returned scope_handle>", "query": "constellation-pilot-marker" }
   ```

3. Take a source-qualified ID from that result and call `constellation_get`:

   ```json
   { "scope_handle": "<returned scope_handle>", "id": "NOTE-001@src:<source from this response>" }
   ```

4. Close the handle with `constellation_close`:

   ```json
   { "handle": "<returned scope_handle>" }
   ```

The read connection has nine tools and rejects `forgeplan_update` as `tool not found`. It cannot edit source artifacts. If a scope expires or its source version changes, open a new scope rather than silently treating fresh bytes as the old answer.

For a Git snapshot, commit the fixture artifacts first, then open a **new** scope and call `constellation_snapshot` with its handle. Search with the returned `snapshot_handle`. **Take source IDs from the snapshot response again**: Git-pinned identities can differ from live ones. A live qualifier can produce `error: "no_source"` against a snapshot. Snapshots read committed Git content, not later uncommitted edits. Close the snapshot and its scope separately.

For these disposable fixtures only, create commits with an explicit test identity without changing global Git settings:

```bash
for name in alpha beta; do
  git -C "$DEMO/$name" add .forgeplan
  git -C "$DEMO/$name" -c user.name="Eli Rum" \
    -c user.email="documentation-smoke@example.invalid" \
    commit -m "docs: record isolated snapshot fixture"
done
```

## 6. Before using real projects

- Back up the entire `.forgeplan/`, client MCP config and registry outside the workspace. Keep the old binary and preserve new canonical changes if you later roll back.
- Stop old publishers before migration; do not let old and new binaries write the same physical index. Leave one maintenance owner per index.
- Register required sources deliberately. Use `--optional` only for a source whose absence you can accept; do not hide a missing required source with permanent partial mode.
- Choose real questions and expected artifacts before testing. Verify source identity, full content, freshness and semantic coverage; an empty answer is not proof that no decision exists.
- Use a binary matching the selected Constellation source snapshot; do not use an old binary to reopen retired tables. Recorded compatibility coverage does not establish every tool, semantic path or platform. Broader limits remain tracked in [#538](https://github.com/ForgePlan/forgeplan/issues/538).

See [Constellation reference](/docs/reference/constellation/) for diagnostics, snapshot semantics, ContextPack limits and the exact command/tool catalog. This walkthrough does not perform a production migration or establish full S1-G/S2 acceptance.
