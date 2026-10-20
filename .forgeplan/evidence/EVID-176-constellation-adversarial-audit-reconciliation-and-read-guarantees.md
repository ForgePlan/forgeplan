---
depth: tactical
id: EVID-176
kind: evidence
links:
- target: RFC-015
  relation: informs
status: draft
title: 'Constellation adversarial audit: reconciliation and read guarantees'
---

---
assigned_number: 176
created: 2026-10-18
predicted_number: 176
slug: evid-constellation-adversarial-audit-reconciliation-and-read-guarantees
updated: 2026-10-18
---

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: code_review

## Scope

Read-only adversarial review of Constellation on feat/constellation-p0-contract,
HEAD 16f6686b, against ForgePlan-Constellation-Architecture-2026-10-16.docx
(SHA-256 1c3c92fd6f1875f1a9be434d0e18f93c30c33c1ba08c9c851cda6d6f228551ec)
and .local/planning/constellation-minimax/ACCEPTANCE.md. Six independent
read-only reviews plus source cross-check; findings are static unless noted.

## Findings

1. HIGH: changing any artifact triggers whole-corpus merge with null
   embedding/profile/semantic state on every matched row, including unchanged
   rows (publisher.rs:1182-1189,1319-1320,1351-1356,2347-2352;
   facade.rs:943-948). Existing facade.rs:3022 test exercises only a
   no-change reconcile, not a changed-file reconcile after embed.
2. HIGH: changing canonical id in place leaves old outgoing relations:
   reconcile matches by path but relation deletion uses only new and tombstone
   keys (reconciler.rs:127-150; facade.rs:884-915). Post-commit edge count
   verification then fails; retry cannot remove the orphan.
3. HIGH: `forgeplan reindex` calls LanceStore::init without migration guard
   (cli/commands/reindex.rs:30-38). On a migrated index this recreates legacy
   tables beside CURRENT.json; common.rs:60-63 then misdetects ownership.
4. HIGH: snapshot git reads have no GIT_NO_LAZY_FETCH setting
   (snapshot.rs:111-155,192-231), so a missing blob in a partial clone
   may fetch from the network, violating F04.
5. HIGH: pinned read acquires a lease by writing into the source index
   (generations.rs:184-202). The read-only MCP profile cannot read from
   a read-only indexed mount; F06's no-write expectation fails.
6. HIGH: snapshot republish_current bypasses the tracked write closure
   sync before moving CURRENT (publisher.rs:368-373 vs 422-435).
7. MEDIUM: context pack budgets only excerpt/title/8, not serialized
   payload, omissions or task (context_pack.rs:315-388); violates F08.
8. MEDIUM: new model profile cannot replace an unchanged row carrying an
   old non-null profile_id (jobs.rs:783-801); owner held across inference
   in drain_embedding_queue (facade.rs:1378-1464).
9. MEDIUM: snapshot CLI parses a qualified id but does not apply the
   qualifier before snapshot_get (cli/commands/constellation.rs:537-545).
10. MEDIUM: working scope handles freeze the source list, not captured
    working bytes (mcp/constellation_read.rs:36-45,225-245); subsequent
    reads re-capture and can see changed bytes under the same handle (F03).

## Tests Run

- `cargo fmt --all -- --check`: exit 0.
- `cargo test -p forgeplan-core --lib constellation::context_pack::tests -- --test-threads=1`: exit 0, 4 passed, 0 failed.
- `cargo test -p forgeplan --test cli_constellation_link -- --test-threads=1`: exit 0, 6 passed, 0 failed.
- `cargo test -p forgeplan-core --lib constellation::facade::tests::an_embedding_drain_survives_the_next_reconcile -- --test-threads=1`: exit 0, 1 passed, 0 failed; covers an unchanged source only.

No full workspace test suite or process-level E2E in this audit. Tests cannot
establish the listed failures without targeted regressions. Earlier F10
reports exist under eval/results/, but they predate this review and are
not independent validation of these edge cases.

## Disposition

REQUEST_CHANGES before merge. Write isolated regressions and fix HIGH
findings, then repeat semantic-enabled workspace checks and runtime scenarios
L06-L16/F03-F08. Keep this evidence in draft until fixes are verified.

