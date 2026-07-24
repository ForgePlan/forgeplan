# Сессии executors: как оркестратор запускает раны в нужном harness

> Ответ на «как делать сессии CC/Codex и т.д., чтобы оркестратор запускал
> задачи там где нужно и в нужном harness». Факты web-верифицированы
> (июль 2026). Выбор ПАРЫ (harness × model) — в `model-routing.md`; здесь —
> механика запуска, наблюдения и завершения сессии.

## 0. Модель сессии

Один **ран = один спавн executor-процесса** в своём worktree, со своим
собранным окружением. Никаких долгоживущих «общих» сессий harness'а на
несколько задач: сессия принадлежит рану, живёт под супервизором (переживает
disconnect оператора — Herdr H-5) и умирает/паркуется вместе с раном.
Всё, что нужно ранy, инжектируется **per-process** (env + флаги + inline
config) — общие конфиги (`~/.claude/settings.json`, глобальный `config.toml`,
`opencode.json`) из оркестратора и воркеров НЕ мутируются никогда.

```
Scheduler выбрал задачу → Runtime Broker выбрал пару (harness, model)
  → Bundle Composer собрал скиллы/агентов/промпты (skill-forge.md)
  → Worktree Governor выдал worktree + scope lease
  → ExecutorDriver.createRun():
      spawn(harness CLI, cwd=worktree, env=собранный, флаги=режим+модель)
      + spawn `forgeplan serve` в том же worktree (stdio MCP, PRD-078 workspace)
  → streamEvents(): контрактный канал (JSON-события) + PTY-эвристика (fallback)
  → collectOutcome(): exit + артефакты + вердикты + eval-строка
```

## 1. Claude Code

| Аспект | Механика |
|---|---|
| Headless-запуск | `claude -p "<prompt>" --output-format stream-json` — потоковые JSON-события → контрактный канал RunEvents |
| Модель per run | `--model <id>` или `ANTHROPIC_MODEL` в env процесса (доки прямо описывают «разные модели в разных терминалах»); remap алиасов `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL`; субагенты — `CLAUDE_CODE_SUBAGENT_MODEL` |
| Продолжение сессии | session-id из первого ответа → `claude --resume <session-id> -p "..."` — многошаговые раны (T2⇄T3 циклы) без потери контекста |
| Права | `--permission-mode`, allowlist в `.claude/settings.local.json` worktree; опасное `--dangerously-skip-permissions` — только в изолированном контейнере по risk-policy |
| Изоляция конфига | cwd=worktree даёт project-scope `.claude/`; env-инъекция per process. **Ловушка:** `env`-блок в settings-файле бьёт shell-export той же переменной — health-check рана обязан верифицировать эффективный конфиг; `/model` в автоматизации запрещён (с v2.1.153 пишет в общий `~/.claude/settings.json` → race между воркерами) |
| Gateway-случай | non-native пары: + `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1`, `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, `CLAUDE_CODE_MAX_OUTPUT_TOKENS` (см. model-routing A.1/A.2) |

## 2. Codex CLI

| Аспект | Механика |
|---|---|
| Headless-запуск | `codex exec "<task>" --json -o out.txt` — non-interactive режим с JSON-выводом |
| Модель per run | `-m <model> --model-provider <id> --profile <p> -c key=value` — всё флагами, без правки config.toml |
| Изоляция конфига | **`CODEX_HOME=<per-worker dir>`** — полная изоляция config/auth/истории per worker; `--ephemeral` для одноразовых ранов; ключ — `CODEX_API_KEY` / `env_key` в env процесса |
| Ограничение | только Responses-API endpoints (`wire_api="responses"` — единственное поддерживаемое); Chat-Completions-провайдеры напрямую — reject (model-routing №7) |
| Локальные модели | `--oss` (Ollama/LM Studio, gpt-oss) — офиц. путь для локального T3 |

## 3. OpenCode

| Аспект | Механика |
|---|---|
| Headless-запуск | `opencode run "<prompt>" -m provider/model --agent <name> --format json` |
| Модель per run | флагом `-m provider/model`; per-agent модели (`agent.<name>.model`) маппятся на tiers T0–T3 |
| Изоляция конфига | **`OPENCODE_CONFIG_CONTENT`** — inline JSON per process (Broker генерирует из шаблона пары); либо `OPENCODE_CONFIG`/`OPENCODE_CONFIG_DIR`; ключи только `{env:VAR}`-ссылками |
| Daemon-топология | `opencode serve --port N` + `opencode run --attach http://host:port` (auth `OPENCODE_SERVER_PASSWORD`) — вариант для пула воркеров на удалённом хосте; для MVP достаточно spawn-per-run |
| Провайдеры | любые: models.dev каталог + `@ai-sdk/openai-compatible`/`@ai-sdk/cerebras` + `options.baseURL` — канонический дом «других моделей» |

## 4. Сквозные правила сессий

1. **Env собирает Broker, ключи — только в env процесса** (никогда в файлы конфигов; у CC/Codex/OpenCode у всех есть env-путь). Ключи провайдеров живут в Model Gateway / secret store.
2. **`forgeplan serve` спавнится per worktree** тем же адаптером, что владеет worktree (stdio, PRD-078 `workspace`); write-дисциплина: literal-body, pinned binary + health smoke.
3. **Наблюдение двухканальное:** primary — JSON-стрим harness'а → RunEvents; fallback/cross-check — PTY-эвристика (Herdr H-1); heartbeat в Lease Manager — из любого канала; тишина обоих > TTL → expiry policy.
4. **Завершение:** `collectOutcome` не верит self-report — вердикт даёт отдельный верификаторский ран + filesystem/CI проверка (evidence-first close); outcome-строка (eval-кортеж) пишется всегда, даже при фейле.
5. **Version guards:** все три CLI движутся быстро (Codex wire-churn, OpenCode schema-renames, CC settings-precedence) — пины версий harness'ов в committed intent + smoke-тест пары на каждый релиз harness'а; фейл smoke → пара в quarantine.
6. **Reload-правило:** новые скиллы/агенты/конфиг подхватываются на старте сессии — «author-then-use» всегда два рана (authoring → respawn).
7. **«Чем ещё можно кодить»:** новый harness = новый ExecutorDriver-адаптер (createRun/streamEvents/cancelRun/collectOutcome) + минимум одна eval-прошедшая пара. Кандидаты дальше по списку Herdr (Droid, Amp, Cursor Agent, Gemini, Cline…) — добавляются по потребности, не впрок.
