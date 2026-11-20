---
title: forgeplan discover complete
description: "Помечает сессию исследования завершённой и рекомендует проверку состояния."
---

`forgeplan discover complete <SESSION_ID>` помечает сохранённую сессию завершённой. CLI печатает число находок, время завершения и подсказку `forgeplan health`. Она не группирует и не объединяет находки, не выбирает глубину артефактов и не выдаёт команды `forgeplan new`.

Последующий разбор и выбор артефактов выполняются отдельно.

## Когда использовать

- **После согласованного обзора исходников** — изучите `discover show`; число находок не доказывает достаточное покрытие.
- **В конце спринта по онбордингу** - кристаллизуйте всё изученное в артефакты.
- **При остановке частичного обзора** — завершение фиксирует состояние, но не подтверждает полноту исследования.

## Когда НЕ использовать

- Пока агент всё ещё активно отправляет результаты - дождитесь, пока он завершит работу.
- Для отката находок — завершение не удаляет находки и не отменяет уже созданные артефакты.
- Для уже завершённой сессии - идемпотентно, но бессмысленно.

## Использование

```text
forgeplan discover complete <SESSION_ID>
```

## Аргументы

```text
  <SESSION_ID>   Идентификатор сессии обнаружения (из discover list / start)
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Замените SESSION_ID значением из start/list
forgeplan discover complete SESSION_ID

# Полный цикл онбординга
forgeplan init -y
forgeplan discover start "Пример проекта"
# Агент читает источники и сообщает находки через MCP
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
# Отдельные решения оператора, не автоматически выданные предложения:
forgeplan new problem "Несоответствие аутентификации между документацией и кодом"
forgeplan new prd  "Унифицированный загрузчик конфигурации"
```

## Разбор находок после завершения

Ниже предлагается работа человека или агента, а не поведение этой CLI-команды:

1. Сгруппируйте находки по категориям и источникам.
2. Проверьте дублирование и противоречия по одному модулю.
3. Решите, какие находки обосновывают Problem, ADR, PRD или Note.
4. Выберите глубину по действующим правилам маршрутизации.
5. Явно создайте принятые артефакты.

CLI завершения не выполняет эти шаги и не создаёт последующие артефакты.

## Как это вписывается

`discover complete` - это **точка передачи** от обнаружения к обычному жизненному циклу Forgeplan:

```
discover start → находки агента → discover complete → ручной разбор
                                                    ↓
                          forgeplan new problem/prd/adr/note ...
                                                    ↓
                          validate → reason → code/evidence → activate
```

Отсюда проект проходит через стандартный цикл Shape → Validate → ADI → Code → Evidence → Activate, как и любая greenfield-работа.

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - обзор сессий
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - находки перед завершением
- [`forgeplan discover list`](/ru/docs/cli/discover-list/) - список сессий
- [`forgeplan new`](/ru/docs/cli/new/) - явное создание выбранных артефактов
- [`forgeplan validate`](/ru/docs/cli/validate/) - следующий шаг после заполнения
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover complete --help`.

```text
Mark a discovery session as completed

Usage: forgeplan discover complete <SESSION_ID>

Arguments:
  <SESSION_ID>  Session ID to complete

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
