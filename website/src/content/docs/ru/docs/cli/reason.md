---
title: forgeplan reason
description: "Анализ артефакта с использованием цикла рассуждений FPF ADI (абдукция→дедукция→индукция)"
---

`forgeplan reason` выполняет структурированный анализ существующего артефакта с помощью ИИ, используя цикл **ADI** FPF: **абдукция** (генерация 3+ гипотез) → **дедукция** (прогнозирование последствий каждой) → **индукция** (проверка прогнозов на основе существующих доказательств). Это гейт между «PRD выглядит разумным» и «я действительно знаю, какой подход выбрать» - он заставляет агента перечислять альтернативы вместо того, чтобы зацикливаться на первом правдоподобном ответе. Для глубины Deep и Critical это **обязательно**: никакого кода, пока `reason` не сгенерирует как минимум три конкурирующие гипотезы и обоснованного победителя.

Это локальный ADI-процесс проекта, не доказательство соответствия upstream FPF. Промпт требует 3+ гипотезы, но парсер не проверяет их количество. Для Deep/Critical проверьте обязательное условие вручную: успешная команда его не доказывает. Нужен LLM-провайдер: HTTP использует `llm.api_key_env`, явно выбранные локальные CLI-профили - собственный login, только для личной локальной разработки.

## Когда использовать

- Сразу после `forgeplan new prd` + `forgeplan validate` PASS, до начала работы с кодом.
- Глубина **Deep** или **Critical** - ADI не подлежит обсуждению.
- Глубина **Standard**, и пространство решений имеет реальные компромиссы (уровень кэширования, алгоритм ограничения скорости, поток аутентификации).

## Когда НЕ использовать

- Tactical-задача без решения, требующего альтернатив; длительность сама по себе не задаёт глубину.
- Разделы MUST ещё не заполнены - ADI на заглушке создаёт галлюцинаторный контекст.
- Артефакт является чистым EvidencePack, Note или RefreshReport (нет решения для рассуждений).

## Использование

```text
forgeplan reason [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  ID артефакта для анализа (PRD, RFC, ADR, Epic, Problem, ...)
```

## Опции

```text
      --json     Вывод структурированного JSON вместо markdown
      --save     Сохранить анализ ADI как артефакт Note, связанный с источником
      --fpf      Внедрить соответствующие шаблоны FPF в промпт ADI
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Стандартный ADI для PRD

```bash
forgeplan reason PRD-001
```

Читает тело артефакта и просит настроенную LLM описать гипотезы, предположения, прогнозы, риски, осуществимость, потребность в доказательствах и рекомендацию. Вход не ограничен именованными Problem/Goals/Related.

### Пример 2: ADI с контекстом базы знаний FPF

```bash
forgeplan reason PRD-001 --fpf
```

`--fpf` запрашивает контекст базы знаний, когда этот источник доступен. В выбранном migrated CLI-пути FPF-обогащение пропускается; legacy-путь может дополнить промпт. Ни один путь не гарантирует конкретные upstream-разделы и не вычисляет соответствие инвариантам FPF. Проверяйте переданный контекст и утверждения отдельно.

### Пример 3: Сохранение анализа как Note

```bash
forgeplan reason PRD-001 --save
```

Создаёт Note с выводом ADI и связывает с источником. Читайте его по возвращённому ID; связи смотрите через `forgeplan graph`. Опции `forgeplan get --graph` нет.

### Пример 4: Машиночитаемый вывод для агентов

```bash
forgeplan reason PRD-001 --json
```

Результат содержит метаданные и `adi_output` при успешном разборе, иначе сырой `adi_analysis`. У гипотез поля `id`, `description`, `assumptions`, строка `confidence`; у дедукций - `hypothesis_id`, `consequence`, `risks`, `feasibility`. `evidence_needed` и `recommendation` отдельные, не массивы evidence/verdict у каждой гипотезы.

## Интерпретация вывода

Проверьте запрошенные этапы:

- **Абдукция**: для Deep/Critical нужны минимум три реально конкурирующие гипотезы; успешный parse не проверяет это.
- **Дедукция**: изучите прогнозы, риски и осуществимость.
- **Индукция**: проверьте реальные потребности в доказательствах и рекомендацию. Команда не проводит эксперименты и не валидирует рекомендацию.

Строки confidence - суждения модели, не калиброванные проценты или измеренное доверие. Согласие гипотез не является независимым доказательством. При отсутствии альтернатив или данных нужно дополнительное shaping либо целевые доказательства до прохождения гейта. Вердикты EvidencePack `supports` / `weakens` / `refutes` отличаются от суждений в сгенерированной прозе.

## Как это вписывается в рабочий процесс

Команда входит в [жизненный цикл](/ru/docs/guides/first-artifact/): после validate PASS и до кода. Для Critical сохраняются `/audit` и человеческое ревью до реализации.

## См. также

- [`forgeplan route`](/ru/docs/cli/route/) - глубина
- [`forgeplan decompose`](/ru/docs/cli/decompose/) - декомпозиция
- [`forgeplan validate`](/ru/docs/cli/validate/) - предварительная валидация
- [`forgeplan generate`](/ru/docs/cli/generate/) - черновик
- [Цикл ADI](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reason --help`.

```text
Analyze an artifact using FPF ADI reasoning cycle (Abduction→Deduction→Induction)

REQUIRES LLM: generates structured three-hypothesis ADI output through the provider configured in `.forgeplan/config.yaml::llm`.

AUTHENTICATION: HTTP providers use the API-key environment variable named by `llm.api_key_env`. Local subscription providers reuse an existing stock CLI login and need no API key in ForgePlan:

llm: provider: codex-cli model: gpt-5.1-codex

Supported local profiles are `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, and `cursor-agent`. They are explicit opt-ins for personal local development, not production/shared/CI use. Each invocation consumes the corresponding vendor plan and is subject to that vendor's terms.

Usage: forgeplan reason [OPTIONS] <ID>

Arguments:
  <ID>
          Artifact ID to analyze

Options:
      --json
          Output structured JSON instead of markdown

      --save
          Save ADI analysis as a Note artifact linked to the source

      --fpf
          Inject relevant FPF patterns into the ADI prompt

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
