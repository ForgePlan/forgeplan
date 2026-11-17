---
title: forgeplan promote
description: "Преобразует локальную запись памяти в новый артефакт явно выбранного типа. Создаёт черновик Tactical с текстом в Content и отметкой происхождения, затем удаляет исходную запись и её связи."
---

Преобразует локальную запись памяти в новый артефакт явно выбранного типа. Создаёт черновик Tactical с текстом в Content и отметкой происхождения, затем удаляет исходную запись и её связи.

## Использование

```text
forgeplan promote --kind <KIND> <MEMORY_ID>
```

## Аргументы

```text
  <MEMORY_ID>  Memory ID to promote (e.g., mem-auth-decisions)
```

## Опции

```text
      --kind <KIND>  Target artifact kind: prd, rfc, adr, note, problem, etc
  -h, --help         Print help
  -V, --version      Print version
```

## Пример

```bash
forgeplan remember --list
# Replace MEMORY_ID with an actual mem-... identifier:
forgeplan promote MEMORY_ID --kind note
# Read the returned artifact ID and fill its required sections:
forgeplan list --type note
```

## Что сохраняется

Переносятся заголовок, текст, автор и теги. Полный шаблон PRD/RFC/ADR не заполняется, исходная запись не сохраняется автоматически как связанное доказательство. Изучите черновик, добавьте обязательные разделы и осмысленные связи, выполните валидацию перед активацией. Продвижение в тип memory отклоняется.

## Выбор операции

Без существующей записи используйте `new`, для фиксации решения с LLM — `capture`; если артефакт не нужен, оставьте запись памятью. У записей `remember` нет автоматического срока действия. Эти команды работают с памятью ядра, а не с банками Hindsight.

## См. также

- [`forgeplan new`](/ru/docs/cli/new/) - создать артефакт с нуля без памяти
- [`forgeplan generate`](/ru/docs/cli/generate/) - артефакт, созданный LLM по подсказке на естественном языке
- [`forgeplan capture`](/ru/docs/cli/capture/) - зафиксировать текущее решение без обратного пути через память
- [`forgeplan validate`](/ru/docs/cli/validate/) - требуется перед активацией артефакта с повышенным статусом
- [Методология: модель артефактов](/ru/docs/methodology/overview/)
