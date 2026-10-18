---
depth: deep
id: NOTE-053
kind: note
last_modified_at: 2026-10-16T20:44:18.859973+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: EPIC-010
  relation: informs
status: draft
title: Constellation — подготовка, передача MiniMax и отложенные Rust-проверки
---

---
assigned_number: 53
created: 2026-10-16
predicted_number: 53
slug: note-constellation-minimax-rust
updated: 2026-10-16
---

# NOTE-053: Constellation — подготовка, передача MiniMax и отложенные Rust-проверки

| Field | Value |
|-------|-------|
| Status | draft |
| Created | 2026-10-16 |
| Valid Until | 2027-01-16 |
| Context | constellation / handoff / verification-policy |

## Note

Этот прогон — **подготовка**, а не реализация. Зарегистрированы программа и 29 задач в Orchestra (проект Development), созданы содержательные draft-артефакты, обновлён пакет передачи MiniMax. Ни Rust-код, ни cargo, ни модельный запуск, ни runtime-smoke не выполнялись. Выбранная пользователем политика: **крупная история пишется целиком без промежуточных Rust-запусков; проверка — один последовательный цикл после всей истории.**

## Источник и проверенные основания

- **Программа-источник**: `ForgePlan-Constellation-Architecture-2026-10-16.docx`, SHA-256 `1c3c92fd6f1875f1a9be434d0e18f93c30c33c1ba08c9c851cda6d6f228551ec`. Это предложение, не выпущенное поведение.
- **Рабочая копия**: `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, ветка `feat/constellation-p0-contract`, база `6d121779c507c67d61131a51e7918c16f92d8879`. Исходное дерево остаётся на `docs/freshness-sweep` (`db96045c0fac5188f4b60d08c3e447f53c1ba0dd`).
- **MiniMax-M3** получил прежний пакет в REVIEW_ONLY без инструментов; это не проверка реализации и не BGE-M3.

## Выбранные границы проверки (политика)

| Событие | Rust / CLI / MCP runtime |
|---------|--------------------------|
| этот прогон (подготовка) | **NOT RUN** |
| S1-C (P1–P5) | **NOT RUN** — пишется без запусков |
| S1-V (после S1-C) | первый разрешённый цикл |
| S2-V / S3-V / S4-V | аналогично, после своей истории |

Rust checks у задач отражает эту политику: `NOT_RUN` у историй/*-C/*-V/*-G, `NOT_APPLICABLE` у организационных задач.

## Четыре крупные истории

- **S1** — 0А/0Б: локальная актуальная проекция и безопасные embeddings (P1–P5).
- **S2** — 1–5: федеративное чтение, версии, native read MCP (P6–P8).
- **S3** — 6: ContextPack и точные основания (P9).
- **S4** — 7–9: impact, измеренный cache, адаптеры (P10–P11).

Каждая история имеет шесть задач: история + A (закрепить контракт) + C (написать) + R (ревью исходников) + V (верификация) + G (гейт).

## Правила запуска и проверки

1. Один source writer / один worktree на историю.
2. P-шаги пишутся подряд; cargo/rustc/rustup/fmt и автоматические LSP builds запрещены до готовности всей истории.
3. Проверка — один последовательный цикл (S1-V/S2-V/S3-V/S4-V), ресурсы: один build owner, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, тесты --test-threads=1, ~10 GiB порог.
4. Ноль test binaries = RUN DID NOT HAPPEN.
5. Дефект Status/completed_at платформы не отменяет предметный гейт и не является поводом ставить ложный Done. Зависимость C→R/V снимается по проверенному code-freeze, а не по Status=Done.

## Карта артефактов программы

| Ключ | Артефакт | Путь (`.forgeplan/…`) |
|------|----------|------------------------|
| E | EPIC-010 | epics/EPIC-010-constellation-contextpack.md |
| S | SPEC-008 | specs/SPEC-008-constellation-0-0-embeddings.md |
| R | RFC-015 | rfcs/RFC-015-constellation-0-0.md |
| D | ADR-028 | adrs/ADR-028-constellation-rust.md |
| J | EVID-173 | evidence/EVID-173-constellation.md |
| N | NOTE-053 | notes/NOTE-053-constellation-minimax-rust.md |
| D_GENERATION | ADR-029 | adrs/ADR-029-constellation-manifest.md |
| J_S1A | EVID-174 | evidence/EVID-174-constellation-s1-a-review-adi-auth.md |

## Реальные Orchestra UID (ключи → UID)

Проверено прямыми чтениями 2026-10-16. Проект **Development**
`ZB8WYUpwNskvAZdRK8Jh`, пространство ForgePlan `kgYnhWodYw7kNbJMAK5q`,
Forge Agent `7eea6f7ad3fc4b3196385e9d3e3cb856`. Ровно 29 задач, дублей нет.

| Ключ | Task UID | Родитель | Blocked by |
|---|---|---|---|
| ROOT | `fdXN7kQgRQDqUnJRBRYu` | — | — |
| REG | `qao3WAXCYuIWqWJ7wYur` | ROOT | — |
| ALIGN | `pjmdnsmAX1gjDYQw1Fre` | ROOT | — |
| PREP-REVIEW | `UmydEvbid3fV7iXoISOr` | ROOT | — |
| EVENT | `0ZrqkBRXYjdXV7deNZPz` | ROOT | — (триггер в описании) |
| S1 | `QuiaGibPPEFs7XACszz2` | ROOT | — |
| S1-A | `it3ctaOPGmxCIXj4jgGv` | S1 | — |
| S1-C | `CAVAw0E2DCftle0W1A7m` | S1 | S1-A |
| S1-R | `HdKRuquoNq6yjDNaHyqW` | S1 | S1-C |
| S1-V | `beVXuCp4h65YYshXbIAX` | S1 | S1-C |
| S1-G | `dyLEeCfPNy74TwNgIDZQ` | S1 | S1-R, S1-V |
| S2 | `6DdOWEziOwxPlcuybDKO` | ROOT | S1-G |
| S2-A | `YrUtgO1HZtx5b0L7M1L1` | S2 | S1-G |
| S2-C | `jRGdRIjrvxEWp46CovVr` | S2 | S2-A |
| S2-R | `nk0AU9SlY05ThtCh93Fl` | S2 | S2-C |
| S2-V | `sUh2WeFv9PeZBdjVkp96` | S2 | S2-C |
| S2-G | `KFkkXxYN3s9JIPIMrK9S` | S2 | S2-R, S2-V |
| S3 | `3cTN4L8OfMry7lPRnzEb` | ROOT | S2-G |
| S3-A | `Ndc66YS0jJOITFHsrUWj` | S3 | S2-G |
| S3-C | `9FZq7utTrE09LvjWirtz` | S3 | S3-A |
| S3-R | `25TXyvvncFgVIkzk49fd` | S3 | S3-C |
| S3-V | `BjAP5w9NexOuJ5IeUkiI` | S3 | S3-C |
| S3-G | `x0f9LCer6r6CSf5GLABy` | S3 | S3-R, S3-V |
| S4 | `J5iwbG72Pndqp0dBCX0l` | ROOT | S3-G |
| S4-A | `FV9eiPSIdpOdZ64QluSU` | S4 | S3-G |
| S4-C | `YkTt7oxjEEEzbHVoaFC0` | S4 | S4-A |
| S4-R | `f7LTMkh1bsnfMWQnVLAo` | S4 | S4-C |
| S4-V | `oh586ALMQOoKOMQOXGZf` | S4 | S4-C |
| S4-G | `sDSN61I3Av401G0CSKmi` | S4 | S4-R, S4-V |

**Актуальный итог продолжения:** статический контракт S1-A принят координатором после реальных Codex ADI4+4, независимого technical fit POSITIVE с0 открытых замечаний и финального validate4/4 безMUST/warnings. S1-A Review/Evidence, checklist10/10. Формальный CONCERNS reviewer сохранён как agent-level zero-findings convention, не переименован в PASS. EVID-175 содержит реальные результаты, полный независимый report и честную содержательную delta после ADI; EVID-174 сохраняет исторические отказы. Только два LLM-поля prepared-worktree изменены на codex-cli/model=""; основной config/credentials не тронуты. Claude login не требуется. S1-C архитектурно разблокирована, но source/MiniMax запуск требует отдельного разрешения; Rust/probe/runtime/activation/commit не выполнялись.

Техническая связка передачи (поля, опции, все UID, результаты read-back) —
`/Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/ORCHESTRA-MAP.json`.
Аудит подготовки — **EVID-173**: авторские наблюдения, исходные независимые результаты и итоговый PrepGuardianGate PASS сохранены раздельно. Полный подписанный вердикт и хеши источников — `audit/review-guardian.json` и EVID-173. Таблица выше и Blocked by карты показывают текущие незакрытые условия. Прежние зависимости сохранены отдельно в planned_blocked_by карты и audit-срезах; выполненные условия не остаются живыми блокерами.

## Подготовка и код разделены

Наличие задач и draft-артефактов **не** является разрешением кодировать. Архитектурные условия старта первой истории остаются явными задачами: S1-A должен закрыть generation/owner контракт до того, как S1-C станет startable. Ни один runtime-сценарий в этом прогоне не выполнялся.

## Фактический ADI — условие до кода

2026-10-16 выполнен настоящий `forgeplan_reason` с explicit workspace подготовки. Настроенный провайдер `claude-code` отказал: `claude --print` завершился с code=1; инструмент требует `claude login`. Полный ответ сохранён в `/Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/adi-results.json`. Аутентификация и конфигурация провайдера не менялись. Это **не положительный ADI**: три варианта ниже — авторский проект сравнения. По утверждённому исключению подготовку можно принять, но S1-A обязан получить и сохранить действительный ADI перед архитектурным принятием и допуском S1-C. Rust, storage probe и модельный запуск MiniMax не выполнялись.

## Докодовый допуск и активация — разные решения

Текущий маршрут — Row 7; автоматический SPARC-переход не выбран. В этой цепочке независимый положительный архитектурный вердикт по валидным draft SPEC/RFC/ADR, успешный ADI и отдельный generation/owner ADR являются докодовым условием S1-C, **не** переводом артефакта в active. Действующий CLAUDE.md не меняется: Validate → Code → Evidence → Activate; активация без кода и связанного Evidence запрещена. Новые исходники допускаются только отдельным разрешением координатора после S1-A. Если позднее выбирается SPARC, сначала нужен отдельный согласованный маршрут и выполненные active-parent/Architecture-RFC предпосылки; эта передача не разрешает молча обойти ни их, ни правила репозитория.

## Related

| Artifact | Relation |
|----------|----------|
| EPIC-010 | informs |

Контекстные ссылки без дополнительных structural edges: SPEC-008, RFC-015 и ADR-028.






