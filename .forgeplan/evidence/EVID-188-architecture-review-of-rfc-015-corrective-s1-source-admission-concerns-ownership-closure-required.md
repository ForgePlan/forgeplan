---
depth: tactical
id: EVID-188
kind: evidence
links:
- target: RFC-015
  relation: informs
- target: SPEC-008
  relation: informs
- target: EVID-181
  relation: based_on
- target: EVID-180
  relation: based_on
status: draft
title: 'Architecture review of RFC-015 corrective S1 source admission: CONCERNS - ownership closure required'
---

---
assigned_number: 188
created: 2026-10-20
predicted_number: 188
slug: evid-architecture-review-of-rfc-015-corrective-s1-source-admission-concerns
updated: 2026-10-20
---

## Structured Fields

evidence_type: audit
verdict: weakens
review_verdict: CONCERNS
congruence_level: 3

## Verdict

**CONCERNS — corrective S1 source admission is HELD.** This is not an RFC activation decision and not runtime acceptance. The four frozen drafts agree on the intended S1 behavior, but the exclusive file assignments do not cover required serialized-generation changes, real fault boundaries, and affected callers. SPEC-008 S1-R3 explicitly requires accepted exact ownership before dispatch. The parent has acknowledged these gaps and will revise the authoritative contract; that future revision is not reviewed by this record.

Three material findings survive the pre-report gate: two modular-boundary findings and one testability finding. They are locatable omissions in the current dispatch contract, not objections to style or evidence that corrective source work has already failed. No source writer is admitted by this evidence. Historical S1-A 10/10 remains accepted static work; no runtime PASS, draft activation, or S2–S4 source admission follows.

## Measurement

Independent artifact/source inspection for task `EffNXWZmj4QGeFZgbYPR`, reviewer `S1ArchitectureGate`, integration root `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, branch `feat/constellation-p0-contract`, governance base and observed HEAD `3bf82667789d8bad095645551f6d59c5f1bc478a`. The current local CLI, not mounted Forgeplan MCP, supplied all artifact reads and this EVID's mutations. No source or existing artifact body was repaired. No claim was taken under this assignment's new-EVID-only mutation restriction.

Reviewed the complete final canonical bodies of RFC-015, SPEC-008, ADR-028 and ADR-029; the complete PRD-085, including its explicit Constellation scope limitation; and EVID-180/181 as retained observations, not current runtime proof. Source references below are in this integration checkout, never the incorrectly rooted mounted Serena checkout. Read the available source interfaces and callers directly without starting LSP/watchers.

## Ground-truth verification

This dispatch claims a **working-tree contract delta before source implementation**, not a landed code change. An empty source diff is therefore expected, not the worker-claim/empty-diff failure rule. Git object base and HEAD are equal; the four uncommitted canonical contract files contain the reviewed delta and are individually fingerprinted. No code-provenance `base_sha/result_sha/changed_paths` claim is made for this audit.

Executed through `bash --noprofile --norc -c` in the integration root:

```sh
set +u
printf 'ROOT='; git rev-parse --show-toplevel
printf 'HEAD='; git rev-parse HEAD
printf 'BRANCH='; git branch --show-current
printf '\nCOMMITTED_DELTA\n'; git diff --stat 3bf82667789d8bad095645551f6d59c5f1bc478a..HEAD
printf '\nSTAGED_DELTA\n'; git diff --cached --stat
printf '\nWORKTREE_CONTRACT_DELTA\n'; git diff --stat 3bf82667789d8bad095645551f6d59c5f1bc478a -- .forgeplan/specs/SPEC-008-constellation-0-0-embeddings.md .forgeplan/rfcs/RFC-015-constellation-0-0.md .forgeplan/adrs/ADR-028-constellation-rust.md .forgeplan/adrs/ADR-029-constellation-manifest.md
printf '\nSOURCE_DELTA\n'; git diff --stat 3bf82667789d8bad095645551f6d59c5f1bc478a -- crates eval Cargo.toml Cargo.lock
if git diff --quiet 3bf82667789d8bad095645551f6d59c5f1bc478a -- crates eval Cargo.toml Cargo.lock; then printf 'SOURCE_DELTA=EMPTY\n'; else printf 'SOURCE_DELTA=PRESENT\n'; fi
if git diff --quiet 3bf82667789d8bad095645551f6d59c5f1bc478a -- .forgeplan/specs/SPEC-008-constellation-0-0-embeddings.md .forgeplan/rfcs/RFC-015-constellation-0-0.md .forgeplan/adrs/ADR-028-constellation-rust.md .forgeplan/adrs/ADR-029-constellation-manifest.md; then printf 'CONTRACT_DELTA=EMPTY\n'; else printf 'CONTRACT_DELTA=PRESENT\n'; fi
```

Literal stdout, exit 0:

```text
ROOT=/Users/explosovebit/Work/ForgePlan-Constellation-P0
HEAD=3bf82667789d8bad095645551f6d59c5f1bc478a
BRANCH=feat/constellation-p0-contract

COMMITTED_DELTA

STAGED_DELTA

WORKTREE_CONTRACT_DELTA
 .forgeplan/adrs/ADR-028-constellation-rust.md      | 113 +++++++-----
 .forgeplan/adrs/ADR-029-constellation-manifest.md  | 197 ++++++++++++++++-----
 .forgeplan/rfcs/RFC-015-constellation-0-0.md       | 108 +++++++----
 .../specs/SPEC-008-constellation-0-0-embeddings.md | 163 ++++++++++++++---
 4 files changed, 434 insertions(+), 147 deletions(-)

SOURCE_DELTA
SOURCE_DELTA=EMPTY
CONTRACT_DELTA=PRESENT
```

Expected contract tokens were independently FOUND by the specialized grep tool with pattern `health_report_captured|section_layout_fingerprint|after_sections_commit|suite=workspace` on RFC-015: lines 180, 198, 210, 212. Exact representative matches include:

```text
180:- `health::health_report_captured(read) -> anyhow::Result<HealthReport>`
210:Общие stage names для реализации и harness: after_artifacts_commit, after_relations_commit, after_sections_commit, after_storage_sync, before_manifest_sync, after_manifest_sync, before_current_replace, after_current_replace, after_current_ack, owner_acquired, native_call_entered. Название подтверждает фактическую точку исполнения, не событие, посланное заранее.
```

The second excerpt stops at a sentence boundary; the canonical snapshot and full diff preserve the rest. This is a contract-presence probe, not proof those functions/hooks exist. The complete real diff was retained using `git diff --no-ext-diff <base> -- <four canonical paths>`: 231247 bytes, SHA-256 `d443527fbeeb5ed06c7e175323d2f7d7812b32d2d448a6e99e7f3021a0a9fe8f`.

## Scope and methodology

- **RFC:** complete RFC-015, especially corrective local APIs, WholeBody layout, RuntimeProof, ownership, risks, pre-code admission versus activation.
- **Parent acceptance:** PRD-085 Goal 2 / AC2, lines 109–111 and explicit scope mapping at 155–183. Pull, checkout and rebase remain distinct obligations. AC1/3/4/5/6 are explicitly not closed by Constellation; this review does not manufacture drift from that justified scope.
- **Companion contracts:** complete SPEC-008 S1-R1–R4, L01–L16, AC-S1-1–4; ADR-028 embedding/read boundaries; ADR-029 generation, workers, ownership, publication and process proof.
- **Source:** `constellation/{capture,jobs,manifest,facade,runtime,publisher,lance_tables,storage,view,reconciler,generations}.rs`, relevant durability code, tokenizer/embedding interfaces, local report/scoring/coverage seams, auxiliary FPF, importer and its CLI callers, twelve MCP refusal sites, runner inventory/fingerprint definitions. Inspection was targeted to these interfaces, not a full security or implementation audit.
- **Priors:** EVID-181 retained negative static review and EVID-180 retained runtime observations. EVID-183's historical matrix is not a new run; the accepted count remains 5673 executable cases plus 10 doctests. EVID-184–187 remain governance-only history and are not S1 runtime support.
- **Memory:** Hindsight compatibility handshake is known unavailable in this assignment; no recall or `mm-gate-failures` result is claimed. Author ADI is supplementary model reasoning, not an external oracle.
- **Categories applied:** modular boundary, coupling, data flow, blast radius, operability, scalability, testability. Step 5 was the reviewer's mental reasoning, not `forgeplan reason`.

### Static analyser execution boundary

| Tool / operation | Availability | Execution / exit | Reason |
|---|---|---|---|
| `cargo tree`, builds, checks, tests, formatters | Cargo installed | NOT_RUN / — | Explicit assignment prohibits Cargo/check/test execution before source dispatch. |
| `cloc` | Installed | NOT_RUN / — | This assignment is a bounded contract/interface review, with no check execution. No LOC or dependency fitness result is fabricated. |
| `madge`, `licensee` | Not installed | skipped / — | No installation attempted; JS graph analysis is not the Rust contract oracle. |
| source/reference reads and git ground truth | Available | executed; git exit 0 | Direct source evidence and frozen bytes only. |
| fresh CLI artifact GET | Available | all seven exit 0 | Four contracts, parent PRD and two retained observations; raw receipts retained. |
| author final ADI/validate | Existing logs | read, not rerun | Actual final logs and exit receipts checked; not reviewer-executed ADI. |

## Parent-PRD fit and preserved requirements

| Requirement | Contract location | Architectural assessment |
|---|---|---|
| PRD-085 AC2, no manual refresh after pull/checkout/rebase | RFC P1/P2/P5; SPEC L01–L08; PRD scope mapping | Covered normatively by complete read-only capture and CURRENT verification. All three Git operations still need separate runtime observations; no current PASS. |
| PRD AC1 and AC3–AC6 | PRD Constellation scope table | Intentionally outside this gate, not silently declared complete. |
| PRD Non-Goals | RFC Non-Goals; ADR-028; ADR-029 | No LanceDB-in-git, CI model load, central SaaS/store or multi-agent-product rewrite is introduced. |
| S1-R1 / AC-S1-1 | RFC 178–193; SPEC 36–59 | Eight async borrowed APIs and twelve actual MCP paths agree; semantics/scoring and file-first writers are preserved. Ownership gap F3 concerns the shared importer caller, not a new API design. |
| S1-R2 / AC-S1-1 | SPEC 61–68; RFC 193 | Canonical-first import, unknown frontmatter/body/links/author/status, id allocation lock, WorkspaceLock, one successful-batch reconcile, dry-run and repeat semantics are explicit. |
| Full-body / AC-S1-2 | SPEC 84–106 and L12; RFC 195–204; ADR-029 147–178 | Immutable metadata-only tokenizer/layout, exact UTF-8 reconstruction, actual token limits, same builder and exact current section set are consistent. Missing metadata means zero current sections, not old active rows. |
| Three-table publication / AC-S1-3 | SPEC 102–106, L09/L10; ADR-029 169–189 | ARTIFACTS/RELATIONS/SECTIONS, never SCORE. Mandatory pins and equality require the missing serialized-contract owner in F1. |
| S1-R3 / AC-S1-3 | SPEC 297–307; RFC 206–212; ADR-029 §8 | Eleven names and newline protocol agree. The direct hook placement claims are not closed by the assigned files: F2. |
| S1-R4 / AC-S1-4 | SPEC 309–329; RFC 212–214 | Phase/workspace inventory, true exits, executable counts, newly owned unstaged files and fingerprint invalidation remain mandatory. File count is explicitly not execution completeness. |
| L01–L16 and feature/surface separation | SPEC sixteen named scenarios | None removed or weakened by this review: metadata/body/delete/rename; same mtime-size changed bytes; Git operations; watcher loss/overflow; failed source/no-delete; process crash; ownership; C→A→B/stale/incarnation/profile; full-body; isolation; coalescing/recovery; stale top-K; actual model/channel/native behavior. |
| Stage sequencing | RFC ownership paragraph; SPEC S1-R4 | S2–S4 source dispatch remains behind the prior G PASS. Later-suite declarations are not proof of execution. |

### Eight shared APIs

All are `async`, borrow `&WorkspaceRead`, retain the existing remaining parameter types/defaults, and return `anyhow::Result<…>`:

| Function | Result payload |
|---|---|
| `health::health_report_captured(read)` | `HealthReport` |
| `health::health_report_with_phase_captured(read, workspace)` | `(HealthReport, Vec<PhaseMismatch>)` |
| `journal::build_journal_captured(read, kind_filter, risk_only)` | `Vec<JournalEntry>` |
| `scoring::decay::decay_report_captured(read)` | `Vec<DecayEntry>` |
| `drift::check_drift_captured(read, project_root)` | `Vec<DriftReport>` |
| `anomalies::detect_anomalies_captured(read, workspace, filter)` | `AnomalyReport` |
| `release_notes::generate_captured(read, repo_root, since, until, draft)` | `ReleaseNotes` |
| `fpf::check_artifact_against_rules_captured(read, artifact_id, fpf_config)` | `Option<RuleCheckResult>` |

Direct source matches the intended existing payloads: health/mod.rs:559–584, journal/mod.rs:20–24, scoring/decay.rs:27, drift/mod.rs:28–31, anomalies.rs:261–265, release_notes.rs:631 onward, fpf/mod.rs:360 onward. Existing `AssuranceSource::Read` and `r_eff_recursive_captured` are in scoring/reff.rs:267–357; `coverage::build_coverage_from_records` already exists. Reusing them preserves one engine. Journal/decay's evidence-link semantics and health phase mismatches must not be simplified away. No `JournalReport`, second scoring engine, generic async closure framework, or rewrite of working file-first score/activate/reason/ingest is authorized.

The twelve production `require_legacy_projection` calls in MCP server.rs match the contract. The label at :7759 is `forgeplan_fpf_rules` inside the search handler; genuine config-only rules is not a thirteenth migration. KB read operations use existing `AuxiliaryStore::open_existing`, not a creating opener. `open_workspace_read` already rejects incomplete capture at facade.rs:257–265 and propagates CURRENT/pin failures through :194–221; the correction must retain these semantics rather than synthesize empty successful reports.

### Tokenizer, data-flow and pending decision

The four contracts bind layout algorithm/version, three metadata checksums, actual `max_tokens <= 512`, header limit 128, and availability into the domain-separated layout fingerprint, both capture passes/config digest and embedding profile. Raw source/input/profile hashes remain separate. No weights, ONNX plan or network is needed to parse metadata; one immutable tokenizer/layout belongs to a capture in both feature modes, with the onig/C-toolchain implication stated. Missing or invalid metadata has a distinct unavailable fingerprint and exactly zero current section rows. Valid metadata without semantic feature can form layout but cannot claim encode readiness.

The builder covers every body byte, including preamble, markup, whitespace and tail; ignores special `(0,0)` offsets as boundaries; keeps full display title/heading; bounds the final prefix/context/separator/special-token input against the same actual engine limit; and makes progress even when context must shrink. No repeated `body_prefix_chars` clipping remains in section capture/digest/admission/recovery. The separate artifact-prefix channel remains separate. Direct source references to the changed section-input signatures are confined to capture/jobs/facade/runtime and their tests; a second builder or runtime engine is unnecessary. Existing cached query worker/CallGate at runtime.rs:516–553 supplies the required native seam.

**Pending representation decision:** `manifest.rs:121` is `BTreeMap<String, EmbeddingJob>`, not a type-enforced artifact-only map. `EmbeddingJob` is serde-derived in jobs.rs:206–221, with scalar key/input/profile/ref fields; recovery at runtime.rs:2588–2605 explicitly handles artifact OR section keys. It is incorrect to claim current parent coalescing already exists. The map container can retain its type while its jobs/runtime-owned value/validation semantics evolve; the map declaration alone does not require a new queue engine or a new map type. That observation does **not** remove the need to own manifest.rs: the actual generation schema/pin validation and constructors are there, and its job/capture literals also need migration. One parent entry must retain all current section refs/bindings and outstanding completion/retry state; completing one window or prefix cannot remove other pending windows. Those behaviors remain future implementation/test obligations.

## Findings

| # | Severity | Category | Concrete location and condition | Consequence | Required closure |
|---|---|---|---|---|---|
| F1 | HIGH | 🏗 Modular boundary | RFC-015 ownership table :296–300 omits manifest.rs/view.rs; SPEC-008:104–106 requires CURRENT/manifest exact mandatory triple. Current manifest.rs:44–77 accepts missing sections, :350–387 has no descriptor triple and independent embedding heads, :434–479 accepts heads newer than manifest pins; :741–748 explicitly tests pre-sections acceptance. view.rs:271–278 and :569–585 expose/test that overlay. | WholeBody cannot implement the normative serialized generation and clean reader cutover solely in its assigned files. Retaining the old acceptance/overlay would violate the corrected protocol; silently editing unowned files violates the pre-dispatch boundary. | Assign WholeBody manifest.rs and view.rs for this exact schema/validation/reader migration and the affected fixtures below; retain explicit unsupported-old-generation errors, never a compatibility fallback. |
| F2 | HIGH | 🧪 Testability | ADR-029:246–250 names publisher.rs for pre-manifest-sync and CURRENT internals; RFC owns only the top-level caller. Actual manifest bytes→fsync is durability/mod.rs:403–409, Windows :355–358; actual CURRENT temp/replace/ACK boundaries are storage.rs:909–925. publisher.rs:491–500 sees whole calls, not those internal points. | A hook merely before/after the outer call can emit the correct name at the wrong operation, making SIGKILL evidence falsely satisfy L09/S1-R3. The contract itself requires ownership reassignment before such a hook. | Assign storage.rs and the actual immutable-write helpers to WholeBody for test-helpers-only boundary calls; correct the placement table and retain separate platform limits. No production env blocker or fake notification. |
| F3 | MEDIUM | 🏗 Modular boundary | RFC-015:193 changes `scan_and_import_to_workspace` to optional legacy projection, but :297 omits CLI commands/init.rs. That real caller is init.rs:1032; :1018–1025 skips migrated imports because the old implementation is unavailable. | A mandatory shared caller remains outside the migration assignment; `init --scan` can retain a misleading legacy-only skip after the shared importer is corrected, or its typed call stops matching. | Assign only the import-adapter path in commands/init.rs to LocalCLI, including migrated invocation/locking and caller regression coverage; do not rewrite unrelated initialization behavior. |

These are current contract-fitness findings, not a request to fix code during this review. The parent acknowledged them and held dispatch; acknowledgement alone does not amend the frozen canonical ownership matrix.

## Minimal ownership closure

Paths below are required operations/caller boundaries, not a grant of access by this reviewer. The coordinator must record them in the canonical contract before the five writers start.

| Proposed existing owner | Concrete additional path | Bounded required operation |
|---|---|---|
| WholeBody | `crates/forgeplan-core/src/constellation/manifest.rs` | CURRENT/manifest triple serialization/equality/validation, missing-section rejection, pending/capture constructor and embedded test migration. The map can stay typed as existing jobs; no second queue engine. |
| WholeBody | `crates/forgeplan-core/src/constellation/storage.rs` | Real CURRENT temp/replace/namespace-ACK boundaries and descriptor/manifest validation; no synthetic stage at the outer publisher call. |
| WholeBody | `crates/forgeplan-core/src/constellation/durability/mod.rs` | Manifest bytes-written/before-sync stage inside the existing immutable installer. Scope the hook to the intended manifest operation, not every durable marker. |
| WholeBody | `crates/forgeplan-core/src/constellation/durability/windows.rs` | The corresponding actual write-complete/before-FlushFileBuffers boundary if Windows test-helper stage support is implemented. This file is used directly on Windows, bypassing the POSIX injected platform path; this review grants no Windows execution claim. |
| WholeBody | `crates/forgeplan-core/src/constellation/view.rs` | Direct reader overlay accessor/expectations tied to the old descriptor fields, retained-layout capture literal :490 and pair-only manifest/CURRENT fixtures :729–763. No general read-model redesign. |
| WholeBody | `crates/forgeplan-core/src/constellation/reconciler.rs` | Embedded fixture migration only: `CapturedSourceView` literal :327 and two-version constructor :385. No new reconciliation algorithm requested. |
| WholeBody | `crates/forgeplan-core/src/constellation/generations.rs` | Embedded pair-only manifest/CURRENT fixture migration at :550–630; no retention/GC redesign. If the chosen representation changes `TableVersions.sections`' type, its direct projection at :200 must be mechanically adapted too; no other production extension follows. |
| LocalCLI | `crates/forgeplan-cli/src/commands/init.rs` | Existing `run_scan_import` caller at :1001–1032 only, with its regression coverage. |

Make existing impacted core test ownership explicit under WholeBody: `crates/forgeplan-core/tests/constellation_embedding_preservation.rs`, `constellation_pinned_search.rs`, and `constellation_runtime_e2e.rs` contain profile/descriptor/overlay API uses. RuntimeProof keeps separate new process/fault targets, not simultaneous edits to these files. Its new helper-gated target registrations, if required, go through the existing core Cargo.toml owner. Do not grant extra CLI/MCP manifest ownership merely because tests are being added; their automatic test discovery has no demonstrated registration gap here.

`constellation/export.rs:181–185` calls the existing `effective_artifacts_version` returning `Result<u64>`; strict validation can remain behind that interface. No new federation/export behavior or S2 source dispatch is needed to close this call. No need was found to assign owner.rs: publisher.rs:301 returns from actual `acquire_owner` before publishing, and a hook in that assigned caller observes the real lock. Native CallGate/permit placement is already in assigned runtime.rs.

## Eleven-stage placement assessment

All rows are future hooks, not exercised observations. Parent announcement JSON is newline `{token,stage,pid}`; release is newline `{token,stage,release:true}`. Both ends validate correlation/stage, the parent checks the actual child pid, and timeouts bound waits. A release ACK is not a storage durability ACK. The harness must cause a real required operation; unchanged/no-commit paths cannot emit a fabricated commit stage.

| Stage | Actual source boundary / required condition |
|---|---|
| `after_artifacts_commit` | Owned lance_tables.rs:421–434 / facade.rs:989–991, after the awaited physical commit and version result, CURRENT still baseline. |
| `after_relations_commit` | Owned lance_tables.rs:466–479 / facade.rs:1025–1027, after its actual table commit, before next publication stage. |
| `after_sections_commit` | Owned lance_tables.rs:500–513 / facade.rs:1047 onward, after exact-set section commit, including a real delete for an empty replacement when needed; before closure durability. |
| `after_storage_sync` | Owned publisher.rs:485–487, after joined writes and successful complete WriteSet/PlatformDurability closure for all three tables; not merely after `finish_writes`. |
| `before_manifest_sync` | Manifest bytes complete, before file durability: durability/mod.rs:403–409; Windows helper :355–358. Additional ownership required. |
| `after_manifest_sync` | storage.rs:865–866 has completed immutable installation and directory/platform durability; publisher.rs:492 has returned, CURRENT still old. Existing publisher caller is sufficient for the after-stage. |
| `before_current_replace` | storage.rs:909–921, after complete checksum-bearing temp and successful temp sync, before atomic replace. Additional storage ownership required. |
| `after_current_replace` | storage.rs:921–925, visible replacement, before caller's full publication ACK. On Windows the write-through replace itself includes namespace durability; distinguish its return from later descriptor/pin verification, do not invent a pre-durability window after it. |
| `after_current_ack` | Only after platform ACK plus successful descriptor/checksum/all-three-version verification. publisher.rs:500 returning alone does not prove that full verification; the assigned publisher/facade/storage path must finish it before announcing the stage. |
| `owner_acquired` | publisher.rs:301–302 after successful real OS-owner acquisition, or its actual runtime/facade caller. Reuse the same stage for takeover after killing the previous owner; no owner.rs edit is forced. |
| `native_call_entered` | runtime.rs:527–531 after NativeCallPermit acquisition and successful existing CallGate entry, before :549–551 encode, with permit/gate still held. Not a queued future or a pre-admission signal. |

Fresh-process inspection after kill must show complete old/new generation or typed unavailable, never a mix. `after_current_ack` expects the acknowledged generation within the supported process-crash boundary. The helper-controlled native stall proves actual worker/gate occupation and process response/exit under deadlines, not a naturally hung ONNX library. A separate helper-free production build must exercise actual encode, semantic positive/negative controls and shutdown during real work. SIGKILL is not power-loss proof; macOS is not Windows proof. No production daemon, transport or env-controlled blocking is admitted.

## Harness and operational bounds

The required phase/workspace design is coherent: phase remains default, workspace discovers supported test/library/bin targets from Cargo metadata and runs serially; missing target, empty mandatory phase, zero executed tests and required SKIP/NOT_RUN cannot be PASS. Compile-only is explicitly NOT_APPLICABLE with a reason. Current run_acceptance.py:81–148 and :704–745 are the existing phase baseline, not evidence that `--suite` already exists. Its current fingerprint globs at :185–197 are not the frozen-manifest oracle; SPEC S1-R4 requires exact tracked roots plus explicitly owned unstaged/new files, sorted paths and bytes, before/after source/test/harness/lock hashes, actual executable counts/exits and model/tokenizer/binary identities. No scanner or fingerprint-file count substitutes for requirement→case→executable→observation coverage.

### Blast radius

- **If the contract is implemented incorrectly:** every local read/search/graph/score depending on a misbound CURRENT can report the wrong generation; full-body section deletion or pending recovery can lose fresh semantic results or expose stale ones. Import adapter omissions affect CLI import callers; incorrectly placed barriers invalidate the claimed crash proof rather than prove production failure.
- **Production scope:** the workspace/index handled by the affected local CLI or long-lived MCP process; shared core code reaches all such users. No fleet percentage was measured. Federation and S2–S4 implementation remain outside this admission.
- **Recovery:** canonical Markdown stays authoritative. Stop inappropriate writers, preserve uncertain state, explicitly inspect/repair or rebuild derived index using the documented maintenance path. Do not roll CURRENT back blindly after a visible but uncertain replacement, recreate retired legacy tables, or treat old vectors as current.
- **Detection:** mandatory complete-capture/CURRENT errors, publication uncertainty, semantic coverage/pending diagnostics and the future L09/L10/L12/L14–L16 probes. No measured detection time or runtime SLO is claimed by this inspection.

### Operability concerns

The contract distinguishes missing metadata, feature unavailable, model failure/retry, incomplete source, corrupt CURRENT and publication uncertainty. It keeps read-only behavior and explicit maintenance, bounded queue/cooldown/deadline values, and separate cold CLI versus warm in-process MCP measurements. Those are sound requirements, not measured capacity or shutdown evidence. Mixed-version preflight/refusal, actual storage closure completeness, per-platform durability and representative-corpus capture cost remain implementation/runtime acceptance work. Existing file-first mutation paths are retained, not redesigned.

## Positive observations

- The corrected contract separates raw/source/input/profile/layout identities and requires the same builder across capture, digest, admission and recovery; it also preserves display text independently of embedding context.
- Existing WorkspaceRead and AssuranceSource are reused with the actual old response types. The FPF-search/rules mislabel is correctly distinguished, avoiding a false thirteenth migration.
- The process-proof contract explicitly separates helper stall from real inference, replacement visibility from ACK, current sections from cached vectors, and static S1-A from runtime S1-G. Those distinctions should survive the ownership correction.

## Result

Normative S1 behavior is consistent across the four reviewed drafts, but the current exact path set is incomplete at three concrete seams. Current review verdict is CONCERNS and source admission remains HELD. No reported author test or ADI score was substituted for source or runtime evidence. No build, test, linter, formatter, installation, source edit, tracker write, commit, push or activation occurred in this review.

## Interpretation and next steps

The coordinator should record the bounded additional owner assignments and correct the actual stage placement mapping, retain the other slices' disjoint ownership, then run the approved final-delta author ADI/validation and request a focused independent re-review. This EVID does not automatically approve that later revision. Source dispatch remains held until the current contract is accepted. After implementation, independent source review, the single build owner, real CLI/stdio verification and guardian S1-G remain required; S2 starts only after that gate.

## Congruence Level Justification

CL3: direct inspection of the exact integration checkout, frozen canonical contracts, real source interfaces and actual retained command output. This congruence applies to **static contract fitness**, not runtime behavior. `verdict: weakens` records that unconditional source admission is unsupported; `review_verdict: CONCERNS` is the review result. There is no automatic zero-findings concern here: the three findings are material. R_eff is artifact assurance bookkeeping, never a readiness score or permission to activate.

## Raw observations and fingerprints

All reviewer proof files live under `.local/s1-architecture-gate-20261020/`. `initial-fingerprints.json`, four `*.before.canonical.md` snapshots, `contract-diff.stdout`, `ground-truth.stdout`, seven fresh GET stdout/stderr/receipts, `source-inventory.json`, `author-result-provenance.json`, and `adi-observations.json` retain the source of this review. Command/cwd/exit/stdout/stderr are recorded, not reconstructed from a success summary.

- CLI binary: `target/debug/forgeplan`, 461012344 bytes, SHA-256 `96cd31a566e92981e99f558462ad1ee7da17bed00daf9db8db3e71dca9b688ea`.
- Static source inventory: 403 files, length-prefixed relative-path-and-byte aggregate SHA-256 `9cb29eb8e56a412c8195ef67772a955457d11040c9d867721a8a576c9e7ac66e`; exact selected paths are in source-inventory.json. `git ls-files --others --exclude-standard -- crates/forgeplan-core crates/forgeplan-cli crates/forgeplan-mcp eval Cargo.toml Cargo.lock` returned empty, exit 0. This is pre-source provenance, not an execution count.

### Author ADI raw outputs personally read

Each is the existing actual `./target/debug/forgeplan reason <ID> --json`, cwd integration, receipt exit 0. Raw stdout contains a 52-byte preamble (SPEC 53 bytes) before JSON; that preamble is preserved. No reason call was rerun by this reviewer. The actual stderr is 294 bytes, SHA-256 `ce235f79b24ca5525904fc417adbbbbc7068a042f97da25b43bf98e69c3be9ac` for each.

| Artifact | Raw stdout path | Bytes | SHA-256 |
|---|---|---:|---|
| SPEC-008 | `.local/s1-spec-delta-20261020/reason-accepted.stdout` | 18930 | `317c4c03af9e9ec04e022c425aabd6ac3594e9548768a9a189987ecd23bdbefb` |
| RFC-015 | `.local/planning/constellation-s1-corrective-contract/coordinator/rfc015-verified-final.reason.stdout.log` | 19290 | `503f08a1c3ecc3981412a6050977bd2a2b533d253194144f7d8755c1f6731139` |
| ADR-028 | `.local/s1-adr-delta-20261020/ADR-028.reason-final.stdout` | 15392 | `e5da7e8ba2ee3639ca1eb4d61692b7180fe7e07cd36792d96eabbf58fae85872` |
| ADR-029 | `.local/s1-adr-delta-20261020/ADR-029.reason-final.stdout` | 17520 | `9d7f9fd1fb0f2322b9a15c7459aa520e057ca845c171561577d2c6a86e2c0972` |

All four recommendations retain draft and require independent ownership/interface acceptance followed by runtime evidence. Their confidence is explicitly confidence in a course of action, not implementation correctness. The actual TractEmbedder is in tract_engine.rs; older fastembed context is not a mandate to introduce another engine.

### Existing final validations, not rerun

All receipt exits are 0. SPEC/RFC/ADR-029 have 0 MUST and 0 SHOULD; ADR-028 has 0 MUST and one intentionally retained `body-links-drift` SHOULD for incoming/context-only mentions of NOTE-053/RFC-015/SPEC-008. This review does not create fake dependency edges/cycles to silence that warning.

| Artifact | Raw stdout path | Bytes | SHA-256 |
|---|---|---:|---|
| SPEC-008 | `.local/s1-spec-delta-20261020/validate.stdout` | 298 | `eedb16b05a7e33120dbb7d11f3dfbfb223db20e9731db5e3e772a308ecba2b1a` |
| RFC-015 | `.local/planning/constellation-s1-corrective-contract/coordinator/rfc015-verified-final.validate.stdout.json` | 242 | `0672ca457688baeb03b0af1e26477f9b6a244a99494178ce482346de01bf4b32` |
| ADR-028 | `.local/s1-adr-delta-20261020/ADR-028.validate-final.stdout` | 807 | `314ef44c4a5076ba8f36be66906ae7c0778c99949011901c3f35b5e64c920f23` |
| ADR-029 | `.local/s1-adr-delta-20261020/ADR-029.validate-final.stdout` | 295 | `1ed9520cf33108e2ffb70348f9fefd99835624a0963f6435206abc8718e381b2` |

### Reviewed canonical contract hashes before link rendering

| Contract | Bytes | SHA-256 |
|---|---:|---|
| SPEC-008 | 64681 | `b498d3f7c14153c1051993efd23e03fa5fbc81009ffcb6e6976046879c9b281c` |
| RFC-015 | 52847 | `1afe2f84c9fee60d8014896a45698133355daafd966fdbe9150be2370ad3aee8` |
| ADR-028 | 26935 | `89dbc79e38275f3e3b646f8c2c4a04e1f79fc32c7ebfa01d485daf1e3bf7bdb8` |
| ADR-029 | 102789 | `783aca64a691997988ffedfbca6219fa9c9725126a62e1c5ed35cbbdc33058c3` |

## Residual risks

No current L/F runtime row was executed. The read set is sufficient for the named boundary findings, not a claim of complete writer/storage interception, security, performance, OS durability or test coverage. Native behavior, unsupported filesystem handling, old-format refusal and real-model tail retrieval still require the approved post-code gates. A later contract/source fingerprint invalidates the applicability of this admission verdict until re-reviewed.

## References

- Under review: RFC-015 and SPEC-008; parent PRD-085, explicitly Goal 2 / AC2 only.
- Companion decisions read in full: ADR-028 and ADR-029.
- Retained observations used: EVID-181 and EVID-180. New evidence informs the RFC/SPEC and is based_on these observations; neither is superseded.
- EVID-183 and EVID-184–187 are preserved historical context, not newly exercised S1 evidence.

## Final canonical fingerprints after this EVID's link writes

The four CLI link calls all exited 0 with no stderr: EVID-188 `informs` RFC-015 and SPEC-008, and `based_on` EVID-181 and EVID-180. No auto-activation flag was passed. Actual post-link canonical bytes were compared with the retained before snapshots, not merely the previous handoff hashes.

| Contract | Final bytes | Final SHA-256 | Observed change |
|---|---:|---|---|
| SPEC-008 | 64682 | `80a76913fe7473f250413466135c085e60b1ecc4d66bf234b9870dd316ef1d9e` | Exactly one trailing LF added by CLI target re-render |
| RFC-015 | 52848 | `30945ea7fb50333c22bb16ec1095e07f9b2c64f23611d1910c14ccf02dc30f70` | Exactly one trailing LF added by CLI target re-render |
| ADR-028 | 26935 | `89dbc79e38275f3e3b646f8c2c4a04e1f79fc32c7ebfa01d485daf1e3bf7bdb8` | Byte-identical |
| ADR-029 | 102789 | `783aca64a691997988ffedfbca6219fa9c9725126a62e1c5ed35cbbdc33058c3` | Byte-identical |

There was no metadata or semantic body rewrite in any of the four contracts; all remain draft. EVID-180 and EVID-181 likewise gained only one trailing LF through the required `based_on` link target re-render, not an authored change to their observations or verdicts. Their final bytes/hash are respectively 19749 / `b5296cd212e142f6c3dc4cbaa66067a42c18cd57c26d22e8e86cad4803cd77e4` and 6338 / `eb92351ff525632e769ab751bc733618dc30105f5d77aca9fa32f62168f8e365`. EVID-183 and all of EVID-184–187 are byte-identical. No stale mounted-MCP body was written back.

`after-link-fingerprints.json` records every before/after classification; `final-fingerprint-manifest.json` is the handoff manifest including this EVID, final contract/history hashes, source provenance and actual CLI receipts. `proof-file-digests.json` records raw proof SHA-256 and size. These records are in the same reviewer proof directory, outside `.forgeplan`; canonical EVID content is written only through the fresh CLI.



