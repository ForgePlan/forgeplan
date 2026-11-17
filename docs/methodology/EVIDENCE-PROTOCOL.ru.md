# Протокол управления доказательствами (Evidence-Driven)

## Обзор

Протокол объединяет **core Forgeplan 0.38.0** (скоринг и гейты активации) с опциональными оркестрацией и хуками **публичного `fpl-skills` 1.75.1**. Наличие CLI само по себе не перехватывает каждый pull request и не устанавливает этот PR-хук.

**Доказательство** — артефакт `EVID` (EvidencePack). Предпочтительная связь: `forgeplan link EVID-NNN <TARGET-ID> --relation informs`; публичный PR-хук распознаёт входящие рёбра от EVID с `informs` или `based_on`. Он проверяет наличие связи, а не качество измерения.

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
Публичный `fpl-skills` содержит Markdown-инструкции `skills/{audit,sprint,build}/SKILL.md`, которые выполняет установленная агентская среда:
- `/audit` создаёт draft EVID с результатами и связывает известные родительские артефакты; у разового аудита может не быть родителя.
- `/sprint` создаёт draft-доказательства при закрытии волны и запрашивает активацию собранных EVID при закрытии спринта.
- `/build` создаёт один EVID для завершённой сборки и предлагает интерактивную активацию; неинтерактивный запуск оставляет draft.

Это требования оркестрации, не фоновые службы core. Они зависят от выполнения скилла агентом и доступности MCP либо CLI. Скиллы должны сообщать об ошибке публикации, а не создавать впечатление успеха. Не предполагаются автоматическая активация или фиксированная строка `_next_action`.

### Layer 2: Pre-PR Hook (FR-014, этот документ)
После копирования и подключения опубликованного `templates/hooks/pre-pr-evidence-check.sh` к Claude Code **PreToolUse** хук разбирает Bash-payload, в котором первая команда после переменных окружения — `gh pr create`:
1. Собирает числовые ID из имени ветки и её собственных коммитов. Выбирает ближайшую доступную базу с непустым диапазоном; если базы нет, берёт последние 20 коммитов.
2. Пропускает EVID и NOTE. Об отсутствующем ID предупреждает, но не блокирует.
3. Ищет входящие рёбра EVID `informs` / `based_on`; более слабый резервный путь проверяет текст секции Related/Links/Relations.
4. Возвращает 2 для существующего артефакта без доказательства; документированные обходы возвращают 0.

Это локальный обходимый хук агента, не защита ветки GitHub. Он не проверяет каждое тело PR, verdict/CL/срок действия и не останавливает PR вне этой агентской среды. Его регулярное выражение числовых ID не распознаёт ссылки только по slug.

### Layer 3: Health Verdict (post-hoc reporting)
`forgeplan health` показывает активные артефакты без доказательств как blind spots и выводит общий вердикт. Это диагностика, а не гейт создания PR.

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
Подготовьте **полное тело** в `evidence-body.md` (без YAML frontmatter), включая реальные измерение, результат, интерпретацию и отдельные строки полей ниже. Сохраните через `forgeplan update EVID-NNN --body @evidence-body.md` или `forgeplan_update`; не редактируйте управляемую проекцию напрямую:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

(Семантика полей — эта секция; участие в скоринге — §Жизненный цикл эвиденции и скоринг ниже.)

Используйте наблюдаемые результаты, не иллюстративные числа из заголовка. Парсер берёт первое отдельное вхождение `key: value`, пропуская строки Markdown-таблиц и HTML-комментарии; значения пишите без кавычек и комментариев в конце строки.

| Поле | Значения runtime / эффект |
|------|---------------------------|
| `verdict` | `supports` (1.0), `weakens` (0.5), `refutes` (0.0) до штрафов |
| `congruence_level` | Целое 0–3; штрафы CL0=0.9, CL1=0.4, CL2=0.1, CL3=0 |
| `source_tier` | Опциональные `t1`/`t2`/`t3` дают CL3/CL2/CL1; вместе с явным CL выбирается меньшее значение |
| `evidence_type` | `test`, `measurement`, `benchmark`, `audit`; поправки рекурсивного score: 0, 0, 0.1, 0.2 |

Отсутствующий/нераспознанный verdict либо отсутствие пригодного источника CL принудительно дают CL0. Неизвестный `evidence_type` (в том числе `code_review` из маркетплейса) сейчас превращается в `measurement`; это не пятый тип core. Выбирайте тип по реальному доказательству. Типичное неоформленное подтверждающее измерение получает 0.1, не 1.0; refutes и другие штрафы могут снизить число ещё.

`link --auto-activate-complete` включается явно. Его проверка полноты требует корректные отдельные verdict + явный CL, одного `source_tier` недостаточно; качество результата она не проверяет. Прямая активация EVID не гарантирует полноту полей: evidence не входит в пять типов с полной lifecycle-валидацией. Проверяйте поля и score даже после успешной активации.

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

## Git-provenance для code-claiming эвиденции (PRD-082 / #360)

EvidencePack, утверждающий, что код изменился, может объявить три отдельных поля.
Используйте **все три или ни одного**; частичный набор даёт `Incomplete`, после чего
режим гейта определяет предупреждение или блокировку:

```markdown
base_sha: 92154e19
result_sha: 808db24
changed_paths: src/a.rs, tests/b.rs
```

SHA и пути выше иллюстративны. Замените их коммитами и путями от корня репозитория для реального изменения. YAML-синтаксис `# comment` в конце строки парсер полей тела не поддерживает.

`forgeplan activate` (CLI и `forgeplan_activate` через MCP) перепроверяет claim против
реальной git-дельты — `git diff --merge-base --name-only --no-renames -z base result` —
вместо доверия самоотчёту исполнителя. Проверяем артефакт, а не утверждение.

| Вердикт | Что значит |
|---|---|
| `NotClaimed` | provenance-полей нет — все паки, написанные до этой фичи; никогда не отказ |
| `Verified` | заявленные пути присутствуют в реальной дельте |
| `EmptyDelta` | две SHA дают **пустую** дельту — зелёные тесты поверх «ничего не изменилось» это NULL-результат, а не пройденная проверка |
| `PathMismatch` | заявленный путь отсутствует в дельте |
| `Incomplete` | присутствует только часть из трёх полей |

Режим гейта задаётся в `.forgeplan/config.yaml`:

```yaml
integrity:
  evidence_provenance_gate: warn   # block | warn | off  (default: warn)
```

- **`block`** — в активации отказано; артефакт остаётся `draft`.
- **`warn`** *(по умолчанию)* — активация проходит, расхождение показывается
  (CLI — в stderr, MCP — внутри success-payload).
- **`off`** — гейт пропускается целиком.

`forgeplan activate --force` обходит гейт — тот же escape hatch, что и у методологических
гейтов. Ошибка git (нерезолвимая SHA — выдуманная либо реальная, но отсутствующая в
shallow-клоне) всегда только **предупреждает**, никогда не блокирует: локально их не
различить без сетевого fetch (ADR-019). Гейт устанавливает, что заявленное изменение
*существует* — он не запускает тесты и ничего не говорит об их качестве.

## Жизненный цикл эвиденции и скоринг (ADR-020)

Какие пакеты участвуют в `R_eff = min(evidence_scores)`, зависит от статуса пакета:

| Статус эвиденции | Участвует в min()? | Почему |
|---|---|---|
| `draft` | **да** | активация не является условием скоринга |
| `active` | **да** | текущее показание; неистёкший `refutes` обнуляет score |
| `stale` | **да** | не терминальный; истёкший `valid_until` отдельно задаёт score 0.1 (в 0.38.0 даже для ранее опровергающего пакета) |
| `superseded` | **нет** | вытеснен наследником (`supersede <старый> --by <новый>`) — история остаётся в графе, но о текущей надёжности больше не говорит |
| `deprecated` | **нет** | закрыт (например, дубль, deprecated с `--reason "superseded by EVID-y"`) |

Каждое исключение логируется в factors (`Skipped EVID-x (status: superseded)`) и помечается в breakdown (`excluded from min`) — вытеснение всегда видимо, никогда не молчаливо. Если ВСЯ слинкованная эвиденция терминальна — артефакт деградирует к **no active evidence** (R_eff 0.0): для восстановления пакет-замена должен быть *слинкован* с артефактом, а не просто существовать.

При замене устаревшего доказательства сохраните старый результат: свяжите пакет повторной проверки с решением и проверьте сохранённый статус старого пакета. `supersede` принимает старый active; заполненный и валидный draft сначала должен пройти `activate`. Для сохранённого stale нужен допустимый вывод из действия с записанной связью на замену. Не активируйте незаполненный пакет и не правьте неудобный вердикт ради повышения оценки.

Рекурсивный score также учитывает связанные решения-зависимости, поэтому минимум собственных доказательств не обязательно равен итоговому числу. Листовой EVID без дочерних доказательств оценивается по своим полям; не придумывайте evidence-of-evidence ради числа.

`forgeplan stale` сообщает об истёкших датах, но не сохраняет `status: stale`. Используйте `renew` для перепроверенных stale-записей. В 0.38 общая проверка перехода также принимает draft без гейтов активации, а дата проверяется на формат, не на будущий срок ([#547](https://github.com/ForgePlan/forgeplan/issues/547)). Не используйте этот дефект для активации черновиков. Просрочка, сохранённый статус и рекомендательная фаза независимы.

## Технические детали

### Поведение hook'а

Hook `.claude/hooks/pre-pr-evidence-check.sh`:
- Требует подключения Claude Code PreToolUse и его JSON-payload на stdin
- Собирает числовые ID из имени ветки и выбранного диапазона её коммитов (последние 20 — только резервный путь)
- Запрашивает `forgeplan get` и `forgeplan graph --json`; резервный путь — текстовая эвристика в секции связей
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
- **Где**: `plugins/fpl-skills/skills/{audit,sprint,build}/SKILL.md` (публичный маркетплейс)
- **Когда**: Завершение аудита/реализации или закрытие волны спринта по правилам скилла
- **Действие**: Draft EVID, связи с определёнными родителями, отчёт о публикации, запрос или отсрочка активации по правилам

### Layer 2 (Pre-PR Hook)
- **Где**: `.claude/hooks/pre-pr-evidence-check.sh` (этот repo)
- **Когда**: Перед `gh pr create`
- **Trigger**: Claude Code PreToolUse Bash hook с JSON-payload
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
О: Хук пропускает любую ветку `docs/*` по имени, а не доказывает, что diff содержит только документацию. Это исключение не является правилом защиты ветки GitHub.

**В: Что если binary `forgeplan` не установлен?**
О: Hook soft-fails (exits 0) вместо блокировки. Вы всё ещё можете создавать PRs, но теряете gate. Это intentional для fresh clones без построенных binaries.

**В: Могу ли я wire hook в Git вместо Claude Code?**
О: Не простым копированием. Скрипт ожидает JSON Claude Code на stdin и команду `gh pr create`. У Git `pre-push` другой входной протокол; создание PR он не перехватывает.

**В: Что если мой артефакт действительно ad-hoc и не нужно доказательство?**
О: Создайте Note, если он соответствует работе, либо обоснуйте обход. `forgeplan new` не назначает Note срок действия автоматически; не выдумывайте измерение EVID ради проверки наличия связи.

## Reference

- **ADR-003**: Markdown как source of truth для артефактов
- **PRD-077**: Wave 2 evidence autopublish и enforcement
- **PROB-035, PROB-039**: Silent failures из happy-path-only testing
- **Hooks**: `.claude/hooks/pre-pr-evidence-check.sh`
- **Health**: `forgeplan health` команда
- **Schema**: §Structured Fields выше (структура EvidencePack; отдельного `docs/schemas/EVIDENCE.md` пока не существует)

