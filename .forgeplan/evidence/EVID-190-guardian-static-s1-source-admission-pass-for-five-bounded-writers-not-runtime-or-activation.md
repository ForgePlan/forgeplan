---
depth: tactical
id: EVID-190
kind: evidence
links:
- target: RFC-015
  relation: informs
- target: SPEC-008
  relation: informs
- target: EVID-188
  relation: based_on
- target: EVID-189
  relation: based_on
status: draft
title: 'Guardian static S1 source admission: PASS for five bounded writers, not runtime or activation'
---

---
assigned_number: 190
created: 2026-10-20
predicted_number: 190
slug: evid-guardian-static-s1-source-admission-pass-for-five-bounded-writers-not
updated: 2026-10-20
---

## Verdict

**PASS — static S1 source admission for the five bounded writers only.**

EVID-188 F1–F3 are closed by the current four-contract delta and independently confirmed in EVID-189. No unresolved material condition remains for approved plan step 3. EVID-189's CONCERNS label is expressly a formal zero-findings convention, not a technical finding; it is preserved, not relabelled. This decision permits the coordinator's bounded step-4 source dispatch. It does **not** accept implementation, runtime, S1-G, S2–S4, deployment, or artifact activation.

## Structured Fields

verdict: supports
review_verdict: PASS
congruence_level: 3
evidence_type: audit
scope: static_s1_source_admission_only
material_open_static_findings: 0
runtime_status: NOT_RUN
activation_authorized: false

## Measurement and artifact under review

- Reviewer: S1StaticAdmissionGate, OMP / openai-codex/gpt-6-astra. Task: `EffNXWZmj4QGeFZgbYPR`.
- Integration: `/Users/explosovebit/Work/ForgePlan-Constellation-P0`; branch `feat/constellation-p0-contract`; governance checkpoint `3bf82667789d8bad095645551f6d59c5f1bc478a`.
- Current drafts: RFC-015, SPEC-008, ADR-028 and ADR-029, all Deep. Parent context: PRD-085 Goal 2 / AC-2; its other acceptance criteria are not closed.
- Route remains Row 4 / RIPER-5 corrective continuation, Validate → Code → Evidence → Activate. Historical S1-A 10/10 is static history, not runtime proof.
- All artifact reads and mutations used fresh `./target/debug/forgeplan` with the explicit integration cwd. Mounted Forgeplan MCP was not used because its explicit-workspace GET is known stale. No stale body was written back.

## Evidence chain and scope disposition

The fresh CLI incoming-link inventory was inspected, including older negatives rather than only the latest review. EVID-188 and EVID-189 were read in full; their closure delta, current contract sections, exact hashes and retained ADI/validation inputs are the decisive evidence. Older records were considered for their stated findings, scope and disposition, not re-audited as current implementation.

| EVID, oldest first | Recorded result / scope | Disposition for this gate |
|---|---|---|
| EVID-174 | weakens; historical ADI failure, formal architectural CONCERNS | Historical failure retained; EVID-175 and the current four actual ADIs are distinct later observations. No renewed login requirement inferred. |
| EVID-175 | supports actual historical Codex ADI; no runtime or unconditional admission | Does not substitute for ADI on the present bodies. |
| EVID-176 | REQUEST_CHANGES; reconciliation/read audit | Preserved implementation findings, not current code acceptance. |
| EVID-177 | REQUEST_CHANGES including a static BLOCKER | Preserved for its older source fingerprint; no resolution by this static gate is claimed. |
| EVID-178 | REQUEST_CHANGES including a static BLOCKER | Same boundary; runtime/source acceptance remains separate. |
| EVID-179 | inconclusive compiler recovery | Type checking is not test or runtime acceptance. |
| EVID-180 | weakens; real CLI/MCP failures | Retained negative observations. Federation/pack failures remain under staged S2/S3 obligations, not erased by S1 admission. |
| EVID-181 | BLOCKER for v0391/v0392 snapshots | Retained negative review; this gate grants no positive runtime verdict for those or current source. |
| EVID-182 | supports historical bounded macOS execution | No transfer to the corrective fingerprint. |
| EVID-183 | supports historical macOS claims | Not current readiness. The count correction remains a separate pending action: 5673 executable cases + 10 doctests, not the artifact's 5683 + 10. |
| EVID-184 | CONCERNS; governance hook-executor ambiguity | EVID-185/186 address that particular governance finding. |
| EVID-185 | CONCERNS solely formal zero; governance fit confirmed | Not a new technical defect. |
| EVID-186 | PASS, governance patch only | No source/runtime authority inherited. |
| EVID-187 | PASS, governance checks/checkpoint only | Establishes the governance checkpoint, not S1-G. |
| EVID-188 | CONCERNS, two HIGH and one MEDIUM static admission findings | F1–F3 closed for the revised contract by EVID-189; the negative original remains unchanged in meaning and status. |
| EVID-189 | CONCERNS solely formal zero; technical prerequisites closed | Zero material findings; no remaining architectural objection to the five bounded writers. |

The decisive closure chain has 0 BLOCKER / 2 CONCERNS / 0 PASS labels, but **0 unresolved material static admission findings**. This record makes a new scoped decision; it does not supersede, deprecate, activate, or improve the recorded verdict of any older EVID. Outstanding implementation/runtime conditions are neither waived nor treated as prerequisites requiring the future code to exist before corrective writing may begin.

## Findings and static gate criteria

| Criterion | Result | Evidence |
|---|---|---|
| Current frozen contract matches independent review | PASS | All four initial raw files exactly matched EVID-189's final manifest; fresh CLI bodies matched the actual final ADI/validation input hashes. |
| Actual Deep ADI on all four final bodies | PASS | Retained codex-cli runs exited 0; parsed `adi_output.hypotheses` counts SPEC/RFC/ADR-028/ADR-029 = 4/3/3/4. No reason rerun or invented model result. |
| Zero MUST validation | PASS | Four retained final validate exits 0, zero errors; only ADR-028 has one explained context-link SHOULD. |
| EVID-188 F1 ownership/schema/reader closure | PASS | RFC-015 Affected Files and all four closure clauses assign manifest/storage/view, required triple validation and descriptor-only-overlay removal; bounded fixture consumers and existing core tests belong to WholeBody. EVID-189 closure row F1 confirms the actual seams. |
| EVID-188 F2 real-stage closure | PASS | ADR-029 §8 and all four closure clauses assign the internal pre-manifest-sync and CURRENT temp/replace boundaries, with the Windows write-through distinction and full verification ACK. EVID-189 F2. |
| EVID-188 F3 importer-caller closure | PASS | LocalCLI owns `commands/init.rs` only for `run_scan_import` and its regression coverage; migrated skip is not successful import. EVID-189 F3. |
| Exclusive five-writer scope and mandatory later gates preserved | PASS | RFC-015 Affected Files, SPEC-008 S1-R3/R4 and AC-S1-1–4, ADR-028/029 preconditions, AGENTS.md:246–299. |

**Static criteria: 7/7.** There is no additional technical fix requested solely to satisfy a findings quota.

### Project-config gates

`.forgeplan/project-config.yaml` and `.forgeplan/quality-gates.yaml` were absent in the explicit integration directory discovery. **project-config: not found, defaults applied.** No configuration was created or changed.

| Criterion | Conservative default | Scoped observation | Result |
|---|---|---|---|
| Coverage | min_test_coverage=80 | No post-code run or coverage measurement; current L/F behavior is NOT_RUN. | Not applicable to pre-code admission; not passed or waived |
| Critical findings | max_findings_critical=0 | 0 current static admission findings | PASS |
| High findings | max_findings_high=3 | EVID-188's 2 HIGH findings closed by EVID-189; 0 unresolved | PASS |
| Medium findings | max_findings_medium=10; CONCERNS above 20 | EVID-188's 1 MEDIUM finding closed; 0 unresolved | PASS |
| Validation | require_validate_pass=true | All four current contract validations have 0 MUST; retained logs match current semantic inputs | PASS |
| Audit PASS label | require_audit_pass=true | This is not an activation gate; EVID-189's actual independent assessment is technical closure, with a disclosed formal-zero label | Not applicable as an activation prerequisite; no circular use of this EVID |
| Evidence chain | require_evidence_chain=[prd,rfc,adr,spec] | EVID-188/189 inform RFC-015 and SPEC-008; this decision is based_on both | PASS for the scoped admission chain |

**Gates summary: 5/5 applicable project-config checks; 2/7 not applicable.** This fraction is not a full-RFC activation or runtime gate score. Parent-wide activation, active-ADR decay, coverage, required tester/security reviews and complete runtime audit-pass requirements remain outside this assignment, not silently satisfied.

## Accepted owner scope

Prefixes: core source = `crates/forgeplan-core/src/`; CLI source = `crates/forgeplan-cli/src/`; MCP = `crates/forgeplan-mcp/`. The current RFC-015/ADR-028/ADR-029 owner tables, not the older documents inherited by source worktrees, are authoritative.

| Writer | Accepted exclusive paths and limits |
|---|---|
| LocalReports | Core `health/mod.rs`, `journal/mod.rs`, `scoring/decay.rs`, `drift/mod.rs`, `anomalies.rs`, `release_notes.rs`, `fpf/mod.rs`, and corresponding behavioral tests. Eight borrowed async captured APIs retain real result types/defaults and shared computations. |
| LocalCLI | CLI `commands/{common,health,blindspots,journal,decay,drift,anomalies,release_notes,status,decompose,discover,scan_import}.rs`; `commands/init.rs` only for existing `run_scan_import` adapter/regressions; core `scan/import.rs`; corresponding CLI/core tests. No unrelated init rewrite or broad common-signature change. |
| MCPCompat | Entire MCP `src/server.rs` and separate MCP regression tests; exactly the twelve actual legacy-refusal paths, including `fpf_search`, not config-only `fpf_rules`. No S2 `constellation_read.rs` ownership. |
| WholeBody | Core `constellation/{capture,jobs,facade,runtime,publisher,lance_tables,manifest,storage,view}.rs`, `constellation/durability/{mod,windows}.rs`; `constellation/reconciler.rs` only affected embedded fixtures; `constellation/generations.rs` only pair fixtures and mechanical sections-type projection; `embed/{tract_engine,tokenizer,mod}.rs`; `crates/forgeplan-core/Cargo.toml`, `Cargo.lock` if needed; existing `crates/forgeplan-core/tests/constellation_{source_view,embedding_state,embedding_preservation,pinned_search,runtime_e2e}.rs` and new section/tokenizer tests. Owns all core barrier calls and core target registration. |
| RuntimeProof | `eval/run_acceptance.py`, `eval/test_run_acceptance.py`, new `eval/constellation_readiness.py`, separate NEW subprocess/fault targets. No existing WholeBody test edits, core production edits or Cargo dependency edits; helper-target registration goes through WholeBody. |

One file owner per worktree remains mandatory, including explicit assignment of new test paths in the coordinator's dispatch/handoff. No owner.rs, S2 export/federation, retention/GC redesign, alternate queue/storage/scoring engine, daemon, or production blocking IPC is admitted. `BTreeMap<String, EmbeddingJob>` may retain its type, but parent-coalesced payload/admission/recovery is still required implementation work, not existing behavior. `effective_artifacts_version -> Result<u64>` stays compatible with its export consumer.

### Accepted stage interface

Test-helpers only: loopback TCP newline `{token,stage,pid}` and release `{token,stage,release:true}`, with correlation/pid checks and deadlines. Names remain `after_artifacts_commit`, `after_relations_commit`, `after_sections_commit`, `after_storage_sync`, `before_manifest_sync`, `after_manifest_sync`, `before_current_replace`, `after_current_replace`, `after_current_ack`, `owner_acquired`, `native_call_entered`.

- Commit notifications follow the actual respective commit; unchanged/no-commit paths cannot fabricate them. `after_storage_sync` follows joined writes and complete WriteSet/PlatformDurability, not only finish_writes.
- `before_manifest_sync` is inside `durability/mod.rs` or the direct Windows helper after complete manifest bytes, before fsync/FlushFileBuffers. `after_manifest_sync` follows successful immutable installation/durability, before CURRENT changes.
- `before_current_replace` is inside storage after the complete synced checksum-bearing temp. `after_current_replace` is visible replacement before complete publication/descriptor-pin verification ACK; Windows write-through has **already** supplied namespace durability. `after_current_ack` requires full successful ACK and exact three-pin verification.
- Owner acquisition is the real OS guard; native entry holds the real permit/CallGate before encode. No outer-call imitation or unowned owner.rs change is needed.
- Fresh-process kill/reopen, helper-induced native stall, helper-free real-model encode/shutdown, hardware power loss and Windows behavior remain distinct evidence obligations. None is observed by this static decision.

## Ground-truth verification and accepted fingerprints

This is contract admission, not a claim that implementation changed. EVID-189 establishes base=HEAD, empty source delta and empty untracked source inventory. This reviewer rehashed all 403 exact files in its source inventory: **403/403 identical**, zero changed/missing paths. The inherited length-prefixed path-and-byte aggregate remains `9cb29eb8e56a412c8195ef67772a955457d11040c9d867721a8a576c9e7ac66e`; this is a file identity count, not test execution. Inventory file: 73909 bytes, SHA-256 `b32c873981077f24547f120b872faab51922edb917cb3ce631b30bf5fd21ddb2`.

Freshly measured CLI binary: `target/debug/forgeplan`, 461012344 bytes, SHA-256 `96cd31a566e92981e99f558462ad1ee7da17bed00daf9db8db3e71dca9b688ea`. No build or installation occurred.

**Accepted canonical bytes after this EVID's four link writes:**

| Contract | Canonical path | Bytes | SHA-256 |
|---|---|---:|---|
| SPEC-008 | `.forgeplan/specs/SPEC-008-constellation-0-0-embeddings.md` | 68281 | `52262c6985c9772835ab036805aa656caa32bf9814948d9b7d9a61c14500fb94` |
| RFC-015 | `.forgeplan/rfcs/RFC-015-constellation-0-0.md` | 57376 | `a58a5ca4368d390d75dccffb3b1994724752a24f491ba8fc57dfd0b1eca97590` |
| ADR-028 | `.forgeplan/adrs/ADR-028-constellation-rust.md` | 31408 | `dda3529e5e53117fae6871a111db708880318fccf789700c797dc98906ff86bd` |
| ADR-029 | `.forgeplan/adrs/ADR-029-constellation-manifest.md` | 107444 | `97c5dad201ec9d06fd185ba3755098d2349b4a50aa96ab2ac5ef9c60f7ffb9ac` |

Before linking these were exactly EVID-189's 68280/57375/31408/107444-byte files. The first two gained exactly one trailing LF; both ADRs are byte-identical. Raw before/after comparison excludes any metadata or semantic change. All remain draft.

Actual ADI/validation input body SHA-256 values were, in the same order: `fefb134c2e782bc8456fcbcec2fe4a46d0821f72a40669e301422c076aca3ac7`, `5156c7242e4a7f9c26afcc5594ec62b506d1afeb0f5607987a758429328b6fc5`, `3bb043ad470211f37a9f58ec88e039ca31bc8b0c4542c6961da94870d5a3e6ff`, `56344a138ffd3530dca1dc9cd2979bbffa849bbf24932ef7667b480020607fe1`. Fresh pre-link CLI bodies matched exactly. Fresh post-link SPEC/RFC body hashes are respectively `9e446b725e565cd1e46d4eace1b022aa622ed36dee2de7cca02baf7027a894d5` and `11c1dcb3c248ef99c825580da5dbddfe41492860f8ddb082fa3189ae0f0e7673`, each differing only by one terminal LF. The ADI is not misrepresented as having consumed those later rendered bytes.

Required basis links likewise added only one trailing LF to EVID-188/189 canonical files: EVID-188 now 37707 bytes / `6a1e0a69f44fd3631c1553902d54daac0dbeeb6c58de9d3a42bf3e2df422d485`; EVID-189 now 25176 bytes / `e33d8334880db6476508087e64d37cb8653e4ec1e95158135e4fb7e2bfc28d99`. Their verdicts/meaning/status are preserved. EVID-180/181/183 and EVID-184–187 are byte-identical to this review's initial snapshots. No superseding link was created.

## Methodology and retained raw proof

Proof root: `.local/s1-static-admission-gate-20261021/`. Every fresh CLI command has argv/cwd/exit and raw stdout/stderr SHA-256/size receipts. Before/after canonical snapshots, `initial-fingerprints.json`, `after-link-fingerprints.json`, `post-link-body-comparison.json`, `source-fingerprint.json`, `binary-fingerprint.json`, `adi-provenance.json` and `validation-provenance.json` record the comparisons.

- EVID-189 manifest independently hashed to 11063 bytes / `459caa5196e26cdecee6a506b9d543d603b14edcfcba9d93bcf4be390f6b5756`.
- Final author raw root: `.local/s1-ownership-closure-coordinator/`; commands `./target/debug/forgeplan reason <ID> --json` and `validate <ID> --json`, explicit integration cwd, each retained exit 0. Raw ADI stdout retains the literal Analyzing preamble (53 bytes SPEC, 52 others), not silently rewritten as pure JSON.

| ID | Final ADI stdout file | Bytes / SHA-256 | Validation stdout file | Bytes / SHA-256 |
|---|---|---|---|---|
| SPEC-008 | `SPEC-008.scenario-heading-final-reason.stdout` | 16369 / `a784235466822356c59596807fbf07adb6e69ecd9cf62d7bd4b7f60bdb497470` | `SPEC-008.scenario-heading-validate.stdout` | 245 / `3f7bce2e145e29ee235b7186c36c03cefda55083680c8483cdaff87949348644` |
| RFC-015 | `RFC-015.final-reason.stdout` | 14700 / `ce8e49594501888c2c8df54e6e5e074b6c6d9c607ef35d5b1486656d60e159e7` | `RFC-015.final-validate.stdout` | 242 / `0672ca457688baeb03b0af1e26477f9b6a244a99494178ce482346de01bf4b32` |
| ADR-028 | `ADR-028.final-reason.stdout` | 15965 / `62b28f7406908558b4f735a1a5167b56f572e913745e61fac9549f6993651e29` | `ADR-028.final-validate.stdout` | 754 / `ff913f4f8f63f96918f68b613bc23c6e6ee9a2f9df6d3f2572f6826e1c7a0edc` |
| ADR-029 | `ADR-029.final-reason.stdout` | 18072 / `fe0f0154a800042fbbcf2a9d0087b2303ddff5988e0b93b9f137317529957a0e` | `ADR-029.final-validate.stdout` | 242 / `515ada569746e4f5bb477be29372c0c2150fa4b6050450376074d531683e99d7` |

All four ADI stderr files are 294 bytes / `ce235f79b24ca5525904fc417adbbbbc7068a042f97da25b43bf98e69c3be9ac`. The sole ADR-028 SHOULD is `body-links-drift` for incoming/context-only NOTE-053/RFC-015/SPEC-008 mentions; no fake dependency edges were added to silence it. No successful upstream run was repeated. This new EVID alone is validated before handoff.

No Cargo/build/check/test/linter/formatter, runtime probe, source edit, tracker write, commit, install, push, activation or source dispatch was performed. Hindsight and `mm-gate-failures` remain unavailable from the known compatibility-handshake failure; not retried to confirm it and not claimed consulted. That unavailable supplementary memory is not a fabricated new contract defect. Broad activation/whole-system audits are deliberately not substituted for the assigned static gate.

The CLI rejected the role's slash-separated claim identity (exit 1). Both claims were then acquired using `claude-code-omp-gpt-6-astra-guardian-task-EffNXWZmj4QGeFZgbYPR`, with the requested identity and actual harness recorded in the note; no foreign claim was stolen. Matching CLI release receipts are retained in the final handoff manifest.

## Blast radius

A wrong admission could leave an importer caller unmigrated, a descriptor/reader bypass in shared core, or a falsely placed barrier that invalidates later process proof. The future affected production scope is local CLI/MCP workspaces using this shared core, not a measured fleet percentage. **This decision itself changes only evidence and permits isolated source work; it changes no running service, source implementation or production data.**

Containment is the five exclusive worktrees, coordinator-only integration, frozen patches and independent review before one build owner. Before integration a changed contract or patch can be held for re-review; no timed rollback guarantee is asserted. Canonical Markdown remains authoritative; no blind CURRENT rollback or retired-table recreation is authorized. The actual scope matches the bounded contract. Downstream: S1-C → S1-R → S1-V/runtime → S1-G; S2 source remains behind S1-G. Detection comes from those mandatory later checks, not a measured detection time here.

## Orchestrator instructions

**PASS → the coordinator may dispatch only LocalReports, LocalCLI, MCPCompat, WholeBody and RuntimeProof under the exact accepted contract/ownership above and approved plan step 4. Do NOT activate any artifact. Do NOT mark S1-G, source implementation or runtime PASS. Do NOT dispatch S2–S4 source from this record.**

Retain task-runner-only tracking, one exclusive file owner/worktree from checkpoint `3bf82667789d8bad095645551f6d59c5f1bc478a`, correctly rooted references, frozen body/patch handoffs and NOT_RUN. Writers do not execute Cargo/tests/formatters or commit. Any ownership expansion or semantic contract change requires explicit bounded reassignment/re-review, not silent cross-slice editing.

After integration, independent source review → sole build-owner behavioral/feature checks → coordinator real CLI and stdio MCP verification → independent S1-G remain mandatory. Every applicable L01–L16 and AC-S1-1–4 observation must be tied to the new source/test/harness/model/tokenizer/binary fingerprint; mandatory FAILED/SKIP/NOT_RUN/zero-run cannot pass that later gate. Full workspace execution after S1–S4 and subsequent S2–S4 gates remain required. Helper-stall success cannot substitute for real-model shutdown or power-loss evidence.

GitHub #539 remains a separate observed post-checkout hook defect, outside these five source scopes (`setup.rs` is unassigned). The narrow 3-arguments → 0 git-sync-call probe is not L06/L07 freshness proof and the hook is not declared fixed. It is not silently added to this admission.

## Result and interpretation

The actual current static prerequisite is satisfied. No additional contract correction is required for EVID-188 F1–F3. Runtime readiness remains NOT_RUN and all four contracts remain draft. R_eff is not admission, readiness or activation authority.

## Congruence Level Justification

CL3 applies only to the same integration checkout, exact current canonical bytes, the independent closure evidence and fingerprint-bound raw author results. It makes no runtime, test coverage, native-hang, operating-system durability or release claim.

## References

- RFC-015 and SPEC-008: informs, bounded source-admission context only.
- EVID-188 and EVID-189: based_on, preserved negative findings and independent closure.
- ADR-028/ADR-029: current owner/protocol contract; PRD-085 Goal 2 / AC-2.
- Approved plan steps 3/4; AGENTS.md:246–299; proof files and exact hashes above.

