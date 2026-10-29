---
title: forgeplan gaps
description: "Находит пробелы в соответствии конвейера - артефакты уровня Deep/Critical, у которых отсутствуют необходимые последующие этапы."
---

Наличие команды в help само по себе не подтверждает её работу на всех раскладках workspace.

`forgeplan gaps` проверяет типы связанных артефактов для PRD, а также отсутствие/слабость evidence активных решений, старые черновики и сиротские артефакты. Учитываются связи в обоих направлениях; полный порядок конвейера и минимальное ветвление Epic не проверяются.

Это не ошибка валидации (сам артефакт может быть в порядке) - это проверка **соответствия методологии**: остановились ли вы на полпути?

## Когда использовать

- В середине спринта: "Я сказал, что это Deep, действительно ли я создал Spec + ADR?"
- После ревью предложения `calibrate` и осознанного изменения depth - увидеть недостающее.
- Проверка состояния перед релизом: "соответствует ли проект собственной методологии?"
- Очистка brownfield-проекта: импортированные артефакты часто имеют глубину без последующей цепочки.

## Когда НЕ использовать

- Для построения порядка выполнения: отчёт проверяет наличие связей, не порядок. Даже Tactical может иметь находки по evidence, возрасту или отсутствию связей.
- В качестве блокиратора для активации - пробелы носят рекомендательный характер, `validate` является гейтом.

## Использование

```text
forgeplan gaps
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Просмотр всех пробелов конвейера

```bash
forgeplan gaps
```

Иллюстрация конвейера, не записанный вывод команды:

```text
Pipeline compliance gaps
─────────────────────────
Deep PRDs without linked Spec:
  PRD-004  Search Intelligence   (depth=Deep, 0 Specs, 1 ADR)
  PRD-011  FPF KB vector search  (depth=Deep, 0 Specs, 0 ADRs)

Standard PRD without linked RFC:
  PRD-018  OpenSpec integration  (no linked RFC)

Orphan artifact:
  PRD-022  Import design  (no links)
```

### Исправление пробелов

Для каждой строки:

1. **Отсутствует Spec для Deep PRD** → `forgeplan new spec "API for PRD-004"` и свяжите.
2. **Отсутствует ADR для Deep решения** → запустите `forgeplan reason`, затем `forgeplan new adr`.
3. **Нет RFC у Standard PRD** → создайте и свяжите подходящий RFC.
4. **Сирота** → добавьте обоснованные связи; снижение глубины не устраняет отсутствие связей.

## Интерпретация вывода

| Записанная глубина PRD | Проверки обработчика |
|---|---|
| Tactical / Note | Нет depth-specific проверки конвейера |
| Standard | Отсутствие RFC — MUST |
| Deep | Отсутствие RFC — MUST; Spec или ADR — SHOULD |
| Буквальное legacy-значение `critical` | Отсутствие parent Epic, RFC или ADR — MUST; минимальное ветвление не проверяется |

Пробел указывает на недостающие ожидаемые артефакты. Создайте их или пересмотрите обоснование глубины; `calibrate` лишь предлагает её, а не меняет.

## Как это вписывается в рабочий процесс

```
calibrate → (depth may change) → gaps → create missing downstream → validate → activate
```

`calibrate` предлагает глубину, `gaps` сравнивает записанную глубину и связи. Ни одна команда не заменяет ревью решения или доказательств.

## См. также

- [`forgeplan calibrate`](/docs/cli/calibrate/) - перекалибровка глубины при появлении пробелов
- [`forgeplan health`](/docs/cli/health/) - агрегированные данные на уровне проекта
- [`forgeplan blocked`](/docs/cli/blocked/) - представление графа зависимостей
- [Модель артефактов](/docs/methodology/overview/) - сопоставление глубины с конвейером
- [Обзор CLI](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan gaps --help`.

```text
Show pipeline compliance gaps by depth

Usage: forgeplan gaps

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
