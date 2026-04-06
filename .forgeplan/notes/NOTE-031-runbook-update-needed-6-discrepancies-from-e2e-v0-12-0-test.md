---
depth: tactical
id: NOTE-031
kind: note
links:
- target: PROB-018
  relation: informs
status: deprecated
title: Runbook update needed — 6 discrepancies from E2E v0.12.0 test
---

## Расхождения Runbook vs Реальность (E2E 2026-04-01)

Runbook: docs/guides/CLI-TEST-RUNBOOK.md (написан под v0.11.1)
Прогон: v0.12.0

### Нужно обновить:

1. **Версия CLI** — 0.11.1 → 0.12.0 (тест 0.4)
2. **Повторный init** — expected 'Error + exit 1' → actual 'Info + exit 0' (idempotent, тест 1.2)
3. **prd-001 lowercase** — expected 'case-insensitive' → actual 'case-sensitive'. Переклассифицировать: NEG тест (тест 2.19)
4. **--depth critical** — expected 'alias для deep' → actual 'отдельный уровень'. Обновить expected (тест в Wave 2)
5. **capture** — expected 'offline' → actual 'requires LLM'. Перенести в Wave 10 (тесты 7.10-7.11)
6. **scan --path /tmp** — был баг, FIXED в Sprint 2. Обновить expected: теперь exit 1 + 'Path traversal rejected' (тест 7.19)

### Дополнительно:
- Добавить тесты 10.1-10.5 с LLM API key (Wave 10)
- Добавить --semantic тесты с feature flag semantic-search (Wave 6)
- Рассмотреть автоматизацию: bash scripts из runbook → CI (NOTE-026)



