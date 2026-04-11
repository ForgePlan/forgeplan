---
title: forgeplan_generate
description: "Генерирует полный текст артефакта из описания на естественном языке, используя настроенный провайдер LLM."
---

Создает новый артефакт **с полностью сгенерированным текстом** из описания на естественном языке. В отличие от `forgeplan_new` (который создает пустой шаблон-заглушку), `generate` использует настроенный LLM (OpenAI / Claude / Gemini / Ollama / любую совместимую с OpenAI конечную точку) для создания всех необходимых разделов за один вызов. Агент все равно должен запустить `forgeplan_validate` после этого, поскольку LLM может пропустить обязательные правила (MUST rules).

**Категория**: Создание артефактов

## Когда агент вызывает это

- Пользователь предоставляет подробное описание и хочет получить готовый к ревью первый черновик, а не пустую заглушку.
- Миграция решения из истории чата в формальный ADR — используйте сводку чата в качестве описания.
- Массовая инициализация: превращение неформальной дорожной карты в 5 черновиков PRD за одну сессию.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | yes | Тип артефакта: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`. |
| `description` | `string` | yes | Описание на естественном языке того, что нужно сгенерировать. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::GenerateParams`_

## Возвращает

Идентификатор нового артефакта плюс сгенерированный текст. В отличие от `forgeplan_new`, агент обычно может сразу перейти к `forgeplan_validate` без промежуточного `forgeplan_update` — текст уже заполнен.

Пример структуры ответа:

```json
{
  "id": "PRD-043",
  "kind": "prd",
  "status": "draft",
  "path": ".forgeplan/prds/prd-043-oauth2-login.md",
  "body": "# PRD-043: OAuth2 login flow\n\n## Problem\n...",
  "llm": { "provider": "gemini", "model": "gemini-3-flash-preview", "tokens": 1847 }
}
```

## Пример вызова

```json
{ "kind": "prd", "description": "OAuth2 login flow" }
```

В типичном контексте агента:

> Пользователь вставляет описание функции в один абзац. Агент генерирует полный черновик PRD вместо пустой заглушки.

```json
{ "kind": "prd", "description": "Add OAuth2 login with Google and GitHub, support PKCE, 15m token TTL." }
```

## Типичная последовательность

`forgeplan_route` (подтверждение глубины) → `forgeplan_search` (проверка на дубликаты) → `forgeplan_generate` → `forgeplan_validate` → (устранение пробелов через `forgeplan_update`) → `forgeplan_reason` (для Standard+) → `forgeplan_activate`.

## Эквивалент CLI

- [`forgeplan generate`](/docs/cli/generate/) — та же операция

## См. также

- [Обзор MCP](/docs/mcp/)
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — альтернатива с пустой заглушкой
- [`forgeplan_capture`](/docs/mcp/forgeplan_capture/) — захват решений из беседы
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — всегда валидировать сгенерированный вывод
