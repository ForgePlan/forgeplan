---
depth: standard
id: EVID-087
kind: evidence
links:
- target: PRD-071
  relation: informs
- target: EVID-086
  relation: refines
status: active
title: Marketplace plugin forgeplan-workflow v1.5.0 published — distribution gap closed for PRD-071 hint contract
valid_until: 2026-10-28
---

# EVID-087: Marketplace plugin v1.5.0 distribution

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: integration

## Context

Forgeplan v0.25.0 (released 2026-04-27) shipped PRD-071 unified hint contract — every CLI/MCP output emits one of 5 markers (`Next:`/`Or:`/`Wait:`/`Done.`/`Fix:`), audit coverage 0% → 100% (70/70 commands). EVID-086 documented that work.

**Distribution gap**: marketplace plugin `forgeplan-workflow` v1.4.0 did not teach agents about the contract — its skill, commands, agent and README didn't mention markers at all. Users installing the plugin would get an agent unaware of v0.25.0's contract → wasted PRD-071 work.

This evidence records the closing of that gap via marketplace v1.5.0.

## Measurement

### Marketplace state — before (2026-04-27)

- forgeplan-workflow plugin: v1.4.0
- Marketplace catalog: v1.6.0
- Hint contract awareness in plugin: **0** (zero mentions of `Next:`/`Done.`/`Fix:` in skill, commands, agent, README)

### Marketplace state — after (2026-04-28, PR #25 merged)

- forgeplan-workflow plugin: **v1.5.0** (squash commit `51ad519` on `main`)
- Marketplace catalog: **v1.7.0**
- Hint contract awareness: **5 touchpoints** updated:

| Touchpoint | Before | After |
|---|---|---|
| `skills/forgeplan-methodology/SKILL.md` | no mention | Section router row + new "Hint Protocol" top-level section |
| `skills/forgeplan-methodology/sections/06-output-hints/agent-protocol.md` | not present | NEW 190-line full agent reading protocol |
| `commands/forge-cycle.md` | no mention | New "Reading Forgeplan Output" prelude before Step 1 |
| `agents/forge-advisor.md` | 4 behaviors | New behavior #5 "Hint Contract Awareness" (SPARC moved to #6, sequential 1→6) |
| `README.md` + `README-RU.md` | no mention | "Hint Contract (v1.5.0+)" subsection in both languages |

### PR metrics

- PR: https://github.com/ForgePlan/marketplace/pull/25 (MERGED)
- Files changed: 7
- Additions: 288
- Deletions: 1
- CI: validate workflow PASS (4s)
- Quality: 2-round audit (3 reviewers + 1 re-verify), 0 HIGH / 2 MEDIUM (fixed) / 8 LOW
- Cross-plugin pollution: zero (scope strictly bounded to forgeplan-workflow)

## Verdict

The distribution gap is closed. PRD-071 work is now reachable to marketplace users:

```bash
/plugin marketplace update ForgePlan-marketplace
# pulls forgeplan-workflow v1.5.0 → agents become hint-contract-aware
```

Requires Forgeplan binary ≥ v0.25.0 to actually see markers. Older binaries: plugin works but no markers in output → no awareness benefit.

## Two-repo sync pattern (lesson)

This sprint demonstrated the **engine + marketplace dual-repo** pattern for methodology contracts:

1. **Engine repo** (`ForgePlan/forgeplan`): contract spec lives in `docs/methodology/agent-protocol.md`, enforcement via `tests/hint_contract.rs` + `scripts/audit-hints.sh`
2. **Marketplace repo** (`ForgePlan/marketplace`): plugin teaches end-user agents to read contract markers

Without #2, #1's work is invisible to the primary user (agents installed via plugin). Future methodology changes should bump both repos in same release window.

## Verification (anyone can reproduce)

```bash
# Pull marketplace
git clone https://github.com/ForgePlan/marketplace
cd marketplace && git checkout main

# Verify version
jq '.plugins[] | select(.name=="forgeplan-workflow") | .version' .claude-plugin/marketplace.json
# Expected: "1.5.0"

# Verify new section file
ls plugins/forgeplan-workflow/skills/forgeplan-methodology/sections/06-output-hints/
# Expected: agent-protocol.md (190 lines)

# Verify forge-cycle prelude
grep -A 5 "Reading Forgeplan Output" plugins/forgeplan-workflow/commands/forge-cycle.md
# Expected: 5 marker descriptions
```

## Related

- **PRD-071** (informs) — Unified hint contract specification
- **EVID-086** (refines) — Original measurement of v0.25.0 sprint completion. This evidence records the follow-on distribution work that EVID-086 left as TODO ("PR merged to dev — pending user approval").
- **PROB-046** (root cause) — Original gap signal
- **Marketplace PR #25** (https://github.com/ForgePlan/marketplace/pull/25) — implementation
