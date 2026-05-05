# Dependabot Triage — 2026-05-05 (Round 4, pre-release v0.29.0)

**Sprint:** v0.29.0 release prep (CLAUDE.md red line #10)
**Triage author:** chore/release-v0.29.0 W5
**Branch base:** `feat/prob-050-claude-print-refactor` (PR #247 awaiting merge to dev)
**Previous round:** [`docs/operations/dependabot-triage-2026-05-03.md`](dependabot-triage-2026-05-03.md)
   (Round 3, pre-release v0.28.0)

---

## TL;DR

- **1 alert open on default branch (main)** — verified via
  `gh api repos/ForgePlan/forgeplan/dependabot/alerts`. Severity
  breakdown: **0 critical, 0 high, 0 medium, 1 low**.
- **17 alerts auto-closed** with the `release/v0.28.0 → main` merge as
  predicted by round 3 disposition (16 scheduled + 1 of 2
  accepted-with-justification self-resolved upstream — `uuid` chain
  cleared via mermaid bump in `npm audit fix` cycle).
- **1 carry-forward**: `lru 0.12.5` (#3) — same disposition as rounds 2 & 3.
- **No new alerts** appeared since 2026-05-03.

**Recommended next step:** proceed с `release/v0.29.0 → main` PR.
The single remaining `lru` alert is **accepted-with-justification** and
documented in CHANGELOG.

---

## Single open alert — full record

| Field | Value |
|---|---|
| Alert # | [3](https://github.com/ForgePlan/forgeplan/security/dependabot/3) |
| Severity | low |
| CVSS | **0.0** (informational) |
| GHSA | `GHSA-rhfx-m35p-ff5j` |
| Package | `lru` (Cargo) |
| Affected range | `>= 0.9.0, < 0.16.3` |
| Patched | `0.16.3` |
| Scope | runtime |
| Manifest | `Cargo.lock` (transitive only) |
| Summary | `IterMut` violates Stacked Borrows by invalidating internal pointer |
| Direct consumer? | **No** — transitive via `tantivy 0.24.2 → lance 4.0.0 → lancedb 0.27.2 → forgeplan-core` |

### Why `accepted-with-justification`

1. **CVSS 0.0** — Miri / Stacked Borrows soundness issue, не runtime
   exploit. No CVE assigned.
2. **Transitive only** — Forgeplan не объявляет `lru` direct в
   `Cargo.toml`. Bump требует upstream работы в `tantivy` (или его
   собственный transitive bump через `lru` 0.16.x). `cargo update -p lru`
   blocked by semver constraint в tantivy 0.24.2 (`lru = "^0.12"`).
3. **No exploit surface** — `IterMut` API из `lru` вызывается только
   `tantivy::indexer` для cache eviction; продакшен path не обнажает
   raw pointers пользователю.
4. **Tracked upstream** — see
   [tantivy issue tracker](https://github.com/quickwit-oss/tantivy)
   for `lru` bump cadence; we will re-evaluate when tantivy 0.25 lands.

---

## Method (same as round 3)

```bash
gh api repos/ForgePlan/forgeplan/dependabot/alerts --paginate \
  --jq '[.[] | select(.state == "open")] | length'
# → 1

gh api repos/ForgePlan/forgeplan/dependabot/alerts --paginate \
  --jq '[.[] | select(.state == "open")] | group_by(.security_advisory.severity)
        | map({severity: .[0].security_advisory.severity, count: length})'
# → [{count:1, severity:"low"}]

cargo tree -i lru
# → lru 0.12.5
#     └── tantivy 0.24.2
#         ├── lance 4.0.0 → lancedb 0.27.2 → forgeplan-core
#         └── lance-index 4.0.0 (same chain)
```

---

## Disposition

| Disposition | Count | Action |
|---|---:|---|
| **accepted-with-justification** | 1 | `lru` (#3) — carry forward; re-eval when tantivy 0.25 ships |
| scheduled | 0 | — |
| addressed (closed by lockfile bump) | 0 | — |

---

## CHANGELOG entry (proposed)

```markdown
### Security — Dependabot

- 17 of 18 alerts from v0.28.0 round 3 closed automatically with the
  v0.28.0 → main merge (rust patches + npm audit fix cycle).
- 1 alert remains open: `lru 0.12.5` ([Alert #3], CVSS 0.0,
  Miri-only stacked-borrows in `IterMut`). Transitive via
  `tantivy → lance → lancedb`; no direct exploit surface. Carry-forward
  to v0.30.0 — accepted-with-justification, re-evaluate on
  tantivy 0.25 release.
```

---

## Post-release validation

After `release/v0.29.0` PR merges на main и tag pushed:

1. `gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '[.[] |
   select(.state == "open")] | length'` should remain `1` (lru
   carry-forward).
2. Confirm в release PR description что the only open alert is
   accepted-with-justification.
3. Round 5 triage fires pre-v0.30.0 — re-check upstream tantivy
   release cadence.

---

## References

- CLAUDE.md red line #10 — "DO NOT ignore Dependabot alerts at release time"
- [`dependabot-triage-2026-05-03.md`](dependabot-triage-2026-05-03.md) — round 3 (canonical disposition table)
- [`dependabot-triage-2026-05-02.md`](dependabot-triage-2026-05-02.md) — round 2
- GHSA-rhfx-m35p-ff5j — `lru` advisory
