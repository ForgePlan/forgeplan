---
depth: tactical
id: PROB-060
kind: problem
status: draft
title: Distributed artifact ID assignment — collisions in parallel branches and multi-agent dispatch
---

---
id: PROB-060
title: "Distributed artifact ID assignment — collisions in parallel branches and multi-agent dispatch"
status: Draft
created: 2026-05-06
depth: tactical / standard / deep
context: "{grouping tag}"
parent_epic: EPIC-060
---

# PROB-060: Distributed artifact ID assignment — collisions in parallel branches and multi-agent dispatch

## Signal

Counter-based assignment в `forgeplan new <kind>` (читает `max(<kind>-*) + 1`) даёт коллизии при параллельной работе:

- **Parallel feature branches**: 2+ разработчика одновременно делают `forgeplan new prd` на разных ветках от общего origin/dev → оба получают одинаковый ID (например, PRD-074). На merge — git add/add conflict на filename, и хуже — семантическая коллизия: refs в коммитах (`Refs: PRD-074`) и теле артефактов указывают неоднозначно.
- **Multi-agent dispatch (PRD-057)**: 3-5 AI-агентов параллельно через `forgeplan_dispatch` создают артефакты быстрее чем counter синхронизируется. На 3+ агентах race window становится почти 100%-вероятным.
- **Commit message immutability**: даже если filename переименовать на reconciliation, refs в commit history остаются на старый ID. Это price of git, обойти нельзя.
- **Cross-PR refs rot**: артефакт A в теле ссылается `Related: PRD-075` (ожидая что коллега возьмёт следующий номер), но коллега получает PRD-076 после reconciliation → broken ref.

Текущая инфраструктура **никак не детектирует и не предупреждает** о потенциальной коллизии при `forgeplan new` — counter инкрементируется локально без сверки с origin/dev.

## Constraints

Hard constraints из CLAUDE.md и архитектуры — нарушать нельзя:

- **Local-first** (Non-Goals в CLAUDE.md): нельзя ввести central ID reservation server, network call в `forgeplan new`, или обязательную координацию через сетевой сервис.
- **ADR-003 Markdown = source of truth**: ID живёт в filename, frontmatter, refs в теле других артефактов. Любая схема должна работать на git+markdown без внешних индексов как primary source.
- **73+ существующих PRD, 8 RFC, 11 ADR, 55 PROB, 112 Evidence**: нельзя перенумеровать legacy. Решение должно быть **forward-only с cutoff point**.
- **Commit history immutable**: refs в существующих коммитах не переписываются никогда. Решение должно либо избегать refs в коммитах до merge, либо принять что часть legacy refs может быть ambiguous.
- **AI-agent compatibility**: ID должны быть стабильными в ADI prompts, search results, MCP responses. Шаткие IDs ломают ADI reasoning (галлюцинации связей которых нет).
- **Multi-agent dispatch уже в production** (PRD-057, v0.24.0): решение должно быть совместимо с 5+ параллельными агентами by construction, не через retry/coordination.

## Optimization Targets (1-3 макс)

1. **Zero filesystem-level collisions** при N параллельных авторов/агентов (target: N=10 без ручной координации)
2. **Zero semantic ref rot** между активированными артефактами (после merge все refs резолвятся в правильный артефакт)
3. **Preserve readable ID mental model** — пользователь должен уметь сослаться на «PRD-074» в Slack/PR без специальной нотации

## Observation Indicators (Anti-Goodhart)

Мониторим, но НЕ оптимизируем (иначе reward hacking):

- **Количество reconciliation-операций в неделю** — не оптимизировать «к нулю» через подавление обнаружения. Высокая частота = здоровая сигнализация что система работает.
- **Latency `forgeplan new` команды** — соблазн «оптимизировать» через убирание pre-commit checks. Не оптимизировать, конечная UX-target — корректность, не скорость.
- **Количество refs в коммитах** — соблазн занижать через политику «не пишите Refs: в коммитах». Refs в коммитах = ценная traceability, цена не та чтобы её жертвовать.

## Acceptance Criteria

Измеримое определение «решено»:

1. **5 параллельных AI-агентов** через `forgeplan_dispatch` создают по 3 артефакта каждый → 15 артефактов, 0 filesystem collisions, 0 semantic ref breakage после sequential merge всех PR.
2. **2 параллельные human-feature-branches** от общего origin/dev создают PRD; обе merge'атся в dev (любой порядок) → оба артефакта существуют с уникальными IDs, refs в теле каждого корректно резолвятся.
3. **Migration of existing 298 artifacts**: после rollout новой схемы все legacy IDs остаются стабильными, refs между ними не трогаются. CI gate блокирует rename legacy artifacts.
4. **Tooling coverage**: `forgeplan new`, `forgeplan list`, `forgeplan search`, `forgeplan score`, MCP tools — все работают с новой схемой без regression на legacy.
5. **Documentation**: workflow в CLAUDE.md обновлён; есть `docs/methodology/ID-ASSIGNMENT.ru.md` с runbook для contributors и для AI-агентов.

## Blast Radius

Затронутые системы и компоненты:

- **CLI**: 76 commands. Особенно `new`, `list`, `search`, `link`, `supersede`, `deprecate`, `route`, `validate`, `score`, `activate`.
- **MCP**: 63 tools. Все которые принимают/возвращают artifact IDs.
- **Storage layer**: `forgeplan-core/src/projection/` — sync_file_to_store, render_projection. Filename = primary key в LanceDB index.
- **Validation**: regex для ID parsing в `validation/`. Сейчас `^(PRD|RFC|ADR|EPIC|SPEC|EVID|PROB|SOL|NOTE|REF)-\d+$`. Расширение схемы = breaking для всех regex.
- **Search/embed**: BGE-M3 индекс в `.forgeplan/lance/`. Если IDs меняются на reconciliation — нужен reindex.
- **External**: commit messages, PR descriptions, GitHub issues references — за пределами форгеплана, не починить retroactively.
- **Hooks**: `.claude/hooks/forge-safety-hook.sh`, `pre-commit-fmt.sh`, `commit-test-check.sh` — могут потребовать обновления regex.
- **CI**: `scripts/check-mcp-tool-count.sh` и drift detector — потенциально нужен новый ID-collision detector.
- **Documentation**: CLAUDE.md (workflow), `docs/operations/GIT-WORKFLOW.ru.md`, `docs/methodology/UNIFIED-WORKFLOW.ru.md`, ADR-003.

## Reversibility

**Medium**. Откат возможен но болезненный:

- Если новая схема rolled out + начали создаваться артефакты по новой схеме → откат требует либо reverse migration (rewrite filenames + refs) либо принятия гибридного состояния «legacy + new + rolled-back-new». Сложно но возможно.
- В отличие от ADR-003 (full file-first invariant — практически нереверсируемый из-за compile-time enforcement), ID schema можно поменять с новым cutoff point без compile-time контрактов. То есть «изменили схему ID» можно, «убрали схему ID полностью» — нельзя.

Mitigation плана отката: feature flag в `.forgeplan/config.yaml` (`id_assignment: legacy | hybrid | new`) на rollout phase, чтобы можно было быстро вернуться без code revert.

---

## Considered Alternatives (preview, full analysis в RFC)

Полный анализ — в RFC после route. Здесь только зафиксировать что рассматривалось:

| Подход | Решает filesystem | Решает ref rot | Решает AI confusion | Local-first | Verdict |
|---|:-:|:-:|:-:|:-:|---|
| **Counter + remote check** (current + warning) | partial | ❌ | ❌ | ✅ | weak — race window остаётся |
| **Suffix on collision** (PRD-074-a/b) | ✅ | ❌ | ❌ (worse — ложная семантическая связь) | ✅ | rejected — суррогат, не решение |
| **Lazy assignment** (slug → number at merge, Rust RFC model) | ✅ | ✅ | ✅ | ✅ | strong candidate |
| **ULID under hood + display number** (Gerrit model) | ✅ | ✅ | partial | ✅ | strong candidate, тяжёлый tooling |
| **Drop numbers, use slugs only** (changesets model) | ✅ | ✅ | ✅ | ✅ | ломает ментальную модель PRD-074 |
| **Central ID server** (Linear/Phabricator model) | ✅ | ✅ | ✅ | ❌ | rejected — нарушает local-first |

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| | based_on / informs |




