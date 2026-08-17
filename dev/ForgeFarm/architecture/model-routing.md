# Model routing через настроенные harness'ы (Т-6)

> Ответ на Т-6: «можно ли передавать модели в настроенные CC/Codex/OpenCode
> (Cerebras, OpenRouter, DeepSeek в CC) и есть ли смысл». Основано на
> web-верифицированных фактах (июль 2026): официальные доки CC model-config /
> llm-gateway, Codex config reference, OpenCode, vendor-доки DeepSeek/Kimi/
> GLM/MiniMax/Qwen, OpenRouter, Cerebras. Адверсариально проверено.
> Сессии/спавн — в executor-sessions.md; skill-слой — в skill-forge.md.


### A.1 Прямой ответ: можно ли передавать модели, и есть ли смысл

**Короткий ответ:** да, можно — все три harness'а официально параметризуются моделью per-process при спавне; смысл есть, но узко: только для T2/T3 и только как allowlist пар (harness × model), допускаемых через eval. Атомарная единица маршрутизации — **пара (harness × model), а не модель**: harness-промпты и tool-схемы затачиваются под родное семейство, и связка эмпирически ломается в обе стороны (документирован кейс, где тот же Opus скорил хуже внутри CC, чем внутри opencode). Поэтому «подключить модель» = «создать новую пару» = «нужен новый eval», а не «переключить параметр».

**Claude Code (CC).**
- *Официально:* `--model` / `ANTHROPIC_MODEL` per process (доки прямо говорят: «разные модели в разных терминалах — запускайте каждый со своим `--model`»); remap алиасов `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU,FABLE}_MODEL`; `CLAUDE_CODE_SUBAGENT_MODEL`; gateway-механизм `ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN` (протокол полностью документирован); провайдерные пути `CLAUDE_CODE_USE_{BEDROCK,VERTEX,FOUNDRY}`; enterprise-governance `availableModels`/`modelOverrides`.
- *Позиция Anthropic:* сам gateway-plumbing официален, но не-Claude модели через любой gateway **явно не поддерживаются**. При этом DeepSeek, Kimi, GLM (Z.ai), MiniMax, Qwen сами публикуют официальные рецепты «наша модель внутри CC» через свои Anthropic-compatible endpoints, и OpenRouter даёт нативный `/api/v1/messages` («Anthropic skin») — т.е. «DeepSeek внутри CC» реален и vendor-supported, но Anthropic-unsupported.
- *Что ломается на чужих upstream'ах:* adaptive-thinking → 400 на нераспознанных моделях; новые beta-поля (`context_management`, `output_config`) → 400 (митигация `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`); auto-compact завязан на точную формулировку ошибки Anthropic; cost-телеметрия CC считает по прайсу Anthropic (мусор для чужих моделей); фоновые вызовы падают тихо без remap'а `ANTHROPIC_DEFAULT_HAIKU_MODEL`; credential-переменная убивает subscription-биллинг. Критично для оркестрации: `/model` с v2.1.153 **пишет в общий** `~/.claude/settings.json` → race между воркерами; в автоматизации — только флаг/env при спавне.

**Codex CLI.**
- *Официально:* `codex exec "task" -m <model> --model-provider <id> -c key=value --profile <p> --json --ephemeral`; изоляция per-process через `CODEX_HOME`; кастомные провайдеры в `[model_providers.<id>]` (config.toml); `--oss` для локальных gpt-oss (Ollama/LM Studio).
- *Критическое ограничение 2026:* официальный Configuration Reference объявил `wire_api = "responses"` **единственным** поддерживаемым значением. Все рецепты 2025 года с `wire_api = "chat"` (Cerebras/DeepSeek напрямую) — мёртвые или умирающие. Chat-Completions-only провайдеры в Codex попадают только через транслирующий слой (OpenRouter Responses beta или LiteLLM) — и это beta поверх чужого harness'а, т.е. двойной штраф.
- *Вывод:* Codex держим для родного GPT-семейства + Responses-совместимых endpoints. Его стратегическая ценность — не экономия, а **cross-vendor diversity для generator≠verifier**: T1-верификация кода, написанного Claude, моделью другого вендора снижает коррелированные слепые пятна.

**OpenCode.**
- *Официально и по дизайну:* единственный harness, спроектированный под multi-provider. Каталог models.dev (75+ провайдеров), произвольные провайдеры через `@ai-sdk/openai-compatible` / `@ai-sdk/cerebras` + `options.baseURL` + `{env:VAR}`; **никакого wire-ограничения** — plain Chat Completions работает, значит Cerebras/DeepSeek/локальные модели подключаются напрямую, без gateway. Per-agent модели (`agent.<name>.model = "provider/model"`) маппятся 1:1 на tiers T0–T3. Для ExecutorDriver — `opencode run -m provider/model --agent <a> --format json` + `OPENCODE_CONFIG_CONTENT` (inline JSON per process) + `serve`/`--attach` (daemon-топология).
- *Вывод:* **канонический дом для «других моделей»**. Ответ на «Cerebras?» — да, сюда: gpt-oss-120b @ ~3000 tok/s — природный T3-движок (validate/repair циклы). Model-swap здесь не ломает harness-контракт, потому что у harness'а нет родного семейства.

### A.2 Архитектура: ExecutorDriver + Model Gateway

**Принцип №1: вся маршрутизация — per-process инъекция при спавне.** Ни одна пара не требует ничего сверх того, что ExecutorDriver уже делает по архитектуре:

```
Runtime Broker: (tier, task_class, autonomy_profile)
  → routing table → (harness, provider_id, model_id, env_template)
  → ExecutorDriver.spawn(worker_env)
```

| Harness | Механизм инъекции |
|---|---|
| CC | `--model` + env: `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL`, `CLAUDE_CODE_SUBAGENT_MODEL`, `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS`, `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, `CLAUDE_CODE_MAX_OUTPUT_TOKENS` |
| Codex | `codex exec -m --model-provider -c key=val --profile` + env: `CODEX_HOME` (изолированный каталог per worker), `CODEX_API_KEY` / `env_key` |
| OpenCode | `opencode run -m provider/model --agent <name>` + env: `OPENCODE_CONFIG_CONTENT` (сгенерированный inline JSON) |

**Запрещено:** мутация общих конфигов (`~/.claude/settings.json`, общий `config.toml`, глобальный `opencode.json`) из воркера или брокера. `/model` в автоматизации — никогда. Health-check воркера обязан верифицировать эффективный конфиг (у CC — `claude --debug` / `/status`-эквивалент), потому что env-блок в settings-файле **бьёт** shell-export для той же переменной — stale-блок тихо перекрывает инъекцию.

**Принцип №2: Model Gateway (задеплоенный LiteLLM) — инфраструктурное условие честности eval-loop, не «ещё один прокси».** CC считает cost по прайсу Anthropic; Codex/OpenCode — по-своему; swapped-пары без своего gateway пишут в eval-кортеж мусорный cost, и routing-таблица, обучаемая на нём, оптимизирует ложную экономику. Gateway закрывает: virtual keys per worker/tier, per-key бюджеты, единый учёт cost по реальному прайсу провайдера, трансляцию Anthropic `/v1/messages` + Responses + Chat Completions поверх 100+ провайдеров, gateway model discovery для CC.

Жёсткие границы скоупа gateway:
1. **Деплоить LiteLLM, не писать своё** — format-translation чужих эволюционирующих payload'ов это ровно то, за что мы отвергаем claude-code-router.
2. **Не гнать через него subscription-трафик CC** — credential-переменная убивает subscription-auth; subscription-воркеры ходят напрямую в Anthropic, gateway только для API-биллинга.
3. **Gateway — учёт и трансляция; routing-РЕШЕНИЯ остаются в Runtime Broker.** Task-type routing не делегируется прокси.

Когда gateway нужен: как только в allowlist появляется первая API-биллинговая или swapped-пара. До этого (чистый subscription CC×Claude) — не нужен.

### A.3 Allowlist-философия: таблица пар

Routing table — **default-deny allowlist пар**. Каждая пара входит в tier/task_class только собственным EvidencePack'ом.

| # | Пара (harness × model-source) | Verdict | Tiers | Ключевое обоснование |
|---|---|---|---|---|
| 1 | CC × Claude native (API/subscription) | **support-first** | T0–T3 (T0/T1 frontier, T2 sonnet, T3 haiku) | Референс-пара, калибровочный ноль eval-loop |
| 2 | CC × Claude через Bedrock/Vertex/Foundry | **support-first** | T0–T3 | Не model-swap, а биллинг/residency тех же моделей; один env-шаблон |
| 3 | CC × vendor Anthropic-endpoints (DeepSeek/GLM/Kimi/MiniMax/Qwen) | **gate-behind-eval** | Только T2/T3, manual/assisted до промоушена | Гипотеза «сильнейший harness + дешёвая модель» — эмпирика для eval, не вера; Anthropic не поддерживает; per-CC-release smoke обязателен |
| 4 | CC × OpenRouter Anthropic skin | **gate-behind-eval** | (i) failover Claude — любой tier при инцидентах; (ii) не-Claude — как №3 | Единый вход по ключам для API-биллинга; лишний коммерческий hop |
| 5 | CC × claude-code-router / transformer-прокси | **reject** | — | Per-request переписывание эволюционирующего payload'а чужим кодом; task-routing — компетенция нашего Broker'а |
| 6 | Codex × OpenAI-native (Responses; + `--oss` gpt-oss локально) | **support-first** | T2 (GPT-family impl), T1 (cross-vendor verification), T3 (gpt-oss локально) | Единственная конфигурация, где Codex оправдан; diversity-арм для generator≠verifier |
| 7 | Codex × Chat-Completions-only напрямую (`wire_api="chat"`) | **reject** | — | Протокол официально вычеркнут вендором |
| 8 | Codex × OpenRouter Responses beta (не-OpenAI модели) | **later** | потенциально T2 | Beta-транспорт × чужой harness = двойной штраф; всё то же доступно через OpenCode дешевле |
| 9 | OpenCode × произвольные провайдеры (Cerebras/DeepSeek/GLM/локальные/Zen) | **support-first** (механизм) | T3 первично (Cerebras/локальные), T2 cost-tier, T1 только frontier как diversity | Harness без родного семейства; каждая КОНКРЕТНАЯ модель всё равно через eval; preview-модели — никогда в autonomous; quota-aware деградация |
| 10 | ForgeFarm Model Gateway (LiteLLM) | **support-first** | инфраслой под №3,4,8,9 и API-частью №1 | Без него cost-поле кортежей — мусор |

**Асимметрия по tiers (главный адверсариальный вывод):** model-swap окупается только в T2/T3 — максимальный объём токенов (экономия реальна) при цене ошибки, ограниченной гейтами T1-verification и T3-validate. T0/T1 на дешёвых swapped-моделях — ложная экономия: объём токенов мал, цена ошибки максимальна; там только native frontier пары (№1, №2, №6-как-diversity).

### A.4 Связка с eval-кортежем

- Кортеж пишется как `(model, harness, task_type, cost, quality, interventions)` — **model и harness вместе**, потому что измеряется связка. Подмена модели внутри harness'а = новая пара = новый eval с нуля; результаты не переносятся ни между моделями в одном harness'е, ни между harness'ами для одной модели.
- `cost` считается по прайсу провайдера **через собственный gateway**, никогда по телеметрии CC.
- Промоушен пары в tier/autonomy-профиль — только через EvidencePack (verdict/congruence_level/evidence_type), гейтящий изменение routing table; то же R_eff-машинерия, что и для остальных решений.
- Любая swapped-пара стартует в autonomy profile manual/assisted и промоутится в autonomous только через evidence. Autonomy profile меняет **кто одобряет**, но не отключает сам гейт.
- Каждая non-native пара несёт **per-release smoke-тест harness'а** (CC/Codex релизы регулярно ломают чужие upstream'ы) — прохождение smoke является условием *пребывания* в allowlist'е, не только входа. Фейл smoke → пара автоматически деградирует в quarantine до починки.

### A.5 Конфиг-поверхность: committed intent vs local resolution

**Committed intent (в git, в repo проекта / ForgeFarm-конфиге):**
- `routing.yaml` — allowlist пар: `capability_class → [(harness, provider_id, model_id, verdict, tiers, autonomy_state, evidence_ref)]`; verdict и tier-привязка меняются только через PR + EvidencePack.
- env-шаблоны пар (какие переменные, без значений секретов): `ANTHROPIC_DEFAULT_*`, `DISABLE_EXPERIMENTAL_BETAS`, `AUTO_COMPACT_WINDOW` per backend, codex-профили, opencode-config-шаблоны.
- пины версий harness'ов для non-native пар (CC/Codex/opencode version guards) + определения smoke-тестов.

**Local resolution (не в git, per machine/per deployment):**
- ключи и endpoints: env vars / secret store; в конфиги — только `{env:VAR}`-ссылки (та же дисциплина, что в `.forgeplan/config.yaml`).
- LiteLLM `config.yaml` с virtual keys и бюджетами — deployment-artifact gateway'я, не проекта.
- резолюция «какой конкретный ключ/квота у этой машины» — lockfile-слой драйвера.

**Материализация:** Runtime Broker читает committed intent → резолвит локально → ExecutorDriver собирает полный env и спавнит. Воркеры не читают routing.yaml сами и не имеют записи в него — generator≠verifier применён к самой конфигурации: тот, кто исполняет, не меняет правила допуска.

---


---

**Сквозные инварианты, соблюдённые в обеих секциях:** generator≠verifier принуждается структурно (воркеры не пишут в routing table; authoring-агент не пишет в реестр; reviewer ≠ admitter); evidence-first (любое изменение allowlist'а пар и любой trust-переход ассета гейтится EvidencePack'ом через ту же R_eff-машинерию); committed intent vs local resolution (routing.yaml / source-allowlist / verdicts — в git; ключи, endpoints, virtual keys — локальная резолюция через env); autonomy-профили меняют форму одобрения, но не отключают ни один контракт (autonomous = только pinned/trusted, swapped-пары стартуют в manual/assisted).