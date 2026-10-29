---
title: forgeplan session
description: "Показать текущее состояние сессии методологии и следующий шаг"
---

Показывает сохранённые фазу сессии, активный артефакт, глубину, режим enforcement и рекомендательные шаги. Фазы: `idle`, `routing`, `shaping`, `coding`, `evidence`, `pr`. Validate, reason и activate - действия процесса, не дополнительные сохранённые состояния сессии.

## Когда использовать

- Возобновить работу после перерыва - «что я делал?»
- AI-агенты между вызовами инструментов - для принятия решения о следующем действии
- Принудительное соблюдение протокола - чтобы случайно не пропустить ни одну фазу
- Многоагентные системы - супервизор считывает состояние сессии для роутинга работы

## Когда не использовать

- Вы хотите список задач → используйте `TODO.md` или ваш трекер задач
- Для состояния проекта есть [`forgeplan health`](/ru/docs/cli/health/).

## Использование

```text
forgeplan session [OPTIONS]
```

## Опции

```text
      --reset    Сбросить сессию в состояние Idle
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

Показать текущую фазу:

```bash
forgeplan session
```

Сбросить состояние после прерывания задачи:

```bash
forgeplan session --reset
```

## Интерпретация вывода

Иллюстрация shaping-сессии, не точный записанный вывод:

```
Session: shaping
  Active artifact:  PRD-046
  Depth:            Standard
  Next step:        forgeplan validate PRD-046
  Last transition:  2026-04-11 14:05 (routing → shaping)
```

Сохранённые фазы:

| Фаза | Смысл |
|---|---|
| `idle` | Нет активной задачи сессии |
| `routing` | Выбор глубины и конвейера |
| `shaping` | Формирование и валидация решения |
| `coding` | Реализация |
| `evidence` | Сбор и ревью доказательств |
| `pr` | Подготовка pull request |

Файл состояния - `.forgeplan/session.yaml`. `--reset` очищает фазу и поля активной сессии; не валидирует артефакты, не устраняет задолженность и не проходит гейты.

## Как это работает

Фаза сессии и lifecycle-фаза артефакта различаются. Применимые validate → reason → code → evidence → review → activate остаются обязательными, но не каждая успешная команда двигает сессию. Перед исправлением устаревшего состояния проверьте артефакт и настоящую работу; reset не должен обходить гейт.

## См. также

- [`forgeplan route`](/ru/docs/cli/route/) - глубина и конвейер
- [`forgeplan health`](/ru/docs/cli/health/) - задолженность
- [Рабочий процесс](/ru/docs/guides/git-workflow/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan session --help`.

```text
Show methodology session state (current phase, active artifact)

Usage: forgeplan session [OPTIONS]

Options:
      --reset    Reset session to Idle
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
