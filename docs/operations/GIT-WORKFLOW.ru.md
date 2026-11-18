# Git Workflow — полные правила

Подробный справочник по git-процессам Forgeplan. CLAUDE.md содержит только краткую выжимку.

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

# 4. Push
git push origin feat/my-feature

# 5. PR (после Code→Audit→Fix→Test→Fmt→Lint→Verify)
gh pr create --base dev

# 6. Merge — merge commit, НЕ squash!
#    (squash теряет поздние коммиты)

# 7. Sync обратно
git checkout dev && git pull
```

**Ветки после merge не удаляются** — они сохраняются как история.

---

## Lifecycle релиза

```bash
# 1. Свежий dev
git checkout dev && git pull

# 2. Release branch
git checkout -b release/v0.x.0

# 3. Финальные тесты + фиксы на release branch
cargo test
# ... фиксы если нужны ...

# 4. PR в main
gh pr create --base main
# merge commit (сохраняет историю RC)

# 5. Sync main
git checkout main && git pull

# 6. Tag
git tag -a v0.x.0 -m "Release v0.x.0: описание"
git push origin v0.x.0

# 7. Sync main → dev отдельным PR (без прямого push в dev)
# Продолжение — RELEASE-PROTOCOL.md, раздел sync; исходную release-ветку сохраняем.
```

---

## PR pipeline

**PR создаётся ТОЛЬКО после всех шагов**:

```
Code → Audit → Fix → Test → Fmt → Lint → Verify → PR
```

1. **Code** — реализация фичи/фикса на feature branch
2. **Audit** — минимум 2 агента (code review + test coverage), `/audit` со skills
3. **Fix** — исправить все HIGH/CRITICAL findings из аудита
4. **Test** — `cargo test` все pass (кроме known preexisting failures)
5. **Fmt** — `cargo fmt` → `cargo fmt -- --check` = 0 diffs. Hook `pre-commit-fmt.sh` блокирует
6. **Lint** — `cargo check` = 0 warnings, 0 errors. Pre-commit hook блокирует если не компилируется
7. **Verify** — ручная проверка каждого фикса/фичи (не поверхностно)
8. **PR** — только после шагов 1-7

**Не создавать PR сразу после кода.** PR = "я проверил, протестировал, отаудитировал, отформатировал, всё работает".

---

## PR formatting

### Перед PR

- Проверить `TODO.md` — все P0 checkboxes должны быть `[x]`. Hook `pr-todo-check.sh` блокирует PR с незакрытыми P0
- Убедиться что все коммиты pushed: `git log origin/dev..HEAD`

### Содержимое PR

- **Title** = `[ARTIFACT-ID] description` — `[PRD-018] OpenSpec DAG integration`
- **Body** = Summary (bullets) + Refs (артефакты) + Test plan + Audit results

### Merge стратегия

- **`feat/* → dev`**: merge commit (НЕ squash!) — правило проекта сохраняет общую историю; поздние коммиты после merge требуют нового PR при любой стратегии
- **`release/* → main`**: merge commit (сохраняет историю RC)
- **Не удалять ветки после merge** — feature/release branches сохраняются как история

### После merge

```bash
git checkout dev && git pull     # сразу sync
# проверить что изменения на месте
```

**Не пушить в ветку после merge PR**: такие коммиты остаются в ветке, но уже закрытый PR не включает их автоматически в dev. Для интеграции нужен новый PR.

---

## Теги и релизы

- **Формат тега**: `v{major}.{minor}.{patch}` — `v0.8.0`, `v1.0.0`
- **Когда**: после merge `release/*` в main
- **Обязательно** тегировать каждый релиз — без тега релиз не считается выпущенным

### Процесс

1. `dev` → `release/v0.x.0` (RC branch)
2. Тесты + финальные фиксы на release branch
3. PR в main → merge commit
4. `git tag -a v0.x.0 -m "Release v0.x.0: описание"` на main
5. `git push origin v0.x.0`
6. Sync main → dev отдельным PR по [RELEASE-PROTOCOL.md](RELEASE-PROTOCOL.md), без прямого push в dev.

**Release notes**: следовать release workflow и проверять фактически опубликованный текст; `gh release create` — публикация, не доказательство корректности notes.
**Binary**: релизная сборка обязана включать `semantic-search`; точные команды и проверки — в release protocol. Обычное `cargo build --release` не заменяет их.

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

Действуют запреты ниже. Настроенный `.claude/hooks/forge-safety-hook.sh` проверяет лишь буквальные шаблоны `Bash`, не все эквивалентные команды и не другие tools; наличие списка — не гарантия технической блокировки:

- `git push --force` / `git push -f`
- `git reset --hard`
- `git clean -fd`
- `rm -rf /` / `rm -rf ~`
- `cargo publish` (explicit manual action)
- `DROP TABLE`

### Перед любым reinit workspace

Не удаляйте `.forgeplan` для исправления индекса. Сначала сохраните проверенный
backup каталога и, где операция поддерживается layout, export. Backup должен
включать Markdown, настройки, связи и нужные metadata, а не только ответ export.

Для производных данных используйте соответствующую layout операцию:
`reindex` обновляет legacy-проекцию, `index` — поколение Constellation,
`embed` — векторы при наличии feature и полного model cache.
`scan-import` импортирует внешние Markdown, а не ремонтирует graph-orphans.
Отказ одной команды не разрешает сброс workspace. Разрушительный reinit —
отдельная явно разрешённая операция с проверенным восстановлением, не шаг
обычной диагностики и не автоматическое следствие наличия backup.

---

## Lessons learned

См. `docs/methodology/LESSONS.ru.md` — детальные разборы инцидентов
(Sprint 13.1.5 dependent branches, squash merge loss, stale dev base и т.д.).
