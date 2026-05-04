# Dependabot Triage — 2026-05-03 (Round 3, pre-release v0.28.0)

**Sprint:** PR 2 — release v0.28.0 (CLAUDE.md red line #10)
**Triage author:** chore/release-v0.28.0 (autonomous run)
**Branch base:** `dev` @ `136a881` (Merge PR #237 from PR 1 — real E2E closure)
**Previous round:** [`docs/operations/dependabot-triage-2026-05-02.md`](dependabot-triage-2026-05-02.md)
   (Round 2, PRD-073 Phase 3c)

---

## TL;DR

- **18 alerts open on default branch (main)** — re-verified via
  `gh api repos/ForgePlan/forgeplan/dependabot/alerts`. Severity
  breakdown: **5 high, 7 medium, 6 low** — identical к round 2.
- **No new alerts** appeared since 2026-05-02. Existing classification
  from round 2 still holds.
- **16 of 18 alerts auto-close on this `release/v0.28.0 → main` merge**.
  Lockfile в `dev` since round 2 has not regressed (verified — same
  Cargo.lock и `website/package-lock.json`); patches that closed
  postcss / openssl / rand / rustls-webpki / dompurify / astro alerts в
  round 2 still live в the dev branch lockfile.
- **2 carry-forward (accepted-with-justification)**:
  - `lru 0.12.5 < 0.16.3` — transitive via `tantivy → lance → lancedb`
    (semver-major upstream gap, Miri-only soundness issue, no exploit
    surface in normal builds).
  - `uuid <14.0.0` — transitive via `mermaid` (npm); no upstream mermaid
    release с uuid 14.

**Recommended next step:** proceed с `release/v0.28.0 → main` PR. After
merge, GitHub Dependabot will mark all 16 "scheduled" alerts as
resolved automatically. Round 4 triage will fire pre-v0.29.0.

---

## Method

Same procedure as round 2:

1. `gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '[.[] |
   select(.state == "open")]'` — pulled 18 open alerts
2. Severity breakdown via `group_by(.security_advisory.severity)` — confirmed
   5 high / 7 medium / 6 low (matches round 2 numbers exactly)
3. Cross-checked open-alert numbers (`gh api ... --jq '[.[] |
   select(.state=="open") | .number] | sort'`) against round 2 doc's
   alert disposition table — **same 18 alert IDs**, no churn
4. No `cargo update -p` re-attempted because round 2 confirmed lockfile is
   already at the latest reachable patches within current semver constraints
5. Verified `Cargo.lock` un-changed since round 2 commit `5c5a182`
   (2026-05-02): `git diff 5c5a182..HEAD -- Cargo.lock website/package-lock.json`
   produces **0 lines of diff** (verified at triage time, see "Verification
   commands" section). No regression risk introduced by the 14 merge-PRs
   between round 2 and v0.28.0 cut.

---

## Disposition (delta from round 2)

**No deltas.** All 18 alerts retain their round 2 disposition:

| Disposition | Count | Action |
|-------------|------:|--------|
| **scheduled** (auto-closes on next `release/v* → main` merge) | 16 | None — closes automatically when this PR merges |
| **accepted-with-justification** | 2 | `lru` (#3) + `uuid` (#24) — carry forward to next release; re-evaluate when upstream tantivy / mermaid bumps the transitive |

For full disposition table see
[`dependabot-triage-2026-05-02.md`](dependabot-triage-2026-05-02.md)
§"Alert disposition table".

---

## Verification commands (reproducibility)

```bash
# Confirm count
gh api repos/ForgePlan/forgeplan/dependabot/alerts \
  --jq '[.[] | select(.state == "open")] | length'
# → 18

# Severity breakdown
gh api repos/ForgePlan/forgeplan/dependabot/alerts \
  --jq '[.[] | select(.state == "open")] | group_by(.security_advisory.severity)
        | map({severity: .[0].security_advisory.severity, count: length})'
# → [{count:5, severity:"high"}, {count:6, severity:"low"}, {count:7, severity:"medium"}]

# Lockfile check (no regression since round 2)
git diff 5c5a182..HEAD -- Cargo.lock website/package-lock.json | wc -l
# → 0  (zero diff vs round 2 commit; lockfile advanced ONLY between
#    v0.27.0 and round 2, not after — confirms round-2 disposition table
#    1:1 reflects current lockfile state)
```

---

## Post-release validation

After `release/v0.28.0` PR merges на main и tag pushed:

1. `gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '[.[] |
   select(.state == "open")] | length'` should drop from 18 → 2 (the two
   accepted-with-justification carry-forwards).
2. Confirm в release PR description что 16 of 18 alerts auto-closed.
3. Update CHANGELOG.md release note для v0.28.0 with «Dependabot: 16 alerts
   addressed via this release; 2 carry-forward documented».

---

## References

- CLAUDE.md red line #10 — "DO NOT ignore Dependabot alerts at release time"
- [`dependabot-triage-2026-05-02.md`](dependabot-triage-2026-05-02.md) — round 2 (canonical disposition table)
- PR #225 — round 1 (closed 8 of 18 via `cargo update` + `npm audit fix`,
  no markdown triage doc artifact — see PR description for disposition)
- PR #233 — round 2 doc commit (canonical disposition table)
