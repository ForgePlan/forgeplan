---
depth: standard
id: EVID-172
kind: evidence
last_modified_at: 2026-10-12T12:38:43.312812+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-014
  relation: informs
- target: PRD-087
  relation: informs
- target: SPEC-007
  relation: informs
- target: ADR-027
  relation: informs
status: active
title: Local CLI providers verified by contract tests and real Codex ADI
---

---
assigned_number: 172
created: 2026-10-12
predicted_number: 172
slug: evid-local-cli-providers-verified-by-12-profile-tests-and-real-codex-adi
updated: 2026-10-12
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Что проверялось

Реализация RFC-014: четыре закрытых subscription-CLI профиля (`codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`), общий subprocess boundary, отсутствие передачи API keys, совместимость существующего `claude-code`, реальный ADI через Codex и сборка пользовательской документации.

## Результат

- `cargo test -p forgeplan-core --lib local_cli`: **13 passed, 0 failed**. Покрыты exact provider mapping, fixed safe argv всех четырёх профилей, prompt как data, vendor parsers, env allowlist, missing binary, non-zero exit, timeout и recursion guard.
- После интеграции PR #493 `cargo test -p forgeplan-core --lib claude_code`: **22 passed, 0 failed**. Keyless Claude provider сохранил config/result/envelope contract, принял Claude CLI 2.x event-array envelope; system и user text идут через stdin. Более широкий фильтр `--lib claude` дал **63 passed, 0 failed**.
- `cargo test -p forgeplan-core --lib playbook::dispatch::helpers::tests`: **21 passed, 0 failed**; после усиления cancellation regression отдельный тест `run_subprocess_kill_on_drop_reaps_direct_child_pid` также прошёл и доказал исчезновение PID прямого потомка.
- Routing: **42 passed**; HTTP config: **3 passed**; default-provider: **1 passed**; runtime-workspace: **1 passed**.
- `cargo fmt --all -- --check`: PASS.
- `cargo check -p forgeplan-core -p forgeplan -p forgeplan-mcp`: PASS.
- Реальный `forgeplan reason PROB-001 --json` с `provider: codex-cli`, `model: ""`, без `OPENAI_API_KEY`: **exit 0**, **62.82 s**, **11,037 bytes** структурированного ADI JSON. Disclosure появился один раз в stderr; prompt прошёл через новый профиль end-to-end.
- `npm run build` в `website/`: PASS, **436 pages**, Pagefind **457 HTML files**. EN/RU configuration docs собираются.

## Установленные альтернативные CLI

- Gemini CLI профиль был реально запущен. Локальная установка настроена на `GEMINI_API_KEY`, а не OAuth/subscription login; ForgePlan правильно не передал ключ и CLI завершился code 41 с требованием API key. Для положительного subscription smoke нужен интерактивный OAuth-вход.
- Cursor Agent профиль был реально запущен и завершился `Authentication required`; нужен свежий `cursor-agent login`.
- Devin CLI на машине отсутствует. Его fixed argv, prompt-file transport и parser прошли общий mock-binary E2E.

Эти три ограничения относятся к состоянию vendor login/installation на workstation, а не к профилю Codex или общему runner. ForgePlan не сделал небезопасный fallback на API key и не выбрал другой provider автоматически.

## Сырые доказательства

| Файл | Bytes | SHA-256 |
|---|---:|---|
| `/tmp/local-cli-tests-round3.log` | 2,000 | `54a44b7775692fb7173795b9082052edf8ff8b8133e727eb39f6b2fcc228ca83` |
| `/tmp/subprocess-helper-tests-round3.log` | 2,555 | `83d378680050ac483df2dea098d90577ae43d36af7ed1516833ab6de6e5db04d` |
| `/tmp/subprocess-cancellation-regression.log` | 16,172 | `26422539b8634bc66233e279ef1ae4a9665039cdb1e745e98c9b4059a79c1b6d` |
| `/tmp/claude-code-regression-round3.log` | 2,455 | `266d9bdac37de15bb084f4e82498b304c3d7886c5d5a15719b066b553f6ffaca` |
| `/tmp/claude-code-merged-provider.log` | 2,532 | `65f4e65e3a5e4bcba859594b1706cb7b47a1259e3a0df69158a2bc711ecd6043` |
| `/tmp/claude-merged-feature.log` | 21,867 | `a78ae6d267a37c4d952c8063565e0b957fa874be479dc8cad921a68d60869c33` |
| `/tmp/routing-tests-round3.log` | 3,661 | `37424bcf03422975e2b387a6eaa1ac204dbdb7ec7a1d17a868e91833cf392122` |
| `/tmp/http-provider-regression-round3.log` | 890 | `f49c4845c2c0285d0afa9e7a3b7877479b626168bc1e3c848f692f91168d1eb1` |
| `/tmp/runtime-workspace-config-test.log` | 782 | `4032c3e61d09fdd2cfc6095c474aebfbfa340dc2b1f9999ccaff065f89241d70` |
| `/tmp/provider-selection-regression.log` | 894 | `8f9c0b5cb38e71415c5acf2d30d1625b8195dc8101ad8056b7b66fcde3b123c9` |
| `/tmp/local-cli-check-round4.log` | 14,877 | `ee3c6a14ec4ae61b53bceaf305d95628718a07658db6cd0c31f63589ee92405d` |
| `/tmp/forgeplan-codex-adi-smoke.out` | 11,037 | `fe7b452553499938941f1c9d0d816a8b10f63dcbf5cc17e86f6bd5b41b0487b7` |
| `/tmp/forgeplan-codex-adi-smoke.err` | 294 | `ce235f79b24ca5525904fc417adbbbbc7068a042f97da25b43bf98e69c3be9ac` |
| `/tmp/website-build-local-cli.log` | 28,607 | `52cdf2a0540c5859730b18f80e1e7a6c1ddee8cbe9c1418c1afacb34fb2455f7` |
| `/tmp/mcp-local-cli-errors-round4.log` | 169,305 | `98bcffa2fbb2eeec9ef596e3a2b2f1b10ffcd08470fe67f2bb5114977c4144d4` |
| `/tmp/gemini-profile-diagnostic.err` | 424 | `0242326d4c24972b70bf2534b000cfa448961fb89d61a079ff55991905b3dcf2` |
| `/tmp/cursor-profile-diagnostic.err` | 108 | `4de7608ae6736aa85dde5c2817bc1ee0f62662e311cd742bcf09ec6ad788d225` |

## Границы

- Полный `cargo test -p forgeplan-core local_cli` не стартует integration binaries из-за существующего несвязанного дефекта: integration tests обращаются к cfg(test)-only `LanceStore::create_artifact_for_test`/`add_relation_for_test`. Поэтому доказательство использует корректный unit target `--lib`; zero-run был явно обнаружен, не засчитан как PASS.
- Отдельный запуск `cargo test -p forgeplan-mcp llm_error_tests` не дошёл до тестов: Cargo начал линковать несвязанный `integration_e2e`, диск заполнился, `ld` завершился с `errno=28`. `cargo check` трёх изменённых crates прошёл; этот запуск записан как ограничение, не как PASS.
- Положительные реальные smoke для Gemini/Cursor и любой реальный Devin smoke остаются условными на vendor login/install. Mock coverage не выдаётся за действующую подписку.
- Проверено на macOS arm64.

## Related

- RFC-014 — реализованный контракт
- SPEC-007 — сценарии и security boundary
- PRD-087 — требования
- ADR-027 — выбранная архитектура
