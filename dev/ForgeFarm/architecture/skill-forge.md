# Skill Forge — discovery + authoring скиллов и суб-агентов (Т-5)

> Ответ на Т-5: «система сама находит нужные скиллы из известных источников
> и по алгоритму и best practices пишет суб-агентов и скиллы сама». Основано
> на web-верифицированных фактах (июль 2026: экосистема skills/marketplaces,
> ToxicSkills-инциденты) + локальных ассетах пользователя (find-skills,
> skill-creator, plugin-dev, agent-creator, skill-reviewer,
> AGENT-AUTHORING-GUIDE). Адверсариально проверено.


### B.1 Discovery: источники и механика

**Split-verdict, определяющий дизайн: механический discovery — реален; механический trust — фикция.** Индексный слой полностью скриптуется, но в экосистеме нет ни подписи, ни review-gate (skills.sh листится по install-телеметрии без модерации; из 3 984 просканированных публичных скиллов 36.8% имели security-flaw, 13.4% — критический; реальная malware-кампания на 30+ скиллов уже прошла через ClawHub, февраль 2026). Поэтому discovery = **INDEX + RANK**, а допуск в реестр = всегда отдельный gated-шаг.

**Allowlisted source catalog** (config-файл, который потребляет core; сам список — methodology-слой, меняется без релиза Rust):

| Источник | Механика запроса | Роль |
|---|---|---|
| anthropics/skills + partner directory | git clone / GitHub API | Первичный доверенный корпус |
| Собственный ForgePlan/marketplace | raw-fetch `marketplace.json` (стабильная документированная схема) | Домашний реестр; канал публикации |
| Curated awesome-lists (VoltAgent/awesome-agent-skills, VoltAgent/awesome-claude-code-subagents, ComposioHQ, hesreallyhim) | parse markdown | Seed-корпус; **единственный** содержательный источник по субагентам |
| skills.sh / `npx skills find` | scriptable CLI | Только ranking-hint, никогда trust-источник |
| GitHub API | code search `filename:SKILL.md`, topics `claude-skill`/`agent-skills`, raw marketplace.json известных маркетплейсов | On-demand поиск |
| MCP Registry (`registry.modelcontextprotocol.io/v0/servers`) | REST API | MCP-нога бандлов; единственный источник с реальной namespace-аутентификацией |

**Ranking-эвристики** — переиспользуем уже кодифицированные в локальном `find-skills` SKILL.md: installs ≥ 1K, официальные org (anthropics/vercel-labs/microsoft), repo ≥ 100 stars, свежесть. Это **rank features для shortlist'а, никогда не admission-критерии** (телеметрия геймится).

**Асимметрия skills vs agents:** SKILL.md — открытый стандарт (30+ tools читают), скиллы discoverable. Субагентские .md — CC-specific, реестра нет → для агентов **authoring — первичный путь приобретения**, discovery — non-goal. Честный выход discovery — shortlist в гейт (десятки, не тысячи), не auto-import поток.

### B.2 Security gate по autonomy-профилям

**Trust state machine:** `discovered → quarantined → trusted(pinned) → deprecated` (supersede, не delete — маппится на lifecycle ForgePlan).

Гейты между состояниями — слоёные, потому что каждый слой по отдельности документированно обходится:

- **G1 STATIC** — `skills-ref validate` + secret-scan + сканер класса Cisco Skill Scanner / Snyk Agent Scan (pattern + LLM-judge) + инспекция bundled `scripts/`. Необходим, никогда не достаточен (multimodal hidden-instruction атаки специально обходят сканеры).
- **G2 ADVERSARIAL LLM REVIEW** — агент профиля skill-reviewer, read-only tools, severity-bucketed verdict. Reviewer ≠ admitter.
- **G3 SANDBOXED TRIAL** — прогон в одноразовом worktree/контейнере **без credentials в env**, затем **blast-radius diff**, явно включающий `MEMORY.md`, `CLAUDE.md`, settings- и hooks-файлы (persistence-атаки, переживающие uninstall, задокументированы в дикой природе). Любая запись вне заявленного scope → reject.
- **G4 PROVENANCE PIN** — контент вендорится в собственный реестр по git SHA / content-hash; live-fetch при композиции **запрещён**; любое изменение upstream сбрасывает ассет в quarantined и перегоняет G1–G3.

**Маппинг на autonomy-профили** (профиль меняет форму одобрения, не отключает контракт):

| Профиль | Правило допуска |
|---|---|
| manual | Человек одобряет каждый допуск, глядя на сырьё |
| assisted | Человек одобряет машинный gate-report (одно решение, evidence приложен) |
| autonomous | Композиция **только** из уже trusted(pinned) ассетов. **Autonomous никогда не допускает новые чужие скиллы** — допуск структурно supervised-операция, точка |

Quarantined-ассеты использовать можно — но только sandboxed low-privilege T2/T3 воркерами под мониторингом; промоушен в trusted после N чистых прогонов, записанных как EvidencePack (verdict/CL/evidence_type) — изменения trust-состояния гейтятся той же R_eff-машинерией, что и routing table.

**Escape hatch для хороших чужих идей:** adaptation policy из AGENT-AUTHORING-GUIDE — импортируй ИДЕЮ, перепиши своим голосом, поставь `origin: community`; это сознательно конвертирует supply-chain-риск в authoring-работу, идущую через pipeline B.3. Community-origin никогда не понижает security-планку.

### B.3 Authoring pipeline: need → spec → draft → adversarial review → eval → registry

**Verdict: auto-authoring реалистичен** — редкий случай, когда каждая стадия уже существует как проверенный локально установленный компонент. Claude генерирует валидный SKILL.md нативно (доктрина Anthropic); работа ForgeFarm — **оркестрация и гейтинг, не authoring-технология**.

8 стадий, 4 жёстких гейта:

1. **NEED (гейт)** — прогнать golden-tasks БЕЗ скилла и записать конкретные фейлы (eval-first доктрина Anthropic) + **pre-creation similarity check** против существующего каталога (embedding-поиск по descriptions; in-house прецедент — slug pre-create check forgeplan). Нет baseline-фейла ИЛИ near-duplicate → reject до первой строчки драфта. Этот гейт — главный контроль sprawl'а.
2. **SPEC** — хранится как ForgePlan-артефакт; **spec — регенерируемый source of truth, не сгенерированный output** (смена канона → re-propagation регенерацией). Выбор CRUD-R-A профиля (A creator / B reviewer / C researcher / D maintainer) и methodology-тега.
3. **DRAFT** — generator-агент эмитит SKILL.md / agent.md. Канонические verbatim-блоки (6-пунктовый prompt-defense preamble, HARD RULES) инжектируются **композером byte-for-byte, никогда не парафразируются LLM** — проверяемо линтом. Дешёвый черновик — паттерн `forgeplan_generate` (~$0.005–0.01 vs $0.10–0.50 in-subagent).
4. **LINT (гейт)** — `skills-ref validate` + frontmatter-валидатор: **`disallowedTools` denylist, не `tools:` allowlist** (upstream-баг #53865 тихо срезает целые MCP-серверы; верифицировано EVID-049/050); explicit model; third-person what+when description с trigger-термами.
5. **ADVERSARIAL REVIEW (гейт)** — ОТДЕЛЬНЫЙ инстанс skill-reviewer. generator≠verifier принуждается **структурно, не промптом**: у authoring-агента нет capability записи в реестр (зеркалит Profile A — «creator не может активировать собственный артефакт»).
6. **EVAL-BEFORE-REGISTRY (гейт, несущая стадия)** — ≥3 сценария в `evals.json`, параллельные baseline vs with-skill прогоны, grader-агенты + **trigger-recall eval** и description-optimization pass (under-triggering — документированный дефолтный фейл-режим; паттерн `improve_description.py`) + **filesystem-верификация**, что заявленные файлы существуют с заявленным контентом (self-reports субагентов доказанно врут в ОБЕ стороны — инциденты Sprint Q/R).
7. **REGISTER** — версионированный релиз в собственный marketplace; запись в реестр **гейтится EvidencePack'ом** с результатами eval (verdict/CL/evidence_type → R_eff > 0), штампом `origin: forgeplan`, и **eval-coverage class** (скиллы с субъективным выходом, сопротивляющиеся assertion-грейдингу, получают низший класс, видимый роутингу).
8. **OBSERVE** — production outcome-tuples из eval-loop кормят per-skill usage/decay.

**Переиспользуемые ассеты (уже на диске):** `skill-creator` (evals.json-схема, run_eval.py / run_loop.py / aggregate_benchmark.py / improve_description.py / package_skill.py — забрать wholesale), `plugin-dev` (skill-development + agent-development playbooks, включая внутренний agent-generation prompt CC), `agent-creator` (one-shot генерация с обработкой overlap-конфликтов), `skill-reviewer` (готовый read-only верификатор), `find-skills` (ranking-эвристики), AGENT-AUTHORING-GUIDE (CRUD-R-A таксономия, verbatim-блоки, frontmatter-канон, provenance policy). ForgeFarm их **вызывает, не реимплементирует**.

**Операционное ограничение ExecutorDriver:** новые агенты/скиллы загружаются при старте сессии — «author-then-immediately-use» это всегда два шага: authoring → explicit reload/respawn воркера.

**Известные upstream-поломки, которые pipeline обязан обходить:** `tools:` allowlist срезает MCP (#53865 → только denylist); MCP `forgeplan_update body='@file'` пишет строку литерально с тихой потерей данных (forgeplan#350 → читать файл, передавать контент строкой); agent .md — CC-specific формат (semantics `disallowedTools`/`model` теряются на других executor'ах).

### B.4 Хранение, версионирование, контроль sprawl

**Слои хранения** (официально одобренный split):

| Слой | Что | Для чего |
|---|---|---|
| Marketplace repo (ForgePlan/marketplace) | версионированные plugin-дирректории (`fpl-skills/1.52.0`), marketplace.json catalog | Cross-project distribution; каждый релиз = version bump + EVID audit-trail |
| Project `.claude/` (в git) | project-specific агенты/скиллы | Team-shared, VCS-ревьюится |
| `~/.claude/`, `~/.agents/skills` | личные cross-project | Вне ForgeFarm-контракта |
| **Core registry/lockfile (Rust)** | pinned content-hashes, provenance (source URL + origin), trust-state, eval-coverage class per asset | Единственный источник для композиции; Bundle Composer материализует pinned-дирректории в executor-specific локации (`~/.claude/skills`, `.agents/skills`, `~/.codex/skills`) |

Skills — портируемая единица (открытый стандарт, одна физическая директория шарится между CC/Codex/Cursor); агенты — CC-scoped, для OpenCode/Codex composer держит тонкие per-executor шаблоны-эквиваленты.

**Контроль sprawl:**
- **Dedup:** similarity-index по descriptions, проверяемый на NEED-гейте (до драфта, не после).
- **Decay:** маппинг каждого authored-ассета на ForgePlan-артефакт → `stale`/`renew`/`supersede`/R_eff-decay приходят **бесплатно из существующей машинерии** — новый lifecycle-движок не пишется.
- **Canonical-source доктрина:** shared policy-блоки живут ОДИН раз (в guide), агенты несут verbatim-копии или cite-директивы; «меняй в каноне, потом re-propagate регенерацией из spec'ов; никогда не правь копию in place».
- Ожидание по размеру: допущенный set остаётся малым (десятки, не тысячи) — и это фича, не баг.

**Разделение core vs methodology (литмус-тест):** если фича должна пережить враждебный скилл или врущего субагента — это **core** (Rust: registry/lockfile, gate executor, blast-radius diff, capability-разделение generator≠verifier, eval-оркестрация, policy enforcement, similarity/decay). Если это текст, который люди должны менять без релиза Rust — это **methodology/marketplace** (сами скиллы и агенты, CRUD-R-A таксономия и verbatim-блоки, allowlist источников, authoring-playbooks, per-methodology bundle-рецепты BMAD/SPARC/TDD).

### B.5 Что НЕ строить

1. **Публичный crawler/реестр** по 670k-корпусу skills.sh или scraping HTML агрегаторов — discovery это on-demand запросы к allowlisted-каталогу, не больше.
2. **Собственный формат скиллов/агентов или собственный валидатор** — SKILL.md это стандарт 30+ инструментов с официальным валидатором (`skills-ref`) и легальным `metadata`-полем для наших provenance/tier/eval-штампов; свой формат = экосистемная изоляция.
3. **Любой human-free путь допуска чужих скиллов в autonomous-профиле** — «autonomous discovery + install» это attack surface ToxicSkills, оформленный как продуктовая фича.
4. **From-scratch eval-runner** — забрать evals.json-схему, baseline/with-skill прогоны, grader-агентов и improve_description.py из skill-creator целиком; ForgeFarm добавляет только оркестрацию и EvidencePack-гейт.
5. **Стандарт code-signing/attestation для скиллов** — общеэкосистемная дыра, которую один оркестратор не закроет; content-hash pinning в своём реестре даёт ту же локальную гарантию.
6. **Cross-executor портируемость субагентов** — agent .md CC-specific; agent-authoring остаётся CC-scoped, эквиваленты для OpenCode/Codex — тонкие шаблоны композера.
7. **Trust-скоринг на install counts / stars / leaderboard** — геймящаяся телеметрия; rank features для shortlist'а — да, входы гейтов — никогда.
8. **Жёсткие зависимости от internals `~/.claude/plugins/*`** (installed_plugins.json, cache layout) — недокументированное состояние CC; читать оппортунистически за адаптером с graceful degradation.
9. **Собственную authoring-«AI-технологию»** (мета-промпты, fine-tuned генераторы) — Claude эмитит форматы нативно; каждый доллар идёт в гейты, dedup, evals и registry-plumbing.

---


---

**Сквозные инварианты, соблюдённые в обеих секциях:** generator≠verifier принуждается структурно (воркеры не пишут в routing table; authoring-агент не пишет в реестр; reviewer ≠ admitter); evidence-first (любое изменение allowlist'а пар и любой trust-переход ассета гейтится EvidencePack'ом через ту же R_eff-машинерию); committed intent vs local resolution (routing.yaml / source-allowlist / verdicts — в git; ключи, endpoints, virtual keys — локальная резолюция через env); autonomy-профили меняют форму одобрения, но не отключают ни один контракт (autonomous = только pinned/trusted, swapped-пары стартуют в manual/assisted).