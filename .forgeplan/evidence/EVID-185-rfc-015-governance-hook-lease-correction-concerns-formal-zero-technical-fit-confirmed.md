---
depth: standard
id: EVID-185
kind: evidence
last_modified_at: 2026-10-20T19:18:20.224164+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
- target: EVID-184
  relation: based_on
status: draft
title: 'RFC-015 governance hook-lease correction: CONCERNS (formal zero; technical fit confirmed)'
---

---
assigned_number: 185
created: 2026-10-20
predicted_number: 185
slug: evid-rfc-015-governance-hook-lease-correction-concerns-formal-zero-technical
updated: 2026-10-20
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: code_review

## Verdict

CONCERNS

Formal zero-findings convention only. Technical fit: CONFIRMED for the exact corrected governance fingerprint below; EVID-184 finding #1 is resolved in this replacement patch. No actionable material finding remains, and no further coder dispatch is requested solely to turn an honest zero into PASS. This record neither changes EVID-184 nor declares Rust/S1-S4 readiness or authorizes artifact activation.

## Scope

- Parent: RFC-015, governance prerequisite only; approved corrective plan step 1.
- Task: Z8cucrnH8xIBowclNr5o; reviewer GovernanceReview.
- Audit identity: claude-code/6.1-sol/code-reviewer-task-Z8cucrnH8xIBowclNr5o; actual runtime/model OMP / openai-codex/gpt-6.1-sol.
- Explicit Forgeplan workspace for every operation: /Users/explosovebit/Work/ForgePlan-Constellation-P0.
- Author tree: /Users/explosovebit/Work/ForgePlan-ready-governance.
- Frozen base: d3d938da0ef53a82064937c98d4302cf97318deb.
- Prior full review: EVID-184, patch 20749 bytes / SHA-256 03193b688130160bc628401df9e1b6ddf32c1ba03e9c0a843307092d14759efe.
- Replacement patch: /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch, 21564 bytes / SHA-256 1ce496cb7d8c6fc2d229d331089152871feee43889e68ccb7648dd4e67ca6843.
- Semantic re-review was bounded to two AGENTS.md correction blocks and immediate instructions: prior lines 225-229 and 286-291 become revised lines 225-231 and 288-299 (19 lines in the two replacement blocks, net +8). The actual textual change is +13/-5 relative to the previous author file.
- Three-file final fingerprint covers 629 lines (AGENTS.md 479, tracker 131, config 19). Unchanged content relies on the prior review and byte identity, not a repeated whole-tree audit.

## Tools run

| Tool / inspection | Observed outcome | Limit |
|---|---|---|
| Clean-shell shasum/wc/git apply --numstat followed by diff -u of old/new frozen patch files | Hash/size/numstat steps succeeded; command exit 1 from diff means differences were found | No patch application or Git/source mutation |
| In-memory reconstruction of the replacement patch from the frozen Git base | All three reconstructed files exactly match author bytes | No materialized file or index write |
| Old/new patch-block and SHA-256 comparisons | Tracker and config identical; AGENTS changes only within the two assigned blocks | Reuses prior review for unchanged material |
| Read/Grep of corrected clauses and adjacent instructions | Required narrow exception and continuing prohibitions FOUND | Static documentary audit only |
| Cargo/build/test/lint/typecheck/formatters/hooks | NOT_RUN | No Rust validation or commit hook execution |
| CLI help, tracker handshake/verifier, role dispatch, model loading | NOT_RUN in this follow-up | Prior evidence is not rebranded as a new run |
| Source edits / tracker changes / commits / activation | NOT_RUN | None performed |

The shell probe ran from the explicit integration directory using /bin/bash --noprofile --norc. Mounted Serena was not used. No token, private environment value or auth store was read.

## Ground-truth verification

The review input is explicitly an uncommitted frozen patch against the Git base, not a claimed new commit. DELTA=PRESENT is established by the literal diff below; exit 1 is the normal diff result, not a failed build. Expected delta tokens come from EVID-184 finding #1: a sole tester-owned lease plus a mandatory pre-commit-only, serialized coordinator exception and explicit no-rebuild/no-hook-disable limits. Those tokens are FOUND in the corrected clauses. The non-empty change is verified directly rather than accepted from the writer's summary.

Exact shell command:

```bash
/bin/bash --noprofile --norc -c 'shasum -a 256 /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch /Users/explosovebit/Work/ForgePlan-ready-governance/AGENTS.md /Users/explosovebit/Work/ForgePlan-ready-governance/docs/agents/issue-tracker.md /Users/explosovebit/Work/ForgePlan-ready-governance/docs/agents/orchestra.json && wc -c /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch && git apply --numstat /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch && diff -u /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch'
```

Literal stdout:

```text
1ce496cb7d8c6fc2d229d331089152871feee43889e68ccb7648dd4e67ca6843  /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch
6192480f5287ec697dbaaec84773deae560833c630d0e18b2fe93f67a0d63ef6  /Users/explosovebit/Work/ForgePlan-ready-governance/AGENTS.md
e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6  /Users/explosovebit/Work/ForgePlan-ready-governance/docs/agents/issue-tracker.md
e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66  /Users/explosovebit/Work/ForgePlan-ready-governance/docs/agents/orchestra.json
   21564 /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch
76	6	AGENTS.md
131	0	docs/agents/issue-tracker.md
19	0	docs/agents/orchestra.json
--- /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o.patch	2026-10-20 22:02:47
+++ /private/tmp/constellation-ready-governance-Z8cucrnH8xIBowclNr5o-hook-lease.patch	2026-10-20 22:14:17
@@ -1,5 +1,5 @@
 diff --git a/AGENTS.md b/AGENTS.md
-index ec0a523f..c4ad717f 100644
+index ec0a523f..f335b5a2 100644
 --- a/AGENTS.md
 +++ b/AGENTS.md
 @@ -39,7 +39,11 @@ This file is the **entry point**. For full details, read the files it points to.
@@ -15,7 +15,7 @@
  
  ## Parallel agents and the build directory
  
-@@ -218,16 +222,74 @@ steps, in order:
+@@ -218,16 +222,82 @@ steps, in order:
     classifies the context against the routing matrix, and returns a Plan that
     names which specialist subagents to dispatch and in what order. Smith picks
     the route; it never writes code itself.
@@ -27,8 +27,10 @@
 +2. **Role-matched dispatch** — `scout` для чтения и исследования, `coder` для
 +   реализации, `code-reviewer` для независимого ревью, `tester` для проверок,
 +   `guardian` для предметного gate, `task-runner` для Orchestra. Независимые срезы
-+   могут идти параллельно в отдельных worktree; **только один назначенный tester
-+   выполняет Cargo** на машине. Ограничения Rust/Serena/диска выше сохраняются.
++   могут идти параллельно в отдельных worktree; **единственной build lease
++   на машине владеет назначенный tester**. Cargo выполняет tester; исключение
++   только для обязательного commit hook координатора описано ниже.
++   Ограничения Rust/Serena/диска выше сохраняются.
  
  Exempt: single-line tactical fixes, conversational answers, and **stateful
  runs** (live processes, DB observation, interactive sessions) — a subagent has
@@ -88,9 +90,15 @@
 +Запрещены сборки в нескольких `target`/worktree, изменение source во время
 +build lease, коммиты с отключёнными hooks, ослабление/подмена oracle ради
 +зелёного результата и чтение одноимённого файла другого checkout через неверно
-+привязанный LSP. Стандартные commit hooks остаются включены; координатор
-+согласует их Cargo-проверки с единственным tester. Не собирать чужие изменения
-+через `git add -A`; до принятого checkpoint авторы передают только owned patch.
++привязанный LSP. Стандартные commit hooks остаются включены.
++Единственное исключение к tester-only Cargo: после завершённой проверки tester
++координатор может вызвать Cargo только через обязательный pre-commit hook
++своего checkpoint-коммита, последовательно под той же build lease tester,
++в той же интеграционной копии и с тем же `target`. Владельцем lease остаётся
++tester; параллельные Cargo-запуски и самостоятельные сборки/пересборки
++координатора запрещены. Нельзя отключать hooks ради обхода их Cargo-проверки.
++Не собирать чужие изменения через `git add -A`; до принятого checkpoint авторы
++передают только owned patch.
 +
  ## Searching the tree: tgrep, not rg
  
```

Specialized Grep probe: pattern=единственной build lease|обязательного commit hook|обязательный pre-commit hook|последовательно под той же build lease|Владельцем lease остаётся|самостоятельные сборки/пересборки|Нельзя отключать hooks; path=<author>/AGENTS.md:225-231,288-299; case=true; gitignore=false. Matched lines below only normalize the tool's file header to file:line prefixes:

```text
AGENTS.md:228:   могут идти параллельно в отдельных worktree; **единственной build lease
AGENTS.md:230:   только для обязательного commit hook координатора описано ниже.
AGENTS.md:293:координатор может вызвать Cargo только через обязательный pre-commit hook
AGENTS.md:294:своего checkpoint-коммита, последовательно под той же build lease tester,
AGENTS.md:295:в той же интеграционной копии и с тем же `target`. Владельцем lease остаётся
AGENTS.md:296:tester; параллельные Cargo-запуски и самостоятельные сборки/пересборки
AGENTS.md:297:координатора запрещены. Нельзя отключать hooks ради обхода их Cargo-проверки.
```

## Findings

No actionable material findings in the correction.

AGENTS.md:228-231 now keeps the appointed tester as the machine-wide lease owner while explicitly pointing to a limited coordinator exception, so it no longer asserts the previous unconditional executor restriction. AGENTS.md:292-297 confines that exception to the mandatory pre-commit hook of the coordinator's checkpoint commit, after tester verification, serialized under the same tester-owned lease, integration worktree and target; it expressly forbids concurrent Cargo, independent coordinator builds/rebuilds and disabled hooks. The immediately adjacent author prohibition at AGENTS.md:255-260 and ownership/gate sequence at :268-276 remain intact, so neither authors nor another verifier gain new mutation or build authority.

These checks resolve EVID-184 finding #1 for the replacement fingerprint only. The actual shared pre-commit invokes cargo check, as inspected in EVID-184; executing that hook and proving its result remain NOT_RUN here. There is no new code defect or requested code fix hidden behind the formal CONCERNS verdict.

## Positive observations

- The repair distinguishes lease ownership from the narrow hook executor exception instead of weakening the one-build-owner rule (AGENTS.md:228-231,292-297).
- The same integration worktree/target, serialization, no-independent-rebuild and no-disabled-hook conditions are explicit in one place; author commits and oracle/LSP restrictions remain intact (AGENTS.md:255-260,288-299).

## Unchanged acceptance and evidence boundaries

Tracker/config patch blocks are identical to those in EVID-184, and their SHA-256 values are unchanged. The earlier checks of exact schema-v2 pins/tokenEnv name, restricted chatWriting scope, self-contained tracker links and no degraded-mode source dispatch therefore remain applicable to those same bytes. The protected disk/Rust/Serena block also remains byte-identical to the prior reviewed file and base (13118 bytes, SHA-256 8f8007efb6ee95955e9c6223d1b0479c0fc843f2ccd8123cce229977f0e17d29). No second endpoint or role authority was added.

This evidence closes no L01-L16 or F01-F11 behavior requirement. It is not a fresh CLI/MCP runtime run, Rust PASS, source-stage gate, model measurement, commit result, or authorization to install/push/activate anything. Required hook/build checks still belong to their named executor and recorded lease in the approved workflow.

## Final source fingerprint

| Path | Bytes | Lines | SHA-256 | Git blob |
|---|---:|---:|---|---|
| AGENTS.md | 33695 | 479 | 6192480f5287ec697dbaaec84773deae560833c630d0e18b2fe93f67a0d63ef6 | f335b5a26ede2fc706edb63521e62ec4897300e5 |
| docs/agents/issue-tracker.md | 11038 | 131 | e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6 | 50e611840208695266c5823a6d9e22df7e68d2cb |
| docs/agents/orchestra.json | 456 | 19 | e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66 | c5aef2bf57cff20bf831e3df1f5eb1a385fa2594 |

Canonical owned-path manifest (264 bytes), SHA-256 e0dbdf888ab8e2562e2667d02cbe0ade12697fedccd9d4e6b63de88b7e0e6ef5:

```text
6192480f5287ec697dbaaec84773deae560833c630d0e18b2fe93f67a0d63ef6  AGENTS.md
e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6  docs/agents/issue-tracker.md
e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66  docs/agents/orchestra.json
```

Static observation payload (in-memory inspection; no log/script file was written):

```text
Revision patch: 21564 bytes; SHA-256 1ce496cb7d8c6fc2d229d331089152871feee43889e68ccb7648dd4e67ca6843
Frozen patch -> author bytes: MATCH for all 3 owned paths
Correction scope: ONLY AGENTS.md prior lines 225-229 and 286-291 -> revised lines 225-231 and 288-299
All AGENTS.md bytes outside those two blocks: UNCHANGED
Tracker and config patch blocks and file SHA-256: UNCHANGED
Protected Rust/Serena/disk block: inherited BYTE_IDENTICAL proof remains valid
AGENTS.md: 33695 bytes, 479 lines, SHA-256 6192480f5287ec697dbaaec84773deae560833c630d0e18b2fe93f67a0d63ef6, Git blob f335b5a26ede2fc706edb63521e62ec4897300e5
docs/agents/issue-tracker.md: 11038 bytes, 131 lines, SHA-256 e683ee608c71dc001b6e9933d0645312b6d0bc7f7789554c7ce1a870b8aa90b6, Git blob 50e611840208695266c5823a6d9e22df7e68d2cb
docs/agents/orchestra.json: 456 bytes, 19 lines, SHA-256 e9242a452913eed464c6c95b4f0323dc3529ed2caee9f556438670cb3e419c66, Git blob c5aef2bf57cff20bf831e3df1f5eb1a385fa2594
Owned-file SHA-256 manifest: 264 bytes, SHA-256 e0dbdf888ab8e2562e2667d02cbe0ade12697fedccd9d4e6b63de88b7e0e6ef5
```

## Reproduction record checksums

These hashes cover the UTF-8 payloads embedded above with LF endings, not a claim of separate on-disk logs. The token payload has normalized file headers as disclosed; timing and transport decorations are excluded.

| Payload | Bytes | SHA-256 |
|---|---:|---|
| Shell command | 711 | a9f7fa31aa548fce965a590e36708f4d86ed582dad44b3f165b5d9a0e79a6844 |
| Shell stdout | 4629 | b4393e7d899afd52005bc54240cf90940fe458b79f39d6aaa60f5bc5c2220531 |
| Token matched-line payload | 940 | 47f079be15f50a328e46a018366e9a6b43464d5528969cfde7fa746d281eb8d3 |
| Static fingerprint proof | 1080 | b9aa577b7fdfd470225846465d9a32bbd5a10956bc1fefc2ac6e235669832ddd |

## Test coverage delta

Before/after coverage and branches: n/a; governance wording only and no test execution. The follow-up changes no source, tests, lockfile or acceptance oracle.

## Next steps

No further writer correction is requested for this bounded governance change. The coordinator can weigh technical acceptance of this exact fingerprint and perform the planned integration/checkpoint under the documented tester-owned lease and enabled hook rules; the present review did not perform those actions. The formal zero-findings CONCERNS must not trigger a fabricated fix or an unbounded re-review loop. Do not activate the parent or this evidence on the reviewer's behalf.

## References

- RFC-015: informs, governance prerequisite.
- EVID-184: based_on, genuinely used prior finding and unchanged-scope observations; original body, verdict and status are preserved.
- Approved corrective plan step 1; the old and replacement frozen patches identified above.
- No EVID-181 provenance edge: that record was not used in this follow-up.




