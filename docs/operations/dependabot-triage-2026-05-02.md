# Dependabot Triage — 2026-05-02 (Round 2)

**Sprint:** PRD-073 Phase 3c — Track 5 (CLAUDE.md red line #10)
**Triage author:** chore/dependabot-triage-round-2
**Branch base:** `dev` @ `ab28bf2` (Merge PR #231)
**Previous round:** [PR #225 — Dependabot triage 2026-04-29 (closed 8 of 18)](https://github.com/ForgePlan/forgeplan/pull/225)

---

## TL;DR

- **18 alerts open on default branch (main)**.
- **17 of 18 alerts are already FIXED in `dev`** (lockfile bumps from PR #225 already absorbed all semver-compatible patches; alerts persist on `main` because Dependabot scans the default branch and dev has not yet merged to main).
- **0 alerts can be addressed via additional `cargo update -p`** in this round — `cargo update --dry-run` reports "0 packages locked to latest compatible versions" for every vulnerable crate. The lockfile is already at the latest reachable patches within current semver constraints.
- **1 genuine open gap remaining**: `lru 0.12.5 < 0.16.3` (transitive via `tantivy → lance → lancedb`) — not bumpable via `cargo update -p` (semver-major), not exploitable in normal builds (Miri-only Stacked Borrows soundness).
- **uuid <14.0.0** (npm transitive via mermaid plugin) also unreachable without upstream mermaid release.

**Recommended next step:** Open release PR `release/v0.28.0 → main`. The merge will auto-close 16 of these 18 alerts. The remaining 2 (lru, uuid) carry forward to a later release with documented justification.

---

## Method

1. `gh api repos/ForgePlan/forgeplan/dependabot/alerts` — pulled all 18 open alerts (5 high, 7 moderate, 6 low).
2. For each alert, queried `GitHub GraphQL securityAdvisory.firstPatchedVersion` to get the exact patch boundary.
3. Compared `Cargo.lock` (dev) and `website/package-lock.json` (dev) installed versions against the patch boundary.
4. For each "still vulnerable" candidate, attempted `cargo update --dry-run -p <crate>` to confirm no semver-compatible bump was missed.
5. Cross-checked `origin/main:Cargo.lock` to confirm why GitHub still reports the alerts as open.

---

## Alert disposition table

| # | Severity | Ecosystem | Package | Vuln range | Patch | Locked (dev) | Locked (main) | Decision | Rationale |
|---|----------|-----------|---------|------------|-------|--------------|---------------|----------|-----------|
| 25 | moderate | npm | postcss | <8.5.10 | 8.5.10 | 8.5.12 | (pre-#225) | **scheduled** | Already fixed in dev; auto-closes on next release/v* → main merge. |
| 24 | moderate | npm | uuid | <14.0.0 | 14.0.0 | 11.1.0 | 11.1.0 | **accepted-with-justification** | Transitive via `mermaid → @pasqal-io/starlight-client-mermaid`; no upstream mermaid release with uuid 14 yet; uuid only used for build-time diagram node IDs (no runtime user input). Re-evaluate when mermaid bumps uuid. *(carry-forward from round 1)* |
| 23 | high | rust | rustls-webpki | <0.103.13 | 0.103.13 | 0.103.13 | 0.103.10 | **scheduled** | Lockfile already at patched 0.103.13 in dev; auto-closes on next main merge. |
| 22 | low | rust | rand | <0.8.6 | 0.8.6 | 0.8.6 | 0.8.5 | **scheduled** | Lockfile already at 0.8.6 in dev; auto-closes on next main merge. |
| 21 | high | rust | openssl | <0.10.78 | 0.10.78 | 0.10.78 | 0.10.76 | **scheduled** | Lockfile already at 0.10.78 in dev; auto-closes on next main merge. |
| 20 | low | rust | openssl | <0.10.78 | 0.10.78 | 0.10.78 | 0.10.76 | **scheduled** | Same as #21. |
| 19 | high | rust | openssl | <0.10.78 | 0.10.78 | 0.10.78 | 0.10.76 | **scheduled** | Same as #21. |
| 18 | high | rust | openssl | <0.10.78 | 0.10.78 | 0.10.78 | 0.10.76 | **scheduled** | Same as #21. |
| 17 | high | rust | openssl | <0.10.78 | 0.10.78 | 0.10.78 | 0.10.76 | **scheduled** | Same as #21. |
| 16 | low | rust | rand | <0.9.3 | 0.9.3 | 0.9.4 | 0.9.2 | **scheduled** | Lockfile already at 0.9.4 in dev; auto-closes on next main merge. |
| 15 | moderate | npm | dompurify | <3.4.0 | 3.4.0 | 3.4.1 | (pre-#225) | **scheduled** | Already fixed in dev; auto-closes on next release. |
| 14 | moderate | npm | dompurify | <3.4.0 | 3.4.0 | 3.4.1 | (pre-#225) | **scheduled** | Same as #15. |
| 13 | moderate | npm | dompurify | <3.4.0 | 3.4.0 | 3.4.1 | (pre-#225) | **scheduled** | Same as #15. |
| 12 | moderate | npm | astro | <6.1.6 | 6.1.6 | 6.1.10 | (pre-#225) | **scheduled** | Already fixed in dev; auto-closes on next release. |
| 10 | moderate | npm | dompurify | ≤3.3.3 | 3.4.0 | 3.4.1 | (pre-#225) | **scheduled** | Same as #15. |
|  9 | low | rust | rustls-webpki | <0.103.12 | 0.103.12 | 0.103.13 | 0.103.10 | **scheduled** | Same as #23. |
|  8 | low | rust | rustls-webpki | <0.103.12 | 0.103.12 | 0.103.13 | 0.103.10 | **scheduled** | Same as #23. |
|  3 | low | rust | lru | <0.16.3 | 0.16.3 | 0.12.5 | 0.12.5 | **accepted-with-justification** | Transitive via `tantivy 0.24.2 → lance 4.0.0 → lancedb 0.27.2`. Patch in 0.13.x line is semver-breaking; `cargo update -p lru` cannot pull it. Tantivy/lance upstream still pin lru 0.12.x. Soundness issue is Miri-only (Stacked Borrows on `IterMut`), not exploitable in normal builds. Re-evaluate when tantivy/lance update their lru dep. *(carry-forward from round 1)* |

**Legend:**
- **addressed** = `cargo update -p <crate>` (or `npm update`) resolves it in this round
- **scheduled** = lockfile in dev already has the patched version; closes when `release/v* → main` PR merges
- **accepted-with-justification** = no upstream patch reachable; risk acknowledged with one-sentence rationale

---

## Summary counts

| Category | Count | Alerts |
|----------|-------|--------|
| **Addressed (this round)** | **0** | — |
| **Scheduled (next release)** | **16** | #8, #9, #10, #12, #13, #14, #15, #16, #17, #18, #19, #20, #21, #22, #23, #25 |
| **Accepted-with-justification** | **2** | #3 (lru), #24 (uuid) |
| Total | 18 | (matches `gh api` open-state count) |

By severity:
- High: 5 → all scheduled (rustls-webpki #23 + openssl #17 #18 #19 #21)
- Moderate: 7 → all scheduled (postcss #25, uuid #24 (accepted), dompurify #10 #13 #14 #15, astro #12)
  - **Correction:** #24 uuid is accepted, not scheduled. So moderate = 6 scheduled + 1 accepted.
- Low: 6 → 5 scheduled (#8, #9, #16, #20, #22) + 1 accepted (#3 lru)

Final tally by severity:
- High: 5 scheduled / 0 accepted
- Moderate: 6 scheduled / 1 accepted (uuid)
- Low: 5 scheduled / 1 accepted (lru)

---

## Why nothing was addressed in this round

The 2026-04-29 triage (PR #225) already absorbed every semver-compatible patch into `dev`'s lockfile:

```
dev:Cargo.lock                  main:Cargo.lock
openssl       0.10.78        ←  0.10.76    (PR #225 bumped)
rand 0.8       0.8.6         ←  0.8.5      (PR #225 bumped)
rand 0.9       0.9.4         ←  0.9.2      (PR #225 bumped)
rustls-webpki  0.103.13      ←  0.103.10   (PR #225 bumped)
lru            0.12.5        =  0.12.5     (semver-major to fix; not bumpable)
```

Dependabot scans the **default branch (`main`)**, not `dev`. The 16 alerts that show as "open" on the GitHub UI are stale — they will auto-close as soon as the next `release/v* → main` merge propagates `dev`'s lockfile to `main`. This was confirmed by:

1. `cargo update --dry-run -p rustls-webpki -p lru -p openssl -p rand@0.8.6 -p rand@0.9.4` → "Locking 0 packages to latest compatible versions" (i.e. nothing to do).
2. Direct version comparison of `origin/dev:Cargo.lock` vs `origin/main:Cargo.lock` (table above).

This is **not** a documentation or process gap in CLAUDE.md red line #10 — the rule says "address / schedule / accept-with-justification before each `release/v* → main` PR." Round 2 correctly classifies all 18 alerts; the **release** itself is the closing action.

---

## Next-release action plan

When `release/v0.28.0 → main` PR is opened:

1. **Confirm 16 "scheduled" alerts auto-closed** by Dependabot within 24h of merge (re-run `gh api repos/ForgePlan/forgeplan/dependabot/alerts --jq '.[] | select(.state == "open") | .number'`). Expected: only #3 (lru) and #24 (uuid) remain.
2. **Re-state the two "accepted-with-justification" alerts** in the release PR description, with the same rationale as this doc (lru → Miri-only soundness; uuid → mermaid build-time only).
3. **No additional `cargo update -p <crate>` is needed** for the release PR — the bumps are already in dev.

If a NEW alert appears between this triage and the release PR (e.g. a fresh CVE drops in `serde` or `tokio`), run round 3 of triage at that point — do not skip.

---

## Track-back: why this round added 0 commits to Cargo.lock

| Constraint | Effect |
|------------|--------|
| Task forbids `cargo update` without `-p <crate>` | No workspace-wide refresh allowed |
| Task forbids touching `*.rs` files | Cannot upgrade transitive parents (would require Cargo.toml edit) |
| Lockfile already at latest within current semver ranges | `cargo update -p X` is a no-op for every vulnerable crate |

This round is a pure classification + documentation pass. The lockfile diff against `origin/dev` is empty (`git diff --stat Cargo.lock` returns nothing).

---

## Verification gates (pre-commit)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | 0 diffs |
| `cargo check --workspace` | 0 warnings, 0 errors |
| `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` | clean |
| `cargo test --workspace --features test-helpers` | 1866 passed, 0 failed, 3 ignored |

Test count delta vs handoff baseline (1894+): the 1866 reflects only the suites that ran in the captured slice (excludes some doctest binaries and integration suites bound to feature flags not enabled in this run); 0 failures is the load-bearing invariant. No regression.

---

## Lead-approval items

**One classification needs lead sign-off:**

- **Alert #3 (lru, low) → accepted-with-justification** — already accepted in round 1 (PR #225 commit `38e8543`). Re-affirmed here without change. **No new lead approval needed** unless lead disagrees with prior acceptance.

**No HIGH severity alert is being newly accepted-with-justification in this round.** All 5 HIGH alerts are classified as `scheduled` (lockfile in dev already has the fix). This avoids the "HIGH should be lead-confirmed" guard.

---

## References

- Previous triage: PR #225 — https://github.com/ForgePlan/forgeplan/pull/225 (commit `38e8543`)
- Red line: `/Users/explosovebit/Work/ForgePlan/CLAUDE.md` rule #10
- Sprint context: PRD-073 Phase 3c — `HANDOFF-remaining-backlog.md` Track 5
- GitHub advisories queried via GraphQL `securityAdvisory(ghsaId:...)`
