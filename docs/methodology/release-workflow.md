# Release workflow

End-to-end recipe for cutting a Forgeplan release. Follow these steps in
order — they reflect what every successful release in `git log --oneline
--all | grep release/v` actually did, plus the two CLAUDE.md red lines
(#9 post-release sync, #10 dependabot triage at release time) we keep
forgetting and re-learning.

> Audience: maintainers cutting a release. Not for one-off feature merges
> — those use the simpler `feat/* → dev` PR flow described in `git/`.

**Owner-consent checkpoint — before remote writes or a publishing playbook:**
obtain explicit approval covering the named preparation/release branch
pushes and PR creation, tag publication (and its cargo-dist release),
and sync/backport branch pushes and PR creation. Pause if an intended
effect is outside that approval. This applies to the appendix as well.
Approve a release playbook's remote effects **before launching it**;
`--yes` and `--allow-shell` acknowledge local execution, not publication
authority. Merging remains subject to CI, review, and repository policy.

---

## 0. Pre-conditions

A release ships when **all** of the following are true on `dev`:

- `cargo fmt -- --check` clean
- `cargo check --workspace` 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo nextest run --workspace --all-targets` and `cargo test --workspace --doc` pass; the historical baseline was 1977+ tests at v0.29.0
- Real E2E smoke on a fresh workspace covers the surfaces that changed
  in this minor (CLAUDE.md red line #5 — automated tests verify code
  correctness, not feature correctness)
- CI checks and lints both default and `semantic-search` configurations;
  its separate embedding oracle runs against cached model weights
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

**Repository playbook:** `marketplace/playbooks/release.yaml` is a local
12-step runbook, not a one-button release service or a bundled public
marketplace skill. Inspect it with `forgeplan playbook show release`.
Replace both `vX.Y.Z` and `X.Y.Z` placeholders before running. Its command
steps require `forgeplan playbook run release --yes --allow-shell`
(or trusted workspace `playbook.allow_shell` configuration).

Do not treat a successful run as verification of all conditions above.
The YAML lists Dependabot alerts but does not enforce their classification;
plain `health` is informational; its CHANGELOG search can match bullets
in later sections. Review the four path-dependency versions after
`cargo set-version` and check the lockfile. The playbook does not generate
release notes, wait for PR review, tag the release, or create/merge the
post-release sync PR. Those remain maintainer responsibilities below.

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

Start with `forgeplan release-notes --since v<PREV> --output markdown`.
This drafts notes from committed artifacts in eight directories and
includes `active` artifacts or those with `r_eff_score > 0`; `--draft`
disables that filter. Review code/security changes that the artifact
selection may omit. See the [canonical release protocol](../operations/RELEASE-PROTOCOL.md).

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

Update `[workspace.package].version` in the root `Cargo.toml` **and**
the four path-dependency version references: two in
`crates/forgeplan-cli/Cargo.toml`, and `forgeplan-core` in both dependency
sections of `crates/forgeplan-mcp/Cargo.toml`. Run `cargo check --workspace`
to refresh `Cargo.lock`, then inspect exact version equality. Commit:

```
chore(release): bump workspace version to X.Y.Z
```

This commit lands on `dev` via the same `chore/changelog-vX.Y.Z` PR
(or its own `chore/version-bump-vX.Y.Z` PR — convention varies, just
make sure both land before opening the release branch).

---

## 4. Release branch + PR to main

```bash
git checkout dev && git pull --ff-only
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

After the release PR merges with explicit user approval, fetch the exact
`main` tip and inspect its applicable checks before creating a tag:

```bash
git fetch origin
git checkout main && git pull --ff-only
MERGE_SHA=$(git rev-parse HEAD)
gh run list --branch main --commit "$MERGE_SHA"
# Continue only after the applicable checks for this SHA passed.
git tag -a vX.Y.Z "$MERGE_SHA" -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

The tag triggers GitHub Actions (`.github/workflows/release.yml`,
configured by `cargo-dist`) which:

- Builds binaries for 5 platforms (aarch64/x86_64 × {apple-darwin,
  unknown-linux-gnu} + x86_64-pc-windows-msvc)
- Creates a GitHub release with the archives, installers, and checksums
  described by that run's dist manifest; do not assume a fixed asset count
- Updates the Homebrew formula in `ForgePlan/homebrew-tap`

**Verify within 30 minutes**:

```bash
gh release view vX.Y.Z --repo ForgePlan/Forgeplan
brew update && brew info forgeplan
forgeplan --version  # should match X.Y.Z after `brew upgrade forgeplan`
```

If cargo-dist fails, inspect the failing job's logs before choosing a
remedy. Re-run with `gh run rerun <run-id> --failed` only when the logs
support a transient failure; a linker or feature-composition error needs
a real fix. Verify the downloaded binary as described in
[Release Protocol, step 9](../operations/RELEASE-PROTOCOL.md#9-tag-after-merge-let-cargo-dist-publish).

---

## 6. Post-release sync main → dev (CLAUDE.md red line #9)

This step is **mandatory** and the most-forgotten one. Branch protection
blocks direct push to `dev`, so without this PR the version bump and
tag commits on `main` never propagate back to `dev`. The next release
attempt then fights merge conflicts on `Cargo.toml`, and any feature
PR to `dev` opened in the gap shows phantom commits.

```bash
git fetch origin
git checkout main && git pull --ff-only
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z \
  --title "chore: sync main to dev after vX.Y.Z" \
  --body "Post-release sync per CLAUDE.md red line #9. Brings vX.Y.Z release commit + tag context back to dev so the next release branch starts from a consistent base."
```

PR #223 (sync after v0.27.0) is the canonical example to copy.

Merge with **merge commit** after CI and the required approvals, following
repository policy. Mandatory sync is not an exception to owner consent
for push/PR creation or to review and merge requirements.

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

1. Branch from `main` (not `dev`): `git checkout main && git pull --ff-only && git checkout -b fix/<short-name>`
2. Apply the minimal fix and tests; update the patch version in all five manifest locations and prepare the CHANGELOG on this branch
3. Open a PR to `main` directly (skip `dev` for urgency), label it `hotfix`, wait for checks and explicit user review approval
4. After merge, tag the verified `main` SHA and publish as in section 5; do not make a separate direct version-bump commit on `main`
5. Open the mandatory main → dev sync PR as in section 6. If the fix needs adaptation, use a separate reviewed backport branch rather than pushing into `dev` or reusing a merged branch.

---

## Anti-patterns we have committed (don't repeat)

- **Force-pushing release/* branches** — rewrites the branch history used
  during review. Keep published tags immutable; if a shipped release needs
  correction, prepare a new patch release from a reviewed commit.
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

# Version bump + CHANGELOG (reviewed PR to dev, never a direct dev commit)
git checkout dev && git pull --ff-only
git checkout -b chore/changelog-vX.Y.Z
$EDITOR Cargo.toml crates/forgeplan-cli/Cargo.toml crates/forgeplan-mcp/Cargo.toml CHANGELOG.md
cargo check --workspace
git add Cargo.toml Cargo.lock crates/forgeplan-cli/Cargo.toml crates/forgeplan-mcp/Cargo.toml CHANGELOG.md
git commit -m "chore(release): bump workspace version to X.Y.Z"
git push -u origin chore/changelog-vX.Y.Z
gh pr create --base dev --head chore/changelog-vX.Y.Z --title "chore(release): prepare vX.Y.Z"
# Wait for checks, review, and merge of the preparation PR.

# Release PR
git checkout dev && git pull --ff-only
git checkout -b release/vX.Y.Z
git push -u origin release/vX.Y.Z
gh pr create --base main --head release/vX.Y.Z --title "release: vX.Y.Z" --body "..."

# After the release PR merges: follow section 5's exact-SHA check and tag gate.
git checkout main && git pull --ff-only

# Post-release sync (mandatory)
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z main
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z --title "chore: sync main to dev after vX.Y.Z"
```
