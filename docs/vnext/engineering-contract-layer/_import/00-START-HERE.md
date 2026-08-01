# START HERE — ForgePlan vNext Agent-Ready Pack

Это готовый пакет для импорта в `ForgePlan/forgeplan` и последующего выполнения программы агентами через GitHub Issues.

## Что сделать человеку

```bash
unzip forgeplan-vnext-agent-ready-pack.zip
cd forgeplan-vnext-agent-ready-pack
./scripts/validate.sh
./scripts/install.sh /absolute/path/to/ForgePlan/forgeplan
```

Затем открой `PROMPT-START-AGENT.md`, скопируй его целиком и передай coordinator-агенту, запущенному в корне локального клона `ForgePlan/forgeplan`.

## Что установится

```text
docs/vnext/engineering-contract-layer/
.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md
```

Существующие файлы не удаляются. Install script останавливается при конфликте содержимого и просит выполнить осознанное объединение.

## Что создаст coordinator

- проверит импорт;
- создаст или найдёт 16 GitHub Issues без дублей;
- свяжет master epic и dependencies;
- откроет bootstrap PR с документацией;
- выберет первую доступную задачу `FPV-01`;
- подготовит builder prompt.

## Важная граница

Master `FPV-00` координирует программу. Реализация начинается с `FPV-01`, затем идёт по dependency graph из `02-EXECUTION-PLAN.md`.
