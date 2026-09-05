---
depth: standard
id: ADR-026
kind: adr
links:
- target: ADR-003
  relation: refines
- target: ADR-025
  relation: based_on
status: draft
title: Storage classes for machine-written records
---

---
assigned_number: 26
predicted_number: 26
slug: adr-storage-classes-for-machine-written-records
---

# ADR-026: Storage classes for machine-written records

## Context

vNext introduces four object classes that no shipped decision houses:
WorkContract, ExecutionReceipt, EvidenceBundle, VerificationVerdict, plus an
authority/audit trail. The audit blocked FPV-03/04/05 on this (B3): ADR-003
declares markdown files the single source of truth and RED LINE #11 forbids
direct edits — but receipts and verdicts are written by machines, at volume,
and a git-tracked tree of machine-written files makes both rules unenforceable
as stated. ADR-018 already rejected a second authoritative non-markdown store.

Two constraints frame every option:

- **Local-first, git for sync** (Non-Goals). Anything that must survive a
  clone or be trusted by another machine has to ride git.
- **One owner per state** (FORGE-O). ForgePlan should not become the canonical
  store for facts another system already owns — CI results are the CI
  provider's; ForgePlan references them.

## Decision

One rule, three storage classes. The rule:

> **Git-tracked if a human must review it or another machine must trust it.
> Local if it is raw per-machine material. Referenced if another system owns
> it. Machine-written tracked files are append-only, schema-validated,
> digest-linked, and mutated only through CLI/MCP — RED LINE #11 extends to
> them verbatim.**

### Class A — canonical, git-tracked, append-only

| Object | Home | Form |
|---|---|---|
| WorkContract | `.forgeplan/contracts/` | JSON, one file per contract version, digest in the record |
| EvidenceBundle | `.forgeplan/evidence/` | the EVID artifact evolved: structured machine section + human prose, same id space |
| VerificationVerdict | `.forgeplan/verdicts/` | JSON, digest-links to bundle and contract |

Reviewable in the PR that carries them, survive cloning, referenced by digest
so retargeting is detectable. Append-only means a new version is a new file
and supersession is a link — no merge conflicts by construction, and "edit"
is not an operation that exists.

This amends ADR-003 rather than violating it: *versioned files under
`.forgeplan/` are the source of truth; markdown for human-authored artifacts,
schema-validated JSON for machine-issued records; LanceDB stays derived.*
ADR-003's actual load-bearing idea was never "markdown" — it was "canonical
truth is versioned plain files, indexes are disposable."

### Class B — local raw material, gitignored

| Object | Home |
|---|---|
| ExecutionReceipt | `.forgeplan/receipts/` |
| audit/authority event stream | the existing journal (`forgeplan-core::journal`) |

Receipts are what a host reports about a run: commands, exit codes, streams.
High-volume, per-machine, valuable for minutes-to-days. They are the raw
material verification consumes on the machine where the run happened. What
deserves to outlive the machine gets promoted: the EvidenceBundle embeds the
receipt extract it relies on plus the receipt digest, and the bundle is
Class A. This mirrors the CI precedent — the CI provider owns the run, the
graph keeps the reference and the extract.

The same promotion rule covers audit: routine events stay in the local
journal; trust-relevant transitions (activation, dismissal, force, gate
override) are recorded in the artifact's own tracked state history, which is
per-artifact and append-capped, so the durable trail rides git without a
global conflict-prone log file.

### Class C — referenced, never stored

CI results, deployment state, tracker assignments. A digest or URL plus the
observation timestamp, inside a Class A record. Copying another system's
state into the graph creates a second owner and guarantees drift.

## Consequences

- FPV-03/04/05 unblock: every object in the protocol has a declared home
  before a schema is written.
- `.gitignore` gains `receipts/`; `contracts/` and `verdicts/` are tracked
  from birth. `.forgeplan/state/` is already gitignored today, consistent
  with phase state being advisory and per-machine (ADR-025).
- RED LINE #11 needs one sentence added: machine-issued records under
  `contracts/`, `evidence/`, `verdicts/` are written only by the binary;
  hand-editing them is the same violation as hand-editing an artifact.
- The pre-existing `journal` module becomes the audit stream's home instead
  of a new subsystem.
- Verification MUST re-derive git facts (delta, SHAs) from the repository at
  verdict time rather than trusting receipt contents — the receipt says what
  the host claims happened; the repo says what happened.

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-003 | refines |
| ADR-025 | based_on |



