---
depth: standard
id: EVID-191
kind: evidence
last_modified_at: 2026-10-22T23:23:57.601151+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: draft
title: 'RFC-015 confirmation: MCP 19/19; latest 197 passed, 1 known out-of-scope failure'
---

---
assigned_number: 191
created: 2026-10-22
predicted_number: 191
slug: evid-rfc-015-narrow-byte-repair-validation-blocker-10-rust-failures-and-marker
updated: 2026-10-22
---

## Structured Fields

verdict: refutes
congruence_level: 3
evidence_type: test

Structured refutes относится только к широкому утверждению «все сохранённые Rust-цели зелёные»: в коллекции остаётся один предсуществующий failure вне объёма (#540). Для фактически назначенного финального rebuild/confirmation/CLI-MCP исполнения все наблюдения PASS. Это не основание активировать артефакт или объявлять полный RFC принятым.

## Verdict

**CONCERNS для полной коллекции; PASS для текущего назначенного прогона.** Release и debug builds exit=0, в обоих выводах **0 строк warning:** и **0 вхождений warning:**. Повторная MCP compatibility target: **19 passed / 0 failed / 0 ignored**, exit=0, **1** строка test result. Новый release CLI/MCP scenario: **60/60 checks, exit=0**, все 12 пунктов PASS.

Единственный retained failure — git_sync_announces_the_index_migration в cli_uncovered_coverage, предсуществующий и вне объёма, отдельная находка #540; он не перезапускался, не изменялся и не скрыт. Latest-recorded-per-target: **197 passed / 1 failed / 0 ignored**, 11 test binaries; это не заявление, что все эти binaries повторялись после последнего удаления mut.

## Measurement

- Workspace `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, только его target. Единственный владелец сборки, все Cargo строго последовательно/foreground, CARGO_BUILD_JOBS=1. Порядок: release → debug → MCP target → Python release scenario.
- Actual model **openai-codex/gpt-6.1-sol**; runtime **OMP / darwin arm64 / Python 3.9.23 / /bin/bash --noprofile --norc**.
- Audit identity `claude-code/not-applicable-omp/tester-task-ByteRepairBuild`; это шаблон роли, не утверждение о runtime Claude Code.
- Окружение ниже экспортировалось перед каждой shell-командой; offline/locked соблюдены, test-helpers в release не включался.
- df -h перед тяжёлыми шагами: 12 GiB перед release, 11 GiB перед debug, 12 GiB перед test/scenario. Ниже 10 GiB в измеренных предпроверках не опускалось; cleanup в этом прогоне не понадобился.
- Исполнитель не менял source/test/driver, не выполнял git add/commit/stash/checkout. Предполет обнаружил уже staged изменения координатора — они не удалялись и не переиндексировались этим исполнителем.
- Новый каталог `/Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z`; предыдущие результаты не перезаписаны. Свежий driver использовал новый временный project/HOME с тем же fixture и последовательностью; прежний временный project уже удалён context manager.
- Current runner Cargo/stable libtest, output_format=text, test-threads=1. Порог statement/branch coverage не задан.

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
````


## Result — текущие команды

| Шаг | Exit | Wall, s | warning: prefix lines | warning: occurrences | test result lines | Passed/failed/ignored |
|---|---:|---:|---:|---:|---:|---|
| 01-release-build | 0 | 744.376 | 0 | 0 | 0 | n/a — build |
| 02-debug-build | 0 | 26.142 | 0 | 0 | 0 | n/a — build |
| 03-mcp-local-compat | 0 | 223.781 | 0 | 0 | 1 | 19/0/0 |


Release wall=744.376s, debug wall=26.142s, MCP target wall=223.781s (libtest finished in196.36s). Exact commands, stdout и exit ниже. Cargo test result count=1, не RUN DID NOT HAPPEN. Для build и Python scenario метка test result неприменима, ноль не выдаётся за выполненные Rust tests.

Проверка warning основана на реальном captured output: count строк с prefix warning: и число всех подстрок warning: равно0 в release, debug и тестовой команде. RUSTFLAGS=-Dwarnings отдельно не запускался; факт отсутствия предупреждения не подменён заявлением о полном CI.

## Release binaries

| Binary | Version | Bytes | SHA-256 |
|---|---|---:|---|
| forgeplan | 0.39.0+constellation.s1.1 | 61865808 | `95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332` |
| forgeplan-mcp | 0.39.0+constellation.s1.1 | 58015792 | `e3154f6642bab4630594092d8a835d332cb77945538af81e1f47c6584a991c89` |


CLI version прочитана --version (exit0), standalone MCP version — initialize.serverInfo.version в свежих трёх MCP сессиях; неподдерживаемый --version у MCP не запускался. Release команда реально перекомпилировала forgeplan-mcp и forgeplan (см. Compiling и Finished в логе), а не была пропущена.

SHA-256 обоих binaries **остались такими же**, как до удаления mut и как до/после scenario. Это наблюдаемый результат оптимизированной пересборки; новый source fingerprint ниже отличается. Не требуется выдумывать изменение machine-code hash для удаления только неиспользуемой mutable-привязки.

Literal binary command output:

````text
95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332  target/release/forgeplan
e3154f6642bab4630594092d8a835d332cb77945538af81e1f47c6584a991c89  target/release/forgeplan-mcp
target/release/forgeplan 61865808 bytes
target/release/forgeplan-mcp 58015792 bytes
forgeplan 0.39.0+constellation.s1.1
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s1   926Gi   873Gi    12Gi    99%     12M  125M    9%   /System/Volumes/Data
````

MCP initialize.serverInfo:

````json
[
  {
    "label": "cli-mcp",
    "serverInfo": {
      "name": "forgeplan",
      "version": "0.39.0+constellation.s1.1"
    }
  },
  {
    "label": "standalone-mcp",
    "serverInfo": {
      "name": "forgeplan",
      "version": "0.39.0+constellation.s1.1"
    }
  },
  {
    "label": "standalone-reopened",
    "serverInfo": {
      "name": "forgeplan",
      "version": "0.39.0+constellation.s1.1"
    }
  }
]
````


## CLI/MCP — 12 пунктов на текущих релизных binaries

| Пункт | Проверка | Итог | Фактический вывод / значения |
|---:|---|---|---|
| 1 | init -y | PASS | exit=0; отдельный временный project и HOME, не рабочий repository. |
| 2 | Ровно один PRD-001, точный fixture | PASS | PRD-001.md — 246 source bytes / 142 parsed-body bytes, SHA=e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef; frontmatter/BODY записаны без Probe.source. |
| 3 | index и index --list | PASS | оба exit=0; entries=1; prd-offset-smoke unavailable prds/PRD-001.md; generation gen-2026-10-22T23-20-39.149Z. |
| 4 | embed offline | PASS | exit=0; 6 embedded / 0 stale / 0 could not be judged / 0 failed; cache revision5617a9f61b028005a4858fdac845db406aefb181, все шесть файлов непусты. |
| 5 | CURRENT и manifest | PASS | generation=gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c; manifest SHA=cc899e49bafc4a357efb672a9c80697eb378225d84ef9ff5ef24d8c5b473d108 совпал с descriptor; source_entries SHA=e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef. |
| 6 | CLI keyword | PASS | exit=0; tailpolicysentinel → results[].id=[PRD-001]. |
| 7 | CLI semantic | PASS | exit=0; index=constellation; semantic_available=true; eligible_rows=expected_rows=6>1; semantic_scope=prefix+sections; PRD-001 top score=0.7283645272254944. |
| 8 | CLI get exact bytes | PASS | exit=0; id=PRD-001; 246 exact source bytes, persisted=true; SHA=e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef. |
| 9 | CLI serve MCP | PASS | initialize version0.39.0+constellation.s1.1; tools/list=76 и три нужных имени; get exact246/search PRD-001/pinned found=true, key=prd-offset-smoke, body exact142, source_digest совпал; EOF exit0. |
| 10 | Standalone MCP | PASS | initialize version0.39.0+constellation.s1.1; tools/list=76; get exact246/search PRD-001/pinned exact142 и digest; EOF exit0. |
| 11 | Marker edit, read-only freshness | PASS | tailpolicysentinel → credentialrotationrev2, только marker; get exact250, persisted=false; новый marker=[PRD-001], старый=[]; CURRENT SHA до/после=6affe26112371f0727a2d0ea4d3cdab5d3828214d30a4799d5e462a464cdb6a4, bytewise unchanged. |
| 12 | Reindex и свежий standalone | PASS | reindex exit0; generation=gen-2026-10-22T23-21-05.052Z; manifest SHA=6c3e16d863444b5fcf86e1673a1c2a6e2b9f7e61cdbfab4733f09e5f147bf8ff; source_entries SHA=92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4; get exact250 persisted=true; fresh MCP get/search/pinned body146; EOF0. |


Scenario command exit=0, wall=30.490s; **60 passed / 0 failed / 0 skipped checks**, плюс 6 вспомогательных initialize/normal_exit — отдельно от60, все PASS. Все 3 host processes exit=0 после EOF, emergency_kill=false. JSON-RPC transcript и source bytes: runtime/events.jsonl, runtime/scenario-summary.json, source-original.md и source-edited.md. До embed шесть cache files проверены по фактическим target sizes, включая tokenizer.json=17098108 bytes и onnx/model.onnx_data=2266820608 bytes.

## Source fingerprint и точная последняя дельта

Список: git ls-files -z --cached --others --exclude-standard -- crates/ eval/, sorted unique; **441 файл**. До/после всех текущих executions source fingerprint одинаков.

- Алгоритм прежнего checkpoint: SHA256(concat(sorted relative_path UTF8 || NUL || raw SHA256(file_bytes))) → **24d36cac957303a3cc37cb1fee8f2ed2ff5bb1797ed2e58a9df271a658df28b6**.
- Формула текущего сообщения с отдельным хешем записи: SHA256(concat(raw SHA256(relative_path UTF8 || NUL || raw SHA256(file_bytes)) for each sorted path)) → **028c0881bc18f733c7a4cbd9b4bf574345c0f2c2ef25a0c0b5e1b241b2634988**. Оба приведены явно, поскольку эти формулы различаются и не взаимозаменяемы.
- Прежний flat digest, заданный для сравнения: **6f241041e06b543f1d48f21d43cda3f888c1f29f44c41f45c65bb705e12997a7**. Новый flat digest отличается.
- При сравнении всех441 per-file hashes изменён только crates/forgeplan-mcp/src/server.rs. Предыдущий hash файла=23f4e85f922c4bc7b24f3356909987e49175eb7c9160c95b515209c4476fa15b, текущий=04dae5dff34805ae00d35445a0e104e50053684cc5c359237c6d093077cdd4aa.
- Независимая побайтовая проверка последней дельты: в текущих bytes найден ровно один полный line `        let hits: Vec<FpfSearchHit> = results`; вставка `mut ` обратно **только в памяти**, без записи файла, даёт ровно предыдущий SHA файла. Поэтому «отличается только этой строкой» подтверждено digest, а не только сообщением автора. Размер файла уменьшен на4 bytes.
- Детальные inventories и proof: source-before.json, source-after.json,00-one-word-change.json,06-stability.json.

## Ground-truth verification

Base=HEAD=55d16282786db5a8111b8bc603fdb5062e2f663e. Новый commit не заявляется, но staged delta координатора **PRESENT**, working-tree source delta также PRESENT. Реальный git object-store/staged probe прочитан независимо, не пересказан с worker claim. Expected token `let hits: Vec<FpfSearchHit> = results` наблюдён в server.rs:7905; отсутствие mut дополнительно подтверждено0warnings и reconstruct-previous-file SHA. Gate PASS-eligible для существующей staged/source delta, но полный parent PASS не выдаётся из-за границ назначения и сохранённого outside failure.

Exact clean-shell commands and literal stdout:

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
set +u
git rev-parse HEAD
git diff --stat 55d16282786db5a8111b8bc603fdb5062e2f663e..HEAD
git diff --cached --stat
if git diff --quiet 55d16282786db5a8111b8bc603fdb5062e2f663e..HEAD && git diff --cached --quiet; then echo FROZEN_OR_STAGED_DELTA=EMPTY; else echo FROZEN_OR_STAGED_DELTA=PRESENT; fi
git diff --stat 55d16282786db5a8111b8bc603fdb5062e2f663e -- crates/ eval/
git diff --cached -- crates/forgeplan-mcp/src/server.rs
````


````text
55d16282786db5a8111b8bc603fdb5062e2f663e
 ...test-197-passed-1-known-out-of-scope-failure.md | 1180 ++++++++++++++++++++
 CHANGELOG.md                                       |   46 +
 .../tests/cli_constellation_local_compat.rs        |  140 ++-
 crates/forgeplan-core/src/artifact/sections.rs     |  139 ++-
 crates/forgeplan-core/src/constellation/capture.rs |  290 ++---
 crates/forgeplan-core/src/coverage/mod.rs          |   31 +-
 crates/forgeplan-core/src/embed/tokenizer.rs       |  266 ++++-
 .../tests/constellation_section_windows.rs         |  222 +++-
 .../tests/constellation_tokenizer.rs               |   54 +-
 crates/forgeplan-mcp/src/server.rs                 |   95 +-
 .../tests/constellation_local_compat.rs            |  161 ++-
 11 files changed, 2361 insertions(+), 263 deletions(-)
FROZEN_OR_STAGED_DELTA=PRESENT
 .../tests/cli_constellation_local_compat.rs        | 140 +++++++---
 crates/forgeplan-core/src/artifact/sections.rs     | 139 ++++++++--
 crates/forgeplan-core/src/constellation/capture.rs | 290 +++++++++++----------
 crates/forgeplan-core/src/coverage/mod.rs          |  31 ++-
 crates/forgeplan-core/src/embed/tokenizer.rs       | 266 +++++++++++++++++--
 .../tests/constellation_section_windows.rs         | 222 +++++++++++++++-
 .../tests/constellation_tokenizer.rs               |  54 +++-
 crates/forgeplan-mcp/src/server.rs                 |  95 ++++++-
 .../tests/constellation_local_compat.rs            | 161 +++++++++++-
 9 files changed, 1135 insertions(+), 263 deletions(-)
diff --git a/crates/forgeplan-mcp/src/server.rs b/crates/forgeplan-mcp/src/server.rs
index 84670b4f..10b6066c 100644
--- a/crates/forgeplan-mcp/src/server.rs
+++ b/crates/forgeplan-mcp/src/server.rs
@@ -7809,6 +7809,17 @@ impl ForgeplanServer {
         }
 
         let limit = p.limit.unwrap_or(5).min(50);
+        // Overfetch by exactly one row, so "there are more matches" is a fact
+        // this handler observed rather than something it infers from having
+        // reached the limit — the distinction between "3 matches exist" and
+        // "3 of 12 exist" is invisible to a consumer that only sees a capped
+        // list. `search_fpf` discards the total, so the extra row is the only
+        // signal available without changing its signature.
+        //
+        // 51 stays inside `AuxiliaryStore::search_fpf`'s accepted range; the
+        // cap the caller asked for is applied to the answer below, never to
+        // the query.
+        let fetch = limit.saturating_add(1);
         let semantic = p.semantic.unwrap_or(false);
         let mut warning: Option<String> = None;
 
@@ -7823,14 +7834,14 @@ impl ForgeplanServer {
                     // never fetch gigabytes. Absence degrades to keyword below,
                     // with the reason reported.
                     Ok(mut model) => match model.embedder.embed(&p.query) {
-                        Ok(qvec) => match fpf.search_fpf_by_vector(&qvec, limit).await {
+                        Ok(qvec) => match fpf.search_fpf_by_vector(&qvec, fetch).await {
                             Ok(r) => r,
                             Err(e) => {
                                 tracing::warn!("FPF vector search failed: {e}");
                                 warning = Some(format!(
                                     "vector search failed ({e}); fell back to keyword search"
                                 ));
-                                match fpf.search_fpf(&p.query, limit).await {
+                                match fpf.search_fpf(&p.query, fetch).await {
                                     Ok(r) => r,
                                     Err(ke) => {
                                         // The keyword fallback failed too. An
@@ -7848,7 +7859,7 @@ impl ForgeplanServer {
                             warning = Some(format!(
                                 "failed to encode query ({e}); fell back to keyword search"
                             ));
-                            match fpf.search_fpf(&p.query, limit).await {
+                            match fpf.search_fpf(&p.query, fetch).await {
                                 Ok(r) => r,
                                 Err(ke) => return Ok(safe_err_result("FPF search", ke)),
                             }
@@ -7860,7 +7871,7 @@ impl ForgeplanServer {
                             "no usable cached embedding model ({e}); fell back to keyword \
                              search. Obtain weights explicitly with `forgeplan setup --warm-model`."
                         ));
-                        match fpf.search_fpf(&p.query, limit).await {
+                        match fpf.search_fpf(&p.query, fetch).await {
                             Ok(r) => r,
                             Err(ke) => return Ok(safe_err_result("FPF search", ke)),
                         }
@@ -7873,7 +7884,7 @@ impl ForgeplanServer {
                     "semantic-search feature not compiled in; falling back to keyword search"
                         .to_string(),
                 );
-                match fpf.search_fpf(&p.query, limit).await {
+                match fpf.search_fpf(&p.query, fetch).await {
                     Ok(r) => r,
                     Err(e) => return Ok(safe_err_result("FPF search", e)),
                 }
@@ -7881,14 +7892,19 @@ impl ForgeplanServer {
         } else {
             // A keyword search that failed must say so. `unwrap_or_default`
             // here answered "no matches" for "the query could not be run".
-            match fpf.search_fpf(&p.query, limit).await {
+            match fpf.search_fpf(&p.query, fetch).await {
                 Ok(r) => r,
                 Err(e) => return Ok(safe_err_result("FPF search", e)),
             }
         };
 
+        // A row beyond the cap means the answer was bounded. Drop it — `count` is the
+        // number of results actually returned, which is what a consumer counts
+        // — and say so in words below.
+        let truncated = results.len() > limit;
         let hits: Vec<FpfSearchHit> = results
             .iter()
+            .take(limit)
             .map(|c| FpfSearchHit {
                 id: c.id.clone(),
                 section_id: c.section_id.clone(),
@@ -7907,27 +7923,42 @@ impl ForgeplanServer {
             .collect();
 
         let count = hits.len();
+        debug_assert!(count <= limit, "the cap must bound the returned rows");
         let first_section = hits.first().map(|h| sanitize_for_hint(&h.section_id));
         let response = FpfSearchResponse {
             query: p.query.clone(),
             semantic,
             count,
             results: hits,
+            // `warning` is not reused here: by contract it means the semantic
+            // channel did not run and keyword search answered instead. FPF-095
+            // asserts exactly that, and overloading the field would make a
+            // perfectly healthy truncated answer indistinguishable from a
+            // degraded one.
             warning,
         };
+        let truncation_note = if truncated {
+            format!(
+                " More matches exist than the {limit} returned — raise `limit` (max 50) to see \
+                 more of them."
+            )
+        } else {
+            String::new()
+        };
         let next_action = if count == 0 {
             format!(
                 "No FPF matches for `{}`. Try broader terms or `forgeplan_fpf_list` for section \
-                 catalog.",
+                 catalog.{truncation_note}",
                 sanitize_for_hint(&p.query)
             )
         } else if let Some(sid) = first_section {
             format!(
                 "{count} section(s) found. Read full section: `forgeplan_fpf_section {sid}`. \
-                 Top hits show chapter IDs (A.x kernel, B.x reasoning, C.x specifications)."
+                 Top hits show chapter IDs (A.x kernel, B.x reasoning, C.x \
+                 specifications).{truncation_note}"
             )
         } else {
-            format!("{count} hit(s). `forgeplan_fpf_section <id>` for full text.")
+            format!("{count} hit(s). `forgeplan_fpf_section <id>` for full text.{truncation_note}")
         };
         hinted_result(&response, next_action)
     }
@@ -15908,9 +15939,12 @@ mod resolve_workspace_tests {
         let default_ws = mk_fake_workspace();
         let param_ws = mk_fake_workspace();
 
+        // The capability takes the **markdown workspace**, not the project root —
+        // see the note on the sibling test. From the root it would publish into
+        // `<root>/lance` and the migrated precondition below would never hold.
         let capability =
             forgeplan_core::constellation::facade::MaintainCapability::for_strict_maintenance(
-                param_ws.path().canonicalize().unwrap(),
+                param_ws.path().join(".forgeplan").canonicalize().unwrap(),
             );
         forgeplan_core::constellation::facade::maintain_index(
             &capability,
@@ -15928,6 +15962,17 @@ mod resolve_workspace_tests {
              otherwise this test would exercise the legacy branch"
         );
 
+        // The legacy branch is only observable on a workspace that actually HAS a
+        // legacy index: `resolve_read_source` on a non-migrated workspace opens
+        // the store, and a bare `.forgeplan/` with no tables would fail that
+        // open with "Table 'artifacts' was not found" — a fixture error that
+        // looks like the branch under test is broken. `make_server` would build
+        // the store too, but doing it here names the precondition instead of
+        // relying on it.
+        forgeplan_core::db::store::LanceStore::init(&default_ws.path().join(".forgeplan"))
+            .await
+            .expect("give the default workspace a real legacy index");
+
         let server = make_server(default_ws.path()).await;
         // SAFETY: serialised with `#[serial(env_forgeplan_workspace)]`; no
         // concurrent thread in this serial group touches FORGEPLAN_WORKSPACE.
@@ -15940,11 +15985,21 @@ mod resolve_workspace_tests {
             .expect("the param workspace resolves");
 
         assert_eq!(via, ResolutionSource::Param);
+        // Canonicalize the **expectation**, exactly as the sibling precedence
+        // test does for its `y`/`z`. `resolve_read_source` returns a
+        // canonicalized path, while `TempDir::path()` is unresolved — on macOS
+        // `/private/var/...` against `/var/...` — so `starts_with` would fail on
+        // the prefix alone and would fail for a correct resolution too.
+        let expected_param_ws = param_ws
+            .path()
+            .join(".forgeplan")
+            .canonicalize()
+            .expect("the param workspace canonicalizes");
         assert!(
-            resolved_dir.starts_with(param_ws.path()),
+            resolved_dir.starts_with(&expected_param_ws),
             "the branch must key on the PARAM workspace {:?}, got {:?} \
              (server default is {:?})",
-            param_ws.path(),
+            expected_param_ws,
             resolved_dir,
             default_ws.path(),
         );
@@ -16097,9 +16152,15 @@ mod resolve_workspace_tests {
     async fn a_migrated_workspace_resolves_with_no_legacy_store() {
         let ws = mk_fake_workspace();
 
+        // The capability takes the **markdown workspace**, not the project
+        // root: it derives `index_root = workspace.join("lance")` itself.
+        // Handed the root it publishes into `<root>/lance`, beside the
+        // `.forgeplan/` this test then resolves — so `workspace_is_migrated`
+        // reads a workspace that was never migrated and the test silently
+        // exercises the legacy branch it exists to rule out.
         let capability =
             forgeplan_core::constellation::facade::MaintainCapability::for_strict_maintenance(
-                ws.path().canonicalize().unwrap(),
+                ws.path().join(".forgeplan").canonicalize().unwrap(),
             );
         forgeplan_core::constellation::facade::maintain_index(
             &capability,
@@ -16184,6 +16245,14 @@ mod resolve_workspace_tests {
         forgeplan_core::workspace::init_workspace(z_root.path(), "explicit-z").expect("init Z");
         let y = y_root.path().join(".forgeplan");
         let z = z_root.path().join(".forgeplan");
+        // `TempDir` hands back an unresolved path (`/var/...` on macOS), while
+        // `resolve_workspace` canonicalizes what it returns. Comparing a
+        // canonical result against an unresolved expectation failed on the
+        // prefix alone, and would have failed for a correct precedence too. The
+        // production canonicalization is what is under test here; only the
+        // expectation is normalized.
+        let y = y.canonicalize().expect("Y canonicalizes");
+        let z = z.canonicalize().expect("Z canonicalizes");
         for ws in [&y, &z] {
             forgeplan_core::db::store::LanceStore::init(ws)
                 .await
````

Probe exit=0. Этот исполнитель ничего в staging не добавлял и не коммитил.

## Все цели — последние записанные результаты

| Цель / группа | Exit | test result lines | Passed | Failed | Ignored | Выполнение |
|---|---:|---:|---:|---:|---:|---|
| core artifact::sections::tests:: | 0 | 1 | 15 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| core embed::tokenizer::tests:: | 0 | 1 | 14 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| core constellation::capture::tests:: | 0 | 1 | 22 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| core constellation_section_windows + constellation_tokenizer | 0 | 2 | 40 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| mcp server::resolve_workspace_tests | 0 | 1 | 16 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| mcp constellation_local_compat (подтверждение) | 0 | 1 | 19 | 0 | 0 | этот прогон |
| CLI cli_constellation_local_compat + cli_reindex_constellation | 0 | 2 | 20 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| core coverage::tests:: | 0 | 1 | 12 | 0 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |
| CLI cli_uncovered_coverage (предсуществующий вне объёма) | 101 | 1 | 39 | 1 | 0 | сохранённый финальный результат, сейчас NOT_RERUN |


Старый MCP результат18/1/0 и промежуточный confirmation19/0/0 сохранены в прежних run-каталогах; здесь текущий MCP result19/0/0. Итог197/1/0 не суммирует повторно проходы одной и той же target. Сумма по финальному initial run196/2 + двум confirmations19/0 и19/0 = **234 passed / 2 failed / 0 ignored,13 summary lines**; это исторические executions на разных source snapshots, не одна suite-сессия.

## Failing tests — не перезапускалось и не менялось

| Target | Test | File:line | Exact error | State |
|---|---|---|---|---|
| cli_uncovered_coverage | git_sync_announces_the_index_migration | crates/forgeplan-cli/tests/cli_uncovered_coverage.rs:286:10 | Unexpected stdout, failed var.contains(migrated to constellation protocol) | Предсуществующее, вне объёма; #540; NOT_RERUN |

Сохранённые числа этой цели: **39 passed / 1 failed / 0 ignored, exit101,1 test result line**. Как в прошлом фактическом логе, сама git-sync --since HEAD exit0: Git sync: HEAD..HEAD (commit5646fe3); No .forgeplan/ files changed since HEAD.; Done. По сообщению координатора это прежнийF181: production намеренно не мигрирует legacy workspace в git-sync, аC12a ждёт именно миграцию. Здесь диагноз не перепроверялся, исправления не предпринимались. Сохранённый exact raw output приложен ниже.

## AC coverage delta

Parent RFC-015, scoped final rebuild/confirmation/runtime contract. Текущие release/debug/test/scenario commands все exit0, warning checks0, target19/19, scenario60/60. Numeric statement/branch threshold n/a, actual NOT_RUN, delta n/a. Бизнес-тесты coverage::tests:: не названы процентом code coverage. Полный RFC и широкая программа этим EVID не приняты.

## Slow tests (top 5)

Per-test timings stable libtest не вывел, честный top5 отдельных tests недоступен. Время текущего invocation и binary summary приведено выше; оно не переименовано в per-test timing.

## Flaky candidates

Flaky устойчивость NOT_ASSESSED; retries без изменения source не назначались. Повторы на новом source не объявлены flaky passed-on-retry. Наблюдённых unchanged-retry recoveries0, но это не доказательство отсутствия flaky cases.

## Interpretation

После удаления неиспользуемого mut обе сборки реально исполнены без warning, compatibility19/19 и fresh release CLI/MCP60/60 прошли. Release bytes получились идентичны предшествующим, source hash изменился только последней строкой. Это положительное измерение назначенного среза; сохранённый #540 не превращён в green и не навязан как новое исправление.

## Congruence Level Justification

CL3: та же integration copy, заданные toolchain/cache/environment, реальные binaries и stdio processes, captured output+exit и source/binary hashes. Structured refutes сохраняет отрицательное наблюдение общей коллекции из-за одной внешней цели; это не отрицание текущего green scoped execution и не попытка искусственно увеличить R_eff.

## NOT_RUN / ограничения

- После снятия mut повторялись только два builds, MCP target и narrow CLI/MCP scenario; прочие таблицированные Rust targets сейчас NOT_RERUN, их сохранённые результаты помечены.
- git_sync_announces_the_index_migration не перезапускался/не менялся, отдельная находка #540.
- Full workspace suite/широкая матрица/все L01–L16, fault/crash/durability/interprocess race, Windows/Linux, внешняя migration не проверялись.
- Instrumented line/branch coverage, fmt/clippy/check, RUSTFLAGS=-Dwarnings CI job, performance benchmark, packaging/signing не запускались. Проверено ровно отсутствие warning: в заданных командных выводах.
- Re-embed и semantic search после source edit/reindex не запускались: acceptance этого шага — fresh get/keyword/pinned source; semantic_state после reindex unavailable, не выдан за готовые новые vectors.
- Artifact остаётся draft, informs RFC-015; активация не вызывалась и не запрашивается. Source/test/driver edits, git add/commit этим исполнителем не выполнялись.

## Next steps

Назначенный финальный rebuild/confirmation/runtime прогон завершён. Координатор получает текущие binaries/source digest и отдельно ведёт #540; EVID-191 оставить draft. Дополнительные Cargo-команды этим отчётом не предлагаются и не запускаются.

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-015 | informs |

## Command run — current captured stdout/stderr

Exact commands применялись с cwd=/Users/explosovebit/Work/ForgePlan-Constellation-P0; возврат измерен напрямую после перенаправления, без pipe.

### 01-release-build

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
cargo build --offline --locked --release -p forgeplan -p forgeplan-mcp --features forgeplan/semantic-search,forgeplan-mcp/semantic-search > /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/01-release-build.log 2>&1
rc=$?
echo "exit=$rc"
exit "$rc"
````

Exit code=0; wrapper stdout=exit=0; wall=744.376s. Captured combined stdout/stderr verbatim:

````text
   Compiling forgeplan-mcp v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-mcp)
   Compiling forgeplan v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-cli)
    Finished `release` profile [optimized] target(s) in 12m 24s
````

### 02-debug-build

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
cargo build --offline --locked -p forgeplan -p forgeplan-mcp --features forgeplan/semantic-search,forgeplan-mcp/semantic-search > /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/02-debug-build.log 2>&1
rc=$?
echo "exit=$rc"
exit "$rc"
````

Exit code=0; wrapper stdout=exit=0; wall=26.142s. Captured combined stdout/stderr verbatim:

````text
   Compiling forgeplan-mcp v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-mcp)
   Compiling forgeplan v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.06s
````

### 03-mcp-local-compat

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
cargo test --offline --locked -p forgeplan-mcp --features semantic-search --test constellation_local_compat -- --test-threads=1 > /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/03-mcp-local-compat.log 2>&1
rc=$?
echo "exit=$rc"
exit "$rc"
````

Exit code=0; wrapper stdout=exit=0; wall=223.781s. Captured combined stdout/stderr verbatim:

````text
   Compiling forgeplan-mcp v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-mcp)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 16.89s
     Running tests/constellation_local_compat.rs (target/debug/deps/constellation_local_compat-78c873ce699a32f5)

running 19 tests
test a_corrupt_current_pointer_is_a_typed_error_not_an_empty_report ... ok
test a_damaged_canonical_source_is_refused_not_silently_dropped ... ok
test a_running_server_follows_an_external_cli_index_migration ... ok
test a_running_server_sees_a_write_made_by_the_real_cli_process ... ok
test a_semantic_request_without_a_model_is_labelled_as_a_keyword_fallback ... ok
test anomalies_find_the_mistyped_link_on_both_layouts ... ok
test blindspots_name_the_same_artifact_on_both_layouts ... ok
test coverage_maps_the_active_adr_onto_the_module_on_both_layouts ... ok
test decay_names_the_decayed_artifact_on_both_layouts ... ok
test drift_reports_the_affected_files_on_both_layouts ... ok
test fpf_check_evaluates_the_artifact_on_both_layouts ... ok
test fpf_reads_of_a_missing_knowledge_base_create_no_table ... ok
test fpf_reads_return_the_ingested_knowledge_base_on_a_migrated_workspace ... ok
test fpf_search_returns_exactly_the_limit_against_a_larger_knowledge_base ... ok
test health_reports_the_real_corpus_on_a_legacy_workspace ... ok
test health_reports_the_same_corpus_on_a_migrated_workspace ... ok
test journal_scores_linked_evidence_the_same_on_both_layouts ... ok
test release_notes_classify_the_committed_corpus_on_both_layouts ... ok
test the_twelve_reads_on_a_migrated_workspace_create_no_retired_tables ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 196.36s
````


### Current CLI/MCP driver

````bash
export RUSTUP_TOOLCHAIN=1.95
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true
export PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib
export HF_HUB_OFFLINE=1
export FORGEPLAN_MODEL_CACHE=/Users/explosovebit/.forgeplan/models
python3 .local/constellation-verification/narrow_offset_smoke.py --out /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/runtime > /Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/05-runtime-scenario.log 2>&1
rc=$?
echo "exit=$rc"
exit "$rc"
````

Exit=0, wall=30.490s. Literal driver stdout/stderr:

````text
PASS setup.one_prd ["PRD-001.md"]
PASS setup.exact_source {"source_bytes": 246, "expected_body_bytes": 142}
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s1   926Gi   873Gi    12Gi    99%     12M  125M    9%   /System/Volumes/Data
PASS index.slug_is_listed {"index_stdout": "Index maintenance (/private/var/folders/bs/vfybfmbx0zd402nf21tj00z40000gn/T/forgeplan-narrow-offset-6n6pgmpw/project/.forgeplan)\n  generation:  gen-2026-10-22T23-20-39.149Z\n  progress:    CreatedTables { generation_id: \"gen-2026-10-22T23-20-39.149Z\" }\n  commits:     0\n  tables:      artifacts@v2 relations@v1 sections@v2\n  verified:    nothing staged (no-op pass)\n  entries:     1\n\nNext: forgeplan index --embed-profile <profile-id>\n", "index_list_stdout": "Published generation gen-2026-10-22T23-20-39.149Z (artifacts@v2)\nprd-offset-smoke         unavailable  prds/PRD-001.md\nDone.\n"}
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s1   926Gi   873Gi    12Gi    99%     12M  125M    9%   /System/Volumes/Data
PASS embed.cache_complete {"revision": "5617a9f61b028005a4858fdac845db406aefb181", "files": {"onnx/model.onnx": 724923, "onnx/model.onnx_data": 2266820608, "tokenizer.json": 17098108, "tokenizer_config.json": 444, "config.json": 687, "special_tokens_map.json": 964}}
PASS embed.exit_zero {"stdout": "  Loading embedding model...\nEmbedding 6 row(s) — artifacts and their sections...\n\n  prd-offset-smoke\n  prd-offset-smokes0\n  prd-offset-smokes1\n  prd-offset-smokes2\n  prd-offset-smokes3\n  prd-offset-smokes4\n\nDone: 6 embedded, 0 stale, 0 could not be judged, 0 failed (0 recorded for retry).\n\nNext: forgeplan search \"<query>\"\n", "stderr": ""}
PASS indexed.manifest_sha256 {"descriptor": {"format_version": 2, "index_epoch": 1, "checkout_id": "local-77b126633f692b70", "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "owner_epoch": 3, "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096", "config_digest": "effe687b7d160717e1243b89fe997c5fdcee4e57b4e80e69d7d62b0a917dc9ad", "profile_id": "162280666fe6bbc8813e4432d9af741d0377dc40724ef0bf7191fe47ccfc0040", "evaluated_at": "2026-10-22T23:20:39.149Z", "manifest_path": "generations/gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c.json", "manifest_sha256": "cc899e49bafc4a357efb672a9c80697eb378225d84ef9ff5ef24d8c5b473d108"}, "actual_manifest_sha256": "cc899e49bafc4a357efb672a9c80697eb378225d84ef9ff5ef24d8c5b473d108", "source_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_entries": [{"canonical_key": "prd-offset-smoke", "relative_path": "prds/PRD-001.md", "checkout_id": "local-77b126633f692b70", "index_epoch": 1, "incarnation": 1, "row_revision": 1, "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "input_digest": "e224bd7bc0f38caebab496729b9d27c33ba39f773c4f8b582560a2ccea873421", "profile_id": null, "semantic_state": "unavailable", "diagnostic_codes": [], "observed_at": "2026-10-22T23:20:39.149Z"}], "previous_generation": null}
PASS indexed.new_generation {"previous": null, "actual": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c"}
PASS indexed.source_digest {"descriptor": {"format_version": 2, "index_epoch": 1, "checkout_id": "local-77b126633f692b70", "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "owner_epoch": 3, "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096", "config_digest": "effe687b7d160717e1243b89fe997c5fdcee4e57b4e80e69d7d62b0a917dc9ad", "profile_id": "162280666fe6bbc8813e4432d9af741d0377dc40724ef0bf7191fe47ccfc0040", "evaluated_at": "2026-10-22T23:20:39.149Z", "manifest_path": "generations/gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c.json", "manifest_sha256": "cc899e49bafc4a357efb672a9c80697eb378225d84ef9ff5ef24d8c5b473d108"}, "actual_manifest_sha256": "cc899e49bafc4a357efb672a9c80697eb378225d84ef9ff5ef24d8c5b473d108", "source_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_entries": [{"canonical_key": "prd-offset-smoke", "relative_path": "prds/PRD-001.md", "checkout_id": "local-77b126633f692b70", "index_epoch": 1, "incarnation": 1, "row_revision": 1, "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "input_digest": "e224bd7bc0f38caebab496729b9d27c33ba39f773c4f8b582560a2ccea873421", "profile_id": null, "semantic_state": "unavailable", "diagnostic_codes": [], "observed_at": "2026-10-22T23:20:39.149Z"}], "previous_generation": null}
PASS keyword.initial {"ids": ["PRD-001"], "expected_present": true, "response": {"_next_action": "forgeplan get PRD-001", "results": [{"id": "PRD-001", "kind": "prd", "mode": "keyword", "status": "draft", "title": "Offset smoke"}]}}
PASS semantic.constellation {"eligible_rows": 6, "expected_rows": 6, "index": "constellation", "mode": "semantic", "persisted": true, "results": [{"canonical_key": "prd-offset-smoke#s0", "distance": 0.2716354727745056, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.7283645272254944, "similarity": 0.7283645272254944, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke", "distance": 0.34019535779953003, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.65980464220047, "similarity": 0.65980464220047, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s3", "distance": 0.6851521134376526, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.3148478865623474, "similarity": 0.3148478865623474, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s1", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s2", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s4", "distance": 0.7128903269767761, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.2871096730232239, "similarity": 0.2871096730232239, "status": "draft", "title": "Offset smoke"}], "semantic_available": true, "semantic_scope": "prefix+sections", "snapshot_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096"}
PASS semantic.available {"eligible_rows": 6, "expected_rows": 6, "index": "constellation", "mode": "semantic", "persisted": true, "results": [{"canonical_key": "prd-offset-smoke#s0", "distance": 0.2716354727745056, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.7283645272254944, "similarity": 0.7283645272254944, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke", "distance": 0.34019535779953003, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.65980464220047, "similarity": 0.65980464220047, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s3", "distance": 0.6851521134376526, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.3148478865623474, "similarity": 0.3148478865623474, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s1", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s2", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s4", "distance": 0.7128903269767761, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.2871096730232239, "similarity": 0.2871096730232239, "status": "draft", "title": "Offset smoke"}], "semantic_available": true, "semantic_scope": "prefix+sections", "snapshot_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096"}
PASS semantic.complete_multiple_rows {"eligible_rows": 6, "expected_rows": 6, "index": "constellation", "mode": "semantic", "persisted": true, "results": [{"canonical_key": "prd-offset-smoke#s0", "distance": 0.2716354727745056, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.7283645272254944, "similarity": 0.7283645272254944, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke", "distance": 0.34019535779953003, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.65980464220047, "similarity": 0.65980464220047, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s3", "distance": 0.6851521134376526, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.3148478865623474, "similarity": 0.3148478865623474, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s1", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s2", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s4", "distance": 0.7128903269767761, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.2871096730232239, "similarity": 0.2871096730232239, "status": "draft", "title": "Offset smoke"}], "semantic_available": true, "semantic_scope": "prefix+sections", "snapshot_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096"}
PASS semantic.scope {"eligible_rows": 6, "expected_rows": 6, "index": "constellation", "mode": "semantic", "persisted": true, "results": [{"canonical_key": "prd-offset-smoke#s0", "distance": 0.2716354727745056, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.7283645272254944, "similarity": 0.7283645272254944, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke", "distance": 0.34019535779953003, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.65980464220047, "similarity": 0.65980464220047, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s3", "distance": 0.6851521134376526, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.3148478865623474, "similarity": 0.3148478865623474, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s1", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s2", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s4", "distance": 0.7128903269767761, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.2871096730232239, "similarity": 0.2871096730232239, "status": "draft", "title": "Offset smoke"}], "semantic_available": true, "semantic_scope": "prefix+sections", "snapshot_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096"}
PASS semantic.relevant_positive {"eligible_rows": 6, "expected_rows": 6, "index": "constellation", "mode": "semantic", "persisted": true, "results": [{"canonical_key": "prd-offset-smoke#s0", "distance": 0.2716354727745056, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.7283645272254944, "similarity": 0.7283645272254944, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke", "distance": 0.34019535779953003, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.65980464220047, "similarity": 0.65980464220047, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s3", "distance": 0.6851521134376526, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.3148478865623474, "similarity": 0.3148478865623474, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s1", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s2", "distance": 0.6960067749023438, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.30399322509765625, "similarity": 0.30399322509765625, "status": "draft", "title": "Offset smoke"}, {"canonical_key": "prd-offset-smoke#s4", "distance": 0.7128903269767761, "document_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "mode": "semantic", "score": 0.2871096730232239, "similarity": 0.2871096730232239, "status": "draft", "title": "Offset smoke"}], "semantic_available": true, "semantic_scope": "prefix+sections", "snapshot_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "source_set_digest": "259c5be2741cf73e9490e5b34a81dbe3a139431bd49df8d6dfd8abb8a9818096"}
PASS cli.get.initial.id {"id": "PRD-001", "persisted": true, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS cli.get.initial.exact_source_bytes {"id": "PRD-001", "persisted": true, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS cli-mcp.initialize
PASS cli-mcp.initialized {"command": ["/Users/explosovebit/Work/ForgePlan-Constellation-P0/target/release/forgeplan", "serve", "--profile", "full", "--workspace", "/var/folders/bs/vfybfmbx0zd402nf21tj00z40000gn/T/forgeplan-narrow-offset-6n6pgmpw/project"]}
PASS cli-mcp.tools_list {"tool_count": 76, "required_found": ["forgeplan_constellation_get", "forgeplan_get", "forgeplan_search"]}
PASS cli-mcp.get.id {"id": "PRD-001", "persisted": null, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS cli-mcp.get.exact_source_bytes {"id": "PRD-001", "persisted": null, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS cli-mcp.search {"ids": ["PRD-001"], "expected_present": true, "response": {"_next_action": "1 hit(s). Read top result: `forgeplan_get PRD-001`.", "coverage": {"graph": false, "lexical": true, "semantic": false}, "results": [{"bm25_score": 0.0, "id": "PRD-001", "id_canonical": "prd-offset-smoke", "id_display": "PRD-001", "kind": "prd", "matched_lines": ["L12: tailpolicysentinel"], "r_eff": 0.0, "score": 0.0, "semantic_score": 0.0, "slug": "prd-offset-smoke", "status": "draft", "title": "Offset smoke"}], "total": 1}}
PASS cli-mcp.pinned_get.found {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS cli-mcp.pinned_get.key {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS cli-mcp.pinned_get.body_bytes {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS cli-mcp.pinned_get.source_digest {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS cli-mcp.normal_exit
PASS cli-mcp.eof_exit_zero {"exit": 0}
PASS standalone-mcp.initialize
PASS standalone-mcp.initialized {"command": ["/Users/explosovebit/Work/ForgePlan-Constellation-P0/target/release/forgeplan-mcp", "--workspace", "/var/folders/bs/vfybfmbx0zd402nf21tj00z40000gn/T/forgeplan-narrow-offset-6n6pgmpw/project"]}
PASS standalone-mcp.tools_list {"tool_count": 76, "required_found": ["forgeplan_constellation_get", "forgeplan_get", "forgeplan_search"]}
PASS standalone-mcp.get.id {"id": "PRD-001", "persisted": null, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS standalone-mcp.get.exact_source_bytes {"id": "PRD-001", "persisted": null, "body_bytes": 246, "expected_bytes": 246, "body_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "expected_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef"}
PASS standalone-mcp.search {"ids": ["PRD-001"], "expected_present": true, "response": {"_next_action": "1 hit(s). Read top result: `forgeplan_get PRD-001`.", "coverage": {"graph": false, "lexical": true, "semantic": false}, "results": [{"bm25_score": 0.0, "id": "PRD-001", "id_canonical": "prd-offset-smoke", "id_display": "PRD-001", "kind": "prd", "matched_lines": ["L12: tailpolicysentinel"], "r_eff": 0.0, "score": 0.0, "semantic_score": 0.0, "slug": "prd-offset-smoke", "status": "draft", "title": "Offset smoke"}], "total": 1}}
PASS standalone-mcp.pinned_get.found {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS standalone-mcp.pinned_get.key {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS standalone-mcp.pinned_get.body_bytes {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS standalone-mcp.pinned_get.source_digest {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "actual_body_bytes": 142, "expected_body_bytes": 142, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ntailpolicysentinel\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "ready", "source_digest": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "pinned_artifacts_version": 4}}
PASS standalone-mcp.normal_exit
PASS standalone-mcp.eof_exit_zero {"exit": 0}
PASS edit.only_marker_changed {"before_sha256": "e9f0446deb328efeb14e521b83e5a363c4b8b6916686f50822b4a6459588bdef", "after_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS cli.get.edited.id {"id": "PRD-001", "persisted": false, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS cli.get.edited.exact_source_bytes {"id": "PRD-001", "persisted": false, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS cli.get.edited.persisted {"id": "PRD-001", "persisted": false, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS keyword.edited.present {"ids": ["PRD-001"], "expected_present": true, "response": {"_next_action": "forgeplan get PRD-001", "results": [{"id": "PRD-001", "kind": "prd", "mode": "keyword", "status": "draft", "title": "Offset smoke"}]}}
PASS keyword.old.absent {"ids": [], "expected_present": false, "response": {"_next_action": "forgeplan list --type prd", "results": []}}
PASS read_only.current_unchanged {"before_sha256": "6affe26112371f0727a2d0ea4d3cdab5d3828214d30a4799d5e462a464cdb6a4", "after_sha256": "6affe26112371f0727a2d0ea4d3cdab5d3828214d30a4799d5e462a464cdb6a4"}
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s1   926Gi   873Gi    12Gi    99%     12M  125M    9%   /System/Volumes/Data
PASS reindex.exit_zero {"stdout": "Index is constellation-managed — `forgeplan index` owns this layout; running the reconcile pass:\n  reconciled into generation gen-2026-10-22T23-21-05.052Z (3 commit(s))\n", "stderr": ""}
PASS reindexed.manifest_sha256 {"descriptor": {"format_version": 2, "index_epoch": 1, "checkout_id": "local-77b126633f692b70", "generation_id": "gen-2026-10-22T23-21-05.052Z", "owner_epoch": 0, "source_set_digest": "cba61450100c900ddd83991ad0010587aaa3310341215fab439a01c1d21f4685", "config_digest": "effe687b7d160717e1243b89fe997c5fdcee4e57b4e80e69d7d62b0a917dc9ad", "profile_id": "162280666fe6bbc8813e4432d9af741d0377dc40724ef0bf7191fe47ccfc0040", "evaluated_at": "2026-10-22T23:21:05.052Z", "manifest_path": "generations/gen-2026-10-22T23-21-05.052Z.json", "manifest_sha256": "6c3e16d863444b5fcf86e1673a1c2a6e2b9f7e61cdbfab4733f09e5f147bf8ff"}, "actual_manifest_sha256": "6c3e16d863444b5fcf86e1673a1c2a6e2b9f7e61cdbfab4733f09e5f147bf8ff", "source_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_entries": [{"canonical_key": "prd-offset-smoke", "relative_path": "prds/PRD-001.md", "checkout_id": "local-77b126633f692b70", "index_epoch": 1, "incarnation": 1, "row_revision": 2, "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "input_digest": "f3d7f855dcd45c6d81455282f2e23f432d5322b6a723c30abd01a1976c3b1b7b", "profile_id": null, "semantic_state": "unavailable", "diagnostic_codes": [], "observed_at": "2026-10-22T23:21:05.052Z"}], "previous_generation": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c"}
PASS reindexed.new_generation {"previous": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c", "actual": "gen-2026-10-22T23-21-05.052Z"}
PASS reindexed.source_digest {"descriptor": {"format_version": 2, "index_epoch": 1, "checkout_id": "local-77b126633f692b70", "generation_id": "gen-2026-10-22T23-21-05.052Z", "owner_epoch": 0, "source_set_digest": "cba61450100c900ddd83991ad0010587aaa3310341215fab439a01c1d21f4685", "config_digest": "effe687b7d160717e1243b89fe997c5fdcee4e57b4e80e69d7d62b0a917dc9ad", "profile_id": "162280666fe6bbc8813e4432d9af741d0377dc40724ef0bf7191fe47ccfc0040", "evaluated_at": "2026-10-22T23:21:05.052Z", "manifest_path": "generations/gen-2026-10-22T23-21-05.052Z.json", "manifest_sha256": "6c3e16d863444b5fcf86e1673a1c2a6e2b9f7e61cdbfab4733f09e5f147bf8ff"}, "actual_manifest_sha256": "6c3e16d863444b5fcf86e1673a1c2a6e2b9f7e61cdbfab4733f09e5f147bf8ff", "source_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_entries": [{"canonical_key": "prd-offset-smoke", "relative_path": "prds/PRD-001.md", "checkout_id": "local-77b126633f692b70", "index_epoch": 1, "incarnation": 1, "row_revision": 2, "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "input_digest": "f3d7f855dcd45c6d81455282f2e23f432d5322b6a723c30abd01a1976c3b1b7b", "profile_id": null, "semantic_state": "unavailable", "diagnostic_codes": [], "observed_at": "2026-10-22T23:21:05.052Z"}], "previous_generation": "gen-2026-10-22T23-20-49.729Z-adm-a3ea60e48ce5bee00af8d5c2e32e9e32b9ad7e0fdfac7660756540b83604240c"}
PASS cli.get.reindexed.id {"id": "PRD-001", "persisted": true, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS cli.get.reindexed.exact_source_bytes {"id": "PRD-001", "persisted": true, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS cli.get.reindexed.persisted {"id": "PRD-001", "persisted": true, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS standalone-reopened.initialize
PASS standalone-reopened.initialized {"command": ["/Users/explosovebit/Work/ForgePlan-Constellation-P0/target/release/forgeplan-mcp", "--workspace", "/var/folders/bs/vfybfmbx0zd402nf21tj00z40000gn/T/forgeplan-narrow-offset-6n6pgmpw/project"]}
PASS standalone-reopened.tools_list {"tool_count": 76, "required_found": ["forgeplan_constellation_get", "forgeplan_get", "forgeplan_search"]}
PASS standalone-reopened.get.id {"id": "PRD-001", "persisted": null, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS standalone-reopened.get.exact_source_bytes {"id": "PRD-001", "persisted": null, "body_bytes": 250, "expected_bytes": 250, "body_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "expected_sha256": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4"}
PASS standalone-reopened.search {"ids": ["PRD-001"], "expected_present": true, "response": {"_next_action": "1 hit(s). Read top result: `forgeplan_get PRD-001`.", "coverage": {"graph": false, "lexical": true, "semantic": false}, "results": [{"bm25_score": 0.0, "id": "PRD-001", "id_canonical": "prd-offset-smoke", "id_display": "PRD-001", "kind": "prd", "matched_lines": ["L12: credentialrotationrev2"], "r_eff": 0.0, "score": 0.0, "semantic_score": 0.0, "slug": "prd-offset-smoke", "status": "draft", "title": "Offset smoke"}], "total": 1}}
PASS standalone-reopened.pinned_get.found {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "actual_body_bytes": 146, "expected_body_bytes": 146, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ncredentialrotationrev2\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "unavailable", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-21-05.052Z", "pinned_artifacts_version": 5}}
PASS standalone-reopened.pinned_get.key {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "actual_body_bytes": 146, "expected_body_bytes": 146, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ncredentialrotationrev2\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "unavailable", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-21-05.052Z", "pinned_artifacts_version": 5}}
PASS standalone-reopened.pinned_get.body_bytes {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "actual_body_bytes": 146, "expected_body_bytes": 146, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ncredentialrotationrev2\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "unavailable", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-21-05.052Z", "pinned_artifacts_version": 5}}
PASS standalone-reopened.pinned_get.source_digest {"found": true, "canonical_key": "prd-offset-smoke", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "actual_body_bytes": 146, "expected_body_bytes": 146, "response": {"_next_action": "Search the same generation: `forgeplan_constellation_search query=\"<text>\"`.", "artifact": {"body": " \t\nIntro. How long can a sign-in credential remain valid?\ncredentialrotationrev2\n\n## Same\n\nalpha\n\n## Same\n\nalpha\n\n## Empty\n\n## Unicode\n\n é\n   \n", "canonical_key": "prd-offset-smoke", "id": "PRD-001", "kind": "prd", "semantic_state": "unavailable", "source_digest": "92df15b80acfaea3d0bb983cc681311a40c5cad85578fba029161de605dd25e4", "source_path": "prds/PRD-001.md", "status": "draft", "title": "Offset smoke"}, "found": true, "generation_id": "gen-2026-10-22T23-21-05.052Z", "pinned_artifacts_version": 5}}
PASS standalone-reopened.normal_exit
PASS standalone-reopened.eof_exit_zero {"exit": 0}
{"result": "PASS", "failed_steps": [], "checks_passed": 60, "checks_failed": 0, "summary": "/Users/explosovebit/Work/ForgePlan-Constellation-P0/.local/constellation-verification/narrow-offset-warning-clean-20261022T230314Z/runtime/scenario-summary.json"}
````


### Retained out-of-scope failure — previous raw output, not a new run

Command: cargo test --offline --locked -p forgeplan --features semantic-search --test cli_uncovered_coverage -- --test-threads=1; exit101.

````text
warning: variable does not need to be mutable
    --> crates/forgeplan-mcp/src/server.rs:7905:13
     |
7905 |         let mut hits: Vec<FpfSearchHit> = results
     |             ----^^^^
     |             |
     |             help: remove this `mut`
     |
     = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: `forgeplan-mcp` (lib) generated 1 warning (run `cargo fix --lib -p forgeplan-mcp` to apply 1 suggestion)
   Compiling forgeplan v0.39.0+constellation.s1.1 (/Users/explosovebit/Work/ForgePlan-Constellation-P0/crates/forgeplan-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.43s
     Running tests/cli_uncovered_coverage.rs (target/debug/deps/cli_uncovered_coverage-27624e65b92ac585)

running 40 tests
test ci_assign_id_dry_run_no_candidates_emits_json ... ok
test context_existing_artifact_renders_id_and_status ... ok
test context_json_has_artifact_and_validation_fields ... ok
test context_missing_artifact_errors ... ok
test git_sync_announces_the_index_migration ... FAILED
test git_sync_since_head_no_changes ... ok
test git_sync_without_orig_head_emits_fix ... ok
test init_non_interactive_never_mentions_or_fetches_the_model ... ok
test init_with_model_flag_is_accepted_on_the_non_interactive_path ... ok
test log_after_create_shows_entry ... ok
test log_empty_workspace_no_entries ... ok
test log_json_emits_entries_array ... ok
test migrate_dry_run_empty_workspace_no_collisions ... ok
test migrate_dry_run_json_schema_v1 ... ok
test migrate_runs_schema_migrations ... ok
test promote_memory_to_note_creates_new_artifact ... ok
test promote_non_memory_errors ... ok
test recall_after_remember_returns_memory ... ok
test recall_empty_workspace_shows_no_memories ... ok
test recall_json_carries_provenance ... ok
test recall_json_emits_memories_array ... ok
test reconcile_ids_check_only_emits_json ... ok
test remember_creates_memory_artifact ... ok
test remember_forget_removes_memory ... ok
test remember_list_after_capture_shows_entry ... ok
test remember_list_empty_shows_no_memories ... ok
test remember_list_shows_the_git_author_column ... ok
test reopen_draft_artifact_errors_with_fix_hint ... ok
test scan_import_dry_run_empty_project ... ok
test scan_import_finds_external_markdown ... ok
test setup_creates_the_fpl_alias_next_to_the_binary ... ok
test setup_is_idempotent ... ok
test setup_refuses_to_overwrite_an_existing_fpl ... ok
test setup_skill_writes_skill_file_under_fake_home ... ok
test tag_adds_tag_to_artifact ... ok
test tag_missing_artifact_errors ... ok
test tree_empty_workspace_shows_empty_message ... ok
test tree_json_emits_valid_array ... ok
test tree_with_artifact_renders_id ... ok
test untag_removes_tag_from_artifact ... ok

failures:

---- git_sync_announces_the_index_migration stdout ----

thread 'git_sync_announces_the_index_migration' (6682278) panicked at crates/forgeplan-cli/tests/cli_uncovered_coverage.rs:286:10:
Unexpected stdout, failed var.contains(migrated to constellation protocol)
├── var:   Git sync: HEAD..HEAD (commit 5646fe3)
│     No .forgeplan/ files changed since HEAD.
│   
│   Done.
└── var as str:   Git sync: HEAD..HEAD (commit 5646fe3)
      No .forgeplan/ files changed since HEAD.
    
    Done.

command=`cd "/var/folders/bs/vfybfmbx0zd402nf21tj00z40000gn/T/.tmpyS0rKe" && "/Users/explosovebit/Work/ForgePlan-Constellation-P0/target/debug/forgeplan" "git-sync" "--since" "HEAD"`
code=0
stdout=```
  Git sync: HEAD..HEAD (commit 5646fe3)
  No .forgeplan/ files changed since HEAD.

Done.
```

stderr=""

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    git_sync_announces_the_index_migration

test result: FAILED. 39 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 52.15s

error: test failed, to rerun pass `-p forgeplan --test cli_uncovered_coverage`
````


