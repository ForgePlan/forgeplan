# Release workflow

End-to-end recipe for cutting a Forgeplan release. Follow these steps in
order — they reflect what every successful release in `git log --oneline
--all | grep release/v` actually did, plus the two CLAUDE.md red lines
(#9 post-release sync, #10 dependabot triage at release time) we keep
forgetting and re-learning.

> Audience: maintainers cutting a release. Not for one-off feature merges
> — those use the simpler `feat/* → dev` PR flow described in `git/`.

---

## 0. Pre-conditions

A release ships when **all** of the following are true on `dev`:

- `cargo fmt -- --check` clean
- `cargo check --workspace` 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace --features test-helpers` 0 failures (1940+ tests as of v0.28.0)
- Real E2E smoke on a fresh workspace covers the surfaces that changed
  in this minor (CLAUDE.md red line #5 — automated tests verify code
  correctness, not feature correctness)
- `forgeplan health` reports 0 blind spots, 0 stale, no advisory phase
  mismatches
- `git log origin/main..origin/dev` shows a non-empty diff (no point
  releasing if dev hasn't diverged)

**Canonical playbook (v0.28.0+)**: `marketplace/playbooks/release.yaml`
кодифицирует pre-merge часть этого workflow как 12-step playbook
(preflight cargo gates → dependabot triage → CHANGELOG check → branch +
version bump → release PR creation → release-summary Note). На текущей
схеме SPEC-003 1.2 нет template engine, поэтому maintainer вручную
правит `vX.Y.Z` placeholder в step args перед `playbook run release --yes`
(tracked как PROB-050 A-1). Запуск playbook'а автоматизирует pipeline
ниже до шага 9; шаги 10+ (post-merge tag + sync PR) остаются
ручными.

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

**Merge strategy: merge commit (NOT squash)**. Squash collapses the
release commit's history with all the dev commits accumulated since the
last release, losing per-commit attribution and breaking `git bisect`
for post-release regressions. This is CLAUDE.md red line #4 (DO NOT
push to a branch after a PR is merged — squash loses late commits)
applied to release branches specifically.

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
- Creates a GitHub release with all 28 artifacts (5 binaries × 2 archive
  formats × {sig, sha256}, plus tarball + checksums manifest)
- Updates the Homebrew formula in the `forgeplan-tap` repo

**Verify within 30 minutes**:

```bash
gh release view vX.Y.Z --repo ForgePlan/Forgeplan
brew update && brew info forgeplan
forgeplan --version  # should match X.Y.Z after `brew upgrade forgeplan`
```

If cargo-dist fails, the most common cause is a flaky linker job on the
windows-msvc target — re-run via `gh run rerun <run-id> --failed`
before debugging.

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

- **Force-pushing release/* branches** — rewrites the merge commit hash
  GitHub Actions tagged. Result: cargo-dist runs against a SHA that
  doesn't exist in the published tag. Recover by re-tagging from the
  actual merge SHA.
- **Squashing release/* PRs** — collapses N feature merges into a single
  commit with the release subject; `git bisect` between releases stops
  working.
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
