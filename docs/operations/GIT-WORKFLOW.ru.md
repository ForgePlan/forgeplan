# Git Workflow — полные правила

Подробный справочник по git-процессам Forgeplan. CLAUDE.md содержит только краткую выжимку.

Это процесс разработки самого репозитория, не гарантия поведения установленного
CLI. Пользовательские команды ниже сверены с релизом **0.38.0**; `Cargo.toml`
этой рабочей ветки пока содержит **0.37.0**. Точные шаги выпуска — в
[RELEASE-PROTOCOL.ru.md](RELEASE-PROTOCOL.ru.md). Имена `v0.8.0`, `v0.9.0`
в схемах — примеры, не указание последнего релиза.

## Оглавление
- [Формат коммита](#формат-коммита)
- [Branching strategy](#branching-strategy)
- [Lifecycle ветки](#lifecycle-ветки)
- [Lifecycle релиза](#lifecycle-релиза)
- [PR pipeline](#pr-pipeline)
- [PR formatting](#pr-formatting)
- [Теги и релизы](#теги-и-релизы)
- [Worktrees](#worktrees)
- [Запрещённые действия](#запрещённые-действия)
- [Lessons learned](#lessons-learned)

---

## Формат коммита

Conventional Commits + Forgeplan refs:

```
<type>(<scope>): <description>

[body — что и почему, на русском]

Refs: RFC-001, FR-001..004
```

### Types

| Type | Когда | Пример |
|------|-------|--------|
| `feat` | Новая функциональность (FR-*) | `feat(cli): implement forgeplan init` |
| `docs` | Артефакты методологии (RFC, PRD, ADR) | `docs(rfc): add RFC-001 CLI architecture` |
| `fix` | Баг-фикс | `fix(frontmatter): handle missing closing ---` |
| `refactor` | Рефакторинг без изменения поведения | `refactor(store): extract slugify` |
| `test` | Тесты | `test(workspace): add init roundtrip tests` |
| `chore` | Build, deps, CI | `chore(deps): add tempfile dev-dependency` |
| `progress` | Обновление прогресса артефактов | `progress: update Phase 3A tracking` |

### Scope

- **Код**: `cli`, `core`, `store`, `template`, `scoring`, `workspace`, `config`
- **Артефакты**: `rfc`, `prd`, `adr`, `epic`

### Правила коммитов

- **Refs обязательны** — каждый коммит ссылается на артефакт (RFC, FR, ADR)
- **Один коммит = одна логическая единица** — не мешать feat + docs + refactor
- **Description на английском** (совместимость), body на русском (контекст)
- **Не коммить напрямую в `main` или `dev`** — всегда через feature branch + PR

---

## Branching strategy

Dev-based flow:

```
main                              ← production (tagged: v0.8.0, v0.9.0, ...)
  │
dev                               ← integration branch
  ├── feat/prd-018-openspec-dag   ← feature (from dev)
  ├── fix/search-ranking          ← bugfix (from dev)
  └── docs/rfc-002-lancedb        ← docs (from dev)
  │
release/v0.9.0                    ← RC (from dev → main)
```

| Ветка | Создаётся из | Мерджится в | Стратегия |
|-------|-------------|-------------|-----------|
| `feat/*`, `fix/*`, `docs/*` | **dev** | **dev** | Merge commit via PR |
| `release/v0.x.0` | **dev** | **main** + **dev** | Merge commit (сохраняет историю) |
| `hotfix/*` | **main** | **main** + **dev** | Cherry-pick |

**Формат имени**: `{type}/{slug}` — например, `feat/prd-018-openspec-dag`.

### Перед созданием ветки

```bash
git checkout dev && git pull origin dev   # всегда pull!
git checkout -b feat/my-feature
```

**Не создавать ветки из stale dev.** Всегда `git pull` первым.

---

## Lifecycle ветки

```bash
# 1. Свежий dev
git checkout dev && git pull origin dev

# 2. Feature branch
git checkout -b feat/my-feature

# 3. Работа + коммиты (с Refs в body)

# 4. Push — только после явного разрешения пользователя
git push origin feat/my-feature

# 5. PR (после Code→Audit→Fix→Test→Fmt→Lint→Verify)
gh pr create --base dev

# 6. Merge по политике репозитория; после merge не дописывать старую ветку
#    Сохраняйте историю integration/release; новую работу ведите в новой ветке

# 7. Sync обратно
git checkout dev && git pull
```

**Ветки после merge не удаляются** — они сохраняются как история.

---

## Lifecycle релиза

Перед выпуском выполните полный [release protocol](RELEASE-PROTOCOL.ru.md),
включая триаж зависимостей, проверку артефактов/evidence, выпускные проверки
и согласование push/PR. Схема веток:

```bash
# 1. Свежий dev
git checkout dev && git pull --ff-only

# 2. Release branch; далее version bump и проверки по release protocol
git checkout -b release/vX.Y.Z

# 3. После разрешённого push — PR в main, review и merge
gh pr create --base main

# 4. Обновить main, затем создать и опубликовать тег с разрешения пользователя
git checkout main && git pull --ff-only
git tag -a vX.Y.Z -m "Release vX.Y.Z: описание"
git push origin vX.Y.Z

# 5. Обязательная синхронизация main → dev ЧЕРЕЗ PR, не прямым push в dev
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z
# После разрешения на push:
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --title "chore: sync main into dev after vX.Y.Z"
```

PR синхронизации нужно довести до merge. Прямой `git push origin dev`
обходит установленный процесс и блокируется защитой ветки.

---

## PR pipeline

**PR создаётся ТОЛЬКО после всех шагов**:

```
Code → Audit → Fix → Test → Fmt → Lint → Verify → PR
```

1. **Code** — реализация фичи/фикса на feature branch
2. **Audit** — независимые code review и проверка полноты тестов; `/audit` из публичного `fpl-skills` 1.75.1 требует минимум 4 специализированных ревьюера. Сам CLI не запускает этот skill
3. **Fix** — исправить все HIGH/CRITICAL findings из аудита
4. **Test** — `cargo test` все pass (кроме known preexisting failures)
5. **Fmt** — `cargo fmt` → `cargo fmt -- --check` = 0 diffs
6. **Lint** — проверки `cargo check`/`clippy` по [CI quality gates](QUALITY-GATES.ru.md)
7. **Verify** — unit tests и реальный E2E каждой затронутой поверхности; наличие хуков зависит от конфигурации, не заменяет эти шаги
8. **PR** — только после шагов 1-7

**Не создавать PR сразу после кода.** PR = "я проверил, протестировал, отаудитировал, отформатировал, всё работает".

---

## PR formatting

### Перед PR

- Проверить `TODO.md` — все P0 закрыты либо явно объяснены по правилам проекта; не предполагать, что локальный хук установлен
- Убедиться, что push разрешён и все коммиты отправлены: `git log origin/dev..HEAD`

### Содержимое PR

- **Title** = `[ARTIFACT-ID] description` — `[PRD-018] OpenSpec DAG integration`
- **Body** = Summary (bullets) + Refs (артефакты) + Test plan + Audit results

### Merge стратегия

- **`feat/* → dev`**: merge commit (НЕ squash!) — squash теряет поздние коммиты
- **`release/* → main`**: merge commit (сохраняет историю RC)
- **Не удалять ветки после merge** — feature/release branches сохраняются как история

### После merge

```bash
git checkout dev && git pull     # сразу sync
# проверить что изменения на месте
```

**Никогда не пушить в ветку после merge PR** — коммиты будут потеряны.

---

## Теги и релизы

- **Формат тега**: `v{major}.{minor}.{patch}` — `v0.8.0`, `v1.0.0`
- **Когда**: после merge `release/*` в main
- **Обязательно** тегировать каждый релиз — без тега релиз не считается выпущенным

### Процесс

1. `dev` → `release/vX.Y.Z` (RC branch).
2. Триаж Dependabot, выпускные проверки, артефакты/evidence и version bump по [release protocol](RELEASE-PROTOCOL.ru.md).
3. Разрешённый push, PR в main, review и merge с сохранением истории RC.
4. `git tag -a vX.Y.Z -m "Release vX.Y.Z: описание"` на main.
5. `git push origin vX.Y.Z` только после разрешения.
6. Обязательный sync-PR из `chore/sync-main-to-dev-after-vX.Y.Z` в `dev`.

**Release notes**: подготовить через `forgeplan release-notes --since v<PREV_TAG> --output markdown` и отредактировать по фактическому релизу.
**Binary**: обычный `cargo build --release` не включает `semantic-search`.
Опубликованные бинарные сборки 0.38.0 включают эту feature через cargo-dist;
для локальной сборки с поиском явно добавьте `--features semantic-search`.

---

## Worktrees

Параллельная работа (hotfix во время фичи, параллельные агенты):

```bash
# Создать worktree
git worktree add ../forgeplan-fix fix/frontmatter-parser

# Удалить после merge
git worktree remove ../forgeplan-fix
```

**Правило**: worktree = временный, удалять после merge.

---

## Запрещённые действия

Опасные операции запрещены политикой проекта. Дополнительный safety-hook
из `forgeplan-workflow` действует только если плагин установлен и хост
исполняет его hooks; установка CLI сама не создаёт такую защиту:

- `git push --force` / `git push -f`
- `git reset --hard`
- `git clean -fd`
- `rm -rf /` / `rm -rf ~`
- `cargo publish` без отдельного разрешённого ручного выпуска
- `DROP TABLE`

### Восстановление индекса и повторная инициализация

Сначала выберите операцию по задаче:

- **После clone/merge не видны отслеживаемые артефакты** —
  `forgeplan reindex` пересобирает LanceDB из markdown.
- **Синхронизировать delta после git merge/pull** — `forgeplan git-sync`;
  после checkout можно явно передать `--since <previous-ref>`.
  `forgeplan setup` ставит соответствующие git-хуки, если не указан
  `--skip-hooks`. Эмбеддинги обновляются отдельно через `forgeplan embed`.
- **Найти внешние документы для импорта** — `scan-import`, не средство
  синхронизации существующих артефактов.
- **Пересоздать конфигурацию/недостающие каталоги и индексы** —
  `forgeplan init --force -y`. В 0.38.0 эта операция не перезаписывает
  существующие тела `.md`, но переписывает `config.yaml` и по умолчанию
  создаёт backup. Поэтому не используйте её вместо `reindex`.

Перед намеренной переинициализацией сохраните экспорт и копию:

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
forgeplan init --force -y
```

Не удаляйте `.forgeplan/` для починки производного индекса.
`--no-backup` отключает штатную защитную копию; не используйте его
как обычный способ восстановления.

---

## Lessons learned

См. `docs/methodology/LESSONS.ru.md` — детальные разборы инцидентов
(Sprint 13.1.5 dependent branches, squash merge loss, stale dev base и т.д.).
