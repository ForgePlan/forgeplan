---
depth: standard
id: RFC-014
kind: rfc
last_modified_at: 2026-10-12T12:38:42.804509+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: SPEC-007
  relation: based_on
status: active
title: Shared subprocess profiles for subscription-authenticated LLM CLIs
---

---
assigned_number: 14
created: 2026-10-12
prd: null
predicted_number: 14
slug: rfc-shared-subprocess-profiles-for-subscription-authenticated-llm-clis
updated: 2026-10-12
---

## Summary

Добавить в `forgeplan-core` закрытый набор профилей локальных LLM CLI и один общий subprocess runner. Профиль отвечает за fixed argv, prompt transport и разбор финального ответа; runner — за binary trust, окружение, timeout, ограниченный вывод, stdin и cleanup. Первая поставка: Codex, Devin, Gemini CLI и Cursor Agent.

## Motivation

`claude-code` уже доказывает ценность локальной подписки, но его реализация привязана к Claude envelope. Четыре новых copy-paste метода дали бы пять реализаций одного security boundary. Пользовательская shell-template, наоборот, слишком гибкая: она превращает `config.yaml` в исполнение произвольной команды и не позволяет ForgePlan гарантировать read-only режим. Нужна середина: общий runner + кодовые профили фиксированных официальных CLI.

## Options Considered

### 1. По отдельному `generate_<vendor>` на каждый CLI

Просто для первого провайдера, но дублирует timeout, output cap, env allowlist, recursion, diagnostics и тестовые hooks. Исправление security-дефекта придётся переносить в пять мест. Отклонено.

### 2. Пользовательский command template в YAML

Позволяет подключить любой CLI без релиза ForgePlan, но вводит прямую поверхность command/argv injection и не даёт проверить семантику флагов. Отклонено.

### 3. Закрытые provider profiles поверх общего runner

Каждый поддержанный CLI явно закодирован и протестирован; общая опасная механика одна. Новый vendor требует небольшой profile + parser + tests, но не даёт пользователю выполнить произвольный binary. Выбрано.

### 4. Только OpenAI-compatible прокси

Сохраняет один HTTP transport, но официальные подписочные CLI не обещают локальный OpenAI-compatible endpoint. Не решает запрос без неофициального token bridge. Отклонено.

## Proposed Direction

### Module boundaries

- `config/types.rs`: mapping строк provider → `LocalCliProvider`; keyless availability.
- `llm/local_cli.rs`: enum/profile metadata, prompt assembly, fixed argv construction, provider parser, shared spawn lifecycle и unit/mock-binary tests.
- `llm/mod.rs`: ранняя dispatch-ветка перед Anthropic/HTTP; существующий `claude-code` сохраняет публичный config/result/envelope contract, переводит repository-controlled system instruction из argv во framed stdin и использует тот же bounded subprocess runner и общий recursion sentinel, чтобы alternating-provider nesting не обходил guard.
- `playbook/dispatch/helpers.rs`: переиспользовать существующие safe binary resolution, env allowlist и bounded drain; расширить visibility только для `which_in_path` и output cap.

### Invocation flow

1. `LlmClient::generate` спрашивает `config.local_cli_provider()`.
2. `local_cli::generate` проверяет общий recursion sentinel и model.
3. Создаёт временный workspace; для Codex — изолированный HOME и output file, для Devin — prompt file.
4. Разрешает фиксированный binary через существующий permission-gated PATH resolver.
5. Вызывает общий `run_subprocess` без shell, очищает env, задаёт fixed argv/cwd/stdin.
6. Runner одним timeout охватывает параллельную запись stdin, bounded drain stdout/stderr и ожидание процесса; по timeout убивает и reap-ит direct child, а truncation возвращает явно.
7. Provider parser извлекает final text; empty/error дают точную диагностику.

### Profiles

- `codex-cli`: stdin + output-last-message; read-only, ephemeral, isolated HOME while auth remains under CODEX_HOME.
- `gemini-cli`: sandbox + default approval + JSON; stdin with empty `--prompt`. Plan mode отвергнут: Gemini CLI 0.28.2 требует experimental flag.
- `cursor-agent`: ask mode + JSON; prompt through stdin; isolated cwd and sandbox.
- `devin-cli`: macOS/Linux only; official `--print` + prompt-file; sandbox and normal permissions; isolated cwd; explicitly weaker read-only guarantee. Windows rejected before spawn because stock Devin rejects the required sandbox.

### Environment

Базовый allowlist остаётся PATH/HOME/USER. XDG передаётся Gemini/Cursor и, на Linux, `claude-code` для штатного credential storage; `CODEX_HOME` — только Codex; API-key переменные не передаются. Для Codex `HOME` указывает на приватный временный каталог, чтобы не загружать посторонние skills/context. Child PATH исключает относительные/пустые entries и project root, выбранный CLI/MCP workspace resolver. Общий `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` всегда устанавливается на child.

### Tests

Тестовые binary overrides cfg-gated и provider-specific (`FORGEPLAN_CODEX_BIN`, `FORGEPLAN_DEVIN_BIN`, `FORGEPLAN_GEMINI_BIN`, `FORGEPLAN_CURSOR_AGENT_BIN`). Mock scripts записывают argv/stdin и возвращают vendor output. Все env-mutating tests используют существующий `DISPATCH_ENV_LOCK`.

## Implementation Phases

- [x] Phase 1: config mapping, keyless classification, local provider enum and dispatch seam.
- [x] Phase 2: shared runner with recursion, fixed binary resolution, timeout, bounded streams, temp workspace and cleanup.
- [x] Phase 3: Codex profile + parser + real ADI smoke.
- [x] Phase 4: Gemini and Cursor profiles + installed-binary smoke attempts; Devin profile + official-contract/mock smoke.
- [x] Phase 5: docs/config examples, regression tests, format/check/focused and website verification.
- [x] Phase 6: EvidencePack EVID-172 linked, scored and activation gates passed.

## Verification Results

- 13 focused local-profile tests pass: exact mapping, fixed safe flags, prompt transport, strict parsers, actual child env/cwd, workspace PATH rejection, missing binary, non-zero exit with stderr withholding, full-lifecycle timeout and cross-profile recursion.
- 22 `claude-code` tests pass after migration to the shared runner and integration of the Claude CLI 2.x event-array envelope fix; its public config/result/envelope contract remains compatible while system and user text no longer enter argv.
- 21 shared-runner tests pass; the cancellation regression was then strengthened and independently passed by observing the direct child PID disappear after task abort. Coverage includes blocked-stdin timeout, 10 MiB cap/truncation, workspace binary rejection, kill-on-drop and direct-child timeout reaping.
- 42 routing tests, 3 HTTP-provider configuration regressions, 1 default-provider regression and 1 runtime-workspace regression pass.
- The MCP estimate path now scopes local binary and secret lookup to the request-selected workspace. Its dedicated test compiles under `cargo check`; executing the `forgeplan-mcp` test target was blocked before test execution because the linker exhausted the shared target disk (`errno=28`), so it is not counted as a pass.
- Real `forgeplan reason PROB-001 --json` through `codex-cli` completed in 62.82 s with 11,037 bytes of structured ADI output and no `OPENAI_API_KEY`.
- Installed Gemini CLI reached the profile, but its local configuration requested `GEMINI_API_KEY`; ForgePlan intentionally did not forward that secret, so an OAuth/subscription login is still required for a success smoke.
- Installed Cursor Agent reached the profile and reported `Authentication required`; `cursor-agent login` must be renewed before a success smoke.
- Devin is not installed on this workstation; the official CLI contract and mock-binary path are covered.
- `cargo fmt --all -- --check`, `cargo check -p forgeplan-core -p forgeplan -p forgeplan-mcp`, the focused suites above, and the bilingual website build pass (436 pages; Pagefind indexed 457 HTML files).

## Risks and Mitigations

- Vendor flags/output drift → exact mock contracts + real smoke for installed binaries; malformed output is error, never fallback text guessing.
- Subscription terms change → explicit opt-in/disclosure, stock clients only, no token extraction/client spoofing.
- Agent executes tools → strongest documented no-write mode, isolated empty cwd, no auto-approval flags; Devin limitation and Windows rejection explicit.
- Descendant survives direct-child timeout → keep all profiles in vendor sandbox/no-write modes; implement process-group/Job Object ownership under GitHub issue #513 before widening execution.
- Secret leakage → `env_clear` and allowlist; tests assert API-key absence for every profile.
- Temp-file disclosure → private tempfile primitives and cleanup on drop; errors never echo prompt.

## Related Artifacts

- PRD-087 — requirements.
- SPEC-007 — invocation contract and scenarios.
- ADR-017 — existing single-vendor precedent.
- EVID-172 — implementation and smoke evidence.
