# Release Protocol — Forgeplan

Canonical end-to-end release procedure for any `release/vX.Y.Z` cut. Read
top-to-bottom the first time, then keep as a checklist. **Don't skip
steps** — most past release incidents traced to one omitted step (see
[Common pitfalls](#common-pitfalls)).

Companion read for the daily git flow:
[`GIT-WORKFLOW.ru.md`](GIT-WORKFLOW.ru.md).

---

## Why this document exists

Forgeplan ships `forgeplan release-notes` for a reason: shipping a release
with a hand-written `CHANGELOG.md` defeats the methodology audit trail.
The tool walks `git log --diff-filter=AM` over artifact directories
between two refs and emits a Keep-a-Changelog-shaped draft you then
polish. This protocol pins the surrounding workflow so the tool is
actually used and the post-merge sync step is not forgotten (RED LINE
#9).

The v0.31.0 cut (PR #282 → #284 → #285) is the canonical reference flow
— anything that worked there is the source of truth; anything that broke
is captured under [Common pitfalls](#common-pitfalls).

(Audit fix 2026-05-14: an earlier draft of this document cited PR #283
as the integration PR. That was a typo — PR #283 was a v0.32 wave-9
integration merge to `dev`, not part of the v0.31.0 cut. The last
feature PR landing in v0.31.0 was PR #282 `feat/v031-w8-quality → dev`.
Verify the chain for any release via `gh pr list --state merged --base
main --search vX.Y` plus the immediately-preceding feature merge to
`dev`.)

---

## When to cut a release

Pre-conditions (all must hold):

- `dev` is green (CI passing on the merge commit you intend to base off)
- TODO.md's "Current sprint" is closed or you have a justified reason to
  ship mid-sprint (security hotfix, regression fix)
- All artifacts intended for the release are `active` and have evidence
  with R_eff > 0
- `cargo test --workspace` passes locally on a clean checkout of `dev`
- `bash scripts/smoke-test.sh` passes
- The dependabot triage doc for the current window is filed under
  `docs/operations/dependabot-triage-YYYY-MM-DD.md` (RED LINE #10)

If any pre-condition fails: do not start the release. Fix it on `dev`
first.

---

## The 10 steps

### 1. Sync to latest `dev`

```bash
cd <repo-root>
git checkout dev && git pull --ff-only
```

Fail fast if `--ff-only` refuses — that means your local `dev` diverged
and silently amending the release with stale commits is a footgun.

### 2. Build a working binary

```bash
cargo build -p forgeplan
```

We need the binary so step 3 can call `release-notes` against the
current source. Use `debug` (faster build); release-optimised binary
isn't required to generate notes.

### 3. Generate the release-notes draft

```bash
./target/debug/forgeplan release-notes \
    --since v<PREV_TAG> \
    --output markdown \
    > /tmp/draft-vX.Y.Z.md
```

`--since v0.30.0` for a v0.31.0 cut. The tool walks
`.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/`
between the two refs, resolves slugs and `KIND-NNN` IDs, and emits a
Keep-a-Changelog-shaped section. By default it includes artifacts with
`status == active` **or** `r_eff_score > 0`; `--draft` disables that filter.
It is a draft from committed artifacts, not a completeness or security audit.

### 4. Polish the draft into `CHANGELOG.md`

Open `/tmp/draft-vX.Y.Z.md` and edit it into the `[Unreleased]` section
of `CHANGELOG.md`. Tasks:

- Write a one-sentence **sprint headline** at the top (what does this
  release *mean*? — e.g. "Wave 9 polish + 5-agent adversarial audit
  closure")
- Promote any security closures to a **bullet list immediately under
  the headline**. Use `**SEC-XX**` style markers and explain the impact
  (what was the silent failure pre-fix?)
- Add a **breaking changes summary** if any. Migration steps go inline
  if short, otherwise reference a SPEC or RFC
- Keep the six Keep-a-Changelog categories (`Added`, `Changed`,
  `Deprecated`, `Removed`, `Fixed`, `Security`) plus our `### Internal`
  for engineering details. Empty categories: omit
- Cross-reference artifact IDs (`PRD-XXX`, `EVID-XXX`) wherever a
  reader would benefit
- Rename the `[Unreleased]` heading to `[X.Y.Z] - YYYY-MM-DD`
- Re-add a fresh empty `## [Unreleased]` line above the new version

### 5. Bump versions across the workspace

Update the workspace version and all four intra-workspace dependency references:

```toml
# Cargo.toml (workspace root)
[workspace.package]
version = "X.Y.Z"

# crates/forgeplan-cli/Cargo.toml
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z" }
forgeplan-mcp  = { path = "../forgeplan-mcp",  version = "X.Y.Z" }

# crates/forgeplan-mcp/Cargo.toml  ([dependencies] and [dev-dependencies])
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z" }
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z", features = ["test-helpers"] }
```

Then regenerate the lockfile:

```bash
cargo check --workspace
```

This produces a deterministic `Cargo.lock` diff alongside the manifest
diffs.

### 6. Update human-readable docs

- `CLAUDE.md` → `## Current status` block: bump version, date, test
  count, and the one-line sprint summary
- `README.md` → test count badge / line
- `TODO.md` (if you have an "In flight" section pinning the release):
  mark closed

These are doc-only, but they're load-bearing for new contributors and
for AI agents priming context at session start (CLAUDE.md is auto-loaded
every turn).

### 7. Commit on a release branch

**Owner-consent checkpoint — before any remote write in steps 7–10:**
obtain explicit approval for the named release-branch push and PR creation,
tag publication (which triggers cargo-dist), and sync-branch push and PR
creation. Approval must cover each intended effect; pause for any effect
not yet authorized. A playbook that performs these operations needs the
same approval **before launch**; its local `--yes`/`--allow-shell` flags
are execution acknowledgements, not permission to publish. PR merges
still require the applicable review, CI, and repository merge policy.

```bash
git checkout -b release/vX.Y.Z
git add Cargo.toml Cargo.lock CHANGELOG.md CLAUDE.md README.md \
        crates/forgeplan-cli/Cargo.toml crates/forgeplan-mcp/Cargo.toml
git commit -m "release: vX.Y.Z (one-line headline)"
git push -u origin release/vX.Y.Z
```

Use a **merge commit** style (not squash) when the PR lands so the
release commit shows up in `main`'s history as-is.

### 8. PR `release/vX.Y.Z` → `main`, wait for CI, **user approves merge**

```bash
gh pr create \
    --base main --head release/vX.Y.Z \
    --title "release: vX.Y.Z (headline)" \
    --body "$(cat CHANGELOG.md | sed -n '/## \['"X.Y.Z"'\]/,/^## \[/p' | sed '$ d')"
```

RED LINE #2 still applies: don't merge without explicit user approval
after they've reviewed the PR.

PR CI must be fully green — including `smoke-e2e`, the embedding correctness
oracle, and `Check MCP tool count drift` in `ci.yml`. `security.yml`
(`cargo deny`) runs on pushes to `dev`/`main`, not PRs; inspect the security
result for the release SHA before tagging. Fix failures on the release
branch and re-push; do not push directly to `main`.

### 9. Tag after merge, let cargo-dist publish

```bash
git fetch origin
git checkout main && git pull --ff-only
MERGE_SHA=$(git rev-parse HEAD)
gh run list --branch main --commit "$MERGE_SHA"
# Confirm the applicable checks passed for this exact SHA before tagging.
git tag vX.Y.Z "$MERGE_SHA"
git push origin vX.Y.Z
```

`cargo-dist` watches tag pushes and runs the binary publishing workflow
automatically. Verify the Actions tab shows the tag workflow kicking
off; if it doesn't, check that the tag actually landed on the remote
(`git ls-remote --tags origin | grep vX.Y.Z`).

**Verify the published binary carries the features it is supposed to.**
Successful publication does not prove correct composition — PROB-088 sat
undetected for months precisely because the binaries built and shipped
fine, just without `semantic-search`. Download a published artifact and
run it in a disposable initialized workspace containing a new artifact:

```bash
/path/to/downloaded/forgeplan embed
```

Keep the full output. `Embedding not available — this build was compiled
without the semantic-search feature` proves the feature is absent.
Workspace, cache, or model-download errors do **not** prove its presence
or correctness. For a functional check, require a completed embedding
with at least one artifact embedded and `0 failed`, then search for it.
This may download the ~2.1 GB model on first use; an empty or already-current
workspace does not exercise inference.

**The old linkage check no longer works and must not be revived.** Until
RFC-013 the marker was `libc++` in `otool -L`, present because the C++
ONNX Runtime was linked in. The engine is now `tract` — pure Rust — so
that library is legitimately absent from a binary that *does* carry the
feature. Kept here as a warning rather than deleted: reintroducing it
would report every correct build as broken.

Reconcile the answer against `features` in `dist-workspace.toml` and
against what the install docs promise users. If the three disagree, the
docs are lying to somebody — fix before announcing.

**Do not delete the release branch.** Keep it as immutable history (per
project convention — see `feedback_keep_branches`).

### 10. **REQUIRED: open the sync-PR** (RED LINE #9)

```bash
git checkout main && git pull --ff-only
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z \
    --title "chore: sync main → dev after vX.Y.Z" \
    --body "Routine post-release sync. Pulls the release commit into dev so the next feature branch starts from the bumped version."
```

Branch protection blocks direct push to `dev`, so this PR is the only
sanctioned path. **Without it, `dev` forever lags `Cargo.toml`'s version
and the next release creates merge conflicts on the manifest.** See PR
#262 (sync-after-v0.30.0) and PR #285 (sync-after-v0.31.0) as the
canonical examples — every release ships with one.

Complete the sync-PR after CI and the required approvals, following the
repository's merge policy. Mandatory synchronization is not an exception
to owner consent for its push/PR or to review and merge requirements;
calling the sync low-risk does not authorize it.

---

## Common pitfalls

### Forgetting the sync-PR (step 10)

Symptom: a week later, someone opens a feature branch off `dev`, bumps
`Cargo.toml` per local convention, gets a merge conflict against `dev`
because `dev`'s `version = "X.Y.(Z-1)"` and their working copy says
`"X.Y.Z"`. Or worse: next release tries to bump `dev`'s version and
discovers it's already at the post-release value, with `Cargo.lock`
inconsistent.

Fix: open the sync-PR before closing the release session. The protocol
sequence ends at step 10 for a reason — don't treat it as optional.

### Pushing to release branch *after* PR merged

Symptom: late commit lands on the release branch after `git merge`
already landed; `git push` succeeds but the commit is dangling. Worse,
if a squash merge was used, the entire late commit is silently
discarded.

Fix: never push to a branch after its PR merged. If you need to
"amend the release", open a fresh `fix/vX.Y.Z-hotfix` branch and treat
it as a patch release (vX.Y.(Z+1)). See `feedback_squash_merge_loss` in
auto-memory.

### Version not bumped in intra-workspace path refs

Symptom: `cargo publish` (if used) errors on dependency version
mismatch. Or end-users installing via `cargo install --git` get
inconsistent linkage warnings.

Fix: step 5 lists four intra-workspace dependency references **plus** the
workspace package version. After editing, run `cargo check --workspace`
and inspect all manifest versions; Cargo compatibility ranges are not an
exact-version equality check. Search for the previous version with
`grep -rn 'version = "<PREV>"' Cargo.toml crates/*/Cargo.toml`.

### Hand-writing CHANGELOG instead of running `release-notes`

Symptom: the changelog is missing artifact cross-references, mis-spells
slugs, or omits artifacts that shipped on `dev` mid-sprint. Audit trail
broken.

Fix: always start from `forgeplan release-notes --since <prev>` output.
Polish for narrative, do not bypass for "speed" — the tool reads exactly
the artifacts you committed; hand-writing diverges from ground truth.

### CI fails on smoke-e2e *after* tag is pushed

Symptom: tag landed, binaries don't publish, users report 404 on the
brew bottle.

Fix: never tag before the applicable checks for the exact `main` SHA are
green. `git pull --ff-only` updates the checkout; it does not wait for CI
or prove that CI ran. Step 9 captures the SHA and lists its runs before
creating the tag.

### Dependabot alerts at release time (RED LINE #10)

Symptom: release ships with open dependabot alerts; user runs
`gh api repos/.../dependabot/alerts` later and sees unaddressed
high-severity CVEs unmentioned in changelog.

Fix: as part of step 4 (changelog polish), check
`gh api repos/.../dependabot/alerts` and add a paragraph to the
`### Security` section listing each alert as **addressed**,
**scheduled** (next release with target version), or **accepted with
justification** (explicit risk-take statement). File the triage doc
under `docs/operations/dependabot-triage-YYYY-MM-DD.md`.

---

## Migration notes for existing workspaces (SEC-H1, CR-C4 — v0.32.0+)

`forgeplan init` short-circuits when `.forgeplan/` already exists. That
means contributors who upgraded `forgeplan` between releases do NOT
automatically receive newly-shipped workspace files (e.g. `.gitkeep`
placeholders from PRD-077 FR-001, `secrets.env` template from FR-002).

`forgeplan init --force` refreshes the workspace skeleton. Its additive
guarantee covers artifact bodies, **not** the active configuration:

- Existing artifact `.md` bodies are NEVER overwritten.
- `config.yaml` is regenerated (the previous version is moved aside as
  `config.yaml.bak-<timestamp>` so contributors can diff their custom
  edits and re-apply them on top of new defaults).
- `.gitkeep` placeholders are backfilled into every artifact subdir
  where one is missing (SEC-H1).
- `secrets.env` template is backfilled if missing — never clobbers an
  existing file the contributor may have populated with real keys
  (SEC-H1).
- The canonical `.gitignore` section is refreshed (PROB-062).

When announcing a release that ships new workspace skeleton files,
include this snippet in the release notes:

```
For existing workspaces created before vX.Y.Z, run:

    git pull
    forgeplan init --force

Artifact bodies and existing secrets.env keys are preserved. config.yaml
is replaced with defaults; compare its timestamped backup and reapply
custom settings before using the workspace.
```

---

## Quick checklist (copy into PR description)

```
- [ ] 1. `git checkout dev && git pull --ff-only`
- [ ] 2. `cargo build -p forgeplan`
- [ ] 3. `forgeplan release-notes --since v<PREV> --output markdown > /tmp/draft.md`
- [ ] 4. Polish draft into `CHANGELOG.md`, rename `[Unreleased]` → `[X.Y.Z] - YYYY-MM-DD`
- [ ] 5. Bump Cargo.toml workspace.version + 4 intra-workspace path-version refs
- [ ] 6. Update `CLAUDE.md` Current status + `README.md` test count
- [ ] 7. Commit `release: vX.Y.Z (...)` on `release/vX.Y.Z` branch
- [ ] 8. PR → main; wait for green CI; user approves merge
- [ ] 9. `git tag vX.Y.Z <merge-sha> && git push origin vX.Y.Z` (cargo-dist publishes)
- [ ] 10. **REQUIRED**: sync-PR `chore/sync-main-to-dev-after-vX.Y.Z` (RED LINE #9)
```

---

## See also

- [`GIT-WORKFLOW.ru.md`](GIT-WORKFLOW.ru.md) — daily flow, branching strategy,
  PR pipeline
- [`QUALITY-GATES.ru.md`](QUALITY-GATES.ru.md) — full CI gate reference
- v0.31.0 cut: PR #282 (`feat/v031-w8-quality → dev`, last v0.31 feature
  PR) → PR #284 (release/v0.31.0 → main) → PR #285 (sync-after) —
  canonical reference flow. (Note: PR #283 was a v0.32 wave-9
  integration merge to dev, NOT part of the v0.31 cut — earlier drafts
  of this document mis-attributed it; corrected 2026-05-14 per audit.)
