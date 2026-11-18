---
title: forgeplan calibrate-estimate
description: "Сравнение оценочных и фактических часов, а также измерение отклонения оценки после спринта."
---

`forgeplan calibrate-estimate` повторяет расчёт оценки и сравнивает один итог
с положительным конечным числом фактических часов. `--grade` выбирает итог грейда;
без него берётся общий балл. Выводятся отношение actual/estimated и точность,
но не фактические часы по FR, сохранённый набор калибровки или автоматическая правка config.

Без калибровки оценки незаметно отклоняются: вы думаете, что вы «сеньор в бэкенде», но
постоянно превышаете оценку на 50%. Эта команда выявляет этот разрыв, чтобы вы могли
скорректировать множитель, а не только свою интуицию.

## Когда использовать

- Конец спринта: запускайте для каждого закрытого PRD/RFC, чтобы создать набор данных для калибровки.
- После неожиданности (значительное превышение/недовыполнение) - немедленная обратная связь, пока контекст свеж.
- Ежеквартальный обзор — сохраняйте текстовые отчёты и меняйте профиль осознанно; у этой команды нет `--json`.
- Бенчмаркинг нового профиля грейдов - сравнение фактических показателей с различными предположениями о грейдах.

## Когда НЕ использовать

- В середине спринта для незавершённого артефакта - фактические данные должны быть окончательными.
- Для артефактов без элементов FR/Phase - первоначальная оценка была предположением, а не моделью.

## Использование

```text
forgeplan calibrate-estimate [OPTIONS] --actual-hours <ACTUAL_HOURS> <ID>
```

## Аргументы

```text
  <ID>  ID артефакта для калибровки
```

## Опции

```text
      --actual-hours <ACTUAL_HOURS>  Фактически затраченные часы
      --grade <GRADE>                Грейд для сравнения (junior, mid, senior). По умолчанию - общий балл
  -h, --help                         Вывести справку
  -V, --version                      Вывести версию
```

## Примеры

### Калибровка одного PRD после закрытия спринта

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18
```

Гипотетический фрагмент при оценке 13.5 ч, не результат прогона:

```text
Estimated:  13.5h
Actual:     18.0h
Ratio:      1.33x (actual / estimated)
Accuracy:   75%
```

### Сравнение с другим грейдом

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18 --grade middle
```

Гипотетический фрагмент при оценке middle 22 ч:

```text
Estimated:  22.0h
Actual:     18.0h
Ratio:      0.82x (actual / estimated)
Accuracy:   82%
```

### Агрегация по всему спринту

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan calibrate-estimate "$id" --actual-hours "$(cat actuals/$id.txt)"
done
```

Передайте результаты в скрипт для расчёта среднего отклонения по грейду/домену и предложения новых
множителей для `.forgeplan/config.yaml`.

## Интерпретация вывода

`ratio = actual / estimated`; точность — `min(ratio, 1 / ratio) × 100%`.
Реализация даёт совет об оптимистичной оценке выше 1.1x, более сильный совет
о запасе выше 1.5x; ниже 0.7x — о консервативной оценке, ниже 0.5x — усиленный совет.

Это эвристика оценки, не доказательство неверной глубины или изменения объёма.
Перед настройкой профиля выясните, что изменилось. Набор сопоставимых завершённых
работ полезнее одного результата.

## Как это вписывается в рабочий процесс

```
закрытие спринта → calibrate-estimate (для каждого артефакта) → настройка grade_profile → следующий спринт точнее
```

Представьте это как телеметрию для вашей собственной оценки. CLI не настраивает конфигурацию
автоматически - вы просматриваете отклонение и решаете, что изменить.

## См. также

- [`forgeplan estimate`](/ru/docs/cli/estimate/) - исходная оценка
- [Конфигурация](/ru/docs/getting-started/configuration/) - настройки профиля
- [Единый рабочий процесс](/ru/docs/guides/git-workflow/)
- [Обзор CLI](/ru/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan calibrate-estimate --help`.

```text
Compare estimated vs actual hours — calibrate estimation accuracy

Usage: forgeplan calibrate-estimate [OPTIONS] --actual-hours <ACTUAL_HOURS> <ID>

Arguments:
  <ID>  Artifact ID to calibrate

Options:
      --actual-hours <ACTUAL_HOURS>  Actual hours spent
      --grade <GRADE>                Grade to compare (junior, mid, senior). Defaults to total score
  -h, --help                         Print help
  -V, --version                      Print version
```
<!-- forgeplan-runtime:end -->
