---
title: Configuration
description: "Complete reference for .forgeplan/config.yaml - LLM providers, embeddings, storage, estimate engine, FPF trust calculus tuning."
---

This reference describes `.forgeplan/config.yaml` in **Forgeplan 0.38.0**. It covers workspace settings and distinguishes fields accepted by the schema from settings actually consumed by the CLI and MCP server.

All fields have defaults, including the top-level metadata (`version`, `project_name`, `default_depth`, `id_digits`, `created_at`). An omitted `llm:` block means no explicitly configured provider; it does not select a local CLI automatically. Credentials belong in the process environment or `.forgeplan/secrets.env`, not in YAML.

## Workspace Structure

After `forgeplan init -y`, the `.forgeplan/` directory is created next to your code:

```
.forgeplan/
├── config.yaml         ← shareable workspace settings (no key values)
│
├── adrs/               ← git-tracked markdown artifacts (source of truth)
├── rfcs/
├── prds/
├── epics/
├── specs/
├── problems/
├── solutions/
├── evidence/
├── notes/
├── refresh/
├── memory/             ← git-tracked Memory artifacts
├── discovery/          ← git-tracked discovery artifacts
├── secrets.env         ← GITIGNORED - local API keys
│
├── lance/              ← GITIGNORED - derived LanceDB index (rebuildable)
└── .fastembed_cache/   ← GITIGNORED - legacy project-local model cache
```

Per ADR-003, **markdown files are the source of truth**. Rebuild the derived `lance/` index with `forgeplan reindex`; `scan-import` discovers and imports other documents, not a replacement for files-first synchronization. New embedding vectors are a separate `forgeplan embed` operation.

The managed `.gitignore` excludes runtime state, model caches, `secrets.env`, its backups and temporary files. It does **not** exclude `config.yaml`. Existing repositories may have additional ignore rules; keep configuration free of credentials whether or not it is tracked.

:::caution[Reinitialization]
`forgeplan init -y` leaves an existing workspace unchanged. `forgeplan init --force -y` refreshes configuration and missing directories without overwriting artifact bodies; it creates an automatic backup unless `--no-backup` is passed. Back up before a deliberate refresh and review the rewritten configuration. Do not delete `.forgeplan/` just to change a provider.
:::

## Top-Level Keys

```yaml
version: 1
project_name: ForgePlan
default_depth: standard
id_digits: 3
created_at: 2026-03-24
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | `u32` | `1` | Config schema version. Bumped only on breaking schema migrations. |
| `project_name` | `string` | `""` | Human-readable project name. Shown in `forgeplan health`, reports, and export. |
| `default_depth` | `string` | `standard` | Workspace depth metadata. Do not treat it as an override of the routing rules. |
| `id_digits` | `u32` | `3` | Accepted configuration metadata. Released CLI and MCP creation paths hardcode three-digit padding; setting `4` does not change newly allocated IDs or renumber existing ones. |
| `created_at` | `date` | today | `YYYY-MM-DD` when the workspace was initialised. Read-only metadata. |

### Depth values

| Value | Released core interpretation |
|-------|------------------------------|
| `note` | Lightweight Note mode. |
| `tactical` | No required artifact pipeline. |
| `standard` | PRD → RFC. |
| `deep` | PRD → Spec → RFC → ADR. |
| `critical` | Accepted as an alias of `deep`, not a separate runtime pipeline. |

Duration estimates, Epic grouping, ADI and adversarial-review requirements belong to methodology policy. They are not extra actions run by the depth parser or `activate`. Marketplace workflows can impose stricter planning and review gates.

See [Depth Calibration guide](/docs/methodology/routing/) for routing heuristics.

## `llm:` - LLM provider

Used by `generate`, `reason` (ADI), LLM routing, `decompose`, `capture`, and the corresponding MCP tools. Core artifact operations do not need an LLM. Routing can fall back to keywords; use `forgeplan route --level 0 "Describe the task"` to explicitly avoid model calls.

```yaml
llm:
  provider: codex-cli
  model: ""                    # let the stock CLI choose its model
  timeout_seconds: 120
  reason_timeout_seconds: 600
```

This example requires a separately installed `codex` binary and a completed `codex login`. Forgeplan does not install or log in to vendor CLIs for you.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `string` | `openai` | Exact HTTP or local CLI profile name from the tables below. There is no arbitrary command profile. |
| `model` | `string` | `gpt-4o-mini` | HTTP model ID, or CLI `--model` value. Use `""` to choose a CLI's own default; see the local-profile distinction below. |
| `api_key_env` | `string?` | provider-specific | Name of the API-key variable, not the key. An explicit name takes precedence over the provider's default name. |
| `base_url` | `string?` | provider-specific | HTTP base URL. OpenAI-compatible requests append `/chat/completions`; `claude` appends `/messages`. Not used by CLI profiles. |
| `max_tokens` | `u32` | `4096` | HTTP response token limit. Not passed to local CLI profiles. |
| `temperature` | `f32` | `0.7` | Sent to OpenAI-compatible HTTP endpoints. Not sent by the native `claude` HTTP path or local CLI profiles; zero does not guarantee deterministic output. |
| `reason_temperature` | `f32?` | unset | ADI override. If absent, `reason` uses `min(temperature, 0.3)`, not the unchanged general temperature. Transport limitations above still apply. |
| `timeout_seconds` | `u64` | `120` | General HTTP request/subprocess budget in seconds. |
| `reason_timeout_seconds` | `u64?` | unset; effective `600` | Separate ADI budget. If omitted, `reason` uses 600 seconds, not `timeout_seconds`. |

### Provider matrix

| HTTP provider | Default API key env | Default base URL | Requirements |
|---------------|---------------------|------------------|--------------|
| `openai` | `OPENAI_API_KEY` | `https://api.openai.com/v1` | API key; OpenAI-compatible chat completions. |
| `claude` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` | Anthropic API key; native Messages API, not a Claude Code subscription. |
| `gemini` | `GEMINI_API_KEY` | `https://generativelanguage.googleapis.com/v1beta/openai` | API key; Google's OpenAI-compatible endpoint, not Gemini CLI. |
| `ollama` | none | `http://localhost:11434/v1` | Running Ollama service and an installed model. Keyless local HTTP. |
| `custom` | none | falls back to OpenAI URL if omitted | Set `base_url` and `api_key_env` explicitly for your OpenAI-compatible endpoint. CLI commands that require LLM configuration reject a missing HTTP key. |

For example, an HTTP configuration can use `provider: openai`, `model: gpt-5-mini`, and `api_key_env: OPENAI_API_KEY`. Model availability, pricing and quotas are controlled by the vendor; Forgeplan does not maintain a model catalog.

### Local CLI profiles (0.38)

All five profiles are **keyless in Forgeplan**, not free or offline. They reuse the stock CLI's login and vendor plan. They are for **personal/local development only, not production, shared or CI environments**, and remain subject to vendor terms and quotas. Each call is headless; it does not continue your interactive conversation.

| `provider` | Install on safe `PATH` | Complete login separately | Profile behavior |
|------------|------------------------|---------------------------|------------------|
| `claude-code` | `claude` | `claude login` | `--print --output-format json`; reads `result`. |
| `codex-cli` | `codex` | `codex login` | Ephemeral, read-only sandbox; ignores user config and rules; reads the final-response file. |
| `devin-cli` | `devin` | `devin auth login`; inspect `devin auth status` | Sandbox, fixed permission profile, private prompt file, plain-text result. **Unsupported on Windows** because the required stock sandbox mode is unavailable there. |
| `gemini-cli` | `gemini` | Start `gemini` interactively and complete login | Sandbox, default approval mode, JSON `response`. |
| `cursor-agent` | `cursor-agent` | `cursor-agent login`; inspect `cursor-agent status` | Ask mode, enabled sandbox, JSON success `result`. |

The four new profiles (`codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`) treat the historical `gpt-4o-mini` default as unspecified. `claude-code` does not: set `model: ""` explicitly if you want Claude's default. Non-empty CLI model values must contain 1–64 ASCII letters, digits, `.`, `_`, `:` or `-`, and must not start with `-`. `base_url`, `api_key_env`, `max_tokens`, and sampling temperatures do not configure these CLI invocations.

The new profiles run in isolated temporary working directories. Prompts travel through stdin, except Devin's private prompt file; they are not interpolated into shell commands. Stock binaries must resolve outside the active workspace; workspace-owned `node_modules/.bin` wrappers are not accepted. On Unix, group/world-writable binaries or parent directories are rejected.

Credential handling uses a restricted child environment:

- Base variables are `PATH`, `HOME`, and `USER`, with workspace paths filtered out of `PATH`.
- Codex preserves `CODEX_HOME` (or the original `HOME/.codex`) for stock auth/config and receives an isolated temporary `HOME`.
- Gemini and Cursor also receive `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `XDG_RUNTIME_DIR`; Claude receives those on Linux.
- API-key variables and unrelated secrets are not forwarded. Forgeplan does not extract vendor tokens. A shared `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` sentinel blocks nested local-provider generation, including calls that switch profiles.

Do not read these controls as a universal guarantee that a vendor CLI cannot use tools. Forgeplan supplies fixed flags; their enforcement also depends on the installed CLI and its sandbox prerequisites. Missing binaries, expired login, unsupported flags/platforms, timeout, malformed output or empty results fail rather than switching silently to another provider.

### Recommended models (2026)

Choose a model available to your API account or CLI plan. The YAML schema default is `gpt-4o-mini`; it is not a recommendation for every provider. HTTP model IDs are passed through, while local CLI IDs are validated as described above. Use `model: ""` for a vendor CLI default instead of copying an API model name into a different profile.

:::note[Switching providers mid-project]
Changing `llm:` changes future calls, not existing artifacts or ADI records. Review model-generated content on its merits; switching providers does not revalidate it.
:::

### Env overrides (LLM)

| Env var | Overrides |
|---------|-----------|
| `FORGEPLAN_LLM_PROVIDER` | `provider` |
| `FORGEPLAN_LLM_MODEL` | `model` |
| `FORGEPLAN_LLM_BASE_URL` | `base_url` |
| `FORGEPLAN_LLM_MAX_TOKENS` | `max_tokens`, if it parses as `u32` |
| `FORGEPLAN_LLM_API_KEY_ENV` | `api_key_env` (name, not value) |

Priority for these fields is **process env > YAML > field default**. These overrides do not create an omitted `llm:` block for CLI commands that require one, and they do not reset other fields. When switching from an HTTP profile, remove an obsolete `api_key_env` and choose the appropriate model. Timeouts and temperatures have no corresponding released environment override.

Provider selection is explicit. The runtime does not scan installed vendor CLIs or choose among OpenAI, Anthropic and Gemini according to which key exists. `ollama` and the five subscription CLI profiles are never auto-selected. Provider names are not enum-validated: an unknown name follows the generic OpenAI-compatible HTTP path, so check spelling rather than expecting a configuration error.

## `embedding:` - semantic search

Configures the embedding model used for semantic search and the FPF KB vector index. Requires the `semantic-search` feature flag at build time.

> **Only `bge-m3` is currently supported.** The engine computes pooling itself, and pooling differs per model: BGE families pool on CLS, the E5 and MiniLM families pool on the mean. Running a mean-pooled model through CLS pooling does not fail; it returns plausible vectors computed the wrong way. So other names are rejected outright rather than silently mis-served. Widening the list is tracked in PROB-091.
>
> The model downloads on first use, ~2.1 GB once per machine, into the platform cache (`~/Library/Caches/forgeplan/models` on macOS, `~/.cache/forgeplan/models` on Linux). `FORGEPLAN_MODEL_CACHE` overrides this path. The released tract downloader receives this cache explicitly; do not rely on the old fastembed-era `HF_HOME` precedence. `forgeplan setup` can download the model deliberately; `init -y` does not download it unless `--with-model` is added.

```yaml
embedding:
  model: bge-m3
  chunk_size: 2000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `enum` | `bge-m3` | Embedding model name. See table below. |
| `chunk_size` | `usize` | `2000` | Max characters of artefact body included in the embedding text. Larger values give richer embeddings at the cost of cache size and ingestion time. |

### Supported models

| Model | Dim | Languages | When to pick |
|-------|-----|-----------|--------------|
| `bge-m3` | 1024 | Multilingual (100+) | **Default** - best quality, supports Russian + English mixed workspaces. |

:::caution[Model and index are separate]
Only `bge-m3` is supported by the released engine. Other names are rejected. `forgeplan reindex` synchronizes markdown into LanceDB; it does not generate vectors. Run `forgeplan embed` separately when vectors need to be created or refreshed.
:::

### Env overrides (embedding)

`FORGEPLAN_EMBEDDING_MODEL` has a helper on the configuration type, but the released CLI/MCP embedding call sites do not apply it. Set `embedding.model` in YAML; do not rely on this variable as a runtime override.

Released 0.38 binaries include semantic search. A custom build can omit `semantic-search`; keyword search remains available. See the [Search guide](/docs/guides/search-v2/).

## `storage:` - storage backend

```yaml
storage:
  driver: lancedb
```

| Field | Type | Default | Contract in 0.38 |
|-------|------|---------|------------------|
| `driver` | `string` | `lancedb` | Driver-factory setting, not a global CLI/MCP backend switch. |
| `path` | `string?` | workspace directory | Driver-factory workspace-root override. The Lance driver appends `lance/`; this is not a direct DB-directory setting. |

### Supported drivers

The library factory supports `lancedb` (alias `lance`) and `memory` (alias `in_memory`), not `sqlite`. Normal CLI/MCP artifact operations open `LanceStore` directly under the resolved `.forgeplan/lance/`; changing this section does not redirect those operations.

### Env overrides (storage)

`FORGEPLAN_STORAGE_DRIVER` and `FORGEPLAN_STORAGE_PATH` exist in a configuration helper, but the released CLI/MCP store-opening paths do not apply them. They are not supported global runtime overrides.

## `memory:` - decision memory bank

```yaml
memory:
  driver: file
```

The schema accepts `driver: file` (default) or `none` as a string setting, but normal CLI/MCP journal operations build their timeline from artifact records in LanceDB. This field is not wired as a switch that disables the journal. `.forgeplan/memory/` stores Memory artifacts; it is not a Hindsight server configuration.

### Env overrides (memory)

`FORGEPLAN_MEMORY_DRIVER` exists in a configuration helper, but is not applied by released journal call sites. Do not use it to disable the journal.

## `estimate:` - estimate engine

Configures `forgeplan estimate` - the multi-grade estimation model that combines your domain expertise with AI task-type multipliers. Every field is optional; omit the whole section to use defaults.

```yaml
estimate:
  grade_profile:
    backend: middle          # your grade in backend development
    frontend: junior         # your grade in frontend
    devops: senior           # your grade in devops/infra
    ai_ml: principal         # your grade in AI/ML
    default: senior          # fallback for unspecified domains
  grade_multipliers:
    junior: 2.0              # relative to senior (baseline 1.0)
    middle: 1.5
    senior: 1.0
    principal: 0.7
    ai: 0.4                  # retained setting; not used by AI-hour calculation
  ai_task_multipliers:
    pure_coding: 0.10        # modeled fraction of baseline hours, not a measured speedup
    coding_infra: 0.25       # mixed coding + infrastructure
    design_coding: 0.30      # design + implementation
    pure_infra: 0.50         # infrastructure only
    coordination: 1.00       # modeled fraction before review overhead
  review_overhead: 0.30      # 30% added to AI time for human review
  safety_margin: 0.50        # stored setting; no sprint-load warning is wired
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `grade_profile` | `map<string, grade>` | default grade `senior` | Per-domain developer grade. Keys are free-form domain names; optional `default` sets the fallback. |
| `grade_multipliers` | `map<string, f64>` | see below | Time multiplier per grade, relative to a senior baseline of `1.0`. |
| `ai_task_multipliers` | `map<string, f64>` | see below | Modeled fraction of baseline senior hours for each task type, before review overhead. |
| `review_overhead` | `f64` | `0.30` | Fraction added to AI-assisted time to cover human review. `0.30` = 30%. |
| `safety_margin` | `f64` | `0.50` | Accepted configuration value; the released estimate path does not consume it to issue sprint-load warnings. |

### Grades

| Grade | Default multiplier | Meaning |
|-------|-------------------:|---------|
| `junior` | 2.0 | Learning the domain - takes 2x a senior's time. |
| `middle` | 1.5 | Can ship independently - 1.5x a senior. |
| `senior` | 1.0 | Baseline. |
| `principal` | 0.7 | Deep domain expert - 30% faster than senior. |
| `ai` | 0.4 | Accepted grade setting, but not used in AI-hour calculation. AI hours use `ai_task_multipliers` instead, then add review overhead. |

### AI task types

| Task type | Default | Typical work |
|-----------|--------:|--------------|
| `pure_coding` | 0.10 | Writing pure code - functions, tests, refactors. AI excels here. |
| `coding_infra` | 0.25 | Coding mixed with tooling (CI, scripts, configs). |
| `design_coding` | 0.30 | Design decisions + implementation. |
| `pure_infra` | 0.50 | Infra-only work - AI helps but human validates each step. |
| `coordination` | 1.00 | Meetings, reviews and stakeholder alignment; the model assumes no time reduction before review overhead. |

### Worked example

> Illustrative calculation with an **8-hour baseline** and task type `pure_coding`:
>
> ```
> human_senior   = 8h × grade_multipliers.senior   = 8h × 1.0 = 8h
> ai_time        = 8h × ai_task_multipliers.pure_coding = 8h × 0.10 = 0.8h
> with_review    = 0.8h × (1 + review_overhead)    = 0.8h × 1.30 = 1.04h
> ```
>
> Modeled AI estimate: **1.04h**, including review overhead. This is a configured estimate, not a measured productivity guarantee.

See [`forgeplan estimate`](/docs/cli/estimate/) for the CLI reference.

## `fpf:` - FPF Trust Calculus engine

Tunes the [FPF Engine](/docs/methodology/adi/): explore/exploit thresholds, reliability weights, congruence-level penalties, and ADI reasoning caps. Every sub-field has safe defaults - tune only if you have empirical reason to.

```yaml
fpf:
  weights:
    reff: 0.5
    links: 0.3
    freshness: 0.2
  thresholds:
    explore_reff: 0.01
    investigate_reff: 0.5
    exploit_reff: 0.7
    exploit_fgr: 0.6
    explore_fgr: 0.4
  cl_penalties:
    cl0: 0.9
    cl1: 0.4
    cl2: 0.1
    cl3: 0.0
  decay:
    expired_score: 0.1
  adi:
    max_hypotheses: 5
    kb_sections_limit: 5
    temperature_cap: 0.3
    auto_save: true
```

### `fpf.weights` - Reliability component weights

Components of the reliability score inside F-G-R. Values do not need to sum to 1.0, but conventionally do.

| Field | Default | Meaning |
|-------|--------:|---------|
| `reff` | `0.5` | Weight of the R_eff score (evidence quality). |
| `links` | `0.3` | Max bonus awarded for incoming/outgoing typed links. |
| `freshness` | `0.2` | Bonus if the artefact is not stale (`valid_until` in the future). |

### `fpf.thresholds` - explore/exploit cutoffs

Artifact trust/action thresholds used by the hardcoded FPF rules. These are distinct from task-depth routing: `forgeplan route` does not consume `fpf.thresholds`.

| Field | Default | Action triggered |
|-------|--------:|------------------|
| `explore_reff` | `0.01` | R_eff below this -> **EXPLORE** (treat as draft). |
| `investigate_reff` | `0.5` | R_eff below this -> **INVESTIGATE** (needs more evidence). |
| `exploit_reff` | `0.7` | R_eff at or above this -> eligible for **EXPLOIT** (safe to rely on). |
| `exploit_fgr` | `0.6` | F-G-R overall required to confirm EXPLOIT (combined with `exploit_reff`). |
| `explore_fgr` | `0.4` | F-G-R below this -> **EXPLORE** priority 1 (combined with `explore_reff`). |

### `fpf.cl_penalties` - congruence level penalties

Penalty applied to evidence based on how well its context matches the artefact it informs. `CL3` (same context) is penalty-free; `CL0` (opposed context) is heavily discounted.

| Field | Default | Meaning |
|-------|--------:|---------|
| `cl0` | `0.9` | Opposed context - near-zero trust. |
| `cl1` | `0.4` | Different context - significant discount. |
| `cl2` | `0.1` | Similar context - minor discount. |
| `cl3` | `0.0` | Same context - no penalty. |

See [Evidence guide](/docs/methodology/evidence/) for how `congruence_level` is set on an EvidencePack body.

### `fpf.decay` - Evidence decay

| Field | Default | Meaning |
|-------|--------:|---------|
| `expired_score` | `0.1` | Score assigned to evidence past `valid_until`. `0.1` reflects "stale, not absent" - the evidence existed, just needs re-verification. |

### `fpf.adi` - ADI reasoning configuration

These fields are accepted by the FPF schema; they are **not wired into the released `reason` generation path**:

| Field | Type | Schema default | Runtime distinction |
|-------|------|---------------:|---------------------|
| `max_hypotheses` | `u32` | `5` | Does not replace the reasoning prompt's hypothesis instructions. |
| `kb_sections_limit` | `usize` | `5` | `build_fpf_context` currently takes at most three sections. |
| `temperature_cap` | `f32` | `0.3` | `reason` uses `llm.reason_temperature`, or `min(llm.temperature, 0.3)`. |
| `auto_save` | `bool` | `true` | Does not control CLI persistence; use the `reason --save` flag. |

### `fpf.rules` - Declarative explore-exploit rules (advanced)

Optional list of user-defined explore/exploit rules (FPF Engine Phase 2). When empty, built-in `default_rules()` are used. Schema and examples live in the [FPF rules guide](/docs/cli/fpf-rules/).

:::caution[FPF validation]
The loader checks finite, non-negative `f64` values in FPF thresholds, weights, CL penalties and decay. Rules must have non-empty conditions and unique names. These checks run when a workspace config is loaded, including by `health`; they do not validate every ADI/estimate value.
:::

## `phase:` and `playbook:`

```yaml
phase:
  enabled: true
playbook:
  allow_shell: false
```

`phase.enabled` defaults to `true` and controls advisory per-artifact phase tracking; it is not the session enforcement lock. `playbook.allow_shell` defaults to `false`. Setting it to `true` opts into shell-command delegation without a per-run `--allow-shell`; do this only for trusted local playbooks, not unreviewed downloaded content. Other playbook execution gates still apply.

## `integrity:` - health & MCP input limits

Thresholds used by `forgeplan health` (duplicate detection, stub detection) and DoS-protection limits enforced by the MCP server on incoming `forgeplan_new` / `forgeplan_update` calls.

```yaml
integrity:
  duplicate_threshold: 0.7
  duplicate_pairs_limit: 10
  stub_marker_threshold: 3
  mcp_max_title_len: 256
  mcp_max_body_len: 1048576      # 1 MiB
  evidence_provenance_gate: warn # block | warn | off
```

| Field | Type | Default | Range | Description |
|-------|------|--------:|-------|-------------|
| `duplicate_threshold` | `f64` | `0.7` | `[0.0, 1.0]` | Jaccard similarity above which two artefacts are flagged as duplicates in `forgeplan health`. |
| `duplicate_pairs_limit` | `usize` | `10` | `[1, 10000]` | Max duplicate pairs shown in health output (pagination). |
| `stub_marker_threshold` | `usize` | `3` | `>= 1` | Minimum number of stub markers (`TODO`, `TBD`, empty headings, etc.) required to flag an artefact body as a stub. |
| `mcp_max_title_len` | `usize` | `256` | `[16, 4096]` | Max artefact title length accepted via MCP. Prevents memory abuse from malicious clients. |
| `mcp_max_body_len` | `usize` | `1048576` | `[1024, 104857600]` | Max artefact body length (bytes) accepted via MCP. Default: 1 MiB. Hard cap: 100 MiB. |
| `evidence_provenance_gate` | `string` | `warn` | `block` \| `warn` \| `off` | Activate-time git-delta provenance gate for code-claiming EvidencePacks (PRD-082). When a pack declares `base_sha` / `result_sha` / `changed_paths`, `forgeplan activate` re-derives the claim against the real git delta instead of trusting the self-report. `block` refuses activation (artefact stays `draft`); `warn` activates and surfaces the discrepancy (CLI on stderr, MCP inside the success payload); `off` skips the check. Packs without provenance fields are unaffected. An unresolvable SHA only ever warns. `activate --force` bypasses. |

:::note[Why these limits exist]
The MCP server communicates over stdio with its host client; stdio is not itself a network listener. These limits bound artifact input from that client, including runaway prompts and buggy agents.
:::

## Environment variables - complete list

The supported LLM field overrides are listed [above](#env-overrides-llm). Credential variables are `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`, or the explicit name under `api_key_env`. `FORGEPLAN_MODEL_CACHE` controls the embedding cache. CLI credential-location variables and the internal recursion sentinel are described under [local profiles](#local-cli-profiles-038); the sentinel is not a user configuration knob.

This is not a claim that every `FORGEPLAN_*` helper is applied by every command. The embedding, storage and memory sections identify the unapplied helper overrides.

### Project-local secrets

For HTTP providers, key lookup checks a non-empty, non-whitespace process variable first, then `.forgeplan/secrets.env` in the selected workspace. The file is read again on each lookup; it is not sourced as shell code. Both `NAME=value` and `export NAME=value` are supported, including quoted literal values. Shell substitutions, variable expansion and escape processing are not performed. The file is capped at 64 KiB; malformed input warns and key lookup fails if there is no usable process variable. Keep Unix permissions at `0600`; broader permissions warn but do not block loading.

`forgeplan migrate-secrets` defaults to a dry run. With `--apply`, it copies the three known API-key variables from the process environment into the local file, backs up an existing file, and does not overwrite existing entries. A differing env/file value is reported as a conflict, not silently replaced. This command does not extract CLI login tokens or migrate an inline YAML key.

Never commit `secrets.env`, `secrets.env.bak-*`, or `.secrets.env.tmp-*`. YAML contains the variable **name**, never its value.

## Critical notes & git safety

:::caution[.forgeplan/ is partially gitignored]
- **Shareable**: artifact markdown and credential-free `config.yaml`.
- **Local/runtime**: `lance/`, model caches, session/phase state, logs, locks, trash, and secrets files covered by the managed ignore block.

Do not assume `config.yaml` is missing after a clone or that ignore rules remove files already tracked by Git. Review the repository's actual tracking rules.
:::

### Before any reinit

For a provider change, edit `llm:` directly. If you need a full configuration refresh, export and back up first:

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
forgeplan init --force -y
```

Review the refreshed config, then run `forgeplan reindex` to synchronize existing artifact markdown if needed. Do not remove the artifact directory as part of ordinary configuration recovery.

### Fresh clone workflow

```bash
# In your cloned repository:
forgeplan init -y                   # no-op if already initialized
$EDITOR .forgeplan/config.yaml      # review or add the llm block
# HTTP: provision the named key privately in env/secrets.env.
# Local CLI: install the stock binary and complete its login separately.
forgeplan reindex                   # sync tracked artifact markdown into LanceDB
forgeplan list                      # inspect indexed artifacts
```

### AI agent mode

Use non-interactive initialization in an agent harness:

```bash
forgeplan init -y
```

`-y` skips prompts and uses defaults; it does not download the model unless `--with-model` is also passed. It does not select or authenticate an LLM provider.

## Troubleshooting

### "API key not found"

**Cause**: the effective HTTP profile's `api_key_env` variable (or its provider default) has no usable value in the process environment or `.forgeplan/secrets.env`.

**Fix**: provision that named variable privately, check literal assignment syntax and file permissions, or explicitly choose a keyless profile with its own prerequisites. Do not print credentials into diagnostics. Changing only `FORGEPLAN_LLM_PROVIDER` can leave an old `api_key_env` or model in place.

`forgeplan health` is not a vendor login or API-key validity probe. No offline check can establish that a future authenticated LLM call will succeed.

### LLM rate limit / 429 errors

**Cause**: the API or CLI vendor rejected a request due to rate limits or quota.

**Fix**: check the vendor plan and retry policy before another call. A smaller HTTP token budget may reduce request cost; it does not bypass rate limits. The core HTTP client produces errors with a status and bounded response excerpt; MCP's caller-facing error hides the HTTP body. Local CLI failures intentionally withhold vendor stderr because it may contain prompts or environment data; use the stock login/status guidance instead.

### Embeddings fail to load / semantic search returns empty

Released 0.38 binaries include semantic search, but the model still needs to be downloaded and loaded. A custom build may omit the feature. Other causes include an unsupported model name, missing/stale artifact vectors, disk/network errors, or a damaged cache.

Set `embedding.model: bge-m3`, inspect the configured cache path and the reported error, and use `forgeplan embed` to create or refresh vectors. This may download ~2.1 GB if the model is absent. `reindex` synchronizes markdown only; `scan-import` does not download or regenerate embeddings. Do not delete a cache before identifying which path the engine actually uses.

Keyword search remains available without semantic search. See the [Search guide](/docs/guides/search-v2/).

### "Invalid config: fpf.thresholds.explore_reff must be finite"

**Cause**: malformed YAML - a numeric field is `NaN`, `Infinity`, or a string that didn't parse as a number.

**Fix**: correct the reported field in `.forgeplan/config.yaml`. The loader validates FPF finite/non-negative `f64` values and integrity ranges; do not assume every estimate/ADI field receives the same validation. Run `forgeplan health` after the correction.

### "integrity.mcp_max_body_len must be in [1024, 104857600]"

**Cause**: MCP body limit set outside the allowed range (1 KiB to 100 MiB).

**Fix**: pick a value inside the range. For most projects, the default (1 MiB) is correct.

### Migration / schema drift after an upgrade

The store runs idempotent schema migrations on open. For file/index drift after an upgrade or Git update, preserve the workspace and use `forgeplan reindex` (or `forgeplan git-sync` for a Git delta). Back up before recovery. If a LanceDB startup error prevents these commands, keep the original directory and the exact error for diagnosis rather than deleting `.forgeplan/` or reinitializing blindly.

## See Also

- [`forgeplan init`](/docs/cli/init/) - workspace bootstrap command
- [`forgeplan estimate`](/docs/cli/estimate/) - estimate engine CLI
- [`forgeplan reason`](/docs/cli/reason/) - ADI reasoning command
- [Evidence guide](/docs/methodology/evidence/) - how `congruence_level` and `valid_until` feed into R_eff
- [Search v2 guide](/docs/guides/search-v2/) - hybrid BM25 + semantic search stack
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/) - artefact state machine and how `integrity:` settings affect health
- [Depth Calibration](/docs/methodology/routing/) - routing rules and depth selection

## Release source

Contracts above are based on the pinned [0.38 configuration structs](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/config/types.rs), [LLM transports](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/mod.rs), [local CLI profiles](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/local_cli.rs), and [secret loader](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/config/secrets.rs). Schema comments alone do not establish that a field is consumed at runtime.
