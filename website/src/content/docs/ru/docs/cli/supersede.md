---
title: forgeplan supersede
description: "Выводит активный артефакт из использования в пользу замены, сохраняя историю. Старый артефакт получает статус superseded. Runtime записывает связь старый ID → ID замены с именем supersedes, не superseded_by."
---

Выводит активный артефакт из использования в пользу замены, сохраняя историю. Старый артефакт получает статус superseded. Runtime записывает связь старый ID → ID замены с именем `supersedes`, не `superseded_by`.

## Использование

```text
forgeplan supersede --by <BY> <ID>
```

## Аргументы

```text
  <ID>  Artifact ID to supersede
```

## Опции

```text
      --by <BY>  Replacement artifact ID
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
# Recommended: review and activate the successor first.
forgeplan activate ADR-005
forgeplan supersede ADR-003 --by ADR-005
forgeplan get ADR-003
```

## Проверки и предупреждения

Исходный артефакт должен быть active, замена должна существовать. Черновик замены вызывает предупреждение, не отказ; замена superseded или deprecated отклоняется. Предпочитайте активированного преемника, чтобы не передавать непроверенное решение. Отсутствующая замена отклоняется, даже если обёртка CLI разрешает синтаксис ссылки на будущий ID.

## Доказательства и восстановление

Заменённое доказательство исключается из текущего минимума, оценки зависимых артефактов пересчитываются. Пакет остаётся для истории. Superseded — конечный статус обычного жизненного цикла. Путь CLI supersede в 0.38.0 не создаёт квитанцию восстановления: нельзя обещать отмену этого вызова через `restore` или `undo-last`. Для них нужна существующая подходящая квитанция; undo-last может вместо этого отменить другую операцию. Без замены используйте `deprecate`, для пересмотра с новым черновиком — `reopen`.

## См. также

- [`forgeplan deprecate`](/ru/docs/cli/deprecate/) - вывести из эксплуатации без замены
- [`forgeplan reopen`](/ru/docs/cli/reopen/) - отменить и начать новый черновик для переоценки
- [`forgeplan activate`](/ru/docs/cli/activate/) - активировать преемника перед замещением
- [`forgeplan get`](/ru/docs/cli/get/) - проверить связь `supersedes`
- [Руководство по жизненному циклу v2](/ru/docs/guides/lifecycle-v2/)
- [Методология: Жизненный цикл артефактов](/ru/docs/methodology/lifecycle/)
