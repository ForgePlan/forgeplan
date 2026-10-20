---
depth: standard
id: EVID-184
kind: evidence
last_modified_at: 2026-10-20T19:11:54.772532+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: draft
title: 'RFC-015 governance frozen-patch review: CONCERNS (hook executor contract)'
---

---
assigned_number: 184
created: 2026-10-20
predicted_number: 184
slug: evid-rfc-015-governance-frozen-patch-review-concerns-hook-executor-contract
updated: 2026-10-20
---

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: code_review

## Verdict

CONCERNS

One material documentation finding remains: the exclusive Cargo-executor rule has no explicit exception for the integrator's mandatory Cargo-running commit hook. The frozen patch otherwise meets the inspected governance/configuration requirements. This is a governance-only review, not an S1 source, Rust, Constellation readiness, or activation verdict.

## Scope

- Parent: RFC-015, linked only as the program's governance prerequisite; approved corrective plan step 1 is the detailed acceptance contract.
- Task UID: Z8cucrnH8xIBowclNr5o.
- Reviewer: GovernanceReview; audit identity: claude-code/6.1-sol/code-reviewer-task-Z8cucrnH8xIBowclNr5o; actual runtime/model: OMP, openai-codex/gpt-6.1-sol. The identity namespace is the required audit convention, not a claim that Claude Code executed this review.
- Author worktree: /Users/explosovebit/Work/ForgePlan-ready-governance.
- Explicit evidence workspace: /Users/explosovebit/Work/ForgePlan-Constellation-P0.
- Frozen base: d3d938da0ef53a82064937c98d4302cf97318deb. Both worktrees resolved to this HEAD when inspected.
- Review input: /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch, 20749 bytes, SHA-256 03193b688130160bc628401df9e1b6ddf32c1ba03e9c0a843307092d14759efe.
- Files reviewed in full: 3 files / 621 lines: AGENTS.md (471), docs/agents/issue-tracker.md (131), docs/agents/orchestra.json (19).
- Frozen patch delta: +218/-6; exactly these three owned paths, no Rust/source/lockfile change in the patch.
- Supporting reads: current CLI command implementations, local link targets, installed role definitions, installed verifier argument handling, effective shared Git pre-commit hook, and RFC-015. Supporting Rust was read only to check the documented command meaning, not audited as S1 source.

## Tools run

| Tool / inspection | Exit / outcome | Boundary |
|---|---|---|
| shasum -a 256 and wc -c on frozen patch | 0; exact supplied digest and 20749 bytes | File identity only |
| Clean-shell git rev-parse, status, diff, git apply --stat/--numstat, hash-object --no-filters | 0 | No patch application, add, commit, config write, checkout, or hook execution |
| In-memory unified-diff reconstruction from git show of frozen base | MATCH for all 3 files | No materialized file, Git index, source, or worktree write |
| Static JSON parsing and exact comparison with approved schema-v2 object | MATCH | No network/auth operation; tokenEnv name only |
| Static local Markdown target and anchor resolution | 11 valid, 0 broken | Author-worktree files, not another checkout's tracker contract |
| Invisible/bidi control scan on the three owned files | none observed | Not a general secret scanner or comprehensive Unicode-confusable audit |
| Existing integration CLI --version and index/reindex/scan-import --help | 0 for the chained invocation | Help only; none of those mutation commands was executed |
| Read/Grep/Glob of scoped documents, source, verifier, role definitions and effective Git hook | completed | Static inspection only; mounted Serena was not used |
| Cargo/build/test/lint/type-check/rustfmt/formatters | NOT_RUN, expressly excluded | No Rust compilation/test execution or new binary |
| Commit hook | NOT_RUN | Read actual hook bytes only |
| Orchestra shell verifier and mounted tracker handshake | NOT_RUN by reviewer | Dispatch reports the parent already observed exit 0 / MATCH; that is attributed context, not independent execution here |
| Tracker writes / messages / commits / artifact activation | NOT_RUN | None authorized for this reviewer |
| Hindsight recall / mental models | NOT_RUN | Existing reported client/server compatibility failure; not retried to re-confirm it |

All shell checks used cwd=/Users/explosovebit/Work/ForgePlan-Constellation-P0 and /bin/bash --noprofile --norc. No special environment values were read or printed.

## Ground-truth verification

- Base..head: d3d938da0ef53a82064937c98d4302cf97318deb..d3d938da0ef53a82064937c98d4302cf97318deb, with an explicitly uncommitted owned-path patch. No worker claim of a new commit was made; a commit-range-only empty diff would omit the real worktree delta and untracked new documents.
- Author status observed: modified AGENTS.md; untracked docs/agents/issue-tracker.md and docs/agents/orchestra.json, with no other status entries.
- Diff state: DELTA=PRESENT. The tracked file differs from the frozen Git object; the frozen patch additionally carries both new documents.
- Expected delta tokens, from approved plan: forgeplan index, ORCHESTRA_FORGE_AGENT_TOKEN, chatWriting and the sole integrator / enabled commit-hook rule. FOUND in the owned files.
- Static reconstruction applies each frozen hunk to the Git base in memory, checks every old context/deletion line, then compares the complete result byte-for-byte with each author file. All three matched, so the review is of the frozen patch, not just the writer's report.
- Ground-truth gate: non-empty claimed patch and expected tokens established; no vacuous-green/empty-diff blocker. Review finding #1 still prevents an unqualified acceptance recommendation.

Exact shell probe (exit 0):

```bash
/bin/bash --noprofile --norc -c 'set +u; git -C /Users/explosovebit/Work/ForgePlan-ready-governance diff --no-ext-diff --no-textconv --stat d3d938da0ef53a82064937c98d4302cf97318deb -- AGENTS.md && git apply --numstat /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch && git -C /Users/explosovebit/Work/ForgePlan-ready-governance hash-object --no-filters AGENTS.md docs/agents/issue-tracker.md docs/agents/orchestra.json && shasum -a 256 /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch; if git -C /Users/explosovebit/Work/ForgePlan-ready-governance diff --quiet d3d938da0ef53a82064937c98d4302cf97318deb -- AGENTS.md; then printf "TRACKED_DELTA=EMPTY\n"; else printf "TRACKED_DELTA=PRESENT\n"; fi'
```

Literal stdout, including tab-separated numstat rows:

```text
 AGENTS.md | 74 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++------
 1 file changed, 68 insertions(+), 6 deletions(-)
68	6	AGENTS.md
131	0	docs/agents/issue-tracker.md
19	0	docs/agents/orchestra.json
c4ad717f356e3f85b3ada8544833f301ff5bea9f
50e611840208695266c5823a6d9e22df7e68d2cb
c5aef2bf57cff20bf831e3df1f5eb1a385fa2594
03193b688130160bc628401df9e1b6ddf32c1ba03e9c0a843307092d14759efe  /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch
TRACKED_DELTA=PRESENT
```

Token probe used the specialized Grep tool, not shell grep: pattern=ORCHESTRA_FORGE_AGENT_TOKEN|"chatWriting"|forgeplan index|единственный интегратор|commit hooks, path=<author>/AGENTS.md;<author>/docs/agents/issue-tracker.md;<author>/docs/agents/orchestra.json, case=true, gitignore=false. Literal matched lines below have only the tool's file headers normalized into file:line prefixes; line text is unchanged:

```text
AGENTS.md:44:   `forgeplan index`, для старого формата — соответствующий `forgeplan reindex`.
AGENTS.md:256:   не интеграционную копию. Координатор — единственный интегратор.
AGENTS.md:289:привязанный LSP. Стандартные commit hooks остаются включены; координатор
docs/agents/issue-tracker.md:20:| Token | Только имя переменной `ORCHESTRA_FORGE_AGENT_TOKEN`; значение в репозиторий не попадает |
docs/agents/orchestra.json:8:      "tokenEnv": "ORCHESTRA_FORGE_AGENT_TOKEN",
docs/agents/orchestra.json:18:  "chatWriting": true
```

## Findings

| # | Severity | Category | Location | Description and real consequence | Recommended fix |
|---|---|---|---|---|---|
| 1 | MEDIUM | 📚 Docs | AGENTS.md:228-229; AGENTS.md:289-291 | The first rule permits Cargo execution only by the appointed tester, but the same workflow retains enabled commit hooks and assigns integration/checkpoint commits to the coordinator. The effective shared pre-commit hook, /Users/explosovebit/Work/ForgePlan/.git/hooks/pre-commit:7, unconditionally invokes cargo check during the committing process; the pinned tester is expressly forbidden to commit. Scheduling that hook with the tester prevents concurrency but does not resolve who is allowed to execute it. Consequently the coordinator cannot perform the required governance checkpoint literally under both rules, even when the machine is otherwise idle; disabling hooks is already correctly forbidden. | Explicitly allow only the coordinator's mandatory hook-spawned Cargo under the same exclusive machine-wide build lease and integration target after writers/other Cargo are quiescent, while retaining enabled hooks, no author commits, and no independent coordinator rebuilds. |

This is a role/executor contract conflict, not a request to remove or rewrite the hook. The pre-existing hook's implementation and its own output parsing were not counted as patch findings.

## Acceptance observations

| Requirement | Independently observed result / location |
|---|---|
| Frozen identity and owned paths | Supplied SHA-256/size match; exact three paths; reconstructed files match the patch and base |
| Exact schema-v2 pins | docs/agents/orchestra.json:1-19 equals the expected object: only servers.default, URL http://127.0.0.1:28174/mcp, endpointVariant agent, auth bearer, tokenEnv ORCHESTRA_FORGE_AGENT_TOKEN, names.omp orchestra-beta-forge-agent, spaceUid kgYnhWodYw7kNbJMAK5q, spaceName ForgePlan, userUid 7eea6f7ad3fc4b3196385e9d3e3cb856, identity bot, top-level chatWriting true; no additional keys or token value |
| Restricted messaging / no extra mutations | AGENTS.md:280-284 and issue-tracker.md:118-126 limit reports to task-runner and the corresponding ROOT-tree task chats; deny other chats, people mentions and parent broadcasts. issue-tracker.md:109-110 denies assignee/owner/due/member changes and destructive history/checklist changes; task-only endpoint is pinned and admin write fallback denied at :27-29 |
| Self-contained tracker contract | All 11 local links resolve, including tracker-to-AGENTS section anchor; skill directory is runtime-resolved rather than taken from a sibling checkout. Verifier source accepts --config as documented at issue-tracker.md:40 |
| No stale TODO fallback | AGENTS.md:237-252 and issue-tracker.md:7-8,56-65 explicitly override generic degraded-mode advice and prohibit TODO fallback/source dispatch on tracker or identity/access failure. Generic fallback remains in the unchanged Unified Workflow but is expressly subordinate for this program |
| Dynamic fields and honest tracking | issue-tracker.md:69-72,98-114 requires list_fields per run, actual model, failed/failedFields/warning checks, readback, NOT_RUN before matching tester proof and Done only after the guardian gate; docs/read-only Rust checks can be NOT_APPLICABLE, not PASSED |
| Roles and handoff | AGENTS.md:225-229 replaces unavailable subagent_explore/subagent_general names with role names; :253-278 specifies isolated ownership, no writer checks/commits, UID/base/owned paths/patch hash/behavioral checks/NOT_RUN/open-findings handoff, fingerprint invalidation and separate runtime/gate. Pinned writer already owns production plus behavioral tests; tester is execution-only. Installed profiles and the active scout/coder/task-runner/reviewer roster support those role identities; future dispatch/model resolution was not executed in this review |
| Index/reindex/scan-import claims | AGENTS.md:42-46 agrees with actual integration help. Static source confirms index invokes maintain_index at crates/forgeplan-cli/src/commands/index.rs:156-158, reindex selects Constellation maintenance instead of legacy init at reindex.rs:68-92, and scan-import calls the external-document importer at scan_import.rs:17-33. No index rebuild/import was executed |
| Rust / Serena / disk preserved | Entire protected block is byte-identical: base AGENTS.md:44-211 = author AGENTS.md:48-215, 13118 bytes, SHA-256 8f8007efb6ee95955e9c6223d1b0479c0fc843f2ccd8123cce229977f0e17d29. No wrong-root Serena call, index warmup, cleanup or Cargo execution |
| Hooks/checkpoint | Hooks remain enabled; author commits and git add -A are forbidden. The approved plan still requires coordinator acceptance and a checkpoint SHA before source-writer dispatch. Finding #1 is the remaining ambiguity/conflict in the executor wording for that checkpoint |

## Positive observations

- The pin file grants no new recipient scope by itself: both AGENTS.md:280-284 and issue-tracker.md:118-126 constrain chatWriting=true to the approved task-local reports, and issue-tracker.md:56-65 fails closed on identity/access failure without extracting credentials.
- The previous Rust/Serena/disk safety block is preserved byte-for-byte, rather than replaced by a second workflow; the handoff now carries the patch hash and explicitly separates NOT_RUN from successful verification (AGENTS.md:259-274).
- Command recovery guidance now distinguishes Constellation maintenance, legacy rebuild and external-document import instead of recommending scan-import for index repair (AGENTS.md:42-46).

## Source and binary fingerprints

| Input | Bytes | Lines | SHA-256 | Git blob |
|---|---:|---:|---|---|
| AGENTS.md | 32888 | 471 | e00c690b158684be516dc6579e94784287145e1c5cf5f8f844e978edc136733c | c4ad717f356e3f85b3ada8544833f301ff5bea9f |
| docs/agents/issue-tracker.md | 11038 | 131 | e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6 | 50e611840208695266c5823a6d9e22df7e68d2cb |
| docs/agents/orchestra.json | 456 | 19 | e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66 | c5aef2bf57cff20bf831e3df1f5eb1a385fa2594 |

The canonical three-line SHA-256 manifest is 264 bytes and hashes to b827b3916c256d460ab3fc6460d272b32e13313779a782dd0abe5396bb7469cb:

```text
e00c690b158684be516dc6579e94784287145e1c5cf5f8f844e978edc136733c  AGENTS.md
e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6  docs/agents/issue-tracker.md
e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66  docs/agents/orchestra.json
```

The inherited help-only binary is /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan, version 0.39.0+constellation.s1.1, 461012344 bytes, SHA-256 96cd31a566e92981e99f558462ad1ee7da17bed00daf9db8db3e71dca9b688ea. It was not rebuilt and is not evidence of any S1 corrective implementation. The effective pre-commit hook was read only: 264 bytes, SHA-256 6a0cc7315bdd858015f6556d435df82ded16ddd03de1ae6bbccec56f12da4958.

## Command and observation records

Help-only shell command (exit 0):

```bash
/bin/bash --noprofile --norc -c '/Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan --version && /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan index --help && /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan reindex --help && /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan scan-import --help && shasum -a 256 /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan'
```

Literal stdout:

```text
forgeplan 0.39.0+constellation.s1.1
Run one Constellation index maintenance pass (capture → reconcile → publish, verified), optionally draining the embedding queue

Usage: forgeplan index [OPTIONS]

Options:
      --embed-profile <EMBED_PROFILE>  Also drain the embedding queue for this profile (requires the semantic-search build feature)
      --deadline-secs <DEADLINE_SECS>  Deadline for the whole pass, in seconds [default: 300]
      --list                           List a read-only page of documents in the published generation
      --offset <OFFSET>                First document to show with --list [default: 0]
      --generations                    List all generations under lance/ with size and protection state (current / leased / keepable) — the generation index
      --gc <KEEP>                      Garbage-collect generations: keep the newest N deletable ones plus CURRENT and anything a live reader holds pinned
  -h, --help                           Print help
  -V, --version                        Print version
Rebuild LanceDB index from .md files (files-first sync)

Usage: forgeplan reindex

Options:
  -h, --help     Print help
  -V, --version  Print version
Scan for existing docs and import as artifacts

Usage: forgeplan scan-import [OPTIONS]

Options:
      --path <PATH>  Directory to scan (default: standard doc dirs)
      --dry-run      Preview only, don't actually import
  -h, --help         Print help
  -V, --version      Print version
96cd31a566e92981e99f558462ad1ee7da17bed00daf9db8db3e71dca9b688ea  /Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan
```

Static proof output (in-memory inspection, no shell script or log file created):

```text
Frozen patch: bytes=20749 sha256=03193b688130160bc628401df9e1b6ddf32c1ba03e9c0a843307092d14759efe owned_paths=3
AGENTS.md: FROZEN_PATCH_MATCH bytes=32888 lines=471 sha256=e00c690b158684be516dc6579e94784287145e1c5cf5f8f844e978edc136733c git_blob=c4ad717f356e3f85b3ada8544833f301ff5bea9f
Protected disk/Rust/Serena block: BYTE_IDENTICAL base_lines=44-211 author_lines=48-215 bytes=13118 sha256=8f8007efb6ee95955e9c6223d1b0479c0fc843f2ccd8123cce229977f0e17d29
docs/agents/issue-tracker.md: FROZEN_PATCH_MATCH bytes=11038 lines=131 sha256=e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6 git_blob=50e611840208695266c5823a6d9e22df7e68d2cb
docs/agents/orchestra.json: FROZEN_PATCH_MATCH bytes=456 lines=19 sha256=e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66 git_blob=c5aef2bf57cff20bf831e3df1f5eb1a385fa2594
Schema-v2 exact expected object: MATCH; only servers.default; tokenEnv name only; chatWriting=true.
Invisible/bidi control scan: NONE
Local Markdown links: 11 valid, 0 broken
AGENTS.md:240: docs/agents/issue-tracker.md -> VALID
AGENTS.md:241: docs/agents/orchestra.json -> VALID
AGENTS.md:242: docs/methodology/UNIFIED-WORKFLOW.ru.md -> VALID
AGENTS.md:467: CLAUDE.md -> VALID
AGENTS.md:468: docs/README.md -> VALID
AGENTS.md:469: docs/methodology/FORGEPLAN-GUIDE.md -> VALID
AGENTS.md:470: docs/operations/AGENT-ENFORCEMENT.md -> VALID
AGENTS.md:471: website/README.md -> VALID
docs/agents/issue-tracker.md:5: ../methodology/UNIFIED-WORKFLOW.ru.md -> VALID
docs/agents/issue-tracker.md:6: ../../AGENTS.md#working-mode-smith-first-then-role-matched-subagents -> VALID (anchor matched)
docs/agents/issue-tracker.md:19: orchestra.json -> VALID
```

Observation payload checksums below are for the UTF-8 text embedded here (with LF endings), not a claim that separate log files were created. The grep payload has normalized file headers as disclosed above. Tool transport decorations/timing are not part of these payloads.

| Payload | Bytes | SHA-256 |
|---|---:|---|
| Ground-truth shell command | 739 | 731efc8f591b1d788dee5015e81fc99ea383b799b78a5a877e4be2808a549866 |
| Ground-truth stdout | 494 | e17d23b6154164992b270c32989060aa0436b54e769983a046af1e9456c193c5 |
| Token matched-line payload | 712 | 998616c8af8ee3bd4f304b5705d237afcbb1b392a3945304f806575dcc31ba24 |
| Help shell command | 491 | aa506f867c15b9a0d5458664b7cb8fd8d27f95fed2e13deb0f38eff94fbc9b77 |
| Help stdout | 1624 | a14bcc1f768de30f08d240f96073da415e61ce4c2892319942c1eab431f176ec |
| Static inspection stdout | 1680 | 8f1b4be24ee1a19df9aad8e29fe22ab65ef21dbd4b9996c3347cc0da02b8f60c |
| Combined observation bundle, ground stdout + LF + token payload + LF + help stdout + LF + static payload | 4513 | 1a3cd9ba3746ca79e8d28ecca62a6580ebdc88c320c5f136978c385c78838ae0 |

## Test coverage delta

- Before / after: n/a; governance-only patch, no coverage runner invoked and no test/code delta.
- Branches gained / still uncovered: n/a for this review.
- No L01-L16 or F01-F11 acceptance row is closed by this evidence. All S1-S4 source/runtime/model/fault/performance/portability obligations remain outside this review and are not converted from negative or NOT_RUN evidence to PASS.

## Next steps

Return finding #1 to the same governance writer, obtain a newly frozen owned-path patch and independently review that wording delta. Keep commit hooks enabled and coordinate their real Cargo execution with the sole build-owner; this review neither executes nor waives those checks. The coordinator integrates only after weighing the verdict, then records the approved governance checkpoint; future source writers start from that exact SHA. No activation or source-stage readiness is authorized by this document.

## References

- RFC-015: program context, read through Forgeplan MCP in the explicit integration workspace; informs linkage is governance-prerequisite only.
- Approved corrective plan step 1 and the frozen patch named above.
- EVID-181 was not read or used in this review; no based_on provenance edge is claimed. Existing negative evidence and prior Rust counts are unchanged.
- Evidence created and updated only through Forgeplan MCP in the explicit integration workspace, left draft.




