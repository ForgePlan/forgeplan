---
title: forgeplan decompose
description: "Разбивает утвержденный PRD на RFC с фазами реализации и подзадачами с помощью ИИ"
---

`forgeplan decompose` принимает утвержденный PRD и просит LLM создать соответствующий RFC: фазы реализации, подзадачи для каждой фазы и порядок зависимостей. Это устраняет разрыв между "мы знаем, что хотим" (PRD) и "вот план спринта" (RFC), не заставляя автора вручную перестраивать требования.

Результатом является артефакт RFC в статусе черновик с заполненными чекбоксами **Implementation Phases**, связанный с исходным PRD через отношение `implements`. Вы по-прежнему просматриваете и редактируете его — decompose это первый черновик, а не окончательное решение.

## Когда использовать

- PRD валидирован (`forgeplan validate PRD-XXX` = PASS) и обоснован (`forgeplan reason`)
- Глубина **Standard** или выше — тактические задачи не требуют отдельного RFC
- Вы переходите от Shape к Code и хотите получить готовый план в виде чек-листа
- PRD содержит 5+ функциональных требований, и разбивка на фазы неочевидна

## Когда НЕ использовать

- Глубина **Tactical** — переходите сразу к коду
- PRD все еще является заглушкой (отсутствуют Problem, Goals, FR) — decompose будет галлюцинировать фазы
- RFC для этого PRD уже существует — используйте `forgeplan update` или процесс замещения вместо этого
- Вы не согласны с целями PRD — сначала исправьте PRD, не пытайтесь замаскировать это с помощью RFC

## Использование

```text
forgeplan decompose <ID>
```

## Аргументы

```text
  <ID>  ID артефакта PRD для декомпозиции
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Декомпозиция валидированного PRD

```bash
forgeplan validate PRD-019
forgeplan decompose PRD-019
```

Читает `PRD-019`, отправляет его Problem/Goals/FR/Non-Goals в LLM и создает `RFC-0XX`, связанный с `implements: PRD-019`. Сгенерированный RFC содержит раздел **Implementation Phases** с неотмеченными чекбоксами, готовыми для отслеживания прогресса.

### Пример 2: Полный конвейер от идеи до плана спринта

```bash
forgeplan route "add OAuth2 login flow"           # -> Standard, PRD -> RFC
forgeplan new prd "OAuth2 login flow"             # -> PRD-042
# ... fill MUST sections ...
forgeplan validate PRD-042                        # PASS
forgeplan reason PRD-042 --fpf                    # ADI cycle
forgeplan decompose PRD-042                       # -> RFC-018 draft
forgeplan validate RFC-018                        # sanity check
```

После декомпозиции откройте `RFC-018` в вашем редакторе, уточните описания фаз и начните отмечать чекбоксы по мере завершения фаз.

## Интерпретация вывода

Decompose выводит ID созданного RFC и сводку сгенерированных фаз:

```
Created RFC-018 (draft) linked to PRD-042
  Phase 1: Authentication provider abstraction (3 tasks)
  Phase 2: OAuth2 flow implementation (5 tasks)
  Phase 3: Session persistence + refresh (4 tasks)
  Phase 4: E2E tests + rollout gate (2 tasks)
```

Красные флаги:

- Одна фаза с 20 подзадачами — PRD слишком широк, разделите его на несколько PRD
- Фазы ссылаются на FR, которых нет в PRD — галлюцинация LLM, повторите запуск или отредактируйте
- Нет фазы отката/доказательства — добавьте ее вручную перед активацией

## Как это вписывается в рабочий процесс

```
Shape → Validate → Reason → [decompose] → Code → Evidence → Activate
                                 ^
                             вы здесь
```

- **До**: `forgeplan validate PRD-XXX` (PASS), `forgeplan reason PRD-XXX`
- **После**: отредактируйте RFC, создайте ветку фичи, начните реализацию фазы 1
- Отслеживание прогресса: отмечайте чекбоксы фаз по мере слияния PR; `forgeplan progress RFC-018` выводит индикатор

## Смотрите также

- [`forgeplan reason`](/docs/cli/reason/) — запустите цикл ADI перед декомпозицией
- [`forgeplan new`](/docs/cli/new/) — создайте исходный PRD
- [`forgeplan validate`](/docs/cli/validate/) — гейт качества для входных данных декомпозиции
- [`forgeplan generate`](/docs/cli/generate/) — генерирует контент для любого типа артефакта
- [Методология: процесс PRD → RFC](/docs/methodology/overview/)
