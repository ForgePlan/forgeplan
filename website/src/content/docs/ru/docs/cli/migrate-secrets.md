---
title: forgeplan migrate-secrets
description: "PRD-077 FR-023 Part B - импортирует известные переменные окружения с API-ключами (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) из окружения процесса в `.forgeplan/secrets.env`, чтобы ключи хранились локально в проекте, а не в вашем shell rc. По умолчанию работает в режиме dry-run; передайте `--apply` для записи"
---

`migrate-secrets` копирует канонические API-ключи из окружения процесса в
`.forgeplan/secrets.env`, но не редактирует shell rc и не снимает переменные.
LLM-поиск ключей предпочитает окружение процесса, затем локальный файл.
Чтобы использовать файл, самостоятельно уберите непустое унаследованное значение.

По умолчанию dry-run. `--apply` добавляет отсутствующие записи, сохраняя
старые значения и комментарии. При добавлении в непустой существующий файл
сначала создаётся резервная копия с временной меткой.

## Когда использовать

- Когда ваши API-ключи в данный момент находятся в shell rc и вы хотите изоляцию на уровне проекта.
- После клонирования рабочего пространства, где ожидается, что ключи будут в `.forgeplan/secrets.env`, но локально они отсутствуют.
- В документации по онбордингу, как первый шаг после `forgeplan init`.

## Когда НЕ использовать

- Для копирования встроенных значений `api_key: "..."` из `config.yaml`. Структура `LlmConfig` не имеет поля `api_key` - только `api_key_env`, которое содержит *имя* переменной окружения, а не значение. Встроенных значений для миграции нет.
- В качестве хранилища учётных данных. Файл в plaintext, в gitignore, и chmod 0600; он не зашифрован на диске.

## Что делает команда

1. Без `--workspace` ищет вверх от текущего каталога и возвращает `2`, если пространство не найдено. В 0.38.0 явный `--workspace` используется буквально как **корень проекта**, без проверки наличия пространства; `.forgeplan/` направит запись в `.forgeplan/.forgeplan/secrets.env`. Передавайте корень, не каталог метаданных.
2. Загружает текущий `.forgeplan/secrets.env`, если он существует, в in-memory карту.
3. Проверяет три канонических ключа в порядке приоритета: `GEMINI_API_KEY`, затем `OPENAI_API_KEY`, затем `ANTHROPIC_API_KEY`. Каждый классифицируется в одно из четырёх состояний:
   - `AbsentInEnv`: не установлен, или установлен в пустое значение/только пробелы. Нечего мигрировать.
   - `AlreadyInFile`: присутствует и в окружении процесса, и в файле, со совпадающими значениями. Ничего не делается.
   - `Conflict`: присутствует в обоих местах с различающимися значениями. Значение из файла сохраняется; значение из окружения **не** сохраняется. Выполнение завершается с кодом `1`, чтобы расхождение было видно.
   - `WouldAdd`: присутствует в окружении процесса, но отсутствует в файле. Добавляется при `--apply`.
4. При `--apply` и новых записях пишет атомарно (`tempfile + rename`) с mode `0600` на Unix. Копия `.forgeplan/secrets.env.bak-<unix_timestamp>` создаётся только для непустого существующего файла. No-op не исправляет права и не создаёт копию.
5. Выводит сводку по каждому ключу в текстовом режиме, или структурированный JSON-конверт с `--json`.

**Имена** ключей появляются в выводе, но **значения** ключей никогда. Сообщается только длина значения (например, `17 chars`), чтобы логи и захваты CI не раскрывали учётные данные.

## Почему локальный файл проекта вместо shell rc

Три причины, все основаны на контракте разрешения:

1. **Локальный резервный источник.** У пространства может быть свой файл. Унаследованные ключи процесса всё равно приоритетнее; миграция сама по себе не изолирует shell и произвольные скрипты.
2. **Локальное хранение, не сейф.** `init` добавляет ignore-шаблоны файла и копий, но Git ignore не гарантирует конфиденциальность. Проверяйте отслеживаемые файлы и защищайте все копии.
3. **Порядок поиска чётко определён.** `forgeplan_core::config::secrets::resolve_api_key` сначала проверяет окружение процесса (с учётом trim, поэтому `export KEY="   "` трактуется как неустановленный) и только затем обращается к файлу. Ключ, присутствующий в обоих местах, однозначен: окружение процесса побеждает.

Forgeplan читает файл напрямую. `source` необязателен и экспортирует ключи
другим программам shell. Удаление присваивания из shell rc влияет на новые
shell-сессии; в существующей отдельно выполните `unset`.

## Использование

```text
forgeplan migrate-secrets [OPTIONS]
```

## Опции

```text
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)
      --apply                  Apply the migration (write `.forgeplan/secrets.env`). Default is dry-run — only lists what would be migrated
      --json                   Emit a JSON summary to stdout instead of human-readable text. Useful for scripting / CI integration
  -h, --help                   Print help
  -V, --version                Print version
```

## Коды завершения

- `0` - успех. Нечего мигрировать, или `--apply` завершился чисто.
- `1` - произошла хотя бы одна ошибка (не удалось записать резервную копию, не удалось записать файл, или конфликт значений окружение-vs-файл остался неразрешённым).
- `2` - рабочее пространство не найдено (нет `.forgeplan/` в текущей директории или в родительских).

## Примеры

### Пример 1: Dry-run с одним установленным ключом

Вывод ниже условный, с примерными путями и значениями; это не реальные ключи и не наблюдавшийся запуск. По умолчанию dry-run ничего не пишет.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets
```

```
forgeplan migrate-secrets - DRY-RUN
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  -> GEMINI_API_KEY: in process env (17 chars) - would add
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Would apply 1 new entries. Re-run with --apply to write.

Next: forgeplan migrate-secrets --apply
```

Код завершения `0`. Подсказка `Next:` следует PRD-071 и сообщает агенту (или шагу CI) точно, что запускать дальше.

### Пример 2: Применение миграции

Передайте `--apply` для добавления отсутствующих записей. Условная строка Backup ниже предполагает, что уже существовал непустой файл.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  -> GEMINI_API_KEY: in process env (17 chars) - added
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Applied: 1 new entries appended to secrets.env.
Backup: /private/tmp/acme-api/.forgeplan/secrets.env.bak-1789566786

Verify secrets.env contents are correct, then optionally unset
the migrated vars from your shell rc to enforce project-local isolation.

Next: forgeplan reason <ID>
```

Итоговый secrets-файл имеет mode `0600` на Unix. Копия сохраняет прежние
права файла; если он был доступен другим, копия тоже может быть доступна.
Защитите оба файла и не публикуйте списки с приватными путями.

Код завершения `0`.

### Пример 3: Повторный запуск - no-op

Повторный запуск `--apply` с тем же значением окружения идемпотентен. Существующая запись сохраняется, и новая резервная копия не создаётся.

```bash
GEMINI_API_KEY=demo-key-not-real forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  v GEMINI_API_KEY: already in secrets.env (values match) - skip
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Nothing to migrate. v

Done.
```

Код `0` относится к этому примеру без конфликтов. В CI остаются возможны конфликты и ошибки I/O.

### Пример 4: Конфликт окружение-vs-файл

При разных значениях env и файла прежнее значение файла сохраняется, а команда возвращает `1`, в том числе при dry-run. При LLM-вызове всё ещё приоритетен env: “file value kept” относится к записи, не к поиску ключа. Другие отсутствующие ключи в том же запуске `--apply` могут быть добавлены.

```bash
GEMINI_API_KEY=a-rotated-key-value forgeplan migrate-secrets --apply
```

```
forgeplan migrate-secrets - APPLY
workspace secrets file: /private/tmp/acme-api/.forgeplan/secrets.env

  ! GEMINI_API_KEY: env value (19 chars) DIFFERS from secrets.env - file value kept, env value NOT persisted
  . OPENAI_API_KEY: not set in process env - skip
  . ANTHROPIC_API_KEY: not set in process env - skip

Nothing to migrate. v

Done.
```

Код завершения `1`. Разрешите, отредактировав `.forgeplan/secrets.env` напрямую, или сняв переменную окружения для этого проекта.

### Пример 5: JSON для CI

Передайте `--json`, чтобы получить структурированный конверт, подходящий для дашбордов или гейтов CI. Поле `_next_action` следует контракту подсказок PRD-071.

```bash
GEMINI_API_KEY=a-rotated-key-value forgeplan migrate-secrets --json
```

```json
{
  "_next_action": null,
  "added_count": 0,
  "applied": false,
  "backup_path": null,
  "entries": [
    { "name": "GEMINI_API_KEY", "status": "conflict", "value_len": 19 },
    { "name": "OPENAI_API_KEY", "status": "absent_in_env", "value_len": null },
    { "name": "ANTHROPIC_API_KEY", "status": "absent_in_env", "value_len": null }
  ],
  "secrets_path": "/private/tmp/acme-api/.forgeplan/secrets.env"
}
```

Здесь `_next_action` равен null из-за отсутствия новых записей, не потому, что любой конфликт терминален. Подсказка зависит от `added_count` и `applied`; смешанный запуск может вернуть её и код `1`. Добавленные записи сохраняют JSON-статус `"would_add"`; факт записи показывает `applied`. Ранние ошибки пространства/чтения/записи могут идти обычным stderr даже при `--json`.

### Пример 6: Рабочее пространство не найдено

Вне рабочего пространства forgeplan команда завершается с кодом `2` и указывает на исправление.

```bash
cd /tmp
forgeplan migrate-secrets
```

```
Error: no `.forgeplan/` workspace found in current dir or any parent.
Fix: cd into a forgeplan workspace, or run `forgeplan init` to create one.
```

Код завершения `2`.

## Формат файла

`.forgeplan/secrets.env` принимает присваивания `[export ]NAME=value`. Ниже условный фрагмент: замените скрытое значение своим, это не буквальная команда для запуска:

```bash
# secrets.env - local-only API keys. NEVER commit this file.
# forgeplan reads keys from env vars whose NAMES are declared in config.yaml::llm.api_key_env.
# This file is a convenience: a place to keep your real keys during local dev.
# Copy any line below and uncomment it (and `source .forgeplan/secrets.env` or use direnv).

# export GEMINI_API_KEY=<your-key-here>
# export OPENAI_API_KEY=<your-key-here>
# export ANTHROPIC_API_KEY=<your-key-here>

# Imported from process env by `forgeplan migrate-secrets`.
# Safe to remove these lines from your shell rc once this file
# is sourced (or once forgeplan reads it directly per FR-023).
export GEMINI_API_KEY='REDACTED_EXAMPLE'
```

Forgeplan 0.38.0 читает файл напрямую при поиске LLM-ключа; `source` не нужен.
Загрузчик принимает значения с кавычками и без, но не выполняет shell expansion.
Мигратор заключает значения в одинарные кавычки и отклоняет ключ с одинарной кавычкой.

## Примечания

- Набор ключей фиксирован: `GEMINI_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, в этом порядке. Добавление нового провайдера требует изменения кода, а не флага.
- Значения окружения, состоящие только из пробелов (`export GEMINI_API_KEY="   "`), трактуются как неустановленные, соответствуя `resolve_api_key`. Это закрывает footgun класса L3-audit, где буквальный `   ` просачивался к провайдеру LLM как 3-символьный ключ.
- Успешная запись использует `0600`, резервная копия сохраняет прежние права. Копии имеют временную метку, но автоматически не ротируются и не удаляются.
- Запись атомарна через `tempfile + rename`. Сбой во время записи оставляет исходный файл нетронутым.
- Миграция выполняется для рабочего пространства, а не для пользователя. Разные клоны одного и того же проекта нуждаются в собственной миграции.

## Связанные команды

- [`forgeplan init`](/ru/docs/cli/init/) - создаёт рабочее пространство, шаблон `secrets.env` и записи `.gitignore`.
- [`forgeplan migrate`](/ru/docs/cli/migrate/) - компаньон миграции схемы (другая задача, несвязанный файл).
- [`forgeplan health`](/ru/docs/cli/health/) - дашборд начала сессии.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan init`](/ru/docs/cli/init/) - настраивает шаблон secrets.env
- [`forgeplan health`](/ru/docs/cli/health/) - проверка здоровья начала сессии
- [Начало работы](/ru/docs/getting-started/installation/) - настройка окружения shell
- [Руководство по методологии](/ru/docs/methodology/overview/)