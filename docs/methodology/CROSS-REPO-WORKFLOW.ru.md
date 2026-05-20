# Cross-repo workflow в ForgePlan org

> Как координировать issues, fix'ы и feature requests между репозиториями
> одной организации (`forgeplan` core ↔ `marketplace` ↔ `forgeplan-hud` ↔ ...)
> чтобы знание не терялось и работа шла предсказуемо.

## Проблема которую это решает

Когда работаешь в одном repo и наткнулся на bug **другого** repo, есть 5 типичных провалов:

1. **Bug фиксится локально workaround'ом**, репорт upstream забывают написать
2. **Issue создаётся в неправильном repo** (там где обнаружено, не там где fix должен жить)
3. **Связь между issues теряется** — fix в repo A не закрывает issue в repo B
4. **Документация в downstream repo описывает workaround**, который перестал быть нужным после upstream fix — но никто не убирает
5. **На session start не видишь** какие issues уже открыты — работаешь "вслепую", дублируешь работу

Этот документ — стандарт как **не попадать** в эти провалы.

## Tax onomy: label hierarchy (org-wide)

Все repos в org должны иметь **одинаковый минимальный набор labels**. Внутренние per-repo labels (типа `prd`, `rfc` в marketplace) — поверх.

### Core (обязательно во всех repos)

**Type** (что это):
- `bug` — что-то работает не так
- `enhancement` — feature request
- `documentation` — docs only
- `refactor` — internal refactor без user-visible change
- `question` — обсуждение, не actionable
- `security` — vulnerability / hardening

**Severity** (насколько срочно):
- `severity:critical` — production-blocker
- `severity:high` — серьёзный bug или важная feature
- `severity:medium` — обычный workflow item
- `severity:low` — nice-to-have

**Origin** (откуда узнали):
- `dogfood` — surfaced через реальную работу
- `audit` — adversarial audit round
- `community` — внешний reporter

**Cross-repo coordination** (важнейшая категория для org):
- `cross-repo` — issue затрагивает >1 repo, ищи Refs в body
- `upstream:<repo>` — заблокировано фиксом в <repo>
- `downstream:<repo>` — этот фикс разблокирует <repo>

### Repo-specific (поверх core)

В каждом repo разрешено добавлять свои labels для domain — например `forgeplan` в marketplace для tracking forgeplan artifacts. Не запрещено, но не должно дублировать core taxonomy.

## Когда какой repo использовать (decision tree)

### Bug surfaced при работе в repo X

```
Где источник bug'а?
├── В repo X → создавай issue в X
├── В upstream dependency (Y) → создавай issue в Y
│   └── Если workaround есть в X → отдельный issue в X
│       label: cross-repo + downstream:Y
│       body: "Workaround for Y#NNN — remove when upstream fixes"
└── Не понятно где → создавай в X, label: triage
```

### Feature request на boundary между двумя repos

```
1. Создай issue в repo где БУДЕТ FIX
2. Если требует изменений в втором repo:
   - Issue #1 в repo A (main fix)
     label: cross-repo + downstream:B
     body: содержит ссылку на #2
   - Issue #2 в repo B (follow-up)
     label: cross-repo + upstream:A
     body: "Depends on A#NNN"
3. PR в repo A merge'ит #1 + crossrefs #2 ("see also B#MMM")
4. PR в repo B merge'ит #2 + closes "Closes A#NNN B#MMM"
```

### Documentation update в downstream repo после upstream fix

Это случай **forgeplan#292 → marketplace#86**:
- Core repo: code fix (forgeplan#292 closed by PR)
- Marketplace repo: docs update issue (marketplace#86, references upstream)
- Marketplace docs PR closes #86 после release upstream

Pattern: документация **никогда не сидит в одном repo если она описывает кросс-repo поведение**.

## Issue body conventions

Каждый cross-repo issue должен иметь в body минимум:

```markdown
## Summary
<one paragraph>

## Affected files (or surface)
- `path/to/file.rs` — что там не так
- Cross-repo: `OtherRepo:path/to/file.md` — что обновить там

## Upstream / Downstream

- Upstream blocker: ForgePlan/repoY#NNN
- Downstream impact: ForgePlan/repoZ#MMM
- Related: see also ForgePlan/repoW#KKK

## Acceptance criteria
- [ ] что нужно done в этом repo
- [ ] что нужно done в кросс-repo (с прямой ссылкой)
```

## Commit / PR conventions для cross-repo

### Closing issues across repos

GitHub поддерживает синтаксис:
```
Closes ForgePlan/marketplace#86
```
В PR body для cross-repo closure. Используй это вместо `Closes #86` если PR в другом repo.

### Referencing without closing

```
Refs ForgePlan/forgeplan#292
```
Создаёт обратную ссылку без auto-close.

### PR title (cross-repo PR)

Префиксуй scope именем второго repo:
```
[+marketplace] fix(discover): add session_status alias (closes #292, refs marketplace#86)
```

## Session start protocol (для AI agents)

### Что должен делать агент при старте session

1. **Read project memory** (CLAUDE.md, MEMORY.md)
2. **Read git state** (branch, recent commits)
3. **Inbox sweep** для всех repos в org:
   ```bash
   for repo in $(gh repo list ForgePlan --json name -q '.[].name'); do
     gh issue list --repo ForgePlan/$repo --state open --label "cross-repo,severity:high,severity:critical" --json number,title,labels
   done
   ```
4. **Triage to user**: показать summary `N open issues across M repos (X critical, Y cross-repo)`. Спросить:
   - "Хочешь review каждый?"
   - "Сразу build sprint plan из них?"
   - "Skip — продолжать что начинал?"
5. Если user выбирает review → итерация по issues с decision'ами:
   - **Close as obsolete** — устарел
   - **Add to current sprint** — берёшь в работу сейчас
   - **Plan for later** — оставить open, schedule
   - **Cross-repo** — открыть partner issue в другом repo

### Конкретная реализация (предложение)

- **Hook**: `SessionStart` hook в `.claude/hooks/` который calls `gh issue list` для всех org repos
- **Skill**: `cross-repo-inbox` skill activates когда user открывает session и spawn'ит triage prompt
- **MCP tool**: `forgeplan_inbox` или `orchestra_inbox` — единая точка sweep

## Best practices summary

1. **Один issue — один repo** (где будет fix), даже если bug surfaced в другом
2. **Cross-repo dependencies явно labeled** — `cross-repo` + `upstream:<repo>` / `downstream:<repo>`
3. **Body ВСЕГДА содержит cross-refs** на partner issues
4. **PR closes issues across repos** через `Closes Org/repo#NNN` синтаксис
5. **Documentation cleanup** в downstream repo — отдельный issue с reference на upstream fix
6. **Session start = inbox sweep** перед началом работы, не после
7. **Labels org-wide consistent** — taxonomy одинакова для всех repos в org

## Что НЕ делать

❌ **Не вносить cross-repo fix в один PR одного repo** — это hides dependency и breaks audit trail
❌ **Не reuse `status` label / `type` label с разными значениями в разных repos** — путает agents и людей
❌ **Не закрывать upstream issue PR'ом в downstream repo** — `Closes` сработает (GitHub permits) но смысл потерян
❌ **Не дублировать issue в каждом repo** — choose primary repo, cross-ref остальные

## Implementation roadmap (org-wide rollout)

### Phase 0: foundation (this doc)
- [x] Methodology written down
- [ ] User review + approve

### Phase 1: label sync
- [ ] Script: sync core label taxonomy across all org repos
- [ ] Apply core labels (`severity:*`, `origin:*`, `cross-repo`, `upstream:*`, `downstream:*`)
- [ ] CI workflow: enforce labels on new issues

### Phase 2: issue templates
- [ ] `.github/ISSUE_TEMPLATE/bug.yml` в org `.github` repo (наследуется всеми)
- [ ] `.github/ISSUE_TEMPLATE/feature.yml`
- [ ] `.github/ISSUE_TEMPLATE/cross-repo.yml` — structured cross-repo dep

### Phase 3: session start automation
- [ ] Hook `.claude/hooks/session-start-issue-inbox.sh` — sweep open issues across org
- [ ] Skill `cross-repo-inbox` — triage flow with user interaction
- [ ] Optional MCP tool `forgeplan_inbox` для integration

### Phase 4: dashboard
- [ ] GitHub Project (v2) org-wide — view across all repos
- [ ] Custom fields: severity, status, origin
- [ ] Saved views: "cross-repo open", "my critical", "this week"

## Connection to existing tooling

- **Orchestra MCP** — уже track'ает tasks, может стать **org-wide inbox UI**
- **Hindsight MCP** — может сохранить decisions ("выбрали Option E для #292") как long-term memory
- **Forgeplan** — может tracking issues как Notes/Problems артефактами с linked external refs

## References

- GitHub docs: [linking PR to issue across repos](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue)
- GitHub docs: [issue templates](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository)
- GitHub Projects (v2): [cross-repo views](https://docs.github.com/en/issues/planning-and-tracking-with-projects)

---

**Last updated**: 2026-05-20. Audit-r7 + dogfood-findings sprint surfaced the
need explicitly when 6 marketplace-discovered bugs got filed in forgeplan core
without a clear protocol — this doc is the protocol.
