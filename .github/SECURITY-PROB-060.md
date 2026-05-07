# Политика безопасности — PROB-060 Phase 0b workflow

**Документ**: контракт безопасности для `.github/workflows/assign-id.yml`
**Phase**: 0b prototype (см. PRD-076 / RFC-009 §Phase 0b)
**Статус**: accept-with-policy (Phase 2.1 productionization закрывает surface полностью)
**Связанные**: ADR-012 §Risks → R-1, CWE-94, CWE-829

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

## Tracking

- **Phase 2.1 productionization** — backlog: rebuild binary из
  `origin/dev`; PR HEAD read-only только для сканирования
  `.forgeplan/*.md`. Закрывает attack surface полностью.
- **Drift detector** — periodic audit что `dev` workflow всё ещё
  использует heredoc + env-var pattern (regression guard на Part A).
