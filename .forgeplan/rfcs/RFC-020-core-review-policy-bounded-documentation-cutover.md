---
depth: deep
id: RFC-020
kind: rfc
last_modified_at: 2026-10-16T20:48:53.101550+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-025
  relation: based_on
- target: ADR-003
  relation: based_on
status: active
title: Core review policy bounded documentation cutover
---

---
assigned_number: 14
created: 2026-10-16
prd: null
predicted_number: 14
slug: rfc-core-review-policy-bounded-documentation-cutover
updated: 2026-10-16
---

# RFC-020: Ограниченное применение core review policy в документации

| Поле | Значение |
|---|---|
| Автор | Eli Rum, https://elirum.me |
| Дата | 2026-10-16 |
| Исполнитель проекта | adr-architect + architecture, Profile A; назначенный OMP Sol6.1 High, run-local overlay |
| Status | DRAFT; source не применён, ни один будущий gate не объявлен PASS |
| Depth | Deep, smith row 7 adr-decision; owning route передан родительской сессией |
| Решение | ADR-027, slug adr-finite-evidence-backed-independent-review-without-a-defect-quota |
| Baseline | docs/freshness-sweep; HEAD db96045c0fac5188f4b60d08c3e447f53c1ba0dd; ниже hashes фактических bytes |
| Период проверки актуальности | до 2027-01-14 либо немедленно при триггерах ADR-027; metadata valid_until этим не объявлен установленным |

## Summary

Применить разрешённый владельцем 2026-10-16 принцип: независимый review MAY завершиться конечным PASS без квоты на дефекты, когда текущий проверенный материал, полнота и достаточные доказательства это позволяют. Заменить только quota/re-spawn утверждения в **17** существующих core файлах. Полный нормативный протокол размещается в существующем QUALITY-GATES §2 (EN/RU); остальные места дают краткое правило и ссылку. ADR-027 хранит выбор и его причины, этот RFC — контракт реализации. Нового нормативного документа, runtime компонента/API/модели данных нет.

## Motivation

Общая Deep/Critical норма docs/methodology/QUALITY-GATES.md:62–83 не допускает честного результата «проверено полностью, дефектов не найдено». CLAUDE.md:685 добавляет ≥3 issues каждому из двух code/security reviewer. EVID-173 §P1 фиксирует этот конфликт отдельно от качества website требований и внешнего D-PRODUCER. Владелец ответил «Да, оформить и в core»; полная цитата и закреплённое происхождение находятся в ADR-027 §Owner authorization and provenance. Новый выбор не переписывает исторические verdict и не делает ноль автоматическим PASS.

## Goals and Non-Goals

Цели: одно согласованное core правило; независимый проверяемый конец review; сохранение всех остальных gates; EN/RU parity; минимальный контролируемый diff; точная граница между design, source, опубликованным бинарём и runtime.

Не цели: website/public mirrors, Marketplace/Runtime/AgentBootstrap, installed/global skills/instructions/settings/auth; Rust/CLI/MCP validator/scoring/runtime; .github security/branch rules; массовая правка исторических .forgeplan bodies; producer acceptance; новые постоянные тесты текста; сборки, серверы, browser; commit/push/merge/release/deploy/tracker. Изменение `templates/prd/_TEMPLATE.md` не доказывает, что текущий установленный бинарь уже раздаёт новый embedded шаблон.

## Options Considered

1. **Только поправить QUALITY-GATES.** Меньше diff, но корневые инструкции, agent prompt, PRD schema и template продолжают требовать дефект. Отклонено из-за нескольких конкурирующих read paths.
2. **Ограниченный единый cutover 17 файлов. Выбрано.** Один полный протокол, короткие ссылки и одинаковая семантика на EN/RU; исторические snapshots сохраняются.
3. **Массовая замена всех похожих строк.** Захватит website, прошлые Evidence и audit результаты, создаст ложную историю и выйдет за разрешение. Отклонено.
4. **Новый runtime validator или отдельная policy система.** Нет запроса на механизм исполнения; текстовая квота решается изменением регламента. Отклонено как расширение объёма.

Полный формальный authored Abduction/Deduction/Induction четырёх вариантов самой политики и engine limitation сохранён в ADR-027. Эта ссылка не выдаётся за новый engine output и не закрывает Deep engine gate.

## Proposed Direction

### Active architecture and lifecycle prerequisites

Предложенное применимое основание: **ADR-025 active** (процесс и запуск агентов над core; core — договорённости и доказательства) плюс **ADR-003 active** (canonical human markdown через owning MCP/CLI, индекс производный). Применение меняет инструкции людей/агентов, не runtime. ADR-026 active рассматривается только как reference по происхождению; его новые JSON record схемы здесь не создаются.

Измеренные canonical body UTF-8 SHA-256 при чтении через owning MCP 2026-10-16:

| Parent/reference | status/depth | bytes | body SHA-256 |
|---|---|---:|---|
| ADR-003 | active/standard | 17857 | 8d034028af55e8ecaa1c2a5047322a6997c903f71c75dc78e2112d748a9ac763 |
| ADR-025 | active/standard | 4918 | 3e3ca3782908b1c81ddbedab0aa816dc54b702c8cd303171a5ded8088e6e7652 |
| ADR-026 | active/standard | 4734 | 8f1fa9fd6561e9a3bef44839d6e7d923c3b3c3c08e4b2e8c2f3c8da64eb6b5ad |

Это **авторское обоснование**, не разрешение исполнения. Fresh Sol High pre-source reviewer и затем fresh Sol High guardian обязаны прямо сертифицировать: (1) эти active решения действительно покрывают ограниченные docs-only правки; (2) новый active RFC не обязателен до них; (3) последовательность не обходит CLAUDE red line 7; (4) authored ADI достаточен либо назван недостающий engine prerequisite; (5) разрешение владельца позволяет оценить само изменение квоты без фабрикации finding, не отменяя прочие gates.

Новые ADR-027/RFC-020 остаются **draft** до реального source применения и независимых доказательств. Если gate считает новый active RFC обязательным до source, а red line 7 требует source до активации, source dispatch BLOCKED до узкого owning разъяснения. Если literal code означает только Rust, markdown не объявляется таким code; конфликт не снимается автором. Ни псевдокод, ни фиктивная заглушка, ни изменение activation policy не допускаются. Engine prerequisite ADR-027 остаётся отдельно открытым: документирован model env override, но supported High-effort control для configured codex-cli в прочитанных help/schema/docs не найден. Новый engine run/default/fallback не выполнен. Старый website reason не удовлетворяет новому gate автоматически.

### Normative read path and precedence

Будущий путь: `CLAUDE.md` §§Rust coding rules / Adversarial audit mandate → `docs/methodology/QUALITY-GATES.md#2-adversarial-review-protocol` (RU twin с тем же anchor) → набор критериев конкретного review → current candidate/evidence → независимый verdict в canonical EVID → отдельный guardian/lifecycle.

QUALITY-GATES §2 содержит полный протокол ADR-027 D1–D4: независимость, exact current baseline, критерий→проверка→результат→доказательство, контраргумент/края, findings/severity, ограничения, конечный verdict и re-review triggers. В самом §2 указать owning ADR и приоритет только по defect quotas над устаревшими quota-фразами, historical и template копиями. Draft не объявлять уже active; момент вступления в действие следует согласованной lifecycle последовательности. Core-only граница указана явно, без обещания синхронизации сайта или установленных пакетов.

Не менять Verification Gate пять пунктов, 13-step validation, R_eff формулу/пороги/CL/decay, прежние depth triggers, число/роли reviewer, Critical ≥2 rounds, согласие, профильные security/runtime проверки, activation требования. В severity таблице можно уточнить различие observation «Pass» и gate PASS; не переименовывать все severity и не превращать Warning в игнорируемый результат.

## Detailed Design — Source Contract

### OWNED for the future MiniMax writer only

Все строки и SHA ниже независимо прочитаны/измерены на указанном baseline. Numbers — ориентиры; surgical изменение делается по совпавшему тексту и актуальному hash, а не слепому номеру. `EN/RU` — два явных файла, не разрешение править всё дерево.

| Exact path | Anchors before change | Required bounded change |
|---|---|---|
| CLAUDE.md | :516; :679–685 | Сохранить significant trigger и 2+ audit, security/code роли. Удалить обязательный issue count/re-spawn при нуле; дать короткое условие и ссылку на QUALITY-GATES §2. Red lines/activation не трогать. |
| docs/methodology/QUALITY-GATES.md | §2 :56–96, особенно :62,:64,:83,:88 | Полный нормативный D1–D4 протокол; ре-review исправлений/недостаточных доказательств/изменений, не ради квоты. Critical ≥2 rounds сохранить. Другие разделы не менять. |
| docs/methodology/QUALITY-GATES.ru.md | §2 :56–96 | Семантически точный RU twin предыдущего изменения. |
| docs/methodology/HOW-TO-USE.md | :209,:437 | Обе краткие нормы без квоты, ссылка на QUALITY-GATES §2. |
| docs/methodology/HOW-TO-USE.ru.md | :209,:437 | То же по-русски. |
| docs/methodology/GLOSSARY.md | :109–110 | Определение adversarial review через поиск опровержений, достаточность и независимость; короткая ссылка, без нового полного протокола. |
| docs/methodology/GLOSSARY.ru.md | :109–110 | То же по-русски. |
| docs/methodology/FORGEPLAN-GUIDE.md | :580, пояснение рядом с code block :575–585 | В строке AUDIT убрать MUST find issues; рядом короткая ссылка на §2. Сам цикл и FIXES для реальных находок сохранить. |
| docs/methodology/FORGEPLAN-GUIDE.ru.md | :580, :575–585 | То же по-русски. |
| docs/methodology/METHODOLOGY-COURSE.md | :539; §11.8 :753–761; summary row :815 | AUDIT без квоты; anti-pattern — неподтверждённый approval, не сам ноль; убрать «>100 LOC обязательно есть что улучшить», «always finds something». В summary detection — нет покрытия/доказательств. Ссылка на §2. |
| docs/methodology/METHODOLOGY-COURSE.ru.md | :539; :753–761; :815 | Тот же смысл, без старого тезиса под новой формулировкой. |
| docs/methodology/USAGE-BY-ROLE.md | встроенный Agent: Review / Audit prompt :118–134, особенно :132–133 | Prompt должен разрешать конечный доказательный ноль, не self-review/автоматический PASS; короткая ссылка после блока, чтобы путь оставался читаемым и для prompt consumer. Существующий flow ADR/score/drift сохранить. |
| docs/methodology/USAGE-BY-ROLE.ru.md | :118–134, :132–133 | То же по-русски. |
| docs/schemas/PRD-SCHEMA.md | :73–82 | Сохранить список проверок measurable goal/edge/timeline/stakeholder/security как вопросы, а не гарантию дефекта. Удалить ≥1/обязательный повтор при нуле; ссылка ../methodology/QUALITY-GATES.md#2-adversarial-review-protocol. Не менять validator/code. |
| docs/schemas/PRD-SCHEMA.ru.md | :73–82 | То же по-русски, ссылка на RU §2. |
| templates/prd/_TEMPLATE.md | comments :275–277 | Будущая авторская подсказка без квоты и с ссылкой ../../docs/methodology/QUALITY-GATES.md#2-adversarial-review-protocol. Старые instantiated artifacts не перегенерировать. |
| CHANGELOG.md | [Unreleased] :12, сейчас пуст | Только после реального применения узкая запись Changed о core review policy и сохранённых gates; никаких заявлений shipped binary/website/runtime. |

### Before hashes — exact raw file bytes, SHA-256

```text
38ac9c796127d84278dd00bb1f1d48cb658c5e9fb54a8e79ec1229b25f4549eb  CLAUDE.md
4260eed9183a79d4701292db4678eddc374ec38486502517ec57a7798b1d55e3  docs/methodology/QUALITY-GATES.md
a16754d3090ac9d116d98212488d3bdc8f2ea12752778afd8ec241b8890a5bc4  docs/methodology/QUALITY-GATES.ru.md
88819a5db9dee46f62006924b098309ccc15a05a9d6a3eeec3a1a6f519a166ea  docs/methodology/HOW-TO-USE.md
28d82ed02724e16eecb0abe23011fa15c13f48c781f8191461a464b7127640ce  docs/methodology/HOW-TO-USE.ru.md
651d4ba8ec58c5b4adcf0fcd9fda9410b2480fd05f01f1cdc4ef496f7973a8c0  docs/methodology/GLOSSARY.md
13a43610875f79383664a77af499d99a567b3d63cbff8847ab4d703eea7b0f60  docs/methodology/GLOSSARY.ru.md
53f21bc6cda60ee0960de05ef8a247327091d4f9f69399685e2414dd9daa9a3d  docs/methodology/FORGEPLAN-GUIDE.md
8325190562e3a209ac0501a12ac3b2972fed152ef1d9c7b35692b3757080dc3a  docs/methodology/FORGEPLAN-GUIDE.ru.md
ecc9ef9bb357aee75b4e38eeb6d5a0a172851a6c564b4138e2a3d1bff0b5ca39  docs/methodology/METHODOLOGY-COURSE.md
bee59df65204ce0a473e33de26fa63405bfd5532c45a1c1857dc282af1e13cd1  docs/methodology/METHODOLOGY-COURSE.ru.md
6b4608d12d33cd3b81d007c729bac0c170311b09f07b476a3e09a1e7b3f89309  docs/methodology/USAGE-BY-ROLE.md
1bd248429d861b7f9d2c6f80a61fd431c9b3d73da995919a80671ea8d79a524f  docs/methodology/USAGE-BY-ROLE.ru.md
a8fc265729dd54aeb02bbfca46f5e0bd0e0c87db19e0685f396fbbe8a85dc7d6  docs/schemas/PRD-SCHEMA.md
8912c40b75a96b7076a5c193a208ee88a7815c6e0696bffc5f8af63c87136cf2  docs/schemas/PRD-SCHEMA.ru.md
f6a08ace5840bbff35dd9d446292deb9a086ef36eabb75480217e3a9972f40eb  templates/prd/_TEMPLATE.md
698c09d5f30f306199233c096655d07d2755077ab8dc94fb7a439cf8f7fa519e  CHANGELOG.md
```

Writer получает собственный worktree только после pre-source gates. Он не наследует предположение «HEAD тот же значит bytes те же»: сверяет каждый before hash на входе. При отличии перечитывает участок и сообщает integration owner, не перезаписывает чужой текст. Одновременных source writer/compile нет. Integration owner только оркестратор; creator сейчас не создаёт worktree, не переносит изменения и не делает commit. При позднем переносе сравниваются incoming/current hashes с этой базой; конфликт не разрешается whole-file overwrite. После интеграции меняется candidate identity и требуется проверка именно integrated дерева.

### FORBIDDEN and historical preservation

Для source writer запрещено всё вне 17-path allowlist, в частности `AGENTS.md`, `.forgeplan/**`, `website/**`, `marketplace/**`, `crates/**`, `Cargo.toml`, `Cargo.lock`, `.cargo/**`, `.github/**`, `scripts/**`, root docx, Runtime/AgentBootstrap, installed plugins/skills, home/global settings/auth. Lifecycle/graph новых ADR/RFC/EVID выполняется отдельно только уполномоченными canonical MCP/CLI actors с явным owning root; MiniMax не переписывает артефакты reviewer.

`docs/operations/BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md` — датированный исторический snapshot (заголовок и :3–10 прямо описывают сессию 2026-04-19…04-21). Квоты :738 и :991 сохраняются в историческом контексте; его assertions/findings не переписываются. `docs/sessions/**`, `docs/vnext/**/_audit/**`, исторические `.forgeplan/**` и instantiated template bodies также исключены. Здесь не утверждается исчерпывающий повторный аудит всех этих каталогов. Их quota-фразы не нормативная замена текущего core QUALITY-GATES; это пояснение размещается в текущем §2, без изменения истории. Отдельную историческую аннотацию пока не добавлять.

Защищённые текущие projections; **никаких новых graph edges к ним не требуется** для этого ADR/RFC:

```text
a0959046856a371d457bc497f6251a827ece9f28b2956663aa6b15ae433c7e7d  .forgeplan/evidence/EVID-172-prd-087-spec-007-r1-independent-shape-fitness-audit.md
5336eddc3526238893d78ebbaa8ef3392b9f56ac2dcd0634ca5d2de77317885e  .forgeplan/evidence/EVID-173-prd-087-spec-007-note-053-independent-finite-local-shape-re-review.md
52788c120dbfdd8fb19c5b51f31005892f8cd53730472da16de56be27014d8ac  .forgeplan/notes/NOTE-053-prd-087-spec-007-bounded-authored-adi-and-factual-shape-handoff-resolution.md
9f83bd7974b82e07b1b2cb2e90787e1ca742e2afa145c2c8e0f6f52294286e98  .forgeplan/prds/PRD-087-website-onboarding-task-stack-selector-and-revision-bound-public-guide.md
4abb2402c0e24e84383a5dd5cb0b29853502e6d5f5f66d0236c1949405d64833  .forgeplan/specs/SPEC-007-website-onboarding-bounded-selection-and-public-guidance-behavior.md
```

В initial owning status отдельно наблюдались чужие изменения `.forgeplan/config.yaml`, PRD-024/047 projections, Cargo.toml, `.cargo/`, docx и untracked website design artifacts. Их не reset/clean/stash/переписывать. Новые additive edges к ADR-003/025 могут менять graph metadata через canonical link, но не разрешают body rewrite действующих ADR.

## Verification and Acceptance

### AC1 — Один полный протокол, без ослабления gates

Fresh Sol High verifier читает candidate все 17 файлов, не только diff и summary. Сопоставляет EN/RU D1–D4, root audit roles/count/trigger, Critical ≥2 rounds, контраргументы, traceability и незатронутые validation/ADI/R_eff/security/runtime/consent/activation требования. В USAGE-BY-ROLE проверяется именно текст внутри prompt; в template — comments для будущих artifacts. Находит реальные оставшиеся противоречия, не выполняет квоту. Изменение reviewer count или automatic PASS — reject.

### AC2 — Независимые решения на конкретных синтетических packs

Это **фиктивные входные материалы для проверки понимания политики**, не receipts реального запуска и не implementation/runtime доказательства. Автор не назначает им независимый PASS. Fresh Sol High reviewer, не автор policy/fixtures/source, должен действительно разобрать каждый pack по новому candidate протоколу, записать собственный verdict/почему/ограничения в новый canonical EVID. Таблица ниже — ожидаемые результаты, не результаты выполненных проверок. String-only assertion или наличие слов PASS/BLOCKER не засчитывается.

Базовый synthetic candidate `SYN-r1` — четыре строки ASCII UTF-8, каждая с завершающим LF (268 bytes), SHA-256 `a4470a220071f5377e876bf8d267e2fbd1360889935f27863dd080ad1d0f6bd2`:

```text
Independent review MAY end PASS with zero defects only after complete current evidence and scope coverage.
Generator and verifier MUST differ.
Unresolved HIGH or Critical findings MUST block.
Runtime and activation gates remain separate and mandatory when applicable.
```

`SYN-r2` = ровно r1 плюс строка `Critical review MUST retain at least two rounds.` и LF (317 bytes), SHA-256 `032fb7d99ccaeb444c42dd78d0e433ffcf5628eba8740443c6933a5256de68d6`.

Общий pack A: local design-only scope, generator G, verifier V != G; target current SYN-r1 с указанным digest; обязательные критерии K1 finite evidence-bound zero, K2 independence, K3 HIGH blocks, K4 no runtime/activation promotion. Synthetic review record содержит K1→строка1→inspected/satisfied; K2→строка2→G!=V confirmed; K3→строка3→inspected/satisfied; K4→строка4→inspected/satisfied. Findings=[]; strongest counterargument «галочки могут не отражать реальное чтение», ответ «V получает весь candidate и независимо проверяет каждую строку и текущий digest»; edge checks missing/stale/self/HIGH — negative decisions приложены как синтетические рассуждения. Runtime skipped, вне local scope, implementation/publication/activation not assessed; Critical/code-trigger не применим к этому учебному design scope. Это не выдаёт fictional V за фактически выполненного reviewer; реальное решение выносит свежий verifier задачи.

| Case | Конкретное отличие входа | Требуемое решение независимого verifier |
|---|---|---|
| A Complete/current, zero-defect | Полный pack A и весь target; V сверяет digest и каждый K | MAY PASS только local design, если его собственная проверка подтверждает достаточность; не обязан выдумать finding |
| B Zero findings, missing coverage | Pack A, но K3 result/evidence отсутствуют, записано skipped; findings по-прежнему [] | Не PASS; запросить достаточную проверку K3. CONCERNS либо BLOCKER по влиянию, не нулевой счётчик как основание |
| C Changed baseline | Current target SYN-r2, review target/pin и traceability остались r1; новая строка о Critical не проверена | Не PASS для current target; review устарел, проверить релевантную дельту и новое покрытие |
| D Unresolved HIGH | Pack A плюс H-001 HIGH: «candidate позволяет преждевременное activation», exact synthetic clause location указана; status=open, исправления/принятия нет | BLOCKER, даже если все остальные K зелёные. Не переименовывать реальную HIGH в улучшение ради PASS |
| E Self-review | Pack A, но generator=G и verifier=G; новый session label не меняет actor | Не независимый PASS; нужен отдельный verifier |
| F Runtime skipped | Pack A явно local design-only; runtime skipped. Вариант F2 требует runtime acceptance при тех же отсутствующих результатах | F может закончиться только bounded local design PASS при собственной достаточности. F2 не может получить runtime/whole-gate PASS; отсутствие producer/runtime явно остаётся dependency |
| G Critical/count | Target SYN-r2, scope Critical + significant new public API; required set security S и code C. Вход G1: S и C есть, но только round1. Вход G2: rounds1/2 есть, но только C. Вход G3: оба требуемых профиля и ≥2 rounds, current evidence каждого предъявлены | G1/G2 не PASS независимо от zero findings; G3 допускает PASS только после собственной проверки качества/полноты и всех прочих gates. Количество reviewer и раундов не заменяется нулём |

Синтетический H-001 в D задан как unresolved finding pack, не как обнаруженный автором дефект r1. Reviewer проверяет правило обработки такого finding, не приписывает его действительному core коду. Если independent verifier считает формулировку pack неоднозначной, это реальная test-design concern; автор уточняет fixtures, reviewer не переписывает собственную находку. Финальный Evidence включает полные использованные fictional inputs и реальные ответы, candidate body/tree hashes, фактическую независимость и назначенную модель/High; не только ожидаемую таблицу.

### AC3 — Реальный read-path smoke и ссылки

После source применения independent verifier действительно проходит `CLAUDE.md` → §2 guide (оба языка) → PRD schema и embedded reviewer prompt → критерии A–G → собственный EVID. Для каждого перехода открывает target и anchor; проверяет root `.forgeplan/adrs/ADR-027-finite-evidence-backed-independent-review-without-a-defect-quota.md`, sibling `QUALITY-GATES.md`/RU, schemas `../methodology/...`, template `../../docs/methodology/...`. Inline скрипт проверки существования путей/anchors допустим как throwaway, но реальный смысл проверяется чтением. Source grep помогает найти остатки квоты, не является доказательством принятия политики. EN/RU parity — смысловая проверка, не равенство строк.

### AC4 — Template rendering/inspection boundary

Не запускать `forgeplan new prd` в owning repo ради smoke. Реальную поддержанную CLI выдачу проверять только в disposable `/tmp` workspace: из него `forgeplan init -y`, затем `forgeplan new prd "Policy template probe"`, `forgeplan get` созданного ID и чтение выданных review comments. Записать binary identity, cwd, exact commands/exit и наблюдённый template. Никаких build/install/server.

До прогона проверить поддерживаемый способ выбора project template по актуальному CLI/help. Не придумывать flag или raw запись `.forgeplan` шаблона. Если установленный бинарь использует embedded старую копию и supported override отсутствует, это явно recorded limitation: root source template inspected, emitted embedded copy unchanged; AC о shipped новом template не заявляется и не входит в этот docs-only cutover. Старую квоту в probe трактуют по ограниченному приоритету core §2; не перегенерируют исторические owning artifacts. Disposable canonical artifacts создаются только CLI/MCP, не raw writes. Этот creator smoke не запускал — это условие проверки будущего source изменения.

### AC5 — Сохранность, происхождение и приёмка

Проверить exact before/after hashes OWNED, отсутствие изменений FORBIDDEN и защищённых projections, узкий diff, реальную интегрированную baseline identity. Документировать чтения/команды и факт выполнения; не выдавать structural validate за независимый review. Для new independent EVID обязательны verdict/congruence_level/evidence_type и корректные informs-связи; pin ADР/RFC body, source candidate, использованные материалы/результаты. Real source provenance derives from current tree, не из самоотчёта MiniMax. Не заявлять Rust validator/runtime/build/website PASS: эти поверхности не меняются и не запускаются этой задачей. Никаких permanent tests по строкам исходника.

## Roles and Handoff Contract

1. **Creator Sol High, Profile A** — этот ADR/RFC и authored ADI; canonical mutations только новых design artifacts; не source, не независимый Evidence, не activation, без дочерних dispatch.
2. **Fresh Sol High pre-source verifier** — собственный independent review exact ADR/RFC, owner choice, architecture applicability, Deep engine limitation и safe sequence; canonical Evidence. Не редактирует собственные findings в исходном решении.
3. **Fresh Sol High guardian** — читает полную цепочку evidence, явно разрешает/останавливает source phase под active architecture. Guardian не активирует и не делает source правок.
4. **MiniMaxM3 High, единственный source writer** — только после положительного независимого разрешения и оркестраторского dispatch; свой worktree; exact 17-path contract. Возвращает candidate hashes, узкий diff, фактические действия и ограничения. Не меняет design/evidence artifacts или reviewer verdict, не подменяется Sol.
5. **Fresh Sol High source verifier** — независимо читает actual integrated candidate, выполняет A–G и read-path/template проверки; записывает реальные результаты/concerns. Требуемый security-expert и code-reviewer/reviewer при их существующих triggers не исчезают и не считаются выполненными одной лишь надписью «source verifier».
6. **Fresh Sol High final guardian** — вся актуальная цепочка, остаточные проблемы, другие gates, достаточность доказательств. Не выполняет activation.
7. **Owning orchestrator** — единственный integration/lifecycle owner; только при guardian PASS и всех применимых правилах выполняет canonical score/activation нужных артефактов с явным root. Сам PASS не активирует автоматически. Producer/website gates отдельно; source policy acceptance не закрывает их.

## Implementation Phases

| Phase | Выход | Состояние при handoff |
|---|---|---|
| 0 Design | Полные ADR-027/RFC-020, formal authored ADI, exact source baseline | Подготовлено автором; validation/readback выполняются отдельно, это не gate verdict |
| 1 Pre-source gates | Independent architecture/ADI/sequence evidence и guardian | Не выполнено; source dispatch запрещён до разрешения |
| 2 Source cutover | Только MiniMax, allowlist, свой worktree, затем безопасная интеграция | Не начато |
| 3 Verification | Fresh independent A–G, semantic parity, read path, links/template boundary, preservation | Не выполнено |
| 4 Lifecycle | Actual acceptance evidence, R_eff, guardian, canonical actions оркестратора | Не выполнено; оба проекта draft |

### Implementation Checklist

- [ ] Независимо решить architecture applicability, red line 7 sequence и engine ADI prerequisite; сертифицировать допустимость проверки изменения квоты без выдумки.
- [ ] Получить pre-source guardian и оркестраторский MiniMaxM3 High dispatch с exact before hashes.
- [ ] Применить все 17 allowlisted core файлов, EN/RU, сохранить FORBIDDEN и исторические записи.
- [ ] Выполнить независимые A–G, read-path/link/template smoke и preservation проверку на integrated candidate; записать реальную эвиденцию.
- [ ] Пройти final guardian/R_eff/другие применимые gates; lifecycle выполняет оркестратор, не этот creator.

## Invariants

Инварианты ADR-027 применяются целиком: generator != verifier; актуальная версия и достаточные доказательства; отсутствие квоты не автоматический PASS; unresolved HIGH/Critical блокируют; никакого сокращения reviewer/раундов/остальных gates; historical/forbidden preservation; нет runtime/shipped/website claims по source-only правке. Exact allowlist и before hashes обязательны.

## Risks

Риск скрытого упрощения → AC1/A–G и отдельные scoped verdict. Риск stale review после интеграции → re-pin/review current tree. Риск уничтожения истории или чужих изменений → allowlist, hashes, surgical изменения, запрет whole-file overwrite/reset. Риск двух норм → полный текст только в existing §2, краткие ссылки в копиях. Риск неправомерной активации → drafts до source/evidence и явная независимая проверка коллизии. Риск неподтверждённого High → не запускать default provider, prerequisite ADR-027 остаётся незакрытым.

## Rollback Plan

До source никакого rollback текста не требуется — оперативные инструкции ещё прежние. При обнаружении дефекта после cutover исправляет writer в рамках отдельно согласованного объёма, затем fresh re-review; старые Evidence не переписываются. Возврат старой квоты не автоматический rollback: требуется новое решение владельца с причиной и проверкой. Триггеры пересмотра ADR-027 применяются ко всему cutover.

## Resume

Следующая допустимая работа родительской сессии — independent pre-source review этих проектов и указанных незакрытых вопросов. Это handoff дизайна, не отправка MiniMax и не утверждение, что P1/website/engine/activation уже закрыты. Health наблюдался через `forgeplan health --json` в owning cwd: exit0, total442, needs_attention, 2 at-risk (PRD-005/RFC-004), 10 advisory phase mismatches; чужой долг не исправлялся и global health PASS не заявляется.

## Related Artifacts

- ADR-027 informs этот RFC: owning решение, точное owner authorization, formal ADI, engine limitation.
- ADR-025 / ADR-003 based_on: применимые active process/storage boundaries; независимая сертификация ещё требуется.
- ADR-026 reference: provenance, не новый data model parent.
- EVID-173 §P1 и защищённые EVID-172/NOTE-053/PRD-087/SPEC-007: только references в тексте, без мутации их bodies/status/verdict.


## Engine Invocation Update and Current Handoff — 2026-10-16

Это factual дополнение creator/operator. Указания выше «engine run не выполнен»/«supported High control не найден» сохраняют исторический статус **до** этой операции, а не текущую оценку. Нормативный контракт, 17-path allowlist, before hashes, title/depth/status не меняются.

Выполнен ровно один новый `/opt/homebrew/bin/forgeplan reason RFC-020 --json --save` с process-local provider=codex-cli/model=gpt-6.1-sol и временным PATH adapter. Время 2026-10-16T20:55:04Z–20:56:47Z, **exit 0**, outer deadline 720 s. Input body SHA-256: 24292ed4a1be007c9def4b7b7e3531cafaaf3ea0a470dabe0b35efd4df5bd7a3. Реальный argv содержит абсолютный stock Codex 0.163.0, ровно один --model gpt-6.1-sol и --config model_reasoning_effort="high"; original stdin и read-only/ephemeral/ignore-user-config/ignore-rules/skip-git-repo-check сохранены. Config/auth/global files не изменены. Подтверждён запрошенный supported High control, не независимая телеметрия effort на сервере.

**NOTE-054**, draft/tactical, title «ADI analysis of RFC-020», создана самим --save и уже informs RFC-020. В ту же NOTE дописаны полные без сокращения engine stdout/stderr, actual argv, времена, exit, bytes/hashes и input pin; вторая одноцелевая analysis NOTE не создавалась. Полный output прочитан: 3 hypotheses, 3 deductions, 8 evidence-needed, ranking H1 > H3 > H2, confidence Medium. H1 совпадает с выбранным 17-file cutover; H2 — ранее отвергнутое изменение только QUALITY-GATES; H3 допустима лишь как организация подготовки полного результата, не частичная приёмка. Новых нормативных поправок не принято.

Engine не читал независимо actual source или связанные canonical bodies и сам оговорил отсутствие такого подтверждения. Его слова о недоказанном собственном engine run сохранены буквально; receipt отдельно фиксирует реальное исполнение. Advisory output не является independent Evidence, guardian verdict или creator self-PASS. Достаточность Deep анализа и factual-only delta до текущего body независимо оцениваются reviewer/guardian.

Отдельный ADR-027 вызов завершился exit 1 до принятого analysis, не повторялся; его receipt — **NOTE-055**, не автоматически сохранённый анализ. RFC вызов не удовлетворяет ADR gate. Остаются pre-source review и guardian по active architecture/red line 7/Deep adequacy; если ADR engine обязателен, нужен отдельно разрешённый новый вызов. До положительного независимого разрешения MiniMax source dispatch запрещён. Ни один implementation/runtime/website/producer/activation gate не объявлен пройденным. Предыдущий §Resume читается вместе с этим уточнением текущего engine состояния.


## Current engine resolution — 2026-10-16T21:03:37.838Z

Factual creator/operator update: the owning orchestrator authorized one NEW bounded ADR-027 invocation after the temporary adapter precondition changed from custom environment dependency (removed by provider) to a literal receipt path, already proven by actual RFC-020 success. Command `forgeplan reason ADR-027 --json --save` completed successfully (wall56.66s) and saved NOTE-056, linked informs ADR-027. Input ADR body SHA256 `e757631639c42e4b44f38a93c9dddb16c260400f465d2b45fb049c3cb5d49da2`; actual model `gpt-6.1-sol`, supported requested effort `high` verified in stock Codex argv. NOTE-056 preserves complete original tool transcript, exact argv, provenance and limits. Four hypotheses/deductions and ten evidence-needed; H1 finite evidence-bound policy preferred, no normative delta. NOTE-055 failed attempt preserved; preceding engine-unresolved handoff statements are historical, not the current execution state. Both new Deep artifacts now have actual separate engine analyses (RFC-020 NOTE-054, ADR-027 NOTE-056) plus authored ADI. Independent reviewer/guardian must assess their adequacy, active architecture/redline7 and actual source readiness. This operator update is not self-PASS, not activation and not MiniMax dispatch. No source/config/auth/website/producer/tracker changes. New successful engine request is not a repetition of the old website call.







