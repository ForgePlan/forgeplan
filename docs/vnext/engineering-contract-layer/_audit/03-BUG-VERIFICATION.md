## 1. К ЗАВЕДЕНИЮ

Четыре issue. Первые две — это один survivor-record (`ci-gates-dead`), намеренно разрезанный: дефект живёт в `crates/forgeplan-core` (Rust), а гейт, который его пропустил, — в `.github/scripts/*.sh` (bash). Разные файлы, разные фиксы, разные ревьюеры; одна issue на два фикса заблокирует друг друга. Порядок — по убыванию severity.

Метки взяты из фактического набора репозитория (`gh label list`): доступны только `bug`, `documentation`, `enhancement`, `duplicate`, `good first issue`, `help wanted`, `invalid`, `question`, `wontfix`, `dependencies`, `github_actions`, `rust`. Ничего вроде `severity:high` в репозитории нет — severity указан текстом в теле.

---

### Issue 1 — HIGH

**Title** (87 chars):
```
forgeplan_update silently destroys the slug identity triple (0/384 artifacts have slug)
```

**Labels**: `bug`, `rust`

**Body**:

```markdown
## Summary

ADR-012 invariant I-1 (slug is the canonical, immutable artifact identity) holds for **zero**
artifacts in this repository. `forgeplan new` does persist `slug` / `predicted_number` /
`assigned_number`, but only as a **nested second frontmatter block inside the body**. The very
next step of the documented workflow — `forgeplan_update body=...`, which CLAUDE.md mandates
immediately after `new` (RED LINE #6) — replaces that body wholesale and the identity triple is
gone from both the markdown file and LanceDB. No warning, no validation error, no health signal.

Severity: HIGH. Not data loss of user prose, but the entire PROB-060 / PRD-076 / SPEC-005 /
RFC-009 distributed-ID design is inert in production while its unit tests pass.

## Reproducer

Corpus-wide state (read-only):

```
$ grep -rl "^slug:" .forgeplan/ | wc -l
1                          # only .forgeplan/specs/SPEC-005-*.md

$ grep -n "^slug:" .forgeplan/specs/SPEC-005-frontmatter-contract-*.md
78:slug: prd-auth-system        # CANONICAL identity. Immutable after creation.
   -> that is a documentation EXAMPLE inside SPEC-005's body, not real frontmatter.
```

Full scan of all 384 artifact `.md` files, both frontmatter blocks (66 files carry a doubled
block):

```
total artifact md files: 384
files with DOUBLED frontmatter: 66
files with `slug:` in EITHER frontmatter block: 0
```

```
$ git log --oneline -S "slug:" -- .forgeplan/
  (4 commits, all SPEC-005 / PRD-080 doc text; no artifact ever carried a slug)
```

Four artifacts created minutes before this report, on 0.33.0, through the sanctioned MCP tools —
i.e. exactly the Phase-2 artifacts that the CLAUDE.md "legacy artifacts" exemption does *not*
cover:

```
$ head -12 .forgeplan/problems/PROB-083-*.md
---
depth: standard
id: PROB-083
kind: problem
last_modified_at: 2026-08-01T18:54:47.164394+00:00
last_modified_by: claude-code/2.1.220
links:
- target: EPIC-009
  relation: informs
status: draft
title: 'Artifact substrate: ...'
---
   -> NO slug, NO predicted_number, NO assigned_number.

$ for id in PROB-083 EPIC-009 EVID-149 PROB-082; do \
    forgeplan get "$id" --json | jq -c '{id,slug,predicted_number,assigned_number,id_display}'; done
{"id":"PROB-083","slug":null,"predicted_number":null,"assigned_number":null,"id_display":null}
{"id":"EPIC-009","slug":null,"predicted_number":null,"assigned_number":null,"id_display":null}
{"id":"EVID-149","slug":null,"predicted_number":null,"assigned_number":null,"id_display":null}
{"id":"PROB-082","slug":null,"predicted_number":null,"assigned_number":null,"id_display":null}
```

Which step loses it — `forgeplan activity` (read-only), same session:

```
18:50:31 forgeplan_new     ok
18:50:58 forgeplan_update  ok      <- 27s later
18:52:07 forgeplan_new     ok
18:52:43 forgeplan_update  ok      <- 36s later
18:53:48 forgeplan_update  ok
18:54:47 forgeplan_update  ok
```

Every `new` is followed by the mandated `update`. `new` alone *does* persist the slug — the
repo's own test says so, and says exactly why it is invisible on disk
(`crates/forgeplan-cli/tests/cli_hint_slug_aware.rs:99-103`):

> "the slug lives in the canonical body's frontmatter, which is the *second* block of the
> rendered markdown — `parse_frontmatter` on the file would pick the projection-layer block
> which has no slug"

The first `forgeplan_update body=...` replaces that body, and the triple is gone.

## Root cause

Five-link chain, all in `crates/**`:

1. `crates/forgeplan-core/src/projection/mod.rs:116-127` — `KNOWN_FM_KEYS` is
   `[id, title, kind, status, depth, author, parent_epic, valid_until, tags, links]`.
   `slug` / `predicted_number` / `assigned_number` are not keys the projection frontmatter
   ever regenerates.
2. `crates/forgeplan-cli/src/commands/new.rs:179` and
   `crates/forgeplan-mcp/src/server.rs:2557` — `augment_frontmatter_with_id_fields(...)`
   injects the triple into the *rendered template's own* frontmatter, and that whole string is
   then handed over as `NewArtifact.body` (`new.rs:198`, `server.rs:2576`).
3. `crates/forgeplan-core/src/projection/mod.rs:552` —
   `Ok(format!("---\n{}---\n\n{}\n", yaml, body))`, where `yaml` is built only from
   `KNOWN_FM_KEYS`. So the triple survives *only* as a nested second block inside the body,
   unreachable by `parse_frontmatter`. Acknowledged in-tree at
   `crates/forgeplan-core/src/artifact/id_alloc.rs:271-279`.
4. `crates/forgeplan-core/src/projection/mod.rs:940-1000` — `update_body_with_projection`
   passes the caller's `body` verbatim to `render_projection_with_body` (`force_body=true`,
   mod.rs:984-997) and to `store.update_body(id, body)` (mod.rs:1000). No preservation, no
   re-augmentation. The one preservation hook, `read_preserved_fm` (mod.rs:336, called at
   mod.rs:292), only rescues keys found in the *outer* block — where the slug has never been.
5. `crates/forgeplan-cli/src/commands/get.rs:41-46` derives `slug` via `slug_from_frontmatter`
   over the *stored body's* frontmatter, so `get --json` returns null once the body was replaced.

Aggravating factor — the documented escape hatch is closed by design:
`crates/forgeplan-mcp/src/server.rs:950-955` strips YAML frontmatter from `@file` bodies
("CLI parity: a file may carry YAML frontmatter; we persist body only"), so a caller who
deliberately tries to resupply the triple through `@file` still loses it.

## Impact

- **The commit-ref contract is unfollowable.** CLAUDE.md mandates `Refs: prd-auth-system`
  (slug) before merge and forbids `Refs: PRD-074`. With no slug anywhere,
  `refs_form_from_body` falls back to the display id, so every hint the tool emits instructs
  the agent to do the thing the methodology forbids.
- **The CI id-assignment bot has nothing to key on.**
  `crates/forgeplan-cli/src/commands/ci_assign_id.rs:414` reads `slug_from_frontmatter`; with
  slug absent, lazy `assigned_number` promotion cannot run.
- **Slug-based resolution degrades to display-number resolution** — precisely the
  collision-prone addressing PROB-060 was filed to eliminate. Relevant while #394
  (duplicate id collision silently overwrites on reindex) is open.
- **Read-only consumers are blocked.** #397 assumes `get --json` "exposes slug (nullable)".
  The real reason it is always null is this write-path loss: #397's requested JSON projection
  would ship and still render nothing.

## Suggested fix

1. Make the identity triple first-class projection frontmatter. Add `slug`,
   `predicted_number`, `assigned_number` to `KNOWN_FM_KEYS`
   (`projection/mod.rs:116-127`) and emit them from `render_markdown_with_extras`
   (mod.rs:459-552), sourced from the LanceDB record. This requires the three fields on the
   store record / `NewArtifact` so they are carried rather than inferred.
2. Until (1) lands, close the destruction path directly: in `update_body_with_projection`
   (mod.rs:940-1000), read the existing triple before the write and re-augment the incoming
   body with it (reuse `augment_frontmatter_with_id_fields`, already idempotent and already
   preserving an explicit `assigned_number: null` per invariant I-2).
3. Regression test must be **E2E, not a unit test** — the existing `slug_for()` helper at
   `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs:104` only probes immediately after
   `new`, which is why this survived. New test: `init -> new -> update --body <plain markdown,
   no frontmatter> -> assert get --json .slug is still non-null`. Mirror on the MCP surface,
   including the `@file` path (`crates/forgeplan-mcp/src/server.rs:937-959`).
4. One-off backfill for the 384 existing artifacts.
   `crates/forgeplan-cli/src/commands/reconcile_ids.rs:307` already reads
   `slug` / `predicted_number` and looks like the right host for a `--backfill-identity` mode.
   Must go through the MCP/CLI mutators per RED LINE #11, not a `sed` script.

Sequencing note: fixing the companion CI gate (validate-forgeplan-frontmatter.sh Rule 1) before
this backfill will make the first subsequent PR unmergeable. Land the backfill first, or land
both together.

## Environment

- forgeplan 0.33.0 (`Cargo.toml` workspace `version = "0.33.0"`), binary `/opt/homebrew/bin/forgeplan`
- Repo SHA `78ed1b289b21ccc20efc1775017d1c3db8464e52`, branch `docs/vnext-pack-import`
- macOS (Darwin 25.1.0), arm64
```

---

### Issue 2 — MEDIUM-HIGH (companion to Issue 1)

**Title** (78 chars):
```
CI: validate-forgeplan-frontmatter.sh Rule 1 never fires (is_new always false)
```

**Labels**: `bug`, `github_actions`

**Body**:

```markdown
## Summary

Rule 1 of the frontmatter validator ("New artifacts MUST have `slug` and `predicted_number`")
is dead code in CI. Its new-file predicate tests *tracked-in-HEAD* instead of *exists-in-base-ref*,
and in an `actions/checkout` PR workspace every discovered file is tracked in HEAD by
construction — so `is_new` is always `false` and the Rule 1 block is unreachable.

This is why the write-path defect in the companion issue (identity triple destroyed by
`forgeplan_update`) went unnoticed: the gate whose single job is to catch it has never once
executed its check since the Round-2 refactor.

## Reproducer

Take an artifact that is genuinely new relative to the base ref:

```
$ f=".forgeplan/problems/PROB-083-....md"
$ git ls-files --error-unmatch "$f" >/dev/null 2>&1; echo $?
0                     # line 129 -> is_new=false
$ git show "origin/dev:$f" >/dev/null 2>&1; echo $?
128                   # file does NOT exist in the base ref -> it IS new
```

Run the script the same way CI does:

```
$ BASE_REF=dev bash .github/scripts/validate-forgeplan-frontmatter.sh
🔍 Validating Forgeplan artifact frontmatter...
📋 Found 6 artifact file(s) to validate:
   .forgeplan/epics/EPIC-009-...md
   .forgeplan/evidence/EVID-149-...md
   .forgeplan/problems/PROB-080-2-1.md
   .forgeplan/problems/PROB-081-...md
   .forgeplan/problems/PROB-082-...md
   .forgeplan/problems/PROB-083-...md
📊 Summary:
   Errors: 0
   Warnings: 0
✅ Validation PASSED
SCRIPT EXIT: 0
```

All six are new-vs-base **and** lack `slug` + `predicted_number` — exactly what Rule 1 exists
to reject. It reported 0 errors.

## Root cause

`.github/scripts/validate-forgeplan-frontmatter.sh:129`

```bash
local is_new=false
if ! git ls-files --error-unmatch "$file" > /dev/null 2>&1; then
    is_new=true
fi
```

`git ls-files --error-unmatch` tests tracked-in-HEAD, not exists-in-base-ref. The discovery
query at line 217 is `git diff --name-only "origin/${base_ref}...HEAD"`, so every candidate
file is tracked in HEAD by construction. `is_new` can never be true, and lines 143-169
(Rule 1) are unreachable.

The same file already carries the correct mechanism at lines 76-79, with a comment naming
this exact distinction:

```bash
# Round 2 audit: file existence в base ref — not currently tracked in HEAD
# (Round 1 used `git ls-files --error-unmatch` which is HEAD-tracking;
# для write-once rule we need "exists in base ref" not "exists in HEAD").
if ! git show "origin/${base_ref}:${file}" > /dev/null 2>&1 \
   && ! git show "${base_ref}:${file}" > /dev/null 2>&1; then
```

The fix was applied to Rule 2 and never back-ported to Rule 1.

Wired into CI at `.github/workflows/ci.yml:36-66` (`BASE_REF: ${{ github.base_ref }}`).

## Impact

A green CI check over a corpus where the invariant it guards is violated by 100% of artifacts
(0 of 384 carry a slug). Both frontmatter gates report clean while ADR-012 invariant I-1 holds
nowhere.

## Suggested fix

Replace line 129's predicate with the base-ref existence test the same file already uses:

```bash
local is_new=false
if ! git show "origin/${base_ref}:${file}" > /dev/null 2>&1 \
   && ! git show "${base_ref}:${file}" > /dev/null 2>&1; then
    is_new=true
fi
```

`base_ref` is already validated against `^[A-Za-z0-9._/-]+$` at lines 207-210, so no new
injection surface.

**Sequencing**: this will fail loudly on the current corpus once fixed. Land the identity
backfill from the companion issue first, or land both together, or the next PR is unmergeable.

Add a self-test fixture that builds a two-commit repo where the artifact exists in HEAD but not
in base and asserts Rule 1 fires; there is a hook-test precedent at
`.claude/hooks/tests/test-pre-pr-evidence-check.sh`.

## Environment

- Repo SHA `78ed1b289b21ccc20efc1775017d1c3db8464e52`, branch `docs/vnext-pack-import`
- bash 5.x, macOS (Darwin 25.1.0); script executed with `BASE_REF=dev`, matching
  `.github/workflows/ci.yml:66`
```

---

### Issue 3 — MEDIUM

**Title** (88 chars):
```
forgeplan link appends a blank line to the target file: projection render not idempotent
```

**Labels**: `bug`, `rust`

**Body**:

```markdown
## Summary

Every `forgeplan link` rewrites the **target** artifact's file and adds exactly one trailing
newline — nothing else changes. The target-side re-render is intentional (PRD-073 FR-005 drift
correction) and, because links are stored outgoing-only, is *specified to be a no-op* for the
target. The blank line is therefore the only output of a call designed to produce no output.

Growth is cumulative and unbounded: +1 newline per render, per mutation. It also fires on
`activate` / `deprecate` / `supersede` / `renew` / `reopen`, all of which call
`render_after_mutation`.

**Not** part of this report: the absence of a reciprocal back-link. That is by design — reverse
edges are resolved at query time via `store.get_incoming_relations()`
(`crates/forgeplan-core/src/db/store.rs:1362`, consumed by `get.rs:28`, `context.rs:45`,
`scoring/reff.rs:253`, `lifecycle/mod.rs:173`, `mcp/server.rs:7316`), verified with
`forgeplan context ADR-001` showing `Dependents: PROB-082`.

## Reproducer

**Observed diff** — linking PROB-082 to four ADRs dirtied all four target files, nothing else:

```
$ git diff -- .forgeplan/adrs/ .forgeplan/problems/
@@ -62,3 +62,4 @@ Accepted
  ## Affected Files
  - crates/forgeplan-core/src/db/**
  - crates/forgeplan-core/src/artifact/**
 +
... identical +1-blank-line hunk for ADR-003, ADR-009, ADR-011, PROB-047.
```

**Cumulative, not idempotent** — byte-level growth of one file across committed history
(each commit is an unrelated mutation that happened to re-render this ADR):

```
99d0cf0 2026-07-25 bytes=18032 trailing_nl=8
4167cc5 2026-05-23 bytes=18031 trailing_nl=7
448f98f 2026-05-08 bytes=18028 trailing_nl=4
8f2c0f3 2026-05-02 bytes=18026 trailing_nl=2
(working tree today: 9)
```

Monotonic +1 per touch; content byte count otherwise frozen at 18026 since 2026-05-02.

**Corpus-wide** (read-only scan of `.forgeplan/*/*.md`):

```
trailing-newline-count -> files
  1: 34    2: 137   3: 88    4: 91    5: 37    6: 20
  7: 4     8: 4     9: 4     10: 1    11: 2    13: 2    14: 1    15: 1
total artifact files=426, files with >1 trailing newline=392 (92%)

worst: 15  .forgeplan/prds/PRD-078-mcp-worktree-aware-projection-routing.md
       14  .forgeplan/adrs/ADR-015-...
       13  .forgeplan/problems/PROB-073-... / PROB-072-...
```

The trailing-newline count is effectively a mutation counter — the most-churned artifacts of the
v0.33 cycle carry the most blank lines.

**Direct increment not demonstrated** (a live `forgeplan link` was out of scope for the
read-only investigation). One command on a throwaway workspace confirms it:

```
forgeplan init -y && forgeplan new prd A && forgeplan new rfc B
wc -c .forgeplan/rfcs/RFC-001-b.md
forgeplan link PRD-001 RFC-001 --relation informs   # RFC-001 gains 1 byte
forgeplan unlink PRD-001 RFC-001 --relation informs # gains 1 more
wc -c .forgeplan/rfcs/RFC-001-b.md
```

## Root cause

`crates/forgeplan-core/src/projection/mod.rs:552`

```rust
Ok(format!("---\n{}---\n\n{}\n", yaml, body))
```

Appends `\n` unconditionally to a `body` that already ends in a newline.

The other half of the round trip, `crates/forgeplan-core/src/artifact/frontmatter.rs:25`:

```rust
content[body_start..].trim_start_matches('\n').to_string()
```

trims only **leading** newlines (the `---\n\n` separator). Trailing newlines survive verbatim.
So: file body ends with *k* newlines -> parse gives back *k* -> render writes *k+1*.

`render_projection_inner` (mod.rs:290-300) deliberately re-uses the file body verbatim on the
non-`force_body` path, which is what makes the growth compound instead of resetting from the DB copy.

The sibling helper `frontmatter::render_frontmatter` (`frontmatter.rs:34-37`) does **not** append
the trailing `\n` — `format!("---\n{}---\n\n{}", yaml, body)`. That asymmetry is why
`link::add_link` (`crates/forgeplan-core/src/link/mod.rs:59`) and `stamp_agent_identity`
(`projection/mod.rs:592`) are byte-stable while the projection render is not. The correct
behaviour already exists in the codebase; the projection path just doesn't use it.

Why the target is rewritten at all — `crates/forgeplan-core/src/projection/mod.rs:1137`:

```rust
if let Err(e) = render_after_mutation(workspace, store, target).await { ... }
```

Intentional and documented (mod.rs:1088-1096, PRD-073 FR-005, fixes PROB-048 symptom #2
"phantom orphan health signal because only the source side was rendered").

**Contract violated** — `crates/forgeplan-core/src/projection/mod.rs:38`:

> `| Re-render | render_projection, render_after_mutation, sync_before_mutation | Idempotent
> file↔store reconciliation used by activate, supersede, etc. |`

It is not idempotent. No test guards it: every projection test (mod.rs:1815, 1857, 1922, 1974,
2099, 2239) asserts with `result.contains(...)`, never byte equality of two consecutive renders.

## Impact

- `git status` / `git diff` on `.forgeplan/` stops being a trustworthy review signal — reviewers
  learn to skim past "+1 blank line" hunks, which is how a real one-line artifact change slips
  through unreviewed.
- Unbounded, linear in mutation count. PRD-078 is at 15 trailing newlines after roughly one
  release cycle; nothing caps it.
- 92% of the corpus (392/426) is already affected, so a normalization commit is itself a
  392-file diff. Cleanup cost rises every release this stays open.
- EOF-adjacent merge surface: two branches each appending a blank line to the same artifact both
  add a line at the identical position. Plausible conflict source on release merges (not
  constructed/verified).
- The module doc at mod.rs:38 is actively misleading for the next person extending the mutator set.
- In tension with ADR-003 ("markdown is source of truth"): a file that changes bytes without
  changing meaning weakens the invariant ADR-003 establishes.

No data loss, no incorrect query results. Hence MEDIUM, not HIGH.

## Suggested fix

One line at `crates/forgeplan-core/src/projection/mod.rs:552`:

```rust
// before
Ok(format!("---\n{}---\n\n{}\n", yaml, body))
// after
Ok(format!("---\n{}---\n\n{}\n", yaml, body.trim_end_matches('\n')))
```

Prefer `trim_end_matches('\n')` over `trim_end()` — the latter would also eat trailing spaces
inside the last content line, a behaviour change beyond the defect. Note in the changeset that
this collapses the 392 already-accreted files on their next mutation.

Regression test (nothing currently guards this — all projection tests use `.contains()`):

```rust
#[tokio::test]
async fn render_projection_is_byte_idempotent() {
    // render once, read bytes, render again with identical inputs,
    // assert_eq!(first, second) on raw bytes -- not contains()
}
```

Add the same assertion for the target side of `add_link_with_projection`, since that render is
specified to be a no-op for the target and is the path actually hit here.

Two follow-ups worth separating from the fix:
1. One-time normalization sweep of the 392 affected files. Touches `.forgeplan/*.md`, so per
   RED LINE #11 not a bare `sed` — either a `forgeplan fmt`-style command or a scripted pass
   through the sanctioned projection path, landed as its own commit.
2. Consider collapsing `render_markdown_with_extras` and `frontmatter::render_frontmatter` onto
   one serializer. Today they disagree about the trailing newline, which is how this diverged.

## Environment

- forgeplan 0.33.0 (`Cargo.toml` workspace `version = "0.33.0"`)
- Repo SHA `78ed1b289b21ccc20efc1775017d1c3db8464e52`, branch `docs/vnext-pack-import`
- macOS (Darwin 25.1.0), arm64
```

---

### Issue 4 — LOW (опциональная, фрагмент опровергнутой претензии)

Родительская претензия про `pre-pr-evidence-check.sh` опровергнута (см. §2). Но одна узкая часть выдержала проверку и стоит отдельной маленькой issue.

**Title** (69 chars):
```
pre-pr-evidence-check.sh: evidence gate skips 9 of 17 artifact kinds
```

**Labels**: `bug`, `documentation`

**Body**:

```markdown
## Summary

The pre-PR evidence hook's artifact-detection regex covers only 8 of the 17 canonical artifact
prefixes. A PR touching only the other 9 kinds passes the evidence gate without an EvidencePack.

## Reproducer

`.claude/hooks/pre-pr-evidence-check.sh:123-124` matches
`(PRD|RFC|ADR|EPIC|SPEC|PROB|EVID|NOTE)`.

Canonical prefixes, `crates/forgeplan-core/src/artifact/types.rs:90-110`:
`prd- epic- spec- rfc- adr- note- prob- sol- evid- ref- mem- uc- glos- inv- scen- hyp- dm-`

Missing from the regex: `SOL`, `REF`, `MEM`, `UC`, `GLOS`, `INV`, `SCEN`, `HYP`, `DM`. Such files
exist on disk today:

```
.forgeplan/hypotheses/HYP-001-*
.forgeplan/invariants/INV-001-*
.forgeplan/use_cases/UC-001-*
.forgeplan/scenarios/SCEN-001-*
.forgeplan/glossary/GLOS-001-*
.forgeplan/domain_models/DM-001-*
```

## Root cause

`.claude/hooks/pre-pr-evidence-check.sh:123-124` — hardcoded prefix list that has drifted from
`crates/forgeplan-core/src/artifact/types.rs:90-110`.

## Impact

Silent evidence-gate bypass for 9 artifact kinds. Low, because those kinds are rarely the sole
content of a PR today — but the drift will widen as brownfield kinds see more use.

## Suggested fix

Widen both regexes to the full set
`(PRD|RFC|ADR|EPIC|SPEC|PROB|SOL|EVID|NOTE|REF|MEM|UC|GLOS|INV|SCEN|HYP|DM)`. Keep the
`EVID`/`NOTE` skip at lines 150-154 and decide explicitly whether the new kinds are
evidence-exempt.

Better: source the list from the binary so it cannot drift again — the same drift class that
`scripts/check-mcp-tool-count.sh` already guards for MCP tools.

**Do not change** the no-artifact-ref pass (line 127) or the `docs/*` bypass. Both are
contractual: `.claude/hooks/pre-pr-evidence-check.sh:10`,
`docs/methodology/EVIDENCE-PROTOCOL.md:63-64` and `:212`, CLAUDE.md.

## Environment

- Repo SHA `78ed1b289b21ccc20efc1775017d1c3db8464e52`, branch `docs/vnext-pack-import`
```

---

## 2. НЕ БАГИ

**«forgeplan_link не пишет обратную ссылку»** — BY_DESIGN, и это ошибка ожидания, а не инструмента. Связи хранятся односторонне; обратные рёбра резолвятся на чтении через `store.get_incoming_relations()` (`crates/forgeplan-core/src/db/store.rs:1362`). Проверено: `forgeplan context ADR-001` печатает `Dependents: PROB-082`. Ребро на месте, ничего не потеряно. В issue 3 попал только мусорный перевод строки — половина исходной претензии, вторая отброшена.

**«ADR-016, ADR-017, EVID-143 не резолвятся даже после полного reindex»** — MISUSE оператора, тот же класс, что `forgeplan list adr`. Reindex не запускался; запускался `scan-import`, который физически не может тронуть эти файлы: `crates/forgeplan-core/src/scan/discovery.rs:33-40` жёстко исключает `.forgeplan/` (`skip_dirs`), и это закреплено юнит-тестом `skips_forgeplan_and_node_modules` (discovery.rs:179). «9 imported, 0 failed» — это 9 файлов из `docs/`, не артефакты. Правильная команда — `forgeplan reindex`, она резолвит все три.

Смягчающее обстоятельство: оператора к этому подтолкнула сама программа. `crates/forgeplan-core/src/db/store.rs:1169` — отгружаемая пользователю строка ошибки — советует «Run `forgeplan scan-import` to rebuild the index from markdown if data is missing», что доказуемо невозможно. То же в доке-комментарии store.rs:1123 и во всех user-facing доках (`docs/README.md:102,110`, `docs/README.ru.md:100,108`, `docs/operations/QUALITY-GATES.md:160,406`, CLAUDE.md RED LINE #11). Слова `forgeplan reindex` нет ни в одном пользовательском индексе. Это отдельный реальный дефект документации — см. §4, он не прошёл собственный adversarial-гейт и потому не заведён.

**«forgeplan health печатает взаимоисключающие вердикты» (claim A)** — не баг, отчёт устарел. Гейт на `zero_signals` (`crates/forgeplan-core/src/health/mod.rs:1699-1717`) делает ветку «Project looks healthy» недостижимой при `orphans=15` и `possible_duplicates=6`. Это фикс PROB-029/PROB-063, закреплён регрессиями `crates/forgeplan-core/tests/health_verdict_test.rs:103-109` и `:194-200`. GitHub #276 закрыт.

**«счётчики health расходятся с файловой системой» (claim B)** — тот же misuse, что выше: индекс не перестроен, `scan-import` для этого не годится. Плюс ADR-013 не появится и после корректного reindex — у файла вообще нет YAML frontmatter (`head -1` = `# ADR-013: CI Security Gate Policy…`, статус записан прозой), и `reindex.rs:74-81` его пропускает с «no id in frontmatter».

**«forgeplan_contradictions не показывает явные `contradicts`-связи»** — BY_DESIGN. Сигнатура `contradictions_v1_heuristic(records: &[ArtifactRecord])`
(`crates/forgeplan-core/src/brownfield.rs:578-582`) вообще не принимает связи — ни один код-путь не может увидеть ребро. Соседний инструмент связи берёт (`crates/forgeplan-mcp/src/server.rs:4959,4974`), и асимметрия намеренная. Область задокументирована трижды: описание MCP-инструмента (`server.rs:4889`), модульный док (`brownfield.rs:10-13`), исходный дизайн-спек (`docs/brownfield-extraction-package/integration/forgeplan-mcp-additions.md:124-133`). Единственный защитимый остаток — поле `limitations` недоговаривает собственную область (не пишет, что сканируются только артефакты с `kind == "hypothesis"`, поэтому воркспейс без гипотез всегда молча возвращает `[]`). Это LOW и на отдельную issue не тянет; уместнее одной строкой дополнить `brownfield.rs:587-591` при следующем касании файла.

**«FPV-01»** — придуманный идентификатор. Такого kind в ForgePlan нет; канонические префиксы перечислены в `crates/forgeplan-core/src/artifact/types.rs:90-110`. Ровно тот же класс ошибки, что `forgeplan list adr`.

**«scan-import затягивает собственные шаблоны репозитория как черновики»** — воспроизводится полностью (run 1: «9 imported», run 2: «6 imported, 3 skipped», run 3: снова 6 — рост неограничен), причина найдена корректно (`resolve_artifact_id`, `crates/forgeplan-core/src/scan/import.rs:539` — `id: DM-{{auto}}` не проходит `is_safe_artifact_id`, управление проваливается в цикл `for n in 1..=999`), и это не misuse: у `scan-import` нет ни одного флага исключения, а вызов был канонический. Но это дубль — см. §3. Заявленный HIGH тоже не держится: коллизия громко репортится блоком `⚠ 1 duplicate-id collision(s)` с `Fix:`-хинтом, а второй драйвер severity («документированное восстановление не работает») оказался диагностирован неверно — цепочка ломается на шаг раньше, в `forgeplan init -y` («Already initialized», no-op на клоне), до того как `scan-import` вообще успевает отработать.

**«68 артефактов несут сдвоенный frontmatter с противоречивым статусом»** — измерения верны (66 настоящих + 2 false positive в fenced-блоках, 37 с противоречием), механизм прочитан правильно, но выводы не держатся по трём пунктам. Во-первых, это дубль (см. §3). Во-вторых, утверждение «будет расти с каждым новым артефактом» эмпирически ложно: по датам рождения (`git log --diff-filter=A`) 78 артефактов созданы после 2026-05-06, из них сдвоенных — ноль; вся популяция закрыта и старше трёх месяцев. В-третьих, заявленное «ограничение на фикс» («второй блок load-bearing, там живёт identity-триплет SPEC-005») неверно ровно для затронутых файлов: `slug:` встречается в одном файле из 426 (SPEC-005, где он документируется), и ни в одном из 66 затронутых. На этом ложном допущении построены два из четырёх предложенных пунктов фикса.

Здесь же — доля вины оператора: вся популяция это артефакты, где пропустили шаг RED LINE #6 («не оставлять PRD-заглушки»). Нормальный поток `new` → `update body=...` затирает body целиком, и второй блок исчезает — именно поэтому PROB-080/PROB-081 (созданы 2026-07-25) одноблочные. 66 файлов — осадок от наполовину заполненных заглушек, которые потом правили руками в мёртвом блоке и прогоняли через lifecycle. Побочно: предупреждение `no-stub-content: Body appears to be unfilled template` на PRD-008 назвали «вечным false-positive». Это не false positive — body PRD-008 действительно незаполненный шаблон. Гейт говорит правду.

---

## 3. ДУБЛИ

| Что | Чем покрыто | Статус покрывающего |
|---|---|---|
| scan-import затягивает шаблоны/примеры как артефакты; повторный прогон множит дубликаты | **PROB-047** (`.forgeplan/problems/PROB-047-scan-import-false-positive-...md`) | active, открыт. Его секция Impact дословно содержит претензию: «**Idempotency violation**: повторный run множит duplicates, **противоречит ADR-003**». Его же таблица симптомов уже содержит строку `SPEC-SCHEMA.md → SPEC-001` — то есть «новый Tier-2 дефект» тоже не новый (проверено: dry-run печатает `+ SPEC [fn] SPEC-001`, `[fn]` = filename-tier). Из 5 предложенных митигаций реализована только 1-я (`is_doc_path`); 3, 4, 5 открыты. Верифицированная причина (`import.rs:539` + `is_safe_artifact_id`) — реальный вклад, но её место в `forgeplan_update PROB-047`, а не в новой issue. GitHub-issue на эту тему нет ни одной (`gh search issues "PROB-047"` → `[]`). |
| 68 файлов со сдвоенным frontmatter; три ADR не резолвятся | **PROB-083** (`.forgeplan/problems/PROB-083-artifact-substrate-three-adrs-unresolvable-after-reindex-68-files-carry-doubled-frontmatter.md`), закоммичен в `78ed1b2` **до** этой проверки | draft, открыт. Problem 2 — дословно та же претензия, с тем же репродьюсером `grep -c '^id: '` → 68 и тем же примером PRD-008. Открытый пункт: «Решить судьбу 68 сдвоенных frontmatter — чистка скриптом или принять, но записать». Плюс `docs/audit/PROB-060-legacy-compat-audit.md` уже вынес по этому вердикт: «**Gap? None.** The parser semantically ignores second blocks», закреплено тестами `edge_cases_frontmatter.rs:96` и `legacy_compat_e2e.rs:422`. |
| health печатает противоречивый вердикт | **#276** | CLOSED (исправлено) |

Соседние, но **не** дубли: **#397** (CLI JSON не отдаёт identity triple) — Issue 1 объясняет, почему его посылка неверна и почему запрошенная там JSON-проекция без фикса write-path ничего не покажет; при заведении стоит сослаться. **#394** (коллизия дублирующихся id при reindex) — смежная, усугубляется отсутствием slug. **#353** (асимметрия CLI vs MCP) — смежная для отдельной находки про strip frontmatter (см. §4).

---

## 4. ЧЕГО НЕ ПРОВЕРИЛИ

**Прямой инкремент от `forgeplan link`.** Мутаторы были запрещены, поэтому +1 байт на вызове не продемонстрирован напрямую — есть только код-путь, живой diff четырёх ADR и историческая динамика (18026 → 18028 → 18031 → 18032 при неизменном контенте). Закрывается одной командой на одноразовом воркспейсе (репродьюсер приведён в теле Issue 3).

**Эмитит ли `forgeplan new` slug сегодня.** Тоже мутатор. Косвенно — да: комментарий `crates/forgeplan-cli/tests/cli_hint_slug_aware.rs:99-103` описывает вложенный второй блок. Закрывается: `init -y` → `new prd X` → `head -20` файла. Если окажется, что `new` уже не пишет триплет, Issue 1 меняет форму (потеря не в `update`, а в `new`), но вывод «0 из 384 артефактов имеют slug» и импакт остаются.

**Merge-конфликт от накопленных пустых строк в EOF.** Заявлен как правдоподобный, но конфликт не сконструирован. Закрывается: две ветки, каждая делает по одной мутации одного артефакта, merge.

**Остаточный путь в `health`.** `zero_signals` (`health/mod.rs:1699-1717`) не учитывает `phase_read_errors` и `stale_ready_drafts`, хотя те повышают вердикт (`health/mod.rs:1198-1199`, `:1214-1215`), а `generate_next_actions` (`health/mod.rs:1623-1632`) их не принимает. Теоретически воркспейс, где ненулевые только эти два сигнала, может напечатать обе строки сразу. Не воспроизводилось — это не та ситуация, что репортили. Закрывается: сконструированный воркспейс ровно с этими двумя сигналами.

**`forgeplan reindex` не может импортировать 4 артефакта — имя файла расходится с `slugify(title)`.** Всплыло попутно, помечено как «genuinely REAL, different candidate», но собственный adversarial-проход не проходило.
```
WARN EVID-086 — create failed: file not found for EVID-086 at evidence/EVID-086-prd-071-...md
WARN EVID-036 — create failed: ...
WARN EVID-087 — create failed: ...
WARN SESSION-2026-04-06 — create failed: ...
```
Причина по чтению кода: `crates/forgeplan-cli/src/commands/reindex.rs:164` (`sync_artifact_from_file`) выводит ожидаемое имя файла из `slugify(frontmatter title)`, поэтому артефакт, у которого on-disk slug разошёлся с текущим title, становится невосстановимым навсегда. Что нужно, чтобы решить: прогнать через те же гейты — проверить, нет ли покрывающего PROB (искали не для него), выяснить, не by-design ли это, и подтвердить, что артефакты остаются нерезолвимыми после корректного reindex.

**`forgeplan init -y` — no-op на клонированном репозитории + дрейф документации `scan-import` vs `reindex`.** Проверено достаточно, чтобы выглядеть настоящим (документированная fresh-clone-последовательность из `docs/README.md:104-110` даёт `Error: No such file or directory (os error 2)` на `forgeplan list`, потому что `init -y` печатает «Already initialized» и ничего не делает — в клоне `.forgeplan/` уже есть). Но как отдельный кандидат через adversarial-гейт не прогонялось: не проверено, нет ли закрывающего PROB/issue, и не разбиралось, не by-design ли поведение `init` на существующем воркспейсе. Заводить пока нельзя — нужен один проход проверки. Это, вероятно, самая ценная незаведённая находка сессии: она проксимальная причина как минимум двух зарегистрированных misuse.

**Асимметрия CLI vs MCP при strip frontmatter.** CLI `crates/forgeplan-cli/src/commands/update.rs:108-116` срезает ведущий frontmatter-блок из `--body`; MCP срезает его только в ветке `@file` внутри `expand_body_filepath` (`crates/forgeplan-mcp/src/server.rs:950-955`), но не для литерального inline-body. Независимо не верифицировалось. Закрывается: сравнение двух вызовов с одинаковым body через обе поверхности; сопоставить с **#353** (ранее заведённая асимметрия CLI/MCP) прежде чем заводить.