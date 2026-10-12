---
depth: standard
id: EVID-172
kind: evidence
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
title: Local CLI providers verified by real Codex and Devin ADI
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

Реализация RFC-014: закрытые subscription-CLI профили `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`, общий subprocess boundary, отсутствие передачи API keys, совместимость `claude-code`, реальные ADI-запуски через Codex и Devin, пользовательская документация.

## Результат

- `cargo test -p forgeplan-core --lib local_cli`: **13 passed, 0 failed**. Проверены exact provider mapping, fixed safe argv, prompt как data, vendor parsers, env allowlist, missing binary, non-zero exit, timeout и recursion guard.
- `cargo test -p forgeplan-core --lib claude_code`: **22 passed, 0 failed**; более широкий фильтр `--lib claude`: **63 passed, 0 failed**.
- Subprocess helpers: **21 passed, 0 failed**; cancellation regression доказал завершение прямого дочернего PID.
- Routing: **42 passed**; HTTP config: **3 passed**; default-provider: **1 passed**; runtime-workspace: **1 passed**.
- `cargo fmt --all -- --check`, `cargo check -p forgeplan-core -p forgeplan -p forgeplan-mcp`, `RUSTFLAGS=-Dwarnings cargo clippy --workspace --all-targets`: PASS.
- Codex: реальный `forgeplan reason PROB-001 --json`, `provider: codex-cli`, `model: ""`, без `OPENAI_API_KEY`: **exit 0**, **62.82 s**, **11,037 bytes** ADI JSON.
- Devin Desktop CLI `3000.10.48`: реальный `--help` до релиза выявил несовместимый режим `normal`. Профиль исправлен на безопасный `auto`, где автоматически разрешены только read-only tools; бинарь доступен как `~/.local/bin/devin`.
- После `devin auth login`, `devin auth status` подтвердил подписку **Devin Pro**. Реальный `forgeplan reason PROB-001 --json` с `provider: devin-cli`, `model: ""`: **exit 0**, **93.65 s**, **9,995 bytes**; ответ содержит 4 гипотезы, deductions, evidence_needed, recommendation и confidence. Prompt прошёл через ForgePlan → Devin end-to-end.
- Финальный CI release PR #520 после Devin correction: check/lint/format, full tests + doc-tests, embedding oracle, website, artifact drift/validator и end-to-end smoke — PASS.
- `npm run build` в `website/`: PASS; EN/RU configuration docs собираются.

## Остальные установленные CLI

- Gemini CLI реально запущен; локальная установка требует `GEMINI_API_KEY`, а не OAuth. ForgePlan ключ не передал и небезопасный fallback не сделал.
- Cursor Agent реально запущен и требует свежий `cursor-agent login`; fallback на другой provider отсутствует.

## Сырые доказательства

| Файл | Bytes | SHA-256 |
|---|---:|---|
| `/tmp/forgeplan-codex-adi-smoke.out` | 11,037 | `fe7b452553499938941f1c9d0d816a8b10f63dcbf5cc17e86f6bd5b41b0487b7` |
| `/tmp/devin-v038-adi.json` | 9,995 | `8583178ed029a227e17390710a246a04bc64a30891ac68172c8f6f2abd4eeab2` |
| `/tmp/devin-v038-adi.err` | 294 | `ce235f79b24ca5525904fc417adbbbbc7068a042f97da25b43bf98e69c3be9ac` |
| `/tmp/local-cli-tests-round3.log` | 2,000 | `54a44b7775692fb7173795b9082052edf8ff8b8133e727eb39f6b2fcc228ca83` |
| `/tmp/devin-auto-profile-tests.log` | 5,350 | `c3ce6df28a8091a8ea70a923a80a0c6a8e0196dfd125f0cd36f7c8185a6a108e` |
| `/tmp/subprocess-cancellation-regression.log` | 16,172 | `26422539b8634bc66233e279ef1ae4a9665039cdb1e745e98c9b4059a79c1b6d` |
| `/tmp/website-build-local-cli.log` | 28,607 | `52cdf2a0540c5859730b18f80e1e7a6c1ddee8cbe9c1418c1afacb34fb2455f7` |

## Границы

- Положительный Gemini/Cursor subscription smoke не выполнен: их vendor login на workstation не настроен.
- Один локальный полный workspace-test ранее не стартовал из-за заполнения диска и не был засчитан как PASS; финальный GitHub CI выполнил full tests, doc-tests и E2E.
- Проверено на macOS arm64.

## Related

- RFC-014 — реализованный контракт
- SPEC-007 — сценарии и security boundary
- PRD-087 — требования
- ADR-027 — выбранная архитектура

