---
title: forgeplan activate
description: "Переводит черновик артефакта в активное состояние через гейт валидации - момент, когда решение становится действующим."
---

`forgeplan activate` переводит черновик в `active`. Для PRD, RFC, ADR, Epic и Spec проверяются правила MUST и методологические условия: длина тела, связь с доказательством и незаполненные части шаблона. `Note` и `Problem` пропускают эти методологические проверки, но подчиняются правилам переходов статуса. Положительный R_eff остаётся требованием рабочего процесса; сама проверка связи не доказывает качество EvidencePack.

## Когда использовать

- Вы только что завершили реализацию PRD: все FR отмечены как выполненные, EvidencePack связан, и `forgeplan score PRD-001` возвращает `R_eff > 0`.
- ADR был рассмотрен командой, и решение окончательное - пришло время сделать его авторитетным.
- RFC прошел гейт верификации, и его этапы реализации начали производить доказательства.

## Когда НЕ использовать

- У артефакта все еще есть ошибки валидации MUST - сначала исправьте их с помощью `forgeplan validate <id>` вместо того, чтобы использовать `--force`.
- За решением еще нет кода, тестов или измерений. Активация без доказательств создает слепое пятно в `forgeplan health`.

## Использование

```text
forgeplan activate [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  ID артефакта
```

## Опции

```text
      --force    Принудительная активация, даже если валидация содержит ошибки MUST
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Активация полностью валидированного PRD

```bash
forgeplan validate PRD-001
forgeplan score PRD-001
forgeplan activate PRD-001
```

Сначала запустите `validate` и `score`, чтобы убедиться, что гейт MUST пройден и R_eff не равен нулю, затем активируйте.

### Пример 2: Активация ADR после ревью

```bash
forgeplan review ADR-005
forgeplan activate ADR-005
```

`review` выполняет общие проверки валидации и методологических условий без изменения состояния. Активация дополнительно проверяет переход статуса, а в legacy-пути CLI — настроенную Git-delta provenance. Успешное ревью не гарантирует успешную активацию.

### Пример 3: Принудительная активация, несмотря на ошибки валидации

```bash
forgeplan activate RFC-012 --force
```

Используйте только в исключительных случаях (например, при миграции существующих артефактов). Предпочтительнее исправлять недочеты.

## Как это вписывается в рабочий процесс

Эта команда входит в [полный жизненный цикл артефакта](/ru/docs/guides/first-artifact/). Она отличает черновую работу от действующих решений.

## Распространенные ошибки

| Ошибка | Причина | Исправление |
|---|---|---|
| Нарушение MUST | Обязательное поле или раздел пуст | Обновите тело через `forgeplan update <id> --body @body.md`, затем повторите валидацию |
| Недопустимый переход из `active` | Артефакт уже активирован | Проверьте `forgeplan get <id>`; повторная активация не является успешным no-op |
| `R_eff = 0 after activation` | Доказательства могут отсутствовать или опровергать решение | Изучите пакеты и поля перед добавлением доказательств или пересчётом |
| `Cannot activate from terminal state` | Артефакт `superseded` или `deprecated` | Прямая активация отклоняется. Создайте новый черновик либо используйте recovery через receipt: `restore`/`undo-last`. |

## См. также

- [`forgeplan validate`](/ru/docs/cli/validate/) - валидация без изменения состояния
- [`forgeplan score`](/ru/docs/cli/score/) - проверка R_eff
- [`forgeplan review`](/ru/docs/cli/review/) - ревью перед активацией
- [`forgeplan supersede`](/ru/docs/cli/supersede/) - замещение решения
- [`forgeplan deprecate`](/ru/docs/cli/deprecate/) - вывод из эксплуатации без замены
- [Жизненный цикл v2](/ru/docs/guides/lifecycle-v2/)
- [Методология: жизненный цикл](/ru/docs/methodology/lifecycle/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan activate --help`.

```text
Activate an artifact (draft → active) with validation gate

Usage: forgeplan activate [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --force    Force activation even if validation has MUST errors
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
