---
depth: standard
id: PROB-106
kind: problem
links:
- target: ADR-021
  relation: refines
- target: RFC-014
  relation: refines
status: active
title: 'Reasoning effort exists only for codex-cli; Devin/Gemini/Cursor have no equivalent mechanism'
---

## Problem

`llm.effort` (added in PRD-087 / RFC-014, ported as `dda8c211`) is forwarded to the
provider subprocess only for `LocalCliProvider::Codex`, via the Codex-specific
`-c model_reasoning_effort="<value>"` flag. Devin, Gemini, and Cursor silently
ignore the configured effort — the test
`effort_is_not_forwarded_to_other_local_providers` encodes that as *expected*
behavior, not a bug.

That is correct as far as it goes, but it leaves the feature asymmetric: an
operator can tune reasoning depth on Codex and cannot tune it on the other three
local-CLI providers, even when those CLIs expose an equivalent control.

## Why it matters

The whole point of `llm.effort` is to let the operator spend the subscription
they already have at the reasoning depth the task needs. Restricting it to one
provider means the other three are stuck at whatever default the vendor picked —
which for a long context-pack or a deep ADI pass is usually "medium" or
"auto", not the high effort the task paid for.

## Per-provider reality (measured 2026-11-18, v0.39.0+constellation.s1.1)

| Provider | Subprocess args built in `build_args` | Effort-shaped control? |
|---|---|---|
| Codex | `exec --ephemeral --sandbox read-only … -c model_reasoning_effort=<v> -` | yes — `-c` key/value |
| Devin | `--print` (+ `--permission-mode auto` upstream) | needs research — Devin has no documented `-c` config flag |
| Gemini | `--prompt ""` | needs research — Gemini CLI takes `--model`, no `-c` |
| Cursor | (none) | needs research — Cursor agent flags differ |

None of the three has a `model_reasoning_effort`-equivalent flag *today* in
`build_args`. Whether the underlying CLIs expose one (env var, config file,
flag) is the open question this problem tracks.

## What to do

1. For each of Devin / Gemini / Cursor, find the native control for reasoning
   depth (CLI flag, env var, config key, or model-variant name) — from the
   vendor's docs, not from the ForgePlan code.
2. If a control exists, extend `LlmConfig` so `effort` maps onto it per-provider
   (the field is already `Option<ReasoningEffort>` and already skips serializing
   when `None`, so no schema break). Keep the "absent → no argv change" contract.
3. If a control does not exist for a provider, record that explicitly in
   `docs/operations/LLM-PROVIDERS.md` next to the Codex section so the asymmetry
   is documented, not silent.
4. Extend `effort_is_not_forwarded_to_other_local_providers` into a per-provider
   assertion table — each provider either forwards a mapped flag or is asserted
   to forward nothing.

## Constraints

- The "absent / null → argv unchanged" contract is load-bearing (tests +
  `LLM-PROVIDERS.md` depend on it) and must survive any extension.
- Read-only sandbox on Codex must be preserved; any new flag must not weaken it.
- Do not invent flags. If a vendor has no effort control, the answer is
  documentation, not a guessed flag name.
