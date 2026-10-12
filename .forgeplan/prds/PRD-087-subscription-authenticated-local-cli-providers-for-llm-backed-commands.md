---
depth: standard
id: PRD-087
kind: prd
last_modified_at: 2026-10-12T11:26:11.650564+00:00
last_modified_by: omp-coding-agent/1.0.0
status: active
title: Subscription-authenticated local CLI providers for LLM-backed commands
---

---
assigned_number: 87
created: 2026-10-12
domain: general / healthcare / fintech / govtech / edtech
epic: null
predicted_number: 87
priority: P0 / P1 / P2 / P3
projectType: web_app / api_backend / mobile_app / cli_tool / library
slug: prd-subscription-authenticated-local-cli-providers-for-llm-backed-commands
stepsCompleted: []
updated: 2026-10-12
---

## Problem

ForgePlan умеет переиспользовать локальную подписку только через `provider: claude-code`. Пользователь с активной авторизацией в Codex CLI, Devin CLI, Gemini CLI или Cursor Agent вынужден отдельно настраивать API-ключ, хотя каждый из этих клиентов имеет официальный неинтерактивный режим. Добавление каждого клиента отдельной копией `generate_claude_code` размножит правила запуска, ограничения вывода и ошибки безопасности.

## Goals

- Дать LLM-backed командам ForgePlan (`reason`, `generate`, Level-2 route, `decompose`, `capture`) явные keyless-провайдеры для локальных подписочных CLI.
- Поставить общий безопасный контракт subprocess-адаптера вместо копирования реализации на каждый CLI.
- Первая поставка: `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`; существующий `claude-code` продолжает работать без изменения конфигурации.
- Ошибка отсутствующего бинаря, авторизации, таймаута или пустого ответа должна называть провайдер и точную команду восстановления.

## Non-Goals

- Автоматический выбор подписочного CLI. Все локальные CLI-провайдеры только opt-in.
- Использование подписочных токенов через неофициальный HTTP endpoint, подмена client identity или обход условий поставщика.
- Поддержка универсальной пользовательской shell-команды из `config.yaml`; исполняются только фиксированные stock binaries с фиксированными argv-профилями.
- Замена HTTP-провайдеров или Ollama.
- Обещание одинаковых квот, стоимости и моделей между подпиской и API.

## Non-Functional Requirements

- **NFR-1 Security:** пользовательский prompt, system instruction, model, workspace path и vendor output не должны создавать shell/argv/PATH injection или утечку process secrets; выбранный MCP workspace является authoritative trust root.
- **NFR-2 Reliability:** один configured timeout покрывает stdin/prompt-file delivery, оба bounded output drain и ожидание child; timeout и cancellation обязаны остановить и reap-нуть прямой child.
- **NFR-3 Compatibility:** HTTP/Ollama и публичная конфигурация `claude-code` остаются совместимыми; внутренний transport и subprocess guardrails могут быть усилены без изменения возвращаемого text contract.
- **NFR-4 Portability:** Codex/Gemini/Cursor работают на поддерживаемых desktop Unix/Windows системах по возможностям vendor CLI; Devin явно отклоняется до spawn на Windows.
- **NFR-5 Operability:** ошибки различают missing binary, login, timeout, truncation, non-zero и malformed output, но не раскрывают vendor stderr, prompt, environment или private paths.

## Functional Requirements

- **FR-1 Provider names:** `LlmConfig` принимает `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent` как явные keyless-провайдеры.
- **FR-2 Shared runner:** все локальные CLI-провайдеры, включая существующий `claude-code`, используют один bounded subprocess runner: безопасное разрешение stock binary вне текущего workspace, `kill_on_drop`, timeout на весь post-spawn lifecycle, параллельные stdin/stdout/stderr, явные признаки truncation и очищенное окружение.
- **FR-3 Prompt transport:** prompt и system instruction не проходят через shell. Используется stdin или закрытый штатный prompt-file; prompt и system instruction не передаются в argv.
- **FR-4 No mutation:** адаптер выбирает официальный read-only/plan/ask/sandbox режим. Когда CLI не даёт строгого read-only режима, запуск выполняется в отдельном пустом временном каталоге без автоматического одобрения инструментов; ограничение явно документируется. `devin-cli` не поддерживается на Windows, где stock CLI отвергает обязательный sandbox.
- **FR-5 Final response:** ForgePlan возвращает только финальный текст модели, а не JSONL events, progress, usage или terminal control sequences. Пустой результат является ошибкой.
- **FR-6 Model safety:** непустой model id проверяется существующим ограниченным набором символов до передачи в argv; пустое значение оставляет выбор модели самому CLI.
- **FR-7 Recursion:** единый sentinel `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` запрещает вложенный local-CLI provider process глубже одного уровня, в том числе переход между разными профилями.
- **FR-8 Authentication:** адаптер переиспользует только штатную локальную авторизацию CLI; API keys и посторонние секреты не прокидываются. Provider-specific home variables разрешаются только нужному профилю (`CODEX_HOME` — только Codex); дочерний PATH исключает project root, выбранный CLI/MCP workspace resolver, и относительные entries.
- **FR-9 Diagnostics:** missing binary, login failure, non-zero exit, timeout, truncated output и malformed output дают разные сообщения с provider name и официальной login/status командой. Vendor stderr не включается в пользовательскую ошибку, потому что может содержать prompt или environment data.
- **FR-10 Compatibility:** `openai`, `claude`, `gemini`, `ollama` сохраняют текущее поведение; `claude-code` сохраняет публичные provider/config/result semantics при усилении внутреннего transport; auto-detection остаётся key-based и не выбирает local CLI.
- **FR-11 Disclosure:** документация помечает каждый local CLI provider как personal/local only, subject to vendor terms and plan usage; shared/CI должен использовать API/Ollama.
- **FR-12 Verification:** каждый профиль имеет mock-binary tests на argv, stdin/prompt-file, env, output parsing, timeout/non-zero/missing binary и cross-profile recursion. Реальный one-prompt smoke обязателен для каждого установленного и аутентифицированного CLI; отсутствие binary/login фиксируется как ограничение среды и не подменяется API-key fallback.

## Acceptance Criteria

1. При `provider: codex-cli` и активном `codex login` команда `forgeplan reason <ID>` завершает ADI без `OPENAI_API_KEY`.
2. На macOS/Linux при `provider: devin-cli` и активном `devin auth login` тот же путь использует `devin --print` и не требует API key; Windows получает явную ошибку unsupported до запуска процесса.
3. `gemini-cli` и `cursor-agent` используют их штатную локальную авторизацию и безопасный non-interactive режим; при отсутствии действующей авторизации возвращается provider-specific recovery hint без fallback.
4. Ни один local CLI provider не выбирается автоматически.
5. Тесты доказывают, что prompt, начинающийся с `--`, остаётся данными и не становится флагом.
6. Тесты доказывают отсутствие API-key переменных в окружении дочернего процесса.
7. Старые тесты `claude-code` и HTTP providers проходят без изменения внешнего поведения.
8. Документация содержит таблицу различий: binary, login, режим без записи, transport, ограничения, production verdict.

## Target Users

- Одиночный разработчик ForgePlan с оплаченной подпиской на Codex, Devin, Gemini или Cursor.
- Автор/рецензент артефактов, запускающий ADI локально без отдельного API-биллинга.

## Constraints and Risks

- CLI flags и JSON envelope меняются независимо от ForgePlan; профили должны падать явно, а не возвращать правдоподобный мусор.
- Подписка может отдельно тарифицировать headless-вызовы. ForgePlan не называет их бесплатными.
- Devin CLI отсутствует в текущей среде: реальная проверка возможна только после установки на macOS/Linux; до этого обязательны официальный контракт + mock-binary E2E.
- Текущий runner гарантированно останавливает и reap-ит прямой child, но ещё не владеет всем descendant process tree; cross-platform process-group/Job Object hardening отслеживается в GitHub issue #513 и обязателен до разрешения tool execution вне vendor sandbox.
- Cursor/Devin являются агентами, а не чистыми completion API; пустой временный workspace и режим без записи уменьшают, но не устраняют vendor-side обработку prompt.

## Related Artifacts

- ADR-017 — существующий `claude-code` provider и его disclosure/security baseline.
- ADR-011 — безопасный `claude --print` subprocess precedent.
- EVID-147 — проверка AC существующего `claude-code` provider.
