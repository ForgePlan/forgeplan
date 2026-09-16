---
title: forgeplan fpf list
description: "Вывести список всех загруженных секций FPF в виде таблицы (ID, часть, заголовок)"
---

`forgeplan fpf list` выводит каждую секцию из базы знаний **First Principles Framework** в виде таблицы - ID секции, часть (A/B/C) и заголовок. Это представление оглавления базы знаний.

## Когда использовать

- **Для ориентации** - просмотреть всю структуру FPF на одном экране.
- **Сразу после `fpf ingest`** - убедиться, что загружены ожидаемые ~204 секции.
- **Чтобы выбрать ID секции для чтения** перед запуском [`fpf section`](/ru/docs/cli/fpf-section/).
- **Для проверки покрытия**, когда поиск возвращает неожиданно мало результатов.

## Когда НЕ использовать

- Для поиска контента - используйте [`fpf search`](/ru/docs/cli/fpf-search/).
- Для чтения одной секции - используйте [`fpf section <id>`](/ru/docs/cli/fpf-section/).
- Для проверки здоровья / актуальности базы знаний - используйте [`fpf status`](/ru/docs/cli/fpf-status/).

## Использование

```text
forgeplan fpf list
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
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

Секции сгруппированы в три части:

- **Часть A - Основы.** Архитектура ядра, примитивы рассуждений, базовый словарь.
- **Часть B - Trust Calculus.** Оценка доказательств, уровни конгруэнтности, семантика R_eff, explore/investigate/exploit.
- **Часть C - Цикл ADI.** Абдукция → Дедукция → Индукция, управление гипотезами, состязательная ревью.

Всего 204 секции поставляются с Forgeplan и загружаются в базу знаний при выполнении `fpf ingest`.

## Как это вписывается

`fpf list` - это точка входа для обнаружения: просмотрите таблицу, найдите интересующий ID, затем перейдите к [`fpf section`](/ru/docs/cli/fpf-section/) или [`fpf search`](/ru/docs/cli/fpf-search/) для более глубокого изучения.

## См. также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/) - сначала заполните базу знаний
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) - прочитать одну секцию
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - найти секции по содержимому
- [`forgeplan fpf status`](/ru/docs/cli/fpf-status/) - здоровье загрузки