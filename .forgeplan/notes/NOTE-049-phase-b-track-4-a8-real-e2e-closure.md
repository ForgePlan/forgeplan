---
depth: standard
id: NOTE-049
kind: note
last_modified_at: 2026-05-03T07:23:38.336155+00:00
last_modified_by: claude-code/2.1.126
links:
- target: ADR-011
  relation: informs
- target: EVID-096
  relation: informs
- target: PROB-050
  relation: informs
status: draft
title: Phase B + Track 4-A8 real E2E closure
---

# NOTE-049: Phase B + Track 4-A8 real E2E closure

| Field | Value |
|-------|-------|
| Status | Draft (until activate after EVID-097) |
| Created | 2026-05-03 |
| Valid Until | 2026-08-01 |
| Context | verification, real-e2e, phase-b, track-4a8 |

## Note

Закрытие двух real-E2E gap-ов, унаследованных от sprint-а 2026-05-02:

1. **Phase B Wave 1 (PROB-050 A-3)** — `PluginDispatcher` и `AgentDispatcher` через `claude --print` имеют 100% покрытие unit + integration тестами на FAKE bash-скриптах. Реальный `claude --print --agent <name>` end-to-end **никогда не вызывался** из dispatcher-кода. Без реального запуска мы не можем утверждать, что argv shape, JSON envelope decoding, `--add-dir` propagation и argv injection guard действительно работают на production-binary `claude` 2.1.126.
2. **Track 4-A8 playbooks** — `release.yaml` и `brownfield-docs.yaml` прошли `forgeplan playbook validate` (schema PASS), но `forgeplan playbook run` ни в `--dry-run`, ни в `--yes` режиме не запускался. Smoke-тесты покрывают только schema-уровень.

Цель note — задокументировать реальный запуск обоих surface-ов на свежем workspace + capture raw output в `docs/operations/phase-b-real-e2e-2026-05-03.md`. Любой surface-bug, найденный при этом, рассматривается как CRITICAL fix и шиппится в этом же PR до merge.

Auto-expires 2026-08-01 (handoff-bounded verification, не feature).

## Hypotheses (ADI seed, handoff-specific)

| ID | Hypothesis | Risk | Test |
|----|-----------|------|------|
| H1 | `claude --print` argv shape работает end-to-end (spike validated) | Low | Запустить минимальный playbook с `Delegation::Agent`, проверить exit code + JSON envelope |
| H2 | argv ordering bug surface только при наличии и `--add-dir`, и `--allowedTools` (R1 fix reorder) | Medium | Playbook с `produces_at` + `allowed_tools`, проверить порядок в ps/strace |
| H3 | `release.yaml` placeholder substitution fails (нет template engine — manual edit) | Medium | `forgeplan playbook run release --dry-run`, ожидать explicit error либо документировать REFERENCE-only статус |
| H4 | `brownfield-docs.yaml` падает на missing skill/mapping graceful, не panic | Low | Запустить с пустым workspace, ожидать `DispatchError::DelegateMissing`, не abort |
| H5 | argv injection guard rejects malicious agent name WITHOUT spawning claude | High | Test playbook с `name: "../../etc/passwd"`, verify reject path не вызывает subprocess |

Дополнительный gemini-3-flash-preview ADI hypotheses в `forgeplan reason NOTE-049` журнале (general structural concerns, не surface-specific).

## Related

| Artifact | Relation |
|----------|----------|
| ADR-011 | informs |
| EVID-096 | informs |
| PROB-050 | informs |




