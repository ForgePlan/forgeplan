---
title: forgeplan decay
description: "Preview how upcoming evidence expirations will impact R_eff scores across the project."
---

`forgeplan decay` looks ahead at the `valid_until` fields on every EvidencePack and
shows which decisions will lose R_eff soon. Remember: expired evidence does not
disappear — it caps at 0.1 (stale, not absent). Because R_eff is a weakest-link
function, one expiring piece of evidence can drag a previously-green decision into
the red.

This is the tool for planning a **Refresh sprint**: see who is about to go stale,
schedule a `RefreshReport`, and renew the evidence before it bites.

## When to use

- Start of the month: "which decisions will need re-verification this sprint?"
- Before relying on an old decision — is its R_eff about to drop?
- After bulk-creating evidence with the same expiry — spot the cliff before it hits.
- Session start alongside `forgeplan health` for the full "what needs attention" view.

## When NOT to use

- When you have no evidence at all (R_eff already 0) — nothing to decay.
- As a one-shot gate — use `stale` for hard expiry detection.

## Usage

```text
forgeplan decay
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### See upcoming decay impact

```bash
forgeplan decay
```

Output:

```text
Evidence decay report
─────────────────────
EVID-012  expires in  14d  PRD-003: R_eff 0.90 → 0.70
EVID-018  expires in  30d  ADR-005: R_eff 1.00 → 0.10  ⚠ cliff
EVID-021  expires in  45d  PRD-007: R_eff 0.80 → 0.60
EVID-030  expires in  60d  PRD-011: R_eff 0.90 → 0.80

4 evidence packs expiring within 60 days
2 decisions will drop below 0.7 (trust threshold)
```

### Feed into a sprint plan

Combine with `stale` and `health`:

```bash
forgeplan decay          # what's coming
forgeplan stale          # what's already expired
forgeplan health         # overall blind-spot view
```

Those three commands are the full "refresh backlog" picture.

## Output interpretation

- **expires in Xd** — days until `valid_until`.
- **R_eff X → Y** — current cached R_eff vs projected after decay.
- **⚠ cliff** — flagged when a single expiry takes R_eff across a trust boundary (0.9 → 0.1 is a cliff; 0.9 → 0.8 is not).
- Cliffs are the priority: schedule a `forgeplan new refresh "Re-verify EVID-XXX"` before the date.

## How it fits the workflow

```
monthly review → decay → plan refresh sprint → renew evidence → score
```

Decay is preventive. Stale is reactive. Run decay before evidence expires; run stale
to catch the ones you missed.

## See also

- [`forgeplan stale`](/docs/cli/stale/) — hard expiry detection after the fact
- [`forgeplan score`](/docs/cli/score/) — recomputes R_eff after renewal
- [`forgeplan renew`](/docs/cli/renew/) — extend valid_until on stale artifacts
- [Evidence methodology](/docs/methodology/evidence/) — decay formula and CL penalties
- [CLI overview](/docs/cli/)
