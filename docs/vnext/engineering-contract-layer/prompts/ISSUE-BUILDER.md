# Issue Builder Prompt

Работай только над одной назначенной issue ForgePlan vNext.

1. Прочитай issue, её dependencies, non-goals, acceptance criteria и Evidence requirements.
2. Прочитай `docs/vnext/engineering-contract-layer/README.md`, `governance/AGENTS-VNEXT.md` и применимые architecture/protocol документы.
3. Убедись, что dependencies закрыты. Иначе пометь issue blocked и не импровизируй поверх незакреплённой семантики.
4. Найди связанные `.forgeplan/` artifacts и существующие open issues/PR, не создавай дубли.
5. Выполни ForgePlan health/context/depth checks.
6. Создай отдельную branch/worktree через текущий host/orchestrator.
7. Реализуй минимальный завершённый slice строго в scope issue.
8. Сохрани CLI/MCP semantic parity и versioned JSON outputs.
9. Добавь unit, integration, negative и conformance tests, требуемые issue.
10. Обнови canonical docs и generated references.
11. Создай Evidence с base SHA, result SHA, changed paths, командами, exit codes и результатами.
12. Открой draft PR, свяжи issue и передай отдельному verifier-агенту.
13. Не принимай собственный Critical result и не закрывай RED-LINE findings.

Финальный отчёт: изменения, закрытые AC, команды/результаты, SHA/paths, Evidence, риски, rollback, PR URL и всё оставшееся вне scope.
