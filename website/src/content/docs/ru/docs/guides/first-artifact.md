---
title: Руководство по первому артефакту
description: "20-минутное практическое руководство: создание, валидация, обоснование, доказательство и активация вашего первого PRD — с распространёнными ошибками и способами их исправления."
---

Это расширенная версия [Краткого руководства](/docs/getting-started/quick-start/).
Оно охватывает те же семь шагов, но с более подробными объяснениями, реалистичным
содержимым файлов и **ошибками, с которыми вы столкнётесь на каждом шаге, а также
способами их исправления**.

Запланируйте 20 минут. В итоге вы получите реальное активированное решение,
которое обучит вас полному циклу Forgeplan.

## Что вы создадите

PRD для небольшой, реальной функции: **«Добавить флаг `--dry-run` к команде
`forgeplan new`, чтобы пользователи могли предварительно просматривать, что будет
создано, без записи файлов.»**

Она намеренно небольшая — роутинг, вероятно, покажет `Standard`, что означает,
что мы пройдём каждый шаг, за исключением уровней Epic/Spec. Идеальная
тренировочная нагрузка.

```mermaid
flowchart LR
  S1["1. Init"] --> S2["2. Роутинг"] --> S3["3. Формирование"]
  S3 --> S4["4. Валидация"] --> S5["5. ADI"]
  S5 --> S6["6. Код"] --> S7["7. Доказательство"]
  S7 --> S8["8. Оценка"] --> S9["9. Активация"]
  S9 --> S10["10. Health ✓"]
```

## Предварительные условия

- Установленный Forgeplan (`forgeplan --version`)
- Каталог, где вы можете создать рабочее пространство (временный подойдёт)
- Необязательно: `GEMINI_API_KEY` или другой провайдер LLM для обоснования ADI

## Шаг 1 — Инициализация рабочего пространства

```bash
mkdir ~/forgeplan-tutorial && cd ~/forgeplan-tutorial
forgeplan init -y
```

Вы должны увидеть:

```
Initialized .forgeplan/ workspace at /Users/you/forgeplan-tutorial/.forgeplan
  Created: prds/, rfcs/, adrs/, evidence/, notes/, problems/, ...
  Created: config.yaml
Ready.
```

Подтвердите:

```bash
forgeplan health
```

```
Project Health
  Total artifacts: 0
  Blind spots: 0
  Orphans: 0
  Stale: 0
  Status: OK — empty workspace
```

### Возможная ошибка

**`Error: .forgeplan/ already exists`** — вы запустили `init` в уже
существующем рабочем пространстве. Либо перейдите в другой каталог (`cd`),
либо удалите `.forgeplan` (`rm -rf .forgeplan`) (только в каталоге для
временного руководства — никогда не делайте этого в реальном проекте без
предварительного экспорта: `forgeplan export --output backup.json`).

## Шаг 2 — Роутинг задачи

```bash
forgeplan route "add --dry-run flag to forgeplan new for preview"
```

Ожидаемый вывод:

```
Task: add --dry-run flag to forgeplan new for preview
Depth: Standard
Pipeline: PRD → RFC
Confidence: 82%
Signals:
  + new feature (not a fix)
  + CLI UX surface change
  + multiple possible implementations
Recommendation:
  1. forgeplan new prd "CLI dry-run flag"
  2. Fill MUST sections (Problem, Goals, FR)
  3. forgeplan reason PRD-XXX  (recommended at Standard)
```

Если роутер покажет `Tactical`, переопределите и всё равно рассматривайте это
как Standard — мы хотим отработать полный цикл. См. [Роутинг и глубина](/docs/methodology/routing/)
для дерева решений.

## Шаг 3 — Shape: создание PRD

```bash
forgeplan new prd "CLI dry-run flag"
```

```
Created: PRD-001 at .forgeplan/prds/PRD-001-cli-dry-run-flag.md
```

Откройте файл в вашем редакторе. Вы увидите шаблон с заголовками разделов.
Заполните разделы MUST:

```markdown
# PRD-001: Флаг CLI dry-run

## Проблема

Пользователи, запускающие `forgeplan new prd "..."` в общем рабочем
пространстве, не могут предварительно просмотреть, какие файлы будут
созданы, до коммита. Ошибки (неправильный заголовок, коллизия ID)
требуют ручной очистки.

## Цели

- G1: Пользователь может видеть полный путь к файлу и содержимое того,
  что `new` создаст, без записи файлов на диск
- G2: Вывод `--dry-run` подходит для передачи в инструменты ревью
- G3: Отсутствие изменений в поведении, когда флаг отсутствует

## Не-цели

- Не полноценный «режим симуляции» — охватывает только команду `new`
- Не механизм отката — файлы, созданные без `--dry-run`, остаются созданными

## Целевые пользователи

Пользователи CLI, создающие артефакты в общих или производственных
рабочих пространствах, и агенты ИИ, которые хотят предварительно
просмотреть свой вывод перед коммитом.

## Функциональные требования

- FR1: Пользователь может передать `--dry-run` команде `forgeplan new <kind> "title"`
- FR2: Пользователь может видеть точный путь к файлу, который будет создан
- FR3: Пользователь может видеть содержимое файла-шаблона в stdout
- FR4: Пользователь может полагаться на код выхода 0 при успешном выполнении
  `--dry-run` и ненулевой код при неудачной валидации
```

Обратите внимание: нет упоминаний «использовать clap» или «выводить JSON»
или какой-либо конкретной реализации. Это правило 3 — функциональные
требования описывают возможности, а не реализации. См. [Обзор методологии](/docs/methodology/overview/).

## Шаг 4 — Validate

```bash
forgeplan validate PRD-001
```

Лучший случай:

```
PRD-001: PASS ✓
  MUST: 0 errors
  SHOULD: 1 warning (density: Problem section is terse)
```

### Ошибки, с которыми вы, вероятно, столкнётесь

**`MUST error: Problem section missing`** — вы забыли обязательный заголовок.
Работают псевдонимы: `## Motivation`, `## Background`, `## Problem Statement` —
все они считаются Problem. Добавьте один.

**`MUST error: implementation leakage in FR2`** — вы написали что-то вроде
«Использовать вывод JSON с serde». Перепишите как «Пользователь может видеть
точный путь к файлу и его содержимое». Валидатор помечает названия библиотек
и технологические решения в требованиях.

**`MUST error: no functional requirements`** — раздел `## Functional Requirements`
существует, но не содержит пунктов. Добавьте хотя бы одно функциональное
требование по шаблону `[Действующее лицо] может [возможность]`.

**`MUST error: vague goal "system should be fast"`** — валидатор обнаружил
неизмеримое утверждение. Перепишите с использованием чисел или удалите.

Перезапускайте `forgeplan validate PRD-001` после каждого исправления.

## Шаг 5 — Reason (ADI)

```bash
forgeplan reason PRD-001
```

Если у вас настроен LLM (`.forgeplan/config.yaml` содержит провайдера и ключ),
вы увидите что-то вроде:

```
ADI cycle for PRD-001
─────────────────────
Abduction — 3 hypotheses:
  H1: Single --dry-run flag that short-circuits file write
  H2: Separate `forgeplan preview new` command
  H3: Interactive confirmation prompt (y/n before write)

Deduction — predictions per hypothesis:
  H1:
    - Minimal code change, 1 branch in `new` command
    - Reusable: same flag can extend to other write commands
    - No new command surface to document
  H2:
    - Discoverable via `forgeplan --help`
    - Duplicates template rendering logic (or extracts it)
    - Doubles the surface area users must learn
  H3:
    - Forces interactivity, breaks AI-agent usage
    - No way to see output before committing
    - Violates `-y` non-interactive contract

Induction — evidence check:
  H1: supports — aligns with existing flag patterns (e.g. `init -y`)
  H2: weakens — duplication + discoverability win does not offset cost
  H3: refutes — breaks AI agent workflow (see MUST in CLAUDE.md)

Recommendation: H1
Confidence: 0.87
```

LLM не настроен? Вы получите шаблон, предлагающий заполнить гипотезы вручную.
Это всё равно считается — ценность заключается в обдумывании альтернатив, а не
в выводе LLM.

См. [Обоснование ADI](/docs/methodology/adi/), чтобы понять, почему этот шаг существует.

### Ошибка

**`Error: no LLM provider configured`** — откройте `.forgeplan/config.yaml`
и добавьте блок провайдера. Для руководства вы можете пропустить это и
записать 3 гипотезы непосредственно в тело PRD в разделе `## Reasoning`.

## Шаг 6 — Build

Напишите код. Для этого руководства представьте, что вы реализовали H1 и
написали тесты. В реальной работе с Forgeplan вы бы:

```bash
cargo test      # or npm test, pytest, go test, ...
cargo fmt
cargo check     # 0 warnings, 0 errors
```

Все три должны пройти, прежде чем вы создадите доказательство, утверждающее это.
Если `cargo check` выдаёт предупреждения, исправьте их — собственное правило
Forgeplan CLAUDE.md гласит: «0 предупреждений, 0 ошибок» при каждом коммите.

## Шаг 7 — Prove: создание Evidence

```bash
forgeplan new evidence "CLI dry-run — 8 unit tests pass, flag works end-to-end"
```

```
Created: EVID-001 at .forgeplan/evidence/EVID-001-cli-dry-run....md
```

Откройте файл и добавьте блок **структурированных полей** в тело. Это самая
важная часть руководства:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Details

- 8 модульных тестов проходят в `tests/cli_dry_run.rs`
- Ручной смоук-тест: `forgeplan new prd "Test" --dry-run` выводит шаблон
  без создания файла
- Проверен код выхода 0 при успехе и 1 при ошибке шаблона
```

Свяжите его с PRD:

```bash
forgeplan link EVID-001 PRD-001 --relation informs
```

### Ошибка №1

**Забыть структурированные поля.** Если вы просто напишете прозу в теле
доказательства и пропустите строки `verdict: supports / congruence_level: 3 /
evidence_type: test`, парсер R_eff вернётся к CL0 со штрафом 0.9. Ваш балл
будет близок к нулю, даже если доказательство сильное. Всегда включайте эти
три поля.

## Шаг 8 — Проверка балла

```bash
forgeplan score PRD-001
```

Ожидаемый результат:

```
PRD-001: CLI dry-run flag
  R_eff = 1.00 — Adequate
  Evidence:
    EVID-001: supports, CL3, test → score 1.0
```

### Что если R_eff = 0.0?

1. `forgeplan list evidence` — есть ли EVID-001 в рабочем пространстве?
2. `cat .forgeplan/evidence/EVID-001-*.md | grep -E "verdict|congruence_level|evidence_type"`
   — все три поля присутствуют?
3. `forgeplan link EVID-001 PRD-001 --relation informs` — была ли создана связь?

См. [Доказательства и R_eff](/docs/methodology/evidence/) для полной формулы.

## Шаг 9 — Активация

```bash
forgeplan review PRD-001
```

```
Reviewing PRD-001...
  Validation: PASS ✓
  Evidence: 1 linked (R_eff = 1.00)
  Status: ready to activate
```

```bash
forgeplan activate PRD-001
```

```
PRD-001: draft → active
  Validation gate: PASS
  R_eff preserved: 1.00
```

Если гейт валидации здесь не проходит, переход отклоняется. Самая
частая причина — вы отредактировали PRD после шага 4 и внесли
нарушение правила MUST. Перезапустите `forgeplan validate PRD-001` и исправьте.

## Шаг 10 — Проверка

```bash
forgeplan health
```

```
Project Health
  Total artifacts: 2 (PRD-001, EVID-001)
  Active: 1
  Draft: 1 (EVID-001 — пакеты доказательств остаются в черновике)
  Blind spots: 0
  Orphans: 0
  Stale: 0
  Status: HEALTHY
```

Поздравляем — у вас есть полностью отслеживаемое решение с измеримым доверием.

## Что вы узнали

- **Сначала роутинг.** Вы не начали сразу кодировать — вы спросили
  Forgeplan, какая глубина нужна задаче.
- **Shape до кода.** PRD зафиксировал «зачем» и «что» до
  первой строки реализации.
- **Ранняя валидация.** Валидатор отловил утечку реализации и
  отсутствующие разделы до того, как они стали привычкой.
- **Обоснование через ADI.** Вы сгенерировали альтернативы до принятия
  обязательств, и именно так вы избегаете моментов «жаль, что не подумали об этом».
- **Подтверждение доказательствами.** R_eff — это не украшение, а число,
  которое говорит вам, стоит ли доверять собственному решению.
- **Активация только при готовности.** Гейт предотвращает появление «активных PRD без
  кода» — ложного обещания будущим читателям.

## Следующие шаги

- Запустите полный цикл на реальной задаче в вашем основном проекте
- Прочитайте [Обзор методологии](/docs/methodology/overview/) для ознакомления с 10 правилами
- Изучите [Справочник CLI](/docs/cli/) — каждая команда задокументирована
- Погрузитесь в [Жизненный цикл артефакта](/docs/methodology/lifecycle/), чтобы узнать
  про `supersede`, `deprecate`, `renew` и `reopen`
- Проверьте [Конфигурацию](/docs/getting-started/configuration/) для настройки
  провайдера LLM для более интеллектуального роутинга и ADI

## Шпаргалка по устранению неполадок

| Симптом | Причина | Исправление |
|---------|---------|-------------|
| `MUST error: Problem missing` | Нет раздела `## Problem` / `## Motivation` | Добавьте один из псевдонимов |
| `implementation leakage in FR` | В требовании указана библиотека/технология | Перепишите как `[Действующее лицо] может [возможность]` |
| `R_eff = 0.0` для оцениваемого артефакта | В доказательстве отсутствуют `verdict` / `congruence_level` / `evidence_type` | Добавьте три поля в тело доказательства |
| `activate` отклонён | Гейт валидации не прошёл после правок | Перезапустите `validate`, исправьте ошибки MUST |
| `Error: no LLM provider` при `reason` | Нет ключа в `.forgeplan/config.yaml` | Добавьте блок провайдера или запишите гипотезы вручную |
| `health` показывает слепое пятно | Активный артефакт без связанных доказательств | Создайте доказательство и `forgeplan link` |
| Предупреждение `stale` для свежего артефакта | Слишком короткий `valid_until` | Установите реалистичный срок действия 90-180 дней |