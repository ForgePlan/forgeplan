# Проверка Codex reasoning effort на 0.38.0 — 2026-11-01

## Ревизия и область

Ветка `fix/codex-effort-v038-20261101`, база
`e3b809f5833d13133c4e4a5c5b1960385e2a7a82` (`origin/dev`, обновлён через fetch).
Release-тег `v0.38.0` указывает на `d56466b10ddf788c62a6e8acffd0e0f578d0c1d1`.
Исправление перенесено из `15f00298c8dd8699a8636607cfdf09a62611555d`.
Точный коммит переноса — коммит, добавляющий этот отчёт.

Это повторная проверка на актуальной базе 0.38.0, а не смена номера версии
в старой ветке. Сохранены изменения базы, включая Devin `--permission-mode auto`.
Конфликт документации разрешён с сохранением обеих независимых вставок;
новая запись changelog находится в Unreleased, не в истории выпущенной версии.

`llm.effort` — необязательное типизированное поле. Пропуск/null сохраняет
прежний argv; строка `none` явно передаёт значение. Другие провайдеры поле
не используют. Основной dirty checkout не использовался для правок.
Глобальные настройки, credentials, установленный бинарник и GertsSystem
не изменялись. Push и установка не выполнялись.

## Проверки на новой базе

- `cargo fmt --all --check` — exit 0.
- `cargo check --workspace` — exit 0.
- `cargo test -p forgeplan-core --lib llm::local_cli::tests` — 13 passed.
- `cargo test -p forgeplan-core --lib config::` — 77 passed.
- `cargo build -p forgeplan --locked` — exit 0.

Полный workspace test suite и сборка с `semantic-search` не запускались.
Проверен debug-бинарник стандартной сборки. Это не проверка release packaging.

Из-за ранее обнаруженного отсутствия abseil dylib у Homebrew `protoc`
для команд Cargo использованы только переменные окружения процесса:

```sh
PROTOC=/Users/explosovebit/.local/pipx/venvs/optimum/lib/python3.13/site-packages/torch/bin/protoc
PROTOC_INCLUDE=/opt/homebrew/Cellar/protobuf/29.3/include
CARGO_BUILD_JOBS=2
```

Пакеты не устанавливались. Сборка и тесты выполнялись последовательно.

## Независимое ревью

Агент `Effort038Review`, роль `reviewer`, настроенный маршрут
`openai-codex/gpt-6.1-sol:high`. Вердикт **PASS только для ревью исходников**,
блокирующих замечаний нет. Проверены staged и working-tree diff относительно
новой базы, production-вызовы, копирование конфигурации ADI, границы argv,
тесты, сохранение Devin auto и документация. Ревьюер не правил файлы
и не запускал сборку. Provider-effective effort ревьюера не установлен.

Замечания к интеграции выполнены: старый отчёт заменён результатами этого
прогона; исправленная запись changelog включается в финальный коммит.

Проверенные SHA-256 исходников:

```text
106f94524eb70c94cf2ad10a64cc64345871ba8f26252fc4aa59dbc316029b91  crates/forgeplan-core/src/config/types.rs
50ef0cc97567b17016f2653465a1eff598d9289fa9d487fac872261b6561f9c4  crates/forgeplan-core/src/llm/local_cli.rs
```

## Реальный MCP-запуск

Версии: `forgeplan 0.38.0`, `codex-cli 0.164.0`.
SHA-256 проверенного debug-бинарника:

```text
45f9da9ac2552db305edd617dda62711e80ccd2c0368ffaa084891668ea428c0
```

В отдельном workspace CLI создал NOTE-001 и записал содержательную заметку
о критериях проверки effort через `forgeplan update --body`. Затем выполнены
stdio JSON-RPC `initialize`, `notifications/initialized`, `tools/call`
с `name: forgeplan_reason`, `id: NOTE-001` и явным `workspace`.
Конфигурация: `codex-cli`, `gpt-6.1-sol`, `effort: high`, reason timeout 600 с.

OS-наблюдение в `2026-11-01T00:05:16.520254Z` зафиксировало один stock Codex
процесс с аргументами (сокращён только временный путь output):

```text
codex exec --ephemeral --sandbox read-only --skip-git-repo-check --ignore-user-config --ignore-rules --model gpt-6.1-sol --output-last-message <temporary-output-file> -c model_reasoning_effort="high" -
```

MCP вернул `isError: false`; JSON анализа содержит 3 гипотезы и 3 дедукции.
Время полного smoke — 39.794853458 с. Адаптер напрямую возвращает результат
локального провайдера без fallback на другую модель/API. Пользовательская
конфигурация Codex отключена флагом. Серверная маршрутизация и эффективные
model/effort в ответе не раскрыты.

**Effective effort: UNCONFIRMED, не PASS.** argv доказывает передачу `high`,
но не исполнение провайдером. Поля model/provider в MCP — конфигурация,
не serving metadata. Отдельного подтверждения отсутствия серверной подмены нет.

Перед запросом native `account/rateLimits/read` сообщил в
`2026-11-01T00:00:40.131813Z`: Pro, 6% использования недельной квоты,
`ordinaryUsageAllowed: true`, credits balance 0, `hasCredits: false`.
Это свидетельство доступной подписки, не финансовый аудит списания.

## Первичные материалы и подключение

В worktree сохранён каталог `.local/codex-effort-v038-verification/`:
`forgeplan` (копия проверенного бинарника), `mcp-transcript.json`,
`mcp-stderr.log`, `codex-processes.json`, `timing.json`, `setup.json`,
`quota-before.json`, `independent-review.json`, `verification.json`.
После коммита `commit-manifest.json` связывает его с хешами проверенных файлов
и бинарника. `.local` не входит в коммит; материалы доступны на этой машине.
Лог сборки и тестов сессии: `artifact://64`; ревью: `history://Effort038Review`.

Инструкция и YAML: [LLM-PROVIDERS.md](LLM-PROVIDERS.md#reasoning-effort-в-codex).
По выбранному пользователем объёму работ подключение в GertsSystem
не выполнялось. Будущее подключение требует отдельного действия: указать
абсолютный путь проверенного бинарника в project-local MCP регистрации,
изменить только блок `llm` и перезапустить сервер. Глобальные настройки
и credentials для этого переносить не требуется.
