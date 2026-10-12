---
depth: standard
id: ADR-027
kind: adr
last_modified_at: 2026-10-12T11:26:10.811737+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-014
  relation: based_on
status: active
title: Use closed local CLI profiles over copied runners or command templates
---

---
assigned_number: 27
created: 2026-10-12
predicted_number: 27
problem_ref: null
slug: adr-use-closed-local-cli-profiles-over-copied-runners-or-command-templates
updated: 2026-10-12
---

## Context

ForgePlan уже запускает `claude --print` для `provider: claude-code`. Запрос расширился до Codex и Devin, а установленные Gemini CLI и Cursor Agent имеют тот же полезный признак: официальный headless режим и локальная subscription/OAuth авторизация. Конфликт: поддержка должна расти без четырёх копий опасной subprocess-механики, но универсальная команда из YAML сделала бы конфигурацию исполняемым кодом и сломала бы гарантии безопасности.

## Decision

Использовать **закрытый enum локальных CLI-провайдеров и один общий subprocess runner**. Каждый поддержанный vendor имеет профиль, скомпилированный в ForgePlan: stock binary name, фиксированные безопасные flags, transport prompt, parser final response и login hint. Пользователь выбирает только provider/model; binary и argv template не конфигурируются.

Первая группа: `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`. Существующий `claude-code` сохраняет публичный config/result contract, переводит repository-controlled system instruction из argv в framed stdin и также переносится на общий runner и общий sentinel, чтобы разные local profiles не могли рекурсивно запускать друг друга.

Local CLI providers:

- только явный opt-in, без auto-detection;
- keyless для ForgePlan, но расходуют vendor plan и подчиняются vendor terms;
- personal/local only; не для shared/CI/production;
- запускаются без shell, с очищенным окружением, bounded output с явным truncation, timeout на весь post-spawn lifecycle и общим recursion guard; четыре новых профиля используют изолированный cwd, а `claude-code` сохраняет caller cwd;
- используют наиболее строгий официальный no-write режим. Для Devin, где строгого read-only режима нет, ограничение является частью публичного контракта.

## Considered Options

1. **Copy-paste vendor methods.** Быстрее для одного CLI, но security fixes расходятся. Отклонено.
2. **Arbitrary command template.** Максимальная расширяемость, но command injection и непроверяемые flags. Отклонено.
3. **OpenAI-compatible proxy around subscription tokens.** Простая HTTP-интеграция, но требует неофициального token bridge/client spoofing. Отклонено.
4. **Closed profiles + shared runner.** Чуть больше кода профиля, но одна проверяемая subprocess boundary. Выбрано.

## Consequences

### Positive

- Codex/Devin/Gemini/Cursor login можно использовать для ADI без отдельного API key.
- Timeout, env, output cap, recursion и binary trust исправляются в одном месте.
- Новый CLI добавляется как явный reviewable profile, а не как новая execution primitive.
- Неверный vendor output падает явно и не маскируется fallback-парсером.

### Negative

- ForgePlan зависит от стабильности vendor CLI flags/output и должен сопровождать каждый профиль.
- Реальная семантика «без записи» неодинакова; Devin остаётся слабее остальных и отклоняется на Windows, где stock CLI не поддерживает обязательный sandbox.
- Runner сейчас гарантирует остановку direct child, но не всего descendant process tree; process-group/Windows Job Object hardening вынесен в GitHub issue #513.
- Подписка и headless quota могут измениться независимо от ForgePlan.
- Закрытый enum требует релиза ForgePlan для нового vendor. Это намеренная цена за безопасность.

### Revisit Triggers

- Vendor выпускает официальный completion SDK/API, который переиспользует локальную авторизацию без agent tools.
- Два и более профиля невозможно выразить общим runner без provider-specific lifecycle forks.
- CLI удаляет безопасный non-interactive/no-write режим или запрещает third-party invocation условиями использования.
- Практический спрос на custom CLI превышает риск command-template; тогда нужен отдельный sandboxed plugin protocol, не строка shell.

## Validation

SPEC-007 scenarios, mock-binary contract tests каждого профиля, реальный one-prompt smoke для каждого установленного и аутентифицированного CLI, отсутствие регрессий `claude-code`/HTTP и security review subprocess boundary.

## Related Artifacts

- PRD-087
- SPEC-007
- RFC-014
- ADR-017 (precedent; not superseded)
