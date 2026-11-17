---
title: forgeplan scan
description: "Сканирует исходные модули и печатает пути, количество файлов и строк. Это отчёт без записи постоянного инвентаря .forgeplan/modules.json."
---

Сканирует исходные модули и печатает пути, количество файлов и строк. Это отчёт без записи постоянного инвентаря `.forgeplan/modules.json`.

## Использование

```text
forgeplan scan [OPTIONS]
```

## Опции

```text
      --path <PATH>  Path to project root (default: current dir)
  -h, --help         Print help
  -V, --version      Print version
```

## Примеры

```bash
forgeplan scan
forgeplan scan --path crates
forgeplan coverage
forgeplan drift
```

## Ограничения путей

`--path` должен разрешаться внутри текущего корня проекта. Соседний путь вроде `../other-project` отклоняется; для другого проекта явно перейдите в него. Путь должен существовать и быть доступен.

## Связь с coverage

`coverage` сам сканирует модули, поэтому предварительный `scan` необязателен. Используйте его для изучения результатов после рефакторинга, затем свяжите реальные пути с решениями. `scan-import` — отдельная операция импорта внешних документов.

## См. также

- [`forgeplan coverage`](/ru/docs/cli/coverage/) - покрытие решений по модулям
- [`forgeplan drift`](/ru/docs/cli/drift/) - решения, чей код был перемещён
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - сканирует проекции пространства: новые записи, синхронизация тел, недостающие связи и очистка сирот; не полное обновление метаданных и удалённых связей существующих строк
- [`forgeplan health`](/ru/docs/cli/health/) - агрегирует coverage в дашборд
- [Обзор CLI](/ru/docs/cli/)
