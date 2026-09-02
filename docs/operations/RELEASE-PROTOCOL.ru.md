# Release Protocol — Forgeplan

Канонический сквозной протокол релиза для любого `release/vX.Y.Z` cut.
Прочитай сверху вниз первый раз, потом держи как чек-лист. **Шаги не
пропускать** — большинство past release incidents трассировались к
одному пропущенному шагу (см. [Common pitfalls](#common-pitfalls)).

Companion read для daily git flow:
[`GIT-WORKFLOW.ru.md`](GIT-WORKFLOW.ru.md).

---

## Зачем этот документ

Forgeplan ship'ает `forgeplan release-notes` not for nothing: ship-нуть
релиз с hand-written `CHANGELOG.md` ломает audit trail методологии.
Инструмент walks `git log --diff-filter=AM` по artifact directories
между двух ref'ов и emit'ит Keep-a-Changelog–shaped draft, который ты
полишь. Этот протокол pin'ит surrounding workflow так, чтобы инструмент
реально использовался и post-merge sync step не забывался (RED LINE
#9).

v0.31.0 cut (PR #282 → #284 → #285) — canonical reference flow. Всё что
там сработало — source of truth; всё что сломалось — captured в
[Common pitfalls](#common-pitfalls).

(Audit fix 2026-05-14: ранний draft этого документа цитировал PR #283
как integration PR. Это была typo — PR #283 был v0.32 wave-9
integration merge в `dev`, не часть v0.31.0 cut. Последний feature PR
landing в v0.31.0 был PR #282 `feat/v031-w8-quality → dev`. Verify
chain для любого релиза через `gh pr list --state merged --base main
--search vX.Y` плюс immediately-preceding feature merge в `dev`.)

---

## Когда резать релиз

Pre-conditions (всё должно держаться):

- `dev` зелёный (CI passing на merge commit'е, с которого ты base'ишься)
- TODO.md "Current sprint" закрыт или есть justified reason ship'ать
  mid-sprint (security hotfix, regression fix)
- Все артефакты intended для релиза в статусе `active` и имеют evidence
  с R_eff > 0
- `cargo test --workspace` PASS локально на чистом checkout'е `dev`
- `bash scripts/smoke-test.sh` PASS
- Dependabot triage doc для current window лежит в
  `docs/operations/dependabot-triage-YYYY-MM-DD.md` (RED LINE #10)

Если любой pre-condition fail'ит: не начинай релиз. Сначала fix на `dev`.

---

## 10 шагов

### 1. Sync до свежего `dev`

```bash
cd <repo-root>
git checkout dev && git pull --ff-only
```

Fail fast если `--ff-only` refuse'ит — значит локальный `dev` diverged,
и silently amend релиза stale-коммитами — footgun.

### 2. Собрать рабочий бинарь

```bash
cargo build -p forgeplan
```

Бинарь нужен чтобы шаг 3 мог вызвать `release-notes` против текущих
sources. `debug` достаточно (быстрее build); release-optimised бинарь
не требуется для генерации notes.

### 3. Сгенерировать release-notes draft

```bash
./target/debug/forgeplan release-notes \
    --since v<PREV_TAG> \
    --output markdown \
    > /tmp/draft-vX.Y.Z.md
```

`--since v0.30.0` для v0.31.0 cut. Инструмент walks
`.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/`
между двух ref'ов, resolves slugs и `KIND-NNN` IDs, и emit'ит
Keep-a-Changelog–shaped section.

### 4. Полировать draft в `CHANGELOG.md`

Открой `/tmp/draft-vX.Y.Z.md` и отредактируй в `[Unreleased]` секцию
`CHANGELOG.md`. Задачи:

- Написать one-sentence **sprint headline** сверху (что этот релиз
  *значит*? — например, "Wave 9 polish + 5-agent adversarial audit
  closure")
- Поднять security closures в **bullet list сразу под headline**.
  Использовать стиль `**SEC-XX**` и explain impact (какой был silent
  failure pre-fix?)
- Добавить **breaking changes summary** если есть. Migration steps
  inline если коротко, иначе reference SPEC или RFC
- Держать 6 категорий Keep-a-Changelog (`Added`, `Changed`,
  `Deprecated`, `Removed`, `Fixed`, `Security`) + наш `### Internal`
  для engineering details. Пустые категории: omit
- Cross-reference artifact IDs (`PRD-XXX`, `EVID-XXX`) везде где
  reader benefits
- Переименовать `[Unreleased]` → `[X.Y.Z] - YYYY-MM-DD`
- Добавить свежую пустую `## [Unreleased]` строку выше новой версии

### 5. Bump'нуть версии по всему workspace

Четыре ref-типа update'ить — `cargo check` орёт если миссанул:

```toml
# Cargo.toml (workspace root)
[workspace.package]
version = "X.Y.Z"

# crates/forgeplan-cli/Cargo.toml
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z" }
forgeplan-mcp  = { path = "../forgeplan-mcp",  version = "X.Y.Z" }

# crates/forgeplan-mcp/Cargo.toml  ([dependencies] и [dev-dependencies])
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z" }
forgeplan-core = { path = "../forgeplan-core", version = "X.Y.Z", features = ["test-helpers"] }
```

Затем перегенерить lockfile:

```bash
cargo check --workspace
```

Это produces deterministic `Cargo.lock` diff alongside manifest diffs.

### 6. Update human-readable docs

- `CLAUDE.md` → `## Current status` блок: bump version, date, test
  count, one-line sprint summary
- `README.md` → test count badge / строка
- `TODO.md` (если есть "In flight" секция pin'ящая релиз): mark closed

Doc-only, но load-bearing для новых контрибьюторов и для AI agents
priming context на session start (CLAUDE.md auto-loaded every turn).

### 7. Commit на release branch

```bash
git checkout -b release/vX.Y.Z
git add Cargo.toml Cargo.lock CHANGELOG.md CLAUDE.md README.md \
        crates/forgeplan-cli/Cargo.toml crates/forgeplan-mcp/Cargo.toml
git commit -m "release: vX.Y.Z (one-line headline)"
git push -u origin release/vX.Y.Z
```

Использовать **merge commit** стиль (НЕ squash) когда PR land'ится,
чтобы release commit показался в `main`'s history as-is.

### 8. PR `release/vX.Y.Z` → `main`, ждать CI, **user approves merge**

```bash
gh pr create \
    --base main --head release/vX.Y.Z \
    --title "release: vX.Y.Z (headline)" \
    --body "$(cat CHANGELOG.md | sed -n '/## \['"X.Y.Z"'\]/,/^## \[/p' | sed '$ d')"
```

RED LINE #2 всё ещё applies: не мерджить без explicit user approval
после review PR.

CI должен быть fully green — включая `smoke-e2e` job, license audit
(`cargo deny`), MCP tool-count drift detector. Если любой fail'ит, fix
на release branch и re-push; НЕ push'ать напрямую в `main`.

### 9. Tag после merge, дать cargo-dist published

```bash
git fetch origin
git checkout main && git pull --ff-only
MERGE_SHA=$(git log -n 1 --format=%H)
git tag vX.Y.Z $MERGE_SHA
git push origin vX.Y.Z
```

`cargo-dist` watches tag pushes и runs binary publishing workflow
автоматически. Verify Actions tab показывает tag workflow kicking off;
если нет — check что tag реально landed на remote
(`git ls-remote --tags origin | grep vX.Y.Z`).

**Проверить, что опубликованный бинарь несёт заявленные фичи.** Успешная
публикация не доказывает правильный состав — PROB-088 просидел незамеченным
месяцами именно потому, что бинари собирались и публиковались нормально,
просто без `semantic-search`. Скачай один опубликованный артефакт и проверь
маркер линковки, а не доверяй факту сборки:

```bash
# Спроси сам бинарь. Отказ означает, что фичи нет; любой другой исход
# (прогресс-бар, реальный прогон) означает, что она есть.
/path/to/downloaded/forgeplan embed 2>&1 | head -3
```

**Прежняя проверка по линковке больше не работает и не должна возвращаться.**
До RFC-013 маркером было наличие `libc++` в выводе `otool -L` — он появлялся
потому, что влинковывался C++-движок ONNX Runtime. Теперь движок — `tract`,
чистый Rust, и этой библиотеки закономерно нет в бинаре, который фичу
**несёт**. Оставлено предупреждением, а не удалено: вернув эту проверку, мы
получим «сломано» на каждой корректной сборке.

Сверь ответ с ключом `features` в `dist-workspace.toml` и с тем, что
обещают пользователю install-доки. Если три источника расходятся — значит
документация кому-то врёт; чинить до анонса.

**Не удалять release branch.** Keep как immutable history (по project
convention — см. `feedback_keep_branches`).

### 10. **REQUIRED: открыть sync-PR** (RED LINE #9)

```bash
git checkout main && git pull --ff-only
git checkout -b chore/sync-main-to-dev-after-vX.Y.Z
git push -u origin chore/sync-main-to-dev-after-vX.Y.Z
gh pr create --base dev --head chore/sync-main-to-dev-after-vX.Y.Z \
    --title "chore: sync main → dev after vX.Y.Z" \
    --body "Routine post-release sync. Pulls релизный commit в dev чтобы следующая feature ветка стартовала от bump'нутой версии."
```

Branch protection блокирует direct push в `dev`, так что этот PR —
единственный sanctioned path. **Без него `dev` forever lag'ает за
`Cargo.toml` версией, и следующий релиз создаст merge conflicts на
manifest.** См. PR #262 (sync-after-v0.30.0) и PR #285
(sync-after-v0.31.0) как canonical examples — каждый релиз ship'ается
с одним.

Approve и merge sync-PR сам после CI passes (low-risk mechanical sync;
user approval — для release PR, не этого).

---

## Common pitfalls

### Забыть sync-PR (шаг 10)

Symptom: через неделю кто-то открывает feature branch off `dev`,
bump'ает `Cargo.toml` per local convention, gets merge conflict против
`dev` потому что `dev`'s `version = "X.Y.(Z-1)"` а working copy
говорит `"X.Y.Z"`. Или worse: следующий релиз tries to bump `dev`'s
version и discovers что она уже на post-release value, с
`Cargo.lock` inconsistent.

Fix: открыть sync-PR ДО закрытия release session. Protocol sequence
кончается на шаге 10 not for nothing — не treat'ить как optional.

### Push'ать в release branch *после* мерджа PR

Symptom: поздний commit land'ится на release branch уже после `git
merge`; `git push` succeeds но коммит висит dangling. Worse: если был
squash merge, весь late commit silently discarded.

Fix: никогда не push'ать в branch после мерджа PR. Если нужно
"amend the release", открой свежую `fix/vX.Y.Z-hotfix` branch и trat'ь
как patch release (vX.Y.(Z+1)). См. `feedback_squash_merge_loss` в
auto-memory.

### Версия не bump'нута в intra-workspace path refs

Symptom: `cargo publish` (если используется) errors на dependency
version mismatch. Или end-users устанавливающие через
`cargo install --git` получают inconsistent linkage warnings.

Fix: шаг 5 lists ровно четыре `version = "X.Y.Z"` локации. После
правки `cargo check --workspace` fail'ит loud если миссанул (он
resolve'ит path dep но warn'ит что explicit `version =` field уже не
satisfied). Trat'ить warning как hard error — re-grep'нуть
`grep -rn 'version = "<PREV>"' Cargo.toml crates/*/Cargo.toml`.

### Hand-write CHANGELOG вместо `release-notes`

Symptom: changelog miss'ит artifact cross-references, mis-spell'ит
slugs, или omit'ит artifacts которые shipped on `dev` mid-sprint. Audit
trail сломан.

Fix: всегда start from `forgeplan release-notes --since <prev>`
output. Polish для narrative, НЕ bypass для "скорости" — инструмент
читает ровно те артефакты которые ты commit'нул; hand-writing diverges
от ground truth.

### CI fail'ит на smoke-e2e *после* того как tag pushed

Symptom: tag landed, бинари не publish'атся, users репортят 404 на
brew bottle.

Fix: никогда не tag'ать до того как CI на `main` fully green. Шаг 9
читает `git checkout main && git pull --ff-only` first; единственный
способ чтобы это succeed'ило после мерджа release PR — это merge
commit на `main`'s tip, и единственный способ что *этот* tip safe to
tag — это что его CI passed. Check `gh run list --branch main --limit
3` ПЕРЕД `git tag`.

### Dependabot alerts на release time (RED LINE #10)

Symptom: релиз ship'ается с open dependabot alerts; user runs
`gh api repos/.../dependabot/alerts` позже и видит unaddressed
high-severity CVEs unmentioned в changelog.

Fix: в рамках шага 4 (changelog polish), check
`gh api repos/.../dependabot/alerts` и добавь параграф в
`### Security` секцию listing каждого alert'а как **addressed**,
**scheduled** (next release с target version), или **accepted with
justification** (explicit risk-take statement). Файл triage doc под
`docs/operations/dependabot-triage-YYYY-MM-DD.md`.

---

## Migration notes для существующих workspace'ов (SEC-H1, CR-C4 — v0.32.0+)

`forgeplan init` short-circuit'ится когда `.forgeplan/` уже существует. Это
значит что контрибьюторы которые upgrade'или `forgeplan` между релизами
**не получают автоматически** newly-shipped workspace файлы (например
`.gitkeep` плейсхолдеры из PRD-077 FR-001, `secrets.env` template из
FR-002).

`forgeplan init --force` — migration entry point. Он **strictly additive**
(PROB-068 контракт):

- Existing artifact `.md` bodies НИКОГДА не перезаписываются.
- `config.yaml` регенерируется (предыдущая версия сдвигается в сторону
  как `config.yaml.bak-<timestamp>` чтобы контрибьютор мог diff'нуть
  свои custom edits и переприменить их поверх new defaults).
- `.gitkeep` плейсхолдеры backfill'ятся в каждый artifact subdir где
  они отсутствуют (SEC-H1).
- `secrets.env` template backfill'ится если missing — **никогда** не
  clobber'ит существующий файл который контрибьютор мог заполнить
  реальными ключами (SEC-H1).
- Canonical `.gitignore` секция refresh'ится (PROB-062).

При анонсе релиза который shipи'т новые workspace skeleton файлы,
включай в release notes такой блок:

```
Для existing workspace'ов созданных до vX.Y.Z, выполни:

    git pull
    forgeplan init --force

Это idempotent и additive — твои artifact bodies, custom config.yaml
edits, и existing secrets.env ключи сохраняются.
```

---

## Quick checklist (copy в PR description)

```
- [ ] 1. `git checkout dev && git pull --ff-only`
- [ ] 2. `cargo build -p forgeplan`
- [ ] 3. `forgeplan release-notes --since v<PREV> --output markdown > /tmp/draft.md`
- [ ] 4. Polish draft в `CHANGELOG.md`, переименовать `[Unreleased]` → `[X.Y.Z] - YYYY-MM-DD`
- [ ] 5. Bump Cargo.toml workspace.version + 4 intra-workspace path-version refs
- [ ] 6. Update `CLAUDE.md` Current status + `README.md` test count
- [ ] 7. Commit `release: vX.Y.Z (...)` на `release/vX.Y.Z` branch
- [ ] 8. PR → main; ждать зелёный CI; user approves merge
- [ ] 9. `git tag vX.Y.Z <merge-sha> && git push origin vX.Y.Z` (cargo-dist publish'ит)
- [ ] 10. **REQUIRED**: sync-PR `chore/sync-main-to-dev-after-vX.Y.Z` (RED LINE #9)
```

---

## See also

- [`GIT-WORKFLOW.ru.md`](GIT-WORKFLOW.ru.md) — daily flow, branching
  strategy, PR pipeline
- [`QUALITY-GATES.ru.md`](QUALITY-GATES.ru.md) — full CI gate reference
- v0.31.0 cut: PR #282 (`feat/v031-w8-quality → dev`, последний v0.31
  feature PR) → PR #284 (release/v0.31.0 → main) → PR #285 (sync-after)
  — canonical reference flow. (Note: PR #283 был v0.32 wave-9
  integration merge в dev, НЕ часть v0.31 cut — ранние drafts этого
  документа mis-attribute'или его; corrected 2026-05-14 per audit.)
