# ForgePlan vNext — Engineering Contract Layer

Этот каталог хранит design-and-execution proposal программы ForgePlan vNext, импортированный 2026-08-01. Это не описание выпущенного API и не замена действующим ADR.

## Статус и границы чтения — 2026-10-23

- `architecture/`, issue bodies и schemas описывают целевой дизайн. Формы настоящего времени в них задают желаемый результат, а не подтверждают реализацию. Связывающие решения принимаются в `.forgeplan/adrs/`; исторические якоря разбора — `EPIC-009`, `PROB-082`, `PROB-083`, `EVID-149`.
- Для выбранной сборки `0.39.0+constellation.s1.1` (`c852f40c`) `forgeplan contract …` и Agent API v2 не являются доступными командами/набором инструментов. Совпадающие имена, например `forgeplan_claim`, не меняют существующую семантику: claim остаётся advisory-лизой работы, а не удостоверением полномочий.
- Действующий MCP сохраняет полный профиль с мутациями и отдельный `constellation-read`; предложенные role profiles, WorkContract, ExecutionReceipt и VerificationVerdict не следует подставлять вместо действующих CLI/MCP-контрактов. Текущий git-provenance gate уже существует (`crates/forgeplan-core/src/scoring/provenance.rs`), но это не реализация всей FPV-05.
- Предложенный `R_eff = min(required_claim_scores)` — модель vNext. Действующий рекурсивный score учитывает собственные EvidencePack и подходящие зависимости, исключая terminal-status packs; flat score в проекциях — отдельная поверхность. Wire-значение `unknown` из draft EvidenceBundle не является действующим scoring verdict.
- `_import/` и `_audit/` сохраняют хронологию импорта и последующих опровержений. Их команды, числа, состояния issues и ссылки на исходники относятся к описанным там ревизиям; они не являются сегодняшними инструкциями восстановления или повторно измеренными результатами.
- Поддержка стороннего host/orchestrator и его enforcement здесь — требования к будущему адаптеру. Без версии, первичного источника и conformance run это не доказательство доступных hooks, permissions, API или изоляции.

### Известные незакрытые пробелы design drafts

Пять JSON Schema — не завершённый Protocol v1. В частности, закрытые объекты (`additionalProperties: false`) пока не обеспечивают требуемый round-trip неизвестных optional fields; ExecutionReceipt не допускает описанный top-level `digest` и полную ActorIdentity; EvidenceBundle допускает пустые criteria/provenance и не переносит весь контекст действующего скоринга. AuthorityPolicy, ExternalReference и другие объявленные типы не представлены отдельными схемами. Канонизация/digest, доверенная идентичность, persistence и связи criterion→Evidence требуют отдельных решений и fixtures. Текстовые требования остаются требованиями: эта оговорка не ослабляет ни их, ни действующие security/gate policies.

## Цель

Довести ForgePlan до зрелого repository-native слоя инженерных контрактов, authority и проверки результатов для AI coding agents, не превращая Core в task tracker, agent runtime, worktree manager или scheduler.

## Навигация

- `architecture/` — продуктовая граница, целевая архитектура и планы Core/Web/Extensions.
- `protocol/schemas/` — design drafts ForgePlan Protocol v1.
- `governance/` — правила исполнения, порядок зависимостей и требования к issues/PR.
- `prompts/` — готовые промпты coordinator, builder и verifier.
- `issues/` — issue manifest, bodies и отдельные читаемые задачи FPV-00…FPV-15.
- `scripts/` — валидация пакета, создание GitHub Issues и выбор следующей задачи.

## Предлагаемая граница

В целевом дизайне ForgePlan владеет engineering intent, graph, WorkContract, authority, Evidence, verification verdict и lifecycle. Внешние системы владеют backlog, workspace, worktree, session, agent process, scheduling, CI runner и deployment. Это предложение не переносит ownership и не отменяет существующие реализации или ADR.

## Исторические команды работы с пакетом

```bash
python3 docs/vnext/engineering-contract-layer/scripts/validate_pack.py
python3 docs/vnext/engineering-contract-layer/scripts/create_github_issues.py
python3 docs/vnext/engineering-contract-layer/scripts/next_issue.py
```

Это команды из import flow, а не подтверждённый безопасный bootstrap текущего checkout. Аудит в `_audit/README.md` и `issues/DRY-RUN-REVIEW.md` фиксирует блокеры качества и создания зависимостей. Не запускать issue creation только потому, что JSON парсится: сначала согласовать актуальные зависимости, provenance и разрешение на внешние записи.

Перед реализацией прочитать:

1. `architecture/01-PRODUCT-BOUNDARY.md`
2. `architecture/02-TARGET-ARCHITECTURE.md`
3. `governance/EXECUTION-ORDER.md`
4. `governance/AGENTS-VNEXT.md`

По исходному плану первая implementation issue — `FPV-01`; это порядок, а не свидетельство её текущей готовности. Master issue `FPV-00` координирует программу, но не является самостоятельным implementation PR.
