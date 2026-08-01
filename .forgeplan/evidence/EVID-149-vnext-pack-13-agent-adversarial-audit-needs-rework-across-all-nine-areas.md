---
depth: standard
id: EVID-149
kind: evidence
last_modified_at: 2026-08-01T18:52:43.094953+00:00
last_modified_by: claude-code/2.1.220
links:
- target: EPIC-009
  relation: informs
- target: PROB-082
  relation: informs
- target: PROB-083
  relation: informs
status: draft
title: 'vNext pack 13-agent adversarial audit: NEEDS_REWORK across all nine areas'
---

# vNext pack 13-agent adversarial audit: NEEDS_REWORK across all nine areas

Адверсариальный аудит пакета `docs/vnext/engineering-contract-layer/` (2026-08-01).

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: audit

CL3 — ревьюеры прогоняли команды против **этого** workspace, а не рассуждали
по описанию. `weakens` — аудит ослабляет утверждение, что пакет готов к исполнению.

## Method

13 агентов в изолированных контекстах, read-only:

- 4 ревьюера материала — architecture (18 док), issues (16 тел + манифест),
  protocol (5 JSON-схем), governance/prompts против `CLAUDE.md`
- 5 ревьюеров реальности — коллизии с 18 ADR, дубли с 65 PRD / 12 RFC / 8 EPIC,
  верификация фактических утверждений против кода, пересечение с 32 открытыми
  GitHub issue, стресс-тест продуктовой границы против шипнутого v0.33.0
- план размещения + инвентарь потерь
- синтез + адверсариальный критик синтеза (искал, что синтез потерял или смягчил)

Расход: ~2.3M токенов, 500 tool-calls, 41 минута.

## Result

**130 находок: 27 BLOCKER, 66 MAJOR.** Вердикт `NEEDS_REWORK` по всем девяти
областям без исключения.

## Key findings

1. **Ноль ссылок на существующие решения.** `grep -rnoE 'ADR-[0-9]{3}' docs/vnext/`
   → 0 при четырёх задетых активных ADR. См. PROB-082.
2. **Пакет не выполняет собственный контракт.** `governance/ISSUE-GOVERNANCE.md`
   требует восемь секций; соответствие **0/16**. Секции `Product boundary`,
   `Rollback/migration` и `Dependencies` в телах — 0/16.
   При этом `validate_pack.py` печатает `OK`, потому что проверяет только
   существование файлов и парсабельность JSON.
3. **Несущий механизм ничего не проверяет.** Программа заявляет проверку git-дельты
   вместо доверия к утверждению, но `evidence-bundle.schema.json` валидирует bundle
   с пустыми `base_sha`/`result_sha`, пустым `changed_paths` и нулём criteria —
   0 ошибок. У SHA нет `pattern`/`minLength`, у `criteria` нет `minItems`.
4. **Ложные предпосылки.** FPV-01 утверждает, что ForgePlan описан как
   «project-management layer» — `docs/methodology/FORGEPLAN-GUIDE.md` говорит
   «**Not Jira.** Not project management. Not a task tracker.». FPV-08 требует
   починить `@file` asymmetry — issue #350 закрыт в v0.33.0. FPV-08 требует
   `forgeplan decay` — команда шипнута. `AGENTS-VNEXT.md` предписывает
   `forgeplan context` без обязательного позиционного `<ID>`.
5. **Границу нарушает шипнутый код, и не тот, который пакет пометил.** FPV-01
   указывает на `forgeplan_dispatch` (pure-read, не мутирует). Настоящий шедулер —
   `playbook::dispatch` с `Delegation::Command`, `budget_usd`, `timeout_seconds` —
   в пакете не упомянут ни разу.
6. **`gh` 2.83.2 не поддерживает `--parent` и `--add-blocked-by`.** Скрипт вызывает
   оба с `check=False`, поэтому не падает, а печатает WARN. Итог: 16 публичных issue
   создались бы **плоским списком** без epic-родителя и без единого ребра
   зависимостей, и доисправить это тем же тулчейном нельзя.
7. **Спроса не обнаружено.** В корпусе из 77 PROB нет ни одной заявки на
   кросс-хостовую переносимость контрактов. Свежая реальная боль — PROB-072
   (worktree drift), PROB-073 (латентность), PROB-077 (silent data loss) — вся
   локальная и репозиционированием не лечится.

## What survives the audit

Ядро измеримо и обосновано: 148 EvidencePack, из них **0** с `base_sha`/`result_sha`
(`grep -rl 'base_sha\|result_sha' .forgeplan/evidence/` → 0). Слайс git-delta
provenance gate (GitHub #360) не зависит ни от Protocol v1, ни от адаптеров, ни от
сервера и отгружаем обычным циклом против текущей границы.

## Verdict split

Доказательства поддерживают два разных вердикта двум отделимым половинам:

- **FPV-05 + корректностная часть FPV-06** — разрыв измерен, репродьюсеры живые,
  внешних зависимостей нет. Годно к отгрузке.
- **FPV-09…FPV-15** — ноль заявок, четыре необъявленных supersession, ноль ссылок
  на возможности третьих сторон (`grep -rn 'http' architecture/` → 0 на 1171 строку
  при требовании version matrix). Нужна не переработка текста, а основание
  существовать.

## Reproduction

Полные отчёты: `brief` (42K) и `critique` (28K) в выводе workflow `wcjuhy9ja`,
журнал по агентам — `journal.jsonl` в каталоге транскрипта.

## Related

- PROB-082 — реестр коллизий с активными ADR (гейт FPV-01)
- PROB-083 — дефекты субстрата, обнаруженные по ходу
- EPIC-009 — якорь программы



