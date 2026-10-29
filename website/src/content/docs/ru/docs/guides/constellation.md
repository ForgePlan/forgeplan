---
title: Constellation — работа с несколькими проектами
description: Подключить два проекта, проверить общий поиск артефактов, сохранить запись через MCP и проверить автоматический embed.
sidebar:
  order: 3
---

Constellation ищет инженерные артефакты сразу в нескольких `.forgeplan/`. Это **не** автоматическая индексация всех файлов исходного кода. У каждого проекта остаются свои канонические Markdown-документы и производный индекс; пользовательский реестр связывает проекты для общего чтения.

:::caution[Неопубликованный интерфейс]
Гайд относится к локальной сборке `0.39.0+constellation.s1.1`, а не к опубликованной `0.38.0`. Нужен бинарник из ветки Constellation. При сборке из исходников включите `semantic-search`, если нужны векторы. Этот сценарий проверяет ограниченный набор операций, а не все платформы, миграции и runtime-гарантии.
:::

## 1. Проверить выбранный бинарник

```bash
forgeplan --version
forgeplan constellation --help
forgeplan serve --help
```

В справке сервера должны быть `--profile`, `--auto-index` и `--workspace`. Установка стабильного релиза не даёт неопубликованную функцию.

Для поиска по словам не нужны LLM-ключ и загрузка модели. Для векторов заранее подготовьте полный кэш BGE-M3. Если нужна только модель, используйте `forgeplan setup --skip-alias --skip-hooks`: так подготовка не добавит псевдоним команды и Git hooks. На **новом** workspace команда `forgeplan init -y --with-model` явно разрешает загрузку примерно 2,1 GB. Обычный `init -y` модель не скачивает. Не применяйте `init --force` ради модели: он сбрасывает активную конфигурацию.

Кэш берётся из `FORGEPLAN_MODEL_CACHE`, если переменная задана, иначе из `~/.forgeplan/models`. `HF_HOME` не переопределяет этот кэш tract. Резервные пути и условия описаны в [конфигурации](/ru/docs/getting-started/configuration/).

## 2. Создать два временных проекта

Выполните команды в **отдельной терминальной оболочке**. Собственный `HOME` отделит учебный реестр от настоящих проектов. Оставьте эту оболочку для следующих шагов; `exit` вернёт окружение родительской оболочки.

```bash
set -eu
MODEL_CACHE="${FORGEPLAN_MODEL_CACHE:-$HOME/.forgeplan/models}"
DEMO=$(mktemp -d "${TMPDIR:-/tmp}/forgeplan-docs.XXXXXX")
export HOME="$DEMO/home"
export FORGEPLAN_MODEL_CACHE="$MODEL_CACHE"
export FORGEPLAN_WORKSPACE=""
mkdir -p "$HOME" "$DEMO/alpha" "$DEMO/beta"

for name in alpha beta; do
  (
    cd "$DEMO/$name"
    git init --quiet
    forgeplan init -y
    if [ ! -f .forgeplan/config.yaml ]; then
      printf '%s\n' 'Refusing artifact writes: init did not create the local workspace. Choose TMPDIR outside any existing workspace.' >&2
      exit 1
    fi
    forgeplan new note "Constellation pilot marker $name"
    printf '# Pilot %s\n\n## Decision\n\nconstellation-pilot-marker belongs to %s.\n\n## Verification\n\nDistinguish projects by their source identity.\n' \
      "$name" "$name" > "$DEMO/$name-body.md"
    forgeplan update NOTE-001 --body @"$DEMO/$name-body.md"
    forgeplan index
  )
done

forgeplan constellation create documentation-demo
forgeplan constellation use documentation-demo
forgeplan constellation add "$DEMO/alpha/.forgeplan" --to documentation-demo
forgeplan constellation add "$DEMO/beta/.forgeplan" --to documentation-demo
forgeplan constellation sources
forgeplan constellation doctor
printf 'DEMO=%s\nMCP_HOME=%s\nMODEL_CACHE=%s\n' "$DEMO" "$HOME" "$FORGEPLAN_MODEL_CACHE"
```

В каждом новом проекте получится свой `NOTE-001`. Если адаптируете пример к существующим данным, используйте ID, который вернул `new`. Регистрируйте каталог **`.forgeplan/`**, а не весь checkout.

`doctor` должен показать оба источника как indexed и healthy. Это проверка доступности источника и индекса, не готовности векторов или качества поиска. У `doctor` и `sources` в этой сборке нет флага `--json`.

Если `init` показал родительский workspace вместо нового проекта, остановитесь. Поиск `.forgeplan/` идёт вверх по дереву; продолжать запись в чужой проект нельзя.

## 3. Проверить поиск и неоднозначность ID

```bash
forgeplan constellation search constellation-pilot-marker --json
forgeplan constellation get NOTE-001 --json
```

Проверьте разобранный JSON, а не только exit code:

- Поиск вернул два ожидаемых заголовка, разные `source.source_id` и правильные пути.
- `failures` пуст; дополнительно проверены `partial` и `truncated`.
- Get без уточнения вернул `result: "ambiguous"`, а не выбрал проект произвольно. Такой ответ может иметь exit code 0.

Уточните ID через `source_id` из этого ответа, например `NOTE-001@src:<возвращённый-хеш>`. Уникальный сегмент пути вроде `@alpha` тоже может выбрать нужный источник; в настоящей группе сначала убедитесь, что сегмент уникален.

Чтобы сравнить локальный документ с файлом, выполните обычный `get` из проекта:

```bash
(cd "$DEMO/alpha" && forgeplan get NOTE-001 --json)
```

Его `body` содержит **весь канонический Markdown**, включая frontmatter. У чтения опубликованной строки через федерацию другой контракт; см. [матрицу чтений](/ru/docs/reference/constellation/#контракты-чтения).

## 4. Сохранить запись через MCP и проверить авто-embed

`full` остаётся обычным MCP-профилем с записью. `constellation-read` — дополнительное подключение, а не его замена.

Для семантической части включите этот **активный** блок в `.forgeplan/config.yaml` проекта alpha, сохранив остальные настройки:

```yaml
embedding:
  model: bge-m3
  chunk_size: 2000
```

Нужны все условия: сборка с `semantic-search`, активная embedding-конфигурация, полный кэш модели и работающий процесс обслуживания **alpha**. Разовый `embed` эту настройку не сохраняет. `chunk_size` — не токеновый предел каждого окна секции.

Настройте MCP-клиент на выбранный бинарник. Замените пути ниже напечатанными абсолютными путями песочницы и реальным путём программы: MCP не запускает оболочку для подстановки `$DEMO` внутри JSON.

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "/absolute/path/to/forgeplan",
      "args": ["serve", "--profile", "full", "--auto-index", "--workspace", "/absolute/path/to/demo/alpha"],
      "env": {
        "HOME": "/absolute/path/to/demo/home",
        "FORGEPLAN_WORKSPACE": "/absolute/path/to/demo/alpha",
        "FORGEPLAN_MODEL_CACHE": "/absolute/path/to/model-cache",
        "HF_HUB_OFFLINE": "1"
      }
    },
    "forgeplan-constellation": {
      "command": "/absolute/path/to/forgeplan",
      "args": ["serve", "--profile", "constellation-read"],
      "env": { "HOME": "/absolute/path/to/demo/home", "FORGEPLAN_WORKSPACE": "/absolute/path/to/demo/alpha" }
    }
  }
}
```

Используйте `HF_HUB_OFFLINE=1` только после полной подготовки кэша. Второму подключению нужен тот же `HOME` с реестром. Перезапустите клиент, чтобы он действительно поднял новый процесс. `serve` — сервер stdio MCP, а не интерактивная shell-команда, которая сама напечатает итог проверки.

Явно задайте `FORGEPLAN_WORKSPACE`, как в примере: у внешнего клиента своё окружение. На старте full-сервера значение приоритетнее `--workspace`; поздний запрос не перемещает хост. В shell песочницы переменная экспортирована пустой, а не снята: так загрузчик `.env` не восстановит чужую привязку. Resolver пропускает явно пустое значение.

Через **full**-подключение вызовите `forgeplan_update`, явно выбрав alpha:

```json
{
  "id": "NOTE-001",
  "workspace": "/absolute/path/to/demo/alpha",
  "body": "# Pilot alpha\n\n## Decision\n\nconstellation-pilot-marker belongs to alpha. constellation-auto-refresh-marker was written through full MCP.\n\n## Verification\n\nThe background worker must publish usable vectors without a manual embed.\n"
}
```

Затем в оболочке песочницы:

```bash
(cd "$DEMO/alpha" && forgeplan search constellation-auto-refresh-marker --semantic --json)
```

Embedding работает асинхронно. Начальный ответ `No embedding profile is published` — **не** успешная семантическая проверка; дождитесь обслуживания, а при сохранении ошибки изучите stderr процесса. Успешный ответ должен иметь `semantic_available: true`, `semantic_scope: "prefix+sections"` и `eligible_rows == expected_rows > 0`. Проверьте релевантность нужного документа или секции. В строки входят префикс артефакта и окна секций, поэтому их число не равно числу документов.

Не вставляйте ручной `embed` между правкой и проверкой: это уже не тест автоматики. Если нужен явный первоначальный пересчёт, остановите фонового писателя, выполните `forgeplan embed` с настроенной моделью в кэше, затем возобновите обслуживание. Сам запуск сервера не доказывает, что начальная очередь обработана.

Один full-сервер обслуживает один выбранный workspace, а не все источники реестра. Для автоматического обновления векторов beta нужен отдельный процесс обслуживания beta.

Начните боевой пилот с одного workspace и измерьте потребление ресурсов перед добавлением остальных. Общие файлы модели не означают один общий движок инференса для отдельных серверных процессов.

## 5. Прочитать данные через handles

На **read**-подключении выполните последовательность ниже. JSON содержит аргументы инструментов, не полные JSON-RPC сообщения. Подставляйте значения handles, возвращённые тем же процессом сервера.

1. `constellation_open`:

   ```json
   { "constellation": "documentation-demo" }
   ```

2. Возьмите `scope_handle` из ответа и вызовите `constellation_search`:

   ```json
   { "scope_handle": "<возвращённый scope_handle>", "query": "constellation-pilot-marker" }
   ```

3. Возьмите ID с источником из этого результата и вызовите `constellation_get`:

   ```json
   { "scope_handle": "<возвращённый scope_handle>", "id": "NOTE-001@src:<источник из этого ответа>" }
   ```

4. Закройте handle через `constellation_close`:

   ```json
   { "handle": "<возвращённый scope_handle>" }
   ```

У read-подключения девять инструментов; `forgeplan_update` отклоняется как `tool not found`. Редактировать исходные артефакты через него нельзя. Если scope истёк или версия источника изменилась, откройте новый scope, а не выдавайте свежие байты за прежний ответ.

Для Git snapshot сначала закоммитьте учебные артефакты, затем откройте **новый** scope и вызовите `constellation_snapshot` с его handle. Ищите через возвращённый `snapshot_handle`. **Снова возьмите source IDs из ответа snapshot**: они могут отличаться от live-источников. Live-квалификатор в snapshot может дать `error: "no_source"`. Snapshot читает закоммиченный Git-текст, не последующие незакоммиченные правки. Закройте snapshot и его scope отдельно.

Только для этих временных проектов создайте коммиты с явной учебной идентичностью, не меняя глобальную настройку Git:

```bash
for name in alpha beta; do
  git -C "$DEMO/$name" add .forgeplan
  git -C "$DEMO/$name" -c user.name="Eli Rum" \
    -c user.email="documentation-smoke@example.invalid" \
    commit -m "docs: record isolated snapshot fixture"
done
```

## 6. Перед подключением настоящих проектов

- Сохраните целиком `.forgeplan/`, MCP config клиента и реестр вне workspace. Оставьте прежний бинарник; при откате не потеряйте новые канонические изменения.
- До миграции остановите старых писателей. Старый и новый бинарники не должны писать в один физический индекс; оставьте одного владельца обслуживания.
- Осознанно выбирайте required-источники. `--optional` подходит лишь там, где отсутствие допустимо; не скрывайте сбой required-источника постоянным partial-режимом.
- Заранее выберите реальные вопросы и ожидаемые артефакты. Проверяйте источник, полный текст, свежесть и покрытие векторов; пустая выдача не доказывает отсутствия решения.
- Используйте бинарник из выбранного среза исходников Constellation; не открывайте выведенные из эксплуатации таблицы старым бинарником. Проверенное покрытие совместимости не подтверждает каждый инструмент, семантический путь или платформу. Общие ограничения остаются в [#538](https://github.com/ForgePlan/forgeplan/issues/538).

В [справочнике Constellation](/ru/docs/reference/constellation/) собраны диагностика, snapshot-семантика, пределы ContextPack и точные каталоги команд и инструментов. Этот гайд не выполняет боевую миграцию и не подтверждает полную приёмку S1-G/S2.
