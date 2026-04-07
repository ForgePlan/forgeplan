---
depth: standard
id: PROB-026
kind: problem
links:
- target: EPIC-003
  relation: informs
- target: PRD-035
  relation: based_on
- target: EVID-060
  relation: informs
status: draft
title: Sprint 13.3 deferred M/L audit findings — tag canonicalization, char validation, CLI display polish
---

# PROB-026: Sprint 13.3 deferred M/L audit findings

## Signal

Sprint 13.3 multi-agent audit (Rust + Security + Architecture + Test Coverage) found 12 MEDIUM + 13 LOW findings beyond the 2 CRITICAL + 5 HIGH that were fixed in W5. Per methodology guideline (fix all HIGH/CRITICAL, defer rest to follow-up), these were documented but not addressed. This problem tracks them so they don't get lost.

## Context

EVID-060 captures the full sprint audit cycle. This problem is the backlog for the "defer" bucket.

## Deferred findings (categorized)

### Tag canonicalization (Rust M6, Security M3, Architecture M1)

1. **Tag normalization inconsistency** — `add_tags` trims whitespace, but `create_artifact` via `NewArtifact.tags` writes literal strings. Same tag with different whitespace stores as duplicates on mixed paths.
2. **Tag character validation missing** — CLI accepts arbitrary input: NUL bytes, newlines, 10MB strings, 10k tags per artifact. No limits.
3. **YAML injection via tags** — if tag contains `"\n---\nstatus: active"`, projection write could break frontmatter (mitigated by current serde_yaml usage but not defensive).
4. **Case sensitivity policy undefined** — `source=Code` vs `source=code` stored as distinct. Need explicit policy: case-insensitive keys, case-sensitive values?
5. **Multi-`=` ambiguity** — `"foo=bar=baz"` parses key="foo", value="bar=baz". Document or reject.
6. **Stringly-typed Vec<String>** — future v2 should consider `Vec<Tag { key, value }>`.

### List_by_tag performance (Rust H4→M, Security M2)

7. **Full-table scan + Rust filter** — `list_by_tag` materializes entire corpus (including body + embedding!). At 1000+ artifacts this is O(n × body_size). Should push filter down via LanceDB `array_contains` predicate.
8. **No early-out on huge workspaces** — hard cap needed.

### Stub detection + duplicate guard (Security L1, L2)

9. **Duplicate guard interaction with tags untested** — tags are not part of dedup key (correct), but no regression test.
10. **Stub detection on tagged artifacts untested** — same.

### CLI / UX polish (Architecture L2, Rust L3)

11. **list --tag accepts only one tag** — `Option<String>` not `Vec<String>`. Can't compose `--tag source=code --tag layer=auth`.
12. **Tag/Untag vs subcommand group** — current top-level flat commands vs `tag add/remove/list` group for future growth.
13. **No CLI tag column in default list output** — requires `--json` to see tags.
14. **remove_tags silent no-op** — `run_remove` reports "Removed N tag(s)" based on input count not actual delta.

### Tracing / observability (Rust M5, Security L4, H2 partial)

15. **eprintln! instead of structured log** — SourceTier precedence warning uses stderr; invisible in MCP mode.
16. **Migration has no "export first" warning** — CLAUDE.md mandates export before reinit but normal startup runs migration eagerly.
17. **No audit log for force bypasses** — --force activate, --allow-duplicate, import-force all bypass silently.

### Test gaps (Test Coverage audit)

18. **Missing migration test for partial-failure recovery** — what if add_columns fails mid-fragment?
19. **Case sensitivity not asserted** — test should pin policy decision.
20. **Unicode / special-char tag content** — no robustness test.
21. **Concurrent add_tags race test** — documented but not tested.
22. **Schema v3→v4 + existing rows** — H4 test covers this, but preservation of values across migration not asserted.
23. **StubReport.count exact value** — tests assert `>= 3` not exact.

### Architecture (M2, M3, M4)

24. **SourceTier placement** — currently in `scoring/evidence.rs` but semantically a provenance concern belonging in `artifact/`.
25. **Three duplicate Frontmatter builders** — residual after Sprint 13.1.5 consolidation; add `ArtifactRecord::frontmatter_map()` usage everywhere.

## Constraints

- Must not break existing Sprint 13.3 behaviour
- Follow-ups should be grouped logically (not 25 tiny PRs)
- Tag canonicalization changes need migration path (existing stored tags might be non-canonical)

## Optimization Targets

1. **Harden tag input validation** — 5-10 LOC in run_add + test suite (~30 min)
2. **Push list_by_tag filter to LanceDB** — requires lancedb `array_contains` verification (~1h)
3. **Structured logging for bypass events** — add tracing dep or custom event log (~2h)
4. **Migration resilience tests** — simulate failure modes (~1h)

## Acceptance Criteria

1. All 25 deferred findings triaged into: (a) fix in Sprint 13.3.x hardening, (b) fold into Sprint 13.4 PRD-035 p2, (c) punt to v0.18
2. Each category has a tracking artifact (NOTE or small PRD)
3. PROB-026 activated only when all 25 items resolved or explicitly accepted

## Blast Radius

- crates/forgeplan-core/src/db/store.rs (add_tags validation)
- crates/forgeplan-core/src/db/migrate.rs (partial-failure tests)
- crates/forgeplan-cli/src/commands/tag.rs (char validation)
- crates/forgeplan-cli/src/commands/list.rs (multi-tag support)

## Reversibility

**High** — all deferred items are improvements, not fixes for broken behaviour. Each can be addressed incrementally without coordinated rollback.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-035 | based_on (Sprint 13.3 p1 of this PRD) |
| EVID-060 | informs (full audit results captured) |
| EPIC-003 | informs (Sprint 13 series) |
