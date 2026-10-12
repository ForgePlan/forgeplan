---
depth: tactical
id: EVID-170
kind: evidence
links:
- target: PRD-086
  relation: informs
- target: PROB-101
  relation: informs
status: active
title: 'PRD-086: five trust-layer defects measured before and after'
---

---
assigned_number: 170
created: 2026-09-05
predicted_number: 170
slug: evid-prd-086-five-trust-layer-defects-measured-before-and-after
updated: 2026-09-05
---

# EVID: the trust layer, measured before and after

Five defects in PRD-086, each with a reproduction on 0.36.0 and a re-measurement
after the fix. Every fix carries a test that was mutation-checked — the fix was
reverted and the test had to fail. Two of my own tests failed that check and were
rewritten; both stories are in the test file, because a test that looks like
proof and is not is the defect this PRD exists to remove.

## Fail-closed (FR-008 / FR-009, PROB-101)

| Body | Score of the informed artifact — before | after |
|---|---|---|
| pure prose, no fields | **1.00 "Adequate"** | **0.10** |
| `verdict: unknown`, `congruence_level: 3` | 1.00 | 0.10 |
| `verdict: supports`, no CL source | 1.00 | 0.10 |
| `verdict: supports`, `congruence_level: 3` | 1.00 | 1.00 (unchanged) |

Three documents already specified the "after" column: `CLAUDE.md` RED LINE #7,
the `/forge` skill installed by `setup-skill`, and `EVIDENCE-PROTOCOL.md`. The
binary did the opposite, and in the inflating direction.

Blast radius here, counted rather than estimated: **3 of 167 packs** lack a CL
source — EVID-033, EVID-034, EVID-035 — feeding PROB-014, PROB-016 and RFC-004
respectively. Those three drop to 0.1 and take their targets with them. That is
the correct reading: the scores were never earned.

## Leaf evidence (#325, FR-001/FR-002)

- canonical pack (`supports` / CL3 / measurement) scored **0.0** with the factor
  `No evidence found (L0)`; now **1.00**
- `weakens` / CL3 / measurement now **0.50** — computed, not stamped
- a canonical pack attached to an unevidenced PRD scored **0.0** because its
  outgoing `informs` was walked as a dependency; now **1.00**

## The cascade (#392, narrowed)

- PRD with one CL3 `supports` pack, `based_on` an active unevidenced NOTE:
  **0.00 AT RISK** before, non-zero after
- PRD with the same evidence, `based_on` an active unevidenced **PRD**:
  **0.00 before and after** — deliberately unchanged, and covered by its own
  test so a future "simplification" cannot quietly soften it

## Phase monotonicity (#330)

`advance_phase(done → validate)` succeeded silently. It now returns an error
naming both phases, and `read_phase` confirms the state was left at `done`.
Forward jumps, no-ops, and the explicit `advance_phase_unchecked` path all still
work, each with a test.

## Anomaly detector (#393)

Three separate misreports, each fixed at its source: the literal `0.0` replaced
by the stored score and renamed `r_eff_cached`; the blanket
`cycle or depth cap` replaced by the reason that actually occurred; and the
ancestor walk given the scorer's skip rules so the two stop disagreeing about
which artifact is the weakest link.

## What dogfooding caught that the tests did not

Running `forgeplan score EVID-170` on the real graph printed:

```
  No evidence linked. R_eff = 0.0
  • Leaf evidence scored on its own fields: Supports CL3 = 1.00
```

The engine computed 1.00 and the command announced 0.0 over it. `score.rs` had
its own display branch keyed on "no linked evidence", written when that
condition could only mean one thing. Neither the unit tests nor the integration
tests read that line, so both stayed green — the same shape as the defects this
PRD closes, committed by the printer instead of the scorer.

Fixed, and verified across all four cases: canonical pack (1.00), fieldless pack
(0.10 plus a remediation naming the missing fields), an artifact with linked
evidence (breakdown unchanged), an artifact with none (unchanged).

## Gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets` | exit 0, 0 warnings |
| `cargo test --workspace --no-fail-fast` | **3307 passed, 3 failed**, 94 binaries |

A first attempt at the full run produced `тестовых бинарей: 0` with
`ld: write() failed, errno=28` — the disk was at 152 MB. The harness reported
that the run had not happened instead of printing a passing summary, which is
the behaviour PROB-090's class of failure needs. Re-run after `cargo clean` gave the numbers above.

The 3 failures are #454 / PROB-090, not this change: all three are in
`git::tests`, none of the commits here touch `crates/forgeplan-core/src/git/`,
and all 51 tests in that module pass when run single-threaded. The flake is
parallel env mutation dropping `PATH`; CI does not see it because `cargo
nextest` gives each test its own process.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement


