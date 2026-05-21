# ADR-013: CI Security Gate Policy — PR Trigger vs Push-Only Timing

**Status:** Accepted

**Date:** 2026-05-14

**Scope:** `security.yml` workflow trigger configuration for cargo-deny CVE/advisory scanning.

---

## Context

The `security.yml` workflow runs `cargo-deny check advisories licenses bans sources` to detect CRITICAL/HIGH CVE findings in dependencies before merge. During v0.31.0 Wave 9 adversarial audit (May 12, 2026), finding **SEC-004** flagged a policy ambiguity: the workflow has `continue-on-error: true`, allowing PRs to show a green checkmark even when cargo-deny detects CRITICAL CVEs. Additionally, the workflow triggers on three events:

1. `push: [dev, main]` — lands on committed code
2. `pull_request: [dev, main]` — runs on every PR targeting dev/main
3. `schedule: "23 7 * * 1"` — weekly cron scan

**The Problem:**

- **Option A (hard gate)**: Removing `continue-on-error: true` makes CVE detection a merge blocker. Trade-off: every dependabot PR touching a newly-CVE'd dependency is blocked until manual triage (via `deny.toml::[advisories.ignore]`). With automated dependabot grouping (configured by W4), this creates friction on every security patch release.
  
- **Option B (remove PR trigger)**: Drop the `pull_request:` event, keep only `push:` and `schedule:`. Workflow runs on landed commits (to dev/main) + weekly cron. PRs no longer carry a "security ✓" badge. Trade-off: CVE detection lag — issues found post-merge on dev branch. Mitigated by: weekly cron catches new advisories within 7 days, and `dev` → `main` is gated by release sprint anyway.

---

## Decision

**Implement Option B: Remove the `pull_request:` trigger from `security.yml`.**

The workflow will run on `push: [dev, main]` and `schedule:` only.

### Rationale

1. **Honest Signal:** A green CI checkmark on a PR should mean "this PR is safe to merge from a CVE standpoint," not "we haven't audited your dependencies yet." Removing the PR trigger prevents false confidence. Reviewers will understand that `dev` is a triage queue where CVE findings are discovered post-merge, not a finished production branch.

2. **Practical Friction Reduction:** Dependabot PRs are frequent (W4 configured grouping reduces noise but still 2-4 PRs/week for patch/minor updates). Under Option A, every PR with a new CVE advisory blocks merge, forcing manual ignore-list entries per PR. Option B amortizes this to the dev triage window.

3. **Safety Not Compromised:** The weekly `schedule:` cron catches new advisories within 7 days. For a branch-based release model (feature → dev, dev → main on release cycles), a 7-day triage window is acceptable and documented in the workflow header.

4. **Release-Time Validation:** RED LINE #10 in CLAUDE.md mandates CVE audit at release time. The push trigger to `main` ensures the release branch gets scanned immediately. PRs targeting dev do not need the extra scan because `dev` → `main` PR is the release commit that WILL be scanned.

5. **Alignment with Methodology:** The project workflow treats `dev` as a staging/triage branch, not production-ready code. Moving security findings to post-merge `dev` discovery fits this model.

---

## Changes

1. **`.github/workflows/security.yml`:**
   - Remove `pull_request:` from `on:` section.
   - Keep `push: [dev, main]` and `schedule:` intact.
   - Update workflow header comment to document the timing.

2. **This ADR:**
   - Codifies the policy so future tool-chain changes (e.g., dependabot config, release process) don't regress.

---

## Consequences

### Positive
- PRs no longer show false "security ✓" if dependencies have known CVEs.
- Reduced friction on dependabot PRs (no per-PR triage bottleneck).
- Clearer intent: `dev` is a merge queue, not a vetted branch.
- Weekly scan still catches emerging advisories within a predictable window.

### Negative
- CVE findings on dev may not be discovered until 24h–7d after merge (vs 0s on PR).
  - **Mitigation:** Weekly cron runs Monday 07:23 UTC; PRs typically land Mon–Fri, so max wait is ~7 days. For critical advisories (RUSTSEC-YYYY-XXXXX marked CRITICAL), developers can manually run the workflow on the dev branch via GitHub Actions UI if they suspect a match.

- Release-blocking CVEs discovered on dev require a `dev → main` PR to be filed and scanned before release. This is the intended gate.

---

## Reversibility

If the team decides to revert to PR-time scanning:
1. Re-add `pull_request: [dev, main]` to the `on:` section.
2. Optionally add a hard gate by removing `continue-on-error: true`, or re-enable it as a warning-only gate.
3. Add clear documentation on the ignore-list process for false positives.

---

## Related Artifacts

- **SEC-004 (v0.31 audit finding):** Original flagged the `continue-on-error` + PR-trigger ambiguity.
- **RED LINE #10 (CLAUDE.md):** Mandates dependabot alert review at release time.
- **PR #278 (v0.31.0):** Closed PROB-063, PROB-064, and this audit closure.
- **Wave 9 Audit (May 12–13, 2026):** 5-agent panel found 19 findings; SEC-004 was deferred to v0.32.

---

## FAQ

**Q: What if a critical CVE lands on `dev` and is not discovered until the weekly scan?**

A: The project maintainer can:
1. Manually trigger the workflow from the GitHub Actions UI via the `workflow_dispatch` event (configured in `security.yml::on` — the "Run workflow" button appears in the Actions tab).
2. Alternatively, push a no-op commit to `dev` to trigger the `push` event immediately.
3. If a CRITICAL CVE is confirmed, file an emergency patch PR to `main` or revert the offending dependency.

**Q: What about `continue-on-error: true`?**

A: Removed. It was inherited from the pre-ADR-013 configuration but contradicts the "honest CI signal" trade-off this ADR was meant to encode: with `continue-on-error: true` the workflow conclusion is always success regardless of cargo-deny exit code, so a CRITICAL advisory landing on `dev` would not turn the security badge red. The v0.32 Wave 4 audit (security review of this branch) flagged this as a CRITICAL finding (S1) — the gate was effectively cosmetic. Fixed in the same commit that publishes this ADR amendment: the workflow now lets cargo-deny's exit propagate, and the badge will reflect actual advisory state.

**Q: Won't this regress our security posture?**

A: No. Security posture is maintained by the weekly scan and the release-time gate (push to main). We are trading per-PR instant feedback for reduced friction and honest signal clarity.
