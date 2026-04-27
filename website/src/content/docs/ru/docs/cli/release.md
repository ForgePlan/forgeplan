---
title: forgeplan release
description: "Release an active claim — drop the lock so other sub-agents can pick up the artifact. Idempotent; missing claim is a no-op."
---

`forgeplan release` снимает клейм — артефакт возвращается в пул кандидатов, и следующий [`forgeplan dispatch`](/ru/docs/cli/dispatch/) сможет передать его другому агенту. Команда удаляет файл клейма по пути `.forgeplan/claims/<id>.yaml`.

По умолчанию команда откажет, если клейм держит другой агент — освободить можно только свою работу. Чтобы перебить ограничение (например, после краха саб-агента, который уже не работает), передайте `--force`. Вызов release на артефакте без активного клейма — это no-op (нет ошибки, ничего не происходит) — поэтому скрипты очистки могут запускаться без предварительной проверки.

Аналог [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) на MCP-стороне.

## Когда использовать

- Агент закончил артефакт — освобождайте, чтобы следующий раунд диспатча передал его кому-то ещё.
- Агент упал или завис — оркестратор запускает `release --force`, чтобы освободить слот.
- Агент по ошибке схватил не тот ID — освобождайте сразу и пробуйте заново.
- Cleanup в конце сессии — пройдитесь по активным клеймам и снимите каждый перед выходом.

## Когда НЕ использовать

- Чтобы удалить сам артефакт — release снимает только клейм. Для удаления артефакта используйте [`forgeplan delete`](/ru/docs/cli/delete/).
- Чтобы сократить TTL клейма — release снимает клейм полностью. Для нового TTL просто вызовите [`forgeplan claim`](/ru/docs/cli/claim/) ещё раз с новым значением (для держателя это идемпотентно).
- Чтобы освободить клейм упавшего агента без `--force` — команда откажет, потому что идентичность не совпадает.

## Использование

```text
forgeplan release [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID to release
```

## Опции

```text
      --agent <AGENT>  Agent identity. Defaults to `cli/<version>` (or empty when --force)
      --force          Force-release regardless of holder (orchestrator escape hatch)
      --json           Output as JSON for machine consumption
  -h, --help           Print help
  -V, --version        Print version
```

## Примеры

### Пример 1: Агент освобождает после завершения

```bash
forgeplan release PRD-057
```

Снимает клейм под дефолтной идентичностью `cli/<version>`. Повторный вызов на уже освобождённом артефакте — no-op (без ошибки).

### Пример 2: Оркестратор подбирает за упавшим агентом

```bash
forgeplan release RFC-012 --force
```

Путь обхода, когда агент умер, но его клейм ещё не истёк. Используйте только из оркестратора — саб-агенты никогда не должны принудительно снимать чужие клеймы.

### Пример 3: Явная идентичность для shell-скрипта оркестратора

```bash
forgeplan release SPEC-018 --agent worker-2
```

Когда shell-скрипту нужно освободить клейм от имени конкретного агента, передайте `--agent` явно. Без `--force` идентичность должна совпадать с текущим держателем, иначе команда откажет.

## Место в рабочем процессе

Замыкает multi-agent цикл: `dispatch` → `claim` → работа → **`release`** → снова `dispatch`. После release слот возвращается в пул кандидатов, и следующий вызов [`forgeplan dispatch`](/ru/docs/cli/dispatch/) сможет передать артефакт другому агенту.

## См. также

- [`forgeplan_release`](/ru/docs/mcp/forgeplan_release/) — MCP-эквивалент
- [`forgeplan claim`](/ru/docs/cli/claim/) — взять клейм
- [`forgeplan claims`](/ru/docs/cli/claims/) — посмотреть, кто что держит
- [`forgeplan dispatch`](/ru/docs/cli/dispatch/) — пере-планировать после release
