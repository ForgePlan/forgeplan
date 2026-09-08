# Handoff: SPEC validator inversion + NFR rules (#449, #450, PROB-105)

**Status:** code complete, gates green, **pushed**; PR open against `dev`.
**Branch:** `fix/spec-validator-inverted` off `dev` — `daa103b` (code) + `3e16e48` (this document).
**Written:** 2026-09-07, against `forgeplan 0.36.0`.

Everything this document points at is committed. That is deliberate: the previous
handoff in this repo pointed at a session scratch directory that did not survive
the session, which is the same defect class the work was about.

---

## What is done

One commit, `daa103b`, closing two GitHub issues and one problem found while
checking them.

### The finding that reframed both issues

Issue #450 asks for a rule enforcing "every `### Requirement` has a
`#### Scenario`", on the grounds that the invariant is declared in prose and
checked nowhere. Measured before implementing, the premise does not hold here —
and the truth is worse than the report.

| Input | Before | After |
|---|---|---|
| untouched SPEC template | `PASS — 0 error(s), 0 warning(s)` → activates at **R_eff 1.00** | stub warning; `activate` refuses |
| behavioural spec, 2 requirements + 2 GIVEN/WHEN/THEN scenarios | **`x [MUST] spec-contracts`** → FAIL, cannot activate | PASS |
| SPEC-002 … SPEC-006 | pass | unchanged |
| SPEC-001 | fails `spec-summary` + `spec-contracts` | unchanged — it is a document *about* writing specs, filed as a spec |

So core did not lack a rule about scenarios. It had a MUST pointing against them,
while waving through documents that said nothing. That is why the marketplace TDD
flow grew its own gate (`tdd-planner` HARD RULE 1): core actively rejected the
shape TDD needs.

### Three fixes, none of which teaches the kernel a methodology

1. **`check_stub` can see a SPEC.** Its twelve phrase markers are all PRD prose;
   the placeholder signal was capped at `+1` against a threshold of 3, so fifteen
   unfilled slots weighed the same as one. The count scales now.

   Threshold from the corpus, not taste — SPEC template **15** placeholders, PRD
   template **5**, the six real SPECs **0–3**. A line-count test was measured and
   rejected: the untouched template has 25 non-empty lines under
   `## API Contracts`, because placeholder JSON is still lines.

2. **`spec-contracts` accepts a behavioural contract.** It still demands *a*
   contract; it stopped demanding one particular form. `## Requirements`,
   `## Contract`, `## Behavioral Contract` join the structural headings.

3. **`spec-requirement-has-scenario`** (Should) — #450, conditional. Silent on a
   structural spec, fires only on a half-authored behavioural one. The blanket
   form would have fired on 6 of 6 SPECs here, none defective.

Plus: the stub gate's remediation told every kind to "Fill MUST sections
(Problem, Goals, FR)". It now names the sections of the kind in hand.

### #449 — NFR rules, and the trap in them

`prd-nfr-exist` and `prd-nfr-measurable`, both Should. `extract_nfr_section`
already existed, called from exactly one place — the tech-leakage check — so the
validator could find the section and asked nothing about its contents.

The subjective-adjective blacklist reads like a list of NFRs (`scalable`,
`robust`, `efficient`, `responsive`, `fast`) and was only ever applied to the FR
section. Across the 69 PRDs: **2** hits in FR (checked), **15** in NFR (not).

**All 15 sit inside the template's own guidance** —
`<!-- BAD: "System should be fast and responsive" -->`. A rule flagging them
would be unclosable: the only fix is deleting the instructions. The rule strips
non-prose first. Verified **0 findings across all 69 PRDs**, while hand-written
vague prose still produces findings with line numbers.

> `check_measurability_adjectives`, the FR-side rule that shipped long ago, does
> **not** strip and carries the same latent bug. It has never fired only because
> the template's BAD examples happen to live under the NFR heading. Left alone —
> noted so the next person does not rediscover it.

---

## Verification already done

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets` | exit 0, **0 warnings** |
| `cargo test -p forgeplan-core --features test-helpers --no-fail-fast` | 2239 passed, 3 failed (17 binaries) |
| `cargo test -p forgeplan --no-fail-fast` | 807 passed, **0 failed** (58 binaries) |
| `cargo test -p forgeplan-mcp --no-fail-fast` | 274 passed, **0 failed** (19 binaries) |

94 binaries total, which matches what `--workspace` produces — the per-crate
split did not skip anything. The 3 failures are #454 in `git::tests`; they pass
in isolation and this diff does not touch that module.

**Mutation checks** — each fix reverted, the matching test had to fail:

- remove the comment strip → `nfr_adjectives_inside_html_comments_are_not_flagged` FAILED
- remove the placeholder scaling → untouched template back to `PASS — 0 error(s), 0 warning(s)`

Four pre-existing rule-count tests failed and were **updated, not silenced**.
They sum named groups with comments rather than asserting a bare number, so
updating one means explaining where the new term came from.
`rules_for_spec_returns_base_plus_3` was **renamed** to `_plus_4` rather than
edited in place — a test name carrying a number goes stale silently.

---

## What is left

1. ~~Push and open the PR~~ — done; see the PR linked from the branch.
2. **CHANGELOG entry** — drafted but not written into the file. Content is in
   this document; the Fixed/Added split is already worked out above.
3. **SPEC template note** (optional, recommended). The template is API-first and
   is the only signal an author has about what a spec looks like. Now that the
   validator accepts a behavioural contract, a two-line comment near the top
   saying both shapes are legitimate removes the surprise. Templates compile into
   the binary via `include_str!`, so do not edit them while a build is running.
4. **Issue comments** for #449 and #450 — #450's especially, because its premise
   was inverted and the reporter deserves to know the truth was worse than the
   report.

---

## Do not re-litigate

- **No whitelist of legitimate SPEC shapes in the kernel.** A judge panel
  considered one and rejected it: a list of six blessed forms rots toward noise,
  and a seventh legitimate shape would need a Rust change and a release before
  its author stops getting a false finding. The substance tests ask whether a
  section is filled, not whether it is Gherkin.
- **The blanket #450 rule.** Measured: fires on 6 of 6 SPECs here, none
  defective.
- **Must instead of Should** for the NFR rules. 30 of 69 PRDs have no NFR
  section.

---

## Environment notes that cost real time

- **A full `cargo test --workspace` does not fit** in the free space on this
  machine. Twice it reported `ld: write() failed, errno=28` at ~300 MB free. Run
  per crate instead; `forgeplan-core` needs `--features test-helpers` because the
  `*_for_test` helpers are gated and normally arrive through `forgeplan-mcp`'s
  dev-dependency.
- **Always pass `--no-fail-fast`.** Without it the run stops after the first
  failing binary and looks like a truncated build.
- **Count test binaries**, not just pass/fail: a run that never happened prints
  zeros that read as success. See `AGENTS.md` § *Parallel agents and the build
  directory*.

## Uncommitted work in the tree that is NOT mine

A second session is working in this worktree. At the time of writing it left
`Cargo.toml` (a `[profile.dev.package."*"] opt-level = 1` block) and a new
`.cargo/config.toml` (aliases only) uncommitted. Both look sane and on-topic —
they speed up builds — but they are someone else's change and were deliberately
left out of `daa103b`. Do not sweep them into a commit without checking with
their author.

## Artifacts

- **PROB-105** — the inversion, with the reproduction and the placeholder
  measurements
- **PRD-086** — the trust-layer work this grew out of; PROB-105 informs it
- **#449**, **#450** — the GitHub issues
