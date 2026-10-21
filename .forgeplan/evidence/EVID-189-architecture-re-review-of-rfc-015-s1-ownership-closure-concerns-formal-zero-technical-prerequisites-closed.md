---
depth: tactical
id: EVID-189
kind: evidence
links:
- target: RFC-015
  relation: informs
- target: SPEC-008
  relation: informs
- target: EVID-188
  relation: based_on
status: draft
title: 'Architecture re-review of RFC-015 S1 ownership closure: CONCERNS - formal zero, technical prerequisites closed'
---

---
assigned_number: 189
created: 2026-10-20
predicted_number: 189
slug: evid-architecture-re-review-of-rfc-015-s1-ownership-closure-concerns-formal
updated: 2026-10-20
---

## Structured Fields

evidence_type: audit
verdict: supports
review_verdict: CONCERNS
congruence_level: 3
static_admission_assessment: technical_prerequisites_closed
dispatch_authority: coordinator

## Verdict

**CONCERNS — formal zero-findings rule only; the scoped technical source-admission prerequisites are CLOSED.**

EVID-188 F1–F3 are resolved by the actual four-document ownership/stage delta. No material architecture finding remains in this focused re-review, and no further contract change is requested for those findings. The fixed Profile-B reviewer discipline requires CONCERNS, not PASS, when zero findings survive; this label is disclosed rather than disguised as a new defect. It is not a fresh technical blocker and does not call for an endless corrective-review loop.

**Static admission recommendation:** no remaining architectural objection to the coordinator dispatching the five bounded S1 writers under the revised contract, after recording the formal zero-findings qualification. This evidence does not itself dispatch writers, activate any artifact, accept source implementation, or grant runtime/S1-G PASS. Source admission remains the coordinator's explicit decision. S2–S4 remain behind their preceding G gates.

## Measurement and scope

Focused independent re-review for task `EffNXWZmj4QGeFZgbYPR`, reviewer `S1ArchitectureGate`, integration `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, branch `feat/constellation-p0-contract`, base/HEAD `3bf82667789d8bad095645551f6d59c5f1bc478a`.

The prior review read all four canonical documents and the parent PRD in full. This review reads every line of the four closure diffs, independently reconstructs each diff from EVID-188's saved bodies to fresh CLI GET bodies, and checks the actual affected source edges. All four independent diffs exactly equal the supplied full diffs, ignoring only terminal LF normalization. Therefore no unreviewed body change is hidden behind the author's closure summary. The parent PRD-085 body and EVID-188 body also match their saved prior bodies after terminal-LF normalization.

Scope is only closure of EVID-188 F1–F3, current author ADI/validation provenance, unchanged normative acceptance, and source ownership before dispatch. It is not another full implementation, security, performance or platform audit. Existing source deficiencies are future correction work, not evidence that a source-admission contract must be rejected merely because implementation has not started.

All artifact reads/mutations use fresh `./target/debug/forgeplan` in this integration checkout; mounted Forgeplan MCP is not used. No source change, Cargo/build/test/check/formatter, tracker write, install, commit, push or activation is performed. No claim was taken under the assignment's new-EVID-only mutation restriction. Hindsight remains known unavailable; no memory recall or mental-model result is invented. Analysis is reviewer reasoning, not another `forgeplan reason` run.

## Ground-truth verification

This is a working-tree **contract** closure, not a claim that code landed. The source delta must remain empty. The four canonical files match the exact dispatched hashes, all fresh CLI GET calls exit 0, and their returned bodies exactly match the body hashes of the actual final author ADI and validation receipts.

Executed in a clean shell via `bash --noprofile --norc -c`:

```sh
set +u
printf 'ROOT='; git rev-parse --show-toplevel
printf 'HEAD='; git rev-parse HEAD
printf '\nCOMMITTED_DELTA\n'; git diff --stat 3bf82667789d8bad095645551f6d59c5f1bc478a..HEAD
printf '\nSTAGED_DELTA\n'; git diff --cached --stat
printf '\nSOURCE_DIFF\n'; git diff --stat 3bf82667789d8bad095645551f6d59c5f1bc478a -- crates eval Cargo.toml Cargo.lock
if git diff --quiet 3bf82667789d8bad095645551f6d59c5f1bc478a -- crates eval Cargo.toml Cargo.lock; then printf 'SOURCE_DELTA=EMPTY\n'; else printf 'SOURCE_DELTA=PRESENT\n'; fi
printf '\nUNTRACKED_SOURCE\n'; git ls-files --others --exclude-standard -- crates/forgeplan-core crates/forgeplan-cli crates/forgeplan-mcp eval Cargo.toml Cargo.lock
```

Literal stdout, exit 0:

```text
ROOT=/Users/explosovebit/Work/ForgePlan-Constellation-P0
HEAD=3bf82667789d8bad095645551f6d59c5f1bc478a

COMMITTED_DELTA

STAGED_DELTA

SOURCE_DIFF
SOURCE_DELTA=EMPTY

UNTRACKED_SOURCE
```

Expected contract delta tokens were FOUND by specialized grep in the canonical files: the EVID-188 closure heading at SPEC-008:303, RFC-015:216, ADR-028:156, ADR-029:290; `commands/init.rs` and additional WholeBody/RuntimeProof paths at RFC-015:311–314, ADR-028:177–180, ADR-029:311–314; the corrected internal stages at ADR-029:246–250. The query was `Уточнение границ по EVID-188|\| WholeBody|\| LocalCLI|\| RuntimeProof|\| \`before_manifest_sync\`|\| \`before_current_replace\`|\| \`after_current_replace\`` over those four canonical files. No executable hook existence is claimed by this token probe.

The prior 403-file exact source inventory was rehashed: aggregate `9cb29eb8e56a412c8195ef67772a955457d11040c9d867721a8a576c9e7ac66e`, unchanged, zero changed paths. This is a provenance file count, not an executable/test count. Raw shell output, command/cwd/exit receipts, source fingerprint and independently reconstructed diffs are retained under `.local/s1-architecture-closure-review-20261020/`.

## Closure findings assessment

| Prior finding | Actual closure and source edge | Assessment |
|---|---|---|
| EVID-188 F1 — generation/schema/reader ownership | RFC-015:313 and ADR-028/029 assign WholeBody `constellation/{manifest,storage,view}.rs`; bounded reconciler/generations fixture migration and existing core suites are explicit. All four closure sections require the mandatory artifacts/relations/sections triple and removal of descriptor-only overlay bypass. Actual manifest.rs:76–77, :121, :434–448 and view.rs:271–278 are precisely the previously unowned definitions/consumer seam. | CLOSED for pre-code ownership. Mandatory triple serialization/validation and reader behavior are still implementation obligations, not already fixed. |
| EVID-188 F2 — false outer-call barrier placement | ADR-029:246 assigns the real before-manifest-sync boundary to durability/mod.rs and the direct Windows helper; :248–249 assign CURRENT temp/replace boundaries to storage.rs. All four closure sections distinguish complete WriteSet sync, visible replacement and full descriptor/pin ACK. Actual manifest bytes→file sync is durability/mod.rs:403–409; actual CURRENT temp sync→replacement→namespace ACK is storage.rs:909–925. | CLOSED for interface/ownership. No fake stage at an outer publisher call is authorized. The Windows write-through distinction is explicit. |
| EVID-188 F3 — importer caller omitted | RFC-015:311 and both ADR ownership tables assign `commands/init.rs` only for the existing run_scan_import adapter and its regression coverage. Each closure section says the migrated skip is not successful import. Actual init.rs:1018–1032 is that skip and mandatory shared importer call. | CLOSED for bounded caller migration; unrelated initialization stays out of scope. |

The closure is an actual canonical assignment by the coordinator, not a promise to choose ownership later. Existing WholeBody suites are expressly separated from RuntimeProof's NEW fault/process targets; the latter also owns the existing `eval/test_run_acceptance.py`. That file is the runner's existing test seam (`AcceptanceRunnerTests`, import of run_acceptance, tests for missing targets, empty phase, zero doctests, fingerprint changes and executable paths). Core helper-target registration goes through the existing core Cargo.toml owner. No second writer gains core source/Cargo dependency access.

### Pending/manifest decision remains sound

`BTreeMap<String, EmbeddingJob>` at manifest.rs:121 can keep its container type; this is not proof that current values or recovery are parent-coalesced. All four new closure clauses explicitly distinguish a future jobs/runtime payload/admission/recovery change with captured section refs from an already implemented envelope. They independently assign manifest validation, so the prior mistaken inference “map type can stay, therefore manifest.rs need not change” is no longer available.

The closure preserves one parent entry, bounded admission, all outstanding section references/completion state and no second queue engine. Existing `effective_artifacts_version` keeps its `Result<u64>` interface, so export's existing consumer need not receive S2 logic. Owner acquisition can be observed in its already owned caller; owner.rs is not added. generations.rs is limited to pair-only fixtures and mechanical sections-type projection if necessary, not retention/GC redesign. These limits match the real dependency edges identified in EVID-188.

### Barrier protocol and placement

The eleven names remain unchanged: `after_artifacts_commit`, `after_relations_commit`, `after_sections_commit`, `after_storage_sync`, `before_manifest_sync`, `after_manifest_sync`, `before_current_replace`, `after_current_replace`, `after_current_ack`, `owner_acquired`, `native_call_entered`.

The unchanged newline protocol is announcement `{token,stage,pid}` and release `{token,stage,release:true}`, with parent correlation/pid checking and child token/stage/release checking under deadlines. Blocking code/calls stay test-helpers-only. The closure requires actual commits; a no-commit/unchanged path may not synthesize a notification. Full storage sync means joined writes plus successful complete WriteSet/PlatformDurability, not merely finish_writes. Before manifest sync occurs after complete manifest bytes and before fsync/FlushFileBuffers, scoped to manifest installation rather than arbitrary markers. Before CURRENT replacement requires a completely written/synced checksum-bearing temp. After replacement means visible replacement before full publication/descriptor-pin ACK; Windows write-through has already supplied namespace durability, so no fictitious pre-durability interval is claimed there. After ACK still requires platform success and descriptor/all-three-pin verification.

Owner/takeover uses actual lock acquisition, not PID/TTL stealing; native entry uses the existing actual acquired permit/CallGate before encode. The source bounds for those already owned seams remain unchanged from EVID-188. Fresh-process kill/reopen, helper stall and helper-free real inference/shutdown are separate obligations. No production env blocker, daemon, new transport, naturally hung ONNX claim, power-loss claim or Windows PASS is introduced.

## Parent-PRD fit and invariants preserved

| Requirement | Focused re-review result |
|---|---|
| PRD-085 Goal 2 / AC2 | Parent body unchanged; the closure changes ownership/placement only. Separate pull/checkout/rebase freshness observations remain required. |
| PRD other AC and Non-Goals | Explicitly scoped-out AC1/3/4/5/6 remain outside Constellation acceptance. No SaaS/store dependency, model inference in CI, Lance-in-git or multi-agent-product redesign is introduced. |
| Ordinary reads and eight async borrowed APIs | The full independent diffs do not modify the eight `&WorkspaceRead` signatures/types, twelve real MCP paths, old report/scoring meanings, complete capture/CURRENT error propagation, FPF auxiliary read boundary or existing file-first writers. |
| Canonical-first import | Unchanged behavior; the real init caller is now assigned, closing rather than weakening coverage. |
| Immutable full-body token layout | No changes to metadata-only loader, one tokenizer/layout per capture, layout/config/profile fingerprint, full UTF-8 reconstruction, actual max<=512/header<=128, intact display fields, shared builder across capture/digest/admission/recovery or zero current sections when metadata is unavailable. |
| Exact three-table generation | Required ARTIFACTS/RELATIONS/SECTIONS pins and exact current section set are retained; schema/reader files are now explicitly owned. No SCORE table or legacy compatibility bypass added. |
| Existing native worker | Retained; no alternate daemon/engine admitted. Source-based TractEmbedder observation remains the implementation reference despite generic ADI context mentioning fastembed. |
| L01–L16 / AC-S1-1–4 | No acceptance row, negative control, feature-mode split, real CLI/stdio requirement, deadline or process-exit obligation is removed by any closure diff. |
| RuntimeProof inventory | Phase/workspace semantics, true exits/nonzero execution, missing/empty failures, compile-only N/A reasons, exact tracked plus explicitly owned new/unstaged path fingerprints and executable counts remain unchanged. Existing runner tests are now explicitly owned. |
| S2–S4 | Still not admitted by S1 static architecture review; previous G PASS remains required. |

## Findings

None at or above LOW severity in this focused closure delta.

I checked each previous finding against the actual canonical owner matrix and the concrete source operation or caller that made it necessary, not against the coordinator's completion summary. I independently reconstructed the complete four-document change and confirmed that no API, token-layout, table, barrier-protocol, runtime-proof or staged-admission obligation was weakened; the assigned paths now cover the three previously missing seams. These checks support closure of F1–F3, while unchanged unimplemented source behavior remains correctly assigned to future S1 work. No style preference, pre-existing out-of-scope issue or unexercised runtime hypothesis is promoted into a manufactured finding.

## Methodology and evidence limitations

Fresh local CLI GET was used for the four contracts, PRD-085 and EVID-188. The four independent body diffs exactly match the complete supplied closure diffs. Current raw ADI stdout/stderr bytes and final validation bytes were read and hashed; their recorded input body hashes match the fresh CLI bodies, and all receipt exits are 0. Preambles before JSON are preserved, not stripped from the raw proof. No author ADI/validation was rerun merely to confirm reported success, and ADI recommendations are not treated as source/runtime proof.

Static dependency/LOC analysers, Cargo, builds, checks, tests and formatters are NOT_RUN under the explicit assignment. Cargo and cloc were known installed from the prior review; no result is fabricated for them. Missing madge/licensee and unavailable memory do not become fake technical findings. The current EVID itself is validated separately through the allowed CLI.

## Blast radius

- **If this closure were wrong:** a writer could be forced to leave an old descriptor/overlay or import caller behind, or a test could claim a durability stage it never reached. That would compromise all local consumers of the affected workspace/index or invalidate the claimed fault proof.
- **Production scope:** unchanged from EVID-188: local CLI/MCP workspaces using this shared core, not a measured percentage of deployed traffic. This document changes no source or production data.
- **Containment/recovery:** separate worktrees, one file owner, frozen integration, independent review and a single build owner remain required. Canonical Markdown stays authoritative; uncertain publication is not repaired by blind CURRENT rollback or retired-table recreation.
- **Detection:** upcoming schema/caller review and mandatory L09/L10/full-body/native probes, with actual command/exit/fingerprint evidence. No measured detection time is claimed here.

## Operability and residual risks

Actual interception of every writer, three-table durability, parent-bound pending recovery, tokenizer offsets/budgets, capture cost, real-model retrieval, native process exit and platform-specific behavior remain future source/runtime gates. They are not new closure defects and are not claimed complete by the author ADI. Helper stall is not ONNX-hang evidence; process kill is not hardware power-loss evidence; a macOS observation cannot prove Windows behavior. Historical negative EVID-180/181, S1-A static acceptance and EVID-183/184–187 retain their original scope. EVID-188 remains the negative review of the earlier fingerprint and is not terminally replaced.

## Positive observations

- The owner expansion is tied to actual serialized definitions, platform helpers and callers, while keeping retention, owner.rs, federation/export redesign and unrelated init behavior outside the work.
- The stage correction distinguishes Windows write-through replacement from the later complete publication/pin-verification ACK rather than pretending every OS has the POSIX gap.
- Existing WholeBody tests, new RuntimeProof process targets and core manifest registration now have explicit non-overlapping owners; the underlying behavioral acceptance has not been relaxed to obtain closure.

## Result and interpretation

All three EVID-188 source-admission findings are closed in the reviewed canonical delta. The scoped technical prerequisite is satisfied; this reviewer has no remaining architectural objection to bounded S1 dispatch by the coordinator. The formal verdict is CONCERNS solely because the fixed reviewer policy forbids PASS on an honest zero-findings report. Acknowledge that policy qualification in the admission record, rather than inventing work or treating a readiness score as permission.

After source integration, the approved independent source review, single-owner build/behavioral matrix, real CLI/stdio verification and S1-G remain mandatory. No S2 source dispatch, RFC activation or runtime acceptance is recommended by this static evidence.

## Congruence Level Justification

CL3 applies to direct inspection of the same integration source fingerprint and exact current canonical contract bytes. `verdict: supports` supports the bounded ownership/interface closure; `review_verdict: CONCERNS` discloses the mandatory zero-findings convention. Neither is a test result. R_eff is evidence bookkeeping, not runtime readiness or an activation decision.

## Raw provenance

Proof root: `.local/s1-architecture-closure-review-20261020/`. Current author raw root: `.local/s1-ownership-closure-coordinator/`. The final manifest and proof-digest ledger retain exact paths, byte sizes, SHA-256, argv/cwd/exit and post-link canonical comparisons. This evidence is based_on EVID-188, the actual observation being re-reviewed, and informs RFC-015/SPEC-008; it does not supersede the earlier negative record.

CLI binary retained from the verified source-admission toolchain: `target/debug/forgeplan`, 461012344 bytes, SHA-256 `96cd31a566e92981e99f558462ad1ee7da17bed00daf9db8db3e71dca9b688ea`. No rebuild or installation occurred.

### Independently reconstructed closure diffs

| Artifact | Bytes | SHA-256 | Full supplied diff equality |
|---|---:|---|---|
| SPEC-008 | 5515 | `2afc3c7b631bfbcc46e483fd3170b4e396e7ed817edb041d7204d6d6cb258409` | true |
| RFC-015 | 8575 | `eef9844c38d1bdeb21e945fd8646a8433f39f26ad8b6fe4e5c92326da03ae759` | true |
| ADR-028 | 7613 | `fe44216b986c4a940a961cc162c6004a2dad222b8e1973d57f9211dea5909173` | true |
| ADR-029 | 9964 | `15469fc976773e5325b969f772a0883f352aa984c247f304bb358c1d2e3b4f35` | true |

### Actual final author ADI

Each command was `./target/debug/forgeplan reason <ID> --json`, cwd integration, receipt exit 0. SPEC uses the corrected final scenario-heading run, not the earlier successful but no-longer-current input. The four recommendations retain draft and distinguish implementation/runtime unknowns from the next architectural step. For each stderr: 294 bytes, SHA-256 `ce235f79b24ca5525904fc417adbbbbc7068a042f97da25b43bf98e69c3be9ac`. Raw stdout preambles are 53 bytes for SPEC, 52 for the others.

| Artifact | Raw stdout under author root | Bytes | SHA-256 |
|---|---|---:|---|
| SPEC-008 | `SPEC-008.scenario-heading-final-reason.stdout` | 16369 | `a784235466822356c59596807fbf07adb6e69ecd9cf62d7bd4b7f60bdb497470` |
| RFC-015 | `RFC-015.final-reason.stdout` | 14700 | `ce8e49594501888c2c8df54e6e5e074b6c6d9c607ef35d5b1486656d60e159e7` |
| ADR-028 | `ADR-028.final-reason.stdout` | 15965 | `62b28f7406908558b4f735a1a5167b56f572e913745e61fac9549f6993651e29` |
| ADR-029 | `ADR-029.final-reason.stdout` | 18072 | `fe0f0154a800042fbbcf2a9d0087b2303ddff5988e0b93b9f137317529957a0e` |

Current CLI body hashes match their ADI and validation inputs exactly: SPEC `fefb134c2e782bc8456fcbcec2fe4a46d0821f72a40669e301422c076aca3ac7`; RFC `5156c7242e4a7f9c26afcc5594ec62b506d1afeb0f5607987a758429328b6fc5`; ADR-028 `3bb043ad470211f37a9f58ec88e039ca31bc8b0c4542c6961da94870d5a3e6ff`; ADR-029 `56344a138ffd3530dca1dc9cd2979bbffa849bbf24932ef7667b480020607fe1`.

### Actual final author validation

Each command was `./target/debug/forgeplan validate <ID> --json`, cwd integration, receipt exit 0; no rerun by this reviewer. SPEC/RFC/ADR-029 have 0 MUST/0 SHOULD. ADR-028 keeps 0 MUST/1 SHOULD for justified incoming/context-only Related Artifacts mentions; no fake dependency cycle was added to silence it.

| Artifact | Raw stdout under author root | Bytes | SHA-256 |
|---|---|---:|---|
| SPEC-008 | `SPEC-008.scenario-heading-validate.stdout` | 245 | `3f7bce2e145e29ee235b7186c36c03cefda55083680c8483cdaff87949348644` |
| RFC-015 | `RFC-015.final-validate.stdout` | 242 | `0672ca457688baeb03b0af1e26477f9b6a244a99494178ce482346de01bf4b32` |
| ADR-028 | `ADR-028.final-validate.stdout` | 754 | `ff913f4f8f63f96918f68b613bc23c6e6ee9a2f9df6d3f2572f6826e1c7a0edc` |
| ADR-029 | `ADR-029.final-validate.stdout` | 242 | `515ada569746e4f5bb477be29372c0c2150fa4b6050450376074d531683e99d7` |

### Reviewed raw canonical hashes before link rendering

| Contract | Bytes | SHA-256 |
|---|---:|---|
| SPEC-008 | 68279 | `b8f62411ab2ac5249533adaa3a478585fe7fbc4411af8f32d1e48a76099bea7a` |
| RFC-015 | 57374 | `7fb29fa2c9aa6c539b7e04f1edcd24118677502933eba46a76dc21155a7fa93b` |
| ADR-028 | 31408 | `dda3529e5e53117fae6871a111db708880318fccf789700c797dc98906ff86bd` |
| ADR-029 | 107444 | `97c5dad201ec9d06fd185ba3755098d2349b4a50aa96ab2ac5ef9c60f7ffb9ac` |

## Recommended next steps

- Coordinator: record that F1–F3 are closed and that CONCERNS is a mandatory formal zero-findings label, then make the bounded five-writer S1 source-dispatch decision. No extra design or contract rewrite is requested by this re-review.
- Source owners: implement only the revised owned scope, retaining NOT_RUN and all acceptance obligations until actual independent evidence exists.
- Build owner/guardian: use the later frozen implementation/test/harness fingerprint; this static record cannot satisfy S1-V/G or admit S2–S4.

## References

RFC-015 and SPEC-008 are the informed artifacts; PRD-085 Goal 2/AC2 is the unchanged parent acceptance context. ADR-028/ADR-029 provide the scoped architecture. EVID-188 is the preserved prior negative review and direct basis of this closure assessment. Historical EVID-180/181/183/184–187 are not rewritten or promoted to current runtime proof.

## Final canonical fingerprints after this review's links

Fresh CLI links all exited 0 without stderr: EVID-189 `informs` RFC-015 and SPEC-008, and `based_on` EVID-188. No auto-activation flag was passed. Byte comparison against this review's before snapshots found only the expected target-render trailing-LF changes, no stale semantic or metadata rewrite.

| Contract | Final bytes | Final SHA-256 | Comparison |
|---|---:|---|---|
| SPEC-008 | 68280 | `3a8042bbe6ad8c71739fdc687252e7bd13eb37af719cbc9031bfc03406b9da93` | One trailing LF added |
| RFC-015 | 57375 | `2728d06cc74a336f16d7e6b7b466942fc675c382518a14580b3aacc5a24d9ab9` | One trailing LF added |
| ADR-028 | 31408 | `dda3529e5e53117fae6871a111db708880318fccf789700c797dc98906ff86bd` | Byte-identical |
| ADR-029 | 107444 | `97c5dad201ec9d06fd185ba3755098d2349b4a50aa96ab2ac5ef9c60f7ffb9ac` | Byte-identical |

EVID-188 also gained exactly one trailing LF from the required basis link; its negative verdict, findings, scope, metadata and body are otherwise unchanged. Final EVID-188: 37706 bytes, SHA-256 `8b6e7acda56a859eab5388b36baba551a7f1e3a28d640dff24d422db3eee154f`. EVID-180/181/183 and all EVID-184–187 are byte-identical to this review's initial snapshots. All four reviewed contracts remain draft.

`final-fingerprint-manifest.json` and `proof-file-digests.json` under the reviewer proof root are the final handoff records: canonical/EVID hashes, raw output SHA-256+size, exact CLI receipts, independent diff equality, source/binary fingerprints, closure results and the explicit formal-zero qualification. They are not runtime results.


