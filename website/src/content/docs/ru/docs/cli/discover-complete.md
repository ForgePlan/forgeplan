---
title: forgeplan discover complete
description: "Отмечает существующую сессию исследования завершённой и сохраняет время завершения. CLI 0.38.0 печатает количество находок и подсказку запустить health. Саму проверку health и генерацию предложений артефактов не выполняет."
---

Отмечает существующую сессию исследования завершённой и сохраняет время завершения. CLI 0.38.0 печатает количество находок и подсказку запустить health. Саму проверку health и генерацию предложений артефактов не выполняет.

## Использование

```text
forgeplan discover complete <SESSION_ID>
```

## Аргументы

```text
  <SESSION_ID>  Session ID to complete
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Пример

```bash
forgeplan discover list
# Replace SESSION_ID with the actual returned identifier:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Переход к дальнейшей работе

Изучите находки, выберите, для каких нужны Problem, PRD, ADR или Note, и создайте их явно через `new`. Завершение не объединяет дубликаты, не выбирает глубину и не создаёт список задач. Повторное завершение уже закрытой сессии возвращает успех.

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда + обзор конвейера
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - проверка покрытия перед завершением
- [`forgeplan discover list`](/ru/docs/cli/discover-list/) - индекс сессий
- [`forgeplan new`](/ru/docs/cli/new/) - создание предложенных артефактов
- [`forgeplan validate`](/ru/docs/cli/validate/) - следующий шаг после new
- [Руководство по методологии](/ru/docs/methodology/overview/)