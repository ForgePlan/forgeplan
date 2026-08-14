---
depth: standard
id: ADR-020
kind: adr
links:
- target: ADR-002
  relation: refines
status: draft
title: R_eff excludes terminal-status evidence from the weakest-link min
---

## Context

`R_eff = min(evidence_scores)` («weakest link, никогда не среднее») сегодня включает в min() эвиденцию с терминальным статусом. Вытесненный (`superseded`) refutes-пак навсегда пиннит R_eff артефакта к 0 — восстановление невозможно даже после честной ре-верификации на более поздних коммитах (PROB-084; upstream #436 / PROB-102: цепочка PRD-177 → EVID-249 refutes superseded + EVID-250/251 supports → R_eff 0.00).

Ресёрч по четырём пулам (доки репо, граф артефактов + первоисточники, forgeplan.dev, код/тесты/CHANGELOG) установил:

1. **Нигде не задокументировано как намеренное.** Все три места сбора прямой эвиденции (`reff.rs:254`, `score.rs:154`, `server.rs:3062`) передают `ArtifactFilter { status: None }` без комментария; ни теста, ни CHANGELOG-записи, ни артефакта, пиннящего поведение. `EvidenceItem` даже не несёт `status`.
2. **Обратное уже обещано минимум пятью поверхностями:** `reff.rs:313` («draft/deprecated/superseded should not drag down R_eff» — реализовано только для зависимостей), ADR-002 (active) + коммит `a76c105a` («no longer drag down the weakest link score»), METHODOLOGY-COURSE Ch8 («Deprecated/Superseded — skipped (closed)»), сайт (cli/score: «…or replace it»; blog averages-lie: worked example с восстановлением после исключения устаревшего источника), health-подсказка дедупа эвиденции (`deprecate EVID-x --reason "superseded by EVID-y"`).
3. **Первоисточники единогласны:** quint-code (первоисточник по ADR-005) фильтрует `Verdict != "superseded"` из WLNK min() (`decision.go:818`, «FPF F.10:6.1 — superseded within same Window»), авто-вытесняет старые измерения при появлении новых (`decision.go:703`) и деградирует all-superseded в «no active evidence», а не в старый score. FPF: «Refuted cancels positives **within the same Window**» (F.10:6.1/8) — вето старого окна на новое окно не имеет основания в исчислении; Deprecate → «claim support is reduced or removed», Refresh — путь восстановления (A.2.4:8.2, B.3.4).

## Decision

**R_eff означает текущую надёжность. Эвиденция с терминальным статусом (`superseded`, `deprecated`) исключается из weakest-link min(). История при этом сохраняется — пакет остаётся в графе со своим статусом и ребром `supersedes`.**

Конкретно:

1. `EvidenceItem` получает поле `status`; `parse_evidence_from_record` заполняет его из записи. Фильтр — в одной точке (`r_eff()` / `r_eff_with_ci()`), так что все потребители (движок, CLI/MCP score, health, decay, journal, gaps) получают согласованную семантику автоматически (choke-point вариант из пре-анализа #436).
2. **Draft-эвиденция ОСТАЁТСЯ в счёте** — осознанное отличие от буквального acceptance #436 («only active»). Draft — штатное рабочее состояние свежего измерения в Standard-flow (score-гейт идёт ДО активации: `new evidence → link → score → activate`); исключение draft обнулило бы почти каждый пре-активационный score и сломало бы гейт. Это же соответствует quint-code, который фильтрует только superseded. Асимметрия с dependency-путём (там draft пропускается) корректна: draft-*зависимость* — не начатая работа, draft-*эвиденция* — свежее показание.
3. **Активный refutes продолжает обнулять.** Исключаются только терминальные статусы; «one strong benchmark and one refuted test is still a risky PRD» (сайт) остаётся в силе. Инвариант min-never-average не тронут: меняется допуск к участию (eligibility), не агрегация.
4. Каждый пропуск логируется в `factors` («Skipped EVID-x (status: superseded)») — симметрично dependency-пути; CLI/MCP-выдача score продолжает ПОКАЗЫВАТЬ терминальные пакеты в списке с пометкой об исключении (прозрачность, не скрытие).
5. **All-terminal edge:** если вся линкованная эвиденция терминальна — артефакт деградирует к «no active evidence» (self_score 0.0), как в quint-code. Восстановление требует линкованного пакета-замены, а не просто флажка supersede.
6. **ADR-002 остаётся в силе без изменений** (dependency-путь); настоящий ADR распространяет тот же принцип на прямую эвиденцию (`refines` ADR-002). Это явный ответ на требование vNext-аудита M5.

## Anti-laundering bounds

- `supersede` требует `--by <successor>` — вытеснить без предъявления замены нельзя; пропуски видимы в factors.
- Полноценный «audited Evidence dismissal» (актор/причина/policy) — зона vNext FPV-06, здесь не реализуется.
- Window-дисциплина FPF: честное вытеснение опирается на измерение из более нового окна (поздний коммит). Механическая проверка этого — provenance-gate (PRD-082, `base_sha`/`result_sha`), уже в dev; связка «supersede требует более нового окна» — кандидат в follow-up.

## Consequences

- Затронутые артефакты не «поднимутся» сами: 4 потребителя кэша `r_eff_score` (PROB-057 — get.rs, filter.rs, fgr.rs, reason.rs) отдают старое значение до пересчёта. После апгрейда нужен `forgeplan score` по затронутым артефактам (или score-all).
- Upstream #436 закрывается этим изменением; расхождение с его acceptance по draft задокументировано выше.
- Доки обновляются синхронно: CLAUDE.md §Key formulas, QUALITY-GATES, METHODOLOGY-COURSE Ch5/Ch8, GLOSSARY, HOW-TO-USE, FORGEPLAN-GUIDE, EVIDENCE-PROTOCOL (EN+RU), CHANGELOG; сайт (score/supersede/evidence + ru) — отдельным PR.

## Invariants

- min-never-average — не нарушен (меняется только население min()).
- Активная refutes-эвиденция обнуляет score — не ослаблено.
- Терминальный пакет никогда не удаляется и остаётся видимым в графе и в score-выдаче (с пометкой) — «supersede, do not delete».
- Пропуск любой эвиденции из min() всегда оставляет след в factors.

## Rollback Plan

Изменение чисто вычислительное, формат данных не меняется (новое поле `EvidenceItem.status` — внутренняя структура, не диск). Откат = revert коммита + `forgeplan score` по затронутым артефактам. Кэшированные значения пересчитываются в обе стороны одной командой.

## Affected Files

- `crates/forgeplan-core/src/scoring/reff.rs` — `EvidenceItem.status`, фильтр в `r_eff`/`r_eff_with_ci`, лог пропусков в self-score блоке, doc-comment формулы.
- `crates/forgeplan-core/src/scoring/evidence.rs` — `parse_evidence_from_record` заполняет status.
- `crates/forgeplan-core/src/scoring/decay.rs` — конструктор EvidenceItem.
- `crates/forgeplan-cli/src/commands/score.rs`, `crates/forgeplan-mcp/src/server.rs` — пометка исключённых пакетов в выдаче; описание tool.
- Тесты: юнит (reff), e2e CLI, e2e MCP.

## Alternatives considered

- **Ронять CL у вытесненной эвиденции** — отвергнуто: CL меряет конгруэнтность контекста, она честно CL3; занижение — ложь по другой оси.
- **Отдельная метрика «исторический минимум»** — отвергнуто: история уже сохранена графом (пакет + статус + ребро supersedes); вторая метрика — оверинжиниринг.
- **Новый relation `resolves`** — отвергнуто: `supersedes` уже значит «заменяет»; чинить надо соблюдение семантики, не плодить словарь.
- **Фильтровать и draft (буквальный acceptance #436)** — отвергнуто: ломает score-гейт Standard-flow (см. Decision §2).



