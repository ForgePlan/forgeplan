---
depth: tactical
id: EVID-147
kind: evidence
links:
- target: ADR-017
  relation: informs
status: draft
title: 'ADR-017 claude-code provider: guardian PASS + 2 audits resolved + AC-1..7 verified'
---

## Summary

Pre-activation verification for ADR-017 (claude-code LLM provider). The full
AD/AID cycle ran: ADR → implementation → two independent adversarial audits
(both CONCERNS) → coder fix-pass → docs/disclosure → guardian gate (PASS).

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

CL3: the audits + guardian read the exact shipped code on this branch and
verified each AC against code/tests line-by-line.

## Audit chain

| Stage | Agent | Verdict |
|---|---|---|
| Security audit (STRIDE/CWE, live `claude` v2.1.165) | security-expert | CONCERNS → resolved |
| Code-quality review | code-reviewer | CONCERNS → resolved |
| Fix pass (commit `c553a66`) | coder | all HIGH/MED closed |
| Pre-activation gate | guardian | **PASS** |

## HIGH findings resolved (commit `c553a66`)

- **F-1 argv-injection defence**: `model` charset-gated `^[A-Za-z0-9._:-]{1,64}$`, leading `-` rejected, before argv. Sibling-dispatcher parity.
- **F-2 dash-leading-prompt DoS**: prompt fed via **stdin**, not `-p` — removes the external-parser dependency for prompt content.
- **CR-1 output cap**: `read_capped(MAX_OUTPUT_BYTES=10MiB)` actually enforced (was unbounded `read_to_end`; comment falsely claimed a cap).
- **CR-2/CR-3 timeout**: pipes dropped before `kill()`/`wait()`; real `sleep`-based timeout test added.
- **CR-4 Linux keychain**: `XDG_{CONFIG,DATA,RUNTIME}` forwarded (cfg-gated) so `claude` finds creds; secrets still excluded.

## AC-1..AC-7 verification (guardian, per code:line)

All seven satisfied: AC-1 disclosure (runtime `Once` + `LLM-PROVIDERS.md` + CHANGELOG); AC-2 stock binary/flags, no spoofing; AC-3 recursion sentinel bounded depth 1; AC-4 every error path → `anyhow::Error`, no panic; AC-5 `env_clear()`+PATH/HOME/USER(+XDG); AC-6 default stays `openai`, opt-in only; AC-7 keyless set = {ollama, claude-code}, api-key gate exempts.

## Test surface

25 test fns in `llm/mod.rs` (arg-builder/CWE-78, model-gate rejection, stdin delivery, recursion guard, missing-binary, non-zero exit, in-band error envelope, timeout) + config keyless tests. forgeplan-core lib: 2018 pass (coder run, raw exit 0). Final green CI on the PR is the hard pre-merge gate.

## Provenance

- Branch `feat/v0.33-claude-code-provider`; commits `146973d` (impl), `c553a66` (fix), `f8e7561`+`5d94326` (AC-7+docs).
- Guardian PASS; security + code-review CONCERNS both resolved.
- Prior art: Hindsight `HINDSIGHT_LLM_PROVIDER=claude-code` (personal/local-only).


