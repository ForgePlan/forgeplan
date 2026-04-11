---
title: forgeplan blindspots
description: "Active decisions without evidence and orphan artifacts — critical health gate"
---

Find **blind spots** — active decisions that have no evidence backing them
(R_eff = 0) and orphan artifacts not linked to any parent. This is the
most important health-triage command: an active PRD/RFC/ADR without
evidence is a "false promise" — a decision that looks real but has no
measurement behind it.

## When to use

- **Session start** — run this right after `forgeplan health` and fix before
  starting new work (do not accumulate debt)
- **CI gate** — fail the build if any active `critical` artifact has no evidence
- **Pre-release** — zero blind spots before cutting a tag
- **Retro** — count how many blind spots accumulated over the sprint

## Not to use when

- You want _stuck_ items (different problem) → use [`forgeplan blocked`](/docs/cli/blocked/)
- You want overall rollup → use [`forgeplan health`](/docs/cli/health/)
- You want decision timeline → use [`forgeplan journal`](/docs/cli/journal/) with `--risk`

## Usage

```text
forgeplan blindspots
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

Print all blind spots:

```bash
forgeplan blindspots
```

Use as a CI gate (exits non-zero on any blind spot):

```bash
forgeplan blindspots || exit 1
```

Combined with [`health --ci`](/docs/cli/health/) for full gating:

```bash
forgeplan health --ci && forgeplan blindspots
```

## Output interpretation

Two sections — _blind decisions_ and _orphans_:

```
BLIND DECISIONS (active, R_eff = 0.00)
  PRD-046  Docs v0.18.0 catch-up    → create EvidencePack + link
  ADR-008  Cloudflare Pages choice  → create EvidencePack + link

ORPHANS (no parent)
  PRD-044  Unused PRD               → link to Epic or deprecate
  NOTE-031 One-off observation      → link to artifact or ignore

Summary: 2 blind decisions, 2 orphans
```

| Section         | What it means                                         |
|-----------------|-------------------------------------------------------|
| Blind decisions | Active PRD/RFC/ADR with `R_eff = 0` (no evidence)     |
| Orphans         | Artifact with no `parent` and no inbound link         |

Exit code: `0` if clean, `1` otherwise. Use this in git pre-push hooks or CI.

## How it fits

`blindspots` is a strict subset of what [`health`](/docs/cli/health/) reports
— extracted as a standalone command because it is the most common gate to
enforce. The unified workflow mandates:

```
session start → health → blindspots → (fix) → new work
```

If `blindspots` is not empty, _fix it first_. Never let active decisions
drift without measurement — that is exactly what the R_eff weakest-link
model is designed to prevent.

## See also

- [`forgeplan health`](/docs/cli/health/) — full project dashboard (with
  `--ci` mode)
- [`forgeplan journal --risk`](/docs/cli/journal/) — time-axis view of
  blind decisions
- [`forgeplan score`](/docs/cli/score/) — R_eff for one artifact
- [`forgeplan new evidence`](/docs/cli/new/) — create an EvidencePack
- [`forgeplan link`](/docs/cli/link/) — attach evidence with `--relation informs`
