---
title: forgeplan validate
description: "Проверяет структуру артефакта по правилам MUST/SHOULD для его типа и глубины. Ошибки MUST возвращают код 1 и в обычном режиме, и в CI; одни SHOULD остаются предупреждениями. Это проверка структуры, не правильности решения или реализации."
---

Проверяет структуру артефакта по правилам MUST/SHOULD для его типа и глубины. Ошибки MUST возвращают код 1 и в обычном режиме, и в CI; одни SHOULD остаются предупреждениями. Это проверка структуры, не правильности решения или реализации.

## Использование

```text
forgeplan validate [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  Artifact ID (validates all if omitted)
```

## Опции

```text
      --json         Output as JSON for machine consumption
      --adversarial  Run adversarial (devil's advocate) review
      --ci           CI mode: exit code 1 if any MUST rules fail
  -h, --help         Print help
  -V, --version      Print version
```

## Выбор артефактов и CI

Без ID обычный `validate` проверяет все записи, а `validate --ci` выбирает только `active` и `stale`. Остальные статусы, включая черновики, полностью пропускаются: предупреждения по ним не выводятся. `validate ID --ci` проверяет явно указанный артефакт независимо от статуса, поэтому ошибки MUST черновика тоже приводят к отказу.

## Примеры

```bash
forgeplan validate PRD-001
forgeplan validate --ci
forgeplan validate PRD-001 --ci
forgeplan validate ADR-005 --adversarial
# Ordinary JSON output, without the successful-CI trailer:
forgeplan validate PRD-001 --json | jq '.results[].errors'
```

## JSON и код завершения

Обычный JSON содержит `results` и `_next_action`. Поля результата: `artifact_id`, `kind`, `depth`, `passed`, целые `errors` и `warnings`, массив `findings` с `rule_id`, `severity`, `message`, `section`. Поля `must_violations` нет. Успешный `--json --ci` дописывает в stdout строку `CI PASSED …`, поэтому весь вывод не является одним JSON-документом. Пустое пространство печатает `No artifacts found.` даже с `--json`. При передаче в jq сохраняйте код завершения исходной команды: успех jq не означает прохождение валидации.

## Состязательные проверки и побочные эффекты

`--adversarial` добавляет детерминированные проверки текста из встроенного модуля; не вызывает LLM и не требует более сильной модели при отсутствии замечаний. Успех для явно указанного ID может перевести методологическую сессию в coding, но не активирует артефакт. Активация дополнительно проверяет длину, связи с доказательствами и шаблонное содержимое, если не задан явный force.

<a id="ci-mode---ci"></a>

## Интеграция CI

```yaml
- name: Forgeplan validate gate
  run: |
    forgeplan reindex
    forgeplan validate --ci
```

Черновики здесь исключены намеренно. Если политика требует проверить конкретный черновик, передайте его ID отдельно. Используйте `health --ci` для документированных порогов состояния; не считайте, что health по умолчанию отклоняет каждый stale или at_risk.

## См. также

- [`forgeplan review`](/ru/docs/cli/review/) - добавляет чек-лист жизненного цикла поверх валидации
- [`forgeplan activate`](/ru/docs/cli/activate/) - гейт, который питает `validate`
- [`forgeplan score`](/ru/docs/cli/score/) - качество за пределами структурных правил
- [Гейты качества](/ru/docs/methodology/evidence/) - философия MUST/SHOULD/состязательной ревью
- [Обзор CLI](/ru/docs/cli/)