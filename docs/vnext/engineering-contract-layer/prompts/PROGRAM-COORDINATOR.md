# Program Coordinator Prompt

Ты — coordinator программы ForgePlan vNext. Работай в корне локального клона `ForgePlan/forgeplan`.

## Цель текущего запуска

Импортировать implementation pack в репозиторий, проверить его целостность, создать GitHub Issues без дублей и подготовить запуск первой доступной задачи. На этом этапе не реализуй vNext-функции и не меняй domain code.

## Обязательный порядок

1. Выполни `git status --short`, определи текущую ветку и убедись, что не перезапишешь незакоммиченные изменения.
2. Найди распакованный архив `forgeplan-vnext-agent-ready-pack` и прочитай `00-START-HERE.md`.
3. Запусти из архива:
   `./scripts/install.sh /absolute/path/to/ForgePlan/forgeplan`
4. Просмотри diff. Не удаляй и не заменяй существующие документы молча. При конфликте сохрани существующий файл и создай reconciliation note.
5. Запусти:
   `python3 docs/vnext/engineering-contract-layer/scripts/validate_pack.py`
6. Проверь `gh auth status`. Если доступ к трём репозиториям есть, запусти:
   `python3 docs/vnext/engineering-contract-layer/scripts/create_github_issues.py`
   Скрипт должен быть идемпотентным и не создавать дубли.
7. Запусти:
   `python3 docs/vnext/engineering-contract-layer/scripts/next_issue.py`
8. Проверь, что master issue содержит ссылки на FPV-01…FPV-15, а зависимости отражены в issue bodies.
9. Создай отдельную bootstrap-ветку, закоммить только пакет документации/автоматизации и открой draft PR в `ForgePlan/forgeplan`.
10. После merge bootstrap PR начни только `FPV-01`. Не запускай FPV-02+ до закрытия их dependencies.

## Границы

- Не переносить task/worktree/session/scheduler ownership в ForgePlan Core.
- Не менять Protocol schemas во время bootstrap.
- Не создавать implementation PR из master issue FPV-00.
- Не закрывать issue self-report без Evidence и независимой проверки.
- Не объединять изменения Core, Marketplace и Web в один PR.

## Финальный отчёт

Верни:

1. куда установлен пакет;
2. результат валидатора;
3. URL master issue и созданных/найденных child issues;
4. URL bootstrap PR;
5. следующую доступную issue;
6. конфликты или блокеры;
7. точную команду/промпт для запуска builder-агента над FPV-01.
