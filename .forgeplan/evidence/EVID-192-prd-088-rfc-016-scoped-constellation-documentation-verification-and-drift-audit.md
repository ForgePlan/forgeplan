---
depth: standard
id: EVID-192
kind: evidence
links:
- target: RFC-016
  relation: informs
- target: PRD-088
  relation: informs
status: draft
title: 'PRD-088 RFC-016: scoped Constellation documentation verification and drift audit'
---

---
assigned_number: 192
created: 2026-10-23
predicted_number: 192
slug: evid-prd-088-rfc-016-scoped-constellation-documentation-verification-and-drift
updated: 2026-10-23
---

# Constellation documentation — scoped verification for PRD-088 / RFC-016

## Verdict

**PASS только для перечисленных механических проверок замороженного рабочего дерева документации/Node.** Это не подтверждение landed commit, публикации сайта, полного перевода, всей доступности, качества поиска или production/S1-G/S2-приёмки. Наблюдения Main о браузере и runtime ниже атрибутированы отдельно: DocsFinalVerify их не запускал.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
review_verdict: PASS

## Measurement

- Исполнитель собственных checks: DocsFinalVerify; фактическая модель `openai-codex/gpt-6.1-sol`. Audit identity для claim/release: `claude-code/2.1.282/tester-task-DocsFinalVerify`. Версия в теге получена через установленный Claude CLI, но проверки исполнял OMP-агент на указанной фактической модели, не Claude-модель.
- Цели: PRD-088 и RFC-016. Принятое направление — EN/RU how-to/reference в действующей структуре; этот pack не меняет ROOT/RFC-015/S1/S2.
- Выбранный бинарник: `0.39.0+constellation.s1.1`; SHA-256 `95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332`; Rust source baseline `c852f40c515085d328f02e4b275edda37279a610`. Cargo не запускался; Rust — **NOT_APPLICABLE**.
- Среда: macOS/arm64 этой машины; Node v22.18.0, npm10.9.3, существующие зависимости без install. Контекст внутренних временных fixtures совпадает с проверяемым checkout и бинарником. Никакие реальные настройки пользовательских проектов не заменялись.
- Исполненный v3 manifest:495 файлов, SHA-256 `63a2b88bb26b57e58d41e72cf6c9984cd48c37e83b7bb661e25f3eee160d2005`;495/495 совпали до/после финальной сборки. Источник EN changelog — корневой CHANGELOG; штатный prebuild пересоздал опубликованную копию, это разрешённое и отдельно записанное исключение вне старого495-набора.
- Delivery v5:496 product-файлов, SHA-256 `2eabe64a18c7e7b645d9d9457e3ee05e17b46be9f15daa81d81d34db6227edfb`;496/496 проверены до записи этого pack. Website/Node inputs исполненияv3 побайтово не менялись. Позже добавлены contributor-отчёт, ссылки двух indexes и комментарий шаблона evidence. Наружные contract metadata допускают evidence links; тела PRD/RFC заморожены отдельно.
- Последняя версия постоянного отчёта: SHA-256 `8024b0a08880f6c9c620f98057f67267f943d7f36eb6744ceaf498f6e90d5f34`. LOW-уточнения:21 неработающая ссылка означает21 вхождение и4 уникальные цели; parent graph имел2 узла/0рёбер, impact — пустой impacted.15 ссылок отчёта не изменились после этих текстовых уточнений.

## Ground-truth verification

Проверяется **снимок рабочего дерева**, не утверждение о состоявшемся commit. Начальная независимая git-проба показала BASE=HEAD=c852f40c515085d328f02e4b275edda37279a610, committed/cached DELTA=EMPTY и непустую working-tree docs delta; полный command/stdout сохранён в .local/documentation-audit/final-verification/git-ground-truth.*. Это не используется как положительное доказательство landed change.

Поля git code-claim `base_sha`, `result_sha` и `changed_paths` намеренно **все три отсутствуют**: pack не выдаёт frozen working-tree измерение за git-delta provenance. Наличие ожидаемых новых токенов и byte hashes зафиксировано в final-v3/expected-delta-token-probe.json, freeze-prebuild.json, freeze-postbuild.json и canonical-evidence/pre-write-fingerprint.json.

## Command run

Точный основной runner: `npm run build`, cwd=website/, exit **0**, wall **15.785006s**; Astro **566 pages in13.89s**. Один final-v3 build; ранее был один v2 intended-final build, после которого отдельный link-check нашёл реальные отказы. Повторная сборка была разрешена только после исправлений и нового freeze. Старые журналы не переписаны.

Остальные реальные команды и exit codes сохранены полностью в final-v3/commands.json и соседних stdout/stderr. Основные существующие entrypoints:

- `node scripts/check-content-completeness.mjs --binary "$FPL"`, exit0; FPL — выбранный выше бинарник, полный разрешённый путь/argv сохранён в receipt.
- `node scripts/check-translation-drift.mjs --verbose`, exit0.
- `node scripts/check-dead-links.mjs` после сборки, exit0.
- Временные private-only regression/AST/fragment drivers: точные command, inputs, code и status в final-v3/audit-instrumentation.json и receipts. Проверка самой функции extractEmbed выполнялась настоящим TypeScript-модулем через Node native stripping, а не копией её regex. Постоянные test files не добавлялись; временные файлы удалены.

Ключевой stdout (дословно):

```text
[cli-gen] 120 command paths, 2 locales; 0 missing/stale pages; 0 unmatched pages
[mcp-gen] full=76, constellation-read=9, 2 locales; 0 missing/stale pages; 0 unmatched pages
[content] PASS: CLI and both MCP profiles covered in EN/RU; generated blocks match the selected binary. Handwritten prose was not semantically reviewed by this check.
[drift] EN files (including changelog): 236 | RU mtime not older: 235 | mtime candidates: 0 | unexpected missing: 0

[drift] Expected RU files exist; no mtime candidates. Changelog is intentionally excluded. Semantic equivalence was not checked.
[links] scanning 587 HTML files
[links] checked 124553 internal links across 587 pages
[links] OK no broken internal links
14:55:19 [build] 566 page(s) built in 13.89s
```

## Result

| Измерение | Результат |
|---|---|
| CLI help/reference |120/120 путей, включая вложенные;оба языка;0missing/stale/unmatched |
| MCP schemas/reference |76full и9read;оба языка;0missing/stale/unmatched;это регистрация контракта, не runtime-приёмка всех tools |
| Генераторы |36/36:render/check,сохранение ручных байтов EN/RU,interrupted/orphan/reversed/repeated markers,stale/missing/obsolete check без мутации |
| Canonical TMPDIR guards |4/4 negative cases:direct/symlink и observer/real binary; named отказ до serve;fixture bytes/mtimes не менялись |
| Ancestor .env isolation |2successful command-cases;4real serve children получают явный пустой FORGEPLAN_WORKSPACE;58file private migrated target и CURRENT остались побайтово/поmtime прежними |
| Guide helper |133/133 проверки настоящего helper и20raw lessons;каталожные href/query/fragment/locale нормализованы,неизвестные/external/non-guide цели не выдуманы;CSS/script extraction сохранены |
| Changelog helper |19/19 private cases;5настоящих contributor targets отображаются на repo blob/main;missing/outside/directory цели остаются видимыми с ожидаемым diagnostic;source/site-root/anchors/query сохранены |
| Сводка regression |194passed,0failed,0skipped;это ad-hoc проверки/command-cases, **не unit suite и не code coverage**;flakiness не измерялась |
| Contributor v3 |181docs+7entrypoints=188Markdown;351/351local file targets;0missing |
| Contributor closure |182docs+7entrypoints=189Markdown;362/362local file targets;0missing;518references;79external и77fragment-only не проверялись на доступность;4relative fragment destinations не проверялись наfragment |
| Собранный сайт |566Astro pages+21raw HTML=587HTML;124553internal links;0broken file targets |
| Новые страницы |4EN/RU routes,36authored crosslinks,1152outbound+1412inbound checks,258fragment checks;0missing targets/fragments/duplicate IDs |
| Копии raw guide |21/21byte-identical;6конкретных built route/preservation checks прошли |
| Сборочные warnings |Только Vite>500kB и Starlight Entry docs→404;других production-build warnings не наблюдалось. Преднамеренный missing/outside/directory diagnostic относится к private negative fixture,не замаскирован. |

Инвентарь источников v3: портал236EN/235RU,единственное ожидаемое EN-only отличие changelog;блог22/22 с одинаковыми именами;20guide entries и21raw HTML;landing1EN без RU twin. Contributor до отчёта181Markdown,теперь182. Все source inventory partitions меньше200;truncation не использовалась. Оба guide locale используют один raw body — это не перевод. В built corpus:landing1,docs237+237,blog24+24,guides21+21,raw21,404other1. RU changelog — fallback,не отдельный переведённый source.

Code statements/branches и delta: **n/a** — PRD не задаёт числового code-coverage threshold;оно не измерялось и194не переименовывается в coverage.

### Slow checks, top5 excluding build

| Check | Wall | Exit |
|---|---:|---:|
| `constellation-anchors` | 2.167 s | 0 |
| `v3-safety-cli-prose-check` | 1.316 s | 0 |
| `v3-safety-cli-ru-orphan-end` | 1.085 s | 1 |
| `v3-safety-cli-en-interrupted-start` | 1.077 s | 1 |
| `v3-safety-cli-normal` | 1.011 s | 0 |

## Parent-owned supplementary evidence

**Исполнитель этого подраздела — Main. DocsFinalVerify прочитал и сопоставил сырые receipts, но не запускал browser,точный опубликованный shell recipe,полный stateful walkthrough или ручной embed. Их наблюдения не прибавлены к194собственным checks.**

- browser-proof.json на v3 built preview:4новые EN/RU страницы,реальные sidebar/locale переходы,read-contract anchors,наблюдаемое раскрытие JSON Schema,keyboard _top,390px reflow и переходы EN/RU embedded lessons. В сохранённых JS error logs нет записей;это не all-site a11y/translation certification. Первоначальный общий summary-selector попал в sidebar,не JSON Schema;он сохранён как instrumentation miss,после него использован наблюдаемый JSON Schema disclosure.
- walkthrough-exact-guide-v3.json:дословный положительный shell exit0,2source IDs,полная выдача,неоднозначный unqualified get;EN/RU shell bytes равны,recipe SHA-256 af29f0324906c6edf578d6a9ecfb616c82ac28b55c794291ec49f385e3a03ec6. Оба negative ancestor-TMPDIR примера exit1 до artifact writes;sentinel body и CURRENT не изменились.
- dotenv-routing-runtime.json+dotenv-routing-provenance.json:unset-control прочитал ancestor sentinel;явный empty дал named no-workspace/maintenance-resolution отказ;explicit binding прочитал alpha;все3MCP EOF exit0,поколение отдельного sentinel target не изменилось. Это отдельные disposable fixtures,не production incident.
- walkthrough-complete.json и walkthrough-auto-and-snapshot.json:canonical CLI get byte-equal source,qualified get,all9read tools и snapshot-specific source identity упражнены;full/read процессы EOF exit0. Graph=2nodes/0edges,impact=0impacted — **не** приёмка dependency walk.
- Main сохранил первоначальный semantic exit1 «No embedding profile is published» и последующий успешный ответ,не выдал начальное отсутствие профиля за pass. После настоящего full MCP update без ручного embed:дословный host stderr `tick: queue drained — 5 embedded, 0 stale, 0 failed, 0 still eligible`;затем5eligible/5expected,prefix+sections,persisted=true,published ready. Это узкая readiness-проверка,не качество/latency benchmark.

## Interpretation

Измерения поддерживают узкое утверждение PRD-088/RFC-016: новые документы доступны в принятой EN/RU структуре, интерфейсный reference соответствует выбранной локальной сборке, генерация сохраняет ручной текст и не уходит в чужой workspace, подтверждённый навигационный drift исправлен и сохранён в постоянном отчёте. Они не расширяют фактическую runtime-приёмку.

Исходные отказы сохранены:21contributor broken-link occurrences/4unique absent targets,94built broken links/30targets и4unique shared main-content fragment failures. Final affected round дал0;v2raw47command receipts сохранены неизменными. Неудобные historical failures не удалены.

Остаются явно ограниченные области:

1. Установленный Starlight0.38.2:host aria-expanded=true,вложенная доступная button остаётся false;визуальный переход работает. Не объявлять all-a11y PASS. node_modules и observer-shim не правились.
2. Embedded EN/RU routes используют общий raw body;на EN BMAD остались русские абзацы. Не объявлять полный перевод. Оба browser findings сохранены в https://github.com/ForgePlan/forgeplan/issues/542#issuecomment-6364622226 . Preview отдельно сообщил DEP0205/module.register();это не production-build warning.
3. Полная semantic parity переводов,внешние URL,все исторические fragments,весь handwritten corpus,поисковое качество,ресурсы,Windows/fault/production rollout не проверялись. CLI validate на migrated layout известным образом отказывает;не запускать старый binary/store в обход;current full MCP validate используется для этого pack.
4. Ранние независимые spec/architecture gates не доказаны,первоначальные Node changes написал parent. Позднее review не переписывает порядок событий. Исторические runtime issues#495/#538/#540 не закрываются этими checks.

## Congruence Level Justification

CL3 — для **этого локального замороженного docs/Node контекста**:внутренние измерения на фактически используемом binary/source snapshot,существующих инструментах сайта и изолированных реальных fixtures. Parent observations взяты с того же local build/binary и явно отделены по исполнителю. CL3 не переносится на иные OS,production,чистую установку Starlight или неизмеренные качество/доступность/перевод. review_verdict=PASS — результат узкого gate; scoring verdict=supports — направление поддержки этого ограниченного утверждения,не полная зелёная оценка проекта.

## Sources and provenance

Постоянный реестр: [Documentation audit](../../docs/operations/DOCUMENTATION-AUDIT-2026-10-23.ru.md); https://github.com/ForgePlan/forgeplan/issues/542 . Локальные raw logs/receipts и весь code/input временных instruments сохраняются отдельно;их hashes ниже. Секреты/credentials не включены.

| Владелец/источник | Путь от repo root | SHA-256 |
|---|---|---|
| Тестер: final-v3 report | `.local/documentation-audit/final-verification/final-v3/report.json` | `df4a4b5f7c162403ca434b345d32ac199b05c7a6c7f7472de58d51fec49cb58d` |
| Тестер: contributor closure | `.local/documentation-audit/final-verification/contributor-closure/closure-record.json` | `075d560c801a9dc761de36ab603b60f741a233d0865026062181b0befca50530` |
| Тестер: финальная точность/ссылки | `.local/documentation-audit/final-verification/canonical-evidence/closure-v5-precision-check.json` | `86dab8c01261ea3e764281cc59452f407d10628f69cf0eca87490e08677afe3b` |
| Исполненный source freeze | `.local/documentation-audit/source-freeze-v3.json` | `63a2b88bb26b57e58d41e72cf6c9984cd48c37e83b7bb661e25f3eee160d2005` |
| Delivery freeze | `.local/documentation-audit/delivery-freeze-v5.json` | `2eabe64a18c7e7b645d9d9457e3ee05e17b46be9f15daa81d81d34db6227edfb` |
| Постоянный отчёт | `docs/operations/DOCUMENTATION-AUDIT-2026-10-23.ru.md` | `8024b0a08880f6c9c620f98057f67267f943d7f36eb6744ceaf498f6e90d5f34` |
| Main: browser-proof.json | `.local/documentation-audit/browser-proof.json` | `9948088228f3260a4a0b89381c33920dc3581776fe00ada69570c0c47f12fbe0` |
| Main: walkthrough-exact-guide-v3.json | `.local/documentation-audit/walkthrough-exact-guide-v3.json` | `2797bd89586033cf7f9d4c21501db2747c1c48279d7cb3d12d3a295b509ffe11` |
| Main: walkthrough-complete.json | `.local/documentation-audit/walkthrough-complete.json` | `61409c3b92d6dfcef9cc61e22904ddfe570ba162196b81f1fff83a3c63d4de99` |
| Main: walkthrough-ancestor-guard.json | `.local/documentation-audit/walkthrough-ancestor-guard.json` | `f9e6c838ba521176e63a26badf60eb00ab81a7ba77463ab31556f898f757b1d9` |
| Main: dotenv-routing-runtime.json | `.local/documentation-audit/dotenv-routing-runtime.json` | `1572d4add1a580e6e1a57bcd28763b9d65b7eea7eb76805e84f78c294a0fd51d` |
| Main: dotenv-routing-provenance.json | `.local/documentation-audit/dotenv-routing-provenance.json` | `a13963c1e86d94fc65ffe14a7cc68446095f43142b16de08d5174957442a5ec3` |
| Main: walkthrough-auto-and-snapshot.json | `.local/documentation-audit/walkthrough-auto-and-snapshot.json` | `f0b3a4a92ed2a258eea7c3e4317e458508ca83aaa34e6cfe2dbf45837657da1f` |

## Related Artifacts

| Artifact | Relation | Scope |
|---|---|---|
| PRD-088 | informs | FR-001–FR-005 documentation/drift evidence |
| RFC-016 | informs | Chosen documentation contract,source boundaries and actual verification |

Этот pack остаётся draft до решения guardian. Никакой activate не вызывался;ROOT/RFC-015/S1/S2/trackers не изменяются. Raw canonical write/validate/score/claim/release request/return payload и EOF exit будут сохранены в .local/documentation-audit/final-verification/canonical-evidence/.
