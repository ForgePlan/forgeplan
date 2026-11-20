---
title: Configuration
description: "Complete reference for .forgeplan/config.yaml - LLM providers, embeddings, storage, estimate engine, FPF trust calculus tuning."
---

`.forgeplan/config.yaml` configures a workspace. This reference distinguishes
schema fields from settings consumed by current CLI/MCP paths; a field or
environment helper in the schema is not a promise that every command uses it.
The schema supplies defaults for metadata as well as optional sections.

Constellation notes below describe the local `0.39.0+constellation.s1.1`
build, not the published 0.38.0. See the
[guide](/docs/guides/constellation/) and [reference](/docs/reference/constellation/).

## Workspace Structure

After `forgeplan init -y`, the `.forgeplan/` directory is created next to your code:

```
.forgeplan/
├── config.yaml         ← workspace config (GITIGNORED - contains env refs)
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
├── memory/             ← decision journal (git-tracked)
│
└── lance/              ← GITIGNORED - derived index, layout depends on version
```

**Markdown files are canonical**; the index is derived. Use `forgeplan index`
for Constellation or `forgeplan reindex` for the legacy layout, not
`scan-import`, which imports documents.
Keep local configuration and index data out of Git according to your project's
ignore policy. The current model cache is shared outside the project.

:::caution[Repeated initialization and defaults]
`forgeplan init -y` returns without changes when `.forgeplan/` is found,
including in a parent directory. It does not repair a missing config on a
fresh clone. `init -y --force` preserves artifact Markdown and makes an
automatic backup, but activates a new config with defaults. Restore custom
settings from that backup deliberately; do not delete the workspace.
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
| `default_depth` | `string` | `standard` | Schema metadata, not a fallback consumed by the current `route` command. Keyword routing without signals returns Tactical. The methodology distinguishes Tactical, Standard, Deep and Critical; runtime `critical` maps to Deep. |
| `id_digits` | `u32` | `3` | Zero-padding width for artifact IDs (e.g. `PRD-001` vs `PRD-0001`). Change only at workspace creation - existing IDs are not renumbered. |
| `created_at` | `date` | today | `YYYY-MM-DD` when the workspace was initialised. Read-only metadata. |

### Depth values

| Value | When to use |
|-------|-------------|
| `tactical` | Quick fix, reversible in a day. No artefact required. |
| `standard` | Feature 1–3 days, one clear tradeoff. PRD -> RFC pipeline. ADI recommended. |
| `deep` | New module, 1–2 weeks. PRD -> Spec -> RFC -> ADR. ADI mandatory. |
| `critical` | Cross-team subsystem, strategic. Epic -> N artifacts. ADI + adversarial review. |

See [Depth Calibration guide](/docs/methodology/routing/) for routing heuristics.

## `llm:` - LLM Provider

Used by `forgeplan generate`, `forgeplan reason` (ADI), `forgeplan route` (Level 1+), and MCP tools that call an LLM.

```yaml
llm:
  provider: gemini
  model: gemini-3-flash-preview
  api_key_env: GEMINI_API_KEY
  # base_url: https://...        # override for custom endpoints
  # max_tokens: 4096
  # temperature: 0.7
  # reason_temperature: 0.3      # lower temp for structured ADI output
  # timeout_seconds: 120         # ordinary LLM-backed calls
  # reason_timeout_seconds: 600  # forgeplan reason only
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `enum` | `openai` | HTTP: `openai`, `claude`, `gemini`, `ollama`, `custom`. Subscription CLI: `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`. |
| `model` | `string` | `gpt-4o-mini` | HTTP model id. Local CLI ids are limited to `[A-Za-z0-9._:-]` (64 bytes); set `model: ""` to use the CLI default. |
| `api_key_env` | `string?` | provider-specific | HTTP providers only: name of the environment variable holding the API key. |
| `base_url` | `string?` | provider-specific | HTTP providers only: override for self-hosted or proxy endpoints. Useful for Ollama, LiteLLM, or Azure-compatible gateways. |
| `max_tokens` | `u32` | `4096` | Max response tokens. Increase for long ADI reasoning; decrease to save cost. |
| `temperature` | `f32` | `0.7` | Sampling temperature for HTTP generation. Lower values reduce sampling variation; `0.0` does not guarantee deterministic provider output. Local CLI profiles do not use every HTTP setting. |
| `reason_temperature` | `f32?` | - | Override used by `forgeplan reason`. If unset, it uses `min(temperature, 0.3)`, not uncapped `temperature`. |
| `timeout_seconds` | `u64` | `120` | Total timeout for ordinary LLM-backed calls, including local CLI stdin delivery and output drain. |
| `reason_timeout_seconds` | `u64?` | effective `600` | `forgeplan reason`-only override; it replaces `timeout_seconds` for ADI. |

### Provider matrix

| Provider | Binary / transport | Login | Safety mode | Limitation | Production |
|----------|--------------------|-------|-------------|------------|------------|
| `openai` | HTTP | `OPENAI_API_KEY` | API boundary | Billable API | supported |
| `claude` | HTTP | `ANTHROPIC_API_KEY` | API boundary | Billable API | supported |
| `gemini` | HTTP | `GEMINI_API_KEY` | API boundary | Billable API | supported |
| `ollama` | local HTTP | none | local model | Operator-owned server | supported |
| `custom` | HTTP | explicit `api_key_env` | operator-defined | Must set `base_url` | supported |
| `claude-code` | `claude`; prompt via stdin | `claude login` | no tools granted | Inherits the caller working directory | local only |
| `codex-cli` | `codex`; prompt via stdin | `codex login` / `codex login status` | ephemeral read-only sandbox + isolated home | Subscription quota | local only |
| `devin-cli` | `devin`; private prompt file | `devin auth login` / `devin auth status` | sandbox + `auto` in an empty temporary workspace; only read-only tools are auto-approved | No documented strict read-only mode; disabled on Windows | local only |
| `gemini-cli` | `gemini`; prompt via stdin | interactive `gemini` login | sandbox + default approval in an empty temporary workspace | Headless entitlement varies by account/product | local only |
| `cursor-agent` | `cursor-agent`; prompt via stdin | `cursor-agent login` / `cursor-agent status` | `ask` + sandbox in an empty temporary workspace | Agent CLI, not completion API | local only |

Devin Desktop may bundle the agent CLI without adding `devin` to `PATH`
(`/Applications/Devin.app/Contents/Resources/app/extensions/windsurf/devin/bin/devin`
on the verified macOS build). Expose that binary as `devin` in a trusted PATH
directory, then run `devin auth login`; Desktop and CLI logins are separate.

Subscription CLI providers are explicit opt-ins; ForgePlan never auto-detects
them and never reads or copies their login tokens. They are for personal/local
development, not production, shared machines, or CI. Prompts are passed as
stdin/private files, model names are validated, and API-key variables are not
forwarded. The four new profiles (`codex-cli`, `devin-cli`, `gemini-cli`,
`cursor-agent`) run in new temporary workspaces; the existing `claude-code`
provider retains its caller-working-directory behavior while sharing the
bounded subprocess runner and recursion guard. If `model` is omitted for one
of the four new profiles, the historical HTTP default `gpt-4o-mini` is treated
as unspecified; set `model: ""` explicitly for the same result.

### Example model selections (2026)

These are configuration examples, not a measured price/quality ranking or a
guarantee of provider availability. Verify the model ID and entitlement with
your provider. The schema default is `gpt-4o-mini`, not a model in this table.

| Provider | Model | When to use |
|----------|-------|-------------|
| Gemini | `gemini-3-flash-preview` | Example model selection; choose a model available to your account. The config default remains `gpt-4o-mini`. |
| Gemini | `gemini-3-pro` | Deep reasoning, critical decisions, long context. |
| OpenAI | `gpt-5-mini` | Balanced price/quality for generate + route. |
| OpenAI | `gpt-5` | Critical ADI, adversarial review. |
| Anthropic | `claude-haiku-4-5-20251001` | Cheap routing and classification. |
| Anthropic | `claude-sonnet-4-6` | Example for generate + reason, not the configuration default. |
| Anthropic | `claude-opus-4-6` | Critical decisions, long-form reasoning. |

:::note[Switching providers mid-project]
You can switch providers at any time - artefacts are model-agnostic. Existing ADI records, reviews, and generated content remain valid. Only new LLM calls will use the new provider.
:::

### Env overrides (LLM)

The following environment variables override `llm:` fields at runtime without editing `config.yaml`:

| Env var | Overrides |
|---------|-----------|
| `FORGEPLAN_LLM_PROVIDER` | `provider` |
| `FORGEPLAN_LLM_MODEL` | `model` |
| `FORGEPLAN_LLM_BASE_URL` | `base_url` |
| `FORGEPLAN_LLM_MAX_TOKENS` | `max_tokens` |
| `FORGEPLAN_LLM_API_KEY_ENV` | `api_key_env` (name, not value) |

Priority: **env var > config.yaml > default**.

## `embedding:` - Semantic Search

Requires the `semantic-search` build feature. The current tract engine accepts
only `bge-m3` (1024 dimensions); names mentioned by older fastembed releases
are not supported by this implementation.

The model is roughly 2.1 GB. Cache resolution is `FORGEPLAN_MODEL_CACHE` →
`~/.forgeplan/models` → platform/legacy fallback only without a home directory.
`HF_HOME` does not override the tract cache path. `forgeplan setup` deliberately
prepares the model; queries and maintenance use an existing cached snapshot.

```yaml
embedding:
  model: bge-m3
  chunk_size: 2000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `string` | `bge-m3` | Only runtime-supported model name. |
| `chunk_size` | `usize` | `2000` | Body-prefix character budget for the embedding profile; not a total token limit or a limit on canonical Markdown. Constellation also indexes section windows. |

### Automatic embedding prerequisites

On a migrated workspace, background queue draining reads **explicit**
`config.embedding` directly. It requires a supported model, complete cached
snapshot, a published generation, the build feature, and live maintenance.
Omitting `embedding:` leaves pending rows pending. `serve --auto-index` alone
does not enable vectors, and a manual `forgeplan embed` does not save this
configuration.

`EmbeddingConfig::with_env_overrides()` exists for
`FORGEPLAN_EMBEDDING_MODEL`, but the automatic drain does not call it. Do not
use that variable as a substitute for `embedding:`. After changing profile
inputs, follow the [Constellation indexing flow](/docs/guides/constellation/);
do not delete caches or use `scan-import` as a rebuild.

## `storage:` - Storage Backend

```yaml
storage:
  driver: lancedb
  # path: /custom/workspace-root
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `driver` | `string` | `lancedb` | Runtime support depends on the caller; see below. |
| `path` | `string?` | workspace root | Root passed to the storage factory, not a universal override for every index path. |

The storage factory implements `lancedb` (`lance`) and an in-memory driver
(`memory`, `in_memory`). `sqlite` is mentioned in schema comments but is
**not implemented by the factory**. The in-memory factory is not a supported
replacement for persistent Constellation storage. Other command paths open
workspace stores directly; do not assume this section switches all CLI/MCP
operations to another backend or directory.

`StorageConfig::with_env_overrides()` defines `FORGEPLAN_STORAGE_DRIVER` and
`FORGEPLAN_STORAGE_PATH`; current storage opening does not invoke that helper.
They are not universal runtime overrides.

## `memory:` - Schema field

```yaml
memory:
  driver: file
```

The schema accepts a `driver` string (default `file`) and defines a
`FORGEPLAN_MEMORY_DRIVER` helper. This is not a runtime guarantee that
`driver: none` disables the journal: current command paths do not select a
memory backend through this setting. Do not use it as a privacy control.

## `estimate:` - Estimate Engine

Configures `forgeplan estimate` - the multi-grade estimation model that combines your domain expertise with AI task-type multipliers. Every field is optional; omit the whole section to use defaults.

The multipliers below are estimation assumptions, not measured speedups or
limits on what AI can assist with. Calibrate them against your own work.

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
    ai: 0.4                  # schema value; unused by the current AI-hours calculation
  ai_task_multipliers:
    pure_coding: 0.10        # assumed fraction of base Senior hours
    coding_infra: 0.25       # mixed coding + infrastructure
    design_coding: 0.30      # design + implementation
    pure_infra: 0.50         # infrastructure only
    coordination: 1.00       # assumes no time reduction
  review_overhead: 0.30      # 30% added to AI time for human review
  safety_margin: 0.50        # accepted schema default; no current calculation/warning consumer
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `grade_profile` | `map<string, grade>` | - | Per-domain developer grade. Keys are free-form domain names; at minimum define `default`. |
| `grade_multipliers` | `map<string, f64>` | see below | Time multiplier per grade, relative to a senior baseline of `1.0`. |
| `ai_task_multipliers` | `map<string, f64>` | see below | Fraction of human time AI takes for each task type. |
| `review_overhead` | `f64` | `0.30` | Fraction added to AI-assisted time to cover human review. `0.30` = 30%. |
| `safety_margin` | `f64` | `0.50` | Accepted/defaulted schema value, currently unused by the estimate calculation and sprint-warning path. It does not trigger a built-in capacity warning. |

### Grades

| Grade | Default multiplier | Meaning |
|-------|-------------------:|---------|
| `junior` | 2.0 | Learning the domain - takes 2x a senior's time. |
| `middle` | 1.5 | Can ship independently - 1.5x a senior. |
| `senior` | 1.0 | Baseline. |
| `principal` | 0.7 | Assumes 30% less time than the Senior baseline, not 30% greater throughput. |
| `ai` | 0.4 | Retained config value; current AI hours use `ai_task_multipliers` instead, then add review overhead. |

### AI task types

| Task type | Default | Typical work |
|-----------|--------:|--------------|
| `pure_coding` | 0.10 | Writing pure code - functions, tests, refactors. AI excels here. |
| `coding_infra` | 0.25 | Coding mixed with tooling (CI, scripts, configs). |
| `design_coding` | 0.30 | Design decisions + implementation. |
| `pure_infra` | 0.50 | Infra-only work - AI helps but human validates each step. |
| `coordination` | 1.00 | Meetings, reviews, stakeholder alignment; assumes no time reduction. |

### Worked example

> A senior backend engineer estimates a task at **8 hours**. AI handles the task as `pure_coding`:
>
> ```
> raw_time       = 8h × grade_multipliers.senior   = 8h × 1.0 = 8h
> ai_time        = 8h × ai_task_multipliers.pure_coding = 8h × 0.10 = 0.8h
> with_review    = 0.8h × (1 + review_overhead)    = 0.8h × 1.30 = 1.04h
> ```
>
> Final estimate: **~1h** of human-facilitated AI time.

See [`forgeplan estimate`](/docs/cli/estimate/) for the CLI reference.

## `fpf:` - FPF Trust Calculus Engine

The [FPF engine](/docs/methodology/adi/) has explore/exploit thresholds,
reliability weights and a separate trust-model calculation. The schema also
accepts ADI settings that current CLI `reason` does not consume; see below.

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
| `freshness` | `0.2` | Bonus when the artifact is not classified stale; absence of an expiry date is not proof of recent verification. |

### `fpf.thresholds` - Explore/Exploit cutoffs

Explore/exploit thresholds in the FPF model. These do not configure task-depth
routing in [`forgeplan route`](/docs/cli/route/).

| Field | Default | Action triggered |
|-------|--------:|------------------|
| `explore_reff` | `0.01` | R_eff below this -> **EXPLORE** (treat as draft). |
| `investigate_reff` | `0.5` | R_eff below this -> **INVESTIGATE** (needs more evidence). |
| `exploit_reff` | `0.7` | R_eff threshold for the model's **EXPLOIT** recommendation, not a guarantee that the artifact is safe to rely on. |
| `exploit_fgr` | `0.6` | F-G-R overall required to confirm EXPLOIT (combined with `exploit_reff`). |
| `explore_fgr` | `0.4` | F-G-R below this -> **EXPLORE** priority 1 (combined with `explore_reff`). |

### `fpf.cl_penalties` - Congruence Level penalties

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

These settings parameterize the FPF trust model. The current ordinary
`score` path separately uses fixed CL penalties and an expired-evidence
score of `0.1`; changing this block is not a global scoring override.

### `fpf.adi` - ADI reasoning configuration

These are accepted schema fields, not effective controls for the current
CLI `reason` path. It requests 3+ hypotheses in its prompt, uses a fixed KB
retrieval policy, and saves a linked Note only with `--save`.

| Field | Type | Default | Meaning |
|-------|------|--------:|---------|
| `max_hypotheses` | `u32` | `5` | Schema setting; current CLI prompt does not read it. |
| `kb_sections_limit` | `usize` | `5` | Schema setting; current CLI KB retrieval does not read it. |
| `temperature_cap` | `f32` | `0.3` | Schema setting; CLI instead uses `llm.reason_temperature`, or `min(llm.temperature, 0.3)`. |
| `auto_save` | `bool` | `true` | Schema setting; CLI persistence is controlled by `reason --save`. |

### `fpf.rules` - Declarative explore-exploit rules (advanced)

Optional list of user-defined explore/exploit rules (FPF Engine Phase 2). When empty, built-in `default_rules()` are used. Schema and examples live in the [FPF rules guide](/docs/cli/fpf-rules/).

:::caution[FPF validation]
The FPF validator checks finite, non-negative values in thresholds, weights,
CL penalties and decay, plus rule structure and unique rule names. Do not
infer equivalent validation of every schema field or treat an existing-workspace
`init` early return as a config validation pass.
:::

## `integrity:` - Health & MCP Input Limits

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
MCP over stdio communicates through pipes, not a network listener.
Input limits protect against oversized requests from an agent.
:::

## `phase:` - Advisory artifact phases

```yaml
phase:
  enabled: true
```

`enabled` is a boolean, default `true` even when the section is absent. It
controls advisory per-artifact phase tracking in `.forgeplan/state/`.
It is not an artifact status or methodology-session phase setting.

## `playbook:` - Shell execution policy

```yaml
playbook:
  allow_shell: false
```

`allow_shell` is a boolean, default `false`. `true` permits command-delegation
steps without a per-invocation `--allow-shell`; it is not a sandbox.
Keep it false for fetched/marketplace playbooks. Opt in only for reviewed,
trusted local workflows; execution confirmation is a separate gate.

## Environment Variables - Runtime and Helper Scope

| Variable | Section | Effect |
|----------|---------|--------|
| `OPENAI_API_KEY` | llm | OpenAI API key (default for `provider: openai`). |
| `ANTHROPIC_API_KEY` | llm | Anthropic API key (default for `provider: claude`). |
| `GEMINI_API_KEY` | llm | Gemini API key (default for `provider: gemini`). |
| `FORGEPLAN_LLM_PROVIDER` | llm | Override `llm.provider`. |
| `FORGEPLAN_LLM_MODEL` | llm | Override `llm.model`. |
| `FORGEPLAN_LLM_BASE_URL` | llm | Override `llm.base_url`. |
| `FORGEPLAN_LLM_MAX_TOKENS` | llm | Override `llm.max_tokens`. |
| `FORGEPLAN_LLM_API_KEY_ENV` | llm | Override `llm.api_key_env` (name of the env var). |
| `FORGEPLAN_WORKSPACE` | MCP routing | Request default after explicit `workspace`, before server default/CWD. At full-server startup it also wins over `--workspace` in maintenance selection; later requests do not move the running host. |
| `FORGEPLAN_MODEL_CACHE` | embedding cache | Explicit cache root for tract; overrides the home-directory default. |
| `FORGEPLAN_EMBEDDING_MODEL` | schema helper | Helper exists; automatic embedding drain does not apply it. |
| `FORGEPLAN_STORAGE_DRIVER`, `FORGEPLAN_STORAGE_PATH` | schema helpers | Not applied by current storage opening; not global backend switches. |
| `FORGEPLAN_MEMORY_DRIVER` | schema helper | Not a working journal-disable control in current command paths. |

Store only the environment-variable **name** under `api_key_env`, never
the key value. Keep secrets in the process environment or the workspace's
ignored `secrets.env`. Gitignore prevents accidental tracking; it does not
redact a file or make arbitrary configuration safe to share.

## Critical Notes & Git Safety

:::caution[.forgeplan/ is partially gitignored]
- **Tracked**: `adrs/`, `rfcs/`, `prds/`, `epics/`, `specs/`, `problems/`, `solutions/`, `evidence/`, `notes/`, `refresh/`, `memory/` - these are the source of truth.
- **Local**: configuration, secrets and derived indexes; check the repository's ignore policy before committing.

A fresh clone can already contain `.forgeplan/` but lack `config.yaml`.
`init -y` then returns early. Inspect the intended workspace root before
choosing deliberate config refresh with `--force`.
:::

### Deliberately refresh defaults

```bash
forgeplan init -y --force
# Review the automatically reported backup.
# Merge required custom settings into .forgeplan/config.yaml.
forgeplan index  # Constellation; use forgeplan reindex for the legacy layout
```

Artifact Markdown is preserved. Custom config is **not** merged back
automatically. Never delete `.forgeplan/` to force this path.

### Fresh clone workflow

```bash
git clone <repo>
cd <repo>
# If .forgeplan/ is absent:
forgeplan init -y
# If it exists but config.yaml is absent and defaults are wanted:
# forgeplan init -y --force
# Review/restore local configuration before indexing.
forgeplan index  # Constellation; use forgeplan reindex for the legacy layout
forgeplan list
```

### AI agent mode

AI agents (Claude Code, Codex, others) running Forgeplan must always use:

```bash
forgeplan init -y      # NEVER interactive - -y is required
```

Interactive mode can block an agent harness. `-y` skips prompts; it writes
defaults only when initialization actually proceeds, not on an existing-workspace early return.

## Troubleshooting

### "API key not found"

```
error: LLM API key not set - expected env var GEMINI_API_KEY
```

**Cause**: `llm.api_key_env` points to an unset variable, or the `provider` default env var is unset.

**Fix**:
```bash
export GEMINI_API_KEY=your-key          # for current shell
# or
export FORGEPLAN_LLM_API_KEY_ENV=MY_CUSTOM_KEY
export MY_CUSTOM_KEY=your-key
```

Configuration or key presence does not prove provider readiness. Check the
next intended LLM operation and its actual response; a health summary is
not an authenticated generation test.

### LLM rate limit / 429 errors

**Cause**: provider rate limits (Gemini free tier is especially tight).

**Fix**:
1. Lower `llm.max_tokens` to reduce per-request cost.
2. Switch to a cheaper model (`gemini-3-flash-preview`, `gpt-5-mini`, `claude-haiku-4-5-20251001`).
3. Retry according to the provider's backoff guidance. Inspect the reported
   status and diagnostic; errors can be wrapped or sanitized rather than
   returned verbatim.

### Embeddings fail to load / semantic search returns empty

Distinguish these conditions:

- The binary lacks `semantic-search`; `--version` does not report features.
- The complete BGE-M3 snapshot is absent from the resolved model cache.
- The workspace generation has pending, missing or stale embeddings.
- Automatic maintenance is not running, or `embedding:` is absent.

Read the diagnostic from `forgeplan embed` and inspect the resolved cache
root before changing anything. Prepare missing weights with `forgeplan setup`,
then index/embed as described in the [guide](/docs/guides/constellation/).
Do not delete model caches. On a migrated workspace forced semantic search
does not promise automatic keyword fallback.

### "Invalid config: fpf.thresholds.explore_reff must be finite"

**Cause**: malformed YAML - a numeric field is `NaN`, `Infinity`, or a string that didn't parse as a number.

**Fix**: open `.forgeplan/config.yaml` and ensure every numeric field under `fpf:`, `estimate:`, and `integrity:` is a plain decimal. Run `forgeplan health` to revalidate.

### "integrity.mcp_max_body_len must be in [1024, 104857600]"

**Cause**: MCP body limit set outside the allowed range (1 KiB to 100 MiB).

**Fix**: pick a value inside the range. For most projects, the default (1 MiB) is correct.

### Migration / schema drift after an upgrade

Preserve Markdown, configuration, and the existing index for diagnosis.
Legacy and Constellation layouts are not interchangeable. Use the
[Constellation guide](/docs/guides/constellation/) to identify the layout and
rebuild through the appropriate indexing command. Repeated `init -y` is not
a schema repair; `scan-import` is not a routine index rebuild.

## See Also

- [`forgeplan init`](/docs/cli/init/) - workspace bootstrap command
- [`forgeplan estimate`](/docs/cli/estimate/) - estimate engine CLI
- [`forgeplan reason`](/docs/cli/reason/) - ADI reasoning command
- [Evidence guide](/docs/methodology/evidence/) - how `congruence_level` and `valid_until` feed into R_eff
- [Search v2 guide](/docs/guides/search-v2/) - hybrid BM25 + semantic search stack
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/) - artefact state machine and how `integrity:` settings affect health
- [Depth Calibration](/docs/methodology/routing/) - task routing and its distinction from schema metadata
