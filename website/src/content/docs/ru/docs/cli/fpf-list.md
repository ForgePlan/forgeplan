---
title: forgeplan fpf list
description: "Показывает загруженные разделы FPF в таблице со столбцами Section, Lines и Title."
---

`forgeplan fpf list` выводит разделы, загруженные в базу знаний **First Principles Framework** рабочего пространства. Столбцы — `Section`, `Lines`, `Title`, затем число разделов. Столбца части и флага JSON нет.

## Когда использовать

- **Для ориентации** - просмотреть всю структуру FPF на одном экране.
- **Сразу после `fpf ingest`** — сравнить число загруженных разделов с выбранным корпусом.
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
# Full table
forgeplan fpf list

# Typical browse flow
forgeplan fpf list
forgeplan fpf section B.3
forgeplan fpf search "trust"
```

## Структура корпуса FPF

Корпус берётся из установленного навыка FPF или явно указанного каталога
`fpf ingest --path`, а не из фиксированного набора 204 разделов внутри CLI.
ID и заголовки извлекаются из заголовков исходников или имён файлов.
Копируйте реальные ID из `list`: состав частей и число разделов зависят от корпуса.

Пустая база — успешный результат с сообщением о необходимости сначала запустить
`forgeplan fpf ingest`.
После заполненной таблицы подсказка указывает на её первый раздел.

## Как это вписывается

`fpf list` - это точка входа для обнаружения: просмотрите таблицу, найдите интересующий ID, затем перейдите к [`fpf section`](/ru/docs/cli/fpf-section/) или [`fpf search`](/ru/docs/cli/fpf-search/) для более глубокого изучения.

## См. также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/) - сначала заполните базу знаний
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) - прочитать одну секцию
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - найти секции по содержимому
- [`forgeplan fpf status`](/ru/docs/cli/fpf-status/) - здоровье загрузки