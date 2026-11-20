---
title: forgeplan generate
description: "Генерирует артефакт с помощью ИИ из описания на естественном языке"
---

Генерирует **черновик артефакта с помощью LLM** из описания на естественном языке, используя провайдера из `.forgeplan/config.yaml`. В отличие от [`forgeplan new`](/ru/docs/cli/new/), просит модель заполнить разделы шаблона. Полнота, точность и экономия времени не гарантированы: результат нужно проверить и валидировать.

## Когда использовать

- Вы знаете, что хотите, и предпочли бы просмотреть черновик LLM, чем набирать шаблонный текст самостоятельно.
- **Прототип артефакта для обсуждения**: сгенерируйте одноразовый PRD, чтобы изучить полусформированную идею с командой, прежде чем приступать к созданию настоящего.
- **Исследование существующей системы (brown-field)**: опишите существующую подсистему и попросите модель реконструировать PRD / Spec / RFC для документации.
- У вас есть четкая однострочная задача («добавить ограничение скорости для /api/v1/») и вы хотите, чтобы все разделы были заполнены согласованно.
- Быстрая итерация на этапе Shape, когда вы хотите быстро попробовать 2–3 варианта формулировки.

## Когда НЕ использовать

- Провайдер LLM не настроен - используйте [руководство конфигурации](/ru/docs/getting-started/configuration/) или [`forgeplan new`](/ru/docs/cli/new/). Команды `forgeplan config set` нет.
- Решение является тактической и вообще не требует артефакта - просто закоммитьте.
- Нужен полный ручной контроль - используйте [`forgeplan new`](/ru/docs/cli/new/).
- Фиксируете решение из разговора - используйте [`forgeplan capture`](/ru/docs/cli/capture/).
- Есть сохранённая запись - используйте [`forgeplan promote`](/ru/docs/cli/promote/).

## Использование

```text
forgeplan generate <KIND> <DESCRIPTION>
```

## Аргументы

```text
  <KIND>         Тип артефакта: prd, epic, spec, rfc, adr, problem, solution, evidence
  <DESCRIPTION>  Описание на естественном языке того, что нужно сгенерировать
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

`generate` читает проектную секцию `llm`. HTTP-провайдерам нужна указанная переменная окружения с API-ключом; явно выбранные локальные subscription-провайдеры используют существующий вход в CLI вендора. Отсутствие конфигурации или ошибка провайдера - ошибка команды, а не успешный пустой черновик.

## Примеры

### Пример 1: Сгенерировать PRD из однострочной задачи

```bash
forgeplan generate prd "add rate limiting to /api/v1/ endpoints"
```

Команда сообщает созданный путь и ID. Модель получает задание заполнить разделы PRD, но фактическое содержимое нужно проверить. После ревью запустите `forgeplan validate` с возвращённым ID; не выводите имя файла или ID из этой иллюстрации.

### Пример 2: Реконструировать RFC для существующей подсистемы

```bash
forgeplan generate rfc "selected Constellation embedding pipeline: tract BGE-M3, 1024 dimensions, model cache root resolved from FORGEPLAN_MODEL_CACHE or ~/.forgeplan/models"
```

Это промпт для документации существующей системы, не результат анализа кода. Сверьте архитектуру с целевым исходником и отмечайте этапы реализации выполненными только при наличии реальных доказательств.

### Пример 3: Составить черновик ProblemCard для нового сигнала

```bash
forgeplan generate problem "users report search returns stale results after renaming artifacts"
```

Модель получает задание заполнить шаблон ProblemCard. Проверьте Signal, Context, Goals и Anti-Goodhart, свяжите реальные Evidence или Solutions по необходимости и выполните валидацию и lifecycle-гейты перед активацией.

## Как это вписывается в рабочий процесс

```
route  →  generate  →  (review + edit)  →  validate  →  reason  →  code  →  evidence  →  review  →  activate
```

`generate` встраивается в ту же фазу Shape, что и `new`, но объединяет «создание заглушки» и «заполнение ОБЯЗАТЕЛЬНЫХ разделов» в один вызов LLM. Остальная часть конвейера не меняется: вам все еще нужно, чтобы `validate` прошел, `reason` для Deep/Critical глубины, и Evidence перед `activate`. Относитесь к сгенерированному тексту как к первому черновику - модель не уловит специфические для предметной области ограничения так, как это сделает человек-оператор.

## Смотрите также

- [`forgeplan new`](/ru/docs/cli/new/) - ручной шаблон
- [`forgeplan validate`](/ru/docs/cli/validate/) - проверка черновика
- [`forgeplan reason`](/ru/docs/cli/reason/) - ADI
- [`forgeplan capture`](/ru/docs/cli/capture/) - решение из разговора
- [Модель артефактов](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan generate --help`.

```text
Generate an artifact using AI from a natural language description

Usage: forgeplan generate <KIND> <DESCRIPTION>

Arguments:
  <KIND>         Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence
  <DESCRIPTION>  Description of what to generate

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
