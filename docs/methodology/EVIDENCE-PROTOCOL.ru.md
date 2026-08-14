# Протокол управления доказательствами (Evidence-Driven)

## Обзор

Forgeplan требует, чтобы каждый артефакт (PRD, RFC, ADR, EPIC, SPEC, PROB), упомянутый в pull request, имел связанное **доказательство** (evidence) перед созданием PR. Это гарантирует, что архитектурные и функциональные решения подкреплены фактами: тестами, бенчмарками, аудитами или измерениями.

**Доказательство** представляется артефактом `EVID` (EvidencePack) с типизированной связью `informs` или `based_on` на артефакт, для которого оно служит доказательством.

## Почему доказательство перед PR важно

Без доказательства на момент PR:
- Решения слепые — R_eff = 0 (нет оценки качества)
- Отчёты здоровья (health) показывают `blind_spots` — необоснованные решения загромождают граф артефактов
- Команда не может оценить trade-offs или валидировать тезисы из описания PR
- Молчаливые сбои накапливаются (см. PROB-035, PROB-039: большинство упущенных багов восходят к «мы тестировали happy path, а не реальный сценарий»)

С доказательством:
- Каждое решение якорировано на факте
- Метрики качества (R_eff) имеют смысл — слепые пятна очевидны
- Code review может оценить доказательство вместе с реализацией
- Жизненный цикл артефакта остаётся согласованным (Shape → Validate → Code → Evidence → Activate)

## Трёхслойный стек enforcement'а доказательств

### Layer 1: Agent Skills (Wave 2 W5+W6)
Плагин маркетплейса `fpl-skills` v1.5.0+ включает улучшенные скиллы `/audit`, `/sprint`, `/build`, которые **автопубликуют EVID** после работы агента. Эти скиллы:
- Захватывают вывод тестов, findings аудитов, результаты бенчмарков
- Автоматически создают артефакт `EVID` со структурированными полями
- Автоматически связывают с затронутым PRD/RFC/ADR связью `informs`
- Эмитируют `_next_action: forgeplan activate <ID>` hint

Этот слой ловит workflows, порождённые агентами, и гарантирует, что доказательство создаётся во время работы, а не после.

### Layer 2: Pre-PR Hook (FR-014, этот документ)
Перед `gh pr create`, запускается `.claude/hooks/pre-pr-evidence-check.sh`:
1. Парсит имя ветки и последние 20 коммитов на предмет artifact ID (PRD-NNN, RFC-NNN и т.д.)
2. Для каждого найденного артефакта (кроме EVID) проверяет граф на наличие связей `informs` или `based_on`
3. **Блокирует создание PR** (exit код 2), если доказательство отсутствует
4. Предоставляет чёткие инструкции по обходу для легитимных исключений

Этот слой — жёсткий gate на границе человеческого PR. Вы не можете пройти дальше без доказательства или явного обхода.

### Layer 3: Health Verdict (post-hoc reporting)
`forgeplan health` детектирует `blind_spots` — активные артефакты без связанных доказательств. Это reported в:
- Выводе CLI: `Health: unhealthy (N blind spots found)`
- JSON: `"blind_spots": ["PRD-NNN", "RFC-MMM"]`

Этот слой не блокирует работу, но поверхностно выявляет gaps для triage и cleanup.

## Когда создавать доказательство

### Must Have (блокирует на PR)
- **Features**: PRD/RFC → реализуй код → захвати результаты тестов → создай EVID → link → PR
- **Архитектурные решения**: ADR → логика решения → findings аудита → создай EVID → link → PR
- **Problems/анализ root-cause**: PROB → investigation → audit/measurement → создай EVID → PR
- **API/data model changes**: SPEC → design review → EVID для корректности схемы → PR

### Should Have (рекомендуется, можно обойти)
- **Bug fixes**: тест что баг fixed → опциональный EVID (многие стандарты требуют только для P0)
- **Refactoring**: code review findings → опциональный EVID (если архитектурный impact значительный)
- **Documentation updates**: могут ссылаться на existing EVID из оригинальной фичи, новый EVID не нужен

### Exempt (auto-bypass, доказательство не нужно)
- **Documentation-only PRs** (ветка `docs/*`)
- **Mechanical sync PRs** (`chore/sync-main-to-dev-*`, `chore/dependabot-*`)
- **Release branch PRs** (`release/v*`)
- **Hotfix branch PRs** (`hotfix/*`)

## Механизм обхода

### Когда обойти

Легитимные случаи:
- **Dependency bump без feature change**: `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create`
- **Retroactive evidence**: Вы смёржили код, затем нужно attach EVID для audit trail (см. раздел ниже)
- **Emergency hotfix**: `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "[HOTFIX] Production outage" --body "...justification..."`

**⚠️ Важно**: обходите с намерением. Всегда документируйте в теле PR ПОЧЕМУ доказательство пропускается. Примеры:
```bash
FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "[HOTFIX] Auth token expiry bug" \
  --body "Production outage fix. Evidence: existing EVID-087 covers token refresh logic. New EVID retroactively attached in follow-up PR #NNN."
```

### Методы обхода

1. **Environment variable**:
   ```bash
   FORGEPLAN_SKIP_EVIDENCE=1 gh pr create --title "..." --body "..."
   ```

2. **Branch prefix** (auto-bypass, env var не нужна):
   - `docs/` — documentation-only
   - `chore/sync-` — sync PRs
   - `chore/dependabot-` — dependency updates
   - `release/v` — release branches
   - `hotfix/` — hotfixes

3. **Via Git alias** (опционально, для удобства):
   ```bash
   git config --global alias.pr-skip '!FORGEPLAN_SKIP_EVIDENCE=1 gh pr create'
   # Использование: git pr-skip --title "..." --body "..."
   ```

## Retroactive Evidence (как захватить уже merged работу)

Если вы смёржили код без создания EVID, можете захватить доказательство retroactively:

### Шаг 1: Создайте артефакт EVID
```bash
forgeplan new evidence "Feature X: test coverage 92%, p95 latency 180ms"
```

### Шаг 2: Заполните structured fields
Отредактируйте `.forgeplan/evidence/EVID-NNN-*.md` и добавьте:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

(Семантика полей — эта секция; участие в скоринге — §Жизненный цикл эвиденции и скоринг ниже.)

### Шаг 3: Link на артефакт
```bash
forgeplan link EVID-NNN PRD-MMM --relation informs
```

### Шаг 4: Activate
```bash
forgeplan activate EVID-NNN
```

### Шаг 5: Создайте follow-up PR для merge EVID ветки
```bash
git add .forgeplan/evidence/EVID-NNN-*.md
git commit -m "docs(evidence): retroactive EVID-NNN for PRD-MMM

Refs: PRD-MMM"
gh pr create --title "[Evidence] Add EVID-NNN for PRD-MMM" \
  --body "Retroactively captured evidence from merged feature. See EVID-NNN for test results."
```

## Жизненный цикл эвиденции и скоринг (ADR-020)

Какие пакеты участвуют в `R_eff = min(evidence_scores)`, зависит от статуса пакета:

| Статус эвиденции | Участвует в min()? | Почему |
|---|---|---|
| `draft` | **да** | свежее измерение, ждущее активации — score-гейт идёт ДО активации в стандартном flow |
| `active` | **да** | текущее показание; активный `refutes` обнуляет score |
| `stale` | **да** | не терминальный — помечен к пересмотру, но не вытеснен; истёкший `valid_until` отдельно роняет score пакета до 0.1 (decay) |
| `superseded` | **нет** | вытеснен наследником (`supersede <старый> --by <новый>`) — история остаётся в графе, но о текущей надёжности больше не говорит |
| `deprecated` | **нет** | закрыт (например, дубль, deprecated с `--reason "superseded by EVID-y"`) |

Каждое исключение логируется в factors (`Skipped EVID-x (status: superseded)`) и помечается в breakdown (`excluded from min`) — вытеснение всегда видимо, никогда не молчаливо. Если ВСЯ слинкованная эвиденция терминальна — артефакт деградирует к **no active evidence** (R_eff 0.0): для восстановления пакет-замена должен быть *слинкован* с артефактом, а не просто существовать.

Честный способ убрать устаревший слабый пакет — вытеснение: слинковать эвиденцию ре-верификации и вытеснить старый пакет. Править verdict пакета ради поднятия score — фальсификация истории; граф хранит исходный пакет ровно для того, чтобы этого никогда не требовалось.

## Технические детали

### Поведение hook'а

Hook `.claude/hooks/pre-pr-evidence-check.sh`:
- Запускается перед `gh pr create` (если wired в Claude Code hooks систему)
- Сканирует имя ветки и последние 20 коммитов на artifact ID
- Запрашивает `forgeplan graph` или `forgeplan get` для проверки наличия evidence links
- Exit коды:
  - **0** = proceed (доказательство найдено или обойдено)
  - **2** = доказательство missing (блокирует PR)
- Soft fallback: если binary `forgeplan` не на PATH, exits 0 (не блокирует)

### Детектирование Artifact ID

Hook ищет эти patterns:
- Имя ветки: `feat/PRD-077-something` → детектирует `PRD-077`
- Сообщение коммита: `feat(prd): implement auth\n\nRefs: PRD-077, FR-001..003` → детектирует `PRD-077`
- Также handles: RFC, ADR, EPIC, SPEC, PROB, EVID, NOTE

### Типы Evidence Relations

Hook проверяет:
- **`informs`**: EVID предоставляет supporting data для артефакта (common direction)
- **`based_on`**: артефакт основан на findings из EVID

Обе relations удовлетворяют requirement на доказательство.

## Интеграция с CI/CD

### Layer 1 (Agent Skills)
- **Где**: `plugins/fpl-skills/skills/{audit,sprint,build}.py` (marketplace repo)
- **Когда**: После завершения таска агента
- **Action**: Auto-create EVID + link + hint для activation

### Layer 2 (Pre-PR Hook)
- **Где**: `.claude/hooks/pre-pr-evidence-check.sh` (этот repo)
- **Когда**: Перед `gh pr create`
- **Trigger**: Claude Code hooks система или Git hook интеграция (если available)
- **Action**: Блокирует PR если доказательство missing; предоставляет инструкции по обходу

### Layer 3 (Health Reporting)
- **Где**: `forgeplan health` команда, CI job (если wired)
- **Когда**: Post-merge или on-demand во время разработки
- **Action**: Report blind spots для triage

## Health Report Integration

Когда запускаете `forgeplan health`, отчёт включает:

```
Artifacts:
  ...
  
Blind Spots (artifacts without evidence):
  - PRD-077 (3 days old, Standard depth)
  - RFC-009 (1 week old, Deep depth)
```

Это помогает команде выявить решения, которые нуждаются в захвате доказательства, либо retroactively, либо в будущей работе.

## FAQ

**В: Что если я в спешке и просто нужно merge?**
О: Используйте `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create`. Но документируйте в теле PR почему вы пропускаете. Retroactively attach EVID в follow-up PR если причина обхода это оправдывает.

**В: Нужно ли доказательство для documentation-only PRs?**
О: Нет — ветки `docs/*` auto-bypass. Branch protection предполагает, что изменения docs не нуждаются в архитектурном доказательстве.

**В: Что если binary `forgeplan` не установлен?**
О: Hook soft-fails (exits 0) вместо блокировки. Вы всё ещё можете создавать PRs, но теряете gate. Это intentional для fresh clones без построенных binaries.

**В: Могу ли я wire hook в Git вместо Claude Code?**
О: Да — скопируйте `.claude/hooks/pre-pr-evidence-check.sh` в `.git/hooks/pre-push` (или custom hook) и переименуйте на `pre-push`. Убедитесь что ваш hook invokes его перед `git push`.

**В: Что если мой артефакт действительно ad-hoc и не нужно доказательство?**
О: Создайте PROB или decision note объясняя почему, затем решите: (1) reclassify как `NOTE` (ephemeral, auto-expires), (2) retroactively create minimal EVID, или (3) использовать bypass + документировать.

## Reference

- **ADR-003**: Markdown как source of truth для артефактов
- **PRD-077**: Wave 2 evidence autopublish и enforcement
- **PROB-035, PROB-039**: Silent failures из happy-path-only testing
- **Hooks**: `.claude/hooks/pre-pr-evidence-check.sh`
- **Health**: `forgeplan health` команда
- **Schema**: §Structured Fields выше (структура EvidencePack; отдельного `docs/schemas/EVIDENCE.md` пока не существует)

