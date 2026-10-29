---
title: forgeplan decompose
description: "Вывести предложения RFC-задач от LLM без создания артефактов."
---

`forgeplan decompose` читает заголовок и тело PRD и передаёт их настроенной LLM.
Команда печатает предложения задач и зависимостей, но **не создаёт RFC**,
чекбоксы или связи. После ревью создайте RFC через `forgeplan new rfc` и свяжите
их с PRD отношением `refines`. Валидация и согласование - отдельные шаги процесса,
а не встроенные проверки этой команды.

## Когда использовать

- PRD валидирован (`forgeplan validate PRD-XXX` = PASS) и обоснован (`forgeplan reason`)
- Глубина **Standard** или выше - тактические задачи не требуют отдельного RFC
- Вы переходите от Shape к Code и хотите получить готовый план в виде чек-листа
- PRD содержит 5+ функциональных требований, и разбивка на фазы неочевидна

## Когда НЕ использовать

- Глубина **Tactical** - переходите сразу к коду
- PRD ещё заглушка - неполные требования могут привести к необоснованным предложениям
- RFC для этого PRD уже существует - используйте `forgeplan update` или процесс замещения вместо этого
- Вы не согласны с целями PRD - сначала исправьте PRD, не пытайтесь замаскировать это с помощью RFC

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

Читает `PRD-019` и печатает предложения LLM. Создайте и заполните RFC отдельно; используйте фактически выданные ID.

### Пример 2: Полный конвейер от идеи до плана спринта

```bash
forgeplan route "add OAuth2 login flow"           # -> Standard, PRD -> RFC
forgeplan new prd "OAuth2 login flow"             # -> PRD-042
# ... fill MUST sections ...
forgeplan validate PRD-042                        # PASS
forgeplan reason PRD-042 --fpf                    # ADI cycle
forgeplan decompose PRD-042                       # только печатает предложения
forgeplan new rfc "OAuth2 implementation"         # запишите выданный ID
# Заполните RFC, свяжите с PRD-042 через --relation refines, затем валидируйте.
```

После ревью предложений создайте и заполните RFC перед реализацией. Отмечайте пункты фаз только по факту завершения работы.

## Интерпретация вывода

Ответ модели зависит от входа. Ниже иллюстрация плана, не квитанция создания:

```
Suggested tasks for PRD-042:
  Phase 1: Authentication provider abstraction (3 tasks)
  Phase 2: OAuth2 flow implementation (5 tasks)
  Phase 3: Session persistence + refresh (4 tasks)
  Phase 4: E2E tests + rollout gate (2 tasks)
```

Красные флаги:

- Одна фаза с 20 подзадачами - PRD слишком широк, разделите его на несколько PRD
- Фазы ссылаются на FR, которых нет в PRD - галлюцинация LLM, повторите запуск или отредактируйте
- Нет фазы отката/доказательства - добавьте ее вручную перед активацией

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

- [`forgeplan reason`](/docs/cli/reason/) - запустите цикл ADI перед декомпозицией
- [`forgeplan new`](/docs/cli/new/) - создайте исходный PRD
- [`forgeplan validate`](/docs/cli/validate/) - гейт качества для входных данных декомпозиции
- [`forgeplan generate`](/docs/cli/generate/) - генерирует контент для любого типа артефакта
- [Методология: процесс PRD → RFC](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan decompose --help`.

```text
Decompose a PRD into RFC tasks using AI

Usage: forgeplan decompose <ID>

Arguments:
  <ID>  PRD artifact ID to decompose

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
