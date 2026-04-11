---
title: Configuration
description: "Complete reference for .forgeplan/config.yaml — LLM providers, embeddings, storage, estimate engine, FPF trust calculus tuning."
---

The `.forgeplan/config.yaml` file is the single configuration point for a Forgeplan workspace. This page is the authoritative reference for every top-level key, every nested section, and every environment variable recognised by the CLI and MCP server.

All sections are optional except the top-level metadata (`version`, `project_name`, `default_depth`, `id_digits`, `created_at`). Missing sections fall back to safe defaults baked into `forgeplan-core`.

## Workspace Structure

After `forgeplan init -y`, the `.forgeplan/` directory is created next to your code:

```
.forgeplan/
├── config.yaml         ← workspace config (GITIGNORED — contains env refs)
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
├── lance/              ← GITIGNORED — derived LanceDB index (rebuildable)
└── .fastembed_cache/   ← GITIGNORED — embedding model cache
```

Per ADR-003, **markdown files are the source of truth**; `lance/` is a derived index you can rebuild at any time with `forgeplan scan-import`. Never commit `lance/`, `.fastembed_cache/`, or `config.yaml` — they are listed in the default `.gitignore`.

:::caution[Reinit wipes local state]
Running `forgeplan init -y` in an existing workspace will overwrite `config.yaml`. Always back up first:

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
```
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
| `default_depth` | `enum` | `standard` | Default depth used by `forgeplan route` when heuristics are inconclusive. One of `tactical`, `standard`, `deep`, `critical`. |
| `id_digits` | `u32` | `3` | Zero-padding width for artifact IDs (e.g. `PRD-001` vs `PRD-0001`). Change only at workspace creation — existing IDs are not renumbered. |
| `created_at` | `date` | today | `YYYY-MM-DD` when the workspace was initialised. Read-only metadata. |

### Depth values

| Value | When to use |
|-------|-------------|
| `tactical` | Quick fix, reversible in a day. No artefact required. |
| `standard` | Feature 1–3 days, one clear tradeoff. PRD -> RFC pipeline. ADI recommended. |
| `deep` | New module, 1–2 weeks. PRD -> Spec -> RFC -> ADR. ADI mandatory. |
| `critical` | Cross-team subsystem, strategic. Epic -> N artifacts. ADI + adversarial review. |

See [Depth Calibration guide](/docs/methodology/routing/) for routing heuristics.

## `llm:` — LLM Provider

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
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | `enum` | `openai` | One of `openai`, `claude`, `gemini`, `ollama`, or `custom`. Determines default base URL and header style. |
| `model` | `string` | `gpt-4o-mini` | Model identifier passed verbatim to the provider. See recommended models below. |
| `api_key_env` | `string?` | provider-specific | Name of the environment variable holding the API key. If omitted, a provider-specific default is used. |
| `base_url` | `string?` | provider-specific | Override base URL for self-hosted or proxy endpoints. Useful for Ollama, LiteLLM, or Azure-compatible gateways. |
| `max_tokens` | `u32` | `4096` | Max response tokens. Increase for long ADI reasoning; decrease to save cost. |
| `temperature` | `f32` | `0.7` | Sampling temperature for `generate`. `0.0` = deterministic, `1.0` = creative. |
| `reason_temperature` | `f32?` | — | Override used only by `forgeplan reason`. Structured ADI output benefits from a lower value (typically `0.2`–`0.3`). Falls back to `temperature` if unset. |

### Provider matrix

| Provider | Default API key env | Default base URL | Notes |
|----------|---------------------|------------------|-------|
| `openai` | `OPENAI_API_KEY` | `https://api.openai.com/v1` | OpenAI-compatible. |
| `claude` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` | Uses Anthropic-specific headers automatically. |
| `gemini` | `GEMINI_API_KEY` | `https://generativelanguage.googleapis.com/v1beta/openai` | Uses Google's OpenAI-compatible shim. |
| `ollama` | — (none) | `http://localhost:11434/v1` | Fully local. No API key required. |
| `custom` | — | — | You **must** set `base_url` and `api_key_env` explicitly. |

### Recommended models (2026)

| Provider | Model | When to use |
|----------|-------|-------------|
| Gemini | `gemini-3-flash-preview` | **Default** — fast, cheap, strong ADI output (currently used by the Forgeplan repo itself). |
| Gemini | `gemini-3-pro` | Deep reasoning, critical decisions, long context. |
| OpenAI | `gpt-5-mini` | Balanced price/quality for generate + route. |
| OpenAI | `gpt-5` | Critical ADI, adversarial review. |
| Anthropic | `claude-haiku-4-5-20251001` | Cheap routing and classification. |
| Anthropic | `claude-sonnet-4-6` | Default for generate + reason. |
| Anthropic | `claude-opus-4-6` | Critical decisions, long-form reasoning. |

:::note[Switching providers mid-project]
You can switch providers at any time — artefacts are model-agnostic. Existing ADI records, reviews, and generated content remain valid. Only new LLM calls will use the new provider.
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

## `embedding:` — Semantic Search

Configures the embedding model used for semantic search and the FPF KB vector index. Requires the `semantic-search` feature flag at build time (included in official release binaries).

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
| `bge-m3` | 1024 | Multilingual (100+) | **Default** — best quality, supports Russian + English mixed workspaces. |
| `bge-small-en` | 384 | English only | Fastest, smallest cache. Pick for English-only projects on low-RAM machines. |
| `multilingual-e5-small` | 384 | Multilingual | Middle ground — faster than bge-m3, still multilingual. |
| `multilingual-e5-base` | 768 | Multilingual | Higher quality than e5-small at ~2x cost. |

:::caution[Switching models requires reindex]
Embedding vectors from different models are **not compatible**. After changing `model`, run `forgeplan scan-import` to rebuild `lance/` from scratch.
:::

### Env overrides (embedding)

| Env var | Overrides |
|---------|-----------|
| `FORGEPLAN_EMBEDDING_MODEL` | `model` |

If the `semantic-search` feature is disabled at build time, Forgeplan still runs — it silently falls back to BM25 keyword search. See the [Search guide](/docs/guides/search-v2/) for details on the hybrid search stack.

## `storage:` — Storage Backend

```yaml
storage:
  driver: lancedb
  # path: /custom/path            # override DB location
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `driver` | `enum` | `lancedb` | Storage backend. See table below. |
| `path` | `string?` | `.forgeplan/lance/` | Override the LanceDB directory. Useful to keep the derived index outside the project tree (e.g. `~/.cache/forgeplan/myproj`). |

### Supported drivers

| Driver | Use case |
|--------|----------|
| `lancedb` | **Default** — embedded columnar DB with native vector search. Persists to `lance/`. Recommended for all real projects. |
| `sqlite` | Legacy / lightweight fallback. No vector search. |
| `memory` | In-memory only, lost on process exit. Used by tests and ephemeral CI runs. |

### Env overrides (storage)

| Env var | Overrides |
|---------|-----------|
| `FORGEPLAN_STORAGE_DRIVER` | `driver` |
| `FORGEPLAN_STORAGE_PATH` | `path` |

## `memory:` — Decision Memory Bank

```yaml
memory:
  driver: file
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `driver` | `enum` | `file` | Memory bank driver. `file` persists to `.forgeplan/memory/`. `none` disables the decision journal entirely. |

### Env overrides (memory)

| Env var | Overrides |
|---------|-----------|
| `FORGEPLAN_MEMORY_DRIVER` | `driver` |

## `estimate:` — Estimate Engine

Configures `forgeplan estimate` — the multi-grade estimation model that combines your domain expertise with AI task-type multipliers. Every field is optional; omit the whole section to use defaults.

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
    ai: 0.4                  # conservative AI base multiplier
  ai_task_multipliers:
    pure_coding: 0.10        # AI does coding ~10x faster
    coding_infra: 0.25       # mixed coding + infrastructure
    design_coding: 0.30      # design + implementation
    pure_infra: 0.50         # infrastructure only
    coordination: 1.00       # meetings, reviews — AI can't help
  review_overhead: 0.30      # 30% added to AI time for human review
  safety_margin: 0.50        # warn if sprint > 50% loaded
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `grade_profile` | `map<string, grade>` | — | Per-domain developer grade. Keys are free-form domain names; at minimum define `default`. |
| `grade_multipliers` | `map<string, f64>` | see below | Time multiplier per grade, relative to a senior baseline of `1.0`. |
| `ai_task_multipliers` | `map<string, f64>` | see below | Fraction of human time AI takes for each task type. |
| `review_overhead` | `f64` | `0.30` | Fraction added to AI-assisted time to cover human review. `0.30` = 30%. |
| `safety_margin` | `f64` | `0.50` | Sprint capacity threshold. Forgeplan warns if the sprint load exceeds this. `0.50` = 50%. |

### Grades

| Grade | Default multiplier | Meaning |
|-------|-------------------:|---------|
| `junior` | 2.0 | Learning the domain — takes 2x a senior's time. |
| `middle` | 1.5 | Can ship independently — 1.5x a senior. |
| `senior` | 1.0 | Baseline. |
| `principal` | 0.7 | Deep domain expert — 30% faster than senior. |
| `ai` | 0.4 | AI as a virtual collaborator — conservative base. Refined by `ai_task_multipliers`. |

### AI task types

| Task type | Default | Typical work |
|-----------|--------:|--------------|
| `pure_coding` | 0.10 | Writing pure code — functions, tests, refactors. AI excels here. |
| `coding_infra` | 0.25 | Coding mixed with tooling (CI, scripts, configs). |
| `design_coding` | 0.30 | Design decisions + implementation. |
| `pure_infra` | 0.50 | Infra-only work — AI helps but human validates each step. |
| `coordination` | 1.00 | Meetings, reviews, stakeholder alignment. AI cannot help. |

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

## `fpf:` — FPF Trust Calculus Engine

Tunes the [FPF Engine](/docs/methodology/adi/): explore/exploit thresholds, reliability weights, congruence-level penalties, and ADI reasoning caps. Every sub-field has safe defaults — tune only if you have empirical reason to.

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

### `fpf.weights` — Reliability component weights

Components of the reliability score inside F-G-R. Values do not need to sum to 1.0, but conventionally do.

| Field | Default | Meaning |
|-------|--------:|---------|
| `reff` | `0.5` | Weight of the R_eff score (evidence quality). |
| `links` | `0.3` | Max bonus awarded for incoming/outgoing typed links. |
| `freshness` | `0.2` | Bonus if the artefact is not stale (`valid_until` in the future). |

### `fpf.thresholds` — Explore/Exploit cutoffs

Decision thresholds used by hardcoded rules and by [`forgeplan route`](/docs/cli/route/).

| Field | Default | Action triggered |
|-------|--------:|------------------|
| `explore_reff` | `0.01` | R_eff below this -> **EXPLORE** (treat as draft). |
| `investigate_reff` | `0.5` | R_eff below this -> **INVESTIGATE** (needs more evidence). |
| `exploit_reff` | `0.7` | R_eff at or above this -> eligible for **EXPLOIT** (safe to rely on). |
| `exploit_fgr` | `0.6` | F-G-R overall required to confirm EXPLOIT (combined with `exploit_reff`). |
| `explore_fgr` | `0.4` | F-G-R below this -> **EXPLORE** priority 1 (combined with `explore_reff`). |

### `fpf.cl_penalties` — Congruence Level penalties

Penalty applied to evidence based on how well its context matches the artefact it informs. `CL3` (same context) is penalty-free; `CL0` (opposed context) is heavily discounted.

| Field | Default | Meaning |
|-------|--------:|---------|
| `cl0` | `0.9` | Opposed context — near-zero trust. |
| `cl1` | `0.4` | Different context — significant discount. |
| `cl2` | `0.1` | Similar context — minor discount. |
| `cl3` | `0.0` | Same context — no penalty. |

See [Evidence guide](/docs/methodology/evidence/) for how `congruence_level` is set on an EvidencePack body.

### `fpf.decay` — Evidence decay

| Field | Default | Meaning |
|-------|--------:|---------|
| `expired_score` | `0.1` | Score assigned to evidence past `valid_until`. `0.1` reflects "stale, not absent" — the evidence existed, just needs re-verification. |

### `fpf.adi` — ADI reasoning configuration

Controls `forgeplan reason` behaviour.

| Field | Type | Default | Meaning |
|-------|------|--------:|---------|
| `max_hypotheses` | `u32` | `5` | Maximum number of competing hypotheses the LLM must generate during the Abduction phase. |
| `kb_sections_limit` | `usize` | `5` | Max FPF KB sections injected into the ADI prompt. Higher = richer context, more tokens. |
| `temperature_cap` | `f32` | `0.3` | Upper bound on temperature used for ADI reasoning, regardless of `llm.temperature`. Keeps ADI output structured. |
| `auto_save` | `bool` | `true` | Automatically persist ADI results as an `AdiRecord` linked to the artefact. |

### `fpf.rules` — Declarative explore-exploit rules (advanced)

Optional list of user-defined explore/exploit rules (FPF Engine Phase 2). When empty, built-in `default_rules()` are used. Schema and examples live in the [FPF rules guide](/docs/cli/fpf-rules/).

:::caution[FPF validation]
All `fpf.*` numeric fields must be finite and non-negative. Rule names must be unique. `forgeplan init` and `forgeplan health` will fail fast on malformed values.
:::

## `integrity:` — Health & MCP Input Limits

Thresholds used by `forgeplan health` (duplicate detection, stub detection) and DoS-protection limits enforced by the MCP server on incoming `forgeplan_new` / `forgeplan_update` calls.

```yaml
integrity:
  duplicate_threshold: 0.7
  duplicate_pairs_limit: 10
  stub_marker_threshold: 3
  mcp_max_title_len: 256
  mcp_max_body_len: 1048576      # 1 MiB
```

| Field | Type | Default | Range | Description |
|-------|------|--------:|-------|-------------|
| `duplicate_threshold` | `f64` | `0.7` | `[0.0, 1.0]` | Jaccard similarity above which two artefacts are flagged as duplicates in `forgeplan health`. |
| `duplicate_pairs_limit` | `usize` | `10` | `[1, 10000]` | Max duplicate pairs shown in health output (pagination). |
| `stub_marker_threshold` | `usize` | `3` | `>= 1` | Minimum number of stub markers (`TODO`, `TBD`, empty headings, etc.) required to flag an artefact body as a stub. |
| `mcp_max_title_len` | `usize` | `256` | `[16, 4096]` | Max artefact title length accepted via MCP. Prevents memory abuse from malicious clients. |
| `mcp_max_body_len` | `usize` | `1048576` | `[1024, 104857600]` | Max artefact body length (bytes) accepted via MCP. Default: 1 MiB. Hard cap: 100 MiB. |

:::note[Why these limits exist]
The MCP server is network-reachable when run over stdio by a shared LLM agent. The MCP limits protect against runaway prompts or buggy clients that would otherwise fill up `lance/` with multi-megabyte artefacts.
:::

## Environment Variables — Complete List

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
| `FORGEPLAN_EMBEDDING_MODEL` | embedding | Override `embedding.model`. |
| `FORGEPLAN_STORAGE_DRIVER` | storage | Override `storage.driver`. |
| `FORGEPLAN_STORAGE_PATH` | storage | Override `storage.path`. |
| `FORGEPLAN_MEMORY_DRIVER` | memory | Override `memory.driver`. |

API keys themselves are **never** stored in `config.yaml` — only the **name** of the env variable is stored under `api_key_env`. This keeps the config file safe to share across machines (once `.forgeplan/config.yaml` itself is in `.gitignore`).

## Critical Notes & Git Safety

:::caution[.forgeplan/ is partially gitignored]
- **Tracked**: `adrs/`, `rfcs/`, `prds/`, `epics/`, `specs/`, `problems/`, `solutions/`, `evidence/`, `notes/`, `refresh/`, `memory/` — these are the source of truth.
- **Not tracked**: `config.yaml`, `lance/`, `.fastembed_cache/` — local, rebuildable, or secret.

This means `config.yaml` is **lost on fresh clone**. Every developer configures their own LLM provider and keys via `forgeplan init -y` + manual edit.
:::

### Before any reinit

```bash
# 1. Export all artefacts to a portable JSON bundle
forgeplan export --output backup.json

# 2. Keep a directory-level backup of the whole workspace
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)

# 3. Only now it's safe to reinit
rm -rf .forgeplan
forgeplan init -y

# 4. Restore artefacts
forgeplan import backup.json
```

### Fresh clone workflow

```bash
git clone <repo> && cd <repo>
forgeplan init -y                   # creates .forgeplan/config.yaml + empty lance/
$EDITOR .forgeplan/config.yaml      # set llm.provider, model, api_key_env
export GEMINI_API_KEY=...           # or whichever provider you chose
forgeplan scan-import               # rebuilds lance/ from tracked markdown
forgeplan list                      # verify artefacts are back
```

### AI agent mode

AI agents (Claude Code, Codex, others) running Forgeplan must always use:

```bash
forgeplan init -y      # NEVER interactive — -y is required
```

Interactive mode will hang in an agent harness. The `-y` flag accepts all defaults and writes a minimal `config.yaml`, which the agent can then edit.

## Troubleshooting

### "API key not found"

```
error: LLM API key not set — expected env var GEMINI_API_KEY
```

**Cause**: `llm.api_key_env` points to an unset variable, or the `provider` default env var is unset.

**Fix**:
```bash
export GEMINI_API_KEY=your-key          # for current shell
# or
export FORGEPLAN_LLM_API_KEY_ENV=MY_CUSTOM_KEY
export MY_CUSTOM_KEY=your-key
```

Use `forgeplan health` to confirm the LLM subsystem reports "ready".

### LLM rate limit / 429 errors

**Cause**: provider rate limits (Gemini free tier is especially tight).

**Fix**:
1. Lower `llm.max_tokens` to reduce per-request cost.
2. Switch to a cheaper model (`gemini-3-flash-preview`, `gpt-5-mini`, `claude-haiku-4-5-20251001`).
3. Retry with exponential backoff — Forgeplan surfaces the provider error verbatim so you can distinguish 429 from 5xx.

### Embeddings fail to load / semantic search returns empty

**Cause**: one of:
- Forgeplan binary was built without the `semantic-search` feature (check `forgeplan --version`).
- `embedding.model` was changed and `lance/` was not reindexed.
- `.fastembed_cache/` is corrupted.

**Fix**:
```bash
rm -rf .forgeplan/.fastembed_cache
forgeplan scan-import                # re-downloads model + reindexes
```

If semantic search is unavailable, Forgeplan falls back to BM25 keyword search automatically — no data is lost. See the [Search guide](/docs/guides/search-v2/) for the hybrid stack.

### "Invalid config: fpf.thresholds.explore_reff must be finite"

**Cause**: malformed YAML — a numeric field is `NaN`, `Infinity`, or a string that didn't parse as a number.

**Fix**: open `.forgeplan/config.yaml` and ensure every numeric field under `fpf:`, `estimate:`, and `integrity:` is a plain decimal. Run `forgeplan health` to revalidate.

### "integrity.mcp_max_body_len must be in [1024, 104857600]"

**Cause**: MCP body limit set outside the allowed range (1 KiB to 100 MiB).

**Fix**: pick a value inside the range. For most projects, the default (1 MiB) is correct.

### Migration / schema drift after an upgrade

Some upgrades add new columns to `lance/`. The symptom is a LanceDB error on startup.

**Fix**:
```bash
forgeplan export --output backup.json
rm -rf .forgeplan/lance
forgeplan init -y                    # recreates lance/
forgeplan scan-import                # reindex from markdown
# markdown is the source of truth — no artefacts are lost
```

## See Also

- [`forgeplan init`](/docs/cli/init/) — workspace bootstrap command
- [`forgeplan estimate`](/docs/cli/estimate/) — estimate engine CLI
- [`forgeplan reason`](/docs/cli/reason/) — ADI reasoning command
- [Evidence guide](/docs/methodology/evidence/) — how `congruence_level` and `valid_until` feed into R_eff
- [Search v2 guide](/docs/guides/search-v2/) — hybrid BM25 + semantic search stack
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/) — artefact state machine and how `integrity:` settings affect health
- [Depth Calibration](/docs/methodology/routing/) — how `default_depth` interacts with `forgeplan route`
