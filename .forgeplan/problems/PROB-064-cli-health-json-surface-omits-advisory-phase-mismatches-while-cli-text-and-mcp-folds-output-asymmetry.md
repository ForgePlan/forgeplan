---
depth: tactical
id: PROB-064
kind: problem
links:
- target: PROB-063
  relation: informs
- target: PROB-051
  relation: informs
- target: PROB-029
  relation: informs
status: draft
title: CLI health --json surface omits advisory_phase_mismatches while CLI text and MCP folds — output asymmetry
---

# PROB-064: CLI `health --json` и MCP `forgeplan_health` используют разные имена для одного и того же поля advisory phase mismatches

## Signal

Одни и те же data — список «active artifacts whose phase is in early cycle (Shape/Validate/Adi)» — сериализуются двумя surface'ами под **разными именами**:

| Surface | Field name | Same data |
|---|---|---|
| CLI `forgeplan health --json` | `phase_mismatches` | ✅ |
| MCP `forgeplan_health` | `advisory_phase_mismatches` | ✅ |
| CLI text `forgeplan health` | (renders as «Phase mismatches (N):») | ✅ |

Reproducible на любом workspace с advisory phase mismatches:

```bash
$ forgeplan health --json | jq 'keys[] | select(contains("phase") or contains("advisory"))'
"phase_mismatches"

$ # MCP forgeplan_health response (issue #276 reporter)
{ ... "advisory_phase_mismatches": [...] ... }
```

## Context

Обнаружено во время PROB-063 fix verification (Phase B forge-cycle). Изначально симптом воспринят как «CLI JSON не fold'ит advisory phase mismatches» (EVID-117 — содержит этот misstatement, требует correction). Дальнейший root cause analysis: данные присутствуют в обоих surface'ах, но под разными ключами.

**Где в коде:**
- CLI JSON emitter: `crates/forgeplan-cli/src/commands/health.rs:119` использует ключ `"phase_mismatches"`
- MCP response emitter: `crates/forgeplan-mcp/src/server.rs:2927` использует ключ `"advisory_phase_mismatches"`

## Why this matters

1. **Agents/CI scripts** branching на JSON output (issue #276 reporter pattern — `jq .advisory_phase_mismatches`) ломаются при переходе с MCP на CLI surface. Silent breakage — field просто missing, не error.
2. **Naming inconsistency** между surface'ами того же tool'а — API design smell.
3. **Documentation drift**: если docs указывают одно имя — другая surface работает иначе. Reporter был bitten by this (искал advisory_phase_mismatches в CLI JSON, не нашёл, репортнул как separate concern).

## Likely fix (out of PROB-063 scope)

**Option A** (lowest churn): унифицировать на `advisory_phase_mismatches` (semantically correct — слово «advisory» в имени точно описывает semantics). CLI JSON меняет ключ. Breaking change для anyone using `jq .phase_mismatches`. Migration: announce in CHANGELOG, possibly emit ОБА ключа в одной release with deprecation warning.

**Option B** (additive only): MCP добавляет alias `phase_mismatches` рядом с `advisory_phase_mismatches`. CLI добавляет alias `advisory_phase_mismatches` рядом с `phase_mismatches`. Оба consumers работают, eventual cleanup в major version.

**Option C** (re-shape): отдельная operation — задизайнить unified JSON schema для health output, документировать как public API contract, обе surfaces реализуют его. Больше работы, лучший long-term outcome.

## Acceptance criteria (для будущего fix)

1. CLI `forgeplan health --json` и MCP `forgeplan_health` возвращают данные advisory phase mismatches под **одним и тем же ключом** (или с явным aliasing).
2. CHANGELOG отражает naming change/decision.
3. Documentation (operations/MCP-TOOLS.md или equivalent) указывает canonical name + любые aliases.
4. Migration guide для agents/CI scripts (если выбран Option A).

## Linked artifacts

- **informs PROB-063** (обнаружено при работе над PROB-063 — verdict aggregator regression)
- **informs PROB-051** (origin of advisory_phase_mismatches signal class)
- **informs PROB-029** (parent anti-contradiction guarantee — naming inconsistency может re-introduce confusion)

## Status

Discovery only — fix не запланирован в текущей итерации. Записан для future triage.








