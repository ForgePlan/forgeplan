# Import Map

| Источник в архиве | Назначение в репозитории |
|---|---|
| `payload/docs/vnext/engineering-contract-layer/` | `docs/vnext/engineering-contract-layer/` |
| `payload/.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md` | `.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md` |
| `PROMPT-START-AGENT.md` | Не копируется; используется для первого запуска coordinator |
| `scripts/install.sh` | Безопасный импорт payload |

## Репозитории, затрагиваемые backlog

- `ForgePlan/forgeplan`: FPV-00…FPV-10 и FPV-15.
- `ForgePlan/marketplace`: FPV-11…FPV-13.
- `ForgePlan/forgeplan-web`: FPV-14.

Каждая issue содержит собственный repository target; агент не должен переносить cross-repo работу в `forgeplan` ради удобства.
