---
title: forgeplan migrate-secrets
description: "PRD-077 FR-023 Part B - импортирует известные переменные окружения с API-ключами (GEMINI_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY) из окружения процесса в `.forgeplan/secrets.env`, чтобы ключи хранились локально в проекте, а не в вашем shell rc. По умолчанию работает в режиме dry-run; передайте `--apply` для записи"
---

`migrate-secrets` перемещает API-ключи из вашего глобального shell rc в локальный файл рабочего пространства `.forgeplan/secrets.env`. После запуска с `--apply` ключи остаются доступными для каждой команды в этом рабочем пространстве (порядок разрешения: сначала окружение процесса, затем файл), и вы можете безопасно удалить их из `~/.zshrc` или `~/.bashrc`.

Команда по умолчанию работает в режиме dry-run. Ничего не записывается, пока вы не передадите `--apply`. С `--apply` команда идемпотентна: существующие записи сохраняются, добавляются только отсутствующие, и рядом создаётся резервная копия существующего файла с временной меткой.

## Когда использовать

- Когда ваши API-ключи в данный момент находятся в shell rc и вы хотите изоляцию на уровне проекта.
- После клонирования рабочего пространства, где ожидается, что ключи будут в `.forgeplan/secrets.env`, но локально они отсутствуют.
- В документации по онбордингу, как первый шаг после `forgeplan init`.

## Когда НЕ использовать

- Для копирования встроенных значений `api_key: "..."` из `config.yaml`. Структура `LlmConfig` не имеет поля `api_key` - только `api_key_env`, которое содержит *имя* переменной окружения, а не значение. Встроенных значений для миграции нет.
- В качестве хранилища учётных данных. Файл в plaintext, в gitignore, и chmod 0600; он не зашифрован на диске.

## Что делает команда

1. Разрешает рабочее пространство: `--workspace` может указывать на корень проекта или на директорию `.forgeplan/`. Без флага выполняется поиск вверх от текущей рабочей директории. Завершается с кодом `2`, если рабочее пространство не найдено.
2. Загружает текущий `.forgeplan/secrets.env`, если он существует, в in-memory карту.
3. Проверяет три канонических ключа в порядке приоритета: `GEMINI_API_KEY`, затем `OPENAI_API_KEY`, затем `ANTHROPIC_API_KEY`. Каждый классифицируется в одно из четырёх состояний:
   - `AbsentInEnv`: не установлен, или установлен в пустое значение/только пробелы. Нечего мигрировать.
   - `AlreadyInFile`: присутствует и в окружении процесса, и в файле, со совпадающими значениями. Ничего не делается.
   - `Conflict`: присутствует в обоих местах с различающимися значениями. Значение из файла сохраняется; значение из окружения **не** сохраняется. Выполнение завершается с кодом `1`, чтобы расхождение было видно.
   - `WouldAdd`: присутствует в окружении процесса, но отсутствует в файле. Добавляется при `--apply`.
4. С `--apply` записывает новые записи атомарно (`tempfile + rename`) и устанавливает chmod файла в `0600` на Unix. Резервная копия с именем `.forgeplan/secrets.env.bak-<unix_timestamp>` создаётся, если файл уже существовал.
5. Выводит сводку по каждому ключу в текстовом режиме, или структурированный JSON-конверт с `--json`.

**Имена** ключей появляются в выводе, но **значения** ключей никогда. Сообщается только длина значения (например, `17 chars`), чтобы логи и захваты CI не раскрывали учётные данные.

## Почему локальный файл проекта вместо shell rc

Три причины, все основаны на контракте разрешения:

1. **Изоляция по проектам.** Ключи для работы и ключи для побочного проекта остаются раздельными. Скрипт, запущенный из `/tmp/acme-api`, не может случайно подхватить `GEMINI_API_KEY`, принадлежащий другому проекту.
2. **Нет риска коммита.** `forgeplan init` добавляет `.forgeplan/secrets.env` и `.forgeplan/secrets.env.bak-*` в `.gitignore` рабочего пространства. Ключи остаются локальными.
3. **Порядок поиска чётко определён.** `forgeplan_core::config::secrets::resolve_api_key` сначала проверяет окружение процесса (с учётом trim, поэтому `export KEY="   "` трактуется как неустановленный) и только затем обращается к файлу. Ключ, присутствующий в обоих местах, однозначен: окружение процесса побеждает.

После успешной миграции вы можете выполнить `unset GEMINI_API_KEY` (и два других) из `~/.zshrc` и загружать `.forgeplan/secrets.env` для каждого проекта (напрямую или через `direnv`).

## Использование

```text
forgeplan migrate-secrets [OPTIONS]
```

## Опции

```text
      --workspace <WORKSPACE>  Корень рабочего пространства, содержащий `.forgeplan/`. По умолчанию: текущая рабочая директория (поиск вверх, как в других командах)
      --apply                  Применить миграцию (записать `.forgeplan/secrets.env`). По умолчанию dry-run - только перечисляет, что будет мигрировано
      --json                   Вывести JSON-сводку в stdout вместо человекочитаемого текста. Полезно для скриптов / интеграции с CI
  -h, --help                   Вывести справку
  -V, --version                Вывести версию
```

## Коды завершения

- `0` - успех. Нечего мигрировать, или `--apply` завершился чисто.
- `1` - произошла хотя бы одна ошибка (не удалось записать резервную копию, не удалось записать файл, или конфликт значений окружение-vs-файл остался неразрешённым).
- `2` - рабочее пространство не найдено (нет `.forgeplan/` в текущей директории или в родительских).

## Примеры

### Пример 1: Dry-run с одним установленным ключом

Поведение по умолчанию - dry-run. Ничего не записывается; команда перечисляет, что произойдёт, и завершается.

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

Передайте `--apply`, чтобы фактически добавить запись в файл. Файл записывается атомарно и получает chmod `0600`.

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

Результат на диске:

```bash
ls -l .forgeplan/secrets.env*
# -rw------- 1 user user 689 ... secrets.env
# -rw-r--r-- 1 user user 460 ... secrets.env.bak-1789566786
```

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

Код завершения `0`. Безопасно запускать безусловно в CI.

### Пример 4: Конфликт окружение-vs-файл

Если один и тот же ключ установлен в обоих местах с разными значениями, файл побеждает, и значение из окружения не сохраняется. Выполнение завершается с кодом `1`, чтобы расхождение было видно - молчаливый no-op был багом до L1.

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

`_next_action` равен `null`, потому что выполнение терминально (конфликт не разрешается автоматически). При успешном dry-run с новыми записями `_next_action` равен `forgeplan migrate-secrets --apply`. После успешного `--apply` это `forgeplan reason <ID>`, чтобы агент мог проверить, что ключи разрешаются.

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

`.forgeplan/secrets.env` - это файл shell, который можно загрузить через `source`. `forgeplan init` записывает шаблон, который выглядит так (комментарии здесь удалены для краткости):

```bash
# secrets.env - локальные API-ключи. НИКОГДА не коммитьте этот файл.
# forgeplan читает ключи из переменных окружения, чьи ИМЕНА объявлены в config.yaml::llm.api_key_env.
# Этот файл - удобство: место для хранения ваших реальных ключей во время локальной разработки.
# Скопируйте любую строку ниже и раскомментируйте её (и выполните `source .forgeplan/secrets.env` или используйте direnv).

# export GEMINI_API_KEY=<your-key-here>
# export OPENAI_API_KEY=<your-key-here>
# export ANTHROPIC_API_KEY=<your-key-here>

# Импортировано из окружения процесса командой `forgeplan migrate-secrets`.
# Безопасно удалить эти строки из вашего shell rc, как только этот файл
# будет загружен (или как только forgeplan прочитает его напрямую согласно FR-023).
export GEMINI_API_KEY=<redacted>
```

Будущая ревизия может читать файл напрямую без source; до тех пор выполняйте `source .forgeplan/secrets.env` (или используйте `direnv`), чтобы загрузить ключи в shell.

## Примечания

- Набор ключей фиксирован: `GEMINI_API_KEY`, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, в этом порядке. Добавление нового провайдера требует изменения кода, а не флага.
- Значения окружения, состоящие только из пробелов (`export GEMINI_API_KEY="   "`), трактуются как неустановленные, соответствуя `resolve_api_key`. Это закрывает footgun класса L3-audit, где буквальный `   ` просачивался к провайдеру LLM как 3-символьный ключ.
- `secrets.env` получает chmod `0600` после каждой успешной записи. Снимок `.bak-<timestamp>` создаётся с существующим режимом файла и затем ротируется при следующем запуске.
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