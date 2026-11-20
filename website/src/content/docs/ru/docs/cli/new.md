---
title: forgeplan new
description: "Создаёт новый артефакт из заготовки-шаблона"
---

Создаёт новую заготовку артефакта из шаблона в `.forgeplan/<kind>s/`. Это первый шаг фазы **Shape**: вы получаете скелет с YAML-преамбулой и отмеченными секциями MUST, затем заполняете их перед запуском `validate`. Используйте это для любой задачи уровня Standard+ - Forgeplan блокирует активацию артефактов с пустыми секциями MUST, поэтому оставление заготовки означает, что работа никогда не будет выпущена.

## Когда использовать

- Начало фазы **Shape** для любой задачи уровня Standard+: создайте PRD / RFC / ADR до написания кода, чтобы зафиксировать обоснование.
- Запись тактической Note без необходимости использования LLM - `new note "..."`.
- Явный контроль шаблона - когда вам нужен точный макет заготовки, а не тело, сгенерированное LLM (используйте [`forgeplan generate`](/docs/cli/generate/) для пути с LLM).

## Когда НЕ использовать

- Тактическое исправление одного файла без проектных решений - полностью пропустите артефакт, просто сделайте коммит с чётким сообщением.
- Решение было принято в ходе живого разговора, и вы хотите только его зафиксировать - используйте [`forgeplan capture`](/docs/cli/capture/).
- У вас уже есть сохранённая память, которую вы хотите превратить в артефакт - используйте [`forgeplan promote`](/docs/cli/promote/).

## Использование

```text
forgeplan new [OPTIONS] <KIND> <TITLE>
```

## Аргументы

```text
  <KIND>   Тип артефакта: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
  <TITLE>  Заголовок артефакта
```

## Опции

```text
      --allow-duplicate  Пропустить запрос на обнаружение дубликатов и всё равно создать [псевдонимы: --force]
  -h, --help             Вывести справку
  -V, --version          Вывести версию
```

Forgeplan выполняет проверку на сходство с существующими артефактами и предупреждает, если заголовок выглядит как дубликат. Передайте `--allow-duplicate` / `--force`, чтобы обойти это, если вы намеренно хотите создать параллельный артефакт (например, второй PRD, исследующий ту же область с другого ракурса).

Без `--allow-duplicate` возможен best-effort fetch и проверка `origin/dev`.
Подтверждение дубликата интерактивно лишь в терминале; скрипт получает
явный отказ вместо запроса.

## Примеры

### Пример 1: Создание PRD для новой функции

```bash
forgeplan new prd "OAuth2 login"
```

Создаёт slug-файл вроде `.forgeplan/prds/prd-oauth2-login.md`; используйте фактически выданные путь и ID. Заполните обязательные разделы, затем валидируйте. Не воссоздавайте выведенные из эксплуатации таблицы артефактов.

### Пример 2: Запись сигнала о проблеме

```bash
forgeplan new problem "Slow search on 10k+ artifacts"
```

Создаёт шаблон ProblemCard. Заполните обязательные разделы и свяжите доказательства. Активация выполняет правила типа/глубины через отдельную команду; ProblemCard не означает универсальный обход валидации.

### Пример 3: Прикрепление бенчмарка в качестве доказательства

```bash
forgeplan new evidence "Benchmark auth handler under 500 concurrent users"
forgeplan link EVID-NNN PRD-012 --relation informs
forgeplan score PRD-012
```

Для расчёта evidence указывайте структурированные `verdict` и `congruence_level`. Отсутствующие или нераспознанные поля оценки могут дать CL0; пропущенный `evidence_type` по умолчанию означает measurement. Свяжите evidence с подтверждаемым артефактом и проверьте результат, не предполагая обязательный рост оценки.

## Как это вписывается в рабочий процесс

Эта команда является частью [полного жизненного цикла артефакта](/docs/guides/first-artifact/) - см. руководство для сквозного процесса. `new` только создаёт заготовку: **работа не считается завершённой, пока не будут заполнены секции MUST и `validate` не пройдёт успешно.** Наполовину заполненные заготовки накапливаются как слепые пятна в `forgeplan health` и блокируют активацию позже.

## См. также

- [`forgeplan generate`](/docs/cli/generate/) - позвольте LLM создать черновик тела артефакта вместо ручного заполнения
- [`forgeplan validate`](/docs/cli/validate/) - запускает правила MUST/SHOULD; требуется перед `activate`
- [`forgeplan route`](/docs/cli/route/) - определяет глубину + конвейер перед созданием артефакта
- [`forgeplan health`](/docs/cli/health/) - выявляет заготовки и слепые пятна после создания
- [Методология: модель артефакта](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan new --help`.

```text
Create a new artifact from template

Usage: forgeplan new [OPTIONS] <KIND> <TITLE>

Arguments:
  <KIND>   Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
  <TITLE>  Artifact title

Options:
      --allow-duplicate  Skip duplicate-detection prompt and create anyway [alias: --force]
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
