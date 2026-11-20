---
title: forgeplan fpf section
description: "Показать конкретный раздел FPF по ID (например, B.3 для Trust Calculus)"
---

`forgeplan fpf section <ID>` выводит полный текст одного раздела **First Principles Framework** по его каноническому ID (например, `B.3` или `A.1.2`). Это `less` для базы знаний FPF - используйте его, когда результат поиска интересен и вы хотите прочитать его полностью.

## Когда использовать

- **После многообещающего результата `fpf search`** - прочитайте раздел целиком, а не только фрагмент.
- **Когда документ по методологии или вывод `forgeplan reason` ссылается на FPF ID** - перейдите прямо к источнику.
- **При написании ADRs или RFCs** - цитируйте раздел, на который вы ссылаетесь, дословно.

## Когда НЕ использовать

- Для поиска без известного ID — [`forgeplan fpf search`](/ru/docs/cli/fpf-search/).
- Для списка разделов — [`forgeplan fpf list`](/ru/docs/cli/fpf-list/).

## Использование

```text
forgeplan fpf section [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>   ID раздела (например, "B.3", "C.2.2")
```

## Опции

```text
      --summary  Показать только краткое содержание (первые 500 символов)
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Прочитайте Trust Calculus загруженной ревизии; отличайте его от адаптации R_eff в ForgePlan
forgeplan fpf section B.3

# Только первые 500 символов, если вам нужна только суть
forgeplan fpf section B.3 --summary

# Другой раздел по ID; проверьте заголовок в загруженной ревизии
forgeplan fpf section B.4

# Abduction–Deduction–Induction (ADI), FPF B.5
forgeplan fpf section B.5
```

## Как это работает

Разделы извлекаются через `fpf search`, загружаются через `fpf ingest` и цитируются правилами FPF. ADI описан в B.5. `forgeplan reason --fpf` подставляет подходящие разделы FPF в промпт ADI, но на мигрированном workspace legacy-корпус FPF выведен из эксплуатации, поэтому этот путь сообщает, что разделов FPF нет, и не открывает выведенное хранилище. R_eff ForgePlan — адаптация продукта, не арифметика, предписанная текущим upstream B.3.

Типичный рабочий процесс:

```bash
forgeplan fpf search "congruence level"   # найти кандидатов
forgeplan fpf section B.3                 # прочитать победителя полностью
forgeplan new adr "Evidence grading policy"
# ...сослаться на B.3 в тексте ADR
```

## Смотрите также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - поиск разделов
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) - список разделов
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf section --help`.

```text
Show a specific FPF section

Usage: forgeplan fpf section [OPTIONS] <ID>

Arguments:
  <ID>  Section ID (e.g. "B.3", "C.2.2")

Options:
      --summary  Show summary only (first 500 chars)
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
