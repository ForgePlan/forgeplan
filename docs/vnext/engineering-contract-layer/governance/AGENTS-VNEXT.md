# ForgePlan vNext Agent Rules

## Mission

Выполнять только одну GitHub Issue за один рабочий контекст и предоставлять проверяемый результат.

## Mandatory flow

1. Прочитать issue полностью, включая dependencies, non-goals и acceptance criteria.
2. Проверить связанные open issues и активные ForgePlan artifacts.
3. Выполнить `forgeplan health`, `forgeplan context` и routing/depth check.
4. Создать или обновить нужный PRD/RFC/ADR/Spec до code changes.
5. Не расширять scope issue без отдельного комментария/approval.
6. Создать отдельную branch/worktree через текущий orchestrator.
7. Реализовать минимальный завершённый slice.
8. Добавить tests, schemas, fixtures и docs, требуемые issue.
9. Запустить все команды проверки из issue.
10. Создать Evidence с точными командами, exit codes, base/result SHA и changed paths.
11. Отдельный verifier должен проверить PR; builder не закрывает собственные RED-LINE findings.
12. PR должен ссылаться на issue и перечислять выполненные acceptance criteria.

## Hard boundaries

- Core не получает task tracker, scheduler, terminal или model runtime.
- Host/orchestrator-specific code размещается в adapters/extensions.
- Web остаётся read-only.
- Не рекламировать planned feature как shipped.
- Не добавлять статические counts в docs.
- CLI и MCP не могут иметь разные domain semantics.
- Нельзя закрыть issue только текстовым self-report.

## PR size

Одна issue — один основной PR. Если issue требует несколько repos, используются связанные PR по одному на repo, а umbrella issue остаётся открытой до их завершения.

## Stop conditions

Остановиться и пометить issue blocked, если:

- dependency не завершена;
- schema/semantic decision отсутствует;
- изменение нарушает product boundary;
- невозможно обеспечить требуемую policy в host;
- тесты требуют изменения unrelated subsystem;
- обнаружено противоречие acceptance criteria.
