---
title: forgeplan fpf list
description: "Список разделов FPF: ID, число строк и заголовок."
---

`forgeplan fpf list` показывает ID раздела, число строк и заголовок.

## Когда использовать

- **Для ориентации** - просмотреть всю структуру FPF на одном экране.
- **После `fpf ingest`** - сравнить список с установленной ревизией корпуса.
- **Чтобы выбрать ID раздела для чтения** перед запуском [`fpf section`](/docs/cli/fpf-section/).
- **Для проверки полноты охвата**, когда поиск возвращает удивительно мало результатов.

## Когда НЕ использовать

- Для поиска содержимого - используйте [`fpf search`](/docs/cli/fpf-search/).
- Для чтения одного раздела - используйте [`fpf section <id>`](/docs/cli/fpf-section/).
- Для проверки состояния / просроченности KB - используйте [`fpf status`](/docs/cli/fpf-status/).

## Использование

```text
forgeplan fpf list [OPTIONS]
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Полная таблица
forgeplan fpf list

# Типичный сценарий просмотра
forgeplan fpf list
forgeplan fpf section B.3
forgeplan fpf search "trust"
```

## Структура корпуса FPF

Корпус берётся из установленного скилла или `fpf ingest --path`. Проверяйте
заголовки выбранной ревизии, не предполагайте три части и ровно 204 раздела.
В текущем upstream FPF B.3 посвящён trust/assurance, B.5 - циклу ADI.
Метрика ForgePlan F-G-R является отдельной адаптацией.

## Как это работает

`fpf list` - это точка входа для обнаружения: просканируйте таблицу, найдите интересный ID, затем перейдите к [`fpf section`](/docs/cli/fpf-section/) или [`fpf search`](/docs/cli/fpf-search/) для более глубокого чтения.

## Смотрите также

- [`forgeplan fpf`](/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - сначала заполните KB
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - прочитать один раздел
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - найти разделы по содержимому
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - состояние загрузки

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf list --help`.

```text
List all FPF sections

Usage: forgeplan fpf list

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
