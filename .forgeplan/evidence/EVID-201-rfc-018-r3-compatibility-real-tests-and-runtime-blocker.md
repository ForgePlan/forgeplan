---
assigned_number: 201
author: null
created: 2026-10-26
depth: tactical
id: EVID-201
kind: evidence
links:
- target: RFC-018
  relation: informs
parent_epic: null
predicted_number: 201
slug: evid-rfc-018-r3-compatibility-real-tests-and-runtime-blocker
status: draft
tags: []
title: 'RFC-018 R3 compatibility: real tests and runtime BLOCKER'
updated: 2026-10-26
valid_until: null
---

---
assigned_number: 201
created: 2026-10-26
predicted_number: 201
slug: evid-rfc-018-r3-compatibility-real-tests-and-runtime-blocker
updated: 2026-10-26
---

# RFC-018: результаты R3, допуск заблокирован

## Structured Fields

verdict: weakens
review_verdict: BLOCKER
congruence_level: 3
evidence_type: test

## Предмет и границы

Проверяется совместимость прежних CLI и full MCP с новым хранением по RFC-018. Проверенный source pin: a83301cb4772ebe61aa08cb66c8b3103667249a058feab1f797683db774ed171, 93 файла, дополнительный search.rs совпадал с c852. Платформа darwin/arm64. Это результаты замороженного R3, а не более поздних исправлений и не опубликованный релиз.

Исполнитель: CompatR3FinishTester, фактическая модель openai-codex/gpt-6.1-sol. Он не правил исходники и завершил аренду в quality-r3-finish-sole/SAFELEASE-RELEASED.json с нулём своих процессов и 93/93 совпадающими SHA. Этот документ подготовлен интегратором по его неизменяемым raw receipts; он не подменяет подписанный исходный отчёт. В исходном payload поле verdict содержит BLOCKER, а congruence_level — CL2; здесь разделены машинный scoring verdict weakens и review_verdict BLOCKER. Это не исправление авторского отчёта и не повышение общего допуска.

## Фактические результаты

- На R3: 320 PASS, 4 FAIL, 0 ignored, 29 непустых test binaries. Из них этот исполнитель выполнил 68 PASS и 4 FAIL в 9 бинарниках; ранее выполненные 252 PASS учтены отдельно, не запускались заново. Исторические R2 unit/bin 3203 PASS остаются на своём снимке.
- Все четыре process_e2e падения: ожидание публикации закончилось сроком с storage_io/post-commit unexpected_row PRD-901#s0. Проверки не ослаблялись и неизменённый прогон не повторялся.
- Semantic all-targets check завершился E0106 в private cfg(test, semantic-search) helper search.rs:773. Производственный semantic build без test-helpers прошёл независимо; его успех не закрывает ошибку тестовой сборки.
- Настоящая локальная BGE-модель: 9 eligible из 9 expected, prefix+sections, релевантный PRD выше нерелевантной NOTE. После принятия начального профиля оба full stdio-хоста автоматически обновили embeddings после правки, без ручного embed после этих правок.
- Offline source edit дал точное ephemeral чтение с persisted:false и неизменным CURRENT; явный reindex дал новый SHA/persisted:true. Повторное открытие обоих хостов прошло.
- Зафиксированы 39 full-host процессов с EOF exit0 и 814 реальных вызовов. Это не доказательство полной приёмки всех flags/layouts/ветвей 103 CLI и 73 MCP.

## Открытые наблюдения

1. Четыре process-publication FAIL.
2. Структурно неверный links mapping принимается CLI get/validate и обоими full MCP как успешный результат с потерей рёбер, несмотря на InvalidLinks diagnostic.
3. Запросы workspace Y пишут activity в startup X; X canonical/db/CURRENT не изменены, но strict изоляция журнала не выполнена.
4. Абсолютный существующий playbook внутри выбранного Y отклоняется; относительные/name вложенные операции прошли отдельно.
5. Повреждённая копия FPF knowledge base вызывает panic101 в Lance dataset.rs:612 вместо recoverable typed error.
6. E0106 тестового helper. Более поздняя lifetime-аннотация не проверена этим R3 прогоном.

Исходный отчёт также содержит два наблюдения, которые нельзя автоматически считать новыми регрессиями: отсутствие CLI receipts после deprecate/supersede и различие legacy API body с полным физическим Markdown. Сохранённый контроль 0.38 и существующий runbook уже фиксировали отсутствие таких receipts и parsed-body семантику. Нельзя придумывать новые receipts или переписывать старое тело ради прохождения неверного сравнения. Исходные FAIL и диагностические строки не удаляются.

## Неисполненное и внешние условия

На момент release оставшийся Cargo был RESOURCE_PAUSED: native Data 9,930,628 KiB при пороге 10,485,760 KiB. Последующая разрешённая пользователем очистка target — отдельное событие и не превращает прежний NOT_RUN в PASS. Реальный успешный внешний provider case, инструментальное Rust coverage >=80% и формальные G1/G2/G3 не закрыты. Нет разрешения установки LLVM, обхода правил, активации или переключения настоящих проектов.

## Проверяемые источники

Все пути относительно .local/constellation-verification/compatibility-20261025/:

- quality-r3-finish-sole/final-case-map.json — case → receipt → status → limitations.
- quality-r3-finish-sole/09-mcp-migrated-process-workspace-default.{json,stdout.log,stderr.log} — четыре реальные ошибки публикации.
- quality-r3-finish-sole/10-check-semantic-all-targets.{json,stderr.log} — фактический E0106.
- quality-r3-finish-sole/production-semantic-provenance.json и model-cache-hashes.json — бинарники, features, source/model SHA.
- quality-r3-finish-sole/runtime-cases.json, runtime/tool-cases.json, measurement-validity.json и execution-adjudications.json — вызовы и сохранённые неверные измерители, а не выдуманные дефекты.
- quality-r3-finish-sole/EVIDENCE-RFC-018-body.md и EVIDENCE-RFC-018-payload.json — полный исходный отчёт исполнителя, не отредактирован интегратором.
- quality-r3-finish-sole/SAFELEASE-RELEASED.json — передача владения.
- corrected75-sole/legacy-same-fixture-comparison.json и docs/operations/CONSTELLATION-PILOT.ru.md — ранее проверенная legacy body/restore семантика.
- https://github.com/ForgePlan/forgeplan/issues/538#issuecomment-6401260584 — актуальное состояние и история.
- https://github.com/ForgePlan/forgeplan/issues/542#issuecomment-6367587222 — отдельные формальные условия.

## Условие возврата

Адресно исправить доказанные причины, заморозить новый source pin и выполнить затронутые проверки плюс реальные сценарии. Сохранить R3 FAIL как историю. Финальное решение о готовности требует полного acceptance и независимого Sol 6.1 review/gate; эта запись его не выдаёт.
