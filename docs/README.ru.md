[English](README.md) · [Русский](README.ru.md)

# Указатель документации

Документация для участников разработки Forgeplan. Пользовательские инструкции
по задачам и справочники CLI/MCP находятся [на сайте](https://forgeplan.dev/ru/docs/).

- [Пользовательский гайд Constellation](https://forgeplan.dev/ru/docs/guides/constellation/)
- [Справочник интерфейсов Constellation](https://forgeplan.dev/ru/docs/reference/constellation/)
- [Переносимый runbook пилота и отката](operations/CONSTELLATION-PILOT.ru.md)

Constellation описан для scoped-сборки `0.39.0+constellation.s1.1`,
не для опубликованной `0.38.0`. Это не полная приёмка S1-G/S2.

> **Локальные заметки** (исследования, планирование, сессии, исходные материалы) находятся в `.local/` (gitignored) — не являются частью этого дерева.

## Структура

```
docs/
├── README.md          ← этот файл — навигационный указатель
├── ROADMAP.md         ← пробелы и приоритеты развития
├── methodology/       ← как работает методология Forgeplan (для людей)
├── architecture/      ← архитектура системы (C4, Constellation)
├── operations/        ← хуки агентов, enforcement, защита репозитория (devops)
└── schemas/           ← конвенции артефактов и реализованные правила валидации
```

**Артефакты** (PRD, RFC, ADR, Epic, Spec, Evidence, Problem, Note) хранятся в рабочем пространстве Forgeplan в `.forgeplan/` — см. раздел [Артефакты](#артефакты) ниже.

## Методология — начните здесь

Полный справочник по методологии. Каноничный источник для людей, изучающих Forgeplan.

| Документ | Назначение |
|---|---|
| [FORGEPLAN-GUIDE.md](methodology/FORGEPLAN-GUIDE.md) | **Начните здесь** — полный гайд: методология + CLI + evidence + lifecycle |
| [HOW-TO-USE.md](methodology/HOW-TO-USE.md) | 10 правил методологии с практическими примерами |
| [ARTIFACT-MODEL.md](methodology/ARTIFACT-MODEL.md) | Иерархия артефактов: Epic → PRD → Spec → RFC → ADR + lifecycle |
| [PRD-RFC-ADR-FLOW.md](methodology/PRD-RFC-ADR-FLOW.md) | Дерево решений: какой тип артефакта создать |
| [DEPTH-CALIBRATION.md](methodology/DEPTH-CALIBRATION.md) | Tactical → Standard → Deep → Critical с авто-эскалацией |
| [QUALITY-GATES.md](methodology/QUALITY-GATES.md) | Verification Gate + Adversarial Review + R_eff scoring |
| [UNIFIED-WORKFLOW.md](methodology/UNIFIED-WORKFLOW.md) | Интеграция Forgeplan × Orchestra × Hindsight |
| [USAGE-BY-ROLE.md](methodology/USAGE-BY-ROLE.md) | Как использовать Forgeplan в зависимости от роли |
| [METHODOLOGY-COURSE.md](methodology/METHODOLOGY-COURSE.md) | Полный курс обучения (формат курса) |
| [GLOSSARY.md](methodology/GLOSSARY.md) | Основные термины и справочная таблица lifecycle |
| [LESSONS.ru.md](methodology/LESSONS.ru.md) | Lessons learned — dependent sprint verification, audit incidents, улучшения процесса |
| [agent-protocol.md](methodology/agent-protocol.md) | **Hint contract (PRD-071)** — 5 маркеров (Next/Or/Wait/Done/Fix), good/bad примеры, agent reading protocol |
| [release-workflow.md](methodology/release-workflow.md) | End-to-end рецепт релиза — dependabot triage gate, version bump, стратегия release/* PR, post-release sync (CLAUDE.md red lines #9 + #10), hotfix flow, антипаттерны |

## Операции

Настройка, хуки и защита репозитория.

| Документ | Назначение |
|---|---|
| [AGENT-ENFORCEMENT.md](operations/AGENT-ENFORCEMENT.md) | Правила и ограничения для AI-агентов, работающих в проекте |
| [AGENT-HOOKS.md](operations/AGENT-HOOKS.md) | Хуки PreToolUse / PostToolUse (безопасность, форматирование, тесты) |
| [LLM-PROVIDERS.md](operations/LLM-PROVIDERS.md) | Настройка LLM: HTTP/Ollama и явные подписочные CLI-профили (`claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`, `cursor-agent`), режимы безопасности, восстановление входа и ограничения personal/local-only |
| [MULTI-AGENT.md](operations/MULTI-AGENT.md) | **v0.24.0+ multi-agent dispatch** — MCP-инструменты `forgeplan_dispatch/claim/release/claims`, file-overlap detection, skill routing |
| [REPO-PROTECTION-GUIDE.md](operations/REPO-PROTECTION-GUIDE.md) | Защита веток, правила PR, предотвращение деструктивных действий |
| [GIT-WORKFLOW.ru.md](operations/GIT-WORKFLOW.ru.md) | Полные Git-правила — lifecycle веток, PR pipeline, процесс релиза, worktrees |
| [RELEASE-PROTOCOL.ru.md](operations/RELEASE-PROTOCOL.ru.md) ([en](operations/RELEASE-PROTOCOL.md)) | **Канонический процесс релиза** — 10 шагов любого среза `release/vX.Y.Z`, pre-conditions и обязательный post-merge sync-PR (RED LINE #9) |
| `dependabot-triage-YYYY-MM-DD.md` | Триаж алертов зависимостей на релиз (RED LINE #10) — каждый открытый алерт помечен addressed / scheduled / accepted-with-justification. По файлу на релизное окно |
| [SOURCE-PORTING.ru.md](operations/SOURCE-PORTING.ru.md) | Reference Code map — что портировано из `sources/{quint-code,git-adr,BMAD,OpenSpec,ccpm}` в наши crates |
| [PLAYBOOK-AUTHORING.ru.md](operations/PLAYBOOK-AUTHORING.ru.md) | **v0.26.0+ авторинг playbook'ов** — декларативные YAML-workflow, 5 типов делегации, fallback hints, DAG ordering. **v0.27.0+ Subprocess lifecycle** секция (real dispatchers, kill_on_drop, timeout policy, security model) per ADR-010/PRD-072 |
| [INGEST-MAPPINGS.ru.md](operations/INGEST-MAPPINGS.ru.md) | **v0.26.0+ авторинг ingest mapping'ов** — перевод output плагинов в forge-артефакты с invariant'ом `## Sources` (PRD-066/SPEC-004) |
| [QUALITY-GATES.ru.md](operations/QUALITY-GATES.ru.md) | **v0.28.0+ CI quality gates** — все CI-гейты (fmt, clippy, test, health, validate, drift detector) с командами для локального запуска и руководствами по исправлению ошибок. Примечание: `docs/methodology/QUALITY-GATES.md` описывает методологические гейты (R_eff, Verification Gate). |
| [CONSTELLATION-PILOT.ru.md](operations/CONSTELLATION-PILOT.ru.md) | Переносимый sandbox-пилот: версия, backups, embedding, один writer и откат |
| [DOCUMENTATION-AUDIT-2026-10-23.ru.md](operations/DOCUMENTATION-AUDIT-2026-10-23.ru.md) | Измеренный аудит документации: источники, покрытие Constellation, исправленный дрейф, проверки и оставшиеся пробелы |

## Архитектура

| Документ | Назначение |
|---|---|
| [constellation.md](architecture/constellation.md) | Constellation: поколения, pinned reads, реестр федерации, семантический индекс секций, MCP read-профиль и границы |
| [c4-context.md](architecture/c4-context.md) | Контекст системы в модели C4 |


## Схемы

Конвенции авторинга; реализованные правила валидации отмечены отдельно.
Рекомендации не означают автоматическую семантическую проверку.

| Документ | Назначение |
|---|---|
| [PRD-SCHEMA.md](schemas/PRD-SCHEMA.md) | PRD: обязательные секции, калибровка глубины, правила валидации |
| [EPIC-SCHEMA.md](schemas/EPIC-SCHEMA.md) | Epic: агрегированный прогресс, правила дочерних элементов |
| [SPEC-SCHEMA.md](schemas/SPEC-SCHEMA.md) | Spec: API-контракты, модели данных, версионирование |

## Артефакты

**Расположение:** `.forgeplan/` в корне репозитория.

**Модель хранения (согласно [ADR-003](../.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**
- **Markdown-файлы** в `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` = **источник истины** (отслеживаются git)
- **LanceDB** в `.forgeplan/lance/` = производный индексный слой (git-ignored, восстанавливаемый)
- **Constellation** публикует производные поколения через `.forgeplan/lance/CURRENT.json`; эта раскладка не взаимозаменяема с legacy-индексом.
- **Конфигурация** `.forgeplan/config.yaml` = настройки workspace, включая LLM и embedding; это не индекс и не тело артефакта. В этом репозитории файл отслеживается git и содержит ссылки на переменные окружения, а не встроенные секреты. Другие проекты могут хранить конфигурацию локально.

**Директории:**

| Директория | Содержимое |
|---|---|
| [`.forgeplan/epics/`](../.forgeplan/epics/) | Epic — стратегические группировки |
| [`.forgeplan/prds/`](../.forgeplan/prds/) | Product Requirements Documents |
| [`.forgeplan/rfcs/`](../.forgeplan/rfcs/) | RFC — архитектурные предложения с фазами реализации |
| [`.forgeplan/adrs/`](../.forgeplan/adrs/) | Architecture Decision Records |
| [`.forgeplan/specs/`](../.forgeplan/specs/) | Формальные спецификации (API-контракты, модели данных) |
| [`.forgeplan/evidence/`](../.forgeplan/evidence/) | EvidencePack — тесты, бенчмарки, измерения |
| [`.forgeplan/problems/`](../.forgeplan/problems/) | ProblemCard — формулировка проблем с индикаторами anti-Goodhart |
| `.forgeplan/solutions/` | SolutionPortfolio — 2-3+ варианта с оценкой по слабейшему звену |
| [`.forgeplan/notes/`](../.forgeplan/notes/) | Note — небольшие решения; сам тип не гарантирует автоматическое истечение |
| `.forgeplan/refresh/` | RefreshReport — переоценка устаревших артефактов |
| [`.forgeplan/memory/`](../.forgeplan/memory/) | Память решений |

`solutions/` и `refresh/` показаны как пути, а не ссылки на имеющиеся данные:
в этом checkout таких каталогов нет. Создавайте артефакты поддерживаемым
интерфейсом; не добавляйте пустые каталоги ради навигации.

**Управление артефактами:** используйте поддерживаемые CLI/MCP-операции вместо
ручной правки YAML frontmatter. CLI `validate` работает на legacy и migrated
Constellation layout; full MCP `forgeplan_validate` также успешно проверен.
Границы проверки см. в [оговорках пилота](operations/CONSTELLATION-PILOT.ru.md):
регистрация 76 full tools не означает, что проверено поведение каждого.

```bash
forgeplan new prd "Title"        # создать новый артефакт
forgeplan list -t adr            # список всех ADR
forgeplan get ADR-003            # прочитать один
forgeplan validate PRD-024       # проверить схему артефакта
forgeplan score PRD-024          # вычислить R_eff
forgeplan index                  # построить/сверить индекс Constellation
forgeplan reindex                # legacy rebuild; на migrated layout делегирует index
```

**Новый каталог и существующий клон:**

- В действительно новом каталоге вне существующего workspace `forgeplan init -y`
  создаёт workspace. Обнаружение идёт вверх по дереву; проверьте показанный путь
  до любых записей.
- Клон с tracked `.forgeplan/` уже обнаруживается: `init -y` может завершиться
  раньше, не создав локальные config и индекс. Если `.forgeplan/config.yaml`
  уже есть, используйте `forgeplan index` для Constellation или `forgeplan reindex`
  для legacy-раскладки, затем проверьте `forgeplan list` и ожидаемые артефакты.
- Если локального config нет, сохраните весь `.forgeplan/` и MCP config, затем
  осознанно создайте config через `forgeplan init --force -y`. Тела артефактов
  сохраняются, но активным становится default config; custom settings, включая
  `embedding`, остаются только в backup до ручного переноса. Затем используйте
  `index` для Constellation или `reindex` для legacy-раскладки.
  Это не обычная команда восстановления индекса.

## См. также

- [`CLAUDE.md`](../CLAUDE.md) — инструкции проекта для Claude Code
- [`AGENTS.md`](../AGENTS.md) — стандартные инструкции для других AI-агентов (Aider, Cursor и др.)
- [`README.md`](../README.md) — README проекта для людей
- [`templates/`](../templates/) — markdown-шаблоны для каждого типа артефакта
- `.local/` (gitignored) — локальные исследования, планирование, сессии, исходные материалы

## Соглашения

- **Пути к исходникам в тексте указаны относительно корня репозитория, если не оговорено другое.** Markdown-ссылки разрешаются относительно содержащего их документа.
- **Файлы артефактов в `.forgeplan/` управляются поддерживаемыми CLI/MCP-операциями.** Для восстановления индекса нужен `index` для Constellation или `reindex` для legacy, не `scan-import`.
- **Методология задаёт конвенции авторинга.** Схемы отделяют рекомендации от реализованной валидации и дополнительных activation gates.
- **По конвенции активные решения сохраняют как историю** — заменяют через `forgeplan supersede ID --by NEW-ID`, а не переписывают принятое решение. Активация не защищает файл от записи и не означает общего runtime-запрета на обновление body.
