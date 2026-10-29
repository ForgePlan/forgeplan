# Release workflow

Maintainer runbook for cutting a Forgeplan release. Its review gates are project
conventions, not evidence that a release has passed them. Dated test counts and
past incident references retain their historical scope. CLAUDE.md red lines
#9 (post-release sync) and #10 (Dependabot triage) remain in force.

> Audience: maintainers cutting a release. Not for one-off feature merges
> — those use the simpler `feat/* → dev` PR flow in [GIT-WORKFLOW.ru.md](../operations/GIT-WORKFLOW.ru.md).

---

## 0. Pre-conditions

A release ships when **all** of the following are true on `dev`:

- `cargo fmt -- --check` clean
- `cargo check --workspace` 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace --features test-helpers` 0 failures (1977+ tests as of v0.29.0)
- Real E2E smoke on a fresh workspace covers the surfaces that changed
  in this minor (CLAUDE.md red line #5 — automated tests verify code
  correctness, not feature correctness)
- `forgeplan health` reports 0 blind spots, 0 stale, no advisory phase
  mismatches
- `./scripts/check-mcp-tool-count.sh` exits 0 — no drift between MCP tool
  count in source and documentation (drift detector, v0.28.0+, PROB-050 A-30).
  If the release adds or removes MCP tools, update all documentation locations
  first, then run the script to verify.
- `git log origin/main..origin/dev` shows a non-empty diff (no point
  releasing if dev hasn't diverged)

Full CI gate reference (what each check does and how to fix failures):
[`docs/operations/QUALITY-GATES.ru.md`](../operations/QUALITY-GATES.ru.md)

**Canonical playbook (v0.28.0+)**: `marketplace/playbooks/release.yaml` contains
12 pre-merge steps (preflight → triage listing → CHANGELOG check → branch/version
bump → commit/push/PR → summary Note). It is a runbook, not one-button acceptance.
Inspect its actual `schema_version` and command arguments; hand-edit `vX.Y.Z`
and `X.Y.Z` before an authorized `forgeplan playbook run release --yes --allow-shell`.
The shell gate is default-deny. The playbook's ordinary `health` invocation does
not establish zero findings, and triage listing does not classify/accept alerts.
Manual gates above still apply. Sections 5–7 below (tag, sync, memory/tracker)
remain separate post-merge work, not automatically completed steps.

If any of these fails, fix on `dev` (PR through `feat/*`) before
opening the release branch.

---

## 1. Dependabot triage gate (CLAUDE.md red line #10)

Run **before** opening the release branch:

```bash
gh api repos/ForgePlan/Forgeplan/dependabot/alerts --jq '.[] | select(.state == "open") | {number, severity: .security_advisory.severity, package: .dependency.package.name, summary: .security_advisory.summary}'
```

For each open alert, classify into one of three buckets and record the
choice in the release notes (Section 5):

- **addressed** — `cargo update -p <crate>` or `npm audit fix` resolves it
  in this release. Include the resolution commit SHA.
- **scheduled** — fix planned for the next release. Reference the
  tracking issue or PRD.
- **accepted-with-justification** — risk accepted (e.g., dev-only
  dependency, no exploit path in our usage). One sentence of why.

Skipping this step is the failure mode that surfaced in PR #225 — alerts
silently accumulate over multiple releases until a forced cleanup sprint.

---

## 2. CHANGELOG entry on `dev`

Open a PR `chore/changelog-vX.Y.Z` from `dev` (or stack on whatever
last feature merged). The entry follows the loose Keep-a-Changelog
format already in `CHANGELOG.md`:

```markdown
## [X.Y.Z] — YYYY-MM-DD — <one-line theme>

<2–3 sentence description of what this minor delivers>

### Added — <capability> (PRD-XXX / RFC-YYY / ADR-ZZZ)
- Bullet 1 (commit refs optional)
- Bullet 2

### Changed
- ...

### Fixed
- ...

### Migration notes
- ...
```

**Reference the active artifacts** in section headings — readers should
be able to jump from CHANGELOG to the PRD/RFC/ADR without searching.

---

## 3. Version bump on `dev`

Bump `version = "X.Y.Z"` in the workspace `Cargo.toml`. Run
`cargo update --workspace` so `Cargo.lock` reflects the new version
across all crates. Commit:

```
chore(release): bump workspace version to X.Y.Z
```

This commit lands on `dev` via the same `chore/changelog-vX.Y.Z` PR
(or its own `chore/version-bump-vX.Y.Z` PR — convention varies, just
make sure both land before opening the release branch).

---

## 4. Release branch + PR to main

```bash
git checkout dev && git pull
git checkout -b release/vX.Y.Z
git push -u origin release/vX.Y.Z
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z" --body "$(cat <<'EOF'
## Summary

<copy CHANGELOG entry>

## Verification

- cargo test --workspace: NNNN passed / 0 failed
- cargo clippy: clean
- Real E2E smoke: <list surfaces verified>
- forgeplan health: clean

## Dependabot triage (red line #10)

- addressed: <PR/SHA references>
- scheduled: <next release>
- accepted-with-justification: <one-line reason>
EOF
)"
```

**Merge strategy: merge commit (NOT squash)**, per the project release convention.
Squash loses individual commit identities and intra-release bisect granularity;
it does not itself omit changes already present in the merged head. The separate
rule against pushing late commits to a merged branch still applies.

If the release branch needs an update from main during review (rare,
e.g., a hotfix on main mid-review), `git merge origin/main` into the
release branch — never rebase a release branch.

---

## 5. Tag on main + cargo-dist + brew

After the release PR merges:

```bash
git checkout main && git pull
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

The tag triggers GitHub Actions (`.github/workflows/release.yml`,
configured by `cargo-dist`) which:

- Builds binaries for 5 platforms (aarch64/x86_64 × {apple-darwin,
  unknown-linux-gnu} + x86_64-pc-windows-msvc)
- Publishes the artifacts listed in that run's cargo-dist manifest; the asset count and archive formats are not a fixed 28-item contract
- Updates the Homebrew formula in `ForgePlan/homebrew-tap`

**Verify within 30 minutes**:

```bash
gh release view vX.Y.Z --repo ForgePlan/Forgeplan
brew update && brew info forgeplan
forgeplan --version  # should match X.Y.Z after `brew upgrade forgeplan`
```

If cargo-dist fails, inspect the actual failed job and logs before deciding
whether retry is appropriate. No measured "most common" failure cause is
established by this runbook.

---

## 6. Post-release sync main → dev (CLAUDE.md red line #9)

This step is **mandatory** and the most-forgotten one. Branch protection
blocks direct push to `dev`, so without this PR the version bump and
tag commits on `main` never propagate back to `dev`. The next release
attempt then fights merge conflicts on `Cargo.toml`, and any feature
PR to `dev` opened in the gap shows phantom commits.

```bash
git checkout main && git pull
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z
git merge --no-edit origin/dev || true   # fast-forward when possible; resolve if needed
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z \
  --title "chore: sync main to dev after vX.Y.Z" \
  --body "Post-release sync per CLAUDE.md red line #9. Brings vX.Y.Z release commit + tag context back to dev so the next release branch starts from a consistent base."
```

PR #223 (sync after v0.27.0) is the canonical example to copy.

Merge with **merge commit** (same reasoning as the release PR — preserve
attribution).

---

## 7. Memory + Orchestra sync

After everything above:

- `memory_retain` an observation: "Forgeplan vX.Y.Z released — <highlights>"
  with entities tagged for the headline PRDs/RFCs.
- Update Orchestra task statuses for any items in `Doing`/`Review` that
  this release shipped → `Done`, phase `Done`.
- If your CLAUDE.md or other docs reference `vN-1`-specific behavior
  that's now changed, update them in the next `feat/*` PR (don't
  pile this onto the release).

---

## Hotfix flow (vX.Y.Z+1)

For an urgent fix that can't wait for the next minor:

1. Branch from `main` (NOT dev): `git checkout main && git pull && git checkout -b fix/<short-name>`
2. Apply minimal fix + test
3. PR to `main` directly (skip `dev` for urgency); label `hotfix`
4. After merge: bump patch version on `main` → tag → release as
   sections 5–6 above
5. **Backport to dev**: open a `chore/backport-vX.Y.Z+1-to-dev` PR
   from main. Either cherry-pick the fix commit or merge the entire
   release/vX.Y.Z+1 ref into dev. Without this, dev silently regresses.

---

## Anti-patterns we have committed (don't repeat)

- **Force-pushing release/* branches** — rewrites branch history. Existing tags
  still reference their original objects; a branch force-push does not rewrite
  a published tag. Diagnose the exact branch/tag SHA mismatch before recovery;
  do not move a published tag as a routine fix.
- **Squashing release/* PRs** — loses individual feature-commit identities and
  intra-release bisect granularity; bisect between retained release commits
  remains possible.
- **Skipping the main → dev sync** — see CLAUDE.md red line #9.
  Symptom: next release branch has unexpected `Cargo.toml` conflicts.
- **Letting Dependabot alerts age across releases** — see red line #10.
  Symptom: an emergency security audit interrupts feature work later.
- **Tagging from `main` before the release PR merges** — tag on a
  ghost SHA. Always wait for the merge to complete and `git pull main`
  before tagging.

---

## Appendix: command summary (copy/paste)

```bash
# Pre-flight (on dev)
cargo fmt -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
forgeplan health
gh api repos/ForgePlan/Forgeplan/dependabot/alerts --jq '.[] | select(.state=="open")'

# Version bump + CHANGELOG
$EDITOR Cargo.toml CHANGELOG.md
cargo update --workspace
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): bump workspace version to X.Y.Z"
git push

# Release PR
git checkout -b release/vX.Y.Z dev
git push -u origin release/vX.Y.Z
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z" --body "..."

# After release PR merges
git checkout main && git pull
git tag -a vX.Y.Z -m "Release vX.Y.Z" && git push origin vX.Y.Z

# Post-release sync (mandatory)
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z main
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z --title "chore: sync main to dev after vX.Y.Z"
```
