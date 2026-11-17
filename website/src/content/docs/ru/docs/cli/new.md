---
title: forgeplan new
description: "Создаёт новый артефакт из заготовки-шаблона"
---

Создаёт черновик из шаблона в соответствующем каталоге `.forgeplan/`. Это начало Shape: заполните содержимое и валидируйте его. Обычная активация PRD/Epic/Spec/RFC/ADR отклоняет незаполненные MUST и другие нарушенные условия; `--force` — явное исключение, а облегчённые типы имеют другие правила активации.

## Forgeplan 0.38.0

Все новые артефакты создаются в draft без автоматического valid_until. Для note/evidence/problem/solution/refresh глубина по умолчанию tactical, для остальных — standard. Кроме десяти типов в help, parser принимает memory, use_case, glossary, invariant, scenario, hypothesis и domain_model. При похожем заголовке того же типа интерактивный запуск запрашивает подтверждение, а неинтерактивный отклоняет создание. --allow-duplicate (алиас --force именно у new) явно обходит эту проверку; это не activation --force. Создание не вызывает LLM.

## Когда использовать

- Начало фазы **Shape** для любой задачи уровня Standard+: создайте PRD / RFC / ADR до написания кода, чтобы зафиксировать обоснование.
- Запись тактической Note без необходимости использования LLM - `new note "..."`.
- Явный контроль шаблона - когда вам нужен точный макет заготовки, а не тело, сгенерированное LLM (используйте [`forgeplan generate`](/ru/docs/cli/generate/) для пути с LLM).

## Когда НЕ использовать

- Для тактического исправления одного файла без проектных решений необходимость Note определяется политикой проекта; CLI не требует PRD для каждой правки.
- Решение было принято в ходе живого разговора, и вы хотите только его зафиксировать - используйте [`forgeplan capture`](/ru/docs/cli/capture/).
- У вас уже есть сохранённая память, которую вы хотите превратить в артефакт - используйте [`forgeplan promote`](/ru/docs/cli/promote/).

## Использование

```text
forgeplan new [OPTIONS] <KIND> <TITLE>
```

## Аргументы

```text
  <KIND>   Artifact kind: prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh
  <TITLE>  Artifact title
```

## Опции

```text
      --allow-duplicate  Skip duplicate-detection prompt and create anyway [alias: --force]
  -h, --help             Print help
  -V, --version          Print version
```

Forgeplan выполняет проверку на сходство с существующими артефактами и предупреждает, если заголовок выглядит как дубликат. Передайте `--allow-duplicate` / `--force`, чтобы обойти это, если вы намеренно хотите создать параллельный артефакт (например, второй PRD, исследующий ту же область с другого ракурса).

## Примеры

### Пример 1: Создание PRD для новой функции

```bash
forgeplan new prd "OAuth2 login"
```

Печатает фактически созданный путь и ID. Используйте возвращённые значения, не угадывайте имя со slug до слияния или отображаемым ID. Немедленно откройте его и заполните разделы Problem, Goals, Non-Goals, Target Users, Related и Functional Requirements - затем запустите `forgeplan validate PRD-NNN`, чтобы убедиться в отсутствии ошибок MUST, прежде чем продолжить.

### Пример 2: Запись сигнала о проблеме

```bash
forgeplan new problem "Slow search on 10k+ artifacts"
```

Создаёт ProblemCard с разделами Signal / Context / Goals / Anti-Goodhart indicators. ProblemCard не требуют дизайна - заполните Signal и Context, затем свяжите с любым Evidence, который у вас уже есть. Проблемы не требуют гейта валидации для активации, поэтому вы можете активировать их, как только карточка станет связной.

### Пример 3: Прикрепление бенчмарка в качестве доказательства

```bash
forgeplan new evidence "Benchmark auth handler under 500 concurrent users"
# Fill the real evidence body first; substitute its returned ID below.
forgeplan link EVID-001 PRD-012 --relation informs
forgeplan score PRD-012
```

Тела Evidence ДОЛЖНЫ содержать структурированные поля (`verdict`, `congruence_level`, `evidence_type`) - без них парсер R_eff возвращается к CL0 и снижает оценку. Свяжите с артефактом, который подтверждает Evidence, затем изучите рекурсивную оценку: новая связь не гарантирует роста R_eff.

## Как это вписывается в рабочий процесс

Эта команда входит в [полный жизненный цикл артефакта](/ru/docs/guides/first-artifact/) - см. руководство для сквозного процесса. `new` только создаёт заготовку: **работа не считается завершённой, пока не будут заполнены секции MUST и `validate` не пройдёт успешно.** Незаполненные черновики могут попасть в замечания об устаревших черновиках или отсутствии связей; blind spots относятся к активным решениям без поддержки доказательствами. Обычная активация проверяемых типов отклоняет незавершённую работу.

## См. также

- [`forgeplan generate`](/ru/docs/cli/generate/) - позвольте LLM создать черновик тела артефакта вместо ручного заполнения
- [`forgeplan validate`](/ru/docs/cli/validate/) - запускает правила MUST/SHOULD; требуется перед `activate`
- [`forgeplan route`](/ru/docs/cli/route/) - определяет глубину + конвейер перед созданием артефакта
- [`forgeplan health`](/ru/docs/cli/health/) - выявляет заготовки и слепые пятна после создания
- [Методология: модель артефакта](/ru/docs/methodology/overview/)
