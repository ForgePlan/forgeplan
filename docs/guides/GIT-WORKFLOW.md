# Git Workflow — как работать с ветками и коммитами

> Простой гайд. Подходит для любого проекта с двумя постоянными ветками (main + dev).

---

## Две постоянные ветки

```
main   ← продакшн (релизы, теги v1.0.0, v2.0.0)
dev    ← интеграция (сюда сливается вся работа)
```

**Главное правило**: никогда не коммить напрямую в `main` или `dev`. Только через feature branch + Pull Request.

---

## Как делать задачу (пошагово)

### Шаг 1: Обновиться

```bash
git checkout dev
git pull origin dev
```

Всегда начинай с актуального dev. Не создавай ветки из устаревшего состояния.

### Шаг 2: Создать ветку

```bash
git checkout -b feat/название-задачи
```

Формат имени: `{тип}/{короткое-описание}`

| Тип | Когда | Пример |
|-----|-------|--------|
| `feat/` | Новая фича | `feat/batch-score-command` |
| `fix/` | Баг-фикс | `fix/reff-write-back` |
| `docs/` | Документация | `docs/methodology-course` |

### Шаг 3: Работать и коммитить

```bash
# Добавляй конкретные файлы, НЕ "git add ."
git add src/scoring/reff.rs src/commands/score.rs

# Коммит с понятным сообщением
git commit -m "feat(scoring): add R_eff write-back to LanceDB"
```

**Почему не `git add .`**: может случайно добавить .env, секреты, бинарники, node_modules.

### Шаг 4: Запушить

```bash
git push origin feat/название-задачи -u
```

`-u` привязывает локальную ветку к remote — дальше можно просто `git push`.

### Шаг 5: Создать Pull Request

```bash
gh pr create --base dev --title "Описание" --body "Что сделано и зачем"
```

Или через GitHub UI.

### Шаг 6: ПРОВЕРИТЬ перед merge

```bash
# Убедись что ВСЕ твои коммиты в PR
git log origin/dev..HEAD
```

Если видишь не все коммиты — запушь ещё раз.

### Шаг 7: Merge

```bash
gh pr merge --merge --delete-branch=false
```

**ВАЖНО**: используй `--merge`, НЕ `--squash`. Squash берёт снимок на момент merge и теряет коммиты, которые были добавлены позже.

### Шаг 8: СРАЗУ проверить

```bash
git checkout dev
git pull origin dev

# Проверить что твои изменения на месте
grep "твоя_функция" src/файл.rs
```

Если изменений нет — squash потерял. Создай recovery PR.

---

## Формат коммитов

```
тип(модуль): описание на английском

Тело на русском или английском — зачем и почему.

Refs: PRD-001, RFC-002
```

### Типы коммитов

| Тип | Когда | Пример |
|-----|-------|--------|
| `feat` | Новая функциональность | `feat(cli): add forgeplan tree command` |
| `fix` | Баг-фикс | `fix(scoring): NaN guard in R_eff` |
| `docs` | Документация | `docs(guide): add Chapter 8` |
| `test` | Тесты | `test(e2e): coverage backfill test` |
| `refactor` | Рефакторинг без изменения поведения | `refactor(store): extract DRY constant` |
| `chore` | Build, зависимости, CI | `chore(deps): update tokio to 1.35` |
| `progress` | Обновление прогресса артефактов | `progress: update Phase 3 tracking` |

### Правила

- **Один коммит = одна логическая вещь** — не мешай feat + docs + fix в одном коммите
- **Описание на английском** — для совместимости с инструментами
- **Тело на русском** — для контекста команды
- **Refs обязательны** — каждый коммит ссылается на артефакт (PRD, RFC, ADR)

---

## Как делать релиз

```bash
# 1. Создать release branch из dev
git checkout dev && git pull origin dev
git checkout -b release/v1.0.0

# 2. Финальные тесты и фиксы
cargo test    # или npm test, pytest, etc.

# 3. PR в main (merge commit)
gh pr create --base main --title "Release v1.0.0"
gh pr merge --merge

# 4. Поставить тег
git checkout main && git pull origin main
git tag -a v1.0.0 -m "Release v1.0.0: описание что в релизе"
git push origin v1.0.0

# 5. Синхронизировать dev
git checkout dev
git merge main
git push origin dev
```

**Без тега релиз не считается выпущенным.**

---

## 5 правил (запомни)

1. **Ветка от dev, PR в dev** — всегда через feature branch
2. **Merge commit, не squash** — squash теряет поздние коммиты
3. **Не пушить после merge** — коммиты в уже merged ветку потеряются
4. **Проверить после merge** — `git pull` и убедиться что изменения на месте
5. **Не удалять ветки** — feature branches остаются как история решений

---

## Запрещено

| Команда | Почему опасна |
|---------|---------------|
| `git push --force` | Переписывает историю, теряет чужие коммиты |
| `git reset --hard` | Безвозвратно удаляет незакоммиченные изменения |
| `git add .` или `git add -A` | Может добавить секреты, .env, бинарники |
| Коммит в main/dev | Только через feature branch + PR |
| `git rebase -i` на shared ветках | Переписывает историю для всех |
| Push в ветку после merge PR | Коммиты не попадут в target branch |

---

## Параллельная работа (worktrees)

Если нужно срочно сделать hotfix пока работаешь над фичей:

```bash
# Создать отдельную рабочую копию
git worktree add ../project-hotfix fix/urgent-bug

# Работать в ней
cd ../project-hotfix
# ... fix, commit, push, PR ...

# Вернуться и удалить
cd ../project
git worktree remove ../project-hotfix
```

---

## Шпаргалка: новая задача за 2 минуты

```bash
git checkout dev && git pull                    # обновиться
git checkout -b feat/моя-задача                 # ветка
# ... работа ...
git add файл1.rs файл2.rs                      # конкретные файлы
git commit -m "feat(модуль): что сделал"        # коммит
git push origin feat/моя-задача -u              # push
gh pr create --base dev                         # PR
# ... review ...
gh pr merge --merge --delete-branch=false       # merge (НЕ squash!)
git checkout dev && git pull                    # проверить
```

---

## FAQ

**Q: Я запушил коммит в ветку после merge PR. Что делать?**
A: Коммит потерян. Создай новую ветку, cherry-pick потерянный коммит, сделай новый PR.

**Q: Merge conflict при PR. Что делать?**
A: `git checkout dev && git pull && git checkout моя-ветка && git merge dev` — разреши конфликты локально, запушь.

**Q: Забыл создать ветку и закоммитил в dev. Что делать?**
A: `git checkout -b feat/моя-задача` (создаст ветку из текущего коммита). Потом `git checkout dev && git reset --hard origin/dev` (откатит dev к remote).

**Q: Нужно ли делать squash merge?**
A: Нет. Используй обычный merge commit. Squash теряет коммиты если вы добавляли их после создания PR.
