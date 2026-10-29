---
assigned_number: 204
author: null
created: 2026-10-28
depth: tactical
id: EVID-204
kind: evidence
links:
- target: RFC-018
  relation: informs
parent_epic: null
predicted_number: 204
slug: evid-r11-automatic-bge-m3-cli-and-mcp-pilot-verification-ba7e7b0a
status: draft
tags: []
title: R11 automatic BGE-M3 CLI and MCP pilot verification ba7e7b0a
updated: 2026-10-28
valid_until: null
---

---
assigned_number: 204
created: 2026-10-28
predicted_number: 204
slug: evid-r11-automatic-bge-m3-cli-and-mcp-pilot-verification-ba7e7b0a
updated: 2026-10-28
---

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3
review_verdict: CONCERNS
source_pin: ba7e7b0a400185eacbe710297532dbce31cc90a5c99c106a84f017a705246c10

## Measurement

2026-10-28, macOS/arm64. Исполнитель: Main, openai-codex/gpt-6.1-sol. Проверена ограниченная гипотеза: настроенный full MCP автоматически вычисляет векторы изменённого документа, а реальные CLI/MCP находят новую редакцию по смыслу без ручного embed после записи. Это не полный релизный gate.

Код: /Users/explosovebit/Work/ForgePlan-Constellation-P0. Снимок включает 442 Rust/Cargo-входа. По сравнению с R9 изменены только constellation/facade.rs, jobs.rs и runtime.rs. Модель, размерность, профиль, схемы и параметры CLI/MCP не менялись. Production CLI/MCP собраны стандартным release-профилем с semantic-search, без запрошенного test-helpers; build exit 0. Это не пересборка default-конфигурации без semantic-search.

Пилот: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/ForgePlan. В нём 449 документов. Первоначальные 6048 разделов полностью подготовлены на R9: публичный semantic consumer подтвердил 6497/6497 пригодных строк. После контрольного редактирования разделов 6049, всего 6498 строк. BGE-M3 взят из существующего локального кэша, chunk_size=2000, HF_HUB_OFFLINE=1; загрузок модели и платных provider-вызовов не было. Оригинальные проекты, их MCP-конфиги и глобальный PATH не переключались.

## Findings

1. Исправлен пропуск startup/full-scan работы. В R9 реальная CLI-правка уже публиковала текст, но не имела durable pending jobs. Затем full MCP выполнил 18 полных сканов с 0 commits и не запускал вычисление: публичный поиск оставался на 6495/6498 и не находил изменённый документ. Условие queued > 0 || Signalled исключало такую работу. R11 проверяет реальную пригодность строк на каждом выполненном проходе; полностью текущие строки не создают модельный worker. Проверка использует существующий guarded capture и освобождает владельца индекса до работы модели.
2. Независимое Sol-ревью R10 нашло HIGH в остановке первого drain: мостик shutdown создавался только после startup. Новый тест на реальном опубликованном индексе с изначально пустым pending выполнил assertion и упал: 0 passed, 1 failed, exit 101. В R11 runner stop/drop обращается к тому же DrainControl до startup; публичные сигнатуры сохранены. Ожидающий вызов не отменяет задачу записи. Тест проверяет завершение runner до установки loop bridge и неизменность публикации после освобождения позднего native call; это не полный host с реальной моделью.
3. Есть четыре реальные аномалии данных, не дефекты автоматического embedding: две прежние high orphan_link из EVID-157 на ADR-021/PROB-086 и две medium expired_evidence. Срок EVID-085 и EVID-086 закончился 2026-10-27. На свежем health от 2026-10-28: needs_attention, 449 документов, orphans=0, at_risk=3; ADR-008, EPIC-006 и PRD-071 имеют R_eff=0.10. Даты отчётов не продлевались, чужая история не импортировалась. Триггер возврата: повторно проверен предмет этих Evidence либо обоснована их замена; судьба отсутствующих документов установлена отдельно.

## Result

| Проверка | Фактическое наблюдение |
|---|---|
| R11 format и production release build | exit 0; новые CLI/MCP выбранного набора |
| Адресные тесты на окончательном R11 | 5 stop-control + 1 eligibility-report + 3 host + 1 runner-drop = 10 passed, 0 failed, 0 ignored; нулевые прогоны не засчитывались |
| Восстановление CLI-правки старого R9 | R11 автоматически обработал прежний backlog; CLI нашёл NOTE-057 по русскому запросу о лавине, keyword-контроль не нашёл; 6498/6498, pending=0. Запись R9 не выдаётся за запись R11 |
| Новая запись через R11 MCP | get/update/get сохранили полный английский текст о восстановлении кораллового рифа; MCP semantic search по-русски вернул NOTE-057, semantic_score=0.5699259638786316, bm25_score=0; отдельный keyword search пуст |
| Привязка после MCP | row_revision=6, source SHA-256 315e92733ff011e795d5b0ee62df6040c1d20c557ef3474d3c06cb8a284d3308; физический файл равен MCP get.body; строгий публичный CLI semantic consumer подтвердил 6498/6498, pending=0 и то же поколение |
| Новая запись CLI той же сборкой R11 | CLI update/get, затем R11 full MCP сам обработал backlog. Строгий CLI semantic search нашёл note-forgeplan#s3: score=0.6127994656562805; keyword-контроль пуст; writer, maintenance и consumer имеют один source pin R11 |
| Последняя редакция пилота | row_revision=7; source SHA-256 9ebb87991a8f945a84dcbbf6d31309a40ffc0546bf1ea5db02a54dd0acaf842f; 6498/6498, pending=0; profile_id не изменился |
| Ручное доиндексирование после правок | explicit embed/index --embed-profile не вызывались; RPC provenance содержит только get/update/get/search/search |

CLI SHA-256: be576c6394f41e47169f7577e2e316ce9c883880cf712f221302bb9ba75b5c48.
MCP SHA-256: 7b2e207f3eb878b50ea2200e3c7bc48a72bd5a3c10b6506687c2ea08bbe5ce2a.
Сборка: .local/constellation-verification/compatibility-20261025/quality-r11-auto-backlog-sole/release-semantic/. Выбрана только в private pilot/bin; точный R9 release сохранён в bin/history/r9-release-before-r11.

## Interpretation

Реальное BGE-M3 вычисление и поиск подтверждены, не заменены keyword fallback или mock. CLI-проба и MCP-проба использовали разные английские темы и русские смысловые запросы. Проверены канонические байты, новая ревизия, текущий профиль, полное покрытие и отсутствие keyword-совпадения. Поле semantic_state в source_entries manifest само по себе не было доказательством готовности: решающими были публичные semantic consumer и привязки опубликованного поколения.

Сохранены неудачные и промежуточные результаты. Первый MCP verifier требовал CLI-поля persisted/mode, которых нет в сохранённом MCP UnifiedSearch-контракте coverage/results/total. Это ошибка verifier, не исправление API. Итоговая проверка учитывает настоящий semantic_score, отрицательный keyword-контроль и отдельно проверяет готовые bindings публичным CLI. В MCP startup наблюдались промежуточные 0 embedded/5 still eligible, затем signalled-проход завершился 2 embedded/0 stale/0 failed/0 eligible. При намеренном EOF после готовой публикации CLI-host печатал IndexBusy stopping; последующий consumer подтвердил pending=0 и полное покрытие. Эти сообщения не скрыты и не засчитаны как model failure.

## Congruence Level Justification

CL3: окончательные адресные проверки, production build и новые CLI/MCP записи относятся к одному R11 pin и одной macOS/arm64 среде. Отдельно подписаны первоначальное индексирование R9 и восстановление записи старого writer; их результаты не прибавлены к 10 тестам R11. После исправления в R11 ни модель, ни embedding profile не подменялись.

## Limits and independent review

SolCoreFinalReview, фактическая модель openai-codex/gpt-6.1-sol, независимо нашёл и затем проверил исправление R10 HIGH. Окончательный ограниченный R11 verdict: CONCERNS, честные 0 оставшихся существенных findings. Это статический разбор, не дополнительный runtime PASS.

Полные Rust matrix/clippy, 80% coverage, внешний provider, Windows/NTFS/crash cases, G1/G2/G3 и #542 не закрыты. Не проверено качество ranking для всего исторического корпуса. ForgePlanWeb с 5 и ForgePlanMarketplace с 11 группами конфликтующих ID не подключались; файлы не объединялись и не перенумеровывались. Ничего не активировано; Evidence и RFC-018 остаются draft.

При оформлении Evidence в исходном рабочем дереве P0 отдельно наблюдались пять старых scoring-предупреждений: EVID-136 verdict=concerns, EVID-179 verdict=inconclusive, EVID-033/EVID-034/EVID-035 без structured fields. Они сохранены в r11-canonical-workspace-warnings.json, не переписаны и не относятся к четырём аномалиям изолированного пилота. Проверка самого EVID-204 дала PASS с 0 errors/0 warnings; это не утверждение об отсутствии предупреждений во всём корпусе P0. В сохранённых native CLI receipts самого пилота строк warn: не обнаружено; управляемые остановки host разобраны выше.

Для следующих изменений должен работать full maintenance host с --auto-index и конфигурацией embedding. Сам файл конфигурации процесс не запускает; обычный CLI update без работающего host сохраняет Markdown и согласовывает текстовый индекс, но не держит фоновую службу. После проверки native MCP-процессы завершены EOF/exit 0. Триггер повторного разбора: новая CLI/MCP запись не достигает текущих bindings и semantic hit без explicit embed либо stop теряет работу/публикует поздний вектор.

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-018 | informs |

## References

- EVID-203 — отдельное историческое измерение R6, не текущий health и не результат R11.
- Issue #538: https://github.com/ForgePlan/forgeplan/issues/538#issuecomment-6401260584 — наблюдения, исправления и условия возврата.
- Raw root: /Users/explosovebit/Work/ForgePlan-Pilots.SoPuaC/semantic-verification/.
- r11-complete-evidence-manifest.json содержит 36 raw receipts с абсолютным путём, размером и SHA-256; SHA-256 manifest: 56b94c1a4d4b56551663f4c4a90c71a75e6c85dd33c12299a07d9f0deaac5888.
- Ключевые proofs: r11-cli-current-automatic-proof.json, r11-mcp-automatic-proof.json; полные stdio transcripts находятся в соседнем receipts/.
