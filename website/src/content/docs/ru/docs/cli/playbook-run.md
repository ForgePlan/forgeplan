---
title: forgeplan playbook run
description: "Запуск плейбука - последовательное выполнение шагов с делегированием"
---

`forgeplan playbook run <TARGET>` выполняет один плейбук. Шаги упорядочены по рёбрам зависимостей, объявленным в поле `requires:` каждого шага, и выполняются последовательно, один за другим. Шаг, предшественники которого не завершились успешно, пропускается, а не выполняется, а падающий шаг с `on_error: abort` тянет за собой остаток выполнения. Каждый переход добавляется в `.forgeplan/journal/playbook-runs.jsonl`, поэтому выполнение можно восстановить постфактум.

Два независимых гейта стоят между вами и выполнением. `--yes` подтверждает, что вы действительно хотели запустить плейбук. `--allow-shell` (или эквивалент в рабочем пространстве) отдельно авторизует шаги `delegate_to: command`, которые порождают произвольные процессы. Оба существуют, потому что плейбук - это данные, которые вы могли получить от кого-то другого; ни один не подразумевает другой.

## Что делает

1. **Отказывает без подтверждения.** Если `--yes` отсутствует и `--dry-run` не установлен, команда завершается с кодом 2 и подсказкой `Fix:` до разрешения чего-либо.
2. **Разрешает и загружает `TARGET`** - имя плейбука, найденное через обнаружение, или путь к `.yaml`-файлу - под ограничениями ввода среды выполнения (1 МиБ, 256 уровней вложенности скобок).
3. **Валидирует `--step N`** относительно количества шагов плейбука заранее, чтобы некорректная точка возобновления упала до выполнения любого шага.
4. **Прерывается на `--dry-run`.** План выводится, и команда возвращается без открытия журнала или построения диспетчера.
5. **Открывает журнал и строит диспетчер роутинга**, который распределяет каждый шаг на бэкенд, названный его `delegate_to:`.
6. **Разрешает эффективное разрешение shell** как флаг CLI ИЛИ значение конфига рабочего пространства, и выводит баннер в stderr, когда только конфиг рабочего пространства открыл гейт.
7. **Проходит по шагам в порядке зависимостей.** Перед каждой отправкой перепроверяет гейт shell; шаг `command:` без разрешения записывается как падение шага, а не молча пропускается.
8. **Выводит отчёт о выполнении** - `run_id`, счётчики `success`, `failed`, `skipped` плюс одна строка `[OK]` / `[FAIL]` / `[SKIP]` на шаг - и завершается с кодом 1, если любой шаг упал.

## Использование

```text
forgeplan playbook run [OPTIONS] <TARGET>

## Аргументы

```text
  <TARGET>  Имя или путь плейбука
```

## Опции

```text
      --yes          Подтвердить выполнение (обязательно - предотвращает случайные запуски)
      --allow-shell  Разрешить выполнение шагов `Delegation::Command` (shell-exec). PROB-053 / PRD-074 §FR-1: гейт с запретом по умолчанию для поверхности CWE-78. Подразумевает `--yes`. Альтернатива: установить `[playbook] allow_shell = true` в `config.yaml` рабочего пространства
      --dry-run      Вывести шаги без выполнения
      --step <STEP>  Начать с конкретного шага (нумерация с 1)
      --json         Вывод в формате JSON
  -h, --help         Вывести справку
  -V, --version      Вывести версию
```

## Гейт для shell

Шаг `delegate_to: command` запускает список argv как дочерний процесс — это поверхность CWE-78, которую материализует плейбук, если он написан кем-то другим. Поэтому runtime по умолчанию запрещает его согласно PROB-053 / PRD-074 FR-1, и гейт проверяется с обеих сторон: один раз CLI перед отправкой, один раз исполнителем для каждого шага.

Когда гейт закрыт, шаг **проваливается** с соответствующим сообщением:

```
step `say-hello` uses `delegate_to: command` - pass `--allow-shell` (or set `[playbook] allow_shell = true` in workspace config) to acknowledge arbitrary shell execution
```

Это именно провал, а не пропуск, и он учитывается в счётчике `failed` — поэтому запуск, который наткнулся на гейт, завершается с кодом 1 и не может быть ошибочно принят за чистый запуск в конвейере.

### Открытие гейта для одного вызова

Передайте `--allow-shell`. Каждый шаг команды затем выводит предупреждающую строку в stderr перед
запуском и выполняет:

```bash
forgeplan playbook run demo-shell --yes --allow-shell
```

```
Running playbook: demo-shell (2 steps)
! shell-exec: echo hello-from-playbook
! shell-exec: echo second

Run summary: demo-shell
  run_id:  0868f4bfd100a13d
  success: 2
  failed:  0
  skipped: 0

  [OK] say-hello
  [OK] after

Done.
```

Предупреждение отображает полный argv с экранированными управляющими символами, поэтому YAML, который внедряет ANSI-
последовательности в команду, не может подделать или замаскировать строку, которую вы читаете. Шаги команд выполняются с
`cwd`, установленным в корень рабочего пространства, строгим списком разрешённых переменных окружения и таймаутом по умолчанию в 180 секунд.

### Открытие гейта для рабочего пространства

Альтернативой является блок конфигурации рабочего пространства, который представляет собой зафиксированный dotfile и, следовательно, является
ножом, который режет с обеих сторон:

```yaml
playbook:
  allow_shell: true

С этим на месте, один `--yes` удовлетворяет оба гейта - согласие на shell теперь приходит из файла
- и CLI громко сообщает об этом при запуске:

```bash
forgeplan playbook run demo-shell --yes
```

```
!! shell-exec: AUTO-APPROVED via [playbook] allow_shell=true in .forgeplan/config.yaml. To force-deny for this run, set the config to false (no CLI override flag exists yet — tracked as PRD-074 follow-up).
Running playbook: demo-shell (2 steps)
! shell-exec: echo hello-from-playbook
! shell-exec: echo second

Run summary: demo-shell
  run_id:  8dfa6c290b4dffcf
  success: 2
  failed:  0
  skipped: 0

  [OK] say-hello
  [OK] after

Done.
```

Блок workspace открывает гейт shell и больше ничего: `--yes` всё ещё обязателен, и запуск
без него отклоняется точно так же, как и раньше, даже в рабочем пространстве, где установлено согласие.

Нет CLI-флага, который *сузил бы* согласие из конфига обратно до отказа для одного запуска - баннер
сообщает вам, когда гейт был открыт файлом, а не вашей командной строкой. Установите конфиг
в `false` вместо того, чтобы заклеивать это флагами.

### Два гейта, а не один

`--yes` и `--allow-shell` - независимые подтверждения, и runtime проверяет оба в
этом порядке. Сгенерированный текст `--help` для `--allow-shell` говорит, что он подразумевает `--yes`; runtime
этого не делает, и это намеренно:

```bash
forgeplan playbook run demo-shell --allow-shell
```

```
Error: playbook run requires --yes confirmation (ADR-009 security gate).
Fix: forgeplan playbook run demo-shell --yes --allow-shell
Note: drop --allow-shell if the playbook contains no `delegate_to: command` steps (PRD-074 §FR-1).
```

Код выхода 2. Передайте оба флага - подсказка, которую выводит отказ, уже содержит пару. Исходный код фиксирует причину разделения: в более ранней сборке два флага были объединены в `yes || allow_shell`, что превращало единственный переключатель в подтверждение двух разных рисков.

И наоборот, `--yes` сам по себе **не** авторизует шаги с shell. Он подтверждает запуск, и шаг с командой без явного разрешения на shell всё равно завершится ошибкой. Подсказка об отказе добавляет `--allow-shell` безусловно, потому что CLI не может знать, содержит ли playbook шаги с командами, пока не загрузит YAML.

## Примеры

### Прочитать план перед запуском

```bash
forgeplan playbook run greenfield-kickoff --yes --dry-run
```

```
Dry-run: greenfield-kickoff (7 steps)
Source:  /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml

  [1] capture-vision: delegate=forgeplan_core:new
  [2] stack-decision: delegate=forgeplan_core:new
  [3] kickoff-epic: delegate=forgeplan_core:new
  [4] prd-feature-1: delegate=forgeplan_core:new
  [5] prd-feature-2: delegate=forgeplan_core:new
  [6] prd-feature-3: delegate=forgeplan_core:new
  [7] scaffold-docs: delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)

```bash
forgeplan playbook run greenfield-kickoff --yes
```

`--dry-run` ничего не выполняет и не требует `--yes` (удаление флага даёт идентичный
вывод). Строка `Next:` убирает `--dry-run`, что является единственным различием между чтением и
выполнением этой команды.

### Закрытие гейта на реальном шаге команды

С плейбуком, содержащим шаги `command:`, одного `--yes` недостаточно. Шаг проваливается, а
зависящие от него шаги пропускаются:

```bash
forgeplan playbook run demo-shell --yes
```

```
Running playbook: demo-shell (2 steps)

Run summary: demo-shell
  run_id:  a99614e19a1ee275
  success: 0
  failed:  1
  skipped: 1

  [FAIL] say-hello
       step `say-hello` uses `delegate_to: command` - pass `--allow-shell` (or set `[playbook] allow_shell = true` in workspace config) to acknowledge arbitrary shell execution
  [SKIP] after
       skipped: prior step failed with on_error: abort

Next: forgeplan playbook show demo-shell
```

Код выхода 1. Обратите внимание на контракт отчётности: провал приписывается шагу, требующему
подтверждения, второй шаг помечен как `skipped`, а не как проваленный, а `Next:` указывает на `show` — то же
продолжение, которое инструмент предлагает всякий раз, когда выполнение было нечистым.

### Машиночитаемый отчёт

```bash
forgeplan playbook run demo-shell --yes --allow-shell --json
```

```json
{
  "_next_action": null,
  "playbook": "demo-shell",
  "report": {
    "failed": 0,
    "per_step": [
      {
        "status": "success",
        "step_id": "say-hello"
      },
      {
        "status": "success",
        "step_id": "after"
      }
    ],
    "run_id": "02bc11f2d5d57138",
    "skipped": 0,
    "success": 2
  }
}
```

`_next_action` имеет значение `null` при успешном выполнении и содержит имя последующей команды в противном случае. В JSON-режиме человекочитаемые строки прогресса (`Running playbook:`, предупреждения `! shell-exec:`) подавляются, поэтому stdout содержит ровно один JSON-документ — безопасно для передачи в `jq`.

### Возобновление с более позднего шага

```bash
forgeplan playbook run demo-shell --yes --allow-shell --step 2
```

```
Run summary: demo-shell
  run_id:  577420d7dec981a9
  success: 0
  failed:  0
  skipped: 2

  [SKIP] say-hello
       skipped via --step 2
  [SKIP] after
       skipped: predecessor not successful

Next: forgeplan playbook show demo-shell
```

Код выхода 0 — запуск, состоящий только из пропущенных шагов, не является ошибкой, поскольку пропуск — это обычный результат политики зависимостей. Но внимательно прочитайте эти две строки, прежде чем полагаться на `--step` для возобновления частично завершённого плейбука: пропущенные шаги записываются как `skipped`, а не как `success`, поэтому любой последующий шаг, который указывает `requires:` на один из них, также пропускается. В строго линейном плейбуке возобновление с шага 2 поэтому пропускает остаток выполнения, а не выполняет его. `--step` полезен в основном для плейбуков, чьи последующие шаги зависят от *подмножества* предыдущих.

Значение вне диапазона перехватывается до выполнения чего-либо:

```bash
forgeplan playbook run greenfield-kickoff --yes --step 99
```

```
Error: --step out of range: requested 99, playbook has 7 step(s) (1..=7)
Fix: forgeplan playbook show greenfield-kickoff

`--step` индексируется с 1, и `0` находится вне диапазона так же, как и `99`.

## Коды завершения

| Код | Значение |
|---|---|
| `0` | Каждый шаг выполнен успешно, или выполнение только пропускало шаги |
| `1` | Хотя бы один шаг завершился неудачей (включая отказ shell-gate) |
| `2` | Выполнение было отклонено до запуска: отсутствует `--yes`, неразрешимая цель, нечитаемый или невалидный YAML, или `--step` вне диапазона |

Завершение с кодом 1 при частичной неудаче критично: конвейер, запускающий `forgeplan playbook run ... --yes` в CI, не должен считать сломанный плейбук успешным. Выполнения только с пропусками остаются на 0, потому что пропуски - это обычное дело.

## Примечания

- **Где записываются выполнения.** `.forgeplan/journal/playbook-runs.jsonl`, один JSON-объект на строку: `run_start` (с разрешёнными значениями `allow_shell` и `yes_flag` и количеством шагов), `step_start`, `step_end` (со статусом, сообщением и путём к выводу) и `run_end` (с финальными счётчиками). ID выполнения, напечатанный в отчёте, соответствует полю `run_id` в каждой записи.
- **Прогресс против отчёта.** `Running playbook:` и предупреждения `! shell-exec:` идут в stderr; отчёт идёт в stdout. Перенаправьте stderr, чтобы оставить только результат.
- **Подсказка `Fix:` заключает TARGET, содержащий пробелы, в двойные кавычки**, чтобы предложенная команда была готова к копированию.
- **`--json` не меняет выполнение.** Он меняет только рендеринг; гейты, порядок и коды завершения идентичны.
- **Делегаты отправляются через слой роутинга**, который покрывает шаги plugin, agent, skill, command и `forgeplan_core`, поэтому смешанному плейбуку не нужен отдельный вызов для каждого делегата.

## См. также

- [`forgeplan playbook`](/ru/docs/cli/playbook/) - родительская команда и обзор семейства
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - чтение плана без выполнения
- [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) - обнаружение установленных плейбуков
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - проверка схемы перед запуском
- [`forgeplan health`](/ru/docs/cli/health/) - гейт начала сессии, который вызывает большинство релизных плейбуков