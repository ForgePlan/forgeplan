---
depth: standard
id: EVID-180
kind: evidence
last_modified_at: 2026-10-19T09:46:33.453811+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: EPIC-010
  relation: informs
status: draft
title: 'Constellation three-project CLI and real MCP E2E: 20 PASS, 4 FAIL, external-agent access observed'
---

---
assigned_number: 180
created: 2026-10-19
predicted_number: 180
slug: evid-constellation-three-project-cli-and-real-mcp-e2e-20-pass-4-fail-external
updated: 2026-10-19
---

# Три проекта: реальный CLI и внешний MCP-клиент

## Structured Fields

evidence_type: measurement
verdict: weakens
congruence_level: 3

## Measurement

Исполнитель: omp-coding-agent / openai-codex/gpt-6.1-sol. Цель — проверить существующую федерацию трёх самостоятельных .forgeplan, а не только сборку. Worktree: /Users/explosovebit/Work/ForgePlan-Constellation-P0, база 925111ec20fc036ead92a61d19636515497d6cc1, исходники UNCOMMITTED.

Пользователь явно разрешил полную очистку только target этого worktree. cargo clean удалил 21.9 GiB; соседние worktree, canonical .forgeplan и модель не очищались. После очистки свободно около 25 GiB. Сборка одного CLI: cargo build -p forgeplan --bin forgeplan --no-default-features --offline --locked. Один cargo за раз, исходники заморожены, защитный останов при свободном месте менее 3 GiB. Окружение сборки: RUSTUP_TOOLCHAIN=1.95.0, CARGO_BUILD_JOBS=4, CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, локальные PROTOC и DYLD_FALLBACK_LIBRARY_PATH из EVID-179. Эти параметры не записаны в глобальную конфигурацию или Cargo.toml.

Первый build runner завершил cargo, но упал в записи checksum: hashlib.file_digest отсутствует в Python 3.9. Исправлен вспомогательный runner; повторный cargo build завершился exit 0 за 0.784 s на уже собранном дереве. Первый log сохранён, его wrapper exit 1 не назван PASS.

Build provenance: .local/constellation-verification/three-project-e2e/02-build-no-semantic-provenance.json. Источники до/после совпали: sha256 75eb9172fb38b034d6bbb2f0db0fdb7db2b31924e132b8178b85586c92977fa1. Cargo.lock sha256 aaf18b0ac557e74848a826d54f8386d21a8d0f16b37c01fcd4555f4f8de2622a. rustc/cargo 1.95.0, aarch64-apple-darwin. Свободно после успешной записи provenance 21,999,652,864 bytes.

Исполнялся сохранённый бинарник .local/constellation-verification/bin/no-semantic/forgeplan, SHA256 cecf8b3d515cb4b3847f2542859d9e0cd35ad9c49d2451f786bcadece619ee2a, 325,718,336 bytes. Это не /opt/homebrew/bin/forgeplan: установленная 0.38.0 не имеет constellation subcommand и в запуске сценариев не участвовала. Модель и semantic channel не использовались.

Драйвер .local/constellation-verification/three_project_probe.py запущен существующим Python 3.13.5 с MCP SDK 1.28.1 из Serena venv. SDK создаёт настоящий stdio subprocess, initialize, tools/list и tools/call. Никакого ручного printf transport или прямых вызовов Rust facade. ClientInfo name=external-context-agent-probe — метаданные клиента, не разрешение.

Изолированные temporary fixtures hub/shared/billing с различимыми ADR-001 и собственными Git commit OID. HOME/USERPROFILE/XDG_DATA_HOME дочерних процессов направлены во временный реестр; пользовательский реестр не менялся. Наборы: product={hub,shared,billing}; hub-only={hub}; backoffice={shared,billing}; optional={hub,shared,billing(optional)}. Данные fixtures и полный stdout/stderr каждого вызова сохранены в probe-01; временные источники после прогона удалены.

## Result

Реальный probe завершился exit 1 за 6.86 s: 20 PASS, 4 FAIL, 3 OBSERVED. Это красный приёмочный результат, не готовая Constellation.

PASS: поиск всех 3 проектов, различение одинаковых ID, квалифицированное get в CLI/MCP, отказ неизвестному qualifier, limit 1 с coverage всех источников, поздний required failure, явный partial и optional failure, qualified Git snapshot, context units из 3 проектов, 9 read-profile tools без write tools, прямой forgeplan_new отвергнут как tool not found, raw path не принят за зарегистрированную федерацию, qualifier не выходит за scope, use не меняет существующий список участников, pinned snapshot сохраняет прежний Git текст после рабочей правки, закрытый snapshot возвращает snapshot_expired. Финальные хеши .forgeplan всех 3 источников совпадают с исходными.

FAIL 1 — CLI.context_final_payload_budget_F08: budget=1, cap=4 bytes, exit 0 и stdout 1603 bytes.
FAIL 2 — CLI.cross_project_dependency_resolution: declared based_on target ADR-001@shared остаётся resolved=null в графе трёх проектов.
FAIL 3 — MCP.working_handle_retains_captured_bytes_F03: старый scope handle после правки вернул shared-marker-v2 вместо захваченного shared-marker-v1; ошибки snapshot_expired не было.
FAIL 4 — MCP.context_final_payload_budget_F08: budget=1, cap=4 bytes, успешный ответ без insufficient_budget; измеренный serialized CallToolResult 1897 bytes.

OBSERVED: один клиент открыл по имени product, hub-only и backoffice. Per-agent ACL не запрашивалась и в reviewed server path отсутствует. use меняет будущий default open, не разрешения. Добавить имя backoffice как вложенный источник product нельзя: CLI exit 1, registry unchanged. Реестр хранит плоские наборы source paths, один проект может участвовать в нескольких наборах. В read-профиле нет list-инструмента для обнаружения имён федераций; имя задаёт оператор либо выясняет через CLI list.

Сырые результаты: .local/constellation-verification/three-project-e2e/probe-01/results.json; events.jsonl содержит точные argv/cwd/env overrides/exit/stdout/stderr и MCP requests/responses; mcp-server.stderr.log содержит stderr процесса. Контрольные fixture markdown сохранены отдельно. Полный build manifest — bin/no-semantic/build.json.

## Interpretation

Проверенная полезная часть — доверенный локальный read-сценарий по трём проектам: lexical search, explicit qualified get и Git-pinned чтение через CLI/MCP. Это не удалённый аутентифицированный gateway и не multi-tenant isolation. Нельзя обещать агенту доступ только к active федерации: явно именованный open может открыть другой зарегистрированный набор. Общий доступ клиента к реестру не означает, что все источники автоматически участвуют в каждом запросе: scope всё равно ограничивает выбранные источники.

Рабочий scope пока не является immutable captured snapshot. Большой стандартный budget, успешный pack или lexical zero result не закрывают F08/F09. Cross-project graph/impact не подтверждён. Ожидания не ослаблены ради GREEN. Не проверялись настоящая модель, semantic matrix, полный legacy/migrated mutation cutover, readonly mount, hostile path swaps, Windows/NTFS, handles TTL/caps и F10 30x3. Сравнение хешей до/после не доказывает отсутствие кратковременных disk writes между измерениями.

## Congruence Level Justification

CL3: настоящий CLI текущего source fingerprint и настоящий MCP stdio consumer; fixtures выполняют конкретный трёхпроектный пользовательский сценарий. Область доказательства ограничена no-semantic режимом и перечисленными проверками.

## Revisit Trigger

После исправления immutable handles, фактического byte budget и qualified graph resolution повторить те же 4 отрицательных сценария без изменения ожидаемых результатов, затем полный L01–L16/F01–F10 и semantic/model/platform gates. При намерении подключить недоверенного или удалённого агента сначала отдельный контракт идентификации, авторизации и изоляции; существующий локальный read profile такого обещания не даёт.

## Дополнительные исполненные проверки

- 03-federation-rust-tests: cargo test -p forgeplan-core --lib constellation::federation::tests --offline --locked -- --test-threads=1, exit 0. Один test binary, 17 PASS, 0 FAIL, 0 ignored; 2553 filtered out. Среди них три новые регрессии обхода источников/лимита. В lib-test compilation было одно dead_code warning в существующем anomalies.rs::new_artifact; clippy-clean не заявляется.
- 04-publication-integration-tests: первая реальная попытка preservation suite — 22 PASS, 2 FAIL из 24, exit 101. Второй pinned-search binary из-за остановки cargo ещё не исполнялся. Сырой красный log сохранён.
- Диагноз двух assertions: C01-тест ожидал публикацию только ADR-002, не учитывая section ADR-002#s0; late-completion тест требовал исчезновения A profile из pending rows, хотя его собственный setup и контракт adoption явно сохраняют эти rows. Производственный код ради этих assertions не менялся. В первом тесте ожидается точный затронутый набор artifact+section; во втором вместо неверного запрета A profile проверяются неизменность CURRENT bytes и равенство semantic rows до/после отказа. Отказ ProfileIncompatible, отсутствие опубликованных vectors и сохранение pending jobs по-прежнему проверяются.
- 05-publication-integration-corrected-oracles: exit 0, два реально исполненных test binaries: preservation 24 PASS/0 FAIL; pinned_search 5 PASS/0 FAIL. Изменён только внешний тестовый файл, он отформатирован rustfmt edition2024. Хеши двух test source files сохранены в 05-publication-test-inputs.json и совпали до/после исполнения. Production source fingerprint и сохранённый CLI binary остались прежними. Это не реальный model inference: в publication tests вычисления подаются различимыми постоянными vectors.
- Итого по финальным Rust-регрессиям: 3 test binaries, 46 PASS/0 FAIL; первоначальные 22 повторных PASS не прибавлены второй раз. Общая приёмка программы остаётся красной из-за четырёх независимых CLI/MCP сценариев probe-01.
- probe-access-02: ещё один настоящий MCP subprocess сценарий, exit 0, 4 проверенных наблюдения. Новая регистрация late-registration становится доступна в уже запущенном MCP; удаление billing из product влияет на новый open, но старый scope продолжает читать billing; запуск с отдельным HOME и реестром только restricted={hub} отвергает open(product) как no_scope; итоговые source bytes не изменены. Это подтверждение поведения реестра, не PASS per-agent ACL. events.jsonl и server stderr сохранены рядом с results.json; исполненный driver.py сохранён для происхождения.
- Уточнение F08 измерения: у MCP ответа один text content содержит 1602 UTF-8 bytes при cap4. Это уже достаточное нарушение независимо от JSON escaping. 1897 bytes — пересериализованная SDK-модель CallToolResult, не перехваченный transport frame; точная величина server-side framing здесь не заявляется.

## Проверенный порядок для агента

Оператор регистрирует source paths через constellation create/add и сообщает агенту имя набора. Один проект допускается в нескольких наборах; например product={hub,shared,billing}, backoffice={shared,billing}. Вложенного набора федераций нет.

MCP-клиент запускает сохранённый CLI с args [serve, --profile, constellation-read]. Не использовать standalone forgeplan-mcp или default full profile вместо проверенного read profile. Для ограниченного набора подключений использовать отдельный пользовательский реестр этого процесса; HOME изолируется на уровне дочернего процесса, как в реальном probe. Это не полноценная OS sandbox и не идентификация пользователя.

Агент вызывает constellation_open({constellation:product}), берёт возвращённый scope_handle. Для воспроизводимой работы по committed данным вызывает constellation_snapshot({handle:<scope_handle>}), далее constellation_search({snapshot_handle:<returned_snapshot_handle>,query:...}) и constellation_get({snapshot_handle:<returned_snapshot_handle>,id:ADR-001@shared}). Для другого набора открывает отдельный именованный scope и snapshot. Обычный working scope пока не даёт обещанную неизменность bytes. Git snapshot фиксирует Git commits, не незакоммиченные рабочие правки.

Клиент получает действующие права read server OS process и его реестр. Active use не является ACL. Удаление source из реестра не отзывает уже выданные handles; новые registry entries видны будущим open без рестарта. Для недоверенного удалённого клиента необходим отдельный будущий контракт доступа; этот бинарник не объявляется remote gateway. Только подключение MCP само по себе не публикует federations как сетевой сервис.

## Дополнительная совместимость full MCP на migrated workspace

06-mcp-migrated-read-tests: настоящий test binary запущен; исходный результат 4 PASS/2 FAIL, exit101. Один graph test создавал corpus без единой связи, но ожидал отображения узла в прежнем edge-only renderer. Routing test использовал default-side/target-side с общим поисковым термом side; его zero-match ожидание не отделяло workspace routing от BM25 token matching.

Исправлены fixtures, не production: у evidence явно объявлена связь informs→PRD-001, graph assertion проверяет оба конца объявленного ребра; для routing заданы разные цельные tokens originworkspacesentinel/targetworkspacesentinel. Ожидание default-scoped zero result и запрет создания legacy tables не ослаблены. Файл отформатирован rustfmt2024.

07-mcp-migrated-corrected-oracles: exit0, один реально исполненный binary, 6 PASS/0 FAIL. Проверены migrated get/list/search/graph/validate без создания legacy tables, explicit workspace Y при default X и сохранение legacy read. Это JSON-RPC fixture этих handlers, отдельно от настоящего CLI stdio consumer probe-01. Проверка full mutation cutover этим результатом не заменяется.

Test source SHA256 209cca31d2b7b143015764207ac71d6ca719dac4f29e903ca2a334b40532772a сохранён в 07-mcp-test-inputs.json и совпал до/после. Production source fingerprint не изменился; test source fingerprints учитываются отдельно. Последний запуск: свободно17,658,503,168 bytes. Финальные уникальные Rust cases: 4 binaries, 52 PASS/0 FAIL. Первоначальные красные результаты06/04 остаются в логах и не прибавлены к итоговым PASS второй раз.

Для двух федераций агент использует два отдельных scope/snapshot handle; один pack-вызов не принимает список handles и не объединяет вложенную сеть. При объединении ответов из пересекающихся наборов нужно сохранять source/version refs; один ADR-001 не является глобально уникальным ключом. В текущем MCP discovery имён наборов отсутствует: оператор сообщает имена или получает их через CLI constellation list.

